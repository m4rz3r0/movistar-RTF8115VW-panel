//! La copia de la configuración del router, como la descarga backup_restore.asp.
//!
//! La web (`backup_restore.js`) pide `cbSaveConfig` con `backup=1` al
//! cargarse, que da el nombre del fichero (`<FileName>`), y al pulsar el
//! botón lo descarga con `POST /cgi-bin/cbDownloadConfig.xml?fileName=…&sessionKey=…`
//! sin cuerpo: el router responde `application/octet-stream` con
//! `Content-Disposition: attachment; filename="config_<fecha>.tar"`. La
//! copia solo lleva la última configuración guardada (lo dice la página),
//! así que antes se guarda, como `save.asp` (`cbSaveConfig` con `save=1`).
//!
//! El fichero lleva las contraseñas del router: aquí no se anota ni se
//! guarda nada de él, y su depuración ([`CopiaDeConfiguracion`]) solo da el
//! nombre y el tamaño.

use super::{Client, State, form_body, looks_logged_out};
use crate::error::Error;
use reqwest::Method;

/// Lo más grande que se descarga: una copia mayor es un error (no sirve a
/// medias). Las del router real ocupan unos KB.
pub const TOPE_COPIA: usize = 16 * 1024 * 1024;

/// Página de la que sale el `sessionKey` del guardado.
const PAGINA_GUARDAR: &str = "save.asp";
/// Página de la que salen el `sessionKey` de la copia y el de la descarga.
const PAGINA_COPIA: &str = "backup_restore.asp";
/// La CGI que guarda y genera la copia.
const GUARDAR: &str = "cbSaveConfig";
/// La CGI que descarga el fichero.
const DESCARGAR: &str = "/cgi-bin/cbDownloadConfig.xml";
/// Lo más largo de un nombre de fichero que se acepta.
const MAX_NOMBRE: usize = 64;

/// La copia descargada, tal cual.
#[derive(Clone, PartialEq, Eq)]
pub struct CopiaDeConfiguracion {
    /// El nombre que da el router (`config_<fecha>.tar`).
    pub nombre: String,
    /// El fichero, sin tocar.
    pub datos: Vec<u8>,
}

impl std::fmt::Debug for CopiaDeConfiguracion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopiaDeConfiguracion")
            .field("nombre", &self.nombre)
            .field("bytes", &self.datos.len())
            .finish()
    }
}

/// ¿Vale `s` como nombre de fichero? Letras, cifras, `.`, `_` y `-`: ni
/// rutas ni nada que haya que escapar.
pub(crate) fn nombre_valido(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_NOMBRE
        && !s.starts_with('.')
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// El `filename` de una cabecera `Content-Disposition`, si es un nombre
/// válido.
fn nombre_de_disposicion(v: &str) -> Option<String> {
    let resto = &v[v.find("filename=")? + "filename=".len()..];
    let nombre = resto
        .trim_start_matches('"')
        .split(['"', ';'])
        .next()?
        .trim();
    nombre_valido(nombre).then(|| nombre.to_owned())
}

/// Lee el cuerpo de `resp` hasta `tope` bytes; el segundo valor dice si
/// era mayor (y se dejó de leer).
pub(super) async fn con_tope(
    mut resp: reqwest::Response,
    tope: usize,
) -> Result<(Vec<u8>, bool), reqwest::Error> {
    let mut buf = Vec::new();
    while let Some(trozo) = resp.chunk().await? {
        if buf.len() + trozo.len() > tope {
            return Ok((Vec::new(), true));
        }
        buf.extend_from_slice(&trozo);
    }
    Ok((buf, false))
}

/// Lo que da una descarga: el fichero o, si el router pidió entrar otra
/// vez, nada.
enum Descarga {
    Fichero(CopiaDeConfiguracion),
    SinSesion,
}

impl Client {
    /// La copia de la configuración, como la web y en un mismo turno del
    /// cerrojo: guarda (`save=1`, con el `sessionKey` de save.asp), genera
    /// la copia (`backup=1`, con el de backup_restore.asp) y la descarga
    /// con el nombre que dio. Guardar no cambia ninguna lectura, así que no
    /// cuenta como escritura ([`Client::escrituras`]). Un fichero mayor que
    /// [`TOPE_COPIA`] es un error.
    pub async fn copia_de_configuracion(&self) -> Result<CopiaDeConfiguracion, Error> {
        let mut st = self.shared.state.lock().await;
        self.write_locked(&mut st, PAGINA_GUARDAR, GUARDAR, &[("save", "1")])
            .await?;
        let n = self
            .write_locked(&mut st, PAGINA_COPIA, GUARDAR, &[("backup", "1")])
            .await?;
        let fichero = n
            .text_of("FileName")
            .map(str::trim)
            .filter(|f| nombre_valido(f))
            .ok_or_else(|| {
                Error::Unexpected(format!(
                    "{GUARDAR}: el router no da un nombre de copia válido"
                ))
            })?
            .to_owned();
        for _ in 0..2 {
            if !self.ensure_advanced(&mut st).await? {
                continue;
            }
            let clave = self.session_key_locked(&mut st, PAGINA_COPIA).await?;
            match self.descargar(&mut st, &fichero, &clave).await? {
                Descarga::Fichero(c) => return Ok(c),
                Descarga::SinSesion => Self::invalidate(&mut st),
            }
        }
        Err(Error::SessionExpired(format!(
            "{DESCARGAR}: el router sigue pidiendo iniciar sesión"
        )))
    }

    /// `POST` vacío a [`DESCARGAR`] con `fileName` y `sessionKey` en la URL.
    async fn descargar(
        &self,
        st: &mut State,
        fichero: &str,
        clave: &str,
    ) -> Result<Descarga, Error> {
        let consulta = form_body(&[("fileName", fichero), ("sessionKey", clave)]);
        let path = format!("{DESCARGAR}?{consulta}");
        let req = self
            .peticion(st, Method::POST, &path, None)
            .await?
            .header("Content-Length", 0);
        let result = async {
            let resp = req.send().await?;
            let set = super::cookies_nuevas(&resp);
            let status = resp.status();
            let cabecera = |k: &str| {
                resp.headers()
                    .get(k)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned)
            };
            let (tipo, disposicion) = (cabecera("content-type"), cabecera("content-disposition"));
            let leido = con_tope(resp, TOPE_COPIA).await?;
            Ok::<_, reqwest::Error>((status, set, (tipo, disposicion, leido)))
        }
        .await;
        let (tipo, disposicion, (datos, excedido)) = self.recibido(st, DESCARGAR, result)?;
        if excedido {
            return Err(Error::Unexpected(format!(
                "{DESCARGAR}: la copia es demasiado grande (más de {} MiB)",
                TOPE_COPIA / (1024 * 1024)
            )));
        }
        let tipo = tipo.unwrap_or_default().to_ascii_lowercase();
        if tipo.contains("html") || tipo.contains("xml") || tipo.starts_with("text/") {
            // Sin la sesión, la página de entrar; si no, un error del
            // router (nunca se enseña su texto).
            let texto = String::from_utf8_lossy(&datos);
            if looks_logged_out(&texto) {
                return Ok(Descarga::SinSesion);
            }
            return Err(Error::Unexpected(format!(
                "{DESCARGAR}: el router no da la copia"
            )));
        }
        let nombre = disposicion
            .as_deref()
            .and_then(nombre_de_disposicion)
            .unwrap_or_else(|| fichero.to_owned());
        Ok(Descarga::Fichero(CopiaDeConfiguracion { nombre, datos }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombres_de_fichero() {
        assert!(nombre_valido("config_20261027101500.tar"));
        for malo in [
            "",
            "../x.tar",
            ".tar",
            "a b.tar",
            "a/b",
            "a\"b",
            &"x".repeat(65),
        ] {
            assert!(!nombre_valido(malo), "{malo}");
        }
        assert_eq!(
            nombre_de_disposicion("attachment; filename=\"config_1.tar\"").as_deref(),
            Some("config_1.tar")
        );
        assert_eq!(
            nombre_de_disposicion("attachment; filename=config_2.tar; x=y").as_deref(),
            Some("config_2.tar")
        );
        assert_eq!(nombre_de_disposicion("attachment; filename=\"../a\""), None);
        assert_eq!(nombre_de_disposicion("inline"), None);
    }

    #[test]
    fn la_depuracion_no_ensena_el_contenido() {
        let c = CopiaDeConfiguracion {
            nombre: "config_1.tar".into(),
            datos: b"ASP_CM secreto".to_vec(),
        };
        let d = format!("{c:?}");
        assert!(d.contains("config_1.tar") && d.contains("14"), "{d}");
        assert!(!d.contains("secreto"), "{d}");
    }
}
