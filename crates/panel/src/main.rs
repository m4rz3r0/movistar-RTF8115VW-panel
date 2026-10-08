//! Servidor del panel. Con argumentos, órdenes de administración
//! (`panel user add|passwd <usuario>`).

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        serve().await
    } else {
        panel::cli::run(&args).await
    }
}

#[cfg(feature = "ssr")]
async fn serve() -> std::process::ExitCode {
    use leptos::config::get_configuration;
    use panel::{build_router, config::Config, poller::Poller, state::AppState};
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
    Poller::spawn(state.clone());
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
