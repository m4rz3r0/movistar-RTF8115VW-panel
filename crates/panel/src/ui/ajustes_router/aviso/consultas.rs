//! Lo que la rejilla pregunta al seguimiento mientras algo cambia: la
//! insignia, las filas que cambian y el valor del interruptor.

use super::*;

impl Seguimiento {
    /// Insignia de `elemento` mientras cambia, también si lo que cambia es
    /// una de sus filas (`<elemento>/<fila>`, un usuario del router): la
    /// tarjeta y el detalle dicen «Aplicando…» o «Comprobando…». Reactivo.
    pub(crate) fn insignia(self, elemento: &str) -> Option<(EstadoVisual, String)> {
        self.cambios.with(|l| {
            let a = l
                .iter()
                .find(|a| !a.terminado() && a.elementos.iter().any(|e| afecta(e, elemento)))?;
            match a.estado.as_ref().map(|c| c.estado) {
                None | Some(EstadoDeCambio::Preparado | EstadoDeCambio::Aplicando) => {
                    Some((EstadoVisual::Proceso, t!("ajustes-insignia-aplicando")))
                }
                Some(EstadoDeCambio::Vigilando) => {
                    Some((EstadoVisual::Proceso, t!("ajustes-insignia-comprobando")))
                }
                Some(EstadoDeCambio::EnCola) => {
                    Some((EstadoVisual::Proceso, t!("ajustes-insignia-en-cola")))
                }
                Some(_) => None,
            }
        })
    }

    /// Las filas de `elemento` (lo que va tras `<elemento>/`) que cambia el
    /// cambio en curso, mientras no termine: la lista las enseña ya, con su
    /// insignia, aunque aún no se haya vuelto a leer. Reactivo.
    pub(crate) fn filas_en_curso(self, elemento: &str) -> Vec<String> {
        self.cambios.with(|l| {
            l.iter()
                .filter(|a| !a.terminado())
                .flat_map(|a| filas_de(&a.elementos, elemento))
                .collect()
        })
    }

    /// Valor del interruptor de `elemento` mientras cambia. Reactivo.
    /// El último que se pide manda.
    pub(crate) fn interruptor(self, elemento: &str) -> Option<bool> {
        self.cambios.with(|l| {
            l.iter()
                .rev()
                .filter(|a| !a.terminado())
                .filter_map(|a| a.interruptor.as_ref())
                .find(|(e, _)| e == elemento)
                .map(|(_, v)| *v)
        })
    }
}
