//! El registro del sistema del router, como lo descarga syslog.asp.

use super::Client;
use crate::error::Error;

/// Lo más que se guarda del registro: sus últimos 2 MiB, en líneas enteras.
/// Un registro mayor llega recortado ([`RegistroDelSistema::recortado`]) y
/// nunca ocupa en memoria mucho más que el doble mientras se lee.
pub const TOPE_REGISTRO: usize = 2 * 1024 * 1024;

/// Página de la que sale el `sessionKey` de la exportación: la de la web.
const PAGINA: &str = "syslog.asp";
/// Donde deja el router el registro exportado.
const RUTA: &str = "/syslog.log";

/// El registro del sistema tal como lo da el router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistroDelSistema {
    /// El texto, con las líneas más nuevas al final.
    pub texto: String,
    /// Era mayor que [`TOPE_REGISTRO`]: faltan las líneas más antiguas.
    pub recortado: bool,
}

impl Client {
    /// El registro del sistema, como lo hace syslog.asp al cargarse: pide
    /// al router que lo exporte (`cbSetSyslog` con solo `export=EXPORT` y
    /// el `sessionKey` de syslog.asp) y lo lee de `/syslog.log`, todo en un
    /// mismo turno del cerrojo. La exportación no cambia ningún ajuste, así
    /// que no cuenta como escritura ([`Client::escrituras`]): lo leído antes
    /// sigue valiendo.
    pub async fn registro_del_sistema(&self) -> Result<RegistroDelSistema, Error> {
        let mut st = self.shared.state.lock().await;
        self.write_locked(&mut st, PAGINA, "cbSetSyslog", &[("export", "EXPORT")])
            .await?;
        let (texto, recortado) = self
            .pagina_con_tope(&mut st, RUTA, Some(TOPE_REGISTRO))
            .await?;
        Ok(RegistroDelSistema { texto, recortado })
    }
}

/// Lee el cuerpo de `resp` guardando solo su cola: como mucho sus últimos
/// `tope` bytes, empezando por una línea entera. El segundo valor dice si
/// se dejó algo fuera.
pub(super) async fn cola(
    mut resp: reqwest::Response,
    tope: usize,
) -> Result<(String, bool), reqwest::Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut recortado = false;
    while let Some(trozo) = resp.chunk().await? {
        buf.extend_from_slice(&trozo);
        if buf.len() > 2 * tope {
            buf.drain(..buf.len() - tope);
            recortado = true;
        }
    }
    if buf.len() > tope {
        buf.drain(..buf.len() - tope);
        recortado = true;
    }
    if recortado {
        buf.drain(..desde_linea_entera(&buf));
    }
    Ok((String::from_utf8_lossy(&buf).into_owned(), recortado))
}

/// Dónde empieza la primera línea entera de `b` (tras el primer salto). Sin
/// salto, el primer carácter entero: nunca a media secuencia UTF-8.
fn desde_linea_entera(b: &[u8]) -> usize {
    match b.iter().position(|&c| c == b'\n') {
        Some(i) => i + 1,
        None => b.iter().take_while(|&&c| c & 0xC0 == 0x80).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::desde_linea_entera;

    #[test]
    fn empieza_en_una_linea_o_un_caracter_entero() {
        assert_eq!(desde_linea_entera(b"ea\nsegunda\n"), 3);
        assert_eq!(desde_linea_entera(b"sin salto"), 0);
        // «í» es 0xC3 0xAD: si el corte cae entre los dos, se salta el 0xAD.
        assert_eq!(desde_linea_entera(&[0xAD, b'a', b'b']), 1);
    }
}
