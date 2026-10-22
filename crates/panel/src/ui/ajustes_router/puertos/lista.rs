//! Lista de reglas de puertos con sus acciones.

use super::formulario::FormRegla;
use super::*;

#[component]
pub(crate) fn Puertos(a: AjusteVista, solo_lectura: bool, recargar: Callback<()>) -> impl IntoView {
    let datos = expect_context::<DatosPuertos>();
    let foco = expect_context::<Foco>();
    let id_dom = dom(&a);
    let leido = matches!(a.estado, EstadoLectura::Leido);
    let editable = leido && !solo_lectura;
    let grupo = RwSignal::new(None::<GrupoVista>);
    let modo = RwSignal::new(None::<Modo>);
    // La regla que se está preparando (borrar o activar): su id.
    let ocupada = RwSignal::new(None::<String>);
    // «Activa» pulsado y aún sin aplicar: id y valor nuevo.
    let activando = RwSignal::new(None::<(String, bool)>);
    let error = RwSignal::new(None::<String>);
    let plantilla = StoredValue::new(a.campos.clone());
    let instancias = StoredValue::new(a.instancias.clone());
    let abrir_id = format!("{id_dom}-abrir");
    let editar_id = {
        let id_dom = id_dom.clone();
        move |id: &str| format!("{id_dom}-editar-{id}")
    };

    // Al dejar el formulario, el foco vuelve al botón que lo abrió.
    let cancelar = {
        let abrir_id = abrir_id.clone();
        let editar_id = editar_id.clone();
        Callback::new(move |()| {
            if let Some(m) = modo.get_untracked() {
                let id = match m {
                    Modo::Nueva => abrir_id.clone(),
                    Modo::Editar(id) => editar_id(&id),
                };
                request_animation_frame(move || enfocar(&id));
            }
            modo.set(None);
        })
    };
    let preparado = Callback::new(move |g: GrupoVista| grupo.set(Some(g)));
    let titulo = format!("{id_dom}-titulo");
    let cerrar = Callback::new(move |aplicado: Option<i64>| {
        grupo.set(None);
        activando.set(None);
        if aplicado.is_some() {
            modo.set(None);
            foco.0.set(Some(titulo.clone()));
            recargar.run(());
        }
    });
    // Borrar o activar/desactivar una regla: prepara el cambio con lo que
    // la identifica y abre el diálogo.
    let preparar = Callback::new(
        move |(i, borrar, activa): (InstanciaVista, bool, Option<bool>)| {
            if ocupada.get_untracked().is_some() {
                return;
            }
            error.set(None);
            modo.set(None);
            ocupada.set(Some(i.id.clone()));
            if let Some(v) = activa {
                activando.set(Some((i.id.clone(), v)));
            }
            let p = PeticionAjuste {
                ajuste: "puertos.reglas".into(),
                instancia: Some(i.id.clone()),
                borrar,
                valores: activa
                    .map(|v| ("activa".to_owned(), v.to_string()))
                    .into_iter()
                    .collect(),
                esperados: IDENTIFICAN_REGLA
                    .iter()
                    .map(|k| ((*k).to_owned(), valor_de(&i.campos, k)))
                    .collect(),
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => grupo.set(Some(g)),
                    Err(e) => {
                        activando.set(None);
                        error.set(Some(texto_error(&e)));
                    }
                }
                ocupada.set(None);
            });
        },
    );

    let dom_form = format!("{id_dom}-form");
    let cuerpo = move |red: RedPuertos| {
        let lista = instancias.get_value();
        let reglas: Vec<(InstanciaVista, Option<ReglaPuertos>)> =
            lista.iter().map(|i| (i.clone(), regla_de(i))).collect();
        let existentes: Vec<ReglaPuertos> = reglas.iter().filter_map(|(_, r)| r.clone()).collect();
        let filas = reglas
            .into_iter()
            .map(|(i, r)| {
                fila(
                    &i,
                    r.as_ref(),
                    &red,
                    editable,
                    editar_id(&i.id),
                    modo,
                    error,
                    ocupada,
                    activando,
                    preparar,
                )
            })
            .collect_view();
        let tabla = if lista.is_empty() {
            view! { <p class="ayuda">{t!("ajuste-puertos-reglas-vacia")}</p> }.into_any()
        } else {
            // Cabecera de columnas solo a la vista (en pantallas anchas):
            // cada fila ya nombra sus datos con `<dt>`.
            view! {
                <ul class="reglas">
                    <li class="reglas-cabecera" aria-hidden="true">
                        <span></span>
                        <span>{t!("ajuste-puertos-reglas-col-puerto")}</span>
                        <span>{t!("ajuste-puertos-reglas-col-dispositivo")}</span>
                        <span>{t!("ajuste-puertos-reglas-activa")}</span>
                        <span></span>
                    </li>
                    {filas}
                </ul>
            }
            .into_any()
        };
        let dom_form = dom_form.clone();
        let red_form = red.clone();
        let formulario = move || {
            modo.get().map(|m| {
                let editando = match &m {
                    Modo::Nueva => None,
                    Modo::Editar(id) => {
                        instancias.with_value(|is| is.iter().find(|i| i.id == *id).cloned())
                    }
                };
                view! {
                    <FormRegla
                        plantilla=plantilla.get_value()
                        editando
                        existentes=existentes.clone()
                        red=red_form.clone()
                        error
                        dom=dom_form.clone()
                        preparado
                        cancelar
                    />
                }
            })
        };
        let abrir = editable.then(|| {
            let abrir_id = abrir_id.clone();
            view! {
                <div class="ajuste-acciones reglas-acciones" class:oculto=move || modo.get() == Some(Modo::Nueva)>
                    <button
                        class="b-sec"
                        type="button"
                        id=abrir_id
                        on:click=move |_| {
                            error.set(None);
                            modo.set(Some(Modo::Nueva));
                        }
                    >
                        {t!("ajuste-puertos-reglas-abrir")}
                    </button>
                </div>
            }
        });
        // La única región de alerta de la tarjeta: errores de borrar,
        // activar y del formulario.
        view! {
            {tabla}
            {abrir}
            {formulario}
            <p class="error-formulario" role="alert">{move || error.get()}</p>
        }
        .into_any()
    };

    let contenido = match &a.estado {
        EstadoLectura::SinLeer(m) => aviso_error(m.clone()),
        EstadoLectura::Leido => view! {
            <Transition fallback=|| view! { <p class="estado-espera">{t!("comun-cargando")}</p> }>
                {
                    let cuerpo = cuerpo.clone();
                    move || {
                        let cuerpo = cuerpo.clone();
                        Suspend::new(async move { cuerpo(datos.leer().await) })
                    }
                }
            </Transition>
        }
        .into_any(),
    };
    view! {
        {cabecera(&a, editable)}
        {ayuda(&a)}
        {contenido}
        {detalles_tecnicos(&a.clave, &a.tecnico)}
        {move || grupo.get().map(|g| view! { <DialogoGrupo grupo=g on_cerrar=cerrar /> })}
    }
}

/// Una regla de la lista: su nombre (con el aviso si su destino ya no está
/// en la red), el puerto con su protocolo, el dispositivo, si está activa y
/// sus acciones. Una regla que la página no entiende se ve con lo que se
/// sabe y sin editar (sí se puede borrar).
#[allow(clippy::too_many_arguments)]
fn fila(
    i: &InstanciaVista,
    r: Option<&ReglaPuertos>,
    red: &RedPuertos,
    editable: bool,
    editar_id: String,
    modo: RwSignal<Option<Modo>>,
    error: RwSignal<Option<String>>,
    ocupada: RwSignal<Option<String>>,
    activando: RwSignal<Option<(String, bool)>>,
    preparar: Callback<(InstanciaVista, bool, Option<bool>)>,
) -> impl IntoView + use<> {
    let v = |id: &str| valor_de(&i.campos, id);
    let destino = v("destino");
    let puerto = r.map_or_else(|| v("externo_desde"), |r| rango(r.externo));
    let nombre_regla = Some(v("nombre"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| puerto.clone());
    // Un protocolo que el panel no conoce, como en el resto de ajustes.
    let protocolo = match i
        .campos
        .iter()
        .find(|c| c.id == "protocolo")
        .map(|c| &c.valor)
    {
        Some(ValorLegible::Opcion(k)) => traducir(&format!("{CLAVE}-protocolo-{k}"), None),
        _ => t!("ajustes-valor-desconocido", valor = v("protocolo")),
    };
    let interno = r
        .filter(|r| r.interno != r.externo)
        .map(|r| t!("ajuste-puertos-reglas-a-ip", puerto = rango(r.interno)));
    let quien = red.nombre_de(&destino);
    let perdido = red.destino_perdido(&destino);
    let activa = v("activa") == "true";
    let aviso = perdido.then(|| {
        view! { <Insignia estado=EstadoVisual::Atencion texto=t!("ajuste-puertos-reglas-destino-perdido") /> }
    });
    // Qué hacer, a lo ancho de la fila.
    let que_hacer = perdido.then(|| {
        view! { <p class="regla-perdida">{t!("ajuste-puertos-reglas-destino-perdido-desc")}</p> }
    });
    let id = i.id.clone();
    let marcada = {
        let id = id.clone();
        move || {
            activando.with(|a| match a {
                Some((aid, v)) if *aid == id => *v,
                _ => activa,
            })
        }
    };
    let ocupado = {
        let id = id.clone();
        move || ocupada.with(|o| o.as_deref() == Some(id.as_str()))
    };
    let interruptor = {
        let i = i.clone();
        let etiqueta = t!(
            "ajuste-puertos-reglas-activa-de",
            nombre = nombre_regla.clone()
        );
        view! {
            <input
                class="interruptor"
                type="checkbox"
                role="switch"
                aria-label=etiqueta
                // Mientras se prepara cualquier regla, ninguno: el cambio
                // se perdería y el interruptor quedaría como no está.
                disabled=move || !editable || ocupada.with(Option::is_some)
                prop:checked=marcada
                on:change=move |ev| preparar.run((i.clone(), false, Some(event_target_checked(&ev))))
            />
        }
    };
    let acciones = editable.then(|| {
        let (id_e, i_b) = (id.clone(), i.clone());
        let se_edita = r.is_some();
        view! {
            <div class="regla-acciones">
                {se_edita.then(|| view! {
                    <button
                        class="b-sec"
                        type="button"
                        id=editar_id
                        aria-label=t!("ajuste-puertos-reglas-editar-de", nombre = nombre_regla.clone())
                        on:click=move |_| {
                            error.set(None);
                            modo.set(Some(Modo::Editar(id_e.clone())));
                        }
                    >
                        {t!("ajustes-editar")}
                    </button>
                })}
                <button
                    class="b-pel"
                    type="button"
                    aria-label=t!("ajuste-puertos-reglas-borrar-de", nombre = nombre_regla.clone())
                    aria-disabled=move || ocupado().then_some("true")
                    on:click=move |_| preparar.run((i_b.clone(), true, None))
                >
                    {t!("ajuste-puertos-reglas-borrar")}
                </button>
            </div>
        }
    });
    view! {
        <li class="regla" class:regla-inactiva=!activa class:regla-sin-destino=perdido>
            <div class="regla-quien">
                <span class="regla-nombre">{nombre_regla.clone()}</span>
                {aviso}
            </div>
            <dl class="regla-datos">
                <div>
                    <dt>{t!("ajuste-puertos-reglas-col-puerto")}</dt>
                    <dd>
                        <span class="mono regla-puerto">{puerto}</span>
                        " "
                        <span class="chip">{protocolo}</span>
                        {interno.map(|t| view! { <span class="regla-interno-nota">{t}</span> })}
                    </dd>
                </div>
                <div>
                    <dt>{t!("ajuste-puertos-reglas-col-dispositivo")}</dt>
                    <dd>
                        {quien.map(|q| view! { <span class="regla-dispositivo">{q}</span> })}
                        <span class="mono regla-ip">{destino}</span>
                    </dd>
                </div>
                <div class="regla-activa">
                    <dt>{t!("ajuste-puertos-reglas-activa")}</dt>
                    <dd>{interruptor}</dd>
                </div>
            </dl>
            {acciones}
            {que_hacer}
        </li>
    }
}
