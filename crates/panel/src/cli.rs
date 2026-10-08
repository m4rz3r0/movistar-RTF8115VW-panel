//! Órdenes de administración del binario `panel`:
//! `panel user add <usuario>` y `panel user passwd <usuario>`.
//!
//! Solo necesitan `RP_DB`. La contraseña se pide dos veces por el terminal sin
//! eco; si la entrada estándar no es un terminal, se lee una línea de ella
//! (para scripts).

use crate::{auth, config, db};
use std::{
    io::{BufRead, IsTerminal},
    process::ExitCode,
};

const USAGE: &str = "uso: panel                         arranca el servidor
     panel user add <usuario>     da de alta un usuario del panel
     panel user passwd <usuario>  cambia la contraseña (cierra sus sesiones)";

/// Ejecuta la orden `args` (sin el nombre del programa).
pub async fn run(args: &[String]) -> ExitCode {
    match run_command(args).await {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("panel: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run_command(args: &[String]) -> Result<String, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let (add, username) = match args.as_slice() {
        ["user", "add", username] => (true, *username),
        ["user", "passwd", username] => (false, *username),
        _ => return Err(USAGE.to_owned()),
    };
    let password = read_password()?;
    let path = config::db_path_from_env();
    let db = db::open(&path)
        .await
        .map_err(|e| format!("no se puede abrir la base de datos {}: {e}", path.display()))?;
    if add {
        auth::create_user(&db, username, &password)
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!("usuario {username} creado"))
    } else {
        auth::set_password(&db, username, &password)
            .await
            .map_err(|e| e.to_string())?;
        Ok(format!(
            "contraseña de {username} cambiada; sus sesiones se han cerrado"
        ))
    }
}

/// Lee la contraseña nueva: dos veces sin eco desde el terminal o una línea de
/// la entrada estándar.
fn read_password() -> Result<String, String> {
    if std::io::stdin().is_terminal() {
        let first = rpassword::prompt_password("Contraseña nueva: ")
            .map_err(|e| format!("no se puede leer la contraseña: {e}"))?;
        let second = rpassword::prompt_password("Repite la contraseña: ")
            .map_err(|e| format!("no se puede leer la contraseña: {e}"))?;
        if first != second {
            return Err("las contraseñas no coinciden".to_owned());
        }
        Ok(first)
    } else {
        let mut line = String::new();
        let n = std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| format!("no se puede leer la contraseña: {e}"))?;
        if n == 0 {
            return Err("no se ha recibido la contraseña por la entrada estándar".to_owned());
        }
        Ok(line.trim_end_matches(['\n', '\r']).to_owned())
    }
}
