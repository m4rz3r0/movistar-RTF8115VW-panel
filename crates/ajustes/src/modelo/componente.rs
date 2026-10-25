//! Componentes propios de la interfaz para un ajuste y sus textos.

use serde::Deserialize;

/// Componente propio de la interfaz para un ajuste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Componente {
    Dispositivos,
    Reservas,
    Puertos,
    ContrasenaRouter,
    UsuariosRouter,
    Cortafuegos,
    Conexion,
}

impl Componente {
    /// Nombre del componente, el mismo que en el TOML.
    pub fn slug(self) -> &'static str {
        match self {
            Componente::Dispositivos => "dispositivos",
            Componente::Reservas => "reservas",
            Componente::Puertos => "puertos",
            Componente::ContrasenaRouter => "contrasena_router",
            Componente::UsuariosRouter => "usuarios_router",
            Componente::Cortafuegos => "cortafuegos",
            Componente::Conexion => "conexion",
        }
    }

    /// Textos propios del componente: finales de claves de traducción
    /// que, tras el prefijo del ajuste (`ajuste-red_local-reservas`),
    /// van en el `.ftl` de su tema.
    pub fn claves(self) -> &'static [&'static str] {
        match self {
            Componente::Dispositivos => &[
                "col-ip",
                "col-mac",
                "sin-nombre",
                "cable",
                "wifi24",
                "wifi5",
                "invitados",
                "desconocida",
                "desconectado",
                "fijar",
                "fijar-de",
                "ya-fija",
                "no-fijable",
                "fijar-desconectado",
                "sin-ip-fija",
                "conexion-ninguna",
                "vacia",
            ],
            Componente::Reservas => &[
                "anadir",
                "nueva",
                "cambiar",
                "borrar-de",
                "activa-de",
                "estado-activa",
                "estado-conectado",
                "estado-desactivada",
                "vacia",
                "error-fuera",
                "error-router",
                "error-ip-repetida",
                "error-mac-repetida",
                "error-difusion",
                "error-cambiada",
            ],
            Componente::Puertos => &[
                "protocolo-ambos",
                "protocolo-ambos-desc",
                "paso-igual",
                "paso-igual-sin-el-primero",
                "abrir",
                "nueva",
                "cambiar",
                "puerto",
                "puerto-desc",
                "dispositivo",
                "dispositivo-desc",
                "dispositivo-elegir",
                "otra-ip",
                "puerto-distinto",
                "interno",
                "interno-desc",
                "a-ip",
                "hacia",
                "estado-abierta",
                "estado-cerrada",
                "estado-sin-destino",
                "destino-perdido-desc",
                "borrar-de",
                "activa-de",
                "vacia",
                "error-puerto",
                "error-dispositivo",
                "error-invertido",
                "error-distintos",
                "error-solapa",
                "error-fuera",
                "error-cero",
                "error-cambiada",
            ],
            Componente::ContrasenaRouter => &[],
            Componente::UsuariosRouter => &[
                "crear",
                "crear-titulo",
                "nuevo-nombre",
                "nuevo-nombre-ayuda",
                "contrasena",
                "contrasena-ayuda",
                "cambiar",
                "cambiar-de",
                "cambiar-boton",
                "cambiar-nota",
                "cambiado",
                "borrar",
                "borrar-de",
                "vacia",
            ],
            Componente::Cortafuegos => &[
                "sin-ping",
                "sin-ping-ayuda",
                "sin-ping-recomendado",
                "ya-puesta",
                "desactivada",
                "sin-leer",
                "sin-filtro",
                "revisar-recomendado",
                "servicio-ping",
                "servicio-web",
                "servicio-ssh",
                "servicio-gestion",
                "servicio-ping-ipv4",
                "servicio-ping-ipv6",
                "y",
            ],
            Componente::Conexion => &[
                "del-operador",
                "internet",
                "sin-identificar",
                "monopuesto-titulo",
                "monopuesto-pasos",
                "monopuesto-avisos",
            ],
        }
    }
}
