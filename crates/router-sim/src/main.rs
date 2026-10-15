//! Router simulado para desarrollo.
//!
//! Uso: `router-sim [--listen 127.0.0.1:8081] [--fixtures crates/hgu/fixtures]
//! [--capa DIR] [--user U] [--password P] [--retardo-ms N]` (por defecto
//! usuario y contraseña `sim`). `--capa` añade fixtures que sustituyen a las
//! de `--fixtures` (p. ej. `crates/hgu/fixtures-variantes/dhcp-sin-voip`).
//! `--retardo-ms` retarda cada respuesta, como la latencia del router real
//! (se cambia en marcha con `POST /__sim/retardo`).

use router_sim::{Sim, SimConfig};
use std::process::ExitCode;

const USAGE: &str = "uso: router-sim [--listen DIR:PUERTO] [--fixtures DIR] [--capa DIR] [--user U] [--password P] [--retardo-ms N]";

#[tokio::main]
async fn main() -> ExitCode {
    let mut cfg = SimConfig::new("sim", "sim", "crates/hgu/fixtures");
    let mut listen = "127.0.0.1:8081".to_owned();
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let Some(value) = args.next() else {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        };
        match flag.as_str() {
            "--listen" => listen = value,
            "--fixtures" => cfg.fixtures_dir = value.into(),
            "--capa" => cfg.capa = Some(value.into()),
            "--user" => cfg.user = value,
            "--password" => cfg.password = value,
            "--retardo-ms" => match value.parse() {
                Ok(ms) => cfg.retardo = std::time::Duration::from_millis(ms),
                Err(e) => {
                    eprintln!("--retardo-ms {value}: {e}");
                    return ExitCode::from(2);
                }
            },
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    match listen.parse() {
        Ok(addr) => cfg.listen = Some(addr),
        Err(e) => {
            eprintln!("--listen {listen}: {e}");
            return ExitCode::from(2);
        }
    }
    let user = cfg.user.clone();
    let sim = Sim::start(cfg).await;
    eprintln!("router-sim escuchando en {} (usuario {user})", sim.base_url);
    std::future::pending::<ExitCode>().await
}
