//! Red local: los dispositivos conectados (la red de casa de `leer_tema`) y las IP fijas
//! (reservas DHCP) creadas, borradas y deshechas como grupos de cambios,
//! con la validación de una reserva (`validar_reserva`) igual en el
//! servidor y en la página.

use crate::comun::{
    Panel, PanelPrueba, aplicar_ajuste, esperar_libre, llamar, ok, pagina, sesion, variante,
};
use panel::ajustes_router::{
    DispositivoVista, ErrorRegla, Protocolo, ReglaPuertos, destino_inexistente, validar_regla,
};
use panel::ajustes_router::{ErrorReserva, validar_reserva};
use panel::ajustes_router::{PeticionAjuste, valida_la_reserva};
use serde_json::{Value, json};
use std::net::Ipv4Addr;

mod pagina;
mod puertos;
mod reservas;
mod validar;
mod validar_puertos;

// ------------------------------------------------------ validar_reserva

/// La red de casa de fábrica: router 192.168.1.1/24.
const LAN: (Ipv4Addr, Ipv4Addr) = (
    Ipv4Addr::new(192, 168, 1, 1),
    Ipv4Addr::new(255, 255, 255, 0),
);

fn existentes() -> Vec<(String, String, String)> {
    [
        ("0", "02:00:00:00:00:04", "192.168.1.68"),
        ("1", "02:00:00:00:00:01", "192.168.1.69"),
    ]
    .iter()
    .map(|(i, m, ip)| ((*i).to_owned(), (*m).to_owned(), (*ip).to_owned()))
    .collect()
}

fn ip(s: &str) -> Ipv4Addr {
    s.parse().unwrap()
}

// ------------------------------------------------------- en el simulador

async fn dispositivos(p: &Panel, cookie: &str) -> Vec<Value> {
    ok(
        p,
        "leer_tema",
        json!({ "tema": "red_local", "fresca": true }),
        cookie,
    )
    .await["red"]["dispositivos"]
        .as_array()
        .unwrap()
        .clone()
}

fn por_mac<'a>(lista: &'a [Value], mac: &str) -> &'a Value {
    lista
        .iter()
        .find(|d| d["mac"] == mac)
        .unwrap_or_else(|| panic!("falta {mac}: {lista:?}"))
}

/// Reservas de `leer_tema` como triples `(id, mac, ip)`.
async fn reservas(p: &Panel, cookie: &str) -> Vec<(String, String, String)> {
    let tema = ok(p, "leer_tema", json!({ "tema": "red_local" }), cookie).await;
    let a = crate::comun::ajustes_de(&tema)
        .into_iter()
        .find(|a| a["id"] == "red_local.reservas")
        .unwrap()
        .clone();
    assert_eq!(a["estado"], "Leido", "{a}");
    let valor = |i: &Value, campo: &str| {
        let c = i["campos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == campo)
            .unwrap();
        c["valor"]["Texto"].as_str().unwrap_or_default().to_owned()
    };
    a["instancias"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["id"].as_str().unwrap().to_owned(),
                valor(i, "mac"),
                valor(i, "ip"),
            )
        })
        .collect()
}

/// `preparar_ajuste` de las reservas: estado y cuerpo. Al editar o borrar,
/// envía como la página la MAC que tiene ahora la reserva en el router (si
/// existe).
async fn preparar(
    p: &Panel,
    cookie: &str,
    instancia: Option<&str>,
    borrar: bool,
    valores: Value,
) -> (u16, String) {
    let mut esperados = json!({});
    if let Some(id) = instancia {
        let actuales = hgu::read::static_leases(&p.state.hgu).await.unwrap();
        if let Some(r) = actuales.iter().find(|r| r.id == id) {
            esperados = json!({ "mac": r.mac });
        }
    }
    llamar(
        p,
        "preparar_ajuste",
        json!({ "p": {
            "ajuste": "red_local.reservas",
            "instancia": instancia,
            "borrar": borrar,
            "valores": valores,
            "esperados": esperados,
        } }),
        Some(cookie),
    )
    .await
}

/// Deshace el cambio `id` y lo aplica.
async fn deshacer(p: &Panel, cookie: &str, id: i64) {
    let g = ok(p, "deshacer_ajuste", json!({ "cambio": id }), cookie).await;
    aplicar_ajuste(p, cookie, &g).await;
}
