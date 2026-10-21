//! Explorador (`/explorador`): todas las funciones del catálogo del router.
//!
//! - Lateral con las áreas (y cuántas funciones tiene cada una) y búsqueda
//!   por nombre o descripción en todas las áreas.
//! - `/explorador/:area`: funciones del área con su tipo, origen y, si el
//!   panel no las usa, el motivo.
//! - `/explorador/:area/:funcion` (nombre codificado con [`codificar`]):
//!   ficha con parámetros; las lecturas se ejecutan y se muestran como tabla
//!   clave/valor o de instancias (secretos censurados, con el ojo); las
//!   escrituras y acciones tienen un formulario generado del catálogo que
//!   pasa por el gestor de cambios ([`DialogoCambio`]).
//!
//! La interfaz se traduce; lo que viene del catálogo (áreas, descripciones,
//! etiquetas de parámetros y opciones) está en español y va con `lang="es"`.

mod datos;
mod dialogo;
mod formulario;
#[cfg(feature = "ssr")]
pub(crate) mod servidor;

pub use datos::*;
pub(crate) use dialogo::abrir_modal;
pub use dialogo::{DialogoCambio, TablaDiff};

use crate::t;
use crate::ui::secreto::SecretoLeido;
use crate::ui::{Estado, IconoEstado, Insignia};
use formulario::Escritura;
use leptos::prelude::*;
use leptos_router::{
    components::{A, Outlet},
    hooks::use_params_map,
};

/// Catálogo compartido por las páginas del explorador.
#[derive(Clone, Copy)]
struct Catalogo(Resource<Result<Vec<AreaVista>, ServerFnError>>);

/// Texto del error de una server function para la interfaz.
pub(crate) fn texto_error(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(msg) => super::mensaje_del_servidor(msg),
        _ => t!("biblioteca-error-conexion"),
    }
}

/// «30 s», «2 min», «5 min 30 s» (unidades iguales en los dos idiomas).
pub(crate) fn duracion(s: u64) -> String {
    match (s / 60, s % 60) {
        (0, s) => format!("{s} s"),
        (m, 0) => format!("{m} min"),
        (m, s) => format!("{m} min {s} s"),
    }
}

/// Etiquetas de una función: tipo, origen y, si procede, «Bloqueada».
#[component]
fn Etiquetas(f: FuncionResumen) -> impl IntoView {
    let origen_clase = match f.origen {
        OrigenFuncion::Oculta | OrigenFuncion::Incompleta => "chip chip-aviso",
        _ => "chip",
    };
    view! {
        <span class="etiquetas">
            <span class=format!("chip chip-{}", f.tipo.clase())>
                {f.tipo.etiqueta()}
            </span>
            <span class=origen_clase title=f.origen.ayuda()>
                {f.origen.etiqueta()}
            </span>
            {f.bloqueada.is_some().then(|| view! { <Insignia estado=Estado::Error texto=t!("biblioteca-bloqueada") /> })}
        </span>
    }
}

/// Marco del explorador: lateral con áreas y búsqueda, y la página hija.
#[component]
pub fn Explorador() -> impl IntoView {
    let cat = Resource::new(|| (), |_| catalogo_explorador());
    provide_context(Catalogo(cat));
    let (busqueda, set_busqueda) = signal(String::new());

    let lateral = move || {
        Suspend::new(async move {
            let areas = match cat.await {
                Ok(a) => a,
                Err(e) => {
                    return view! { <p class="aviso estado-error" role="alert">{texto_error(&e)}</p> }
                        .into_any();
                }
            };
            let areas = StoredValue::new(areas);
            let lista = move || {
                let q = busqueda.get().trim().to_lowercase();
                if q.is_empty() {
                    let items = areas.with_value(|areas| {
                        areas
                            .iter()
                            .map(|a| {
                                let href = format!("/explorador/{}", a.slug);
                                let n = a.funciones.len();
                                let etiqueta = a.etiqueta.clone();
                                view! {
                                    <li>
                                        <A href=href>
                                            <span lang="es">{etiqueta}</span>
                                            <span class="cuenta cifras" aria-label=t!("biblioteca-funciones", n = n)>
                                                {n}
                                            </span>
                                        </A>
                                    </li>
                                }
                            })
                            .collect_view()
                    });
                    view! {
                        <nav aria-label=t!("biblioteca-areas")>
                            <ul class="lista-areas">{items}</ul>
                        </nav>
                    }
                    .into_any()
                } else {
                    let encontradas: Vec<(FuncionResumen, String)> = areas.with_value(|areas| {
                        areas
                            .iter()
                            .flat_map(|a| a.funciones.iter().map(move |f| (f, &a.etiqueta)))
                            .filter(|(f, _)| {
                                f.nombre.to_lowercase().contains(&q)
                                    || f.descripcion.to_lowercase().contains(&q)
                            })
                            .map(|(f, e)| (f.clone(), e.clone()))
                            .collect()
                    });
                    let total = encontradas.len();
                    let items = encontradas
                        .into_iter()
                        .take(40)
                        .map(|(f, area)| {
                            let href = ruta_funcion(&f.area, &f.nombre);
                            view! {
                                <li>
                                    <A href=href>
                                        <span class="mono">{f.nombre}</span>
                                        <span class="resultado-area" lang="es">{area}</span>
                                    </A>
                                </li>
                            }
                        })
                        .collect_view();
                    view! {
                        <p class="resultados-cuenta" role="status">
                            {match total {
                                0 => t!("biblioteca-ninguna-coincide"),
                                n if n > 40 => t!("biblioteca-funciones-parcial", n = n, mostradas = 40),
                                n => t!("biblioteca-funciones", n = n),
                            }}
                        </p>
                        <ul class="lista-resultados">{items}</ul>
                    }
                    .into_any()
                }
            };
            lista.into_any()
        })
    };

    view! {
        <div class="explorador">
            <aside class="explorador-lateral">
                <p class="campo buscador">
                    <label for="buscar-funcion">{t!("biblioteca-buscar")}</label>
                    <input
                        id="buscar-funcion"
                        type="search"
                        placeholder=t!("biblioteca-buscar-placeholder")
                        autocomplete="off"
                        spellcheck="false"
                        prop:value=busqueda
                        on:input=move |ev| set_busqueda.set(event_target_value(&ev))
                    />
                </p>
                <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
                    {lateral}
                </Transition>
            </aside>
            <div class="explorador-pagina">
                // El catálogo, la ficha, la lectura, la lista y el formulario
                // llegan o se hacen en el navegador: sin JavaScript, o con el
                // wasm caído, la página se quedaría en «Cargando…» sin decir
                // por qué. El aviso lo oculta el CSS en cuanto hidrata. El
                // `<noscript>` lleva solo texto: con JavaScript, el navegador
                // lo guarda como texto y así hidrata igual.
                <p class="aviso estado-neutro aviso-sin-hidratar" role="status">
                    <IconoEstado estado=Estado::Neutro />
                    <span>
                        <noscript>{t!("biblioteca-sin-js")}</noscript>
                        " "
                        {t!("biblioteca-sin-hidratar")}
                    </span>
                </p>
                <Outlet />
            </div>
        </div>
    }
    .into_any()
}

/// `/explorador`: presentación y resumen por áreas.
#[component]
pub fn InicioExplorador() -> impl IntoView {
    let Catalogo(cat) = expect_context::<Catalogo>();
    let tarjetas = move || {
        Suspend::new(async move {
            let areas = cat.await.unwrap_or_default();
            let total: usize = areas.iter().map(|a| a.funciones.len()).sum();
            let tarjetas = areas
                .into_iter()
                .map(|a| {
                    let cuenta = |tipo| a.funciones.iter().filter(|f| f.tipo == tipo).count();
                    let resumen = [
                        t!("biblioteca-n-lecturas", n = cuenta(TipoFuncion::Lectura)),
                        t!("biblioteca-n-escrituras", n = cuenta(TipoFuncion::Escritura)),
                        t!("biblioteca-n-acciones", n = cuenta(TipoFuncion::Accion)),
                    ]
                    .join(" · ");
                    view! {
                        <li>
                            <a class="tarjeta tarjeta-enlace" href=format!("/explorador/{}", a.slug)>
                                <h2 lang="es">{a.etiqueta}</h2>
                                <p class="pagina-sub">{resumen}</p>
                            </a>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <p class="pagina-sub">
                    {t!("biblioteca-inicio-total", n = total)}
                </p>
                <ul class="rejilla-tarjetas lista-limpia">{tarjetas}</ul>
            }
        })
    };
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("biblioteca-titulo")}</h1>
            </div>
        </header>
        <p>{t!("biblioteca-inicio-texto")}</p>
        <Transition fallback=|| ()>{tarjetas}</Transition>
    }
    .into_any()
}

/// `/explorador/:area`: funciones de un área.
#[component]
pub fn PaginaArea() -> impl IntoView {
    let Catalogo(cat) = expect_context::<Catalogo>();
    let params = use_params_map();
    let contenido = move || {
        let slug = params.with(|p| p.get("area").unwrap_or_default());
        Suspend::new(async move {
            let areas = match cat.await {
                Ok(a) => a,
                Err(e) => {
                    return view! { <p class="aviso estado-error" role="alert">{texto_error(&e)}</p> }
                        .into_any();
                }
            };
            let Some(area) = areas.into_iter().find(|a| a.slug == slug) else {
                return view! {
                    <header class="pagina-cabecera">
                        <h1>{t!("biblioteca-area-desconocida")}</h1>
                    </header>
                    <p>{t!("biblioteca-elige-area")}</p>
                }
                .into_any();
            };
            let n = area.funciones.len();
            let items = area
                .funciones
                .into_iter()
                .map(|f| {
                    let href = ruta_funcion(&f.area, &f.nombre);
                    let descripcion = f.descripcion.clone();
                    view! {
                        <li class="funcion-item">
                            <div class="funcion-item-cabecera">
                                <a class="mono funcion-nombre" href=href>{f.nombre.clone()}</a>
                                <Etiquetas f=f.clone() />
                            </div>
                            <p class="funcion-descripcion" lang="es">{descripcion}</p>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <header class="pagina-cabecera">
                    <div>
                        <h1 lang="es">{area.etiqueta}</h1>
                        <p class="pagina-sub">{t!("biblioteca-funciones", n = n)}</p>
                    </div>
                </header>
                <ul class="lista-funciones">{items}</ul>
            }
            .into_any()
        })
    };
    view! { <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>{contenido}</Transition> }
        .into_any()
}

/// Tabla de parámetros de la función.
fn tabla_parametros(params: &[ParamVista]) -> impl IntoView + use<> {
    let filas = params
        .iter()
        .map(|p| {
            let opciones = if p.opciones.is_empty() {
                view! { <span class="vacia" aria-hidden="true">"—"</span> }.into_any()
            } else {
                p.opciones
                    .iter()
                    .map(|(v, e)| format!("{v} = {e}"))
                    .collect::<Vec<_>>()
                    .join(" · ")
                    .into_any()
            };
            view! {
                <tr>
                    <td><span class="mono">{p.nombre.clone()}</span></td>
                    <td lang="es">{p.etiqueta.clone()}</td>
                    <td>{p.tipo.etiqueta()}</td>
                    <td class="celda-larga" lang="es">{opciones}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="tabla-scroll">
            <table class="tabla">
                <thead>
                    <tr>
                        <th scope="col">{t!("biblioteca-col-parametro")}</th>
                        <th scope="col">{t!("biblioteca-col-etiqueta")}</th>
                        <th scope="col">{t!("biblioteca-col-tipo")}</th>
                        <th scope="col">{t!("biblioteca-col-opciones")}</th>
                    </tr>
                </thead>
                <tbody>{filas}</tbody>
            </table>
        </div>
    }
}

/// `/explorador/:area/:funcion`: ficha, lectura y formulario.
#[component]
pub fn PaginaFuncion() -> impl IntoView {
    let params = use_params_map();
    let nombre = Memo::new(move |_| params.with(|p| p.get("funcion").unwrap_or_default()));
    let detalle = Resource::new(move || nombre.get(), detalle_funcion);
    let contenido = move || {
        Suspend::new(async move {
            match detalle.await {
                Ok(d) => ficha(d).into_any(),
                Err(e) => view! {
                    <header class="pagina-cabecera">
                        <h1>{nombre.get_untracked()}</h1>
                    </header>
                    <p class="aviso estado-error" role="alert">{texto_error(&e)}</p>
                }
                .into_any(),
            }
        })
    };
    view! { <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>{contenido}</Transition> }
        .into_any()
}

fn ficha(d: FuncionDetalle) -> impl IntoView {
    let r = d.resumen.clone();
    let area_href = format!("/explorador/{}", r.area);
    let riesgo = (r.tipo != TipoFuncion::Lectura).then(|| {
        let texto = t!(
            "biblioteca-riesgo-vigilancia",
            riesgo = r.riesgo.etiqueta(),
            vigilancia = duracion(d.vigilancia_s)
        );
        view! { <Insignia estado=r.riesgo.estado() texto /> }
    });
    let bloqueo = r.bloqueada.clone().map(|m| {
        view! {
            <p class="aviso estado-error">
                <IconoEstado estado=Estado::Error />
                <span>
                    <strong>{t!("biblioteca-bloqueada-motivo")}</strong>
                    " "
                    <span lang="es">{m}</span>
                </span>
            </p>
        }
    });
    let datos = {
        let mut v = Vec::new();
        if let Some(p) = &d.pagina {
            v.push(
                view! {
                    <div>
                        <dt>{t!("biblioteca-pagina-router")}</dt>
                        <dd class="mono">{p.clone()}</dd>
                    </div>
                }
                .into_any(),
            );
        }
        if let Some((lp, area)) = &d.lectura_pareada {
            v.push(
                view! {
                    <div>
                        <dt>{t!("biblioteca-lectura-pareada")}</dt>
                        <dd>
                            <a class="mono" href=ruta_funcion(area, lp)>{lp.clone()}</a>
                        </dd>
                    </div>
                }
                .into_any(),
            );
        }
        if let Some(l) = &d.lista {
            v.push(
                view! {
                    <div>
                        <dt>{t!("biblioteca-lista")}</dt>
                        <dd>
                            {t!("biblioteca-lista-por")}
                            " "
                            <span class="mono">{l.id_param.clone()}</span>
                            {l.borrar.then(|| t!("biblioteca-lista-borrar"))}
                        </dd>
                    </div>
                }
                .into_any(),
            );
        }
        (!v.is_empty()).then(|| view! { <dl class="datos-funcion">{v}</dl> })
    };
    let parametros = (!d.parametros.is_empty()).then(|| {
        view! {
            <section class="seccion" aria-labelledby="titulo-parametros">
                <h2 id="titulo-parametros" class="titulo-seccion">{t!("biblioteca-parametros")}</h2>
                {tabla_parametros(&d.parametros)}
            </section>
        }
    });
    let cuerpo = match (r.tipo, &r.bloqueada) {
        (_, Some(_)) => None,
        (TipoFuncion::Lectura, None) => {
            Some(view! { <VistaLectura detalle=d.clone() /> }.into_any())
        }
        (_, None) => Some(view! { <Escritura detalle=d.clone() /> }.into_any()),
    };
    view! {
        <header class="pagina-cabecera">
            <div class="cabecera-funcion">
                <p class="miga">
                    <a href=area_href lang="es">{d.etiqueta_area.clone()}</a>
                </p>
                <h1>{r.nombre.clone()}</h1>
                <p class="pagina-sub" lang="es">{r.descripcion.clone()}</p>
            </div>
            <div class="insignias">
                <Etiquetas f=r.clone() />
                {riesgo}
            </div>
        </header>
        {bloqueo}
        {datos}
        {parametros}
        {cuerpo}
    }
}

// ------------------------------------------------------------ lectura

/// Celda de un valor de lectura (del campo `campo`); los secretos,
/// censurados con el ojo.
fn celda_valor(v: Valor, campo: &str) -> AnyView {
    match v {
        Valor::Texto(t) if t.is_empty() => view! {
            <span class="vacia" aria-hidden="true">{t!("biblioteca-vacio-par")}</span>
            <span class="solo-lector">{t!("biblioteca-vacio")}</span>
        }
        .into_any(),
        Valor::Texto(t) => t.into_any(),
        Valor::Opcion { valor, etiqueta } => {
            Valor::texto_opcion(&valor, etiqueta.as_deref()).into_any()
        }
        Valor::Ausente => view! {
            <span class="vacia" aria-hidden="true">"—"</span>
            <span class="solo-lector">{t!("comun-sin-valor")}</span>
        }
        .into_any(),
        Valor::Clave(v) => {
            view! { <SecretoLeido valor=Some(v) nombre=campo.to_owned() /> }.into_any()
        }
        // Una lectura siempre trae el valor; por si acaso, la máscara.
        Valor::Secreto => view! {
            <span class="secreto" aria-hidden="true">{MASCARA}</span>
            <span class="solo-lector">{t!("comun-valor-oculto")}</span>
        }
        .into_any(),
    }
}

/// Valores de una lectura: tabla clave/valor, tablas de instancias y XML.
fn vista_valores(v: Vista) -> impl IntoView + use<> {
    let vacia = v.pares.is_empty() && v.grupos.is_empty();
    let pares = (!v.pares.is_empty()).then(|| {
        let filas = v
            .pares
            .into_iter()
            .map(|p| {
                let celda = celda_valor(p.valor, &p.nombre);
                view! {
                    <tr>
                        <th scope="row"><span class="mono">{p.nombre}</span></th>
                        <td class="celda-larga">{celda}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <div class="tabla-scroll">
                <table class="tabla tabla-clave-valor">
                    <thead>
                        <tr>
                            <th scope="col">{t!("biblioteca-col-campo")}</th>
                            <th scope="col">{t!("biblioteca-col-valor")}</th>
                        </tr>
                    </thead>
                    <tbody>{filas}</tbody>
                </table>
            </div>
        }
    });
    let grupos = v
        .grupos
        .into_iter()
        .map(|g| {
            let cabecera = g
                .columnas
                .iter()
                .map(|c| view! { <th scope="col"><span class="mono">{c.clone()}</span></th> })
                .collect_view();
            let mostradas = g.filas.len();
            let filas = g
                .filas
                .into_iter()
                .map(|f| {
                    let celdas = f
                        .valores
                        .into_iter()
                        .zip(&g.columnas)
                        .map(|(v, c)| view! { <td>{celda_valor(v, c)}</td> })
                        .collect_view();
                    view! { <tr>{celdas}</tr> }
                })
                .collect_view();
            let instancias = if g.total > mostradas {
                t!(
                    "biblioteca-instancias-parcial",
                    n = g.total,
                    mostradas = mostradas
                )
            } else {
                t!("biblioteca-instancias", n = g.total)
            };
            let titulo = format!("{}: {instancias}", g.nombre);
            view! {
                <div class="tabla-scroll">
                    <table class="tabla">
                        <caption>{titulo}</caption>
                        <thead>
                            <tr>{cabecera}</tr>
                        </thead>
                        <tbody>{filas}</tbody>
                    </table>
                </div>
            }
        })
        .collect_view();
    let crudo = v.crudo.map(|x| {
        view! {
            <details class="crudo">
                <summary>{t!("biblioteca-xml")}</summary>
                <pre class="mono">{x}</pre>
            </details>
        }
    });
    view! {
        {vacia.then(|| view! { <p class="estado-espera">{t!("biblioteca-sin-datos")}</p> })}
        {pares}
        {grupos}
        {crudo}
    }
}

/// `vista` solo en el navegador, ya hidratado; antes (en el HTML del servidor
/// y al hidratar), `espera`. Para lo que depende de un `LocalResource` creado
/// dentro de un `Suspend`: el servidor no lo puede pintar, y así el navegador
/// hidrata lo mismo que pintó el servidor (los `Effect` solo corren en el
/// navegador, después de hidratar).
pub(super) fn en_el_navegador<E, V>(
    espera: impl Fn() -> E + Send + Sync + 'static,
    vista: impl Fn() -> V + Send + Sync + 'static,
) -> impl IntoView
where
    E: IntoView + 'static,
    V: IntoView + 'static,
{
    let (listo, set_listo) = signal(false);
    Effect::new(move |_| set_listo.set(true));
    move || {
        if listo.get() {
            vista().into_any()
        } else {
            espera().into_any()
        }
    }
}

/// Lectura: parámetros, botón «Leer» y resultado.
#[component]
fn VistaLectura(detalle: FuncionDetalle) -> impl IntoView {
    let funcion = detalle.resumen.nombre.clone();
    if let Err(motivo) = &detalle.lectura {
        return view! {
            <p class="aviso estado-neutro">
                <IconoEstado estado=Estado::Neutro />
                <span>{motivo.clone()}</span>
            </p>
        }
        .into_any();
    }
    let campos: Vec<(ParamVista, RwSignal<String>)> = detalle
        .parametros
        .iter()
        .map(|p| (p.clone(), RwSignal::new(p.inicial.clone())))
        .collect();
    let valores_de = {
        let campos = campos.clone();
        move || -> Vec<(String, String)> {
            campos
                .iter()
                .map(|(p, s)| (p.nombre.clone(), s.get_untracked()))
                .collect()
        }
    };
    let consulta = RwSignal::new(valores_de());
    // Solo en el navegador (`LocalResource`, pintado con `en_el_navegador`):
    // este recurso nace dentro del `Suspend` de `detalle`, y un recurso
    // anidado que llega en el HTML del servidor puede no casar al hidratar
    // con el que crea el navegador (pasaba según lo que tardase el router, y
    // el wasm moría con un pánico de hidratación). Sin JavaScript no hay
    // lectura: «Leer de nuevo» ya lo necesitaba.
    let resultado = LocalResource::new({
        let funcion = funcion.clone();
        move || leer(funcion.clone(), consulta.get())
    });
    let leyendo = RwSignal::new(false);

    let entradas = campos
        .iter()
        .enumerate()
        .map(|(i, (p, s))| {
            let id = format!("lectura-{i}");
            let s = *s;
            view! {
                <p class="campo">
                    <label for=id.clone()>
                        <span lang="es">{p.etiqueta.clone()}</span>
                        <span class="mono campo-nombre">{p.nombre.clone()}</span>
                    </label>
                    <input
                        id=id
                        type="text"
                        autocomplete="off"
                        spellcheck="false"
                        prop:value=s
                        on:input=move |ev| s.set(event_target_value(&ev))
                    />
                </p>
            }
        })
        .collect_view();
    let hay_params = !campos.is_empty();
    let leer_de_nuevo = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let nuevos = valores_de();
        if consulta.with_untracked(|c| *c == nuevos) {
            // Misma consulta: se fuerza otra lectura.
            resultado.refetch();
        } else {
            // Consulta nueva: el recurso ya lee de nuevo (una sola vez).
            consulta.set(nuevos);
        }
    };

    let datos = {
        move || {
            Suspend::new(async move {
                match resultado.await {
                    Ok(v) => vista_valores(v).into_any(),
                    Err(e) => view! {
                        <p class="aviso estado-error" role="alert">
                            <IconoEstado estado=Estado::Error />
                            <span>{texto_error(&e)}</span>
                        </p>
                    }
                    .into_any(),
                }
            })
        }
    };

    view! {
        <section class="seccion" aria-labelledby="titulo-lectura">
            <div class="seccion-cabecera">
                <h2 id="titulo-lectura" class="titulo-seccion">{t!("biblioteca-lectura")}</h2>
            </div>
            <form class="formulario formulario-lectura" on:submit=leer_de_nuevo>
                {hay_params.then(|| view! { <div class="rejilla-campos">{entradas}</div> })}
                <div class="formulario-pie">
                    <button class="boton-sutil" type="submit" disabled=move || leyendo.get()>
                        {move || if leyendo.get() { t!("biblioteca-leyendo") } else { t!("biblioteca-leer-de-nuevo") }}
                    </button>
                </div>
            </form>
            <div class="resultado-lectura" aria-live="polite" aria-busy=move || leyendo.get().to_string()>
                {en_el_navegador(
                    || view! { <p class="estado-espera">{t!("biblioteca-leyendo")}</p> },
                    move || view! {
                        <Transition set_pending=leyendo fallback=|| view! { <p class="estado-espera">{t!("biblioteca-leyendo")}</p> }>{datos}</Transition>
                    },
                )}
            </div>
        </section>
    }
    .into_any()
}
