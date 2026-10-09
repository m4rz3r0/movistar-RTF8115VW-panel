//! Escrituras y acciones: claves de la instancia, lista de instancias con
//! «Añadir», «Editar» y «Borrar», y el formulario generado del catálogo
//! (rellenado con la lectura pareada) que termina en el gestor de cambios.

use super::{
    CampoForm, DialogoCambio, EstadoDeCambio, FormVista, FuncionDetalle, ListaInstancias, MASCARA,
    Operacion, ParamVista, Peticion, PreparadoVista, TipoCampo, TipoFuncion, Valor, formulario,
    instancias, preparar_cambio, texto_error, validar_campo,
};
use crate::ui::{Estado, IconoEstado};
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};

/// Qué formulario se muestra.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Modo {
    Ninguno,
    /// Instancia que se edita (`None`: nueva, o la única de una función sin
    /// lista).
    Form(Option<String>),
}

fn aviso_error(texto: String) -> impl IntoView {
    view! {
        <p class="aviso estado-error" role="alert">
            <IconoEstado estado=Estado::Error />
            <span>{texto}</span>
        </p>
    }
}

/// Parte de escritura de la ficha de una función.
#[component]
pub(super) fn Escritura(detalle: FuncionDetalle) -> impl IntoView {
    let funcion = StoredValue::new(detalle.resumen.nombre.clone());
    let es_lista = detalle.lista.is_some() && detalle.resumen.tipo == TipoFuncion::Escritura;
    let se_borra = detalle.lista.as_ref().is_some_and(|l| l.borrar);
    let campos_clave: Vec<(ParamVista, RwSignal<String>)> = detalle
        .claves
        .iter()
        .map(|p| (p.clone(), RwSignal::new(p.inicial.clone())))
        .collect();
    let leer_claves = {
        let c = campos_clave.clone();
        move || -> Vec<(String, String)> {
            c.iter()
                .map(|(p, s)| (p.nombre.clone(), s.get_untracked().trim().to_owned()))
                .collect()
        }
    };
    let claves = RwSignal::new(leer_claves());
    let modo = RwSignal::new(if es_lista {
        Modo::Ninguno
    } else {
        Modo::Form(None)
    });
    let recarga = RwSignal::new(0u32);
    let preparado = RwSignal::new(None::<PreparadoVista>);
    let error = RwSignal::new(None::<String>);

    // ---- claves de la instancia
    let form_claves = (!campos_clave.is_empty()).then(|| {
        let entradas = campos_clave
            .iter()
            .enumerate()
            .map(|(i, (p, s))| {
                let id = format!("clave-{i}");
                let s = *s;
                view! {
                    <p class="campo">
                        <label for=id.clone()>
                            {p.etiqueta.clone()}
                            <span class="mono campo-nombre">{p.nombre.clone()}</span>
                        </label>
                        <input
                            id=id
                            type="text"
                            inputmode="numeric"
                            autocomplete="off"
                            spellcheck="false"
                            prop:value=s
                            on:input=move |ev| s.set(event_target_value(&ev))
                        />
                    </p>
                }
            })
            .collect_view();
        let cargar = move |ev: SubmitEvent| {
            ev.prevent_default();
            error.set(None);
            claves.set(leer_claves());
            if es_lista {
                modo.set(Modo::Ninguno);
            }
            recarga.update(|n| *n += 1);
        };
        view! {
            <form class="formulario formulario-claves" on:submit=cargar>
                <p class="pagina-sub">"Instancia sobre la que trabajar:"</p>
                <div class="rejilla-campos">{entradas}</div>
                <div class="formulario-pie">
                    <button class="boton-sutil" type="submit">"Cargar"</button>
                </div>
            </form>
        }
    });

    // ---- lista de instancias
    let lista = es_lista.then(|| {
        let res = Resource::new(
            move || (funcion.get_value(), claves.get(), recarga.get()),
            |(f, c, _)| instancias(f, c),
        );
        let borrar = move |id: String| {
            error.set(None);
            let peticion = Peticion {
                funcion: funcion.get_value(),
                instancia: Some(id),
                valores: claves.get_untracked(),
                borrar: true,
            };
            spawn_local(async move {
                match preparar_cambio(peticion).await {
                    Ok(p) => preparado.set(Some(p)),
                    Err(e) => error.set(Some(texto_error(&e))),
                }
            });
        };
        let tabla = move || {
            Suspend::new(async move {
                match res.await {
                    Ok(i) => tabla_instancias(i, se_borra, modo, borrar).into_any(),
                    Err(e) => aviso_error(texto_error(&e)).into_any(),
                }
            })
        };
        view! {
            <section class="seccion" aria-labelledby="titulo-instancias">
                <div class="seccion-cabecera">
                    <h2 id="titulo-instancias" class="titulo-seccion">"Instancias"</h2>
                    <button
                        class="boton-sutil"
                        type="button"
                        on:click=move |_| {
                            error.set(None);
                            modo.set(Modo::Form(None));
                        }
                    >
                        "Añadir"
                    </button>
                </div>
                <Transition fallback=|| view! { <p class="estado-espera">"Leyendo…"</p> }>
                    {tabla}
                </Transition>
            </section>
        }
    });

    // ---- formulario
    let form_res = Resource::new(
        move || match modo.get() {
            Modo::Form(i) => Some((funcion.get_value(), i, claves.get(), recarga.get())),
            Modo::Ninguno => None,
        },
        |k| async move {
            match k {
                Some((f, i, c, _)) => Some(formulario(f, i, c).await),
                None => None,
            }
        },
    );
    let titulo = move || match (detalle.resumen.tipo, modo.get()) {
        (TipoFuncion::Accion, _) => "Ejecutar".to_owned(),
        (_, Modo::Form(Some(i))) => format!("Editar la instancia {i}"),
        (_, Modo::Form(None)) if es_lista => "Nueva instancia".to_owned(),
        _ => "Editar".to_owned(),
    };
    let form = move || {
        Suspend::new(async move {
            match form_res.await {
                None => ().into_any(),
                Some(Err(e)) => aviso_error(texto_error(&e)).into_any(),
                Some(Ok(f)) => view! {
                    <section class="seccion tarjeta" aria-labelledby="titulo-formulario">
                        <div class="seccion-cabecera">
                            <h2 id="titulo-formulario" class="titulo-seccion">{titulo}</h2>
                            {es_lista.then(|| view! {
                                <button
                                    class="boton-sutil"
                                    type="button"
                                    on:click=move |_| modo.set(Modo::Ninguno)
                                >
                                    "Cancelar"
                                </button>
                            })}
                        </div>
                        <Formulario form=f on_preparado=move |p| preparado.set(Some(p)) />
                    </section>
                }
                .into_any(),
            }
        })
    };

    let dialogo = move || {
        preparado.get().map(|p| {
            view! {
                <DialogoCambio
                    preparado=p
                    on_cerrar=move |_| preparado.set(None)
                    on_terminado=move |e: EstadoDeCambio| {
                        recarga.update(|n| *n += 1);
                        if es_lista && e == EstadoDeCambio::Aplicado {
                            modo.set(Modo::Ninguno);
                        }
                    }
                />
            }
        })
    };

    view! {
        {form_claves}
        {move || error.get().map(aviso_error)}
        {lista}
        <Transition fallback=|| view! { <p class="estado-espera">"Leyendo los valores actuales…"</p> }>
            {form}
        </Transition>
        {dialogo}
    }
}

/// Tabla de instancias con «Editar» y «Borrar» por fila.
fn tabla_instancias(
    i: ListaInstancias,
    se_borra: bool,
    modo: RwSignal<Modo>,
    borrar: impl Fn(String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let cabecera = i
        .columnas
        .iter()
        .map(|c| view! { <th scope="col"><span class="mono">{c.clone()}</span></th> })
        .collect_view();
    let mostradas = i.filas.len();
    let vacia = (mostradas == 0).then(|| {
        view! {
            <tr class="fila-vacia">
                <td colspan=i.columnas.len() + 1>"No hay instancias"</td>
            </tr>
        }
    });
    let filas = i
        .filas
        .into_iter()
        .map(|f| {
            let celdas = f
                .valores
                .into_iter()
                .map(|v| {
                    let c = match v {
                        Valor::Texto(t) => t.into_any(),
                        Valor::Secreto => view! {
                            <span class="secreto">{MASCARA}</span>
                            <span class="solo-lector">"(valor oculto)"</span>
                        }
                        .into_any(),
                        Valor::Ausente => view! { <span class="vacia">"—"</span> }.into_any(),
                        Valor::Opcion { valor, etiqueta } => {
                            Valor::texto_opcion(&valor, etiqueta.as_deref()).into_any()
                        }
                    };
                    view! { <td>{c}</td> }
                })
                .collect_view();
            // Sin id propio (`#n`) no se puede pedir la instancia.
            let con_id = !f.id.starts_with('#');
            let id_editar = f.id.clone();
            let id_borrar = f.id.clone();
            let etiqueta = f.id.clone();
            view! {
                <tr>
                    <td class="acciones-fila">
                        {con_id.then(|| view! {
                            <button
                                class="boton-mini"
                                type="button"
                                on:click=move |_| modo.set(Modo::Form(Some(id_editar.clone())))
                            >
                                "Editar"
                                <span class="solo-lector">{format!(" la instancia {etiqueta}")}</span>
                            </button>
                        })}
                        {(con_id && se_borra).then(|| view! {
                            <button
                                class="boton-mini boton-peligro"
                                type="button"
                                on:click=move |_| borrar(id_borrar.clone())
                            >
                                "Borrar"
                                <span class="solo-lector">{format!(" la instancia {}", f.id)}</span>
                            </button>
                        })}
                    </td>
                    {celdas}
                </tr>
            }
        })
        .collect_view();
    let titulo = if i.total > mostradas {
        format!("{} instancias (se muestran {mostradas})", i.total)
    } else {
        format!("{} instancias", i.total)
    };
    view! {
        <div class="tabla-scroll">
            <table class="tabla">
                <caption>{titulo}</caption>
                <thead>
                    <tr>
                        <th scope="col">
                            <span class="solo-lector">"Acciones"</span>
                        </th>
                        {cabecera}
                    </tr>
                </thead>
                <tbody>{vacia}{filas}</tbody>
            </table>
        </div>
    }
}

/// Error de validación de un campo (solo si el valor cambió: lo que viene del
/// router no se juzga).
fn error_campo(c: &CampoForm, v: &str) -> Option<String> {
    let sin_tocar = v == c.valor || (c.param.tipo == TipoCampo::Secret && v.is_empty());
    if sin_tocar || v.trim().is_empty() {
        return None;
    }
    validar_campo(c.param.tipo, &c.param.opciones, v).err()
}

/// Control de un campo según su tipo.
fn control(i: usize, c: &CampoForm, s: RwSignal<String>, op: Operacion) -> impl IntoView + use<> {
    let id = format!("campo-{i}");
    let id_error = format!("campo-{i}-error");
    let id_ayuda = format!("campo-{i}-ayuda");
    let c2 = c.clone();
    let error = Memo::new(move |_| s.with(|v| error_campo(&c2, v)));
    let invalido = move || error.with(Option::is_some).to_string();
    let p = &c.param;
    let etiqueta = view! {
        <label for=id.clone()>
            {p.etiqueta.clone()}
            <span class="mono campo-nombre">{p.nombre.clone()}</span>
        </label>
    };
    let ayuda = match (p.tipo, op) {
        (TipoCampo::Secret, Operacion::Editar) => Some("Déjalo como está para no cambiarlo."),
        (TipoCampo::Secret, _) => Some("Vacío: no se envía."),
        (TipoCampo::Ip, _) => Some("Dirección IPv4, p. ej. 192.168.1.60."),
        (TipoCampo::Mac, _) => Some("Seis pares hexadecimales separados por «:»."),
        _ => None,
    };
    let describe = format!(
        "{id_error}{}",
        if ayuda.is_some() {
            format!(" {id_ayuda}")
        } else {
            String::new()
        }
    );
    let entrada = match p.tipo {
        TipoCampo::Bool => {
            return view! {
                <div class="campo campo-interruptor">
                    <input
                        id=id
                        class="interruptor"
                        type="checkbox"
                        role="switch"
                        prop:checked=move || s.get() == "1"
                        on:change=move |ev| s.set(if event_target_checked(&ev) { "1" } else { "0" }.into())
                    />
                    {etiqueta}
                </div>
            }
            .into_any();
        }
        TipoCampo::Enum => {
            let mut opciones = p.opciones.clone();
            if !c.valor.is_empty() && !opciones.iter().any(|(v, _)| *v == c.valor) {
                opciones.push((c.valor.clone(), format!("{} (valor actual)", c.valor)));
            }
            let vacia = c.valor.is_empty().then(|| {
                view! {
                    <option value="" selected=move || s.get().is_empty()>
                        "—"
                    </option>
                }
            });
            let opciones = opciones
                .into_iter()
                .map(|(v, e)| {
                    let v2 = v.clone();
                    view! {
                        <option value=v selected=move || s.with(|x| *x == v2)>
                            {e}
                        </option>
                    }
                })
                .collect_view();
            view! {
                <select
                    id=id
                    aria-describedby=describe
                    on:change=move |ev| s.set(event_target_value(&ev))
                >
                    {vacia}
                    {opciones}
                </select>
            }
            .into_any()
        }
        TipoCampo::Secret => view! {
            <input
                id=id
                type="password"
                autocomplete="new-password"
                aria-describedby=describe
                aria-invalid=invalido
                prop:value=s
                on:input=move |ev| s.set(event_target_value(&ev))
            />
        }
        .into_any(),
        t => {
            let (modo_teclado, ejemplo) = match t {
                TipoCampo::Ip => ("decimal", "192.168.1.60"),
                TipoCampo::Mac => ("text", "02:00:00:00:00:01"),
                TipoCampo::Int => ("numeric", ""),
                _ => ("text", ""),
            };
            view! {
                <input
                    id=id
                    type="text"
                    inputmode=modo_teclado
                    placeholder=ejemplo
                    autocomplete="off"
                    spellcheck="false"
                    aria-describedby=describe
                    aria-invalid=invalido
                    prop:value=s
                    on:input=move |ev| s.set(event_target_value(&ev))
                />
            }
            .into_any()
        }
    };
    let es_secreto = p.tipo == TipoCampo::Secret && op == Operacion::Editar;
    view! {
        <div class="campo">
            {etiqueta}
            {entrada}
            {ayuda.map(|a| view! {
                <p class="ayuda" id=id_ayuda>
                    {a}
                    {es_secreto.then(|| view! {
                        " "
                        <span class="sin-cambios">
                            {move || if s.with(|v| v == MASCARA) { "Sin cambios." } else { "Se cambiará." }}
                        </span>
                    })}
                </p>
            })}
            <p class="campo-error" id=id_error>
                {move || error.get()}
            </p>
        </div>
    }
    .into_any()
}

/// Formulario de una instancia: un control por parámetro y «Revisar
/// cambios», que prepara el cambio en el gestor.
#[component]
fn Formulario(
    form: FormVista,
    #[prop(into)] on_preparado: Callback<PreparadoVista>,
) -> impl IntoView {
    let campos: Vec<(CampoForm, RwSignal<String>)> = form
        .campos
        .iter()
        .map(|c| (c.clone(), RwSignal::new(c.valor.clone())))
        .collect();
    let campos = StoredValue::new(campos);
    let error = RwSignal::new(None::<String>);
    let enviando = RwSignal::new(false);
    let form = StoredValue::new(form);
    let op = form.with_value(|f| f.operacion);

    let controles = campos.with_value(|c| {
        c.iter()
            .enumerate()
            .map(|(i, (c, s))| control(i, c, *s, op))
            .collect_view()
    });
    let omitidos = form.with_value(|f| {
        (!f.omitidos.is_empty()).then(|| {
            view! {
                <p class="pagina-sub">
                    "No editables (la lectura del router no los muestra y sin valor anterior no se podría deshacer): "
                    <span class="mono">{f.omitidos.join(", ")}</span>
                </p>
            }
        })
    });
    let sin_campos = campos.with_value(Vec::is_empty) && op == Operacion::Editar;

    let revisar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        let malos = campos.with_value(|c| {
            c.iter()
                .filter(|(c, s)| s.with_untracked(|v| error_campo(c, v)).is_some())
                .count()
        });
        if malos > 0 {
            error.set(Some(format!(
                "Corrige {} marcado{} antes de revisar.",
                if malos == 1 { "el campo" } else { "los campos" },
                if malos == 1 { "" } else { "s" }
            )));
            return;
        }
        let (funcion, instancia, mut valores) =
            form.with_value(|f| (f.funcion.clone(), f.instancia.clone(), f.claves.clone()));
        campos.with_value(|c| {
            for (c, s) in c {
                let v = s.get_untracked();
                // Al crear o ejecutar, lo que se deja vacío no se envía.
                if op != Operacion::Editar && v.trim().is_empty() {
                    continue;
                }
                valores.push((c.param.nombre.clone(), v));
            }
        });
        let peticion = Peticion {
            funcion,
            instancia,
            valores,
            borrar: false,
        };
        enviando.set(true);
        spawn_local(async move {
            match preparar_cambio(peticion).await {
                Ok(p) => on_preparado.run(p),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            enviando.set(false);
        });
    };

    view! {
        <form class="formulario" on:submit=revisar novalidate>
            {sin_campos.then(|| view! {
                <p class="aviso estado-neutro">
                    <IconoEstado estado=Estado::Neutro />
                    <span>
                        "La lectura del router no muestra ninguno de los campos de esta función: no se puede editar sin poder deshacer."
                    </span>
                </p>
            })}
            <div class="rejilla-campos">{controles}</div>
            {omitidos}
            <p class="error-formulario" role="alert">
                {move || error.get()}
            </p>
            <div class="formulario-pie">
                <button class="boton" type="submit" disabled=move || enviando.get() || sin_campos>
                    {move || if enviando.get() { "Revisando…" } else { "Revisar cambios" }}
                </button>
            </div>
        </form>
    }
}
