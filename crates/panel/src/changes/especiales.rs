//! Escrituras del gestor que no siguen el molde del catálogo: los usuarios de la web del router y la regla propia del cortafuegos.
//!
//! `cbAddUser` y `cbDeleteUser` no tienen lista en el catálogo (la lectura,
//! `cbGetUserList`, da pares nombre/papel seguidos, sin id). El gestor las
//! trata como una lista cuyo id es el nombre (`rqUser`):
//!
//! - crear es `cbAddUser`; su deshacer, el borrado genérico de esa
//!   instancia;
//! - borrar se guarda como función `cbAddUser` (o `cbDeleteUser`, si llegó
//!   así de la Biblioteca) y operación `Borrar`, y se escribe como
//!   `cbDeleteUser` con `rqDelList = "<nombre>:"` ([`Especial::baja`]); su
//!   deshacer vuelve a crearlo con su grupo y una contraseña nueva aleatoria
//!   ([`con_contrasena_nueva`]): la de antes no se puede leer.
//!
//! Las guardas viven aquí ([`ChangeManager::planificar_especial`]), así
//! valen para la tarjeta, la Biblioteca y cualquier petición fabricada:
//! nunca el usuario con el que entra el panel ni el del operador (`system`),
//! un solo nombre por baja, grupo `user` o `admin`, el nombre de
//! `^\w{4,20}$` y sin repetir, y la contraseña de 8 a 16 caracteres.
//!
//! La regla «sin ping» del cortafuegos tiene su propio módulo
//! ([`cortafuegos`]).

use super::*;
use hgu::fmt::{ALTA_USUARIO, BAJA_USUARIO, lista_de_baja, nombres_de_baja, usuario_valido};
use hgu::xml::XmlNode;

mod cortafuegos;
pub(crate) use cortafuegos::{SIN_PING_YA_PUESTA, filas_del_cortafuegos};

/// Lectura de la lista de usuarios ([`Lectura::Usuarios`]).
pub(crate) const LECTURA_USUARIOS: &str = "cbGetUserList";
/// Longitud de la contraseña aleatoria del deshacer de un borrado.
const LARGO_ALEATORIA: usize = 16;
/// Grupos que se pueden dar a un usuario (`system` es del operador).
const GRUPOS: [&str; 2] = ["user", "admin"];
/// Papel del usuario del operador: nunca se toca.
const SISTEMA: &str = "system";

/// Una escritura especial del gestor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Especial {
    /// Alta y baja de usuarios de la web del router.
    Usuarios,
    /// La regla «sin ping» del cortafuegos (alta y su baja), con el RID como
    /// id.
    Cortafuegos,
}

impl Especial {
    /// La especial de `f`, si lo es.
    pub(super) fn de(f: &Funcion) -> Option<Especial> {
        match f.name.as_str() {
            ALTA_USUARIO | BAJA_USUARIO => Some(Especial::Usuarios),
            hgu::cortafuegos::ALTA | hgu::cortafuegos::BAJA => Some(Especial::Cortafuegos),
            _ => None,
        }
    }

    /// Lectura con la que se planifica y se comprueba.
    pub(super) fn lectura(self) -> Lectura {
        match self {
            Especial::Usuarios => Lectura::Usuarios,
            Especial::Cortafuegos => Lectura::Cortafuegos,
        }
    }

    /// Campo que identifica cada instancia en lo que da [`ChangeManager::listar`].
    pub(super) fn id(self) -> &'static str {
        match self {
            Especial::Usuarios => "rqUser",
            Especial::Cortafuegos => "RID",
        }
    }

    /// Función y parámetros que borran la instancia `id` (con los valores
    /// del plan, `valores`: el filtro de una regla del cortafuegos).
    pub(super) fn baja(
        self,
        id: &str,
        valores: &BTreeMap<String, String>,
    ) -> (&'static str, Vec<(String, String)>) {
        match self {
            Especial::Usuarios => (
                BAJA_USUARIO,
                vec![("rqDelList".to_owned(), lista_de_baja(&[id]))],
            ),
            Especial::Cortafuegos => cortafuegos::baja(id, valores),
        }
    }
}

/// Los usuarios de una respuesta de `cbGetUserList` como
/// `{rqUser, rqGroup}`. Cuentan solo `UserName` y `UserRole`, y empieza un
/// usuario nuevo cada vez que se repite uno (vienen seguidos, sin elemento
/// que los agrupe): una hoja suelta no crea un usuario de más.
pub(crate) fn usuarios_de(n: &XmlNode) -> Vec<BTreeMap<String, String>> {
    let mut out = Vec::new();
    let mut actual = BTreeMap::new();
    for c in n.children.iter().filter(|c| c.children.is_empty()) {
        let k = match c.tag.as_str() {
            "UserName" => "rqUser",
            "UserRole" => "rqGroup",
            _ => continue,
        };
        if actual.contains_key(k) {
            out.push(std::mem::take(&mut actual));
        }
        actual.insert(k.to_owned(), c.text.clone());
    }
    if !actual.is_empty() {
        out.push(actual);
    }
    out
}

/// Contraseña aleatoria de [`LARGO_ALEATORIA`] letras y cifras ASCII.
fn contrasena_aleatoria() -> String {
    let alfabeto: Vec<u8> = (b'A'..=b'Z')
        .chain(b'a'..=b'z')
        .chain(b'0'..=b'9')
        .collect();
    // Solo los bytes por debajo del mayor múltiplo de 62 que cabe en uno
    // (248): sin sesgo hacia los primeros caracteres.
    let tope = u8::try_from(256 / alfabeto.len() * alfabeto.len()).unwrap_or(u8::MAX);
    let mut out = String::with_capacity(LARGO_ALEATORIA);
    while out.len() < LARGO_ALEATORIA {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("generador aleatorio del sistema");
        for b in bytes.into_iter().filter(|b| *b < tope) {
            if out.len() < LARGO_ALEATORIA {
                out.push(char::from(alfabeto[usize::from(b) % alfabeto.len()]));
            }
        }
    }
    out
}

/// Si `req` vuelve a crear un usuario (el deshacer o la reversión de un
/// borrado), le pone una contraseña nueva aleatoria: la de antes no se
/// puede leer. Solo queda, cifrada, en los valores nuevos del cambio (el
/// ojo del detalle en el Historial).
pub(super) fn con_contrasena_nueva(req: &mut ChangeRequest) {
    if req.funcion == ALTA_USUARIO && !req.borrar && !req.valores.contains_key("rqPwd") {
        req.valores
            .insert("rqPwd".to_owned(), contrasena_aleatoria());
    }
}

/// La instancia de `plan` para el registro: el nombre de un usuario es un
/// dato sensible y no va en él.
pub(super) fn instancia_en_registro(plan: &Plan) -> &str {
    match Especial::de(&plan.funcion) {
        Some(Especial::Usuarios) => MASCARA,
        Some(Especial::Cortafuegos) | None => plan.instancia.as_deref().unwrap_or("-"),
    }
}

impl ChangeManager {
    /// Nombre del usuario con el que entra el panel en el router.
    async fn usuario_del_panel(&self) -> Result<String, CambioError> {
        crate::instancia::leer_credenciales(&self.state.db, &self.state.history_key)
            .await
            .map_err(CambioError::Router)?
            .map(|c| c.usuario)
            .ok_or_else(|| CambioError::Invalida("el panel no tiene el router configurado".into()))
    }

    /// Plan de una escritura especial ([`Especial`]; ver el módulo), con sus
    /// guardas. Con `explicito` (deshacer, reversión), solo no se valida la
    /// forma del nombre, que viene del router; las demás valen siempre.
    pub(super) async fn planificar_especial(
        &self,
        e: Especial,
        f: &Funcion,
        req: &ChangeRequest,
        explicito: bool,
    ) -> Result<Plan, CambioError> {
        if e == Especial::Cortafuegos {
            return self.planificar_cortafuegos(f, req).await;
        }
        for k in req.valores.keys() {
            if f.parametro(k).is_none() {
                return Err(CambioError::Invalida(format!(
                    "{}: parámetro desconocido {k}",
                    f.name
                )));
            }
        }
        let panel = self.usuario_del_panel().await?;
        let lista = self.listar(&Lectura::Usuarios).await.map_err(|e| {
            CambioError::Router(format!("no se puede leer la lista de usuarios: {e}"))
        })?;
        let papel = |n: &str| {
            lista
                .iter()
                .find(|u| buscar(u, "rqUser") == Some(n))
                .and_then(|u| buscar(u, "rqGroup"))
                .map(str::to_owned)
        };
        let protegido = |n: &str| n == panel || papel(n).as_deref() == Some(SISTEMA);
        let bloqueado = || CambioError::Bloqueada(BLOQUEO_USUARIO_PROTEGIDO.into());
        let valor = |k: &str| req.valores.get(k).map(String::as_str).unwrap_or_default();
        let m = &self.state.mascara;
        let campo = |nombre: &str, antes: &str, despues: &str, leido: bool| Campo {
            nombre: nombre.to_owned(),
            antes: antes.to_owned(),
            despues: despues.to_owned(),
            secreto: es_secreto_param(f, m, nombre),
            leido,
        };
        let bajas: Option<Vec<&str>> = match (f.name.as_str(), req.borrar) {
            (BAJA_USUARIO, false) => Some(nombres_de_baja(valor("rqDelList"))),
            (ALTA_USUARIO, true) => Some(req.instancia.as_deref().into_iter().collect()),
            (ALTA_USUARIO, false) => None,
            _ => {
                return Err(CambioError::Invalida(format!(
                    "{}: no se pueden borrar usuarios así",
                    f.name
                )));
            }
        };
        if let Some(nombres) = bajas {
            if nombres.iter().any(|n| protegido(n)) {
                return Err(bloqueado());
            }
            let [nombre] = nombres.as_slice() else {
                return Err(CambioError::Invalida("se borra un usuario cada vez".into()));
            };
            let grupo = papel(nombre)
                .ok_or_else(|| CambioError::Invalida("ese usuario no está en el router".into()))?;
            return Ok(Plan {
                funcion: f.clone(),
                operacion: Operacion::Borrar,
                instancia: Some((*nombre).to_owned()),
                valores: BTreeMap::new(),
                payload: e.baja(nombre, &BTreeMap::new()).1,
                campos: vec![
                    campo("rqUser", nombre, "", true),
                    campo("rqGroup", &grupo, "", true),
                ],
                lectura: Lectura::Usuarios,
                ids_antes: None,
            });
        }
        let (nombre, contrasena, grupo) = (valor("rqUser"), valor("rqPwd"), valor("rqGroup"));
        if protegido(nombre) || grupo == SISTEMA {
            return Err(if grupo == SISTEMA {
                CambioError::Invalida("el grupo system es del operador".into())
            } else {
                bloqueado()
            });
        }
        if !GRUPOS.contains(&grupo) {
            return Err(CambioError::Invalida(
                "grupo no válido: user o admin".into(),
            ));
        }
        // Lo que viene del router (el deshacer de un borrado) no se valida:
        // su nombre ya estaba en él.
        if !explicito && !usuario_valido(nombre) {
            return Err(CambioError::Invalida(
                "el nombre tiene que ser de 4 a 20 letras, cifras o _".into(),
            ));
        }
        crate::contrasena_forma::validar_forma_hasta(contrasena, crate::usuarios_router::MAX_OTRO)
            .map_err(|_| CambioError::Invalida("rqPwd: contraseña no válida".into()))?;
        if papel(nombre).is_some() {
            return Err(CambioError::InstanciaCambiada(USUARIO_YA_EXISTE.into()));
        }
        let valores: BTreeMap<String, String> = [
            ("rqUser", nombre),
            ("rqPwd", contrasena),
            ("rqGroup", grupo),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        Ok(Plan {
            funcion: f.clone(),
            operacion: Operacion::Crear,
            instancia: Some(nombre.to_owned()),
            payload: valores
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            valores,
            campos: vec![
                campo("rqUser", "", nombre, false),
                campo("rqPwd", "", contrasena, false),
                campo("rqGroup", "", grupo, false),
            ],
            lectura: Lectura::Usuarios,
            ids_antes: None,
        })
    }

    /// ¿Muestra el router el lado `lado` de un plan de usuarios? El usuario
    /// del plan es su instancia: al crear, tiene que aparecer con su grupo
    /// (la contraseña no se lee); al borrar, desaparecer.
    pub(super) async fn comprobar_usuarios(&self, plan: &Plan, lado: Lado) -> Comprobacion {
        let lista = match self.listar(&plan.lectura).await {
            Ok(v) => v,
            Err(e) => {
                return Comprobacion::SinLectura(format!("no se puede releer el estado: {e}"));
            }
        };
        let nombre = plan.instancia.as_deref().unwrap_or_default();
        let fila = lista
            .iter()
            .find(|u| buscar(u, Especial::Usuarios.id()) == Some(nombre));
        let difiere = |m: &str| Comprobacion::Difiere(vec![m.to_owned()]);
        let f = &plan.funcion;
        match (plan.operacion, lado, fila) {
            (Operacion::Crear, Lado::Nuevo, Some(u)) => comparar(f, &plan.campos, u, Lado::Nuevo),
            (Operacion::Crear, Lado::Nuevo, None) => difiere("no aparece el usuario nuevo"),
            (Operacion::Crear, Lado::Anterior, Some(_)) => difiere("el usuario nuevo existe"),
            (Operacion::Borrar, Lado::Nuevo, Some(_)) => difiere("el usuario sigue existiendo"),
            (Operacion::Borrar, Lado::Anterior, Some(u)) => {
                comparar(f, &plan.campos, u, Lado::Anterior)
            }
            (Operacion::Borrar, Lado::Anterior, None) => difiere("el usuario ya no existe"),
            (Operacion::Crear, Lado::Anterior, None) | (Operacion::Borrar, Lado::Nuevo, None) => {
                Comprobacion::Coincide(None)
            }
            (Operacion::Editar | Operacion::Accion, _, _) => Comprobacion::Limitada,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrasena_aleatoria_de_16_letras_y_cifras() {
        let a = contrasena_aleatoria();
        assert_eq!(a.len(), LARGO_ALEATORIA);
        assert!(a.bytes().all(|b| b.is_ascii_alphanumeric()), "{a}");
        assert_ne!(a, contrasena_aleatoria());
    }

    #[test]
    fn usuarios_por_pares_sin_ruido() {
        let raw = "<?xml version=\"1.0\" ?><Response><cbGetUserList><Result>0</Result>\
            <opr_status>1</opr_status><UserName>ana1</UserName><UserRole>admin</UserRole>\
            <Result>0</Result><UserName>pepe1</UserName><UserRole>user</UserRole>\
            </cbGetUserList></Response>";
        let n = hgu::xml::parse_response("cbGetUserList", raw).unwrap();
        let v = usuarios_de(&n);
        let pares: Vec<(&str, &str)> = v
            .iter()
            .map(|u| (u["rqUser"].as_str(), u["rqGroup"].as_str()))
            .collect();
        assert_eq!(pares, [("ana1", "admin"), ("pepe1", "user")]);
    }

    #[test]
    fn el_deshacer_de_un_borrado_lleva_contrasena_nueva() {
        let mut req = ChangeRequest {
            funcion: ALTA_USUARIO.into(),
            instancia: None,
            valores: [("rqUser", "ana1"), ("rqGroup", "user")]
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .into(),
            borrar: false,
            identidad: BTreeMap::new(),
        };
        con_contrasena_nueva(&mut req);
        assert_eq!(req.valores["rqPwd"].len(), LARGO_ALEATORIA);
        // El borrado genérico de una creación, no.
        let mut baja = ChangeRequest {
            borrar: true,
            valores: BTreeMap::new(),
            ..req
        };
        con_contrasena_nueva(&mut baja);
        assert!(baja.valores.is_empty());
    }
}
