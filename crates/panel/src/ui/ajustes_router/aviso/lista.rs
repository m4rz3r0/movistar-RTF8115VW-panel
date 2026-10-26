//! Los cambios que sigue la página: el que se aplica o se vigila, los que
//! esperan en la cola y los terminados que aún se ven en el aviso.

use crate::ui::explorador::CambioVista;

/// Un cambio que sigue la página.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CambioActivo {
    /// Para encontrarlo en la lista mientras se confirma (aún sin `id`).
    pub(super) clave: u64,
    /// `None` mientras se prepara o se confirma.
    pub(super) id: Option<i64>,
    /// Se está confirmando (si no, y sin `id`, solo se está preparando).
    pub(super) confirmando: bool,
    /// Elementos que cambian (su insignia lo dice).
    pub(super) elementos: Vec<String>,
    /// Lo que cambia, en una línea («Red WiFi 5 GHz: Red encendida,
    /// Apagada → Encendida»).
    pub(super) texto: String,
    /// El interruptor pulsado y su valor nuevo, que se ve mientras tanto.
    pub(super) interruptor: Option<(String, bool)>,
    /// Claves de los campos que escribe (`ajuste-wifi-red_24-nombre`).
    pub(super) campos: Vec<String>,
    pub(super) estado: Option<CambioVista>,
    /// Al aplicarse, sus elementos se cierran (lo borrado ya no existe; lo
    /// creado desde el detalle de una nueva, ya está en su lista).
    pub(super) cerrar: bool,
    /// Los borra: al aplicarse, se desmarcan y el foco va a su vecina.
    pub(super) borra: bool,
}

impl CambioActivo {
    pub(super) fn terminado(&self) -> bool {
        self.estado.as_ref().is_some_and(|c| c.estado.final_())
    }

    /// Confirmado (o confirmándose) y sin terminar.
    pub(super) fn pendiente(&self) -> bool {
        (self.id.is_some() || self.confirmando) && !self.terminado()
    }

    /// Solo se está preparando (un interruptor que aún no se ha confirmado).
    pub(super) fn preparandose(&self) -> bool {
        self.id.is_none() && !self.confirmando
    }
}

/// Añade `c` a los cambios que sigue la página: lo que solo se preparaba
/// sale; si no queda nada pendiente, empieza una lista nueva (lo terminado
/// deja de verse).
pub(super) fn anadir(lista: &mut Vec<CambioActivo>, c: CambioActivo) {
    lista.retain(|a| !a.preparandose());
    if !lista.iter().any(CambioActivo::pendiente) {
        lista.clear();
    }
    lista.push(c);
}

/// ¿Cambiar `cambia` afecta a `elemento`? Si es él o una de sus filas
/// (`<elemento>/<fila>`).
pub(super) fn afecta(cambia: &str, elemento: &str) -> bool {
    cambia == elemento
        || cambia
            .strip_prefix(elemento)
            .is_some_and(|resto| resto.starts_with('/'))
}

/// Las filas de `elemento` entre los `elementos` que cambian.
pub(super) fn filas_de(elementos: &[String], elemento: &str) -> Vec<String> {
    elementos
        .iter()
        .filter_map(|e| e.strip_prefix(elemento)?.strip_prefix('/'))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CambioActivo, afecta, anadir, filas_de};
    use crate::ui::explorador::{CambioVista, EstadoDeCambio};

    fn cambio(clave: u64, id: Option<i64>, estado: Option<EstadoDeCambio>) -> CambioActivo {
        CambioActivo {
            clave,
            id,
            confirmando: false,
            elementos: Vec::new(),
            texto: String::new(),
            interruptor: None,
            campos: Vec::new(),
            estado: estado.map(|estado| CambioVista {
                id: id.unwrap_or_default(),
                estado,
                error: None,
                nota: None,
                restante_s: None,
                terminable: false,
                puesto: None,
                quitable: false,
            }),
            cerrar: false,
            borra: false,
        }
    }

    fn claves(l: &[CambioActivo]) -> Vec<u64> {
        l.iter().map(|a| a.clave).collect()
    }

    /// Con uno en curso, lo siguiente se añade detrás (irá a la cola); sin
    /// nada pendiente, la lista empieza de nuevo. Lo que solo se preparaba
    /// sale siempre.
    #[test]
    fn la_lista_de_cambios_de_la_pagina() {
        let mut l = vec![cambio(1, Some(10), Some(EstadoDeCambio::Vigilando))];
        anadir(&mut l, cambio(2, None, None));
        assert_eq!(claves(&l), [1, 2]);
        let mut confirmando = cambio(3, None, None);
        confirmando.confirmando = true;
        anadir(&mut l, confirmando);
        assert_eq!(claves(&l), [1, 3], "el que se preparaba sale");
        // Terminado todo, empieza de nuevo.
        let mut l = vec![
            cambio(1, Some(10), Some(EstadoDeCambio::Aplicado)),
            cambio(2, Some(11), Some(EstadoDeCambio::Cancelado)),
        ];
        anadir(&mut l, cambio(4, None, None));
        assert_eq!(claves(&l), [4]);
    }

    /// La tarjeta de los usuarios del router dice que cambia cuando cambia
    /// uno de sus usuarios, no cuando cambia otra tarjeta con un nombre que
    /// empieza igual.
    #[test]
    fn una_fila_afecta_a_su_elemento() {
        assert!(afecta("usuarios_router", "usuarios_router"));
        assert!(afecta("usuarios_router/ana", "usuarios_router"));
        assert!(!afecta("usuarios_router_x", "usuarios_router"));
        assert!(!afecta("usuarios_router", "usuarios_router/ana"));
    }

    #[test]
    fn filas_que_cambian() {
        let e = vec!["usuarios_router/ana".to_owned(), "wifi".to_owned()];
        assert_eq!(filas_de(&e, "usuarios_router"), ["ana"]);
        assert!(filas_de(&e, "wifi").is_empty());
    }
}
