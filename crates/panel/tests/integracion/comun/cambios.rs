//! Gestor de cambios en las pruebas: tiempos cortos, aplicar y esperar.

use super::{PASS, Panel, PanelPrueba, USER, esperar_hasta, esperar_hasta_con, ok};
use panel::changes::{Cambio, CambioError, ChangeManager, ChangeRequest, Estado, Tiempos};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// Tiempos rápidos del gestor en las pruebas: vigilancia de 300 ms (600 ms
/// en riesgo alto) y un sondeo cada 30 ms.
pub fn tiempos_rapidos() -> Tiempos {
    Tiempos {
        bajo: Duration::from_millis(300),
        medio: Duration::from_millis(300),
        alto: Duration::from_millis(600),
        intervalo: Duration::from_millis(30),
        fallos_para_revertir: 3,
        pausa_max: Duration::from_secs(2),
    }
}

/// Vigilancia mínima del gestor: 20 ms (60 ms en riesgo alto) mirando cada
/// 5 ms.
pub fn tiempos_minimos() -> Tiempos {
    Tiempos {
        bajo: Duration::from_millis(20),
        medio: Duration::from_millis(20),
        alto: Duration::from_millis(60),
        intervalo: Duration::from_millis(5),
        fallos_para_revertir: 3,
        pausa_max: Duration::from_secs(2),
    }
}

/// Panel con un cliente del router de espera `timeout`, sin gestor.
pub async fn panel_rapido(timeout: Duration) -> Panel {
    let mut p = PanelPrueba::nuevo().arrancar().await;
    let mut cfg = hgu::ClientConfig::new(&p.sim.base_url, USER, PASS);
    cfg.timeout = timeout;
    cfg.min_gap = Duration::from_millis(1);
    p.state.hgu = hgu::Client::new(cfg);
    // Un solo cliente del router para las páginas, `/salud` y el gestor.
    p.app = panel::build_router(p.state.clone());
    p
}

/// Gestor de cambios con los tiempos `t`, ya retomados los pendientes.
pub async fn gestor(p: &Panel, t: Tiempos) -> Arc<ChangeManager> {
    let m = ChangeManager::new(p.state.clone(), t);
    m.reanudar_pendientes().await;
    m
}

/// Panel con un cliente de 500 ms y su gestor con [`tiempos_rapidos`].
pub async fn preparado() -> (Panel, Arc<ChangeManager>) {
    let p = panel_rapido(Duration::from_millis(500)).await;
    let m = gestor(&p, tiempos_rapidos()).await;
    (p, m)
}

pub fn req(funcion: &str, instancia: Option<&str>, valores: &[(&str, &str)]) -> ChangeRequest {
    ChangeRequest {
        funcion: funcion.into(),
        instancia: instancia.map(str::to_owned),
        valores: valores
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        borrar: false,
        identidad: Default::default(),
    }
}

/// Prepara y confirma; devuelve el id del cambio.
pub async fn aplicar(p: &Panel, cookie: &str, peticion: Value) -> i64 {
    let prep = ok(
        p,
        "preparar_cambio",
        json!({ "peticion": peticion }),
        cookie,
    )
    .await;
    let id = ok(
        p,
        "confirmar_cambio",
        json!({ "token": prep["token"], "palabra": null }),
        cookie,
    )
    .await;
    id.as_i64().unwrap()
}

/// Confirma el grupo de ajustes `grupo` sin desmarcar nada y espera a que
/// se aplique; devuelve el id del cambio.
pub async fn aplicar_ajuste(p: &Panel, cookie: &str, grupo: &Value) -> i64 {
    let id = ok(
        p,
        "confirmar_ajuste",
        json!({ "token": grupo["token"], "desmarcados": [] }),
        cookie,
    )
    .await
    .as_i64()
    .unwrap();
    let c = esperar_final(p, cookie, id).await;
    assert_eq!(c["estado"], "Aplicado", "{c}");
    esperar_libre(p).await;
    id
}

/// Sondea `estado_cambio` como la interfaz hasta un estado final.
pub async fn esperar_final(p: &Panel, cookie: &str, id: i64) -> Value {
    esperar_hasta_con(Duration::from_secs(10), async || {
        let c = ok(p, "estado_cambio", json!({ "id": id }), cookie).await;
        if ["Aplicado", "Revertido", "Fallido", "Cancelado"]
            .contains(&c["estado"].as_str().unwrap())
        {
            Ok(c)
        } else {
            Err(format!("sin final: {c}"))
        }
    })
    .await
}

/// Espera a que el gestor suelte el cambio activo: lo hace justo después de
/// guardarlo en su estado final.
pub async fn esperar_libre(p: &Panel) {
    let g = p.state.cambios().unwrap();
    esperar_hasta(
        Duration::from_secs(5),
        "el gestor sigue ocupado",
        async || g.activo().is_none(),
    )
    .await
}

/// Como [`esperar_libre`], con el gestor `m` (instalado o no).
pub async fn esperar_libre_gestor(m: &ChangeManager) {
    esperar_hasta(Duration::from_secs(10), "sigue activo", async || {
        m.activo().is_none()
    })
    .await
}

/// Espera a que el cambio `id` del gestor `m` llegue a `estado`.
pub async fn esperar_estado(m: &ChangeManager, id: i64, estado: Estado) -> Cambio {
    esperar_hasta_con(Duration::from_secs(10), async || {
        let c = m.cambio(id).await.expect("el cambio existe");
        if c.estado == estado {
            Ok(c)
        } else {
            Err(format!(
                "el cambio {id} sigue en {:?} (esperado {estado:?}): {:?}",
                c.estado, c.error
            ))
        }
    })
    .await
}

/// Un error o una nota guardados por el gestor, en castellano y con lo
/// técnico entre corchetes (para comprobar su contenido; las filas antiguas,
/// tal cual).
pub fn legible(guardado: &str) -> String {
    use panel::changes::Nota;
    let es = Nota::texto(guardado, i18n::Idioma::Es);
    match Nota::detalle(guardado, i18n::Idioma::Es) {
        Some(d) => format!("{es} [{d}]"),
        None => es,
    }
}

/// Confirmar con el gestor directamente: liga antes el token a `usuario`,
/// como hacen las server functions (un token sin ligar no se confirma).
#[allow(async_fn_in_trait)]
pub trait ConfirmarLigado {
    async fn confirmar_ligado(
        &self,
        token: &str,
        palabra: Option<&str>,
        usuario: &str,
    ) -> Result<i64, CambioError>;
    async fn confirmar_grupo_ligado(
        &self,
        token: &str,
        desmarcados: &[usize],
        usuario: &str,
    ) -> Result<i64, CambioError>;
}

impl ConfirmarLigado for Arc<ChangeManager> {
    async fn confirmar_ligado(
        &self,
        token: &str,
        palabra: Option<&str>,
        usuario: &str,
    ) -> Result<i64, CambioError> {
        self.ligar(token, usuario);
        self.confirmar(token, palabra, usuario).await
    }

    async fn confirmar_grupo_ligado(
        &self,
        token: &str,
        desmarcados: &[usize],
        usuario: &str,
    ) -> Result<i64, CambioError> {
        self.ligar(token, usuario);
        self.confirmar_grupo(token, desmarcados, usuario).await
    }
}
