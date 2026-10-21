//! Servidor del panel. Con argumentos, órdenes de administración
//! (`panel user add|passwd <usuario>`).

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    init_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        serve().await
    } else {
        panel::cli::run(&args).await
    }
}

/// Registros a stderr; nivel según `RP_LOG` (sintaxis de `EnvFilter`), por
/// defecto `info`.
#[cfg(feature = "ssr")]
fn init_logging() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("RP_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(feature = "ssr")]
async fn serve() -> std::process::ExitCode {
    use leptos::config::get_configuration;
    use panel::{
        build_router, changes::ChangeManager, config::Config, instancia, poller::Poller,
        state::AppState,
    };
    use std::{net::SocketAddr, process::ExitCode};

    let cfg = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("panel: configuración no válida: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut leptos = match get_configuration(None) {
        Ok(conf) => conf.leptos_options,
        Err(e) => {
            eprintln!("panel: configuración de Leptos no válida: {e}");
            return ExitCode::FAILURE;
        }
    };
    // `RP_LISTEN` manda sobre `site-addr` de cargo-leptos.
    leptos.site_addr = cfg.listen;
    panel::estaticos::configurar_hash(&mut leptos);
    let listen = cfg.listen;
    let state = match AppState::new(cfg, leptos).await {
        Ok(state) => state,
        Err(e) => {
            eprintln!("panel: no se puede arrancar: {e}");
            return ExitCode::FAILURE;
        }
    };
    let listener = match tokio::net::TcpListener::bind(listen).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("panel: no se puede escuchar en {listen}: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = instancia::arrancar_router(&state).await {
        eprintln!("panel: no se puede arrancar: {e}");
        return ExitCode::FAILURE;
    }
    // Sin usuarios: token del asistente de instalación (registro y fichero).
    panel::instalar::token_inicial(&state).await;
    Poller::spawn(state.clone());
    // Retoma los cambios que quedaron a medias si el proceso se cortó (en
    // segundo plano: si el router no responde, no retrasa el arranque).
    let tiempos = tiempos_del_gestor(
        std::env::var("RP_E2E_VIGILANCIA_S").ok().as_deref(),
        cfg!(debug_assertions),
    );
    let cambios = ChangeManager::instalar(&state, tiempos);
    tokio::spawn(async move { cambios.reanudar_pendientes().await });
    eprintln!("panel escuchando en http://{listen}");
    let app = build_router(state).into_make_service_with_connect_info::<SocketAddr>();
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("panel: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Tiempos del gestor de cambios: los de producción, salvo en el binario de
/// depuración con `RP_E2E_VIGILANCIA_S` (segundos, mayor que cero), que usan
/// las pruebas de navegador para no esperar 30 s o 2 min en cada cambio:
/// riesgo bajo y medio se vigilan N s, el alto 2N s, comprobando cada
/// segundo. `alto` es también la espera máxima a que el router vuelva tras
/// un reinicio y el plazo para conseguir revertir (`Tiempos::alto`): con la
/// variable, esos dos plazos pasan igualmente a 2N s. En release
/// (`depuracion` falso) la variable no hace nada; lo comprueba
/// `deploy/pruebas/release-local.sh`.
#[cfg(feature = "ssr")]
fn tiempos_del_gestor(vigilancia_e2e: Option<&str>, depuracion: bool) -> panel::changes::Tiempos {
    use panel::changes::Tiempos;
    use std::time::Duration;
    let Some(valor) = vigilancia_e2e.filter(|_| depuracion) else {
        return Tiempos::default();
    };
    match valor.trim().parse::<u64>() {
        Ok(n) if n > 0 => {
            eprintln!("panel: vigilancia de pruebas de {n} s (RP_E2E_VIGILANCIA_S)");
            Tiempos {
                bajo: Duration::from_secs(n),
                medio: Duration::from_secs(n),
                alto: Duration::from_secs(2 * n),
                intervalo: Duration::from_secs(1),
                ..Tiempos::default()
            }
        }
        _ => {
            eprintln!("panel: RP_E2E_VIGILANCIA_S no válida ({valor:?}): vigilancia normal");
            Tiempos::default()
        }
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::tiempos_del_gestor;
    use panel::changes::Tiempos;
    use std::time::Duration;

    /// Compara todos los campos (`Tiempos` no implementa `PartialEq`).
    fn campos(t: &Tiempos) -> (Duration, Duration, Duration, Duration, u32, Duration) {
        (
            t.bajo,
            t.medio,
            t.alto,
            t.intervalo,
            t.fallos_para_revertir,
            t.pausa_max,
        )
    }

    #[test]
    fn release_ignora_la_vigilancia_de_pruebas() {
        let normal = campos(&Tiempos::default());
        assert_eq!(campos(&tiempos_del_gestor(Some("10"), false)), normal);
        assert_eq!(campos(&tiempos_del_gestor(None, false)), normal);
    }

    #[test]
    fn depuracion_sin_variable_usa_los_tiempos_normales() {
        assert_eq!(
            campos(&tiempos_del_gestor(None, true)),
            campos(&Tiempos::default())
        );
    }

    #[test]
    fn depuracion_con_variable_acorta_la_vigilancia() {
        let t = tiempos_del_gestor(Some("10"), true);
        let normal = Tiempos::default();
        assert_eq!(t.bajo, Duration::from_secs(10));
        assert_eq!(t.medio, Duration::from_secs(10));
        assert_eq!(t.alto, Duration::from_secs(20));
        assert_eq!(t.intervalo, Duration::from_secs(1));
        assert_eq!(t.fallos_para_revertir, normal.fallos_para_revertir);
        assert_eq!(t.pausa_max, normal.pausa_max);
    }

    #[test]
    fn valor_no_valido_usa_los_tiempos_normales() {
        let normal = campos(&Tiempos::default());
        for valor in ["", "0", "-5", "diez", "1.5"] {
            assert_eq!(
                campos(&tiempos_del_gestor(Some(valor), true)),
                normal,
                "{valor:?}"
            );
        }
    }
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // Sin `ssr` no hay servidor: el binario solo existe para cargo-leptos.
}
