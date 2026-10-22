//! Los pasos del diálogo de un grupo: sus casillas, agrupados por pestaña, con su tabla «Ahora → Después».

use super::texto_valor;
use crate::ajustes_router::{CambioLegible, GrupoVista, PasoVista};
use crate::t;
use crate::ui::explorador::MASCARA;
use crate::ui::i18n::{idioma_actual, traducir};
use leptos::prelude::*;

fn valor(c: &CambioLegible, v: &str) -> AnyView {
    if c.secreto {
        return view! {
            <span class="secreto" aria-hidden="true">{MASCARA}</span>
            <span class="solo-lector">{t!("comun-valor-oculto")}</span>
        }
        .into_any();
    }
    let texto = texto_valor(c, v);
    if texto != v {
        // Un texto traducido (un sí/no, una opción): se parte por palabras.
        return view! { <span>{texto}</span> }.into_any();
    }
    // Un valor tal cual (una MAC, una IP, una dirección web): se parte solo
    // tras sus separadores.
    let trozos = trozos(v)
        .into_iter()
        .map(|t| view! { {t.to_owned()}<wbr /> })
        .collect_view();
    view! { <span>{trozos}</span> }.into_any()
}

/// `texto` en trozos que acaban en «:», «.» o «/»: tras cada uno puede
/// partirse la línea (una MAC por sus «:», una dirección web por sus «.» y
/// «/»), y en ningún otro sitio salvo un espacio o un «-».
fn trozos(texto: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut inicio = 0;
    for (i, c) in texto.char_indices() {
        if matches!(c, ':' | '.' | '/') {
            out.push(&texto[inicio..=i]);
            inicio = i + 1;
        }
    }
    if inicio < texto.len() {
        out.push(&texto[inicio..]);
    }
    out
}

/// Tabla «antes → después» de los campos de un paso.
fn tabla(campos: &[CambioLegible]) -> AnyView {
    if campos.is_empty() {
        return ().into_any();
    }
    let filas = campos
        .iter()
        .map(|c| {
            view! {
                <tr>
                    <th scope="row">{traducir(&c.campo, None)}</th>
                    <td>{valor(c, &c.antes)}</td>
                    <td>{valor(c, &c.despues)}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="tabla-scroll">
            <table class="tabla">
                <thead>
                    <tr>
                        <th scope="col">{t!("ajustes-grupo-col-dato")}</th>
                        <th scope="col">{t!("ajustes-grupo-col-antes")}</th>
                        <th scope="col">{t!("ajustes-grupo-col-despues")}</th>
                    </tr>
                </thead>
                <tbody>{filas}</tbody>
            </table>
        </div>
    }
    .into_any()
}

/// Marca inicial de un paso: marcado si cambia algo y no es de los que
/// salen desmarcados (un paso opcional de un lote).
fn marca_inicial(p: &PasoVista) -> bool {
    !p.sin_cambios && !p.desmarcado
}

/// Casillas iniciales del grupo `g`.
pub(super) fn marcados_de(g: &GrupoVista) -> Vec<bool> {
    g.pasos.iter().map(marca_inicial).collect()
}

/// Casillas de `nuevo`, que sustituye a `viejo` (con las casillas `marcas`)
/// al cambiar una opción de la revisión: cada paso conserva la marca del
/// paso de `viejo` con su misma `clave`, aunque cambie de posición. Un paso
/// sin clave, o que no estaba, sale con su marca inicial; uno que ya no
/// cambia nada, desmarcado.
pub(super) fn marcas_conservadas(
    viejo: &GrupoVista,
    marcas: &[bool],
    nuevo: &GrupoVista,
) -> Vec<bool> {
    nuevo
        .pasos
        .iter()
        .map(|p| {
            let antes = p.clave.as_ref().and_then(|c| {
                viejo
                    .pasos
                    .iter()
                    .position(|v| v.clave.as_ref() == Some(c))
                    .and_then(|i| marcas.get(i).copied())
            });
            !p.sin_cambios && antes.unwrap_or_else(|| marca_inicial(p))
        })
        .collect()
}

/// Índices de los pasos con cambios que se han desmarcado.
pub(super) fn desmarcados(pasos: &[PasoVista], marcados: &[bool]) -> Vec<usize> {
    pasos
        .iter()
        .zip(marcados)
        .filter(|(p, m)| !p.sin_cambios && !**m)
        .map(|(p, _)| p.indice)
        .collect()
}

/// Los pasos seguidos de la misma pestaña, juntos: la pestaña (su clave) y
/// las posiciones de sus pasos, en orden.
pub(super) fn por_pestana(pasos: &[PasoVista]) -> Vec<(Option<String>, Vec<usize>)> {
    let mut out: Vec<(Option<String>, Vec<usize>)> = Vec::new();
    for (i, p) in pasos.iter().enumerate() {
        match out.last_mut() {
            Some((pestana, posiciones)) if *pestana == p.pestana => posiciones.push(i),
            _ => out.push((p.pestana.clone(), vec![i])),
        }
    }
    out
}

/// Un paso: su casilla (si hay varios), «ya estaba así», su nota y su tabla.
fn paso(
    i: usize,
    p: &PasoVista,
    varios: bool,
    marcados: RwSignal<Vec<bool>>,
    bloqueado: Signal<bool>,
) -> impl IntoView + use<> {
    let sin_cambios = p.sin_cambios;
    let id = format!("grupo-paso-{i}");
    // Su título («TCP») o, sin él, «Paso N».
    let nombre = p.titulo.as_ref().map_or_else(
        || t!("ajustes-grupo-paso", n = i + 1),
        |c| traducir(c, None),
    );
    // La nota de un paso que solo enseña lo que cambia respecto al primero
    // («Igual que la regla TCP…»): si el primero se desmarca, su variante
    // `-sin-el-primero` lo dice.
    let nota_paso = p.nota.clone().map(|c| {
        let sin_el_primero = format!("{c}-sin-el-primero");
        let texto = move || {
            let desmarcado = i > 0 && !marcados.with(|m| m.first().copied().unwrap_or(true));
            if desmarcado && i18n::existe(idioma_actual(), &sin_el_primero) {
                traducir(&sin_el_primero, None)
            } else {
                traducir(&c, None)
            }
        };
        view! { <p class="pagina-sub grupo-paso-nota">{texto}</p> }
    });
    // En un lote, varios pasos son del mismo ajuste (el WPS y el WPA2 de
    // una red): la casilla se nombra con el paso y su elemento («Quitar el
    // WPS · Red WiFi 5 GHz»).
    let nombre_casilla = p
        .pestana
        .as_ref()
        .and(p.nota.as_ref())
        .map(|nota| format!("{} · {nombre}", traducir(nota, None)));
    let cabeza = varios.then(|| {
        view! {
            <legend class="grupo-paso-titulo">
                <input
                    id=id.clone()
                    type="checkbox"
                    aria-label=nombre_casilla
                    prop:checked=move || marcados.with(|m| m.get(i).copied().unwrap_or(false))
                    disabled=move || sin_cambios || bloqueado.get()
                    on:change=move |ev| {
                        let v = event_target_checked(&ev);
                        marcados.update(|m| {
                            if let Some(x) = m.get_mut(i) {
                                *x = v;
                            }
                        });
                    }
                />
                <label for=id>{nombre}</label>
                {sin_cambios.then(|| view! {
                    <span class="chip">{t!("ajustes-grupo-ya-esta")}</span>
                })}
            </legend>
        }
    });
    let nota =
        sin_cambios.then(|| view! { <p class="pagina-sub">{t!("ajustes-grupo-ya-esta-desc")}</p> });
    view! {
        <fieldset class="grupo-paso">
            {cabeza}
            {nota}
            {nota_paso}
            {tabla(&p.campos)}
        </fieldset>
    }
}

/// Los pasos del grupo `g`, con un encabezado por pestaña si los pasos
/// dicen la suya (los de un lote). `bloqueado`: las casillas no se tocan
/// (mientras se confirma o se sigue el cambio).
pub(super) fn lista(
    g: &GrupoVista,
    marcados: RwSignal<Vec<bool>>,
    bloqueado: Signal<bool>,
) -> impl IntoView + use<> {
    let varios = g.pasos.len() > 1;
    let bloques = por_pestana(&g.pasos)
        .into_iter()
        .map(|(pestana, posiciones)| {
            let pasos = posiciones
                .into_iter()
                .map(|i| paso(i, &g.pasos[i], varios, marcados, bloqueado))
                .collect_view();
            match pestana {
                Some(clave) => view! {
                    <section class="grupo-pestana">
                        <h3 class="grupo-pestana-titulo">{traducir(&clave, None)}</h3>
                        <div class="grupo-pasos">{pasos}</div>
                    </section>
                }
                .into_any(),
                None => view! { <div class="grupo-pasos">{pasos}</div> }.into_any(),
            }
        })
        .collect_view();
    view! {
        {varios.then(|| view! { <p class="pagina-sub">{t!("ajustes-grupo-pasos-desc")}</p> })}
        <div class="grupo-pasos">{bloques}</div>
    }
}

#[cfg(test)]
mod tests {
    use super::trozos;

    /// Una MAC, una IP y una dirección web se cortan tras sus separadores;
    /// lo demás, entero.
    #[test]
    fn trozos_tras_los_separadores() {
        assert_eq!(
            trozos("02:00:00:00:00:03"),
            ["02:", "00:", "00:", "00:", "00:", "03"]
        );
        assert_eq!(trozos("192.168.1.62"), ["192.", "168.", "1.", "62"]);
        assert_eq!(
            trozos("https://a.example/cwmp"),
            ["https:", "/", "/", "a.", "example/", "cwmp"]
        );
        assert_eq!(trozos("RED-1"), ["RED-1"]);
        assert_eq!(trozos("hora.roa.es."), ["hora.", "roa.", "es."]);
        assert!(trozos("").is_empty());
    }
}
