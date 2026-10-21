//! Ayudas comunes: arranque del simulador, cliente y lectura de las respuestas.

use super::*;

pub(crate) const USER: &str = "prueba";

pub(crate) const PASS: &str = "clave-de-prueba";

pub(crate) fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../hgu/fixtures"))
}

pub(crate) fn variantes_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hgu/fixtures-variantes"
    ))
}

pub(crate) async fn start() -> SimHandle {
    Sim::start(SimConfig::new(USER, PASS, fixtures_dir())).await
}

pub(crate) fn client(sim: &SimHandle, timeout: Duration) -> Client {
    Client::new(ClientConfig {
        base_url: sim.base_url.clone(),
        user: USER.into(),
        password: PASS.into(),
        timeout,
        min_gap: Duration::from_millis(1),
    })
}

/// Recorre las hojas `(etiqueta, texto)` de un documento.
pub(crate) fn leaves(raw: &str) -> Vec<(String, String)> {
    let doc = roxmltree::Document::parse(raw).expect("fixture XML válida");
    doc.descendants()
        .filter(|n| n.is_element() && !n.children().any(|c| c.is_element()))
        .map(|n| {
            (
                n.tag_name().name().to_owned(),
                n.text().unwrap_or("").trim().to_owned(),
            )
        })
        .collect()
}

pub(crate) fn leases(n: &XmlNode) -> Vec<(String, String, String)> {
    n.all("rtm_dhcpv4_svr_pool_static")
        .into_iter()
        .map(|e| {
            (
                e.text_of("rtm_dhcpv4_svr_pool_static_id")
                    .unwrap()
                    .to_owned(),
                e.text_of("mac_addr").unwrap().to_owned(),
                e.text_of("ipv4_addr").unwrap().to_owned(),
            )
        })
        .collect()
}

pub(crate) const LIST_STATIC: &[(&str, &str)] =
    &[("rtm_cfg_dhcpv4_svr_pool_id", "0"), ("all", "ALL")];

pub(crate) fn add_static(mac: &'static str, ip: &'static str) -> [(&'static str, &'static str); 5] {
    [
        ("adm_state", "1"),
        ("mac_addr", mac),
        ("ipv4_addr", ip),
        ("rtm_dhcpv4_svr_pool_static_id", "-1"),
        ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
    ]
}

// ------------------------------------------------ escritura genérica

/// Valor del primer nodo con hijo `campo` en `n` (o en sus descendientes).
pub(crate) fn valor(n: &XmlNode, campo: &str) -> Option<String> {
    n.text_of(campo)
        .map(str::to_owned)
        .or_else(|| n.children.iter().find_map(|c| valor(c, campo)))
}

// ------------------------------------------------ ronda de corrección 1

pub(crate) fn campo_de(n: &XmlNode, instancia: &str, campo: &str) -> Option<String> {
    n.get(instancia)?.text_of(campo).map(str::to_owned)
}
