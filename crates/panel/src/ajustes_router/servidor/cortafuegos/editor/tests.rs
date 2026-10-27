//! Pruebas del lado del servidor del editor del cortafuegos: la petición al gestor y los valores legibles del resumen.

use super::*;

fn v(pares: &[(&str, &str)]) -> ValoresCortafuegos {
    pares
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// Cada petición de la página va con su función, su instancia y su
/// identidad: una regla lleva su filtro (`belong`) y de lo que enseñaba la
/// página solo cuenta lo que es de una regla o de un filtro.
#[test]
fn peticiones_del_gestor() {
    let r = peticion(PeticionCortafuegos::ReglaNueva {
        filtro: "1".into(),
        valores: v(&[("protocol", "2"), ("belong", "9")]),
    });
    assert_eq!(
        (r.funcion.as_str(), r.instancia, r.borrar),
        (ALTA_REGLA, None, false)
    );
    assert_eq!(r.valores["belong"], "1");
    let r = peticion(PeticionCortafuegos::ReglaBorrada {
        filtro: "0".into(),
        regla: "3".into(),
        antes: v(&[("protocol", "2"), ("otro", "x")]),
    });
    assert_eq!((r.funcion.as_str(), r.borrar), (ALTA_REGLA, true));
    assert_eq!(r.instancia.as_deref(), Some("3"));
    assert!(r.valores.is_empty());
    assert_eq!(r.identidad, v(&[("belong", "0"), ("protocol", "2")]));
    let r = peticion(PeticionCortafuegos::FiltroBorrado {
        filtro: "4".into(),
        antes: v(&[("name", "X"), ("belong", "0")]),
    });
    assert_eq!((r.funcion.as_str(), r.borrar), (BAJA_FILTRO, true));
    assert_eq!(r.identidad, v(&[("name", "X")]));
}

/// El resumen enseña las opciones con las claves de la tabla de reglas y
/// los tipos ICMP por su nombre.
#[test]
fn valores_legibles() {
    assert_eq!(
        legible("protocol", "2"),
        (
            "ajuste-seguridad-cortafuegos-protocolo".into(),
            TipoCampo::Opcion,
            "tcp".into()
        )
    );
    assert_eq!(legible("enabled", "1").2, "true");
    assert_eq!(legible("icmpType", "25").2, "echo request");
    assert_eq!(legible("type", "1").2, "salida");
    assert_eq!(legible("srcaddr", "192.0.2.1").1, TipoCampo::Texto);
}
