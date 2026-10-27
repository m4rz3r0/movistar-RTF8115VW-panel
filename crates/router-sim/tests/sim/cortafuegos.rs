//! Cortafuegos: la página de la lista, el alta, la edición y la baja de reglas y filtros, y la respuesta 301 del router.

use super::*;
use hgu::cortafuegos::{
    ALTA, BAJA, MODIFICAR, PAGINA_EDITOR, PAGINA_LISTA, carga_sin_ping, es_sin_ping, item, leer,
};

/// GET de `scbidirectionalflt.cmd` con `consulta` (sin el `sessionKey`) en
/// la sesión de `c`; devuelve el estado y la cabecera `Location`.
async fn cmd(sim: &SimHandle, c: &Client, consulta: &str) -> (u16, Option<String>) {
    // El cliente entra y deja el `sessionKey` en el simulador; la petición
    // va a mano para ver la respuesta tal cual.
    c.basic_page(PAGINA_LISTA).await.unwrap();
    let (cookie, clave) = sim.control.sesion_basica_y_clave().unwrap();
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let r = http
        .get(format!(
            "{}/cgi-bin/scbidirectionalflt.cmd?{consulta}&sessionKey={clave}",
            sim.base_url
        ))
        .header("Cookie", format!("_httpdSessionId_={cookie}"))
        .send()
        .await
        .unwrap();
    let loc = r
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    (r.status().as_u16(), loc)
}

/// Cada escritura responde 301 a la lista, como el router real.
#[tokio::test]
async fn las_escrituras_responden_301_a_la_lista() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let (st, loc) = cmd(
        &sim,
        &c,
        "action=add&target=entry&name=web&interface=0.2&type=0&version=0&default=1",
    )
    .await;
    assert_eq!(st, 301);
    assert_eq!(loc.as_deref(), Some("/scbidirectionalflt_ls.asp"));
}

/// Alta de filtro, su edición y su baja (`item=<FID>`), con las cargas de
/// la web.
#[tokio::test]
async fn alta_edicion_y_baja_de_un_filtro() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let alta = [
        ("target", "entry"),
        ("name", "test filter entr"),
        ("interface", "0.2"),
        ("type", "0"),
        ("version", "0"),
        ("default", "1"),
    ];
    c.basic_write(PAGINA_EDITOR, ALTA, &alta).await.unwrap();
    let f = leer(&c).await.unwrap();
    assert_eq!(f.len(), 3, "{f:?}");
    let nuevo = &f[2];
    assert_eq!(
        (
            nuevo.fid.as_str(),
            nuevo.name.as_str(),
            nuevo.intf_id.as_str(),
            nuevo.interface.as_str(),
            nuevo.default.as_str()
        ),
        ("2", "test filter entr", "0.2", "ppp0", "1")
    );
    assert!(nuevo.reglas.is_empty());
    let edicion = [
        ("id", "2"),
        ("target", "entry"),
        ("name", "test filter"),
        ("interface", "0.2"),
        ("type", "1"),
        ("version", "0"),
        ("default", "0"),
    ];
    c.basic_write(PAGINA_EDITOR, MODIFICAR, &edicion)
        .await
        .unwrap();
    let f = leer(&c).await.unwrap();
    assert_eq!(
        (
            f[2].name.as_str(),
            f[2].tipo.as_str(),
            f[2].default.as_str()
        ),
        ("test filter", "1", "0")
    );
    c.basic_write(PAGINA_LISTA, BAJA, &[("item", "2"), ("escape", "true")])
        .await
        .unwrap();
    assert_eq!(leer(&c).await.unwrap().len(), 2);
}

/// La edición de una regla cambia sus campos y conserva su RID; lo que no
/// viaja queda en «cualquiera».
#[tokio::test]
async fn edicion_de_una_regla() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let edicion = [
        ("id", "8"),
        ("target", "rule"),
        ("belong", "0"),
        ("version", "0"),
        ("enabled", "0"),
        ("protocol", "2"),
        ("specificAction", "1"),
        ("rejectType", "7"),
        ("srcaddr", "192.0.2.10"),
        ("srcmask", "255.255.255.0"),
        ("dstportstart", "8080"),
        ("dstportend", "8081"),
    ];
    c.basic_write(PAGINA_EDITOR, MODIFICAR, &edicion)
        .await
        .unwrap();
    let f = leer(&c).await.unwrap();
    let r = f[0].reglas.iter().find(|r| r.rid == "8").unwrap();
    assert_eq!(f[0].reglas.len(), 10);
    assert_eq!(
        (
            r.enable.as_str(),
            r.protocol.as_str(),
            r.action.as_str(),
            r.reject_type.as_str(),
            r.icmp_type.as_str()
        ),
        ("0", "2", "1", "7", "0")
    );
    assert_eq!(
        (r.orig_addr.as_str(), r.orig_mask.as_str()),
        ("192.0.2.10", "255.255.255.0")
    );
    assert_eq!(
        (r.dest_addr.as_str(), r.dest_mask.as_str()),
        ("0.0.0.0", "255.255.255.255")
    );
    assert_eq!(
        (r.dest_start_port.as_str(), r.dest_end_port.as_str()),
        ("8080", "8081")
    );
}

#[tokio::test]
async fn alta_y_baja_de_una_regla() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let filtros = leer(&c).await.unwrap();
    assert_eq!(filtros, sim.control.cortafuegos());
    assert_eq!(filtros[0].reglas.len(), 10);
    let carga = carga_sin_ping(&filtros[0]);
    let p: Vec<(&str, &str)> = carga.iter().map(|(k, v)| (*k, v.as_str())).collect();
    c.basic_write(PAGINA_EDITOR, ALTA, &p).await.unwrap();
    let f = leer(&c).await.unwrap();
    assert_eq!(f[0].reglas.len(), 11);
    // La carga comprobada en el router real: sin `escape`.
    assert_eq!(p.first(), Some(&("target", "rule")));
    let nueva = f[0].reglas.last().unwrap();
    assert_eq!(nueva.rid, "16");
    assert!(es_sin_ping(nueva), "{nueva:?}");
    let it = item("0", "16");
    c.basic_write(PAGINA_LISTA, BAJA, &[("item", &it), ("escape", "true")])
        .await
        .unwrap();
    assert_eq!(leer(&c).await.unwrap()[0].reglas.len(), 10);
    // Las dos escrituras, por la UI básica con su acción.
    let w = sim.control.writes();
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(w.iter().all(|(f, _)| f == "scbidirectionalflt.cmd"));
    assert!(
        w[1].1.contains(&("item".to_owned(), "0|16".to_owned())),
        "{w:?}"
    );
}

#[tokio::test]
async fn baja_sin_regla_borra_el_filtro_como_el_router() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.basic_write(PAGINA_LISTA, BAJA, &[("item", "0"), ("escape", "true")])
        .await
        .unwrap();
    let f = sim.control.cortafuegos();
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].fid, "1");
}

#[tokio::test]
async fn cortafuegos_ilegible_sin_var_filter() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.cortafuegos_ilegible(true);
    assert!(leer(&c).await.is_err());
    sim.control.cortafuegos_ilegible(false);
    assert_eq!(leer(&c).await.unwrap().len(), 2);
}
