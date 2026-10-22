//! El orden de una lista: por el nombre o por una columna, en un sentido u otro.

use super::columnas::{Fila, Orden};

/// Por qué se ordena la lista: el nombre o una columna (su posición), y en
/// qué sentido.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Criterio {
    /// `None`: el nombre.
    pub columna: Option<usize>,
    pub ascendente: bool,
}

impl Criterio {
    /// Pulsar la cabecera `columna`: la primera vez, de menor a mayor; otra
    /// vez la misma, al revés.
    pub(crate) fn al_pulsar(self, columna: Option<usize>) -> Criterio {
        Criterio {
            columna,
            ascendente: self.columna != columna || !self.ascendente,
        }
    }
}

/// Ordena `filas` según `c` (sin criterio, como las da el router).
pub(crate) fn ordenar(filas: &mut [Fila], c: Option<Criterio>) {
    let Some(c) = c else {
        return;
    };
    let clave = |f: &Fila| match c.columna {
        None => Orden::Texto(f.nombre.to_lowercase()),
        Some(i) => f
            .celdas
            .get(i)
            .map_or(Orden::Numero(u64::MAX), |x| x.orden.clone()),
    };
    filas.sort_by(|a, b| {
        let o = clave(a).cmp(&clave(b));
        if c.ascendente { o } else { o.reverse() }
    });
}

#[cfg(test)]
mod tests {
    use super::super::columnas::Celda;
    use super::*;
    use crate::ui::comun::EstadoVisual;

    fn fila(nombre: &str, puerto: u64) -> Fila {
        Fila {
            id: nombre.into(),
            nombre: nombre.into(),
            sub: None,
            sub_mono: false,
            celdas: vec![Celda::mono(puerto.to_string(), Orden::Numero(puerto))],
            estado: EstadoVisual::Emite,
            texto_estado: String::new(),
            activa: None,
        }
    }

    fn nombres(f: &[Fila]) -> Vec<&str> {
        f.iter().map(|f| f.nombre.as_str()).collect()
    }

    /// Por número (el 9000 después del 800), por nombre, y al revés al
    /// pulsar otra vez la misma cabecera.
    #[test]
    fn ordenar_por_columna() {
        let mut f = vec![fila("b", 9000), fila("c", 800), fila("a", 27015)];
        ordenar(&mut f, None);
        assert_eq!(nombres(&f), ["b", "c", "a"], "sin criterio, como vienen");
        let c = Criterio {
            columna: None,
            ascendente: true,
        }
        .al_pulsar(Some(0));
        assert!(c.ascendente);
        ordenar(&mut f, Some(c));
        assert_eq!(nombres(&f), ["c", "b", "a"]);
        let c = c.al_pulsar(Some(0));
        assert!(!c.ascendente);
        ordenar(&mut f, Some(c));
        assert_eq!(nombres(&f), ["a", "b", "c"]);
        let c = c.al_pulsar(None);
        assert!(c.ascendente);
        ordenar(&mut f, Some(c));
        assert_eq!(nombres(&f), ["a", "b", "c"]);
    }
}
