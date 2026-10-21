//! Páginas SSR protegidas: extractor `AuthUser` y redirección al login.

use super::*;

/// Extractor para rutas SSR: el usuario de la sesión, o una redirección a
/// `/login?next=<ruta pedida>` si no hay sesión vigente.
#[derive(Clone, Debug)]
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match user_from_headers(&state.db, &parts.headers).await {
            Ok(Some(user)) => Ok(AuthUser(user)),
            Ok(None) => {
                let next = parts.uri.path_and_query().map_or("/", |pq| pq.as_str());
                let next: String = form_urlencoded::byte_serialize(next.as_bytes()).collect();
                Err(Redirect::to(&format!("/login?next={next}")).into_response())
            }
            Err(e) => {
                eprintln!("panel: error interno en la autenticación: {e}");
                Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        }
    }
}

/// Middleware de las rutas de páginas: exige sesión salvo en `/login` y en
/// las server functions (`/api/*`), que se protegen con [`require_user`].
///
/// Sin usuarios (panel recién instalado), toda página salvo `/instalar`
/// redirige a `/instalar`. Con usuarios, `/instalar` solo existe mientras
/// la instalación sigue en curso (y con sesión); si no, 404.
pub async fn require_page_session(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if path.starts_with("/api/") {
        return next.run(req).await;
    }
    let instalar = path == "/instalar";
    match crate::instalar::hay_usuarios(&state).await {
        Ok(false) if instalar => return next.run(req).await,
        Ok(false) => return Redirect::to("/instalar").into_response(),
        Ok(true) if instalar && !crate::instalar::en_curso(&state).await => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Ok(true) => {}
        Err(e) => {
            eprintln!("panel: error interno en la autenticación: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    if path == "/login" {
        // Con una sesión vigente, el formulario no hace falta: a `next`
        // (como al entrar).
        return match user_from_headers(&state.db, req.headers()).await {
            Ok(Some(_)) => {
                let destino = req.uri().query().and_then(|q| {
                    form_urlencoded::parse(q.as_bytes())
                        .find(|(k, _)| k == "next")
                        .map(|(_, v)| v.into_owned())
                });
                Redirect::to(super::super::safe_next(destino.as_deref())).into_response()
            }
            Ok(None) => next.run(req).await,
            Err(e) => {
                eprintln!("panel: error interno en la autenticación: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }
    let (mut parts, body) = req.into_parts();
    match AuthUser::from_request_parts(&mut parts, &state).await {
        Ok(user) => {
            parts.extensions.insert(user);
            next.run(Request::from_parts(parts, body)).await
        }
        Err(redirect) => redirect,
    }
}
