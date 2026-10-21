//! Notas del gestor («verificación limitada», «vigilancia retomada»…). Se
//! guardan como claves de traducción con sus argumentos, no como texto: el
//! Historial las pinta en el idioma de quien lo mira. Las filas de antes
//! guardan el texto en castellano y se muestran tal cual.

use i18n::{FluentArgs, Idioma, tr};
use serde::{Deserialize, Serialize};

/// Marca de una nota guardada como claves (el texto de una fila antigua no
/// empieza nunca así).
const PREFIJO: &str = "i18n:";

pub(crate) const VERIFICACION_LIMITADA: &str = "nota-verificacion-limitada";
pub(crate) const REINICIO_SIN_RESPUESTA: &str = "nota-reinicio-sin-respuesta";
pub(crate) const SIN_CONFIRMAR: &str = "nota-sin-confirmar";
pub(crate) const RELEIDO_APLICADA: &str = "nota-releido-aplicada";
pub(crate) const SIN_CAMPOS_ESCRITOS: &str = "nota-sin-campos-escritos";
pub(crate) const SIN_RELEER: &str = "nota-sin-releer";
pub(crate) const WAN_CAIDA: &str = "nota-wan-caida";
pub(crate) const RETOMADA: &str = "nota-retomada";
/// «Paso 2: …», para la nota de un paso de un grupo.
const PASO: &str = "nota-paso";

/// Una nota: uno o varios trozos, que se pintan separados por «; ».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nota(Vec<Trozo>);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Trozo {
    clave: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<(String, String)>,
    /// El paso del grupo al que se refiere (desde 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    paso: Option<usize>,
}

impl Nota {
    /// Una nota sin argumentos.
    pub fn clave(clave: &str) -> Nota {
        Nota::con(clave, &[])
    }

    /// Una nota con sus argumentos (`$nombre` en el `.ftl`).
    pub fn con(clave: &str, args: &[(&str, String)]) -> Nota {
        Nota(vec![Trozo {
            clave: clave.to_owned(),
            args: args
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
            paso: None,
        }])
    }

    /// Esta nota y, detrás, `otra`.
    pub fn y(mut self, otra: Nota) -> Nota {
        self.0.extend(otra.0);
        self
    }

    /// La nota del paso `orden` (desde 0) de un grupo.
    pub fn del_paso(mut self, orden: usize) -> Nota {
        for t in &mut self.0 {
            t.paso.get_or_insert(orden + 1);
        }
        self
    }

    /// Las notas juntas, en orden; `None` si no hay ninguna.
    pub fn juntar(notas: impl IntoIterator<Item = Nota>) -> Option<Nota> {
        notas.into_iter().reduce(Nota::y)
    }

    /// Lo que se guarda en la columna `nota`.
    pub fn guardar(&self) -> String {
        // Una lista de cadenas y números siempre se serializa.
        let json = serde_json::to_string(self).unwrap_or_default();
        format!("{PREFIJO}{json}")
    }

    /// El texto de una nota guardada, en `idioma`. Una fila antigua (o lo
    /// que no se pueda leer como claves) se muestra tal cual.
    pub fn texto(guardada: &str, idioma: Idioma) -> String {
        let Some(nota) = guardada
            .strip_prefix(PREFIJO)
            .and_then(|j| serde_json::from_str::<Nota>(j).ok())
        else {
            return guardada.to_owned();
        };
        nota.0
            .iter()
            .map(|t| {
                let mut args = FluentArgs::new();
                for (k, v) in &t.args {
                    args.set(k.clone(), v.clone());
                }
                let texto = tr(idioma, &t.clave, Some(&args));
                match t.paso {
                    Some(n) => {
                        let mut a = FluentArgs::new();
                        a.set("n", n);
                        a.set("nota", texto);
                        tr(idioma, PASO, Some(&a))
                    }
                    None => texto,
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_claves_existen_en_los_dos_idiomas() {
        for clave in [
            VERIFICACION_LIMITADA,
            REINICIO_SIN_RESPUESTA,
            SIN_CONFIRMAR,
            RELEIDO_APLICADA,
            SIN_CAMPOS_ESCRITOS,
            SIN_RELEER,
            WAN_CAIDA,
            RETOMADA,
            PASO,
        ] {
            for idioma in Idioma::TODOS {
                assert!(i18n::existe(idioma, clave), "{clave} en {idioma:?}");
            }
        }
    }

    #[test]
    fn se_pinta_en_el_idioma_de_quien_mira() {
        let nota = Nota::con(SIN_CONFIRMAR, &[("error", "sin respuesta".into())])
            .y(Nota::clave(RELEIDO_APLICADA));
        let guardada = nota.guardar();
        assert_eq!(
            Nota::texto(&guardada, Idioma::Es),
            "el router no confirmó la escritura (sin respuesta); releído: aplicada"
        );
        assert_eq!(
            Nota::texto(&guardada, Idioma::En),
            "the router didn't confirm the write (sin respuesta); read again: applied"
        );
    }

    #[test]
    fn la_nota_de_un_paso_lleva_su_numero_y_no_la_funcion() {
        let nota = Nota::juntar([
            Nota::clave(RETOMADA),
            Nota::clave(VERIFICACION_LIMITADA).del_paso(1),
        ])
        .unwrap();
        let es = Nota::texto(&nota.guardar(), Idioma::Es);
        assert_eq!(
            es,
            "vigilancia retomada tras un reinicio del panel; paso 2: verificación limitada: \
             el router no muestra los campos escritos"
        );
        let en = Nota::texto(&nota.guardar(), Idioma::En);
        assert_eq!(
            en,
            "watching resumed after the panel restarted; step 2: limited check: the router \
             doesn't show the fields written"
        );
        assert!(!es.contains("cb") && !en.contains("cb"));
        // Un paso ya numerado no se renumera.
        let otra = Nota::clave(WAN_CAIDA).del_paso(0).del_paso(4);
        assert!(Nota::texto(&otra.guardar(), Idioma::Es).starts_with("paso 1: "));
    }

    #[test]
    fn una_fila_antigua_se_muestra_tal_cual() {
        let antigua = "vigilancia retomada tras un reinicio del panel; verificación limitada";
        assert_eq!(Nota::texto(antigua, Idioma::En), antigua);
        // Con la marca pero sin claves que leer, también tal cual.
        assert_eq!(Nota::texto("i18n:{roto", Idioma::En), "i18n:{roto");
        assert_eq!(Nota::juntar([]), None);
    }
}
