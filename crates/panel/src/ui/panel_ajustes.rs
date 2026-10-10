//! «Ajustes del panel» (`/panel`): router, panel (idioma y zona horaria),
//! privacidad e instancia (DNS preferidos, dispositivos de confianza,
//! mantenimiento, servidor propio) y usuarios del panel.
//!
//! Lo delicado (credenciales del router, confirmar el firmware, usuarios)
//! pide la contraseña del panel si no se ha escrito en los últimos 5 minutos:
//! el servidor responde con [`crate::auth::ERROR_REAUTENTICACION`], la página
//! la pide en un diálogo y repite la operación.

use super::{Estado, Insignia, es_error_de_reautenticacion, mensaje_del_servidor};
use crate::panel_ajustes::{
    AjustesPanel, CamposInstancia, EquipoLan, InfoRouter, MAX_DNS, borrar_usuario,
    cambiar_contrasena, confirmar_firmware, crear_usuario, guardar_instancia, guardar_router,
    leer_panel, probar_router, reautenticar, zonas,
};
use crate::t;
use crate::ui::i18n::nombre_idioma;
use ::i18n::Idioma;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};

/// Longitud mínima de la contraseña del panel (la misma que comprueba el
/// servidor).
const MIN_CONTRASENA: usize = 12;

fn texto_error(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => mensaje_del_servidor(m),
        _ => t!("comun-error-conexion"),
    }
}

/// Diálogo de reautenticación compartido por las secciones: guarda la
/// operación que lo pidió para repetirla después.
#[derive(Clone, Copy)]
struct Reautenticacion {
    abierto: RwSignal<bool>,
    pendiente: StoredValue<Option<Callback<()>>>,
}

impl Reautenticacion {
    /// Trata el error de una operación delicada: si falta reautenticarse,
    /// abre el diálogo y repetirá `reintentar`; si no, deja el mensaje en
    /// `error`.
    fn tratar(self, e: &ServerFnError, error: RwSignal<Option<String>>, reintentar: Callback<()>) {
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
    let reauth = Reautenticacion {
        abierto: RwSignal::new(false),
        pendiente: StoredValue::new(None),
    };
    provide_context(reauth);
    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("panel-titulo")}</h1>
                <p class="pagina-sub">{t!("panel-sub")}</p>
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
        {move || reauth.abierto.get().then(|| view! { <DialogoReautenticar /> })}
    }
}

#[component]
fn Ajustes(a: AjustesPanel) -> impl IntoView {
    let campos = CamposInstancia {
        idioma: a.idioma,
        zona_horaria: a.zona_horaria.clone(),
        dns_preferidos: a.dns_preferidos.clone(),
        dispositivos_confianza: a.dispositivos_confianza.clone(),
        mantenimiento: a.mantenimiento.clone(),
        servidor_propio: a.servidor_propio.clone(),
    };
    let instancia = Instancia::new(campos);
    view! {
        <div class="ajustes-panel">
            <SeccionRouter
                url=a.router.url.clone()
                usuario=a.router.usuario.clone()
                solo_lectura=a.solo_lectura
                firmware=a.firmware.clone()
            />
            <SeccionPanel instancia />
            <SeccionPrivacidad instancia equipos=a.equipos.clone() />
            <SeccionUsuarios usuarios=a.usuarios.clone() yo=a.yo.clone() />
        </div>
    }
}

/// Operación que, si hay que reautenticarse, se repite a sí misma: `f`
/// recibe el propio callback para pasárselo a [`Reautenticacion::tratar`].
fn repetible(f: impl Fn(Callback<()>) + Send + Sync + 'static) -> Callback<()> {
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
fn mensajes(error: RwSignal<Option<String>>, aviso: RwSignal<Option<String>>) -> impl IntoView {
    view! {
        <p class="error-formulario" role="alert">{move || error.get()}</p>
        <p class="aviso-formulario" role="status">{move || aviso.get()}</p>
    }
}

/// Campo de texto ligado a `valor`.
fn campo(
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

// ---- Router

#[component]
fn SeccionRouter(
    url: String,
    usuario: String,
    solo_lectura: bool,
    firmware: Option<String>,
) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let configurado = !url.is_empty();
    let url = RwSignal::new(url);
    let usuario = RwSignal::new(usuario);
    let contrasena = RwSignal::new(String::new());
    let info = RwSignal::new(None::<InfoRouter>);
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    let solo_lectura = RwSignal::new(solo_lectura);
    let datos = move || {
        (
            url.get_untracked(),
            usuario.get_untracked(),
            contrasena.get_untracked(),
        )
    };

    let probar = move |_| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let (u, us, c) = datos();
        spawn_local(async move {
            match probar_router(u, us, c).await {
                Ok(i) => info.set(Some(i)),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            pendiente.set(false);
        });
    };
    let guardar = repetible(move |yo| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let (u, us, c) = datos();
        spawn_local(async move {
            match guardar_router(u, us, c).await {
                Ok(i) => {
                    // Con un firmware probado se quita el solo lectura; con
                    // uno sin probar, queda como estaba (o confirmado).
                    if i.probado {
                        solo_lectura.set(false);
                    }
                    info.set(Some(i));
                    contrasena.set(String::new());
                    aviso.set(Some(t!("panel-router-guardadas")));
                }
                Err(e) => reauth.tratar(&e, error, yo),
            }
            pendiente.set(false);
        });
    });
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        guardar.run(());
    };
    let confirmar = repetible(move |yo| {
        error.set(None);
        spawn_local(async move {
            match confirmar_firmware().await {
                Ok(()) => solo_lectura.set(false),
                Err(e) => reauth.tratar(&e, error, yo),
            }
        });
    });

    let firmware_texto = match &firmware {
        Some(f) => t!("panel-firmware", firmware = f.clone()),
        None => t!("panel-firmware-desconocido"),
    };
    let modo = move || {
        if solo_lectura.get() {
            view! {
                <Insignia estado=Estado::Aviso texto=t!("panel-modo-solo-lectura") />
                <p class="nota">{t!("panel-confirmar-texto")}</p>
                <div class="botones">
                    <button class="boton" type="button" on:click=move |_| confirmar.run(())>
                        {t!("panel-confirmar")}
                    </button>
                </div>
            }
            .into_any()
        } else {
            view! { <Insignia estado=Estado::Ok texto=t!("panel-modo-escritura") /> }.into_any()
        }
    };
    let detectado = move || {
        info.get().map(|i| {
            view! {
                <p>
                    {t!("instalar-detectado", modelo = i.modelo.clone(), firmware = i.firmware.clone())}
                </p>
            }
        })
    };

    view! {
        <section class="tarjeta" aria-labelledby="panel-router">
            <h2 id="panel-router">{t!("panel-router")}</h2>
            <div class="ajustes-estado">
                <p>{firmware_texto}</p>
                {modo}
            </div>
            {(!configurado)
                .then(|| view! { <p class="nota">{t!("panel-router-sin-configurar")}</p> })}
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="nota">{t!("panel-router-texto")}</p>
                {campo("panel-router-url", t!("panel-router-url"), "url", "off", url, None)}
                {campo(
                    "panel-router-usuario",
                    t!("panel-router-usuario"),
                    "text",
                    "off",
                    usuario,
                    None,
                )}
                {campo(
                    "panel-router-contrasena",
                    t!("panel-router-contrasena"),
                    "password",
                    "new-password",
                    contrasena,
                    None,
                )}
                {detectado}
                {mensajes(error, aviso)}
                <div class="botones">
                    <button
                        class="boton-sutil"
                        type="button"
                        disabled=move || pendiente.get()
                        on:click=probar
                    >
                        {t!("panel-router-probar")}
                    </button>
                    <button class="boton" type="submit" disabled=move || pendiente.get()>
                        {t!("panel-router-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}

// ---- Parámetros de la instancia

/// Parámetros de la instancia en edición: los guardan juntas las secciones
/// «Panel» y «Privacidad e instancia».
#[derive(Clone, Copy)]
struct Instancia {
    idioma: RwSignal<Idioma>,
    zona: RwSignal<String>,
    dns: RwSignal<String>,
    confianza: RwSignal<String>,
    mantenimiento: RwSignal<String>,
    servidor: RwSignal<String>,
    pendiente: RwSignal<bool>,
}

/// Lista escrita a mano: separada por comas, espacios o saltos de línea.
fn trocear(s: &str) -> Vec<String> {
    s.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|x| !x.is_empty())
        .map(str::to_owned)
        .collect()
}

impl Instancia {
    fn new(c: CamposInstancia) -> Self {
        Instancia {
            idioma: RwSignal::new(c.idioma),
            zona: RwSignal::new(c.zona_horaria),
            dns: RwSignal::new(c.dns_preferidos.join(", ")),
            confianza: RwSignal::new(c.dispositivos_confianza.join(", ")),
            mantenimiento: RwSignal::new(c.mantenimiento.unwrap_or_default()),
            servidor: RwSignal::new(c.servidor_propio.unwrap_or_default()),
            pendiente: RwSignal::new(false),
        }
    }

    fn fijar(self, c: CamposInstancia) {
        let n = Instancia::new(c);
        self.idioma.set(n.idioma.get_untracked());
        self.zona.set(n.zona.get_untracked());
        self.dns.set(n.dns.get_untracked());
        self.confianza.set(n.confianza.get_untracked());
        self.mantenimiento.set(n.mantenimiento.get_untracked());
        self.servidor.set(n.servidor.get_untracked());
    }

    fn campos(self) -> CamposInstancia {
        let opcional = |s: String| Some(s).filter(|s| !s.trim().is_empty());
        CamposInstancia {
            idioma: self.idioma.get_untracked(),
            zona_horaria: self.zona.get_untracked(),
            dns_preferidos: trocear(&self.dns.get_untracked()),
            dispositivos_confianza: trocear(&self.confianza.get_untracked()),
            mantenimiento: opcional(self.mantenimiento.get_untracked()),
            servidor_propio: opcional(self.servidor.get_untracked()),
        }
    }

    /// Guarda todos los parámetros; el resultado, en `error` o `aviso`.
    fn guardar(self, error: RwSignal<Option<String>>, aviso: RwSignal<Option<String>>) {
        error.set(None);
        aviso.set(None);
        self.pendiente.set(true);
        let campos = self.campos();
        spawn_local(async move {
            match guardar_instancia(campos).await {
                Ok(c) => {
                    self.fijar(c);
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
            self.pendiente.set(false);
        });
    }
}

#[component]
fn SeccionPanel(instancia: Instancia) -> impl IntoView {
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        instancia.guardar(error, aviso);
    };
    let idiomas = Idioma::TODOS
        .into_iter()
        .map(|i| {
            view! {
                <option value=i.codigo() lang=i.codigo() selected=move || instancia.idioma.get() == i>
                    {nombre_idioma(i)}
                </option>
            }
        })
        .collect_view();
    let lista_zonas = zonas()
        .into_iter()
        .map(|z| {
            let (valor, texto) = (z.clone(), z.clone());
            view! {
                <option value=z selected=move || instancia.zona.get() == valor>
                    {texto}
                </option>
            }
        })
        .collect_view();
    view! {
        <section class="tarjeta" aria-labelledby="panel-panel">
            <h2 id="panel-panel">{t!("panel-panel")}</h2>
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="campo">
                    <label for="panel-idioma">{t!("panel-idioma")}</label>
                    <select
                        id="panel-idioma"
                        on:change=move |ev| {
                            if let Some(i) = Idioma::from_codigo(&event_target_value(&ev)) {
                                instancia.idioma.set(i);
                            }
                        }
                    >
                        {idiomas}
                    </select>
                </p>
                <p class="campo">
                    <label for="panel-zona">{t!("panel-zona")}</label>
                    <select
                        id="panel-zona"
                        aria-describedby="panel-zona-ayuda"
                        on:change=move |ev| instancia.zona.set(event_target_value(&ev))
                    >
                        {lista_zonas}
                    </select>
                    <span class="nota" id="panel-zona-ayuda">{t!("panel-zona-ayuda")}</span>
                </p>
                {mensajes(error, aviso)}
                <div class="botones">
                    <button class="boton" type="submit" disabled=move || instancia.pendiente.get()>
                        {t!("panel-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}

/// Sugerencias de DNS públicos: nombre, direcciones y política de
/// privacidad (claves i18n).
fn sugerencias_dns() -> [(String, String, String); 3] {
    [
        (
            t!("panel-dns-quad9"),
            t!("panel-dns-quad9-ips"),
            t!("panel-dns-quad9-politica"),
        ),
        (
            t!("panel-dns-mullvad"),
            t!("panel-dns-mullvad-ips"),
            t!("panel-dns-mullvad-politica"),
        ),
        (
            t!("panel-dns-cloudflare"),
            t!("panel-dns-cloudflare-ips"),
            t!("panel-dns-cloudflare-politica"),
        ),
    ]
}

#[component]
fn SeccionPrivacidad(instancia: Instancia, equipos: Vec<EquipoLan>) -> impl IntoView {
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        instancia.guardar(error, aviso);
    };
    let sugerencias = sugerencias_dns()
        .into_iter()
        .map(|(nombre, ips, politica)| {
            let usar = t!("panel-dns-usar", nombre = nombre.clone());
            let enlace = t!("panel-dns-politica", nombre = nombre);
            view! {
                <li>
                    <button
                        class="boton-sutil"
                        type="button"
                        on:click=move |_| instancia.dns.set(ips.clone())
                    >
                        {usar}
                    </button>
                    <a href=politica rel="noopener noreferrer" target="_blank">
                        {enlace}
                    </a>
                </li>
            }
        })
        .collect_view();
    let anadir = move |mac: String| {
        instancia.confianza.update(|c| {
            let mut l = trocear(c);
            if !l.iter().any(|m| m.eq_ignore_ascii_case(&mac)) {
                l.push(mac);
            }
            *c = l.join(", ");
        });
    };
    let equipos = (!equipos.is_empty()).then(|| {
        let botones = equipos
            .into_iter()
            .map(|e| {
                let nombre = if e.nombre.is_empty() {
                    t!("panel-equipo-sin-nombre")
                } else {
                    e.nombre.clone()
                };
                let mac = e.mac.clone();
                view! {
                    <li>
                        <button
                            class="boton-sutil"
                            type="button"
                            on:click=move |_| anadir(mac.clone())
                        >
                            {t!("panel-confianza-anadir", nombre = nombre)}
                            <span class="mono">{e.mac}</span>
                        </button>
                    </li>
                }
            })
            .collect_view();
        view! {
            <div class="ajustes-sugerencias">
                <p class="nota">{t!("panel-confianza-equipos")}</p>
                <ul class="lista-limpia ajustes-botones">{botones}</ul>
            </div>
        }
    });
    view! {
        <section class="tarjeta" aria-labelledby="panel-privacidad">
            <h2 id="panel-privacidad">{t!("panel-privacidad")}</h2>
            <form class="ajustes-formulario" on:submit=enviar>
                <p class="nota">{t!("panel-privacidad-texto")}</p>
                {campo(
                    "panel-dns",
                    t!("panel-dns"),
                    "text",
                    "off",
                    instancia.dns,
                    Some(t!("panel-dns-ayuda", n = MAX_DNS)),
                )}
                <div class="ajustes-sugerencias">
                    <p class="nota">{t!("panel-dns-sugerencias")}</p>
                    <ul class="lista-limpia ajustes-botones">{sugerencias}</ul>
                </div>
                {campo(
                    "panel-confianza",
                    t!("panel-confianza"),
                    "text",
                    "off",
                    instancia.confianza,
                    Some(t!("panel-confianza-ayuda")),
                )}
                {equipos}
                {campo(
                    "panel-mantenimiento",
                    t!("panel-mantenimiento"),
                    "text",
                    "off",
                    instancia.mantenimiento,
                    Some(t!("panel-mantenimiento-ayuda")),
                )}
                {campo(
                    "panel-servidor",
                    t!("panel-servidor"),
                    "text",
                    "off",
                    instancia.servidor,
                    Some(t!("panel-servidor-ayuda")),
                )}
                {mensajes(error, aviso)}
                <div class="botones">
                    <button class="boton" type="submit" disabled=move || instancia.pendiente.get()>
                        {t!("panel-guardar")}
                    </button>
                </div>
            </form>
        </section>
    }
}

// ---- Usuarios

#[component]
fn SeccionUsuarios(usuarios: Vec<String>, yo: String) -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
    let usuarios = RwSignal::new(usuarios);
    let yo = StoredValue::new(yo);
    let error = RwSignal::new(None::<String>);
    let aviso = RwSignal::new(None::<String>);
    let pendiente = RwSignal::new(false);
    // Usuario cuya contraseña se cambia / cuyo borrado se confirma.
    let editando = RwSignal::new(None::<String>);
    let borrando = RwSignal::new(None::<String>);
    let clave_nueva = RwSignal::new(String::new());
    let nombre = RwSignal::new(String::new());
    let clave = RwSignal::new(String::new());

    let borrar = repetible(move |yo_cb| {
        let Some(u) = borrando.get_untracked() else {
            return;
        };
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        spawn_local(async move {
            match borrar_usuario(u.clone()).await {
                Ok(()) => {
                    usuarios.update(|l| l.retain(|x| *x != u));
                    borrando.set(None);
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => reauth.tratar(&e, error, yo_cb),
            }
            pendiente.set(false);
        });
    });
    let cambiar = repetible(move |yo_cb| {
        let Some(u) = editando.get_untracked() else {
            return;
        };
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let c = clave_nueva.get_untracked();
        spawn_local(async move {
            match cambiar_contrasena(u, c).await {
                Ok(()) => {
                    editando.set(None);
                    clave_nueva.set(String::new());
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => reauth.tratar(&e, error, yo_cb),
            }
            pendiente.set(false);
        });
    });
    let crear = repetible(move |yo_cb| {
        error.set(None);
        aviso.set(None);
        pendiente.set(true);
        let (u, c) = (nombre.get_untracked(), clave.get_untracked());
        spawn_local(async move {
            match crear_usuario(u.clone(), c).await {
                Ok(()) => {
                    usuarios.update(|l| l.push(u));
                    nombre.set(String::new());
                    clave.set(String::new());
                    aviso.set(Some(t!("panel-guardado")));
                }
                Err(e) => reauth.tratar(&e, error, yo_cb),
            }
            pendiente.set(false);
        });
    });

    let fila = move |u: String| {
        let es_yo = yo.with_value(|y| *y == u);
        let (u1, u2, u3, u4) = (u.clone(), u.clone(), u.clone(), u.clone());
        let editar_este = move || editando.get().as_deref() == Some(u1.as_str());
        let borrar_este = move || borrando.get().as_deref() == Some(u2.as_str());
        let formulario = move || {
            editar_este().then(|| {
                view! {
                    <form
                        class="ajustes-usuario-form"
                        on:submit=move |ev: SubmitEvent| {
                            ev.prevent_default();
                            cambiar.run(());
                        }
                    >
                        {campo(
                            "panel-usuario-clave-nueva",
                            t!("panel-usuario-clave-nueva", n = MIN_CONTRASENA),
                            "password",
                            "new-password",
                            clave_nueva,
                            None,
                        )}
                        <div class="botones">
                            <button
                                class="boton-sutil"
                                type="button"
                                on:click=move |_| editando.set(None)
                            >
                                {t!("panel-cancelar")}
                            </button>
                            <button class="boton" type="submit" disabled=move || pendiente.get()>
                                {t!("panel-usuario-cambiar")}
                            </button>
                        </div>
                    </form>
                }
            })
        };
        let botones = move || {
            if borrar_este() {
                let u = u3.clone();
                view! {
                    <button class="boton-sutil" type="button" on:click=move |_| borrando.set(None)>
                        {t!("panel-cancelar")}
                    </button>
                    <button
                        class="boton boton-borrar"
                        type="button"
                        disabled=move || pendiente.get()
                        on:click=move |_| borrar.run(())
                    >
                        {t!("panel-usuario-borrar-de", usuario = u.clone())}
                    </button>
                }
                .into_any()
            } else {
                let (ue, ub) = (u4.clone(), u4.clone());
                view! {
                    <button
                        class="boton-sutil"
                        type="button"
                        aria-label=t!("panel-usuario-contrasena-de", usuario = ue.clone())
                        on:click=move |_| {
                            borrando.set(None);
                            clave_nueva.set(String::new());
                            editando.set(Some(ue.clone()));
                        }
                    >
                        {t!("panel-usuario-contrasena")}
                    </button>
                    <button
                        class="boton-sutil"
                        type="button"
                        aria-label=t!("panel-usuario-borrar-de", usuario = ub.clone())
                        disabled=move || es_yo || usuarios.with(|l| l.len() <= 1)
                        on:click=move |_| {
                            editando.set(None);
                            borrando.set(Some(ub.clone()));
                        }
                    >
                        {t!("panel-usuario-borrar")}
                    </button>
                }
                .into_any()
            }
        };
        view! {
            <li class="ajustes-usuario">
                <div class="ajustes-usuario-fila">
                    <span class="ajustes-usuario-nombre">
                        {u.clone()}
                        {es_yo.then(|| view! { " " <span class="nota">{t!("panel-usuario-tu")}</span> })}
                    </span>
                    <div class="botones">{botones}</div>
                </div>
                {formulario}
            </li>
        }
    };

    view! {
        <section class="tarjeta" aria-labelledby="panel-usuarios">
            <h2 id="panel-usuarios">{t!("panel-usuarios")}</h2>
            <p class="nota">{t!("panel-usuarios-texto")}</p>
            <ul class="lista-limpia ajustes-usuarios">
                <For each=move || usuarios.get() key=|u| u.clone() children=fila />
            </ul>
            {mensajes(error, aviso)}
            <form
                class="ajustes-formulario"
                on:submit=move |ev: SubmitEvent| {
                    ev.prevent_default();
                    crear.run(());
                }
            >
                <h3>{t!("panel-usuario-nuevo")}</h3>
                {campo("panel-usuario-nombre", t!("panel-usuario-nombre"), "text", "off", nombre, None)}
                {campo(
                    "panel-usuario-clave",
                    t!("panel-usuario-clave", n = MIN_CONTRASENA),
                    "password",
                    "new-password",
                    clave,
                    None,
                )}
                <div class="botones">
                    <button class="boton" type="submit" disabled=move || pendiente.get()>
                        {t!("panel-usuario-crear")}
                    </button>
                </div>
            </form>
        </section>
    }
}

// ---- Reautenticación

/// Pide la contraseña del panel y, si es buena, repite la operación
/// pendiente.
#[component]
fn DialogoReautenticar() -> impl IntoView {
    let reauth = expect_context::<Reautenticacion>();
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
    let enviar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if pidiendo.get_untracked() {
            return;
        }
        error.set(None);
        pidiendo.set(true);
        let clave = contrasena.get_untracked();
        spawn_local(async move {
            let r = reautenticar(clave).await;
            contrasena.set(String::new());
            pidiendo.set(false);
            match r {
                Ok(()) => {
                    let pendiente = reauth.pendiente.get_value();
                    reauth.pendiente.set_value(None);
                    if let Some(d) = dlg.get_untracked() {
                        d.close();
                    }
                    if let Some(p) = pendiente {
                        p.run(());
                    }
                }
                Err(e) => error.set(Some(texto_error(&e))),
            }
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
            aria-labelledby="reautenticar-titulo"
            aria-modal="true"
            on:close=move |_| reauth.abierto.set(false)
        >
            <form class="dialogo-cuerpo" on:submit=enviar>
                <header class="dialogo-cabecera">
                    <h2 id="reautenticar-titulo">{t!("panel-reautenticar")}</h2>
                </header>
                <p>{t!("panel-reautenticar-texto")}</p>
                <p class="campo">
                    <label for="reautenticar-contrasena">
                        {t!("panel-reautenticar-contrasena")}
                    </label>
                    <input
                        id="reautenticar-contrasena"
                        type="password"
                        autocomplete="current-password"
                        required
                        autofocus
                        prop:value=contrasena
                        on:input=move |ev| contrasena.set(event_target_value(&ev))
                    />
                </p>
                <p class="error-formulario" role="alert">{move || error.get()}</p>
                <div class="dialogo-pie">
                    <button class="boton-sutil" type="button" on:click=cerrar>
                        {t!("panel-cancelar")}
                    </button>
                    <button class="boton" type="submit" disabled=move || pidiendo.get()>
                        {t!("panel-reautenticar-seguir")}
                    </button>
                </div>
            </form>
        </dialog>
    }
}
