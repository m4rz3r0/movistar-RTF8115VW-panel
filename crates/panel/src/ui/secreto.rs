//! Un secreto en pantalla ([`Secreto`]): la máscara con un botón pequeño
//! para mostrarlo. Mostrarlo pide la contraseña del panel (salvo con la
//! ventana de reautenticación abierta), lo deja a la vista
//! [`VISIBLE`] con «Copiar» (si el navegador tiene portapapeles) y una
//! cuenta atrás que solo leen los lectores de pantalla, y después lo borra
//! de la señal y del DOM. El valor nunca va en el HTML del servidor: solo
//! llega al pulsar, por una server function con `no-store`.
//!
//! Lo usan la Biblioteca (un campo de una lectura), Ajustes (un campo de un
//! ajuste) y el Historial (el valor anterior de un paso). La contraseña se
//! pide con el diálogo de reautenticación de la página
//! ([`Reautenticacion`]) si lo hay; si no (la Biblioteca), con
//! [`DialogoRevelar`].

use crate::t;
use crate::ui::explorador::{MASCARA, texto_error};
use crate::ui::panel_ajustes::Reautenticacion;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use std::time::Duration;

/// Tiempo que se ve un secreto mostrado.
pub const VISIBLE: Duration = Duration::from_secs(30);

/// Secreto que se quiere mostrar: de dónde se lee.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjetivoSecreto {
    /// Un campo de una lectura de la Biblioteca (de la fila `instancia`).
    Funcion {
        funcion: String,
        campo: String,
        instancia: Option<String>,
        params: Vec<(String, String)>,
    },
    /// El campo `campo` del ajuste `ajuste` (de su instancia, en una lista).
    Ajuste {
        ajuste: String,
        campo: String,
        instancia: Option<String>,
    },
    /// El valor anterior del parámetro `campo` del paso `orden` de un cambio.
    Anterior {
        cambio: i64,
        orden: i64,
        campo: String,
    },
}

impl ObjetivoSecreto {
    /// Pide el valor al servidor (con `contrasena` vacía, solo si la sesión
    /// tiene abierta la ventana de reautenticación).
    pub async fn revelar(self, contrasena: String) -> Result<String, ServerFnError> {
        match self {
            ObjetivoSecreto::Funcion {
                funcion,
                campo,
                instancia,
                params,
            } => {
                crate::ui::explorador::revelar(funcion, campo, instancia, params, contrasena).await
            }
            ObjetivoSecreto::Ajuste {
                ajuste,
                campo,
                instancia,
            } => crate::ajustes_router::revelar_ajuste(ajuste, campo, instancia, contrasena).await,
            ObjetivoSecreto::Anterior {
                cambio,
                orden,
                campo,
            } => crate::ui::historial::revelar_anterior(cambio, orden, campo, contrasena).await,
        }
    }

    /// Texto del botón.
    fn boton(&self) -> String {
        match self {
            ObjetivoSecreto::Funcion { .. } => t!("biblioteca-revelar"),
            ObjetivoSecreto::Ajuste { .. } => t!("secreto-mostrar"),
            ObjetivoSecreto::Anterior { .. } => t!("historial-mostrar-anterior"),
        }
    }
}

/// ¿Puede la página escribir en el portapapeles? Solo en un contexto seguro
/// (HTTPS o `localhost`); por HTTP en la red de casa no hay portapapeles.
#[cfg(feature = "hydrate")]
fn portapapeles() -> Option<leptos::wasm_bindgen::JsValue> {
    use leptos::web_sys::js_sys::Reflect;
    let w = window();
    if !w.is_secure_context() {
        return None;
    }
    let nav = Reflect::get(&w, &"navigator".into()).ok()?;
    let c = Reflect::get(&nav, &"clipboard".into()).ok()?;
    (!c.is_undefined() && !c.is_null()).then_some(c)
}

#[cfg(not(feature = "hydrate"))]
fn portapapeles() -> Option<()> {
    None
}

/// Copia `texto` al portapapeles y, si sale bien, marca `copiado`. Un fallo
/// (sin permiso) no da error: el valor sigue a la vista para copiarlo a mano.
#[cfg(feature = "hydrate")]
fn copiar(texto: &str, copiado: RwSignal<bool>) {
    use leptos::wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use leptos::web_sys::js_sys::{Function, Reflect};
    let metodo = |objeto: &JsValue, nombre: &str| {
        Reflect::get(objeto, &nombre.into())
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
    };
    let Some(c) = portapapeles() else {
        return;
    };
    let Some(promesa) =
        metodo(&c, "writeText").and_then(|f| f.call1(&c, &JsValue::from_str(texto)).ok())
    else {
        return;
    };
    let Some(entonces) = metodo(&promesa, "then") else {
        return;
    };
    let bien = Closure::once_into_js(move |_: JsValue| {
        copiado.try_set(true);
    });
    let mal = Closure::once_into_js(|_: JsValue| {});
    let _ = entonces.call2(&promesa, &bien, &mal);
}

#[cfg(not(feature = "hydrate"))]
fn copiar(_texto: &str, _copiado: RwSignal<bool>) {}

/// La máscara de un secreto con su botón para mostrarlo; mostrado, el valor
/// con «Copiar» y, pasado [`VISIBLE`], otra vez la máscara. `nombre` es lo
/// que se muestra, para el lector de pantalla («Contraseña»).
#[component]
pub fn Secreto(objetivo: ObjetivoSecreto, #[prop(into)] nombre: String) -> impl IntoView {
    let valor = RwSignal::new(None::<String>);
    let restante = RwSignal::new(0u64);
    let copiado = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let pidiendo = RwSignal::new(false);
    let pide_contrasena = RwSignal::new(false);
    let intervalo: StoredValue<Option<IntervalHandle>> = StoredValue::new(None);
    let reauth = use_context::<Reautenticacion>();
    let boton = objetivo.boton();
    let o = StoredValue::new(objetivo);

    // Vuelve a la máscara: el valor sale de la señal (y así del DOM).
    let ocultar = move || {
        if let Some(Some(h)) = intervalo.try_update_value(Option::take) {
            h.clear();
        }
        valor.try_set(None);
        restante.try_set(0);
        copiado.try_set(false);
    };
    // Al desmontarse, solo se cancela el temporizador: escribir en las
    // señales aquí volvería a pintar una vista que ya se está liberando.
    on_cleanup(move || {
        if let Some(Some(h)) = intervalo.try_update_value(Option::take) {
            h.clear();
        }
    });
    let mostrar = move |v: String| {
        ocultar();
        if valor.try_set(Some(v)).is_some() {
            // La página ya no está: nada que mostrar.
            return;
        }
        restante.try_set(VISIBLE.as_secs());
        let paso = move || match restante.try_get_untracked() {
            Some(r) if r > 1 => {
                restante.try_set(r - 1);
            }
            _ => ocultar(),
        };
        if let Ok(h) = set_interval_with_handle(paso, Duration::from_secs(1)) {
            intervalo.try_update_value(|i| *i = Some(h));
        }
    };
    // Tras reautenticarse en el diálogo de la página: la ventana ya está
    // abierta, sin contraseña.
    let tras_reautenticar = Callback::new(move |()| {
        let Some(obj) = o.try_get_value() else {
            return;
        };
        pidiendo.set(true);
        spawn_local(async move {
            let r = obj.revelar(String::new()).await;
            pidiendo.try_set(false);
            match r {
                Ok(v) => mostrar(v),
                Err(e) => {
                    error.try_set(Some(texto_error(&e)));
                }
            }
        });
    });
    let pedir = move |_| {
        if pidiendo.get_untracked() {
            return;
        }
        error.set(None);
        pidiendo.set(true);
        let obj = o.get_value();
        spawn_local(async move {
            let r = obj.revelar(String::new()).await;
            pidiendo.try_set(false);
            match (r, reauth) {
                (Ok(v), _) => mostrar(v),
                (Err(e), Some(r)) => r.tratar(&e, error, tras_reautenticar),
                // Sin diálogo de la página (la Biblioteca): el suyo, que pide
                // la contraseña y enseña el error.
                (Err(_), None) => {
                    pide_contrasena.try_set(true);
                }
            }
        });
    };

    let lector = format!(" {nombre}");
    let lector_copiar = lector.clone();
    let dialogo = {
        let nombre = nombre.clone();
        move || {
            pide_contrasena.get().then(|| {
                view! {
                    <DialogoRevelar
                        objetivo=o.get_value()
                        nombre=nombre.clone()
                        on_cerrar=move |_| pide_contrasena.set(false)
                        on_revelado=move |v: String| mostrar(v)
                    />
                }
            })
        }
    };
    let vista = move || match valor.get() {
        Some(v) => {
            let vacio = v.is_empty();
            let texto = if vacio {
                t!("secreto-vacio")
            } else {
                v.clone()
            };
            let copiar_boton = (portapapeles().is_some() && !vacio).then(|| {
                let lector = lector_copiar.clone();
                view! {
                    <button
                        class="boton-mini"
                        type="button"
                        on:click=move |_| copiar(&v, copiado)
                    >
                        {move || if copiado.get() { t!("secreto-copiado") } else { t!("secreto-copiar") }}
                        <span class="solo-lector">{lector.clone()}</span>
                    </button>
                }
            });
            view! {
                <span class="secreto-mostrado">
                    <span class="mono revelado" class:ajuste-vacio=vacio>{texto}</span>
                    {copiar_boton}
                    <span class="solo-lector">
                        {move || t!("secreto-cuenta", s = restante.get())}
                    </span>
                </span>
            }
            .into_any()
        }
        None => {
            let lector = lector.clone();
            let boton = boton.clone();
            view! {
                <span class="celda-secreta">
                    <span class="secreto" aria-hidden="true">{MASCARA}</span>
                    <span class="solo-lector">{t!("comun-valor-oculto")}</span>
                    <button
                        class="boton-mini"
                        type="button"
                        aria-disabled=move || pidiendo.get().then_some("true")
                        on:click=pedir
                    >
                        {boton}
                        <span class="solo-lector">{lector}</span>
                    </button>
                </span>
            }
            .into_any()
        }
    };
    view! {
        {vista}
        {move || error.get().map(|e| view! { <span class="campo-error secreto-error" role="alert">{e}</span> })}
        {dialogo}
    }
}

/// Diálogo para mostrar un secreto cuando la página no tiene el de
/// reautenticación (la Biblioteca): pide la contraseña del panel.
#[component]
fn DialogoRevelar(
    objetivo: ObjetivoSecreto,
    nombre: String,
    #[prop(into)] on_cerrar: Callback<()>,
    #[prop(into)] on_revelado: Callback<String>,
) -> impl IntoView {
    let dlg = NodeRef::<html::Dialog>::new();
    Effect::new(move |_| {
        if let Some(d) = dlg.get()
            && !d.open()
        {
            let _ = d.show_modal();
        }
    });
    let contrasena = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let pidiendo = RwSignal::new(false);
    let objetivo = StoredValue::new(objetivo);

    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if pidiendo.get_untracked() {
            return;
        }
        error.set(None);
        pidiendo.set(true);
        let o = objetivo.get_value();
        let clave = contrasena.get_untracked();
        spawn_local(async move {
            match o.revelar(clave).await {
                Ok(v) => {
                    contrasena.try_set(String::new());
                    on_revelado.run(v);
                    if let Some(d) = dlg.get_untracked() {
                        d.close();
                    }
                }
                Err(e) => {
                    contrasena.try_set(String::new());
                    error.try_set(Some(texto_error(&e)));
                }
            }
            pidiendo.try_set(false);
        });
    };
    let cerrar = move |_| {
        if let Some(d) = dlg.get_untracked() {
            d.close();
        }
    };

    view! {
        <dialog
            class="dialogo dialogo-estrecho"
            node_ref=dlg
            aria-labelledby="revelar-titulo"
            aria-modal="true"
            on:close=move |_| on_cerrar.run(())
        >
            <form class="dialogo-cuerpo" on:submit=enviar>
                <header class="dialogo-cabecera">
                    <h2 id="revelar-titulo">{t!("biblioteca-revelar-titulo")}</h2>
                </header>
                <p>
                    {t!("biblioteca-revelar-para-ver")}
                    " "
                    <span class="mono">{nombre}</span>
                    {t!("biblioteca-revelar-texto")}
                </p>
                <p class="campo">
                    <label for="revelar-contrasena">{t!("biblioteca-contrasena-panel")}</label>
                    <input
                        id="revelar-contrasena"
                        type="password"
                        autocomplete="current-password"
                        required
                        autofocus
                        prop:value=contrasena
                        on:input=move |ev| contrasena.set(event_target_value(&ev))
                    />
                </p>
                <p class="error-formulario" role="alert">
                    {move || error.get()}
                </p>
                <div class="dialogo-pie">
                    <button class="boton-sutil" type="button" on:click=cerrar>
                        {t!("biblioteca-cancelar")}
                    </button>
                    <button class="boton" type="submit" disabled=move || pidiendo.get()>
                        {move || if pidiendo.get() { t!("biblioteca-comprobando") } else { t!("biblioteca-revelar") }}
                    </button>
                </div>
            </form>
        </dialog>
    }
}
