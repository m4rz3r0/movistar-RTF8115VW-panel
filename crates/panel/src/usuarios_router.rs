//! Usuarios de la web del router (Ajustes → Seguridad → Usuarios): crear, borrar y cambiar la contraseña o el grupo de otro.
//!
//! Crear y borrar van por el gestor de cambios
//! ([`preparar_usuario_router`], que se confirma con
//! `confirmar_ajuste`): exigen reautenticación, también desde la
//! Biblioteca, y se vigilan y se deshacen. El deshacer de un borrado
//! vuelve a crear el usuario con una contraseña nueva aleatoria, que solo
//! se ve con el ojo del detalle del cambio en el Historial.
//!
//! Cambiar la contraseña (y con ella, si se quiere, el grupo) de otro
//! usuario no pasa por el gestor: la anterior no se puede leer, así que no
//! se podría deshacer. Sigue el patrón de `contrasena_router`
//! ([`cambiar_usuario_router`]): reautenticación, el gestor apartado, las
//! guardas, la escritura y la prueba de que entra con la nueva desde un
//! cliente aparte, con una nota en el Historial sin valores.
//!
//! Guardas (en el gestor y aquí): nunca el usuario con el que entra el
//! panel ni el del operador (`system`) ([`protegido`]); 403 si se piden.

use crate::ajustes_router::GrupoVista;
use crate::contrasena_forma::ErrorNueva;
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
pub(crate) mod servidor;

/// Grupo que se puede dar a un usuario (`system` es del operador: no se
/// ofrece).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrupoUsuario {
    User,
    Admin,
}

impl GrupoUsuario {
    /// Los dos, en el orden de la interfaz.
    pub const TODOS: [GrupoUsuario; 2] = [GrupoUsuario::User, GrupoUsuario::Admin];

    /// Como lo escribe y lo lee el router (`rqGroup`, `UserRole`).
    pub fn del_router(self) -> &'static str {
        match self {
            GrupoUsuario::User => "user",
            GrupoUsuario::Admin => "admin",
        }
    }

    /// Clave de su opción en el campo «Tipo de cuenta» del ajuste.
    pub fn clave(self) -> &'static str {
        match self {
            GrupoUsuario::User => "usuario",
            GrupoUsuario::Admin => "administrador",
        }
    }

    /// El grupo de la opción `clave` del campo «Tipo de cuenta».
    pub fn de_clave(clave: &str) -> Option<GrupoUsuario> {
        GrupoUsuario::TODOS.into_iter().find(|g| g.clave() == clave)
    }
}

/// Lo que se pide al gestor de cambios.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperacionUsuario {
    Crear {
        usuario: String,
        contrasena: String,
        grupo: GrupoUsuario,
    },
    Borrar {
        usuario: String,
    },
}

impl OperacionUsuario {
    /// El usuario de la operación.
    pub fn usuario(&self) -> &str {
        match self {
            OperacionUsuario::Crear { usuario, .. } | OperacionUsuario::Borrar { usuario } => {
                usuario
            }
        }
    }
}

/// Sin la contraseña: puede acabar en un registro.
impl std::fmt::Debug for OperacionUsuario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperacionUsuario::Crear { grupo, .. } => f
                .debug_struct("Crear")
                .field("grupo", grupo)
                .finish_non_exhaustive(),
            OperacionUsuario::Borrar { .. } => f.debug_struct("Borrar").finish_non_exhaustive(),
        }
    }
}

/// Lo más largo que puede ser la contraseña de otro usuario (o de uno
/// nuevo), en bytes: el `maxlength` de la web oficial. La regla del
/// JavaScript llega a 32, pero el panel no pone una que la web oficial no
/// deje escribir.
pub const MAX_OTRO: usize = 16;

/// Texto de una contraseña nueva que no vale (de 8 a [`MAX_OTRO`]), en el
/// idioma de la página: el mismo en la página y en el servidor.
pub fn texto_error_otro(e: ErrorNueva) -> String {
    e.texto(MAX_OTRO)
}

/// ¿Está protegido el usuario `nombre`, de papel `papel`, si el panel entra
/// con `usuario_panel`? La clave de la nota que lo dice, o `None`.
pub fn protegido(nombre: &str, papel: &str, usuario_panel: &str) -> Option<&'static str> {
    if nombre == usuario_panel {
        Some("usuarios-router-protegido-panel")
    } else if papel == "system" {
        Some("usuarios-router-protegido-sistema")
    } else {
        None
    }
}

/// Nota de la fila de `nombre` en la tarjeta: la de [`protegido`]; y si el
/// panel no sabe con qué usuario entra (`None`: no se pueden leer sus
/// credenciales), una nota en todas, que las deja sin acciones (el servidor
/// las rechazaría igual).
pub fn nota_de_fila(
    nombre: &str,
    papel: &str,
    usuario_panel: Option<&str>,
) -> Option<&'static str> {
    match usuario_panel {
        Some(u) => protegido(nombre, papel, u),
        None => Some(protegido(nombre, papel, "").unwrap_or("usuarios-router-sin-credenciales")),
    }
}

/// Prepara crear o borrar un usuario de la web del router: el resumen del
/// cambio y su token, que se confirma con `confirmar_ajuste`. Exige
/// reautenticación (401 sin ella). 403 con un
/// usuario protegido; 400 con un nombre, un grupo o una contraseña que no
/// valen; 409 con un nombre que ya existe o con otro cambio en curso.
#[server(endpoint = "preparar_usuario_router", input = Json)]
pub async fn preparar_usuario_router(op: OperacionUsuario) -> Result<GrupoVista, ServerFnError> {
    crate::ajustes_router::servidor::ligada(servidor::preparar(op).await?).await
}

/// Deja a `usuario` con la contraseña `nueva` y el grupo `grupo`, y
/// comprueba que entra con ella (ver el módulo). Exige reautenticación. 403
/// con un usuario protegido; 409 con otro cambio en curso.
#[server(endpoint = "cambiar_usuario_router", input = Json)]
pub async fn cambiar_usuario_router(
    usuario: String,
    nueva: String,
    grupo: GrupoUsuario,
) -> Result<(), ServerFnError> {
    servidor::cambiar(usuario, nueva, grupo).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sin saber con qué usuario entra el panel, ninguna fila tiene
    /// acciones: todas llevan nota.
    #[test]
    fn sin_credenciales_todas_las_filas_con_nota() {
        assert_eq!(nota_de_fila("invitado1", "user", Some("1234")), None);
        assert_eq!(
            nota_de_fila("invitado1", "user", None),
            Some("usuarios-router-sin-credenciales")
        );
        assert_eq!(
            nota_de_fila("operador01", "system", None),
            Some("usuarios-router-protegido-sistema")
        );
        for idioma in ::i18n::Idioma::TODOS {
            assert!(::i18n::existe(idioma, "usuarios-router-sin-credenciales"));
        }
    }

    #[test]
    fn protegidos_el_del_panel_y_el_del_operador() {
        assert_eq!(
            protegido("1234", "admin", "1234"),
            Some("usuarios-router-protegido-panel")
        );
        assert_eq!(
            protegido("operador01", "system", "1234"),
            Some("usuarios-router-protegido-sistema")
        );
        assert_eq!(protegido("invitado1", "user", "1234"), None);
    }

    #[test]
    fn sin_la_contrasena_en_el_debug() {
        let op = OperacionUsuario::Crear {
            usuario: "prueba1".into(),
            contrasena: "clave-de-prueba-1".into(),
            grupo: GrupoUsuario::Admin,
        };
        let d = format!("{op:?}");
        assert!(
            !d.contains("clave-de-prueba-1") && !d.contains("prueba1"),
            "{d}"
        );
        assert_eq!(GrupoUsuario::de_clave("usuario"), Some(GrupoUsuario::User));
    }
}
