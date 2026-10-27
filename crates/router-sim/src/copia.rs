//! La copia de la configuración: `cbSaveConfig` con `save=1` la guarda y con `backup=1` genera el fichero (da su `<FileName>`); `cbDownloadConfig` lo descarga.
//!
//! Como el router real (capturado de la web): la descarga es un `POST`
//! sin cuerpo con `fileName` y `sessionKey` en la URL, en la sesión
//! avanzada, y responde `application/octet-stream` con
//! `Content-Disposition: attachment; filename="<fichero>"` (el real, sin
//! `Content-Length`; aquí va con él, el cliente no depende de eso). El contenido es inventado (empieza por `ASP_CM`, como
//! el cifrado del router, y dice cuántas veces se había guardado: la copia
//! es de lo último guardado). Con cuerpo, sin la sesión, con otro
//! `sessionKey` o con otro fichero, no la da.

use crate::LOGGED_OUT;
use crate::estado::{Params, SimState};
use crate::http::{form, param};
use crate::xml::{leaf, response, result_node};
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};

/// La CGI de la descarga (sin `.xml`).
pub(crate) const DESCARGA: &str = "cbDownloadConfig";

/// `cbSaveConfig` (ya con el `sessionKey` comprobado): `save=1` guarda y
/// `backup=1` genera la copia y da su nombre.
pub(crate) fn guardar(st: &mut SimState, p: &Params) -> String {
    if param(p, "save") == Some("1") {
        st.guardados += 1;
    }
    let mut hijos = Vec::new();
    if param(p, "backup") == Some("1") {
        let nombre = format!("config_2026102710{:04}.tar", st.counter % 10_000);
        st.copia = Some((nombre.clone(), st.guardados));
        hijos.push(leaf("FileName", &nombre));
    }
    response(&result_node("cbSaveConfig", hijos, "0", ""))
}

/// `POST /cgi-bin/cbDownloadConfig.xml?<consulta>` con `cuerpo`, con la
/// sesión avanzada `sesion_ok`.
pub(crate) fn descargar(
    st: &mut SimState,
    sesion_ok: bool,
    consulta: &str,
    cuerpo: &[u8],
) -> Response {
    if !sesion_ok {
        return LOGGED_OUT.into_response();
    }
    let q = form(consulta.as_bytes());
    let clave_ok = st
        .session_key
        .as_deref()
        .is_some_and(|k| param(&q, "sessionKey") == Some(k));
    let fichero = param(&q, "fileName").unwrap_or_default().to_owned();
    let Some((nombre, guardados)) = st.copia.clone().filter(|(n, _)| *n == fichero) else {
        return (StatusCode::NOT_FOUND, "<html><body>404</body></html>").into_response();
    };
    if !cuerpo.is_empty() || !clave_ok {
        return (StatusCode::BAD_REQUEST, "<html><body>400</body></html>").into_response();
    }
    let datos = st.copia_fijada.clone().unwrap_or_else(|| {
        let veces = if guardados == 1 { "vez" } else { "veces" };
        format!("ASP_CM\0copia simulada, inventada; guardada {guardados} {veces}\n").into_bytes()
    });
    st.copias_descargadas.push(nombre.clone());
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{nombre}\""),
            ),
        ],
        datos,
    )
        .into_response()
}
