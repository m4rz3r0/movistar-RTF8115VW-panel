//! Usuarios de la web del router simulado: la tabla, su lista, el alta, la baja, el cambio de otro usuario y los inicios de sesión.
//!
//! Como el router real (`js/user_profile.js`): `cbAddUser` lleva `rqUser`,
//! `rqPwd` y `rqGroup` en claro; `cbDeleteUser`, `rqDelList` con los
//! nombres seguidos de `:`; `cbModifyPwdOther`, `rqUser`, `rqNPwd` y
//! `rqGroup` en claro. Un error responde `Result=1` con un `<Msg>`.
//!
//! El simulador no protege a nadie: deja borrar o cambiar a cualquiera,
//! también a su propio usuario o al del operador. Las guardas son del panel.

use crate::estado::FixtureKey;
use crate::http::param;
use crate::xml::{leaf, response, result_node};
use hgu::fmt::nombres_de_baja;
use hgu::xml::XmlNode;
use std::collections::HashMap;

/// Lectura de la lista de usuarios.
pub(crate) const LISTA: &str = "cbGetUserList";
/// Contraseña de los usuarios de las fixtures que no son el del simulador.
const CONTRASENA_NEUTRA: &str = "clave-de-prueba-1";

/// Un usuario de la web del router.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Usuario {
    pub(crate) nombre: String,
    pub(crate) contrasena: String,
    /// Papel: `system`, `admin` o `user`.
    pub(crate) grupo: String,
}

/// Los usuarios de partida: los papeles de la fixture de la lista, en orden
/// (sin ella, los tres del router real), con nombres neutros en lugar de
/// los tapados (`X-REDACTED-X`): `operador01` el `system`, el del
/// simulador (`user`) el primer `admin` e `invitado1` el primer `user`.
pub(crate) fn de_partida(
    fixtures: &HashMap<FixtureKey, String>,
    user: &str,
    password: &str,
) -> Vec<Usuario> {
    let papeles: Vec<String> = fixtures
        .get(&(LISTA.to_owned(), Vec::new()))
        .and_then(|raw| hgu::xml::parse_response(LISTA, raw).ok())
        .map(|n| {
            n.children
                .iter()
                .filter(|c| c.tag == "UserRole")
                .map(|c| c.text.clone())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| ["system", "admin", "user"].map(String::from).to_vec());
    let mut out: Vec<Usuario> = Vec::new();
    for (i, grupo) in papeles.into_iter().enumerate() {
        let ya = |n: &str| out.iter().any(|u| u.nombre == n);
        let (nombre, contrasena) = match grupo.as_str() {
            "admin" if !ya(user) => (user.to_owned(), password.to_owned()),
            "system" if !ya("operador01") => ("operador01".to_owned(), CONTRASENA_NEUTRA.into()),
            "user" if !ya("invitado1") => ("invitado1".to_owned(), CONTRASENA_NEUTRA.into()),
            _ => (format!("usuario{i}"), CONTRASENA_NEUTRA.into()),
        };
        out.push(Usuario {
            nombre,
            contrasena,
            grupo,
        });
    }
    out
}

/// La respuesta de [`LISTA`]: por cada usuario, `Result`, `UserName` y
/// `UserRole` seguidos, sin elemento que los agrupe (como el router real).
pub(crate) fn lista(usuarios: &[Usuario]) -> XmlNode {
    let mut hijos = Vec::new();
    for u in usuarios {
        hijos.push(leaf("Result", "0"));
        hijos.push(leaf("UserName", &u.nombre));
        hijos.push(leaf("UserRole", &u.grupo));
    }
    XmlNode {
        tag: LISTA.to_owned(),
        text: String::new(),
        children: hijos,
    }
}

/// ¿Entra `nombre` con `contrasena`?
pub(crate) fn entra(usuarios: &[Usuario], nombre: &str, contrasena: &str) -> bool {
    usuarios
        .iter()
        .any(|u| u.nombre == nombre && u.contrasena == contrasena)
}

/// ¿Es `name` una escritura de usuarios ([`escribir`])?
pub(crate) fn es_suya(name: &str) -> bool {
    [
        hgu::fmt::ALTA_USUARIO,
        hgu::fmt::BAJA_USUARIO,
        hgu::fmt::CAMBIO_OTRO,
    ]
    .contains(&name)
}

/// Aplica la escritura de usuarios `name` con los parámetros `p` (sin
/// `sessionKey`). Devuelve la respuesta del router.
pub(crate) fn escribir(usuarios: &mut Vec<Usuario>, name: &str, p: &[(String, String)]) -> String {
    let r = match name {
        hgu::fmt::ALTA_USUARIO => alta(usuarios, p),
        hgu::fmt::BAJA_USUARIO => baja(usuarios, p),
        _ => cambio_otro(usuarios, p),
    };
    match r {
        Ok(()) => response(&result_node(name, Vec::new(), "0", "")),
        Err(msg) => response(&result_node(name, vec![leaf("Msg", msg)], "1", "")),
    }
}

fn falta<'a>(p: &'a [(String, String)], k: &'static str) -> Result<&'a str, &'static str> {
    param(p, k).filter(|v| !v.is_empty()).ok_or("faltan datos")
}

fn alta(usuarios: &mut Vec<Usuario>, p: &[(String, String)]) -> Result<(), &'static str> {
    let nombre = falta(p, "rqUser")?;
    let contrasena = falta(p, "rqPwd")?;
    let grupo = falta(p, "rqGroup")?;
    if usuarios.iter().any(|u| u.nombre == nombre) {
        return Err("el usuario ya existe");
    }
    usuarios.push(Usuario {
        nombre: nombre.to_owned(),
        contrasena: contrasena.to_owned(),
        grupo: grupo.to_owned(),
    });
    Ok(())
}

fn baja(usuarios: &mut Vec<Usuario>, p: &[(String, String)]) -> Result<(), &'static str> {
    let nombres = nombres_de_baja(falta(p, "rqDelList")?);
    if nombres
        .iter()
        .any(|n| !usuarios.iter().any(|u| u.nombre == *n))
    {
        return Err("el usuario no existe");
    }
    usuarios.retain(|u| !nombres.contains(&u.nombre.as_str()));
    Ok(())
}

fn cambio_otro(usuarios: &mut [Usuario], p: &[(String, String)]) -> Result<(), &'static str> {
    let nombre = falta(p, "rqUser")?;
    let nueva = falta(p, "rqNPwd")?;
    let grupo = falta(p, "rqGroup")?;
    let u = usuarios
        .iter_mut()
        .find(|u| u.nombre == nombre)
        .ok_or("el usuario no existe")?;
    u.contrasena = nueva.to_owned();
    u.grupo = grupo.to_owned();
    Ok(())
}
