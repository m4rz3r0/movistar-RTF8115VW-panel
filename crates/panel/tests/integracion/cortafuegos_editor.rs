//! Editor completo del cortafuegos (T13b): reglas y filtros (alta, cambio y baja) por la cola de cambios, con la carga de la web, la relectura y el deshacer; sin comprobar en el router real, la 1.0 solo los enseña.

use crate::comun::{Panel, PanelPrueba, aplicar_ajuste, llamar, ok, reautenticar};
use hgu::cortafuegos::{Filtro, es_sin_ping};
use serde_json::{Value, json};

/// Texto del 409 de lo que no está comprobado en el router real.
const SIN_COMPROBAR: &str = "no se ha comprobado en este router";

fn par(k: &str, v: &str) -> (String, String) {
    (k.to_owned(), v.to_owned())
}

fn pares(p: &[(&str, &str)]) -> Vec<(String, String)> {
    p.iter().map(|(k, v)| par(k, v)).collect()
}

/// Las escrituras del cortafuegos recibidas por el simulador, en orden.
fn escrituras(p: &Panel) -> Vec<Vec<(String, String)>> {
    p.sim
        .control
        .writes()
        .into_iter()
        .filter(|(f, _)| f == "scbidirectionalflt.cmd")
        .map(|(_, ps)| ps)
        .collect()
}

/// La última escritura del cortafuegos.
fn ultima(p: &Panel) -> Vec<(String, String)> {
    escrituras(p).pop().expect("una escritura del cortafuegos")
}

fn filtros(p: &Panel) -> Vec<Filtro> {
    p.sim.control.cortafuegos()
}

/// Prepara `peticion` en el editor y la aplica; devuelve el id del cambio.
async fn aplicar(p: &Panel, cookie: &str, peticion: Value) -> i64 {
    let g = ok(p, "preparar_cortafuegos", json!({ "p": peticion }), cookie).await;
    aplicar_ajuste(p, cookie, &g).await
}

/// Deshace el cambio `id` y lo aplica.
async fn deshacer(p: &Panel, cookie: &str, id: i64) -> i64 {
    let d = ok(p, "deshacer_ajuste", json!({ "cambio": id }), cookie).await;
    aplicar_ajuste(p, cookie, &d).await
}

/// Lo que lee el editor.
async fn editor(p: &Panel, cookie: &str) -> Value {
    ok(p, "leer_cortafuegos", json!({}), cookie).await
}

/// La regla `rid` del editor en forma de formulario.
async fn regla_del_editor(p: &Panel, cookie: &str, rid: &str) -> Value {
    let e = editor(p, cookie).await;
    e["filtros"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|f| f["reglas"].as_array().unwrap().clone())
        .find(|r| r["rid"] == rid)
        .unwrap_or_else(|| panic!("sin la regla {rid}: {e}"))
}

/// Una regla TCP que deja pasar el 8080 desde una red de documentación.
fn regla_8080() -> Value {
    json!({
        "version": "0", "enabled": "1", "protocol": "2", "specificAction": "2",
        "srcaddr": "192.0.2.0", "srcmask": "255.255.255.0",
        "dstportstart": "8080"
    })
}

/// Las operaciones del editor, una petición de cada (para la 1.0).
fn las_seis() -> Vec<Value> {
    let antes_filtro = json!({ "name": "IPv6WanIn", "interface": "3.-1", "type": "0",
        "version": "1", "default": "0" });
    vec![
        json!({ "que": "regla_nueva", "filtro": "0", "valores": regla_8080() }),
        json!({ "que": "regla_cambiada", "filtro": "0", "regla": "0",
            "valores": regla_8080(), "antes": {} }),
        json!({ "que": "regla_borrada", "filtro": "0", "regla": "0", "antes": {} }),
        json!({ "que": "filtro_nuevo", "valores": { "name": "Prueba", "interface": "0.2",
            "type": "0", "version": "0", "default": "1" } }),
        json!({ "que": "filtro_cambiado", "filtro": "1", "valores": antes_filtro,
            "antes": {} }),
        json!({ "que": "filtro_borrado", "filtro": "1", "antes": {} }),
    ]
}

/// Regla nueva: la carga de la web (`escape=true`, lo que no aplica no
/// viaja), al final de su filtro, comprobada por relectura; el deshacer la
/// quita con `item=<FID>|<RID>`.
#[tokio::test]
async fn regla_nueva_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let antes = filtros(&p)[0].reglas.len();
    let id = aplicar(
        &p,
        &cookie,
        json!({ "que": "regla_nueva", "filtro": "0", "valores": regla_8080() }),
    )
    .await;
    assert_eq!(
        ultima(&p),
        pares(&[
            ("action", "add"),
            ("escape", "true"),
            ("target", "rule"),
            ("belong", "0"),
            ("version", "0"),
            ("enabled", "1"),
            ("protocol", "2"),
            ("specificAction", "2"),
            ("srcaddr", "192.0.2.0"),
            ("srcmask", "255.255.255.0"),
            ("dstportstart", "8080"),
            ("dstportend", "8080"),
        ])
    );
    let f = filtros(&p);
    assert_eq!(f[0].reglas.len(), antes + 1);
    let nueva = f[0].reglas.last().unwrap().clone();
    assert_eq!(nueva.dest_start_port, "8080");
    let (estado, instancia): (String, Option<String>) =
        sqlx::query_as("SELECT estado, instancia FROM changes WHERE id = ?")
            .bind(id)
            .fetch_one(&p.state.db)
            .await
            .unwrap();
    assert_eq!(estado, "aplicado");
    assert_eq!(instancia.as_deref(), Some(nueva.rid.as_str()));
    deshacer(&p, &cookie, id).await;
    assert_eq!(
        ultima(&p),
        pares(&[
            ("action", "remove"),
            ("item", &format!("0|{}", nueva.rid)),
            ("escape", "true"),
        ])
    );
    assert_eq!(filtros(&p)[0].reglas.len(), antes);
}

/// Regla cambiada: la regla entera con su `id` (sin `escape`); el deshacer
/// vuelve a escribir la de antes.
#[tokio::test]
async fn regla_cambiada_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let r0 = regla_del_editor(&p, &cookie, "0").await;
    assert_eq!(r0["reescribible"], true, "{r0}");
    let original = filtros(&p)[0].reglas[0].clone();
    let mut valores = r0["valores"].clone();
    valores["enabled"] = json!("0");
    let id = aplicar(
        &p,
        &cookie,
        json!({ "que": "regla_cambiada", "filtro": "0", "regla": "0",
            "valores": valores, "antes": r0["valores"] }),
    )
    .await;
    assert_eq!(
        ultima(&p),
        pares(&[
            ("action", "modify"),
            ("id", "0"),
            ("target", "rule"),
            ("belong", "0"),
            ("version", "0"),
            ("enabled", "0"),
            ("protocol", "2"),
            ("specificAction", "2"),
            ("srcaddr", "192.0.2.100"),
            ("srcmask", "255.255.255.240"),
            ("dstportstart", "22"),
            ("dstportend", "22"),
        ])
    );
    assert_eq!(filtros(&p)[0].reglas[0].enable, "0");
    deshacer(&p, &cookie, id).await;
    assert!(ultima(&p).contains(&par("enabled", "1")));
    assert_eq!(filtros(&p)[0].reglas[0], original);
}

/// Si la regla ya no es la que enseñaba la página, 409 y sin escribir.
#[tokio::test]
async fn regla_cambiada_en_el_router_no_se_escribe() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let r0 = regla_del_editor(&p, &cookie, "0").await;
    let mut otra = filtros(&p)[0].reglas[0].clone();
    otra.dest_start_port = "23".into();
    otra.dest_end_port = "23".into();
    p.sim.control.cortafuegos_sustituir("0", "0", otra);
    let (status, body) = llamar(
        &p,
        "preparar_cortafuegos",
        json!({ "p": { "que": "regla_borrada", "filtro": "0", "regla": "0",
            "antes": r0["valores"] } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(escrituras(&p).is_empty());
}

/// Regla borrada: `item=<FID>|<RID>&escape=true`, solo esa regla; el
/// deshacer la vuelve a crear con sus valores (al final de su filtro).
#[tokio::test]
async fn regla_borrada_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let r0 = regla_del_editor(&p, &cookie, "0").await;
    let antes = filtros(&p)[0].reglas.len();
    let id = aplicar(
        &p,
        &cookie,
        json!({ "que": "regla_borrada", "filtro": "0", "regla": "0", "antes": r0["valores"] }),
    )
    .await;
    assert_eq!(
        ultima(&p),
        pares(&[("action", "remove"), ("item", "0|0"), ("escape", "true")])
    );
    let f = filtros(&p);
    assert_eq!(f[0].reglas.len(), antes - 1);
    assert!(f[0].reglas.iter().all(|r| r.rid != "0"));
    deshacer(&p, &cookie, id).await;
    let w = ultima(&p);
    assert_eq!(
        w[..3],
        pares(&[("action", "add"), ("escape", "true"), ("target", "rule")])
    );
    assert!(w.contains(&par("dstportstart", "22")), "{w:?}");
    assert!(w.contains(&par("srcaddr", "192.0.2.100")), "{w:?}");
    assert!(
        !w.iter().any(|(k, _)| k == "icmpType" || k == "rejectType"),
        "{w:?}"
    );
    let f = filtros(&p);
    assert_eq!(f[0].reglas.len(), antes);
    let vuelta = f[0].reglas.last().unwrap();
    assert_eq!(
        (vuelta.orig_addr.as_str(), vuelta.dest_start_port.as_str()),
        ("192.0.2.100", "22")
    );
}

/// Filtro nuevo: `action=add&target=entry&…` sin `escape`; el deshacer lo
/// borra con `item=<FID>` (está vacío).
#[tokio::test]
async fn filtro_nuevo_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let id = aplicar(
        &p,
        &cookie,
        json!({ "que": "filtro_nuevo", "valores": { "name": "Prueba", "interface": "0.2",
            "type": "0", "version": "0", "default": "1" } }),
    )
    .await;
    assert_eq!(
        ultima(&p),
        pares(&[
            ("action", "add"),
            ("target", "entry"),
            ("name", "Prueba"),
            ("interface", "0.2"),
            ("type", "0"),
            ("version", "0"),
            ("default", "1"),
        ])
    );
    let f = filtros(&p);
    assert_eq!(f.len(), 3);
    assert_eq!((f[2].fid.as_str(), f[2].name.as_str()), ("2", "Prueba"));
    deshacer(&p, &cookie, id).await;
    assert_eq!(
        ultima(&p),
        pares(&[("action", "remove"), ("item", "2"), ("escape", "true")])
    );
    assert_eq!(filtros(&p).len(), 2);
}

/// Filtro cambiado: el filtro entero con su `id`; el deshacer lo deja como
/// estaba. Con reglas, no cambia de versión de IP.
#[tokio::test]
async fn filtro_cambiado_ida_y_vuelta() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let e = editor(&p, &cookie).await;
    let f1 = &e["filtros"][1];
    assert_eq!(f1["reescribible"], true, "{f1}");
    let mut valores = f1["valores"].clone();
    valores["default"] = json!("1");
    let id = aplicar(
        &p,
        &cookie,
        json!({ "que": "filtro_cambiado", "filtro": "1", "valores": valores,
            "antes": f1["valores"] }),
    )
    .await;
    assert_eq!(
        ultima(&p),
        pares(&[
            ("action", "modify"),
            ("id", "1"),
            ("target", "entry"),
            ("name", "IPv6WanIn"),
            ("interface", "3.-1"),
            ("type", "0"),
            ("version", "1"),
            ("default", "1"),
        ])
    );
    assert_eq!(filtros(&p)[1].default, "1");
    deshacer(&p, &cookie, id).await;
    assert_eq!(filtros(&p)[1].default, "0");
    let n = escrituras(&p).len();
    let mut v4 = f1["valores"].clone();
    v4["version"] = json!("0");
    let (status, body) = llamar(
        &p,
        "preparar_cortafuegos",
        json!({ "p": { "que": "filtro_cambiado", "filtro": "1", "valores": v4,
            "antes": f1["valores"] } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert_eq!(escrituras(&p).len(), n);
}

/// Filtro borrado: solo uno vacío (`item=<FID>&escape=true`), y el deshacer
/// lo vuelve a crear; con reglas, 409 sin escribir.
#[tokio::test]
async fn filtro_borrado_solo_vacio() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let (status, body) = llamar(
        &p,
        "preparar_cortafuegos",
        json!({ "p": { "que": "filtro_borrado", "filtro": "1", "antes": {} } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 409, "{body}");
    assert!(escrituras(&p).is_empty());
    aplicar(
        &p,
        &cookie,
        json!({ "que": "filtro_nuevo", "valores": { "name": "Vacio", "interface": "3.-1",
            "type": "1", "version": "1", "default": "0" } }),
    )
    .await;
    let e = editor(&p, &cookie).await;
    let vacio = e["filtros"][2].clone();
    assert_eq!(vacio["fid"], "2", "{e}");
    let id = aplicar(
        &p,
        &cookie,
        json!({ "que": "filtro_borrado", "filtro": "2", "antes": vacio["valores"] }),
    )
    .await;
    assert_eq!(
        ultima(&p),
        pares(&[("action", "remove"), ("item", "2"), ("escape", "true")])
    );
    assert_eq!(filtros(&p).len(), 2);
    deshacer(&p, &cookie, id).await;
    let f = filtros(&p);
    assert_eq!(f.len(), 3);
    assert_eq!(
        (
            f[2].name.as_str(),
            f[2].tipo.as_str(),
            f[2].version.as_str()
        ),
        ("Vacio", "1", "1")
    );
}

/// Un campo que no vale (ICMP con puertos): 400 que dice cuál, sin
/// escribir.
#[tokio::test]
async fn campo_que_no_vale() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let mut v = regla_8080();
    v["protocol"] = json!("1");
    v["icmpType"] = json!("25");
    let (status, body) = llamar(
        &p,
        "preparar_cortafuegos",
        json!({ "p": { "que": "regla_nueva", "filtro": "0", "valores": v } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("Puerto de destino desde: el valor no vale."),
        "{body}"
    );
    assert!(escrituras(&p).is_empty());
}

/// En las pruebas contra router-sim todo se puede; en la 1.0 (sin
/// comprobar en el router real), el editor enseña cada operación con su
/// motivo y el servidor no escribe ninguna.
#[tokio::test]
async fn la_1_0_solo_lo_ensena() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let e = editor(&p, &cookie).await;
    for (k, v) in e["bloqueos"].as_object().unwrap() {
        assert!(v.is_null(), "{k}: {v}");
    }
    assert_eq!(e["filtros"].as_array().unwrap().len(), 2);
    assert!(!e["icmp"][0].as_array().unwrap().is_empty());

    // Fijas como no comprobadas: la compilación de pruebas las habilita.
    let (p, cookie) = PanelPrueba::nuevo()
        .config(|c| c.habilitar_todo = false)
        .sin_comprobar("scbidirectionalflt.cmd?action=add.regla")
        .sin_comprobar("scbidirectionalflt.cmd?action=modify")
        .sin_comprobar("scbidirectionalflt.cmd?action=add.filtro")
        .sin_comprobar("scbidirectionalflt.cmd?action=modify.filtro")
        .sin_comprobar("scbidirectionalflt.cmd?action=remove.filtro")
        .arrancar_con_gestor()
        .await;
    reautenticar(&p, &cookie).await;
    let e = editor(&p, &cookie).await;
    for (k, v) in e["bloqueos"].as_object().unwrap() {
        assert!(
            v.as_str().is_some_and(|m| m.contains(SIN_COMPROBAR)),
            "{k}: {v}"
        );
    }
    for peticion in las_seis() {
        let (status, body) = llamar(
            &p,
            "preparar_cortafuegos",
            json!({ "p": peticion }),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, 409, "{peticion}: {body}");
        assert!(body.contains(SIN_COMPROBAR), "{peticion}: {body}");
    }
    assert!(escrituras(&p).is_empty(), "{:?}", escrituras(&p));
}

/// «No responder al ping» sigue igual con el editor: su carga comprobada
/// (sin `escape`), su resumen de sí/no y su deshacer, también tras un
/// cambio del editor.
#[tokio::test]
async fn sin_ping_igual_con_el_editor() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    aplicar(
        &p,
        &cookie,
        json!({ "que": "regla_nueva", "filtro": "0", "valores": regla_8080() }),
    )
    .await;
    let g = ok(&p, "preparar_sin_ping", json!({}), &cookie).await;
    assert_eq!(
        g["pasos"][0]["campos"],
        json!([{ "campo": "ajuste-seguridad-cortafuegos-sin-ping", "antes": "false",
            "despues": "true", "secreto": false, "tipo": "booleano" }]),
        "{g}"
    );
    let id = aplicar_ajuste(&p, &cookie, &g).await;
    let mut alta = ultima(&p);
    alta.sort();
    assert_eq!(
        alta,
        pares(&[
            ("action", "add"),
            ("belong", "0"),
            ("enabled", "1"),
            ("icmpType", "25"),
            ("protocol", "1"),
            ("specificAction", "0"),
            ("target", "rule"),
            ("version", "0"),
        ])
    );
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    assert_eq!(
        d["pasos"][0]["campos"][0]["campo"], "ajuste-seguridad-cortafuegos-sin-ping",
        "{d}"
    );
    aplicar_ajuste(&p, &cookie, &d).await;
    assert!(!filtros(&p)[0].reglas.iter().any(es_sin_ping));
}
