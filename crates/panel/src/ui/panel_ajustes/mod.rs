//! «Ajustes del panel» (`/panel`): solo lo del panel. Panel (idioma por
//! defecto y zona horaria), visualización (el modo de cada usuario,
//! [`super::vista::SeccionVista`]), tema ([`super::tema::SeccionTema`]) y usuarios del panel. No está en el menú: se abre
//! con el nombre de usuario. Lo del router está en «Configuración del
//! router» ([`super::router`]). Los demás parámetros de la instancia (DNS
//! preferidos, dispositivos de confianza…) los pedirán en contexto las
//! recomendaciones de la fase 4.
//!
//! Lo delicado (usuarios aquí; credenciales y firmware en «Configuración del
//! router») pide la contraseña del panel si no se ha escrito en los últimos
//! 5 minutos: el servidor responde con
//! [`crate::auth::ERROR_REAUTENTICACION`], la página la pide en un diálogo
//! ([`DialogoReautenticar`]) y repite la operación.

use super::{es_error_de_reautenticacion, mensaje_del_servidor};
use crate::panel_ajustes::{
    AjustesPanel, CamposInstancia, IdiomaPorDefecto, desfases_fijos, guardar_instancia, leer_panel,
};
use crate::t;
use crate::ui::charts::Zona;
use crate::ui::i18n::nombre_idioma;
use crate::ui::tema::SeccionTema;
use crate::ui::vista::SeccionVista;
use ::i18n::Idioma;
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use leptos_router::components::A;

mod reautenticar;
mod usuarios;

pub(crate) use reautenticar::DialogoReautenticar;
use usuarios::SeccionUsuarios;

/// Longitud mínima de la contraseña del panel (la misma que comprueba el
/// servidor).
const MIN_CONTRASENA: usize = 12;

pub(crate) fn texto_error(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => mensaje_del_servidor(m),
        _ => t!("comun-error-conexion"),
    }
}

/// Diálogo de reautenticación compartido por las secciones: guarda la
/// operación que lo pidió para repetirla después.
#[derive(Clone, Copy)]
pub(crate) struct Reautenticacion {
    abierto: RwSignal<bool>,
    pendiente: StoredValue<Option<Callback<()>>>,
}

impl Reautenticacion {
    /// Pone el diálogo en el contexto de la página; la página pinta
    /// [`DialogoReautenticar`] mientras [`Reautenticacion::abierto`].
    pub(crate) fn proveer() -> Self {
        let r = Reautenticacion {
            abierto: RwSignal::new(false),
            pendiente: StoredValue::new(None),
        };
        provide_context(r);
        r
    }

    /// El diálogo de la página si ya lo hay y, si no, uno nuevo; `true`
    /// si es nuevo, y entonces lo pinta quien lo pidió.
    pub(crate) fn de_la_pagina() -> (Self, bool) {
        match use_context::<Reautenticacion>() {
            Some(r) => (r, false),
            None => (Self::proveer(), true),
        }
    }

    /// ¿Hay que pintar el diálogo?
    pub(crate) fn abierto(self) -> bool {
        self.abierto.get()
    }

    /// Trata el error de una operación delicada: si falta reautenticarse,
    /// abre el diálogo y repetirá `reintentar`; si no, deja el mensaje en
    /// `error`.
    pub(crate) fn tratar(
        self,
        e: &ServerFnError,
        error: RwSignal<Option<String>>,
        reintentar: Callback<()>,
    ) {
        if es_error_de_reautenticacion(e) {
            self.pendiente.set_value(Some(reintentar));
            self.abierto.set(true);
        } else {
            error.set(Some(texto_error(e)));
        }
    }
}

/// Página «Ajustes del panel».
#[component]
pub fn PaginaPanel() -> impl IntoView {
    let datos = Resource::new(|| (), |_| leer_panel());
    let reauth = Reautenticacion::proveer();
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("panel-titulo")}</h1>
                <p class="pagina-sub">
                    {t!("panel-sub")} " " {t!("panel-sub-router")} " "
                    <A href=super::router::RUTA_CONEXION>{t!("panel-sub-router-enlace")}</A>
                    "."
                </p>
            </div>
        </header>
        <Suspense fallback=|| view! { <p>{t!("comun-cargando")}</p> }>
            {move || Suspend::new(async move {
                match datos.await {
                    Ok(a) => view! { <Ajustes a /> }.into_any(),
                    Err(e) => {
                        view! { <p class="error-formulario" role="alert">{texto_error(&e)}</p> }
                            .into_any()
                    }
                }
            })}
        </Suspense>
        {move || reauth.abierto().then(|| view! { <DialogoReautenticar /> })}
    }
}

#[component]
fn Ajustes(a: AjustesPanel) -> impl IntoView {
    let instancia = Instancia::new(&a);
    view! {
        <div class="ajustes-panel">
            <SeccionPanel instancia />
            <SeccionVista />
            <SeccionTema />
            <SeccionUsuarios usuarios=a.usuarios.clone() yo=a.yo.clone() />
        </div>
    }
}

/// Operación que, si hay que reautenticarse, se repite a sí misma: `f`
/// recibe el propio callback para pasárselo a [`Reautenticacion::tratar`].
pub(crate) fn repetible(f: impl Fn(Callback<()>) + Send + Sync + 'static) -> Callback<()> {
    let yo = StoredValue::new(None::<Callback<()>>);
    let cb = Callback::new(move |()| {
        if let Some(cb) = yo.get_value() {
            f(cb);
        }
    });
    yo.set_value(Some(cb));
    cb
}

/// Mensaje de error y aviso de éxito de un formulario.
pub(crate) fn mensajes(
    error: RwSignal<Option<String>>,
    aviso: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <p class="error-formulario" role="alert">{move || error.get()}</p>
        <p class="aviso-formulario" role="status">{move || aviso.get()}</p>
    }
}

/// Campo de texto ligado a `valor`.
pub(crate) fn campo(
    id: &'static str,
    etiqueta: String,
    tipo: &'static str,
    autocompletar: &'static str,
    valor: RwSignal<String>,
    ayuda: Option<String>,
) -> impl IntoView {
    let ayuda_id = format!("{id}-ayuda");
    let descrita = ayuda.is_some().then(|| ayuda_id.clone());
    view! {
        <p class="campo">
            <label for=id>{etiqueta}</label>
            <input
                id=id
                name=id
                type=tipo
                autocomplete=autocompletar
                autocapitalize="none"
                spellcheck="false"
                aria-describedby=descrita
                prop:value=valor
                on:input=move |ev| valor.set(event_target_value(&ev))
            />
            {ayuda.map(|a| view! { <span class="nota" id=ayuda_id>{a}</span> })}
        </p>
    }
}

// ---- Idioma y zona horaria

/// Idioma por defecto y zona horaria en edición. Solo se guardan estos dos
/// parámetros de la instancia: los de las recomendaciones no se envían.
#[derive(Clone, Copy)]
struct Instancia {
    idioma: RwSignal<IdiomaPorDefecto>,
    /// El idioma por defecto que tiene guardado la instancia: si cambia, la
    /// página se recarga para pintarse con el que le toque ahora.
    idioma_guardado: StoredValue<IdiomaPorDefecto>,
    zona: RwSignal<String>,
    /// La zona horaria que tiene guardada la instancia: si cambia, la página
    /// se recarga para pintar las fechas en la nueva.
    zona_guardada: StoredValue<String>,
}

impl Instancia {
    fn new(a: &AjustesPanel) -> Self {
        Instancia {
            idioma: RwSignal::new(a.idioma),
            idioma_guardado: StoredValue::new(a.idioma),
            zona: RwSignal::new(a.zona_horaria.clone()),
            zona_guardada: StoredValue::new(a.zona_horaria.clone()),
        }
    }

    /// Pone los valores ya normalizados que devolvió el servidor (solo los
    /// que guardó).
    fn fijar(self, c: CamposInstancia) {
        if let Some(i) = c.idioma {
            self.idioma.set(i);
        }
        if let Some(z) = c.zona_horaria {
            self.zona.set(z);
        }
    }

    fn campos(self) -> CamposInstancia {
        CamposInstancia {
            idioma: Some(self.idioma.get_untracked()),
            zona_horaria: Some(self.zona.get_untracked()),
            ..CamposInstancia::default()
        }
    }

    /// Guarda el idioma y la zona; el resultado, en `error` o `aviso`.
    fn guardar(
        self,
        pendiente: RwSignal<bool>,
        error: RwSignal<Option<String>>,
        aviso: RwSignal<Option<String>>,
    ) {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let campos = self.campos();
        spawn_local(async move {
            match guardar_instancia(campos).await {
                Ok(c) => {
                    // Otro idioma por defecto puede cambiar el de esta misma
                    // página (si no se ha elegido uno en el menú), y otra zona
                    // horaria, sus fechas: se vuelve a pedir, para no quedarse
                    // a medias entre dos idiomas o dos zonas.
                    let otro_idioma = c
                        .idioma
                        .is_some_and(|i| i != self.idioma_guardado.get_value());
                    let otra_zona = c
                        .zona_horaria
                        .as_ref()
                        .is_some_and(|z| *z != self.zona_guardada.get_value());
                    if let Some(i) = c.idioma {
                        self.idioma_guardado.set_value(i);
                    }
                    if let Some(z) = &c.zona_horaria {
                        self.zona_guardada.set_value(z.clone());
                    }
                    if otro_idioma || otra_zona {
                        let _ = window().location().reload();
                    }
                    self.fijar(c);
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    }
}

/// Valor de la opción «el del navegador» del idioma por defecto.
const VALOR_NAVEGADOR: &str = "navegador";

/// Valor de la opción «Otra» (desfase fijo) de la zona horaria.
const VALOR_OTRA: &str = "otra";

/// Desfase de «Otra» si no hay uno guardado.
const OTRA_POR_DEFECTO: &str = "UTC+01:00";

/// La zona es un desfase fijo distinto de UTC (va en «Otra»).
fn es_otra(codigo: &str) -> bool {
    matches!(Zona::desde_texto(codigo), Some(Zona::Fija(d)) if d != 0)
}

/// Idioma por defecto de un valor del selector.
fn idioma_por_defecto(valor: &str) -> Option<IdiomaPorDefecto> {
    if valor == VALOR_NAVEGADOR {
        Some(IdiomaPorDefecto::Navegador)
    } else {
        Idioma::from_codigo(valor).map(IdiomaPorDefecto::Fijo)
    }
}

#[component]
fn SeccionPanel(instancia: Instancia) -> impl IntoView {
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        instancia.guardar(pendiente, error, aviso);
    };
    let navegador = view! {
        <option
            value=VALOR_NAVEGADOR
            selected=move || instancia.idioma.get() == IdiomaPorDefecto::Navegador
        >
            {t!("panel-idioma-navegador")}
        </option>
    };
    let idiomas = Idioma::TODOS
        .into_iter()
        .map(|i| {
            view! {
                <option
                    value=i.codigo()
                    lang=i.codigo()
                    selected=move || instancia.idioma.get() == IdiomaPorDefecto::Fijo(i)
                >
                    {nombre_idioma(i)}
                </option>
            }
        })
        .collect_view();
    // El último desfase elegido en «Otra», para volver a él.
    let otra = RwSignal::new(if es_otra(&instancia.zona.get_untracked()) {
        instancia.zona.get_untracked()
    } else {
        OTRA_POR_DEFECTO.to_owned()
    });
    // Solo cambia al entrar o salir de «Otra»: elegir otro desfase no rehace
    // su selector (no pierde el foco).
    let en_otra = Memo::new(move |_| instancia.zona.with(|z| es_otra(z)));
    let lista_zonas = [
        (Zona::Peninsular, t!("panel-zona-peninsular")),
        (Zona::Canarias, t!("panel-zona-canarias")),
        (Zona::Fija(0), t!("panel-zona-utc")),
    ]
    .into_iter()
    .map(|(z, texto)| {
        let valor = z.codigo();
        let v = valor.clone();
        view! {
            <option value=valor selected=move || instancia.zona.with(|z| *z == v)>
                {texto}
            </option>
        }
    })
    .collect_view();
    let cambiar_zona = move |ev| {
        let valor = event_target_value(&ev);
        if valor == VALOR_OTRA {
            instancia.zona.set(otra.get_untracked());
        } else {
            instancia.zona.set(valor);
        }
    };
    let desfases = move || {
        en_otra.get().then(|| {
            // Uno guardado que no está en la lista (escrito a mano), también.
            let mut lista = desfases_fijos();
            let actual = instancia.zona.get_untracked();
            if !lista.contains(&actual) {
                lista.push(actual);
            }
            let opciones = lista
                .into_iter()
                .map(|d| {
                    let (valor, texto) = (d.clone(), d.clone());
                    view! {
                        <option value=d selected=move || instancia.zona.with(|z| *z == valor)>
                            {texto}
                        </option>
                    }
                })
                .collect_view();
            view! {
                <p class="campo">
                    <label for="panel-zona-otra">{t!("panel-zona-desfase")}</label>
                    <select
                        id="panel-zona-otra"
                        on:change=move |ev| {
                            let d = event_target_value(&ev);
                            otra.set(d.clone());
                            instancia.zona.set(d);
                        }
                    >
                        {opciones}
                    </select>
                </p>
            }
        })
    };
    view! {
        <section class="tarjeta" aria-labelledby="panel-panel">
            <h2 id="panel-panel">{t!("panel-panel")}</h2>
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="campo">
                    <label for="panel-idioma">{t!("panel-idioma")}</label>
                    <select
                        id="panel-idioma"
                        aria-describedby="panel-idioma-ayuda"
                        on:change=move |ev| {
                            if let Some(i) = idioma_por_defecto(&event_target_value(&ev)) {
                                instancia.idioma.set(i);
                            }
                        }
                    >
                        {navegador}
                        {idiomas}
                    </select>
                    <span class="nota" id="panel-idioma-ayuda">{t!("panel-idioma-ayuda")}</span>
                </p>
                <p class="campo">
                    <label for="panel-zona">{t!("panel-zona")}</label>
                    <select
                        id="panel-zona"
                        aria-describedby="panel-zona-ayuda"
                        on:change=cambiar_zona
                    >
                        {lista_zonas}
                        <option value=VALOR_OTRA selected=en_otra>
                            {t!("panel-zona-otra")}
                        </option>
                    </select>
                    <span class="nota" id="panel-zona-ayuda">{t!("panel-zona-ayuda")}</span>
                </p>
                {desfases}
                {mensajes(error, aviso)}
                <div class="botones">
                    <button class="b-pri" type="submit" disabled=move || pendiente.get()>
                        {t!("panel-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}
