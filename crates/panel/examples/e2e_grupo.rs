//! Preparación del e2e (no es parte del panel): con la configuración `RP_*`
//! del panel que va a arrancar (`e2e/arrancar.sh`), aplica contra el
//! simulador un cambio de dos pasos (un grupo: UPnP y la IP de una reserva
//! DHCP) por el gestor de cambios, con una vigilancia corta, y espera a que
//! quede aplicado. Así el Historial tiene un grupo que mostrar. En la fase 1
//! los grupos no tienen aún interfaz propia.
//!
//! `cargo build -p panel --features ssr --example e2e_grupo`

use leptos::prelude::LeptosOptions;
use panel::{
    changes::{ChangeManager, ChangeRequest, Estado, GrupoRequest, Paso, Tiempos},
    config::Config,
    instancia,
    state::AppState,
};
use std::{collections::BTreeMap, process::ExitCode, time::Duration};

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
        },
        etiqueta: None,
    }
}

async fn aplicar() -> Result<i64, String> {
    let cfg = Config::from_env()?;
    let leptos = LeptosOptions::builder().output_name("panel").build();
    let state = AppState::new(cfg, leptos).await?;
    instancia::arrancar_router(&state).await?;
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
            e => return Err(format!("el grupo acabó {e:?}: {:?}", c.error)),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("el grupo no terminó en 20 s".into())
}

#[tokio::main]
async fn main() -> ExitCode {
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
