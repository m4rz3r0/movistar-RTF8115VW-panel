//! El e2e exige el binario de `cargo leptos build`: el de `cargo test` (este,
//! sin `--cfg erase_components`) se para con un motivo claro.

use std::process::Command;

#[test]
fn el_binario_de_cargo_test_no_vale_para_el_e2e() {
    let salida = Command::new(env!("CARGO_BIN_EXE_panel"))
        .env("RP_E2E_CARGO_LEPTOS", "1")
        .output()
        .unwrap();
    // Si las pruebas se compilaron con `--cfg erase_components` (p. ej. con
    // los RUSTFLAGS de cargo-leptos), el binario también, y sí vale para el
    // e2e: la prueba no tiene sentido, y lo dice.
    if cfg!(erase_components) {
        panic!(
            "las pruebas se compilaron con `--cfg erase_components` (los RUSTFLAGS de \
             cargo-leptos): compílalas sin él"
        );
    }
    assert!(!salida.status.success());
    let error = String::from_utf8_lossy(&salida.stderr);
    assert!(
        error.contains("ejecuta `cargo leptos build`"),
        "el binario de las pruebas no se reconoció como el de `cargo test`: ¿se \
         compiló con `--cfg erase_components` (RUSTFLAGS, .cargo/config.toml)? {error}"
    );
}

#[test]
fn fuera_del_e2e_no_se_comprueba() {
    // Sin la variable, sigue como siempre: una orden que no existe tiene su
    // propio error (y no arranca ningún servidor).
    let salida = Command::new(env!("CARGO_BIN_EXE_panel"))
        .arg("no-existe")
        .env_remove("RP_E2E_CARGO_LEPTOS")
        .output()
        .unwrap();
    assert!(!salida.status.success());
    let error = String::from_utf8_lossy(&salida.stderr);
    assert!(
        !error.is_empty() && !error.contains("cargo leptos build"),
        "{error}"
    );
}
