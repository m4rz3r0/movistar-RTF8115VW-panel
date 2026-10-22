//! Preparación del e2e (no es parte del panel): con la configuración `RP_*`
//! del panel que va a arrancar (`e2e/arrancar.sh`), toma la foto de partida
//! (como el asistente de instalación) y después aplica contra el
//! simulador un cambio de dos pasos (un grupo: UPnP y la IP de una reserva
//! DHCP) por el gestor de cambios, con una vigilancia corta, y espera a que
//! quede aplicado. Así el Historial tiene un grupo que mostrar. Después,
//! como desde Ajustes, cambia la contraseña de la red WiFi de 5 GHz a
//! [`CLAVE_5`]: el Historial tiene un paso con «Mostrar el valor anterior» y
//! Ajustes, una clave conocida que mostrar. Así «Cómo estaba al instalar»
//! tiene diferencias (UPnP y esa clave).
//!
//! Con el argumento `foto`, solo toma otra foto de partida (sin
//! diferencias desde ese momento), para ver la página así.
//!
//! `cargo build -p panel --features ssr --example e2e_grupo`

use leptos::prelude::LeptosOptions;
use panel::{
    changes::{ChangeManager, ChangeRequest, Estado, GrupoRequest, Paso, Tiempos},
    config::Config,
    fotos::{self, TipoFoto},
    instancia,
    state::AppState,
};
use std::{collections::BTreeMap, process::ExitCode, sync::Arc, time::Duration};

fn paso(funcion: &str, instancia: Option<&str>, valores: &[(&str, &str)]) -> Paso {
    Paso {
        req: ChangeRequest {
            funcion: funcion.into(),
            instancia: instancia.map(str::to_owned),
            valores: valores
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect::<BTreeMap<_, _>>(),
            borrar: false,
            identidad: Default::default(),
        },
        etiqueta: None,
    }
}

/// Contraseña de la red WiFi de 5 GHz tras la preparación (neutra).
const CLAVE_5: &str = "clave-de-prueba-1";

/// Prepara y confirma `grupo`, y espera a que quede aplicado; su id.
async fn aplicar_grupo(gestor: &Arc<ChangeManager>, grupo: GrupoRequest) -> Result<i64, String> {
    let prep = gestor
        .preparar_grupo(grupo)
        .await
        .map_err(|e| format!("preparar: {e:?}"))?;
    let id = gestor
        .confirmar_grupo(&prep.token, &[], "e2e")
        .await
        .map_err(|e| format!("confirmar: {e:?}"))?;
    for _ in 0..200 {
        let c = gestor.cambio(id).await.ok_or("el cambio no existe")?;
        match c.estado {
            Estado::Aplicado => return Ok(id),
            Estado::Preparado | Estado::Aplicando | Estado::Vigilando => {}
            e => return Err(format!("el cambio {id} acabó {e:?}: {:?}", c.error)),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("el cambio {id} no terminó en 20 s"))
}

/// Toma la foto de partida como el asistente (la misma lectura); su id.
async fn tomar_foto(state: &AppState) -> Result<i64, String> {
    let foto = fotos::leer_partida(state).await;
    fotos::guardar(
        &state.db,
        &state.history_key,
        TipoFoto::Partida,
        &foto.contenido,
    )
    .await
}

async fn estado() -> Result<AppState, String> {
    let mut cfg = Config::from_env()?;
    // Contra el simulador no hace falta la pausa entre peticiones que el
    // panel respeta con el router real (150 ms, para no saturarlo): la foto
    // de partida son unas 60 lecturas, y cada panel instalado del e2e la
    // toma al arrancar (en serie: Playwright levanta los servidores uno tras
    // otro).
    cfg.router_min_gap = Duration::ZERO;
    let leptos = LeptosOptions::builder().output_name("panel").build();
    let state = AppState::new(cfg, leptos).await?;
    instancia::arrancar_router(&state).await?;
    Ok(state)
}

async fn aplicar() -> Result<i64, String> {
    let state = estado().await?;
    tomar_foto(&state).await?;
    let gestor = ChangeManager::new(
        state,
        Tiempos {
            bajo: Duration::from_millis(500),
            medio: Duration::from_millis(500),
            alto: Duration::from_secs(1),
            intervalo: Duration::from_millis(100),
            fallos_para_revertir: 3,
            pausa_max: Duration::from_secs(2),
        },
    );
    // Como al arrancar el panel: hasta retomar lo pendiente no acepta cambios.
    gestor.reanudar_pendientes().await;
    let grupo = GrupoRequest {
        pasos: vec![
            paso("cbSetUpnpCfg", None, &[("adm_state", "1")]),
            paso(
                "cbDhcpV4SvrPoolSetStatic",
                Some("0"),
                &[
                    ("rtm_cfg_dhcpv4_svr_pool_id", "0"),
                    ("ipv4_addr", "192.168.1.80"),
                ],
            ),
        ],
        titulo: None,
    };
    let id = aplicar_grupo(&gestor, grupo).await?;
    // Hasta que el gestor suelte el grupo (justo tras guardarlo), no acepta
    // otro cambio.
    while gestor.activo().is_some() {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let titulo = Some("ajuste-wifi-red_5-titulo".to_owned());
    let clave = GrupoRequest {
        pasos: vec![Paso {
            etiqueta: titulo.clone(),
            ..paso(
                "cbSetWifiAccPt",
                None,
                &[("dpm_cfg_wifi_acc_pt_id", "1"), ("key_phrase", CLAVE_5)],
            )
        }],
        titulo,
    };
    aplicar_grupo(&gestor, clave).await?;
    Ok(id)
}

#[tokio::main]
async fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("foto") {
        return match async { tomar_foto(&estado().await?).await }.await {
            Ok(id) => {
                println!("foto {id} tomada");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("e2e_grupo: {e}");
                ExitCode::FAILURE
            }
        };
    }
    match aplicar().await {
        Ok(id) => {
            println!("grupo {id} aplicado");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("e2e_grupo: {e}");
            ExitCode::FAILURE
        }
    }
}
