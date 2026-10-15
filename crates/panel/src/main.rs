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
        build_router,
        changes::{ChangeManager, Tiempos},
        config::Config,
        instancia,
        poller::Poller,
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
    let cambios = ChangeManager::instalar(&state, Tiempos::default());
    tokio::spawn(async move { cambios.reanudar_pendientes().await });
    eprintln!("panel escuchando en http://{listen}");
    let app = build_router(state).into_make_service_with_connect_info::<SocketAddr>();
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("panel: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // Sin `ssr` no hay servidor: el binario solo existe para cargo-leptos.
}
