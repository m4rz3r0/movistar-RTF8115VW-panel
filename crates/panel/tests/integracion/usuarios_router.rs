//! Usuarios de la web del router (Tarea 2 de la fase 3): crear y borrar por
//! el gestor, con deshacer; cambiar la contraseña o el grupo de otro
//! usuario por el flujo propio; y las guardas del usuario del panel y del
//! `system` del operador, también desde la Biblioteca.

use crate::comun::Panel;
use crate::comun::{
    PanelPrueba, USER, aplicar_ajuste, leer, llamar, ok, pagina, reautenticar, registro_global,
};
use serde_json::{Value, json};

const INVITADO: &str = "invitado1";
const OPERADOR: &str = "operador01";
/// Contraseña de los usuarios que se crean: de 8 a 16 caracteres
/// («clave-de-prueba-1» tiene 17, uno más de los que admite la web oficial).
const CLAVE: &str = "clave-prueba-1";

fn originales() -> Vec<(String, String)> {
    [(OPERADOR, "system"), (USER, "admin"), (INVITADO, "user")]
        .map(|(u, g)| (u.to_owned(), g.to_owned()))
        .to_vec()
}

fn par(u: &str, g: &str) -> (String, String) {
    (u.to_owned(), g.to_owned())
}

fn crear(usuario: &str, contrasena: &str, grupo: &str) -> Value {
    json!({ "Crear": { "usuario": usuario, "contrasena": contrasena, "grupo": grupo } })
}

fn borrar(usuario: &str) -> Value {
    json!({ "Borrar": { "usuario": usuario } })
}

async fn preparar(p: &Panel, cookie: &str, op: Value) -> (u16, String) {
    llamar(
        p,
        "preparar_usuario_router",
        json!({ "op": op }),
        Some(cookie),
    )
    .await
}

async fn preparado_ok(p: &Panel, cookie: &str, op: Value) -> Value {
    ok(p, "preparar_usuario_router", json!({ "op": op }), cookie).await
}

async fn cambiar(
    p: &Panel,
    cookie: &str,
    usuario: &str,
    nueva: &str,
    grupo: &str,
) -> (u16, String) {
    llamar(
        p,
        "cambiar_usuario_router",
        json!({ "usuario": usuario, "nueva": nueva, "grupo": grupo }),
        Some(cookie),
    )
    .await
}

/// Panel con el gestor, la ventana de reautenticación abierta y la cookie.
async fn con_reautenticacion() -> (Panel, String) {
    registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    reautenticar(&p, &cookie).await;
    (p, cookie)
}

/// Todo lo guardado en el Historial, como texto.
async fn historial_en_texto(p: &Panel) -> String {
    let filas: Vec<String> = sqlx::query_scalar(
        "SELECT COALESCE(diff, '') || COALESCE(anteriores, '') || COALESCE(error, '') || \
         COALESCE(nota, '') || COALESCE(solicitud, '') || COALESCE(nuevos, '') FROM changes",
    )
    .fetch_all(&p.state.db)
    .await
    .unwrap();
    filas.join("\n")
}

/// Los `cbModifyPwdOther` recibidos por el simulador.
fn cambios_de_otro(p: &Panel) -> Vec<Vec<(String, String)>> {
    p.sim
        .control
        .writes()
        .into_iter()
        .filter(|(f, _)| f == hgu::fmt::CAMBIO_OTRO)
        .map(|(_, ps)| ps)
        .collect()
}

#[tokio::test]
async fn crear_usuario_y_deshacer() {
    let (p, cookie) = con_reautenticacion().await;
    let grupo = preparado_ok(&p, &cookie, crear("prueba1", CLAVE, "user")).await;
    assert_eq!(grupo["borra"], false, "{grupo}");
    let id = aplicar_ajuste(&p, &cookie, &grupo).await;
    assert!(p.sim.control.usuarios().contains(&par("prueba1", "user")));
    // El alta va en claro, como en la web oficial.
    let (f, params) = p.sim.control.writes().last().cloned().unwrap();
    assert_eq!(f, hgu::fmt::ALTA_USUARIO);
    let mut params = params;
    params.sort();
    assert_eq!(
        params,
        [
            par("rqGroup", "user"),
            par("rqPwd", CLAVE),
            par("rqUser", "prueba1")
        ]
    );
    // Deshacer lo borra.
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &d).await;
    assert_eq!(p.sim.control.usuarios(), originales());
    assert_eq!(
        p.sim.control.writes().last().cloned().unwrap(),
        (
            hgu::fmt::BAJA_USUARIO.to_owned(),
            vec![par("rqDelList", "prueba1:")]
        )
    );
}

#[tokio::test]
async fn borrar_usuario_avisa_y_deshacer_crea_con_contrasena_nueva() {
    let registro = registro_global();
    let (p, cookie) = con_reautenticacion().await;
    let grupo = preparado_ok(&p, &cookie, borrar(INVITADO)).await;
    // El diálogo avisa de que el deshacer no puede devolver su contraseña,
    // y lo confirma con el botón de peligro.
    assert!(
        grupo["aviso"].as_str().is_some_and(|a| !a.is_empty()),
        "{grupo}"
    );
    assert_eq!(grupo["borra"], true, "{grupo}");
    let id = aplicar_ajuste(&p, &cookie, &grupo).await;
    assert!(
        !p.sim.control.usuarios().iter().any(|(u, _)| u == INVITADO),
        "{:?}",
        p.sim.control.usuarios()
    );
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    let deshacer = aplicar_ajuste(&p, &cookie, &d).await;
    assert!(p.sim.control.usuarios().contains(&par(INVITADO, "user")));
    // La contraseña nueva: aleatoria, de 16, y solo con el ojo del detalle
    // del Historial.
    let secretos = p.state.cambios().unwrap().secretos_en_claro(deshacer).await;
    let x = secretos
        .iter()
        .find(|(_, k, _, _, _)| k == "rqPwd")
        .and_then(|(_, _, _, n, _)| n.clone())
        .unwrap_or_else(|| panic!("sin rqPwd: {secretos:?}"));
    assert_eq!(x.len(), 16, "{x}");
    assert!(x.bytes().all(|b| b.is_ascii_alphanumeric()), "{x}");
    assert_eq!(
        p.sim.control.contrasena_de(INVITADO).as_deref(),
        Some(x.as_str())
    );
    assert!(!registro.texto().contains(&x), "en el registro");
    assert!(
        !historial_en_texto(&p).await.contains(&x),
        "en el Historial"
    );
    let html = pagina(&p, &cookie, "/historial").await;
    assert!(!html.contains(&x), "en el HTML de /historial");
}

#[tokio::test]
async fn usuario_protegido_403_sin_escribir() {
    let (p, cookie) = con_reautenticacion().await;
    for u in [USER, OPERADOR] {
        let (status, body) = preparar(&p, &cookie, borrar(u)).await;
        assert_eq!(status, 403, "{u}: {body}");
        assert!(body.contains("protegido"), "{u}: {body}");
        let (status, body) = cambiar(&p, &cookie, u, "clave-nueva-1", "user").await;
        assert_eq!(status, 403, "{u}: {body}");
        assert!(body.contains("protegido"), "{u}: {body}");
    }
    // Una petición fabricada a la Biblioteca, con el del panel en la lista.
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbDeleteUser", "instancia": null,
            "valores": [["rqDelList", "invitado1:1234:"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("protegido"), "{body}");
    assert!(
        p.sim.control.writes().is_empty(),
        "{:?}",
        p.sim.control.writes()
    );
    assert_eq!(p.sim.control.usuarios(), originales());
}

#[tokio::test]
async fn crear_usuario_valida_nombre_grupo_y_contrasena() {
    let (p, cookie) = con_reautenticacion().await;
    let largo = "a".repeat(17);
    for op in [
        crear("ab", CLAVE, "user"),
        crear("prueba1", &largo, "user"),
        crear("prueba1", "clave`de-prueba", "user"),
    ] {
        let (status, body) = preparar(&p, &cookie, op.clone()).await;
        assert_eq!(status, 400, "{op}: {body}");
    }
    // El papel `system`, solo por la Biblioteca (la tarjeta no lo ofrece).
    let (status, body) = llamar(
        &p,
        "preparar_cambio",
        json!({ "peticion": { "funcion": "cbAddUser", "instancia": null,
            "valores": [["rqUser", "prueba1"], ["rqPwd", CLAVE],
                ["rqGroup", "system"]], "borrar": false } }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    // Uno que ya existe.
    let (status, body) = preparar(&p, &cookie, crear(INVITADO, CLAVE, "user")).await;
    assert_eq!(status, 409, "{body}");
    assert!(
        p.sim.control.writes().is_empty(),
        "{:?}",
        p.sim.control.writes()
    );
}

#[tokio::test]
async fn cambiar_contrasena_de_otro_entra_con_la_nueva() {
    let (p, cookie) = con_reautenticacion().await;
    let (status, body) = cambiar(&p, &cookie, INVITADO, "clave-nueva-1", "user").await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        p.sim.control.contrasena_de(INVITADO).as_deref(),
        Some("clave-nueva-1")
    );
    let cambios = cambios_de_otro(&p);
    assert_eq!(cambios.len(), 1, "{cambios:?}");
    let mut params = cambios[0].clone();
    params.sort();
    assert_eq!(
        params,
        [
            par("rqGroup", "user"),
            par("rqNPwd", "clave-nueva-1"),
            par("rqUser", INVITADO)
        ]
    );
    let estado: String = sqlx::query_scalar(
        "SELECT estado FROM changes WHERE funcion = 'cbModifyPwdOther' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&p.state.db)
    .await
    .unwrap();
    assert_eq!(estado, "aplicado");
    assert!(!historial_en_texto(&p).await.contains("clave-nueva-1"));
    // El panel sigue entrando con lo suyo: no ha tocado al usuario del panel.
    assert!(p.sim.control.usuarios().contains(&par(USER, "admin")));
}

#[tokio::test]
async fn cambiar_grupo_de_otro() {
    let (p, cookie) = con_reautenticacion().await;
    let (status, body) = cambiar(&p, &cookie, INVITADO, "clave-nueva-1", "admin").await;
    assert_eq!(status, 200, "{body}");
    assert!(p.sim.control.usuarios().contains(&par(INVITADO, "admin")));
}

#[tokio::test]
async fn cambiar_otro_exige_reautenticacion() {
    registro_global();
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let cuerpo = json!({ "usuario": INVITADO, "nueva": "clave-nueva-1", "grupo": "user" });
    let (status, body) = llamar(&p, "cambiar_usuario_router", cuerpo.clone(), Some(&cookie)).await;
    assert_eq!(status, 401, "{body}");
    assert!(body.contains(panel::auth::ERROR_REAUTENTICACION), "{body}");
    let (status, _) = llamar(&p, "cambiar_usuario_router", cuerpo, None).await;
    assert_eq!(status, 401);
    assert!(cambios_de_otro(&p).is_empty());
    assert_eq!(
        p.sim.control.contrasena_de(INVITADO).as_deref(),
        Some("clave-de-prueba-1")
    );
}

#[tokio::test]
async fn usuarios_con_su_marca_de_protegidos() {
    let (p, cookie) = PanelPrueba::nuevo().arrancar_con_gestor().await;
    let tema = leer(&p, &cookie, "seguridad").await;
    let usuarios = crate::comun::ajustes_de(&tema)
        .into_iter()
        .find(|a| a["id"] == "seguridad.usuarios_router")
        .unwrap();
    assert_eq!(usuarios["componente"], "usuarios_router", "{usuarios}");
    let instancias = usuarios["instancias"].as_array().unwrap();
    let ids: Vec<&Value> = instancias.iter().map(|i| &i["id"]).collect();
    assert_eq!(ids, [&json!(OPERADOR), &json!(USER), &json!(INVITADO)]);
    let notas: Vec<&Value> = instancias.iter().map(|i| &i["nota"]).collect();
    assert_eq!(
        notas,
        [
            &json!("usuarios-router-protegido-sistema"),
            &json!("usuarios-router-protegido-panel"),
            &Value::Null
        ],
        "{usuarios}"
    );
}

/// Si el router rechaza un alta o una baja de usuarios, su texto no llega
/// a la respuesta ni al Historial: solo el código `Result`.
#[tokio::test]
async fn texto_del_router_no_se_anota() {
    const TEXTO: &str = "texto-del-router-X1";
    let (p, cookie) = con_reautenticacion().await;
    for op in [crear("prueba1", CLAVE, "user"), borrar(INVITADO)] {
        let g = preparado_ok(&p, &cookie, op).await;
        p.sim.control.fail_next_write(TEXTO);
        let (status, body) = llamar(
            &p,
            "confirmar_ajuste",
            json!({ "token": g["token"], "desmarcados": [] }),
            Some(&cookie),
        )
        .await;
        assert!(!body.contains(TEXTO), "{status}: {body}");
    }
    let historial = historial_en_texto(&p).await;
    assert!(historial.contains("Result=1"), "{historial}");
    assert!(!historial.contains(TEXTO), "{historial}");
    let html = pagina(&p, &cookie, "/historial").await;
    assert!(!html.contains(TEXTO), "en el HTML de /historial");
    assert_eq!(p.sim.control.usuarios(), originales());
}

/// Si el router responde bien al cambio de otro usuario pero no guarda la
/// contraseña, la prueba de acceso (básico con el panel, avanzado con el
/// otro) no entra: el cambio no se da por hecho.
#[tokio::test]
async fn cambiar_otro_sin_guardar_no_entra() {
    let (p, cookie) = con_reautenticacion().await;
    p.sim.control.ignore_next_write();
    let (status, body) = cambiar(&p, &cookie, INVITADO, "clave-nueva-1", "user").await;
    assert_eq!(status, 502, "{body}");
    assert!(body.contains("no entra con la contraseña nueva"), "{body}");
    assert_eq!(
        p.sim.control.contrasena_de(INVITADO).as_deref(),
        Some("clave-de-prueba-1")
    );
}

/// La función que se anota de una baja es la que se envía (`cbDeleteUser`),
/// en el cambio, en su paso, en el registro y en el detalle del Historial; y
/// su deshacer sigue funcionando.
#[tokio::test]
async fn baja_anota_la_funcion_que_se_envia() {
    let registro = registro_global();
    // El registro es de todo el binario (otras pruebas también borran): basta
    // con que haya una línea más de bajas con la función que se envía.
    let bajas = || {
        registro
            .texto()
            .lines()
            .filter(|l| l.contains("cambio aplicándose") && l.contains("funcion=cbDeleteUser"))
            .count()
    };
    let antes = bajas();
    let (p, cookie) = con_reautenticacion().await;
    let grupo = preparado_ok(&p, &cookie, borrar(INVITADO)).await;
    let id = aplicar_ajuste(&p, &cookie, &grupo).await;
    assert!(bajas() > antes, "el registro no anota cbDeleteUser");
    let (cambio, paso): (String, String) = sqlx::query_as(
        "SELECT c.funcion, p.funcion FROM changes c JOIN cambio_pasos p ON p.cambio_id = c.id \
         WHERE c.id = ?",
    )
    .bind(id)
    .fetch_one(&p.state.db)
    .await
    .unwrap();
    assert_eq!(
        (cambio.as_str(), paso.as_str()),
        ("cbDeleteUser", "cbDeleteUser")
    );
    let html = pagina(&p, &cookie, "/historial").await;
    assert!(
        html.contains("cbDeleteUser"),
        "sin cbDeleteUser en /historial"
    );
    let d = ok(&p, "deshacer_ajuste", json!({ "cambio": id }), &cookie).await;
    aplicar_ajuste(&p, &cookie, &d).await;
    assert!(p.sim.control.usuarios().contains(&par(INVITADO, "user")));
}
