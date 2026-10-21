//! Contraseñas: hash Argon2id, verificación y precalentamiento.

use super::*;

/// Hash Argon2id (parámetros por defecto del crate `argon2`) en formato
/// PHC, con sal aleatoria del sistema.
pub fn hash_password(password: &str) -> String {
    let hash: PasswordHash = Argon2::default()
        .hash_password(password.as_bytes())
        .expect("Argon2 con parámetros por defecto y sal del sistema");
    hash.to_string()
}

/// Comprueba `password` contra el hash PHC `hash`. Un hash mal formado
/// cuenta como no válido.
pub fn verify(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}

/// Hash con el que se verifica cuando el usuario no existe, para que el
/// coste (y el tiempo) sea el mismo que con un usuario real.
pub(super) static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("contraseña ficticia para igualar tiempos"));

/// Calcula ya el hash ficticio, para que el primer login con un usuario
/// inexistente no tarde más que los demás. Se llama al arrancar.
pub fn warm_up() {
    LazyLock::force(&DUMMY_HASH);
}

pub(super) async fn hash_blocking(password: &str) -> String {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .expect("la tarea de hash no entra en pánico")
}
