//! Ida y vuelta de cada ajuste del catálogo real contra `router-sim`: para
//! cada campo editable, un valor distinto del actual se prepara, se aplica,
//! se comprueba en la lectura del simulador, se deshace y se comprueba que
//! vuelve el valor original. Todo por las server functions, como la página.

mod common;

use ajustes::{Ajuste, Campo, CatalogoAjustes, TipoCampo};
use common::{Panel, esperar_final, esperar_libre, llamar, ok, panel_con_gestor};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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
    crudo_opcional(p, a, lee)
        .await
        .unwrap_or_else(|| panic!("{}: el simulador no da {lee}", a.id))
}

/// Como [`crudo`], `None` si el simulador no da el elemento.
async fn crudo_opcional(p: &Panel, a: &Ajuste, lee: &str) -> Option<String> {
    let params: Vec<(&str, &str)> = a
        .leer
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let n = p.state.hgu.call(&a.leer.funcion, &params).await.unwrap();
    elemento(&n, lee)
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
    esperar_libre(p).await;
    id
}

/// Valor de cada campo del ajuste `a`: el legible de `leer_tema` y el
/// crudo del simulador (los secretos solo se distinguen en crudo).
async fn foto(p: &Panel, cookie: &str, a: &Ajuste) -> BTreeMap<String, (Value, Option<String>)> {
    let mut out = BTreeMap::new();
    for c in &a.campos {
        let legible = legible(p, cookie, a, &c.id).await;
        let crudo = crudo_opcional(p, a, &c.lee).await;
        out.insert(c.id.clone(), (legible, crudo));
    }
    out
}

/// Tras escribir `tocados`, el resto de campos de `a` siguen como en `antes`.
fn hermanos_iguales(
    que: &str,
    tocados: &[&str],
    antes: &BTreeMap<String, (Value, Option<String>)>,
    ahora: &BTreeMap<String, (Value, Option<String>)>,
) {
    for (id, valor) in antes {
        if !tocados.contains(&id.as_str()) {
            assert_eq!(&ahora[id], valor, "{que}: ha cambiado también {id}");
        }
    }
}

/// Último valor escrito del parámetro `param` en el simulador.
fn escrito(p: &Panel, param: &str) -> Option<String> {
    p.sim
        .control
        .writes()
        .into_iter()
        .rev()
        .find_map(|(_, ps)| ps.into_iter().find(|(k, _)| k == param).map(|(_, v)| v))
}

/// ¿Escribe `c` un secreto que el router da tapado al leer (la contraseña de
/// la línea de voz)? Guardar cualquier campo de su función exige escribirlo,
/// y el deshacer lo omite (no hay valor anterior de verdad): se queda el
/// escrito.
fn tapado(c: &Campo) -> bool {
    let f = catalogo::Catalogo::load().unwrap();
    c.escribible()
        .and_then(|e| {
            f.funcion(&e.funcion)
                .and_then(|func| func.parametro(&e.parametro))
        })
        .is_some_and(|p| p.tapado)
}

#[tokio::test]
async fn cada_ajuste_antes_aplicar_verificar_deshacer() {
    let (p, cookie) = panel_con_gestor().await;
    let catalogo = CatalogoAjustes::load().unwrap();
    let mut n = 0u32;
    let mut con_tapado = Vec::new();
    for a in catalogo.todos() {
        if a.lista || a.componente.is_some() {
            continue;
        }
        for c in a.campos.iter().filter(|c| c.escribible().is_some()) {
            n += 1;
            let que = format!("{} / {}", a.id, c.id);

            let todo_antes = foto(&p, &cookie, a).await;
            let antes = todo_antes[&c.id].0.clone();
            let antes_crudo = crudo(&p, a, &c.lee).await;
            let (nuevo, esperado) = otro_valor(c, &antes, n);
            let mut valores = json!({ c.id.as_str(): nuevo });
            let mut tocados = vec![c.id.as_str()];
            // La clave que el router no deja leer va con cualquier guardado
            // de su función; el deshacer no la vuelve atrás.
            let tapados: Vec<&Campo> = a.campos.iter().filter(|x| tapado(x)).collect();
            for t in &tapados {
                if t.id != c.id {
                    valores[t.id.as_str()] = json!(format!("clave-de-prueba-voz-{n}"));
                }
                tocados.push(&t.id);
                con_tapado.push(que.clone());
            }
            // Con el valor que exige otro campo (encender el espejo), ese
            // campo va también: su primera opción.
            if let Some(r) = c.requiere.as_ref().filter(|r| r.cuando == nuevo) {
                let exigido = a.campo(&r.campo).unwrap();
                valores[&r.campo] = json!(exigido.opciones[0].clave);
                tocados.push(&exigido.id);
            }
            let (status, body) = llamar(
                &p,
                "preparar_ajuste",
                json!({ "p": { "ajuste": a.id, "instancia": null, "borrar": false, "valores": valores } }),
                Some(&cookie),
            )
            .await;
            assert_eq!(status, 200, "{que}: preparar: {body}");
            let grupo: Value = serde_json::from_str(&body).unwrap();
            let id = aplicar(&p, &cookie, &grupo, &que).await;
            match esperado {
                Some(v) => assert_eq!(legible(&p, &cookie, a, &c.id).await, v, "{que}"),
                // El router (y el simulador) no deja leer uno tapado: lo que
                // cuenta es lo que se escribió.
                None if tapado(c) => assert_eq!(
                    escrito(&p, &c.escribible().unwrap().parametro).as_deref(),
                    Some(nuevo.as_str()),
                    "{que}"
                ),
                None => assert_eq!(crudo(&p, a, &c.lee).await, nuevo, "{que}"),
            }
            hermanos_iguales(&que, &tocados, &todo_antes, &foto(&p, &cookie, a).await);
            let (status, body) = llamar(
                &p,
                "deshacer_ajuste",
                json!({ "cambio": id }),
                Some(&cookie),
            )
            .await;
            if tapado(c) {
                // Solo cambió ella, y su valor anterior es el vacío que dio
                // el router: no hay nada que volver atrás (el deshacer no la
                // vacía).
                assert_eq!(status, 409, "{que}: deshacer: {body}");
                continue;
            }
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
            let mut despues = foto(&p, &cookie, a).await;
            let mut esperado = todo_antes.clone();
            for t in &tapados {
                despues.remove(&t.id);
                esperado.remove(&t.id);
            }
            assert_eq!(despues, esperado, "{que}: tras deshacer, todo el ajuste");
        }
    }
    assert!(n > 20, "solo {n} campos: ¿se ha saltado alguno?");
    // Solo la línea de voz lleva una clave tapada al leer.
    assert!(!con_tapado.is_empty(), "ninguno con la clave de la voz");
    assert!(
        con_tapado.iter().all(|q| q.starts_with("telefono.")),
        "{con_tapado:?}"
    );
}
