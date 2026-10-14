//! Ida y vuelta de cada ajuste del catálogo real contra `router-sim`: para
//! cada campo editable, un valor distinto del actual se prepara, se aplica,
//! se comprueba en la lectura del simulador, se deshace y se comprueba que
//! vuelve el valor original. Todo por las server functions, como la página.

mod common;

use ajustes::{Ajuste, Campo, CatalogoAjustes, TipoCampo};
use common::{Panel, esperar_final, llamar, ok, panel_con_gestor};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

/// Valor legible actual del campo `campo` del ajuste `ajuste` según
/// `leer_tema` (`{"Texto": …}`, `{"Opcion": …}`…).
async fn legible(p: &Panel, cookie: &str, a: &Ajuste, campo: &str) -> Value {
    let tema = ok(p, "leer_tema", json!({ "tema": a.tema.slug() }), cookie).await;
    let ajuste = tema["ajustes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == a.id.as_str())
        .unwrap_or_else(|| panic!("{}: falta en leer_tema", a.id));
    assert_eq!(ajuste["estado"], "Leido", "{}: {ajuste}", a.id);
    ajuste["campos"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == campo)
        .unwrap_or_else(|| panic!("{}.{campo}: falta en leer_tema", a.id))["valor"]
        .clone()
}

/// Valor tal cual lo da el simulador del elemento `lee` de la lectura del
/// ajuste (para los secretos, que `leer_tema` no enseña).
async fn crudo(p: &Panel, a: &Ajuste, lee: &str) -> String {
    let params: Vec<(&str, &str)> = a
        .leer
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let n = p.state.hgu.call(&a.leer.funcion, &params).await.unwrap();
    elemento(&n, lee).unwrap_or_else(|| panic!("{}: el simulador no da {lee}", a.id))
}

fn elemento(n: &hgu::xml::XmlNode, nombre: &str) -> Option<String> {
    n.children.iter().find_map(|c| {
        if c.children.is_empty() {
            c.tag.eq_ignore_ascii_case(nombre).then(|| c.text.clone())
        } else {
            elemento(c, nombre)
        }
    })
}

/// Texto `base` ajustado a los límites en bytes del campo.
fn dentro_del_limite(c: &Campo, base: String) -> String {
    let (min, max) = c.validar.map_or((None, None), |v| (v.min, v.max));
    let mut t = base;
    if let Some(max) = max {
        t.truncate(max as usize);
    }
    while min.is_some_and(|m| (t.len() as i64) < m) {
        t.push('x');
    }
    t
}

/// Un valor legible distinto de `actual` para el campo `c` (`n` lo hace
/// único) y lo que `leer_tema` debe mostrar tras escribirlo (`None` para un
/// secreto, que se comprueba en crudo).
fn otro_valor(c: &Campo, actual: &Value, n: u32) -> (String, Option<Value>) {
    match c.tipo {
        TipoCampo::Opcion => {
            let o = c
                .opciones
                .iter()
                .find(|o| actual["Opcion"] != o.clave.as_str())
                .expect("un campo opción con una sola opción no se puede cambiar");
            (o.clave.clone(), Some(json!({ "Opcion": o.clave })))
        }
        TipoCampo::Booleano => {
            let nuevo = actual["Booleano"] != true;
            (nuevo.to_string(), Some(json!({ "Booleano": nuevo })))
        }
        TipoCampo::Numero => {
            let (min, max) = c.validar.map_or((None, None), |v| (v.min, v.max));
            let actual: Option<i64> = actual["Texto"].as_str().and_then(|t| t.parse().ok());
            let base = min.unwrap_or(1);
            let nuevo = [base, base + 1, max.unwrap_or(base + 2)]
                .into_iter()
                .find(|v| Some(*v) != actual && max.is_none_or(|m| *v <= m))
                .unwrap();
            (
                nuevo.to_string(),
                Some(json!({ "Texto": nuevo.to_string() })),
            )
        }
        TipoCampo::Texto => {
            let t = dentro_del_limite(c, format!("prueba-{n}"));
            (t.clone(), Some(json!({ "Texto": t })))
        }
        TipoCampo::Ip => {
            let t = format!("192.0.2.{n}");
            (t.clone(), Some(json!({ "Texto": t })))
        }
        TipoCampo::Mac => {
            let t = format!("02:00:00:00:00:{n:02x}");
            (t.clone(), Some(json!({ "Texto": t })))
        }
        TipoCampo::Secreto => (dentro_del_limite(c, format!("clave-de-prueba-{n}")), None),
    }
}

/// Aplica el resumen `grupo` y espera a que quede aplicado; devuelve el id
/// del cambio.
async fn aplicar(p: &Panel, cookie: &str, grupo: &Value, que: &str) -> i64 {
    let id = ok(
        p,
        "confirmar_ajuste",
        json!({ "token": grupo["token"], "desmarcados": [] }),
        cookie,
    )
    .await;
    let id = id.as_i64().unwrap();
    let c = esperar_final(p, cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{que}: {c}");
    // El gestor suelta el cambio justo después de guardarlo como aplicado.
    let g = p.state.cambios().unwrap();
    let inicio = Instant::now();
    while g.activo().is_some() {
        assert!(
            inicio.elapsed() < Duration::from_secs(5),
            "{que}: sigue activo"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    id
}

/// Ajustes del catálogo que hoy no se pueden guardar, con el motivo. La
/// prueba exige que sigan sin poder guardarse: cuando se arregle uno, hay
/// que quitarlo de aquí para que haga la ida y vuelta.
const SIN_GUARDAR: &[(&str, &str)] = &[(
    "puertos.alg",
    "su fijo set_nf_conntrack no sale en la lectura del router, y el gestor no \
     escribe un parámetro cuyo valor anterior no conoce (no podría deshacerlo)",
)];

/// El ajuste `a` sigue sin poder guardarse: preparar su primer campo
/// editable da 400 y no escribe nada.
async fn sigue_sin_guardarse(p: &Panel, cookie: &str, a: &Ajuste, motivo: &str) {
    let c = a.campos.iter().find(|c| c.escribible().is_some()).unwrap();
    let antes = legible(p, cookie, a, &c.id).await;
    let (nuevo, _) = otro_valor(c, &antes, 1);
    let escritas = p.sim.control.writes().len();
    let (status, body) = llamar(
        p,
        "preparar_ajuste",
        json!({ "p": { "ajuste": a.id, "instancia": null, "borrar": false, "valores": { c.id.as_str(): nuevo } } }),
        Some(cookie),
    )
    .await;
    assert_eq!(
        status, 400,
        "{}: ¿ya se puede guardar? Quítalo de SIN_GUARDAR ({motivo}): {body}",
        a.id
    );
    assert_eq!(p.sim.control.writes().len(), escritas, "{}", a.id);
}

#[tokio::test]
async fn cada_ajuste_antes_aplicar_verificar_deshacer() {
    let (p, cookie) = panel_con_gestor().await;
    let catalogo = CatalogoAjustes::load().unwrap();
    let mut n = 0u32;
    for a in catalogo.todos() {
        if a.lista || a.componente.is_some() {
            continue;
        }
        if let Some((_, motivo)) = SIN_GUARDAR.iter().find(|(id, _)| *id == a.id) {
            sigue_sin_guardarse(&p, &cookie, a, motivo).await;
            continue;
        }
        for c in a.campos.iter().filter(|c| c.escribible().is_some()) {
            n += 1;
            let que = format!("{} / {}", a.id, c.id);
            let antes = legible(&p, &cookie, a, &c.id).await;
            let antes_crudo = crudo(&p, a, &c.lee).await;
            let (nuevo, esperado) = otro_valor(c, &antes, n);
            let (status, body) = llamar(
                &p,
                "preparar_ajuste",
                json!({ "p": { "ajuste": a.id, "instancia": null, "borrar": false, "valores": { c.id.as_str(): nuevo } } }),
                Some(&cookie),
            )
            .await;
            assert_eq!(status, 200, "{que}: preparar: {body}");
            let grupo: Value = serde_json::from_str(&body).unwrap();
            let id = aplicar(&p, &cookie, &grupo, &que).await;
            match esperado {
                Some(v) => assert_eq!(legible(&p, &cookie, a, &c.id).await, v, "{que}"),
                None => assert_eq!(crudo(&p, a, &c.lee).await, nuevo, "{que}"),
            }
            let (status, body) = llamar(
                &p,
                "deshacer_ajuste",
                json!({ "cambio": id }),
                Some(&cookie),
            )
            .await;
            assert_eq!(status, 200, "{que}: deshacer: {body}");
            let inverso: Value = serde_json::from_str(&body).unwrap();
            aplicar(&p, &cookie, &inverso, &format!("{que} (deshacer)")).await;
            assert_eq!(
                legible(&p, &cookie, a, &c.id).await,
                antes,
                "{que}: tras deshacer"
            );
            assert_eq!(
                crudo(&p, a, &c.lee).await,
                antes_crudo,
                "{que}: tras deshacer (en crudo)"
            );
        }
    }
    assert!(n > 20, "solo {n} campos: ¿se ha saltado alguno?");
}
