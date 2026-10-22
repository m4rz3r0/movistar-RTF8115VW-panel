//! Los campos de un ajuste en edición y lo que cambia, ya validado.

use super::campo::{cambiado, elegido, etiqueta, inicial_campo, validar};
use crate::ajustes_router::CampoVista;
use crate::t;
use leptos::prelude::*;
use std::collections::BTreeMap;

/// Los campos de un ajuste en edición: el valor y el error de cada uno, en
/// el orden de los campos.
#[derive(Clone, Copy)]
pub(crate) struct Edicion {
    campos: StoredValue<Vec<CampoVista>>,
    valores: StoredValue<Vec<RwSignal<String>>>,
    errores: StoredValue<Vec<RwSignal<Option<String>>>>,
}

impl Edicion {
    /// Con lo que se leyó del router.
    pub(crate) fn nueva(campos: &[CampoVista]) -> Edicion {
        Edicion {
            campos: StoredValue::new(campos.to_vec()),
            valores: StoredValue::new(
                campos
                    .iter()
                    .map(|c| RwSignal::new(inicial_campo(c)))
                    .collect(),
            ),
            errores: StoredValue::new(campos.iter().map(|_| RwSignal::new(None)).collect()),
        }
    }

    /// Valor en edición del campo `i`.
    pub(crate) fn valor(self, i: usize) -> RwSignal<String> {
        self.valores.with_value(|v| v[i])
    }

    /// Error junto al campo `i`.
    pub(crate) fn error(self, i: usize) -> RwSignal<Option<String>> {
        self.errores.with_value(|e| e[i])
    }

    /// Primer campo que se puede cambiar (al que va el foco).
    pub(crate) fn primero(self) -> Option<usize> {
        self.campos
            .with_value(|cs| cs.iter().position(|c| c.solo_lectura.is_none()))
    }

    /// Lo deja todo como se leyó, sin errores.
    pub(crate) fn reiniciar(self) {
        self.campos.with_value(|cs| {
            for (i, c) in cs.iter().enumerate() {
                self.valor(i).set(inicial_campo(c));
                self.error(i).set(None);
            }
        });
    }

    /// ¿Algún campo editable tiene ahora otro valor que el leído?
    pub(crate) fn cambiada(self) -> bool {
        self.campos.with_value(|cs| {
            cs.iter().enumerate().any(|(i, c)| {
                c.solo_lectura.is_none() && cambiado(c, &self.valor(i).get_untracked())
            })
        })
    }

    /// Lo que cambia (id del campo → valor legible), validado como lo hará
    /// el servidor. Marca el error de cada campo que no vale; `Err` con el
    /// primero. Un campo que exige otro (encender el espejo exige la toma)
    /// lleva también el exigido, y guardar reenvía la clave que el router no
    /// deja leer: hay que escribirla.
    pub(crate) fn pedir(self) -> Result<BTreeMap<String, String>, usize> {
        let mut pedir = BTreeMap::new();
        let mut primero_mal = None::<usize>;
        self.campos.with_value(|cs| {
            let v = |i: usize| self.valor(i).get_untracked();
            let marcar = |i: usize, m: String| self.error(i).set(Some(m));
            for (i, c) in cs.iter().enumerate() {
                self.error(i).set(None);
                if c.solo_lectura.is_some() || !cambiado(c, &v(i)) {
                    continue;
                }
                match validar(c, &v(i)) {
                    Ok(()) => {
                        pedir.insert(c.id.clone(), v(i));
                    }
                    Err(m) => {
                        marcar(i, m);
                        primero_mal.get_or_insert(i);
                    }
                }
            }
            for c in cs {
                let Some(r) = &c.requiere else {
                    continue;
                };
                if pedir.get(&c.id) != Some(&r.cuando) {
                    continue;
                }
                let Some(q) = cs.iter().position(|x| x.id == r.campo) else {
                    continue;
                };
                if elegido(&cs[q], &v(q)) {
                    pedir.entry(r.campo.clone()).or_insert_with(|| v(q));
                } else {
                    marcar(q, t!("ajustes-error-requiere", campo = etiqueta(c)));
                    primero_mal.get_or_insert(q);
                }
            }
            if !pedir.is_empty() {
                for (i, c) in cs.iter().enumerate() {
                    if c.tapado && c.solo_lectura.is_none() && !pedir.contains_key(&c.id) {
                        marcar(i, t!("ajustes-secreto-tapado-pedir"));
                        primero_mal.get_or_insert(i);
                    }
                }
            }
        });
        match primero_mal {
            Some(i) => Err(i),
            None => Ok(pedir),
        }
    }
}
