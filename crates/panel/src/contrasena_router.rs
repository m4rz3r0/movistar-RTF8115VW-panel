//! Cambiar la contraseña del router (la de su web, con la que también entra
//! el panel) desde «Configuración del router → Conexión con el router».
//!
//! No pasa por el gestor de cambios: su vigilancia entra en el router con
//! las credenciales guardadas, y justo esas son las que cambian. El flujo
//! propio ([`cambiar_contrasena_router`]) aparta el gestor, para el sondeo,
//! escribe la nueva y prueba a entrar con ella antes de quedársela; si no
//! entra, comprueba si sigue valiendo la anterior y, si tampoco, vuelve a
//! poner la anterior por la sesión que sigue abierta. Cada intento acaba en
//! el Historial (`seguridad.contrasena_router`) sin ningún valor y sin
//! «Deshacer».
//!
//! Exige reautenticación. Las contraseñas no salen nunca en el registro, en
//! los errores ni en el Historial; las respuestas van con `no-store`.

use leptos::prelude::*;
use leptos::server_fn::codec::Json;

/// Longitud mínima de la contraseña nueva, en bytes.
pub const MIN_BYTES: usize = 8;
/// Longitud máxima de la contraseña nueva, en bytes.
pub const MAX_BYTES: usize = 64;

/// Por qué no vale la contraseña nueva.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorNueva {
    Corta,
    Larga,
    /// Lleva caracteres de control (saltos de línea, tabuladores…).
    Control,
    /// Es la que ya tiene el router.
    Igual,
}

/// Comprueba la forma de la contraseña nueva: de [`MIN_BYTES`] a
/// [`MAX_BYTES`] bytes y sin caracteres de control. Qué caracteres admite el
/// router de verdad queda para la fase 3: aquí no se inventan más reglas.
pub fn validar_forma(nueva: &str) -> Result<(), ErrorNueva> {
    if nueva.len() < MIN_BYTES {
        Err(ErrorNueva::Corta)
    } else if nueva.len() > MAX_BYTES {
        Err(ErrorNueva::Larga)
    } else if nueva.chars().any(char::is_control) {
        Err(ErrorNueva::Control)
    } else {
        Ok(())
    }
}

/// Como [`validar_forma`], y además distinta de la actual.
pub fn validar_nueva(nueva: &str, actual: &str) -> Result<(), ErrorNueva> {
    validar_forma(nueva)?;
    if nueva == actual {
        return Err(ErrorNueva::Igual);
    }
    Ok(())
}

/// Texto de un [`ErrorNueva`] en el idioma de la página.
pub fn texto_error_nueva(e: ErrorNueva) -> String {
    match e {
        ErrorNueva::Corta => crate::t!("contrasena-router-error-corta", n = MIN_BYTES),
        ErrorNueva::Larga => crate::t!("contrasena-router-error-larga", n = MAX_BYTES),
        ErrorNueva::Control => crate::t!("contrasena-router-error-control"),
        ErrorNueva::Igual => crate::t!("contrasena-router-error-igual"),
    }
}

/// Comprueba, antes de pedir la confirmación, que se puede cambiar a
/// `nueva`: reautenticación y la contraseña válida (también distinta de la
/// actual). No escribe nada.
#[server(endpoint = "comprobar_contrasena_router", input = Json)]
pub async fn comprobar_contrasena_router(nueva: String) -> Result<(), ServerFnError> {
    servidor::comprobar(nueva).await
}

/// Cambia la contraseña del router a `nueva` y el panel pasa a usarla, si
/// entra con ella (ver el módulo). Exige reautenticación; 409 con otro
/// cambio en curso.
#[server(endpoint = "cambiar_contrasena_router", input = Json)]
pub async fn cambiar_contrasena_router(nueva: String) -> Result<(), ServerFnError> {
    servidor::cambiar(nueva).await
}

#[cfg(feature = "ssr")]
pub(crate) mod servidor {
    use super::*;
    use crate::{
        instalar::servidor::{fallo, interno, probar},
        instancia::{self, CredencialesRouter},
        panel_ajustes::servidor::preparar_delicada,
        state::AppState,
    };
    use ::i18n::{FluentArgs, Idioma, tr};
    use axum::http::StatusCode;
    use hgu::client::Exclusiva;
    use std::time::Duration;

    /// Lo más que se espera a que se pare el sondeo (acaba su ciclo).
    const ESPERA_SONDEO: Duration = Duration::from_secs(30);
    /// Título del cambio en el Historial: el del ajuste.
    const TITULO: &str = "ajuste-seguridad-contrasena_router-titulo";

    /// Cómo acaba un intento que no cambia la contraseña.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum Fallo {
        /// Hay un cambio aplicándose o vigilándose.
        EnCurso,
        /// El panel aún no tiene el router configurado.
        SinRouter,
        /// Panel en solo lectura (firmware sin probar ni confirmar).
        SoloLectura,
        Nueva(ErrorNueva),
        /// El sondeo no se paró a tiempo: no se ha escrito nada.
        Sondeo,
        /// El router sigue con la anterior: no aceptó la nueva.
        Rechazada,
        /// No se pudo entrar con la nueva; se ha vuelto a poner la anterior.
        Restaurada,
        /// No se entra ni con la nueva ni con la anterior.
        Grave,
        /// El router entró con la nueva, pero el panel no pudo guardarla;
        /// `restaurada`: se volvió a poner la anterior.
        NoGuardada {
            restaurada: bool,
        },
        /// Error del propio panel (sin contraseñas en el texto).
        Interno(String),
    }

    /// Texto de un [`Fallo`] que llegó a hablar con el router, en `idioma`
    /// (para la respuesta y para el Historial).
    fn texto(f: &Fallo, idioma: Idioma) -> String {
        let n = |n: usize| {
            let mut a = FluentArgs::new();
            a.set("n", n);
            a
        };
        match f {
            Fallo::EnCurso => tr(idioma, "contrasena-router-error-en-curso", None),
            Fallo::SinRouter => tr(idioma, "contrasena-router-error-sin-router", None),
            Fallo::SoloLectura => tr(idioma, "panel-solo-lectura", None),
            Fallo::Nueva(ErrorNueva::Corta) => {
                tr(idioma, "contrasena-router-error-corta", Some(&n(MIN_BYTES)))
            }
            Fallo::Nueva(ErrorNueva::Larga) => {
                tr(idioma, "contrasena-router-error-larga", Some(&n(MAX_BYTES)))
            }
            Fallo::Nueva(ErrorNueva::Control) => {
                tr(idioma, "contrasena-router-error-control", None)
            }
            Fallo::Nueva(ErrorNueva::Igual) => tr(idioma, "contrasena-router-error-igual", None),
            Fallo::Sondeo => tr(idioma, "contrasena-router-error-sondeo", None),
            Fallo::Rechazada => tr(idioma, "contrasena-router-error-rechazada", None),
            Fallo::Restaurada => tr(idioma, "contrasena-router-error-restaurada", None),
            Fallo::Grave => tr(idioma, "contrasena-router-error-grave", None),
            Fallo::NoGuardada { restaurada: true } => {
                tr(idioma, "contrasena-router-error-no-guardada", None)
            }
            Fallo::NoGuardada { restaurada: false } => {
                tr(idioma, "contrasena-router-error-grave", None)
            }
            Fallo::Interno(_) => tr(idioma, "comun-error-interno", None),
        }
    }

    /// Error de la server function para `f` (con su código HTTP).
    fn a_error(f: Fallo, idioma: Idioma) -> ServerFnError {
        let status = match &f {
            Fallo::EnCurso => StatusCode::CONFLICT,
            Fallo::SinRouter | Fallo::Nueva(_) => StatusCode::BAD_REQUEST,
            Fallo::SoloLectura => StatusCode::FORBIDDEN,
            Fallo::Sondeo => StatusCode::SERVICE_UNAVAILABLE,
            Fallo::Rechazada | Fallo::Restaurada | Fallo::Grave => StatusCode::BAD_GATEWAY,
            Fallo::NoGuardada { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Fallo::Interno(m) => return interno(m),
        };
        fallo(status, texto(&f, idioma))
    }

    pub(super) async fn comprobar(nueva: String) -> Result<(), ServerFnError> {
        let (state, _) = preparar_delicada().await?;
        let idioma = crate::ui::i18n::idioma_actual();
        let c = instancia::leer_credenciales(&state.db, &state.history_key)
            .await
            .map_err(interno)?
            .ok_or_else(|| a_error(Fallo::SinRouter, idioma))?;
        if instancia::solo_lectura(&state.db).await {
            return Err(a_error(Fallo::SoloLectura, idioma));
        }
        validar_nueva(&nueva, &c.contrasena).map_err(|e| a_error(Fallo::Nueva(e), idioma))
    }

    pub(super) async fn cambiar(nueva: String) -> Result<(), ServerFnError> {
        let (state, yo) = preparar_delicada().await?;
        let idioma = crate::ui::i18n::idioma_actual();
        // En su propia tarea: si quien lo pidió se va (cierra la página), el
        // cambio sigue hasta el final y no queda a medias entre el router y
        // el panel. Las guardas (el gestor apartado, el sondeo en pausa) se
        // sueltan al acabar, también con un pánico.
        let tarea =
            tokio::spawn(
                async move { cambiar_y_anotar(&state, &yo.username, &nueva, idioma).await },
            );
        match tarea.await {
            Ok(r) => r.map_err(|f| a_error(f, idioma)),
            Err(e) => Err(interno(format!(
                "el cambio de la contraseña del router se interrumpió: {e}"
            ))),
        }
    }

    /// El flujo completo de [`cambiar_contrasena_router`] para el usuario
    /// del panel `usuario` (para el Historial).
    async fn cambiar_y_anotar(
        state: &AppState,
        usuario: &str,
        nueva: &str,
        idioma: Idioma,
    ) -> Result<(), Fallo> {
        // Mientras viva, el gestor no empieza ningún cambio; al soltarse,
        // invalida la caché de lecturas.
        let _reserva = state
            .cambios()
            .map(|m| m.excluir_cambios())
            .transpose()
            .map_err(|_| Fallo::EnCurso)?;
        let actual = instancia::leer_credenciales(&state.db, &state.history_key)
            .await
            .map_err(Fallo::Interno)?
            .ok_or(Fallo::SinRouter)?;
        if instancia::solo_lectura(&state.db).await {
            return Err(Fallo::SoloLectura);
        }
        validar_nueva(nueva, &actual.contrasena).map_err(Fallo::Nueva)?;
        // Sin sondeo hasta el final: que nadie entre en el router con la
        // contraseña de antes mientras cambia.
        let _pausa = tokio::time::timeout(ESPERA_SONDEO, state.poller.pause())
            .await
            .map_err(|_| Fallo::Sondeo)?;
        let r = en_el_router(state, &actual, nueva).await;
        anotar(state, usuario, &r, idioma).await;
        // Lo leído para mostrar pudo ser con la sesión de antes.
        state.lecturas.invalidar();
        r
    }

    /// Escribe el cambio de `de` a `a` por la sesión del panel (`sesion`, con
    /// su cerrojo tomado). Es la única
    /// escritura del panel que no pasa por el gestor de cambios ni por el
    /// bloqueo del catálogo (`bloqueada`): el gestor no podría vigilarla ni
    /// revertirla, porque cambia las credenciales con las que entra. La
    /// Biblioteca la sigue teniendo bloqueada. El resultado solo se anota:
    /// lo que vale es si después se entra.
    async fn escribir(sesion: &mut Exclusiva<'_>, usuario: &str, de: &str, a: &str) {
        let params = hgu::fmt::parametros_cambio_contrasena(usuario, de, a);
        let params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let r = sesion
            .write(
                hgu::fmt::PAGINA_CAMBIO_CONTRASENA,
                hgu::fmt::CAMBIO_CONTRASENA,
                &params,
            )
            .await;
        match r {
            Ok(_) => {}
            // El texto del router no se anota: no se sabe qué repite.
            Err(hgu::Error::CallFailed { result, .. }) => {
                tracing::warn!("el router rechazó el cambio de su contraseña (Result={result})")
            }
            Err(e @ (hgu::Error::Network(_) | hgu::Error::NoConfigurado)) => {
                tracing::warn!("cambio de la contraseña del router sin respuesta: {e}")
            }
            Err(_) => tracing::warn!("cambio de la contraseña del router sin respuesta válida"),
        }
    }

    /// ¿Entra el panel en el router con `c`? (Un cliente y una sesión
    /// aparte, que se cierra: no espera al cerrojo de `state.hgu`.)
    /// Si tarda más de [`AppState::espera_prueba_router`], no entra.
    async fn entra(state: &AppState, c: &CredencialesRouter) -> bool {
        let prueba = probar(state, &c.url, &c.usuario, &c.contrasena);
        match tokio::time::timeout(state.espera_prueba_router, prueba).await {
            Ok(r) => r.is_ok(),
            Err(_) => {
                tracing::warn!("la prueba de acceso al router no respondió a tiempo");
                false
            }
        }
    }

    /// Vuelve a poner `actual.contrasena` en el router (que tendría `nueva`)
    /// por la sesión del panel, que sigue abierta, y dice si se entra con
    /// ella.
    async fn restaurar(
        state: &AppState,
        sesion: &mut Exclusiva<'_>,
        actual: &CredencialesRouter,
        nueva: &str,
    ) -> bool {
        escribir(sesion, &actual.usuario, nueva, &actual.contrasena).await;
        entra(state, actual).await
    }

    async fn en_el_router(
        state: &AppState,
        actual: &CredencialesRouter,
        nueva: &str,
    ) -> Result<(), Fallo> {
        // El cerrojo del cliente del panel, sin ceder, durante todo el
        // cambio: ninguna otra llamada por él (una página de Ajustes, la
        // Biblioteca) puede cerrar ni renovar la sesión de la que depende la
        // vuelta atrás. Las pruebas de acceso usan su propio cliente, así que
        // no esperan por él. Se suelta antes de `reconfigurar`, que lo toma.
        let mut sesion = state.hgu.en_exclusiva().await;
        escribir(&mut sesion, &actual.usuario, &actual.contrasena, nueva).await;
        let c = CredencialesRouter {
            contrasena: nueva.to_owned(),
            ..actual.clone()
        };
        if entra(state, &c).await {
            if let Err(e) = instancia::guardar_credenciales(&state.db, &state.history_key, &c).await
            {
                // El router ya tiene la nueva, pero el panel seguiría con la
                // anterior: se vuelve a poner la anterior.
                tracing::error!("contraseña del router cambiada pero sin guardar: {e}");
                let restaurada = restaurar(state, &mut sesion, actual, nueva).await;
                return Err(Fallo::NoGuardada { restaurada });
            }
            drop(sesion);
            state.hgu.reconfigurar(c.client_config()).await;
            state.poller.despertar();
            tracing::info!("contraseña del router cambiada: el panel ya entra con la nueva");
            return Ok(());
        }
        if entra(state, actual).await {
            tracing::warn!(
                "el router no aceptó la contraseña nueva: sigue con la anterior y el panel también"
            );
            return Err(Fallo::Rechazada);
        }
        if restaurar(state, &mut sesion, actual, nueva).await {
            tracing::warn!(
                "el panel no pudo entrar con la contraseña nueva del router: se ha vuelto a poner \
                 la anterior"
            );
            Err(Fallo::Restaurada)
        } else {
            tracing::error!(
                "el panel no entra en el router ni con la contraseña nueva ni con la anterior; \
                 sigue con la anterior guardada"
            );
            Err(Fallo::Grave)
        }
    }

    /// Riesgo del cambio en el Historial: el de la función en el catálogo.
    fn riesgo(state: &AppState) -> &'static str {
        match state
            .catalogo
            .funcion(hgu::fmt::CAMBIO_CONTRASENA)
            .map(|f| f.riesgo)
        {
            Some(catalogo::Riesgo::Bajo) => "bajo",
            Some(catalogo::Riesgo::Alto) => "alto",
            _ => "medio",
        }
    }

    /// Apunta el intento en el Historial: aplicado o fallido con el motivo,
    /// sin ningún valor y sin nada que deshacer (sin pasos).
    async fn anotar(state: &AppState, usuario: &str, r: &Result<(), Fallo>, idioma: Idioma) {
        let (estado, error, nota) = match r {
            Ok(()) => (
                "aplicado",
                None,
                Some(tr(idioma, "contrasena-router-historial-nota", None)),
            ),
            Err(f) => ("fallido", Some(texto(f, idioma)), None),
        };
        let ahora = crate::series::now();
        let fila = sqlx::query(
            "INSERT INTO changes (ts, updated, user_id, usuario, funcion, riesgo, estado, \
             operacion, diff, anteriores, error, nota, titulo) VALUES (?, ?, \
             (SELECT id FROM users WHERE username = ?), ?, ?, ?, ?, 'accion', '[]', '{}', ?, ?, \
             ?)",
        )
        .bind(ahora)
        .bind(ahora)
        .bind(usuario)
        .bind(usuario)
        .bind(hgu::fmt::CAMBIO_CONTRASENA)
        .bind(riesgo(state))
        .bind(estado)
        .bind(error)
        .bind(nota)
        .bind(TITULO)
        .execute(&state.db)
        .await;
        if let Err(e) = fila {
            tracing::error!("no se puede apuntar en el Historial el cambio de contraseña: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valida_la_contrasena_nueva() {
        assert_eq!(validar_forma("1234567"), Err(ErrorNueva::Corta));
        assert_eq!(validar_forma(""), Err(ErrorNueva::Corta));
        assert_eq!(validar_forma("12345678"), Ok(()));
        assert_eq!(validar_forma(&"a".repeat(MAX_BYTES)), Ok(()));
        assert_eq!(
            validar_forma(&"a".repeat(MAX_BYTES + 1)),
            Err(ErrorNueva::Larga)
        );
        // En bytes: 32 «ñ» son 64 bytes; 33, demasiados.
        assert_eq!(validar_forma(&"ñ".repeat(32)), Ok(()));
        assert_eq!(validar_forma(&"ñ".repeat(33)), Err(ErrorNueva::Larga));
        // 4 «ñ» son 8 bytes (4 caracteres).
        assert_eq!(validar_forma("ññññ"), Ok(()));
        for c in ["clave\nde-prueba", "clave\tde-prueba", "clave\u{7f}prueba"] {
            assert_eq!(validar_forma(c), Err(ErrorNueva::Control), "{c:?}");
        }
        assert_eq!(validar_forma("clave con espacios"), Ok(()));
        assert_eq!(
            validar_nueva("clave-de-prueba-1", "clave-de-prueba-1"),
            Err(ErrorNueva::Igual)
        );
        assert_eq!(
            validar_nueva("clave-nueva-de-prueba", "clave-de-prueba-1"),
            Ok(())
        );
    }
}
