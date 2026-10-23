//! Usuarios de la web del router: alta, baja, cambio de otro usuario y lista.

use super::*;
use hgu::fmt::{ALTA_USUARIO, BAJA_USUARIO, CAMBIO_OTRO, PAGINA_USUARIOS, parametros_cambio_otro};

/// ¿Entra `usuario` con `contrasena`? (Un cliente aparte, solo los dos
/// inicios de sesión.)
async fn entra(sim: &SimHandle, usuario: &str, contrasena: &str) -> bool {
    let c = Client::new(ClientConfig {
        base_url: sim.base_url.clone(),
        user: usuario.into(),
        password: contrasena.into(),
        timeout: Duration::from_secs(2),
        min_gap: Duration::from_millis(1),
    });
    let r = c.entrar().await.is_ok();
    c.logout().await;
    r
}

fn pares(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

#[tokio::test]
async fn alta_baja_y_cambio_de_otro_usuario() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let originales = pares(&[
        ("operador01", "system"),
        (USER, "admin"),
        ("invitado1", "user"),
    ]);
    assert_eq!(sim.control.usuarios(), originales);
    // La lista tal como la da el router: nombre y papel seguidos.
    let n = c.call("cbGetUserList", &[]).await.unwrap();
    let nombres: Vec<&str> = n
        .children
        .iter()
        .filter(|h| h.tag == "UserName")
        .map(|h| h.text.as_str())
        .collect();
    assert_eq!(nombres, ["operador01", USER, "invitado1"]);

    // Alta, en claro.
    c.write(
        PAGINA_USUARIOS,
        ALTA_USUARIO,
        &[
            ("rqUser", "prueba1"),
            ("rqPwd", "clave-de-prueba-1"),
            ("rqGroup", "user"),
        ],
    )
    .await
    .unwrap();
    assert_eq!(sim.control.usuarios().len(), 4);
    assert!(
        sim.control
            .usuarios()
            .contains(&pares(&[("prueba1", "user")])[0])
    );
    assert!(entra(&sim, "prueba1", "clave-de-prueba-1").await);
    assert!(!entra(&sim, "prueba1", "otra-clave-1").await);

    // Cambio de otro usuario: contraseña y grupo.
    let p = parametros_cambio_otro("prueba1", "clave-de-prueba-2", "admin");
    let p: Vec<(&str, &str)> = p.iter().map(|(k, v)| (*k, v.as_str())).collect();
    c.write(PAGINA_USUARIOS, CAMBIO_OTRO, &p).await.unwrap();
    assert!(
        sim.control
            .usuarios()
            .contains(&pares(&[("prueba1", "admin")])[0])
    );
    assert_eq!(
        sim.control.contrasena_de("prueba1").as_deref(),
        Some("clave-de-prueba-2")
    );
    assert!(entra(&sim, "prueba1", "clave-de-prueba-2").await);
    assert!(!entra(&sim, "prueba1", "clave-de-prueba-1").await);

    // Baja: la lista con «:» detrás de cada nombre.
    c.write(PAGINA_USUARIOS, BAJA_USUARIO, &[("rqDelList", "prueba1:")])
        .await
        .unwrap();
    assert_eq!(sim.control.usuarios(), originales);
    assert_eq!(sim.control.contrasena_de("prueba1"), None);

    // Un alta repetida: el router responde con error y un mensaje.
    let r = c
        .write(
            PAGINA_USUARIOS,
            ALTA_USUARIO,
            &[
                ("rqUser", "invitado1"),
                ("rqPwd", "clave-de-prueba-1"),
                ("rqGroup", "user"),
            ],
        )
        .await;
    // El mensaje llega en `<Msg>` (no en `ErrorString`): el cliente lo da
    // como texto del error.
    assert!(
        matches!(&r, Err(Error::CallFailed { result, error, .. })
            if result != "0" && error.contains("existe")),
        "{r:?}"
    );
    assert_eq!(sim.control.usuarios(), originales);
}
