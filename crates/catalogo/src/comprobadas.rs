//! La lista blanca de escrituras: el campo `comprobada` de cada escritura o acción.
//!
//! El panel solo escribe con lo comprobado en el router real
//! ([`Funcion::comprobada`]): lo demás solo se lee. Marcar una función como
//! comprobada es cambiar su línea en `funciones.toml`
//! (`comprobada = false` → `comprobada = "E3, cambio 14"`). Falla cerrada:
//! sin el campo, no está comprobada (y un test pide que toda escritura lo
//! lleve, para que se decida a propósito).
//!
//! Las pruebas contra router-sim lo habilitan todo con
//! [`Catalogo::habilitar_todo_para_pruebas`], que solo vale en un binario
//! de depuración.

use crate::{Catalogo, Comprobacion, Funcion, Tipo};

/// Referencia que pone [`Catalogo::habilitar_todo_para_pruebas`].
pub const EN_PRUEBAS: &str = "habilitada para las pruebas contra router-sim";

/// Reglas del campo al cargar: solo en escrituras y acciones, con una
/// referencia no vacía o `false`.
pub(crate) fn validar(f: &Funcion) -> Result<(), &'static str> {
    match (&f.comprobada, f.tipo) {
        (Some(_), Tipo::Lectura) => Err("una lectura no lleva comprobada"),
        (Some(Comprobacion::Valor(true)), _) => {
            Err("comprobada = true sin referencia: pon la de la prueba en el router real")
        }
        (Some(Comprobacion::Referencia(r)), _) if r.trim().is_empty() => {
            Err("comprobada sin referencia")
        }
        _ => Ok(()),
    }
}

impl Catalogo {
    /// Las escrituras y acciones comprobadas en el router real, en el orden
    /// del fichero.
    pub fn comprobadas(&self) -> impl Iterator<Item = &Funcion> {
        self.todas().iter().filter(|f| f.comprobada())
    }

    /// Da por comprobadas todas las escrituras y acciones. Solo para las
    /// pruebas contra router-sim, que recorren los caminos felices de lo que
    /// aún no se ha comprobado; el servidor solo lo hace con
    /// `RP_E2E_HABILITAR_TODO` en un binario de depuración. Sin
    /// `debug_assertions` (release) no hace nada. Devuelve si lo hizo.
    #[doc(hidden)]
    pub fn habilitar_todo_para_pruebas(&mut self) -> bool {
        habilitar(self, cfg!(debug_assertions))
    }

    /// Da por no comprobada la función `nombre`. Solo restringe, así que no
    /// necesita un binario de depuración: las pruebas de la 1.0 la usan para
    /// tener una función sin comprobar fija aunque se comprueben más en el
    /// router real. Devuelve si la encontró.
    #[doc(hidden)]
    pub fn quitar_comprobada(&mut self, nombre: &str) -> bool {
        match self.funciones.iter_mut().find(|f| f.name == nombre) {
            Some(f) => {
                f.comprobada = Some(Comprobacion::Valor(false));
                true
            }
            None => false,
        }
    }
}

/// [`Catalogo::habilitar_todo_para_pruebas`] con `depuracion` explícito.
fn habilitar(c: &mut Catalogo, depuracion: bool) -> bool {
    if !depuracion {
        return false;
    }
    for f in c.funciones.iter_mut().filter(|f| f.tipo != Tipo::Lectura) {
        f.comprobada = Some(Comprobacion::Referencia(EN_PRUEBAS.into()));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat() -> Catalogo {
        Catalogo::load().expect("funciones.toml válido")
    }

    /// Las comprobadas en el router real en la 1.0. Al comprobar una más,
    /// se cambia su línea en `funciones.toml` y se añade aquí.
    #[test]
    fn comprobadas_de_la_1_0() {
        let c = cat();
        let nombres: Vec<&str> = c.comprobadas().map(|f| f.name.as_str()).collect();
        let mut esperadas = vec![
            "cbSetTr69Config",
            "cbSetTimeNtp",
            "cbSetSyslog",
            "cbSetWifiAccPt",
            "cbSetVoiceSip",
            "cbAddUser",
            "cbDeleteUser",
            "scbidirectionalflt.cmd?action=add",
            "scbidirectionalflt.cmd?action=remove",
            "cbSetUpnpCfg",
            "cbSetMcastIgmpSetting",
            "cbSetDiagConfig",
            "cbSetMcastIntf",
            "cbSetVoicePhysical",
            "cbSetRipIntf",
            "cbSetDnsRelay",
            "cbDhcpV4SvrPoolSetStatic",
            "cbSetLanIPv6Config",
            "cbSetWifi",
            "cbSetNatPortMapping",
            "cbReboot",
        ];
        let mut vistas = nombres.clone();
        esperadas.sort_unstable();
        vistas.sort_unstable();
        assert_eq!(vistas, esperadas);
    }

    /// Toda escritura o acción dice si está comprobada (una referencia o
    /// `false`): que nadie la deje sin decidir.
    #[test]
    fn toda_escritura_lo_decide() {
        let c = cat();
        let sin: Vec<&str> = c
            .todas()
            .iter()
            .filter(|f| f.tipo != Tipo::Lectura && f.comprobada.is_none())
            .map(|f| f.name.as_str())
            .collect();
        assert!(sin.is_empty(), "escrituras sin `comprobada`: {sin:?}");
    }

    /// Sin el campo o con `false`, no está comprobada; una lectura con él,
    /// `true` o una referencia vacía no cargan.
    #[test]
    fn falla_cerrada() {
        let escritura = |extra: &str| {
            format!(
                "[[funcion]]\nname = \"cbGetX\"\narea = \"wan\"\norigen = \"avanzada\"\n\
                 tipo = \"lectura\"\ndescripcion = \"x\"\nriesgo = \"bajo\"\n\
                 [[funcion]]\nname = \"cbSetX\"\narea = \"wan\"\norigen = \"avanzada\"\n\
                 tipo = \"accion\"\ndescripcion = \"x\"\nriesgo = \"alto\"\n\
                 acceso = \"puede_cortar\"\npagina = \"x.asp\"\n{extra}\n"
            )
        };
        let comprobada = |extra: &str| {
            Catalogo::from_toml(&escritura(extra))
                .map(|c| c.funcion("cbSetX").unwrap().comprobada())
        };
        assert_eq!(comprobada(""), Ok(false));
        assert_eq!(comprobada("comprobada = false"), Ok(false));
        assert_eq!(comprobada("comprobada = \"E9, cambio 1\""), Ok(true));
        assert!(comprobada("comprobada = true").is_err());
        assert!(comprobada("comprobada = \" \"").is_err());
        let lectura = escritura("").replace(
            "riesgo = \"bajo\"\n",
            "riesgo = \"bajo\"\ncomprobada = \"x\"\n",
        );
        assert!(Catalogo::from_toml(&lectura).is_err());
    }

    /// Habilitar para las pruebas las da todas por comprobadas, salvo en un
    /// binario sin `debug_assertions`.
    #[test]
    fn habilitar_solo_en_depuracion() {
        let mut c = cat();
        assert!(!habilitar(&mut c, false));
        assert_eq!(c.comprobadas().count(), cat().comprobadas().count());
        assert!(habilitar(&mut c, true));
        assert!(
            c.todas()
                .iter()
                .all(|f| f.tipo == Tipo::Lectura || f.comprobada())
        );
        assert!(c.funcion("cbGetSummary").unwrap().comprobada.is_none());
    }
}
