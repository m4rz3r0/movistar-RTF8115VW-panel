//! Tarjeta de un ajuste ([`Tarjeta`]): título, nivel, riesgo, descripción,
//! «¿Qué hace esto?», valor actual resumido con su recomendación, edición en
//! el sitio («Editar» → campos → «Guardar» abre [`DialogoGrupo`]; «Cancelar»
//! lo deja como estaba) y, al final, los detalles técnicos plegados: lo
//! único que nombra la API.
//!
//! Un ajuste que no se pudo leer muestra el motivo y no se edita. Uno con
//! componente propio (reservas, puertos…) es por ahora un hueco con su
//! título y descripción; una lista sin componente se ve como tabla.

use super::Foco;
use super::campo::{
    Campo, cambiado, elegido, es_recomendado, etiqueta, inicial, texto_legible, validar,
    valor_resumido,
};
use crate::ajustes_router::{
    AjusteVista, CampoVista, DetallesTecnicos, EstadoLectura, GrupoVista, PeticionAjuste,
    preparar_ajuste,
};
use crate::t;
use crate::ui::explorador::{RiesgoCambio, texto_error};
use crate::ui::grupo::DialogoGrupo;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, IconoEstado, Insignia};
use ajustes::Nivel;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use std::collections::BTreeMap;

/// Prefijo de los ids de la tarjeta en la página (`ajuste-wifi-red_24`).
fn dom(a: &AjusteVista) -> String {
    format!("ajuste-{}", a.id.replace('.', "-"))
}

/// Enfoca el elemento `id` de la página, si existe.
pub(super) fn enfocar(id: &str) {
    use leptos::wasm_bindgen::JsCast;
    if let Some(e) = document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok())
    {
        let _ = e.focus();
    }
}

/// Etiqueta de nivel: nada en lo esencial; «Avanzado» u «Oculto» (con lo
/// que significa) en lo demás.
fn nivel(n: Nivel) -> Option<AnyView> {
    match n {
        Nivel::Basico => None,
        Nivel::Avanzado => Some(
            view! { <span class="chip chip-nivel">{t!("ajustes-nivel-avanzado")}</span> }
                .into_any(),
        ),
        Nivel::Oculto => Some(
            view! {
                <span class="chip chip-nivel chip-oculto">{t!("ajustes-nivel-oculto")}</span>
                <span class="nivel-nota">{t!("ajustes-nivel-oculto-desc")}</span>
            }
            .into_any(),
        ),
    }
}

fn riesgo(r: RiesgoCambio) -> AnyView {
    let (texto, desc) = match r {
        RiesgoCambio::Bajo => (t!("ajustes-riesgo-bajo"), t!("ajustes-riesgo-bajo-desc")),
        RiesgoCambio::Medio => (t!("ajustes-riesgo-medio"), t!("ajustes-riesgo-medio-desc")),
        RiesgoCambio::Alto => (t!("ajustes-riesgo-alto"), t!("ajustes-riesgo-alto-desc")),
    };
    view! {
        <Insignia estado=r.estado() texto=texto />
        <span class="solo-lector">{desc}</span>
    }
    .into_any()
}

/// Cabecera común: título (enfocable, para volver a él tras guardar), nivel,
/// riesgo si se puede cambiar, y la descripción corta.
fn cabecera(a: &AjusteVista, editable: bool) -> impl IntoView + use<> {
    let titulo_id = format!("{}-titulo", dom(a));
    view! {
        <header class="ajuste-cabecera">
            <h2 id=titulo_id tabindex="-1">{traducir(&format!("{}-titulo", a.clave), None)}</h2>
            <div class="ajuste-etiquetas">
                {nivel(a.nivel)}
                {editable.then(|| riesgo(a.riesgo))}
            </div>
        </header>
        <p class="ajuste-desc">{traducir(&format!("{}-desc", a.clave), None)}</p>
    }
}

/// «¿Qué hace esto?»: la ayuda larga, plegada.
fn ayuda(a: &AjusteVista) -> impl IntoView + use<> {
    view! {
        <details class="ajuste-ayuda">
            <summary>{t!("ajustes-que-hace")}</summary>
            <p>{traducir(&format!("{}-ayuda", a.clave), None)}</p>
        </details>
    }
}

/// Funciones y parámetros del router detrás del ajuste, plegados. Lo único
/// de la página con nombres de la API.
fn detalles_tecnicos(clave: &str, d: &DetallesTecnicos) -> impl IntoView + use<> {
    let parametros = d
        .parametros
        .iter()
        .map(|(campo, param)| {
            view! {
                <tr>
                    <th scope="row">{traducir(&format!("{clave}-{campo}"), None)}</th>
                    <td><code class="mono">{param.clone()}</code></td>
                </tr>
            }
        })
        .collect_view();
    let escrituras = (!d.escrituras.is_empty()).then(|| {
        let lista = d
            .escrituras
            .iter()
            .map(|e| view! { <code class="mono">{e.clone()}</code> " " })
            .collect_view();
        view! {
            <div>
                <dt>{t!("ajustes-detalles-escrituras")}</dt>
                <dd>{lista}</dd>
            </div>
        }
    });
    let con_parametros = !d.parametros.is_empty();
    view! {
        <details class="detalles-tecnicos">
            <summary>{t!("ajustes-detalles-tecnicos")}</summary>
            <p class="ayuda">{t!("ajustes-detalles-tecnicos-desc")}</p>
            <dl class="detalles-lista">
                <div>
                    <dt>{t!("ajustes-detalles-lectura")}</dt>
                    <dd><code class="mono">{d.lectura.clone()}</code></dd>
                </div>
                {escrituras}
            </dl>
            {con_parametros.then(|| view! {
                <div class="tabla-scroll">
                    <table class="tabla">
                        <caption class="solo-lector">{t!("ajustes-detalles-parametros")}</caption>
                        <thead>
                            <tr>
                                <th scope="col">{t!("ajustes-grupo-col-dato")}</th>
                                <th scope="col">{t!("ajustes-detalles-parametros")}</th>
                            </tr>
                        </thead>
                        <tbody>{parametros}</tbody>
                    </table>
                </div>
            })}
        </details>
    }
}

/// Motivos (sin repetir) de los campos que solo se pueden consultar.
fn motivos(campos: &[CampoVista]) -> Option<impl IntoView + use<>> {
    let mut claves: Vec<&String> = campos
        .iter()
        .filter_map(|c| c.solo_lectura.as_ref())
        .collect();
    claves.dedup();
    (!claves.is_empty()).then(|| {
        claves
            .into_iter()
            .map(|k| view! { <p class="nota-solo-lectura">{traducir(k, None)}</p> })
            .collect_view()
    })
}

/// Tarjeta de un ajuste del tema. `solo_lectura`: el panel no escribe
/// (firmware sin confirmar). `recargar` vuelve a leer el tema tras aplicar.
#[component]
pub(crate) fn Tarjeta(
    a: AjusteVista,
    solo_lectura: bool,
    #[prop(into)] recargar: Callback<()>,
) -> impl IntoView {
    let id_dom = dom(&a);
    let cuerpo = if a.componente.is_some() {
        hueco(&a).into_any()
    } else if a.lista {
        lista(&a).into_any()
    } else {
        view! { <TarjetaAjuste a solo_lectura recargar /> }.into_any()
    };
    view! {
        <article class="tarjeta ajuste" aria-labelledby=format!("{id_dom}-titulo")>
            {cuerpo}
        </article>
    }
}

/// Ajuste con componente propio: por ahora, su título y su descripción.
fn hueco(a: &AjusteVista) -> impl IntoView + use<> {
    view! {
        {cabecera(a, false)}
        <p class="nota-hueco">{t!("ajustes-hueco")}</p>
    }
}

/// Lista sin componente propio: sus instancias en una tabla, sin editar.
fn lista(a: &AjusteVista) -> impl IntoView + use<> {
    let sin_leer = match &a.estado {
        EstadoLectura::SinLeer(m) => Some(m.clone()),
        EstadoLectura::Leido => None,
    };
    let cuerpo = if let Some(m) = sin_leer {
        view! { <p class="aviso estado-error" role="status"><IconoEstado estado=Estado::Error /><span>{m}</span></p> }
            .into_any()
    } else if a.instancias.is_empty() {
        view! { <p class="ayuda">{t!("ajustes-lista-vacia")}</p> }.into_any()
    } else {
        let cabeza = a
            .campos
            .iter()
            .map(|c| view! { <th scope="col">{etiqueta(c)}</th> })
            .collect_view();
        let filas = a
            .instancias
            .iter()
            .map(|i| {
                let celdas = i
                    .campos
                    .iter()
                    .map(|c| view! { <td>{valor_resumido(c)}</td> })
                    .collect_view();
                view! { <tr>{celdas}</tr> }
            })
            .collect_view();
        view! {
            <div class="tabla-scroll">
                <table class="tabla">
                    <thead><tr>{cabeza}</tr></thead>
                    <tbody>{filas}</tbody>
                </table>
            </div>
        }
        .into_any()
    };
    let todo_consulta = a.campos.iter().all(|c| c.solo_lectura.is_some());
    view! {
        {cabecera(a, false)}
        {ayuda(a)}
        {cuerpo}
        {if todo_consulta {
            motivos(&a.campos).into_any()
        } else {
            view! { <p class="nota-solo-lectura">{t!("ajustes-lista-sin-editar")}</p> }.into_any()
        }}
        {detalles_tecnicos(&a.clave, &a.tecnico)}
    }
}

/// Una fila de la vista resumida: etiqueta, valor y, si no es el
/// recomendado, cuál lo es (cuando coincide no se marca: menos ruido).
fn fila(c: &CampoVista) -> impl IntoView + use<> {
    let marca = (es_recomendado(c) == Some(false)).then(|| {
        let r = c.recomendado.clone().unwrap_or_default();
        view! {
            <span class="chip chip-sugerencia">
                {t!("ajustes-recomendado-valor", valor = texto_legible(c, &r))}
            </span>
        }
    });
    view! {
        <div class="ajuste-fila">
            <dt>{etiqueta(c)}</dt>
            <dd>{valor_resumido(c)} {marca}</dd>
        </div>
    }
}

/// Ajuste con la tarjeta genérica: vista resumida y edición en el sitio.
#[component]
fn TarjetaAjuste(a: AjusteVista, solo_lectura: bool, recargar: Callback<()>) -> impl IntoView {
    let id_dom = dom(&a);
    let leido = matches!(a.estado, EstadoLectura::Leido);
    let editable = leido && !solo_lectura && a.campos.iter().any(|c| c.solo_lectura.is_none());
    let campos = StoredValue::new(a.campos.clone());
    let valores: Vec<RwSignal<String>> = a
        .campos
        .iter()
        .map(|c| RwSignal::new(inicial(&c.valor)))
        .collect();
    let errores: Vec<RwSignal<Option<String>>> =
        a.campos.iter().map(|_| RwSignal::new(None)).collect();
    let valores = StoredValue::new(valores);
    let errores = StoredValue::new(errores);
    let editando = RwSignal::new(false);
    let preparando = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let grupo = RwSignal::new(None::<GrupoVista>);
    let boton_editar = NodeRef::<html::Button>::new();
    let foco = expect_context::<Foco>();
    let control = {
        let id_dom = id_dom.clone();
        move |i: usize| format!("{id_dom}-c{i}-control")
    };

    let abrir = {
        let control = control.clone();
        move |_| {
            editando.set(true);
            // El foco, al primer campo que se puede cambiar.
            let primero = campos.with_value(|cs| cs.iter().position(|c| c.solo_lectura.is_none()));
            let control = control.clone();
            request_animation_frame(move || {
                if let Some(i) = primero {
                    enfocar(&control(i));
                }
            });
        }
    };
    let cancelar = move |_| {
        campos.with_value(|cs| {
            for (i, c) in cs.iter().enumerate() {
                valores.with_value(|v| v[i].set(inicial(&c.valor)));
                errores.with_value(|e| e[i].set(None));
            }
        });
        error.set(None);
        editando.set(false);
        request_animation_frame(move || {
            if let Some(b) = boton_editar.get_untracked() {
                let _ = b.focus();
            }
        });
    };

    let id_ajuste = a.id.clone();
    let guardar = {
        let control = control.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if preparando.get_untracked() {
                return;
            }
            error.set(None);
            let mut pedir = BTreeMap::new();
            let mut primero_mal = None::<usize>;
            campos.with_value(|cs| {
                let v = |i: usize| valores.with_value(|v| v[i].get_untracked());
                let marcar = |i: usize, m: String| errores.with_value(|e| e[i].set(Some(m)));
                for (i, c) in cs.iter().enumerate() {
                    errores.with_value(|e| e[i].set(None));
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
                // Un campo que exige otro (encender el espejo exige la
                // toma): el exigido va también, o se avisa junto a él.
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
                    if elegido(&v(q)) {
                        pedir.entry(r.campo.clone()).or_insert_with(|| v(q));
                    } else {
                        marcar(q, t!("ajustes-error-requiere", campo = etiqueta(c)));
                        primero_mal.get_or_insert(q);
                    }
                }
            });
            if let Some(i) = primero_mal {
                enfocar(&control(i));
                return;
            }
            if pedir.is_empty() {
                error.set(Some(t!("ajustes-error-sin-cambios")));
                return;
            }
            preparando.set(true);
            let p = PeticionAjuste {
                ajuste: id_ajuste.clone(),
                instancia: None,
                borrar: false,
                valores: pedir,
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => grupo.set(Some(g)),
                    Err(e) => error.set(Some(texto_error(&e))),
                }
                preparando.set(false);
            });
        }
    };

    let dom_cerrar = id_dom.clone();
    let cerrar = Callback::new(move |aplicado: Option<i64>| {
        grupo.set(None);
        if aplicado.is_some() {
            editando.set(false);
            foco.0.set(Some(format!("{}-titulo", dom_cerrar)));
            recargar.run(());
        }
    });

    let sin_leer = match &a.estado {
        EstadoLectura::SinLeer(m) => Some(view! {
            <p class="aviso estado-error" role="status">
                <IconoEstado estado=Estado::Error />
                <span>{m.clone()}</span>
            </p>
        }),
        EstadoLectura::Leido => None,
    };
    let resumen = {
        let filas = a.campos.iter().map(fila).collect_view();
        let motivos = motivos(&a.campos);
        view! {
            <dl class="ajuste-valores">{filas}</dl>
            {motivos}
        }
    };
    let formulario = {
        let filas = a
            .campos
            .iter()
            .enumerate()
            .map(|(i, c)| {
                if c.solo_lectura.is_some() {
                    view! {
                        <dl class="ajuste-valores">{fila(c)}</dl>
                    }
                    .into_any()
                } else {
                    let valor = valores.with_value(|v| v[i]);
                    let error = errores.with_value(|e| e[i]);
                    view! {
                        <Campo
                            c=c.clone()
                            dom=format!("{id_dom}-c{i}")
                            valor
                            error
                            bloqueado=preparando
                        />
                    }
                    .into_any()
                }
            })
            .collect_view();
        let efecto = a.efecto.clone().map(|e| {
            view! {
                <p class="ajuste-efecto">
                    <strong>{t!("ajustes-efecto")}</strong>
                    ": "
                    {traducir(&e, None)}
                </p>
            }
        });
        let motivos = motivos(&a.campos);
        view! {
            <form class="ajuste-formulario" novalidate=true on:submit=guardar>
                <div class="ajuste-campos">{filas}</div>
                {motivos}
                {efecto}
                <p class="error-formulario" role="alert">{move || error.get()}</p>
                <div class="ajuste-acciones">
                    <button class="boton-sutil" type="button" disabled=move || preparando.get() on:click=cancelar>
                        {t!("ajustes-cancelar")}
                    </button>
                    // Sin `disabled`: el botón conserva el foco y el diálogo
                    // se lo devuelve al cerrarse (el envío ya se ignora
                    // mientras prepara).
                    <button
                        class="boton"
                        type="submit"
                        aria-disabled=move || preparando.get().then_some("true")
                    >
                        {move || if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-guardar") }}
                    </button>
                </div>
            </form>
        }
    };
    // La vista y el formulario se pintan una vez; solo se alterna cuál se ve.
    view! {
        {cabecera(&a, editable)}
        {ayuda(&a)}
        {sin_leer}
        {leido.then(|| view! {
            <div class="ajuste-resumen" class:oculto=move || editando.get()>
                {resumen}
                {editable.then(|| view! {
                    <div class="ajuste-acciones">
                        <button class="boton-sutil" type="button" node_ref=boton_editar on:click=abrir>
                            {t!("ajustes-editar")}
                        </button>
                    </div>
                })}
            </div>
        })}
        {editable.then(|| view! {
            <div class="ajuste-edicion" class:oculto=move || !editando.get()>{formulario}</div>
        })}
        {detalles_tecnicos(&a.clave, &a.tecnico)}
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}
