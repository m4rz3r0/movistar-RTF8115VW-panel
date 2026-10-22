//! Lista con columnas (Puertos, reservas y dispositivos): para lo que puede crecer a decenas.
//!
//! Una tabla a todo el ancho ([`Lista`]) con la marca redonda, el nombre con
//! su subtítulo, las columnas que declara el catálogo ([`columnas`]), el
//! estado, el interruptor si lo hay y «Abrir» ([`fila`]). Se ordena por
//! cualquier columna y, con más de ocho filas, se filtra por texto
//! ([`filtro`]). Al abrir una fila (`/ajustes/<tema>/<id>`), la lista se
//! estrecha a unos 360 px, se queda con el nombre, el estado y el
//! interruptor, y el detalle entra desde la derecha ([`detalle`]); Esc o ✕
//! lo cierran y la lista recupera su ancho. En el móvil, las filas van
//! apiladas y el detalle ocupa la pantalla. La dirección de la propia lista
//! (`/ajustes/puertos/reglas`) abre el detalle de una nueva.
//!
//! Lo marcado entra en la barra de la página como cualquier elemento
//! ([`filas_de_la_pagina`]): activar, desactivar y borrar varias a la vez.

mod columnas;
mod detalle;
mod fila;
mod filtro;

pub(crate) use columnas::Tipo;

use super::detalle::{Borrador, Borradores, Registro};
use super::piezas::{ayuda, detalles_tecnicos};
use super::rejilla::Pagina;
use super::seleccion::Seleccion;
use crate::ajustes_router::{
    AccionVista, AjusteVista, ElementoVista, EstadoLectura, InstanciaVista, PaginaTema, PieVista,
    RedCasa,
};
use crate::t;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado};
use ajustes::TipoCampo;
use columnas::{Criterio, Fila, cabecera, ordenar};
use detalle::DetalleFila;
use fila::FilaLista;
use filtro::{Filtro, coincide, hay_filtro};
use leptos::prelude::*;

/// La lista del elemento `e`, si lo es (su ajuste es una lista con
/// componente propio).
fn lista_de(e: &ElementoVista) -> Option<(Tipo, &AjusteVista)> {
    let a = e.ajustes.first()?;
    Some((Tipo::de(a)?, a))
}

/// ¿Se pinta `e` como una lista (y no como una tarjeta)?
pub(crate) fn es_lista(e: &ElementoVista) -> bool {
    lista_de(e).is_some()
}

/// Las filas de las listas de la página como elementos (para marcarlas y
/// para la barra, con las acciones de su lista), con su nombre.
pub(crate) fn filas_de_la_pagina(p: &PaginaTema) -> Vec<(ElementoVista, String)> {
    let red = p.red.clone().unwrap_or_default();
    p.elementos
        .iter()
        .filter_map(|e| lista_de(e).map(|(tipo, a)| (e, tipo, a)))
        .flat_map(|(e, tipo, a)| {
            columnas::filas(tipo, a, &red)
                .into_iter()
                .map(move |f| (como_elemento(e, &f), f.nombre))
        })
        .collect()
}

/// La fila `f` de la lista `e` como un elemento de la página.
fn como_elemento(e: &ElementoVista, f: &Fila) -> ElementoVista {
    ElementoVista {
        id: f.id.clone(),
        clave: e.clave.clone(),
        papel: None,
        ssid: None,
        icono: None,
        estado: f.estado,
        texto_estado: f.texto_estado.clone(),
        interruptor: None,
        datos: Vec::new(),
        recomendacion: None,
        pie: PieVista::default(),
        ajustes: Vec::new(),
        lectura: e.lectura.clone(),
        acciones: e.acciones.clone(),
    }
}

/// Namespace de lo editado en el formulario de una fila (en [`Borradores`]).
const BORRADOR: &str = "fila";

/// Sigue lo editado en el formulario de la fila `fila`: `campos` son el
/// nombre de cada campo, su valor en edición y el leído. Lo que se editó
/// antes de que la página volviera a leer vuelve a su sitio; desde ahora,
/// se guarda mientras se edita, y `registro` dice si hay cambios sin
/// guardar. Devuelve con qué olvidarlo (al preparar el cambio).
pub(crate) fn seguir_lo_editado(
    fila: String,
    campos: Vec<(String, RwSignal<String>, String)>,
    registro: Registro,
) -> Callback<()> {
    let borradores = expect_context::<Borradores>();
    for ((ns, campo), v) in borradores.de(&fila) {
        if let Some((_, s, _)) = campos
            .iter()
            .find(|(k, _, _)| ns == BORRADOR && *k == campo)
        {
            s.set(v);
        }
    }
    let campos = StoredValue::new(campos);
    let sucio = Signal::derive(move || {
        campos.with_value(|cs| cs.iter().any(|(_, s, leido)| s.get() != *leido))
    });
    registro.set(Some((fila.clone(), sucio)));
    let id = fila.clone();
    Effect::new(move |_| {
        let cambios: Borrador = campos.with_value(|cs| {
            cs.iter()
                .filter_map(|(k, s, leido)| {
                    let v = s.get();
                    (v != *leido).then(|| ((BORRADOR.to_owned(), k.clone()), v))
                })
                .collect()
        });
        borradores.guardar(&id, cambios);
    });
    Callback::new(move |()| borradores.olvidar(&fila))
}

/// Lo que comparten las filas de una lista.
#[derive(Clone, Copy)]
pub(crate) struct DatosLista {
    pub tipo: Tipo,
    /// Lleva la marca redonda (la lista tiene acciones).
    pub marcar: bool,
    /// Se puede cambiar (ni solo lectura ni sin leer).
    pub editable: bool,
    /// Tiene la columna del interruptor.
    pub con_interruptor: bool,
    /// El texto de la cabecera de cada columna (para las filas apiladas).
    pub cabeceras: StoredValue<Vec<String>>,
    pub instancias: StoredValue<Vec<InstanciaVista>>,
    /// Id del ajuste de la lista y del campo del interruptor.
    pub ajuste: StoredValue<String>,
    pub campo_interruptor: StoredValue<String>,
}

/// El campo sí/no que se puede escribir entre las columnas de `a`: el del
/// interruptor de cada fila.
fn campo_interruptor(a: &AjusteVista) -> Option<String> {
    a.columnas
        .iter()
        .find(|col| {
            a.campos
                .iter()
                .any(|c| c.id == **col && c.tipo == TipoCampo::Booleano && c.solo_lectura.is_none())
        })
        .cloned()
}

/// Botón de la cabecera que ordena por `columna` (`None`: el nombre), con
/// `aria-sort` en su celda.
fn cabeza(
    texto: String,
    columna: Option<usize>,
    criterio: RwSignal<Option<Criterio>>,
    clase: &'static str,
) -> impl IntoView + use<> {
    let orden = move || {
        criterio.get().filter(|c| c.columna == columna).map(|c| {
            if c.ascendente {
                "ascending"
            } else {
                "descending"
            }
        })
    };
    // Sin criterio, la lista va como la da el router: la primera pulsación
    // en cualquier cabecera ordena de menor a mayor.
    let pulsar = move |_| {
        criterio.update(|c| {
            let actual = c.unwrap_or(Criterio {
                columna: None,
                ascendente: false,
            });
            *c = Some(actual.al_pulsar(columna));
        });
    };
    view! {
        <th scope="col" class=clase aria-sort=orden>
            <button type="button" class="lista-ordenar" on:click=pulsar>
                {texto}
                <span class="lista-flecha" aria-hidden="true"></span>
            </button>
        </th>
    }
}

/// La lista del elemento `e`. `red`: la red de casa de la página;
/// `acciones`, las de la página (la de borrar de esta lista va también en
/// su detalle); `existentes`, las IP fijas de la página (`(id, mac, ip)`),
/// para saber si se puede fijar la IP de un dispositivo.
#[component]
pub(crate) fn Lista(
    e: ElementoVista,
    red: RedCasa,
    acciones: Vec<AccionVista>,
    existentes: Vec<(String, String, String)>,
    registro: Registro,
) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let sel = expect_context::<Seleccion>();
    let Some((tipo, a)) = lista_de(&e).map(|(t, a)| (t, a.clone())) else {
        return ().into_any();
    };
    let leida = e.lectura == EstadoLectura::Leido;
    let filas: Vec<Fila> = columnas::filas(tipo, &a, &red);
    let ids: Vec<String> = filas.iter().map(|f| f.id.clone()).collect();
    let interruptor = campo_interruptor(&a);
    let cabeceras: Vec<String> = a.columnas.iter().map(|c| cabecera(&a, c)).collect();
    let d = DatosLista {
        tipo,
        marcar: leida && !e.acciones.is_empty(),
        editable: leida && !pagina.solo_lectura,
        con_interruptor: interruptor.is_some(),
        cabeceras: StoredValue::new(cabeceras.clone()),
        instancias: StoredValue::new(a.instancias.clone()),
        ajuste: StoredValue::new(a.id.clone()),
        campo_interruptor: StoredValue::new(interruptor.clone().unwrap_or_default()),
    };
    let borrar = acciones
        .iter()
        .find(|x| x.peligro && e.acciones.contains(&x.id))
        .map(|x| x.id.clone());
    let criterio = RwSignal::new(None::<Criterio>);
    let texto = RwSignal::new(String::new());
    let con_filtro = hay_filtro(filas.len());
    let id_lista = e.id.clone();
    // Abierta: una de sus filas o la nueva (la dirección de la lista).
    let abierta = {
        let (ids, id_lista) = (ids.clone(), id_lista.clone());
        Memo::new(move |_| {
            pagina
                .abierto
                .get()
                .filter(|a| *a == id_lista || ids.contains(a))
        })
    };
    let todas = StoredValue::new(filas);
    let visibles = move || {
        let escrito = texto.get();
        let mut v: Vec<Fila> = todas.with_value(|fs| {
            fs.iter()
                .filter(|f| coincide(f, &escrito))
                .cloned()
                .collect()
        });
        ordenar(&mut v, criterio.get());
        v
    };
    let ninguna = move || {
        let escrito = texto.get();
        let hay = todas.with_value(|fs| fs.iter().any(|f| coincide(f, &escrito)));
        (!hay && !escrito.trim().is_empty()).then(|| {
            view! { <p class="lista-ninguna">{t!("ajustes-lista-ninguna", texto = escrito.clone())}</p> }
        })
    };
    let titulo_id = format!("lista-{}-titulo", e.id);
    let titulo = traducir(&format!("{}-titulo", a.clave), None);
    let desc = traducir(&format!("{}-desc", a.clave), None);
    // Cabecera de la tabla: lo que no es el interruptor, el estado y él.
    let cab_datos = a
        .columnas
        .iter()
        .enumerate()
        .filter(|(_, c)| Some(*c) != interruptor.as_ref())
        .map(|(i, _)| cabeza(cabeceras[i].clone(), Some(i), criterio, "lista-dato"))
        .collect_view();
    let cab_interruptor = interruptor.as_ref().and_then(|col| {
        let i = a.columnas.iter().position(|c| c == col)?;
        Some(cabeza(cabeceras[i].clone(), Some(i), criterio, "lista-int"))
    });
    let nueva = (tipo.admite_nuevas() && d.editable).then(|| {
        let id = id_lista.clone();
        let texto = match tipo {
            Tipo::Reglas => t!("ajuste-puertos-reglas-abrir"),
            _ => t!("ajuste-red_local-reservas-anadir"),
        };
        view! {
            <button type="button" class="b-sec lista-nueva" on:click=move |_| pagina.abrir.run(id.clone())>
                {texto}
            </button>
        }
    });
    let vacia = todas.with_value(Vec::is_empty).then(|| {
        let clave = format!("{}-vacia", a.clave);
        view! { <p class="ayuda lista-vacia">{traducir(&clave, None)}</p> }
    });
    let sin_leer = match &e.lectura {
        EstadoLectura::SinLeer(m) => Some(view! {
            <div class="tarjeta-sin-leer">
                <p class="aviso estado-error">
                    <IconoEstado estado=Estado::Error />
                    <span>{m.clone()}</span>
                </p>
                <button type="button" class="b-sec" on:click=move |_| pagina.volver_a_leer.run(())>
                    {t!("ajustes-volver-a-leer")}
                </button>
            </div>
        }),
        EstadoLectura::Leido => None,
    };
    let tabla = (leida && !todas.with_value(Vec::is_empty)).then(|| {
        view! {
            <table class="lista" aria-labelledby=titulo_id.clone()>
                <thead>
                    <tr>
                        {d.marcar.then(|| view! {
                            <td class="lista-marca"></td>
                        })}
                        {cabeza(t!("ajustes-lista-nombre"), None, criterio, "lista-nombre")}
                        {cab_datos}
                        <th scope="col" class="lista-estado">{t!("ajustes-lista-estado")}</th>
                        {cab_interruptor}
                        <td class="lista-abrir"></td>
                    </tr>
                </thead>
                <tbody class:hay-marcadas=move || { sel.cuantas() > 0 }>
                    {move || {
                        visibles()
                            .into_iter()
                            .map(|f| view! { <FilaLista f d /> })
                            .collect_view()
                    }}
                </tbody>
            </table>
        }
    });
    let detalle = {
        let (a, red) = (a.clone(), red.clone());
        let existentes = StoredValue::new(existentes);
        move || {
            abierta.get().map(|abierto| {
                let fila = todas.with_value(|fs| fs.iter().find(|f| f.id == abierto).cloned());
                view! {
                    <DetalleFila
                        tipo
                        a=a.clone()
                        fila
                        abierto
                        red=red.clone()
                        existentes=existentes.get_value()
                        borrar=borrar.clone()
                        registro
                    />
                }
            })
        }
    };
    view! {
        <section class="lista-seccion" aria-labelledby=titulo_id.clone()>
            <header class="lista-cabecera">
                <div>
                    <h2 id=titulo_id.clone()>{titulo}</h2>
                    <p class="lista-desc">{desc}</p>
                </div>
                {nueva}
            </header>
            {ayuda(&a)}
            {sin_leer}
            {con_filtro.then(|| view! { <Filtro id=format!("lista-{}-filtro", e.id) texto /> })}
            <div class="lista-cuerpo" class:lista-abierta=move || abierta.with(Option::is_some)>
                <div class="lista-tabla">
                    {tabla}
                    {ninguna}
                    {leida.then_some(vacia)}
                </div>
                {detalle}
            </div>
            {detalles_tecnicos(&a.clave, &a.tecnico)}
        </section>
    }
    .into_any()
}
