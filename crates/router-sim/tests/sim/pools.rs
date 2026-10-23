//! Dos pools DHCP en el orden del router real: la lectura elige el pool por
//! su posición (`…_0i`), que no es su id (`…_id`), y la escritura, por su id.

use super::*;

const LECTURA: &str = "cbGetDhcpv4ServerPool";
const POSICION: &str = "rtm_cfg_dhcpv4_svr_pool_0i";
const ID: &str = "rtm_cfg_dhcpv4_svr_pool_id";

async fn en(c: &Client, posicion: &str) -> Result<XmlNode, Error> {
    c.call(LECTURA, &[(POSICION, posicion)]).await
}

/// `(id, inicio del rango)` del pool de la posición `posicion`.
async fn pool(c: &Client, posicion: &str) -> (String, String) {
    let n = en(c, posicion)
        .await
        .unwrap_or_else(|e| panic!("posición {posicion}: {e}"));
    (valor(&n, ID).unwrap(), valor(&n, "ipv4_addr_min").unwrap())
}

async fn escribir(c: &Client, id: &str, inicio: &str) {
    c.write(
        "dhcp.asp",
        "cbSetDhcpv4ServerPool",
        &[(ID, id), ("ipv4_addr_min", inicio)],
    )
    .await
    .unwrap();
}

/// Antes y después de escribir, la posición 0 es el pool de id 1 y la 1, el
/// de id 0; la 2 no existe.
#[tokio::test]
async fn la_posicion_no_es_el_id() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let p = |id: &str, inicio: &str| (id.to_owned(), inicio.to_owned());
    assert_eq!(pool(&c, "0").await, p("1", "192.168.1.230"));
    assert_eq!(pool(&c, "1").await, p("0", "192.168.1.100"));
    assert!(en(&c, "2").await.is_err(), "no hay tercera posición");

    escribir(&c, "0", "192.168.1.61").await;
    assert_eq!(pool(&c, "0").await, p("1", "192.168.1.230"));
    assert_eq!(pool(&c, "1").await, p("0", "192.168.1.61"));
    assert!(en(&c, "2").await.is_err(), "no hay tercera posición");

    escribir(&c, "1", "192.168.1.62").await;
    assert_eq!(pool(&c, "0").await, p("1", "192.168.1.62"));
    assert_eq!(pool(&c, "1").await, p("0", "192.168.1.61"));
}
