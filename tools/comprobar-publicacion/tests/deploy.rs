//! Pruebas de `deploy/` (guiones de `sh`): las lanza `cargo test` para que
//! no se queden sin ejecutar. Necesitan `sh` (y `git` la del historial).

use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tools/comprobar-publicacion está dos niveles bajo la raíz")
        .to_path_buf()
}

/// Ejecuta `deploy/pruebas/<guion>` y falla con su salida si no sale con 0.
fn ejecutar(guion: &str) {
    let salida = Command::new("sh")
        .arg(raiz().join("deploy/pruebas").join(guion))
        .env("COMPROBAR", env!("CARGO_BIN_EXE_comprobar-publicacion"))
        .output()
        .expect("no se puede ejecutar sh");
    assert!(
        salida.status.success(),
        "{guion} falla:\n{}{}",
        String::from_utf8_lossy(&salida.stdout),
        String::from_utf8_lossy(&salida.stderr)
    );
}

#[test]
#[cfg_attr(not(unix), ignore = "necesita sh")]
fn plantillas_con_la_configuracion_de_ejemplo() {
    ejecutar("plantillas.sh");
}

#[test]
#[cfg_attr(not(unix), ignore = "necesita sh y git")]
fn actualizador_con_historial_reescrito() {
    ejecutar("historial-reescrito.sh");
}
