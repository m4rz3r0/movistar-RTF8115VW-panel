//! El registro del sistema: `cbSetSyslog` con `export=EXPORT` (lo que hace
//! syslog.asp al cargarse) lo genera y `GET /syslog.log` lo sirve.
//!
//! Supuesto, sin comprobar en el router real: antes de la primera
//! exportación desde el arranque el fichero no existe y su servidor web
//! responde 404; después queda servido (con lo de la última exportación)
//! hasta el siguiente reinicio, como un fichero en memoria. Lo sirve con la
//! sesión básica, como sus páginas.

use crate::LOGGED_OUT;
use crate::estado::{Inner, SimState};
use crate::http::{has_basic_en, param};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Ruta del registro exportado.
pub(crate) const RUTA: &str = "/syslog.log";

/// Registro de serie: líneas inventadas, con direcciones de documentación
/// (192.0.2.0/24, 2001:db8::/32) y MAC administradas localmente.
const DE_SERIE: &str = "\
Jan  1 00:00:12 router syslog.info syslogd started: BusyBox v1.00
Jan  1 00:00:13 router kern.notice kernel: Linux version 3.4.11
Jan  1 00:00:20 router daemon.info dnsmasq[512]: started, version 2.62 cachesize 150
Jan  1 00:00:25 router user.notice wan: PPP session up, IPv4 192.0.2.10
Jan  1 00:00:26 router user.notice wan: IPv6 prefix 2001:db8:1::/56
Jan  1 00:01:02 router daemon.info dnsmasq-dhcp[512]: DHCPACK(br0) 192.0.2.100 02:00:00:00:00:01
Jan  1 00:05:40 router auth.info httpd: login from 192.0.2.100 accepted
Jan  1 00:09:15 router user.warning wifi: station 02:00:00:00:00:02 deauthenticated
";

/// ¿`params` (sin `sessionKey`) piden exportar el registro?
pub(crate) fn pide_exportar(params: &[(String, String)]) -> bool {
    param(params, "export") == Some("EXPORT")
}

/// Exporta el registro: desde ahora se sirve en [`RUTA`].
pub(crate) fn exportar(st: &mut SimState) {
    st.registro_exportado = true;
}

/// `GET /syslog.log`: el registro exportado, 404 si no se ha exportado
/// desde el arranque, o la página de entrar sin sesión.
pub(crate) async fn servir(State(s): State<Arc<Inner>>, headers: HeaderMap) -> Response {
    let st = s.state();
    if !has_basic_en(&st, &headers) {
        return LOGGED_OUT.into_response();
    }
    if !st.registro_exportado {
        return (
            StatusCode::NOT_FOUND,
            "<html><body>404 Not Found</body></html>",
        )
            .into_response();
    }
    let texto = st.registro.as_deref().unwrap_or(DE_SERIE).to_owned();
    ([(header::CONTENT_TYPE, "text/plain")], texto).into_response()
}
