//! Catálogo revisado de las funciones de gestión del router Askey RTF8115VW.
//!
//! La fuente es `funciones.toml` (generado una vez por `tools/gen-catalogo` a
//! partir del análisis del router y revisado a mano). Se compila dentro del
//! binario y se valida al cargar: un catálogo inválido impide arrancar.

pub mod mask;
mod model;

pub use model::{
    Area, Funcion, LecturaPareada, Lista, Origen, Parametro, Partido, Riesgo, Tipo, TipoParam,
};

use serde::Deserialize;
use std::collections::HashSet;
use thiserror::Error;

/// Texto de `funciones.toml`.
pub const FUNCIONES_TOML: &str = include_str!("../funciones.toml");

/// Errores al cargar el catálogo.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CatalogoError {
    /// El TOML no se puede interpretar.
    #[error("funciones.toml no válido: {0}")]
    Toml(String),
    /// El TOML se interpreta pero incumple una regla.
    #[error("catálogo no válido: {0}")]
    Invalido(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fichero {
    #[serde(default)]
    funcion: Vec<Funcion>,
}

/// Catálogo de funciones validado.
#[derive(Debug, Clone)]
pub struct Catalogo {
    funciones: Vec<Funcion>,
}

impl Catalogo {
    /// Carga y valida el catálogo compilado en el binario.
    pub fn load() -> Result<Catalogo, CatalogoError> {
        Self::from_toml(FUNCIONES_TOML)
    }

    /// Carga y valida un catálogo desde texto TOML.
    pub fn from_toml(text: &str) -> Result<Catalogo, CatalogoError> {
        let f: Fichero = toml::from_str(text).map_err(|e| CatalogoError::Toml(e.to_string()))?;
        let cat = Catalogo {
            funciones: f.funcion,
        };
        cat.validar()?;
        Ok(cat)
    }

    /// Todas las funciones, en el orden del fichero.
    pub fn todas(&self) -> &[Funcion] {
        &self.funciones
    }

    /// Función por nombre.
    pub fn funcion(&self, name: &str) -> Option<&Funcion> {
        self.funciones.iter().find(|f| f.name == name)
    }

    /// Funciones de un área, en el orden del fichero.
    pub fn por_area(&self, area: Area) -> impl Iterator<Item = &Funcion> {
        self.funciones.iter().filter(move |f| f.area == area)
    }

    fn validar(&self) -> Result<(), CatalogoError> {
        let mal = |f: &Funcion, msg: String| CatalogoError::Invalido(format!("{}: {msg}", f.name));
        let mut nombres = HashSet::new();
        for f in &self.funciones {
            if f.name.trim().is_empty() {
                return Err(CatalogoError::Invalido("función sin nombre".into()));
            }
            if !nombres.insert(f.name.as_str()) {
                return Err(mal(f, "nombre repetido".into()));
            }
            if f.descripcion.trim().is_empty() {
                return Err(mal(f, "sin descripción".into()));
            }
            if f.bloqueada.as_deref().is_some_and(|m| m.trim().is_empty()) {
                return Err(mal(f, "bloqueada sin motivo".into()));
            }
            if f.tipo == Tipo::Lectura && f.riesgo != Riesgo::Bajo {
                return Err(mal(f, "una lectura no puede tener riesgo".into()));
            }
            let mut params = HashSet::new();
            for p in &f.parametros {
                if !params.insert(p.nombre.as_str()) {
                    return Err(mal(f, format!("parámetro {} repetido", p.nombre)));
                }
                if p.etiqueta.trim().is_empty() {
                    return Err(mal(f, format!("parámetro {} sin etiqueta", p.nombre)));
                }
                match (p.tipo, p.opciones.is_empty()) {
                    (TipoParam::Enum, true) => {
                        return Err(mal(f, format!("enum {} sin opciones", p.nombre)));
                    }
                    (t, false) if t != TipoParam::Enum => {
                        return Err(mal(f, format!("{} tiene opciones sin ser enum", p.nombre)));
                    }
                    _ => {}
                }
                match (p.partido, p.tipo) {
                    (None, _) | (Some(Partido::Ip4), TipoParam::Ip) => {}
                    (Some(Partido::Mac6), TipoParam::Mac) => {}
                    _ => {
                        return Err(mal(
                            f,
                            format!("{}: partido no cuadra con el tipo", p.nombre),
                        ));
                    }
                }
            }
        }
        for f in &self.funciones {
            if let Some(lp) = &f.lectura_pareada {
                match self.funcion(&lp.funcion) {
                    None => {
                        return Err(mal(f, format!("lectura pareada {} no existe", lp.funcion)));
                    }
                    Some(l) if l.tipo != Tipo::Lectura => {
                        return Err(mal(f, format!("{} no es una lectura", lp.funcion)));
                    }
                    Some(_) => {}
                }
                for (_, v) in &lp.params {
                    if let Some(p) = v.strip_prefix('{').and_then(|v| v.strip_suffix('}'))
                        && f.parametro(p).is_none()
                    {
                        return Err(mal(
                            f,
                            format!("la lectura pareada usa {{{p}}}, que no existe"),
                        ));
                    }
                }
            }
            if f.tipo != Tipo::Lectura && f.bloqueada.is_none() && f.pagina.is_none() {
                return Err(mal(f, "sin página de la que sacar el sessionKey".into()));
            }
            if let Some(l) = &f.lista {
                let en_lectura = f
                    .lectura_pareada
                    .as_ref()
                    .is_some_and(|lp| lp.params.iter().any(|(k, _)| *k == l.id_param));
                if f.parametro(&l.id_param).is_none() && !en_lectura {
                    return Err(mal(
                        f,
                        format!("id_param {} no es un parámetro", l.id_param),
                    ));
                }
                if l.del && f.parametro("del").is_some() {
                    return Err(mal(
                        f,
                        "con lista.del, `del` no es un parámetro editable (lo añade el gestor)"
                            .into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat() -> Catalogo {
        Catalogo::load().expect("funciones.toml válido")
    }

    #[test]
    fn catalogo_carga_y_valida() {
        let c = cat();
        assert!(c.todas().len() > 100, "{}", c.todas().len());
        for a in Area::TODAS {
            assert_eq!(Area::from_slug(a.slug()), Some(*a));
            assert!(c.por_area(*a).next().is_some(), "área vacía: {}", a.slug());
        }
        assert_eq!(Area::from_slug("nada"), None);
        assert_eq!(Area::LanDhcp.etiqueta(), "LAN/DHCP");
        assert!(c.funcion("cbGetSummary").is_some());
        assert!(c.funcion("noexiste").is_none());
        for f in c.todas() {
            assert!(
                !f.descripcion.contains("PENDIENTE"),
                "{} sin revisar",
                f.name
            );
        }
    }

    #[test]
    fn validacion_rechaza_catalogos_rotos() {
        let base = r#"
[[funcion]]
name = "cbGetX"
area = "wan"
origen = "avanzada"
tipo = "lectura"
descripcion = "x"
riesgo = "bajo"
"#;
        assert!(Catalogo::from_toml(base).is_ok());
        let repetido = format!("{base}{base}");
        assert!(Catalogo::from_toml(&repetido).is_err());
        let sin_motivo = base.replace("riesgo = \"bajo\"", "riesgo = \"bajo\"\nbloqueada = \" \"");
        assert!(Catalogo::from_toml(&sin_motivo).is_err());
        let pareada_falsa = format!(
            "{base}\n[[funcion]]\nname = \"cbSetX\"\narea = \"wan\"\norigen = \"avanzada\"\n\
             tipo = \"escritura\"\ndescripcion = \"x\"\nriesgo = \"alto\"\n\
             lectura_pareada = {{ funcion = \"cbGetY\" }}\n"
        );
        assert!(Catalogo::from_toml(&pareada_falsa).is_err());
        let sin_tipo = format!("{base}\n[[funcion.parametro]]\nnombre = \"a\"\netiqueta = \"A\"\n");
        assert!(Catalogo::from_toml(&sin_tipo).is_err());
        let enum_vacio = format!(
            "{base}\n[[funcion.parametro]]\nnombre = \"a\"\netiqueta = \"A\"\ntipo = \"enum\"\n"
        );
        assert!(Catalogo::from_toml(&enum_vacio).is_err());
        let lectura_con_riesgo = base.replace("\"bajo\"", "\"alto\"");
        assert!(Catalogo::from_toml(&lectura_con_riesgo).is_err());
        let escritura = |extra: &str| {
            format!(
                "{base}\n[[funcion]]\nname = \"cbSetX\"\narea = \"wan\"\norigen = \"avanzada\"\n\
                 tipo = \"escritura\"\ndescripcion = \"x\"\nriesgo = \"alto\"\n\
                 lectura_pareada = {{ funcion = \"cbGetX\", params = [[\"x_0i\", \"{{x_id}}\"]] }}\n{extra}\n\
                 [[funcion.parametro]]\nnombre = \"x_id\"\netiqueta = \"X\"\ntipo = \"int\"\n"
            )
        };
        let buena = escritura("pagina = \"x.asp\"\nlista = { id_param = \"x_id\", del = true }");
        assert!(
            Catalogo::from_toml(&buena).is_ok(),
            "{:?}",
            Catalogo::from_toml(&buena)
        );
        let sin_pagina = escritura("lista = { id_param = \"x_id\", del = true }");
        assert!(Catalogo::from_toml(&sin_pagina).is_err());
        let bloqueada_sin_pagina = escritura("bloqueada = \"motivo suficiente\"");
        assert!(Catalogo::from_toml(&bloqueada_sin_pagina).is_ok());
        let id_ajeno = escritura("pagina = \"x.asp\"\nlista = { id_param = \"y_id\" }");
        assert!(Catalogo::from_toml(&id_ajeno).is_err());
        let id_de_lectura = escritura("pagina = \"x.asp\"\nlista = { id_param = \"x_0i\" }");
        assert!(Catalogo::from_toml(&id_de_lectura).is_ok());
        let del_editable = format!(
            "{buena}\n[[funcion.parametro]]\nnombre = \"del\"\netiqueta = \"D\"\ntipo = \"text\"\n"
        );
        assert!(Catalogo::from_toml(&del_editable).is_err());
    }

    #[test]
    fn bloqueadas_tienen_motivo() {
        let c = cat();
        for n in [
            "cbRestoreConfig",
            "cbFWUpgradeByFTP",
            "restoreinfo.cgi",
            "upload/cfgupload.asp",
            "upload/imgupgrade.asp",
            "te_uploadsettings.asp",
            "te_uploadinfo.asp",
            "syslog.asp",
            "backup_restore.asp",
            "cbModifyPwd",
            "cbModifyPwdOther",
        ] {
            let f = c.funcion(n).unwrap_or_else(|| panic!("falta {n}"));
            let m = f
                .bloqueada
                .as_deref()
                .unwrap_or_else(|| panic!("{n} no está bloqueada"));
            assert!(m.len() > 10, "{n}: motivo demasiado corto");
        }
        for f in c.todas() {
            if f.name.starts_with("upload/") {
                assert!(f.bloqueada.is_some(), "{}", f.name);
            }
        }
    }

    #[test]
    fn reboot_es_accion_alto() {
        let c = cat();
        let r = c.funcion("cbReboot").unwrap();
        assert_eq!(
            (r.tipo, r.riesgo, r.bloqueada.as_deref()),
            (Tipo::Accion, Riesgo::Alto, None)
        );
        assert_eq!(c.funcion("cbSaveConfig").unwrap().tipo, Tipo::Accion);
    }

    /// El registro del sistema se configura como un ajuste más: su escritura
    /// se lee con su lectura pareada y nunca lleva `export`, que exporta el
    /// registro (spec de mejoras, §5.2).
    #[test]
    fn set_syslog_es_escritura_sin_export() {
        let c = cat();
        let f = c.funcion("cbSetSyslog").unwrap();
        assert_eq!(f.tipo, Tipo::Escritura);
        assert!(f.parametro("export").is_none());
        assert_eq!(f.lectura_pareada.as_ref().unwrap().funcion, "cbGetSyslog");
    }

    #[test]
    fn toda_escritura_tiene_lectura_pareada_o_es_accion() {
        for f in cat().todas() {
            if f.tipo == Tipo::Escritura {
                assert!(
                    f.lectura_pareada.is_some(),
                    "{} sin lectura pareada",
                    f.name
                );
            }
        }
    }

    #[test]
    fn reglas_de_riesgo_y_origen() {
        let c = cat();
        for f in c.todas().iter().filter(|f| f.tipo != Tipo::Lectura) {
            if matches!(f.area, Area::Wan | Area::Cortafuegos | Area::Tr069) {
                assert_eq!(f.riesgo, Riesgo::Alto, "{}", f.name);
            }
            if matches!(f.area, Area::Wifi | Area::Dns) && f.bloqueada.is_none() {
                assert!(f.riesgo >= Riesgo::Medio, "{}", f.name);
            }
        }
        for n in ["cbSetLanIPv4Addr", "cbSetLanIPv4Intf"] {
            assert_eq!(c.funcion(n).unwrap().riesgo, Riesgo::Alto, "{n}");
        }
        assert_eq!(
            c.funcion("cbSetDhcpv4ServerPool").unwrap().riesgo,
            Riesgo::Medio
        );
        for f in c.todas().iter().filter(|f| f.name.starts_with("te_")) {
            assert_eq!(f.origen, Origen::Basica, "{}", f.name);
        }
        for (n, o) in [
            ("cwmp.asp", Origen::Oculta),
            ("cbGetTr69Config", Origen::Oculta),
            ("cbGetTimeNtp", Origen::Oculta),
            ("scbidirectionalflt_ls.asp", Origen::Oculta),
            ("user_profile_add.asp", Origen::Oculta),
            ("user_profile_modifyOthers.asp", Origen::Oculta),
            ("notsupport.asp", Origen::Incompleta),
            ("system.asp", Origen::Incompleta),
        ] {
            assert_eq!(
                c.funcion(n).unwrap_or_else(|| panic!("falta {n}")).origen,
                o,
                "{n}"
            );
        }
    }

    #[test]
    fn parametros_secretos_de_la_ui_basica() {
        let c = cat();
        let m = mask::Mascara::load().unwrap();
        for f in c.todas() {
            for p in &f.parametros {
                if m.es_secreto(&p.nombre) {
                    assert_eq!(p.tipo, TipoParam::Secret, "{}.{}", f.name, p.nombre);
                }
            }
        }
        let wl = c.funcion("te_wifi.wl").unwrap();
        assert_eq!(wl.parametro("wlWpaPsk").unwrap().tipo, TipoParam::Secret);
    }

    /// Solo las credenciales son `secret` (censuradas con el ojo): un
    /// usuario, una URL o un número es texto, que se ve en claro.
    #[test]
    fn solo_las_credenciales_son_secret() {
        let c = cat();
        let m = mask::Mascara::load().unwrap();
        let mut mal = Vec::new();
        for f in c.todas() {
            for p in f.parametros.iter().filter(|p| p.tipo == TipoParam::Secret) {
                if !m.es_secreto(&p.nombre) {
                    mal.push(format!("{}.{}", f.name, p.nombre));
                }
            }
        }
        assert!(mal.is_empty(), "secret sin ser credencial: {mal:?}");
    }
}
