//! Historial (`/historial`): los cambios hechos desde el panel y los
//! secretos revelados, del más reciente al más antiguo, 25 por página.
//!
//! Cada cambio muestra fecha (en la zona de la instancia), usuario, función
//! (enlace a su página del explorador), riesgo, estado, un resumen del diff y
//! su detalle desplegable. «Deshacer» solo aparece si el gestor lo aceptaría
//! (el servidor lo vuelve a comprobar) y abre el mismo diálogo que el
//! explorador, con la inversa. Los cambios en `aplicando` o `vigilando` se
//! refrescan cada 2 s hasta que terminan; si no, no se sondea.
//!
//! Un cambio de varios pasos (un grupo) es una sola fila; su detalle lista
//! los pasos con su resumen humano, su estado y su diff.
//!
//! Los revelados no llevan el valor: solo quién, cuándo y qué campo (de qué
//! ajuste, si se mostró desde Ajustes; de qué cambio, si era un valor
//! anterior). Un paso que cambió un secreto ofrece «Mostrar el valor
//! anterior» ([`revelar_anterior`]), con la contraseña del panel.

#[cfg(feature = "ssr")]
pub(crate) mod servidor;

use crate::t;
use crate::ui::{
    Estado, Insignia,
    charts::{formato_fecha, hora_local},
    explorador::{
        CampoCambio, DialogoCambio, EstadoDeCambio, PreparadoVista, RiesgoCambio, TablaDiff,
        texto_error,
    },
    panel_ajustes::{DialogoReautenticar, Reautenticacion},
    secreto::{ObjetivoSecreto, Secreto},
};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use leptos::task::spawn_local;
use leptos_router::{components::A, hooks::use_query_map};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Duration};

/// Entradas por página.
pub const POR_PAGINA: u32 = 25;

/// Cada cuánto se refrescan los cambios en curso.
const REFRESCO: Duration = Duration::from_secs(2);

/// Cambio al que lleva un enlace «deshecho por» o «deshace a», con la
/// página del historial en la que está.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vinculo {
    pub id: i64,
    pub pagina: u32,
}

/// Un cambio del historial (secretos enmascarados).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CambioFila {
    pub id: i64,
    /// Creación y último cambio de estado (segundos Unix).
    pub ts: i64,
    pub updated: i64,
    pub usuario: Option<String>,
    pub funcion: String,
    /// Página de la función en el explorador, si sigue en el catálogo.
    pub ruta: Option<String>,
    pub instancia: Option<String>,
    pub riesgo: RiesgoCambio,
    pub estado: EstadoDeCambio,
    /// «3 campos: Activado, Dirección IPv4, …».
    pub resumen: String,
    pub campos: Vec<CampoCambio>,
    pub nota: Option<String>,
    pub error: Option<String>,
    /// Cambio que lo deshizo.
    pub deshecho_por: Option<Vinculo>,
    /// Cambio que este deshace.
    pub deshace: Option<Vinculo>,
    /// El gestor aceptaría deshacerlo ahora.
    pub puede_deshacer: bool,
    /// Resumen humano del grupo, si lo tiene.
    pub titulo: Option<String>,
    /// Pasos del grupo (vacío si es de un solo paso).
    pub pasos: Vec<PasoFila>,
    /// Secretos con valor anterior guardado (cifrado): `(orden del paso,
    /// parámetro)`. Cada uno ofrece «Mostrar el valor anterior».
    #[serde(default)]
    pub anteriores: Vec<(i64, String)>,
}

/// Estado de un paso de un grupo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstadoPasoVista {
    Pendiente,
    Omitido,
    Aplicando,
    Aplicado,
    Fallido,
    Revertido,
}

impl EstadoPasoVista {
    pub fn etiqueta(self) -> String {
        match self {
            EstadoPasoVista::Pendiente => t!("historial-paso-pendiente"),
            EstadoPasoVista::Omitido => t!("historial-paso-omitido"),
            EstadoPasoVista::Aplicando => t!("historial-paso-aplicando"),
            EstadoPasoVista::Aplicado => t!("historial-paso-aplicado"),
            EstadoPasoVista::Fallido => t!("historial-paso-fallido"),
            EstadoPasoVista::Revertido => t!("historial-paso-revertido"),
        }
    }

    pub fn estado(self) -> Estado {
        match self {
            EstadoPasoVista::Aplicado => Estado::Ok,
            EstadoPasoVista::Revertido => Estado::Aviso,
            EstadoPasoVista::Fallido => Estado::Error,
            _ => Estado::Neutro,
        }
    }
}

/// Un paso de un grupo del historial (secretos enmascarados).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasoFila {
    /// Posición en el grupo (desde 1).
    pub numero: i64,
    /// Resumen humano (o el nombre de la función).
    pub etiqueta: String,
    pub funcion: String,
    pub instancia: Option<String>,
    pub estado: EstadoPasoVista,
    pub campos: Vec<CampoCambio>,
    pub error: Option<String>,
}

/// Un secreto revelado (sin su valor).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReveladoFila {
    pub id: i64,
    pub ts: i64,
    /// `None` si el usuario ya no existe.
    pub usuario: Option<String>,
    pub funcion: String,
    /// Página de la función en el explorador; `None` si se mostró desde
    /// Ajustes (allí no se nombra la función).
    pub ruta: Option<String>,
    /// El parámetro; si se mostró desde Ajustes, la etiqueta del campo.
    pub campo: String,
    pub instancia: Option<String>,
    /// Título del ajuste, si se mostró desde Ajustes.
    #[serde(default)]
    pub ajuste: Option<String>,
    /// Cambio cuyo valor anterior se mostró, si fue eso.
    #[serde(default)]
    pub cambio: Option<i64>,
}

/// Entrada del historial.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntradaVista {
    Cambio(CambioFila),
    Revelado(ReveladoFila),
}

/// Una página del historial.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginaHistorial {
    pub entradas: Vec<EntradaVista>,
    /// Número de página (desde 1).
    pub pagina: u32,
    /// Entradas en total (cambios y revelados).
    pub total: u32,
    /// Hay una página más antigua.
    pub hay_mas: bool,
}

impl PaginaHistorial {
    /// Páginas que hay con `total` entradas (al menos una).
    pub fn paginas(&self) -> u32 {
        self.total.div_ceil(POR_PAGINA).max(1)
    }

    /// Algún cambio que aún no ha terminado.
    pub fn hay_en_curso(&self) -> bool {
        self.entradas.iter().any(|e| match e {
            EntradaVista::Cambio(c) => en_curso(c.estado),
            EntradaVista::Revelado(_) => false,
        })
    }
}

/// El cambio todavía va a cambiar de estado.
fn en_curso(e: EstadoDeCambio) -> bool {
    !e.final_()
}

/// «1 campo: Activado» / «3 campos: Activado, Dirección IPv4, Máscara, …».
pub fn resumen(campos: &[CampoCambio]) -> String {
    const MOSTRADOS: usize = 3;
    let n = campos.len();
    if n == 0 {
        return t!("historial-sin-campos");
    }
    let nombres: Vec<&str> = campos
        .iter()
        .take(MOSTRADOS)
        .map(|c| c.etiqueta.as_str())
        .collect();
    let mut texto = t!("historial-campos", n = n, nombres = nombres.join(", "));
    if n > MOSTRADOS {
        texto.push_str(", …");
    }
    texto
}

/// «09/10/2026 14:05» o, con segundos, «09/10/2026 14:05:09» (hora
/// en la zona de la instancia; la fecha, según el idioma).
pub fn fecha_hora(ts: i64, con_segundos: bool) -> String {
    let h = hora_local(ts);
    let base = format!("{} {:02}:{:02}", formato_fecha(&h), h.hora, h.minuto);
    if con_segundos {
        format!("{base}:{:02}", h.segundo)
    } else {
        base
    }
}

// ------------------------------------------------------------ server functions

/// Página `pagina` (desde 1) del historial: cambios y revelados del más
/// reciente al más antiguo.
#[server(endpoint = "historial", input = Json)]
pub async fn historial(pagina: u32) -> Result<PaginaHistorial, ServerFnError> {
    crate::auth::require_user().await?;
    let state = crate::ui::explorador::servidor::app_state()?;
    servidor::pagina(&state, pagina).await
}

/// Prepara el cambio inverso de `change_id` (solo uno aplicado que nadie
/// haya deshecho). Se confirma con `confirmar_cambio`, como cualquier otro.
#[server(endpoint = "deshacer", input = Json)]
pub async fn deshacer(change_id: i64) -> Result<PreparadoVista, ServerFnError> {
    crate::auth::require_user().await?;
    let state = crate::ui::explorador::servidor::app_state()?;
    servidor::deshacer(&state, change_id).await
}

/// Valor anterior del secreto `campo` (el parámetro) del paso `orden` del
/// cambio `cambio`, descifrado del historial. Mismas reglas que
/// [`revelar_ajuste`](crate::ajustes_router::revelar_ajuste): con
/// `contrasena` vacía, solo con la ventana de reautenticación abierta (si
/// no, 401); con la del panel, con el freno del login. Queda anotado, sin el
/// valor, en `reveals` (con el cambio). La respuesta va con `no-store`. 404
/// si ese paso no guardó un valor anterior secreto con ese nombre.
#[server(endpoint = "revelar_anterior", input = Json)]
pub async fn revelar_anterior(
    cambio: i64,
    orden: i64,
    campo: String,
    #[server(default)] contrasena: String,
) -> Result<String, ServerFnError> {
    servidor::sin_guardar();
    let user = crate::auth::require_user().await?;
    let state = crate::ui::explorador::servidor::app_state()?;
    servidor::revelar_anterior(&state, user, cambio, orden, &campo, &contrasena).await
}

// ------------------------------------------------------------ interfaz

/// Icono de ojo de los revelados (decorativo).
#[component]
fn IconoOjo() -> impl IntoView {
    view! {
        <svg
            class="icono icono-ojo"
            viewBox="0 0 16 16"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
        >
            <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
            <circle cx="8" cy="8" r="1.9" />
        </svg>
    }
}

fn enlace_vinculo(texto: String, v: Vinculo) -> impl IntoView {
    view! {
        <A href=format!("/historial?pagina={}#cambio-{}", v.pagina, v.id)>
            {format!("{texto} #{}", v.id)}
        </A>
    }
}

/// Función con su enlace al explorador (si sigue existiendo) y la instancia.
fn funcion_enlazada(funcion: String, ruta: Option<String>, instancia: Option<String>) -> AnyView {
    let nombre = match ruta {
        Some(r) => view! { <A href=r attr:class="mono">{funcion}</A> }.into_any(),
        None => view! { <span class="mono">{funcion}</span> }.into_any(),
    };
    view! {
        {nombre}
        {instancia.map(|i| view! { <span class="hist-instancia">{t!("historial-instancia-sep", i = i)}</span> })}
    }
    .into_any()
}

#[component]
fn FilaRevelado(r: ReveladoFila) -> impl IntoView {
    let instancia = r.instancia.map(|i| t!("historial-instancia-par", i = i));
    // Desde Ajustes: el ajuste y el campo en lenguaje claro; un valor
    // anterior: el parámetro y su cambio; si no, el parámetro y la función.
    let que = match (r.ajuste, r.cambio) {
        (Some(ajuste), _) => view! {
            {t!("historial-revelado")}
            " "
            <span class="hist-campo">{r.campo}</span>
            " "
            {t!("historial-revelado-de")}
            " "
            <span>{ajuste}</span>
            {instancia}
        }
        .into_any(),
        (None, Some(cambio)) => view! {
            {t!("historial-revelado-anterior")}
            " "
            <span class="mono">{r.campo}</span>
            " "
            {t!("historial-revelado-cambio", id = cambio)}
        }
        .into_any(),
        (None, None) => view! {
            {t!("historial-revelado")}
            " "
            <span class="mono">{r.campo}</span>
            " "
            {t!("historial-revelado-de")}
            " "
            {funcion_enlazada(r.funcion, r.ruta, None)}
            {instancia}
        }
        .into_any(),
    };
    view! {
        <li class="hist-entrada hist-revelado" id=format!("revelado-{}", r.id)>
            <div class="hist-cabeza">
                <span class="hist-hora cifras">{fecha_hora(r.ts, false)}</span>
                <span class="hist-quien">{r.usuario.unwrap_or_else(|| "—".to_owned())}</span>
                <span class="hist-que">
                    <IconoOjo />
                    <span>{que}</span>
                </span>
                <span class="chip">{t!("historial-secreto")}</span>
            </div>
        </li>
    }
}

/// Los secretos del paso `orden` del cambio `cambio` con valor anterior
/// guardado, cada uno con «Mostrar el valor anterior». La etiqueta es la del
/// campo en `campos` (o el parámetro).
fn valores_anteriores(
    cambio: i64,
    orden: i64,
    campos: &[CampoCambio],
    anteriores: &[(i64, String)],
) -> Option<impl IntoView + use<>> {
    let filas: Vec<_> = anteriores
        .iter()
        .filter(|(o, _)| *o == orden)
        .map(|(_, campo)| {
            let etiqueta = campos
                .iter()
                .find(|c| c.nombre == *campo)
                .map_or_else(|| campo.clone(), |c| c.etiqueta.clone());
            let objetivo = ObjetivoSecreto::Anterior {
                cambio,
                orden,
                campo: campo.clone(),
            };
            view! {
                <li>
                    <span class="hist-anterior-campo">{etiqueta.clone()}</span>
                    <Secreto objetivo nombre=etiqueta />
                </li>
            }
        })
        .collect();
    (!filas.is_empty()).then(|| {
        view! {
            <ul class="hist-anteriores" aria-label=t!("historial-anteriores")>
                {filas}
            </ul>
        }
    })
}

/// Lista desplegable de los pasos de un grupo.
fn lista_pasos(cambio: i64, pasos: Vec<PasoFila>, anteriores: &[(i64, String)]) -> impl IntoView {
    let filas = pasos
        .into_iter()
        .map(|p| {
            let anteriores = valores_anteriores(cambio, p.numero - 1, &p.campos, anteriores);
            let instancia = p
                .instancia
                .map(|i| view! { <span class="hist-instancia">{t!("historial-instancia-sep", i = i)}</span> });
            let error = p.error.map(|e| view! { <p class="hist-error">{e}</p> });
            view! {
                <li class="hist-paso">
                    <details>
                        <summary class="hist-paso-cabeza">
                            <span class="hist-paso-n cifras">
                                {t!("historial-paso-num", n = p.numero)}
                            </span>
                            <span>{p.etiqueta}</span>
                            <span class="mono hist-instancia">{p.funcion}</span>
                            {instancia}
                            <span class="hist-paso-estado">
                                <Insignia estado=p.estado.estado() texto=p.estado.etiqueta() />
                            </span>
                        </summary>
                        <div class="hist-paso-cuerpo">
                            <TablaDiff campos=p.campos />
                            {anteriores}
                            {error}
                        </div>
                    </details>
                </li>
            }
        })
        .collect_view();
    view! {
        <ol class="hist-pasos" aria-label=t!("historial-pasos-lista")>
            {filas}
        </ol>
    }
}

#[component]
fn FilaCambio(
    c: CambioFila,
    abiertos: RwSignal<BTreeSet<i64>>,
    ocupado: Signal<bool>,
    #[prop(into)] on_deshacer: Callback<i64>,
) -> impl IntoView {
    let id = c.id;
    let estado = match c.estado {
        EstadoDeCambio::Aplicando | EstadoDeCambio::Vigilando | EstadoDeCambio::Preparado => {
            Estado::Neutro
        }
        e => e.estado(),
    };
    let nota = c.error.clone().or_else(|| c.nota.clone());
    let nota_clase = if c.error.is_some() {
        "hist-nota hist-error"
    } else {
        "hist-nota"
    };
    let tiempos = {
        let mut t = vec![(t!("historial-creado"), fecha_hora(c.ts, true))];
        if c.updated != c.ts {
            t.push((t!("historial-ultimo-estado"), fecha_hora(c.updated, true)));
        }
        t.into_iter()
            .map(|(k, v)| {
                view! {
                    <div>
                        <dt>{k}</dt>
                        <dd class="cifras">{v}</dd>
                    </div>
                }
            })
            .collect_view()
    };
    let enlaces = (c.deshecho_por.is_some() || c.deshace.is_some()).then(|| {
        view! {
            <p class="hist-enlaces">
                {c.deshecho_por.map(|v| enlace_vinculo(t!("historial-deshecho-por"), v))}
                {c.deshace.map(|v| enlace_vinculo(t!("historial-deshace-a"), v))}
            </p>
        }
    });
    // Un grupo: su resumen y el número de pasos; el detalle, los pasos.
    let n_pasos = c.pasos.len();
    let que = if n_pasos > 0 {
        let titulo = c.titulo.clone().unwrap_or_else(|| c.funcion.clone());
        view! {
            <span>
                {titulo}
                <span class="hist-instancia">{t!("historial-pasos", n = n_pasos)}</span>
            </span>
        }
        .into_any()
    } else {
        view! { <span>{funcion_enlazada(c.funcion.clone(), c.ruta, c.instancia.clone())}</span> }
            .into_any()
    };
    let detalle = if n_pasos > 0 {
        lista_pasos(id, c.pasos, &c.anteriores).into_any()
    } else {
        let anteriores = valores_anteriores(id, 0, &c.campos, &c.anteriores);
        view! {
            <TablaDiff campos=c.campos />
            {anteriores}
        }
        .into_any()
    };
    let boton = c.puede_deshacer.then(|| {
        view! {
            <button
                class="boton-sutil hist-deshacer"
                type="button"
                disabled=move || ocupado.get()
                on:click=move |_| on_deshacer.run(id)
            >
                {t!("historial-deshacer")}
                <span class="solo-lector">{t!("historial-deshacer-lector", id = id)}</span>
            </button>
        }
    });
    view! {
        <li class="hist-entrada hist-cambio" id=format!("cambio-{id}")>
            <details
                prop:open=move || abiertos.with(|a| a.contains(&id))
                on:toggle=move |ev| {
                    let abierto = event_target::<web_sys::HtmlDetailsElement>(&ev).open();
                    abiertos
                        .update(|a| {
                            if abierto {
                                a.insert(id);
                            } else {
                                a.remove(&id);
                            }
                        });
                }
            >
                <summary class="hist-cabeza">
                    <span class="hist-hora cifras">{fecha_hora(c.ts, false)}</span>
                    <span class="hist-quien">{c.usuario.unwrap_or_else(|| "—".to_owned())}</span>
                    <span class="hist-que">
                        <span class="hist-id cifras">{format!("#{id}")}</span>
                        {que}
                    </span>
                    <span class="hist-diff">{c.resumen}</span>
                    <span class="hist-insignias">
                        <Insignia estado=c.riesgo.estado() texto=c.riesgo.etiqueta() />
                        <Insignia estado texto=c.estado.etiqueta() />
                    </span>
                </summary>
                <div class="hist-detalle">
                    {detalle}
                    <dl class="hist-tiempos">{tiempos}</dl>
                    {enlaces}
                </div>
            </details>
            {nota.map(|n| view! { <p class=nota_clase>{n}</p> })}
            {boton.map(|b| view! { <div class="hist-acciones">{b}</div> })}
        </li>
    }
}

/// Anterior y siguiente.
#[component]
fn Paginacion(pagina: u32, paginas: u32, hay_mas: bool, total: u32) -> impl IntoView {
    let reciente = (pagina > 1).then(|| {
        view! { <A href=format!("/historial?pagina={}", pagina - 1)>{t!("historial-mas-recientes")}</A> }
    });
    let antiguo = hay_mas.then(|| {
        view! { <A href=format!("/historial?pagina={}", pagina + 1)>{t!("historial-mas-antiguos")}</A> }
    });
    view! {
        <nav class="paginacion" aria-label=t!("historial-paginas")>
            <span class="paginacion-lado">{reciente}</span>
            <span class="pagina-sub cifras">
                {t!("historial-pagina-de", pagina = pagina, paginas = paginas, total = total)}
            </span>
            <span class="paginacion-lado paginacion-fin">{antiguo}</span>
        </nav>
    }
}

/// Página `/historial`.
#[component]
pub fn PaginaHistorial() -> impl IntoView {
    let consulta = use_query_map();
    let pagina = Memo::new(move |_| {
        consulta
            .with(|q| q.get("pagina").and_then(|p| p.parse::<u32>().ok()))
            .unwrap_or(1)
            .max(1)
    });
    let tick = RwSignal::new(0u32);
    let datos = Resource::new(move || (pagina.get(), tick.get()), |(p, _)| historial(p));
    let abiertos = RwSignal::new(BTreeSet::<i64>::new());
    // «Mostrar el valor anterior» pide la contraseña del panel con este
    // diálogo.
    let reauth = Reautenticacion::proveer();
    let preparado = RwSignal::new(None::<(i64, PreparadoVista)>);
    let preparando = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    // Hay cambios sin terminar: se vuelve a pedir la página cada 2 s.
    // (El recurso solo se lee en un efecto o dentro de la transición.)
    let hay_en_curso = RwSignal::new(false);
    Effect::new(move |_| {
        hay_en_curso.set(
            datos
                .read()
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .is_some_and(PaginaHistorial::hay_en_curso),
        );
    });
    let en_curso = Memo::new(move |_| hay_en_curso.get());
    Effect::new(move |_| {
        if !en_curso.get() {
            return;
        }
        let h = set_interval_with_handle(move || tick.update(|t| *t += 1), REFRESCO).ok();
        on_cleanup(move || {
            if let Some(h) = h {
                h.clear();
            }
        });
    });

    let deshacer_cambio = move |id: i64| {
        if preparando.get_untracked() {
            return;
        }
        error.set(None);
        preparando.set(true);
        spawn_local(async move {
            match deshacer(id).await {
                Ok(p) => preparado.set(Some((id, p))),
                Err(e) => error.set(Some(texto_error(&e))),
            }
            preparando.set(false);
        });
    };
    let refrescar = move || tick.update(|t| *t += 1);

    let dialogo = move || {
        preparado.get().map(|(id, p)| {
            view! {
                <DialogoCambio
                    preparado=p
                    deshace=id
                    on_cerrar=move |_| {
                        preparado.set(None);
                        refrescar();
                    }
                    on_terminado=move |_: EstadoDeCambio| refrescar()
                />
            }
        })
    };

    let contenido = move || {
        datos.get().map(|r| match r {
            Err(e) => {
                view! { <p class="error-formulario" role="alert">{texto_error(&e)}</p> }.into_any()
            }
            Ok(p) if p.total == 0 => view! {
                <section class="tarjeta hist-vacio">
                    <p>{t!("historial-vacio")}</p>
                    <p class="pagina-sub">{t!("historial-vacio-sub")}</p>
                </section>
            }
            .into_any(),
            Ok(p) => {
                let paginacion = view! {
                    <Paginacion
                        pagina=p.pagina
                        paginas=p.paginas()
                        hay_mas=p.hay_mas
                        total=p.total
                    />
                };
                let lista = if p.entradas.is_empty() {
                    view! {
                        <p class="pagina-sub">
                            {t!("historial-pagina-vacia")}
                            " "
                            <A href="/historial">{t!("historial-volver-recientes")}</A>
                        </p>
                    }
                    .into_any()
                } else {
                    let filas = p
                        .entradas
                        .into_iter()
                        .map(|e| match e {
                            EntradaVista::Cambio(c) => view! {
                                <FilaCambio
                                    c
                                    abiertos
                                    ocupado=preparando.into()
                                    on_deshacer=deshacer_cambio
                                />
                            }
                            .into_any(),
                            EntradaVista::Revelado(r) => view! { <FilaRevelado r /> }.into_any(),
                        })
                        .collect_view();
                    view! { <ol class="hist-lista">{filas}</ol> }.into_any()
                };
                view! { {lista} {paginacion} }.into_any()
            }
        })
    };

    view! {
        <header class="pagina-cabecera">
            <div>
                <h1>{t!("historial-titulo")}</h1>
                <p class="pagina-sub">{t!("historial-sub")}</p>
            </div>
        </header>
        <p class="error-formulario" role="alert">
            {move || error.get()}
        </p>
        <Transition fallback=|| view! { <p class="pagina-sub">{t!("comun-cargando")}</p> }>
            {contenido}
        </Transition>
        {dialogo}
        {move || reauth.abierto().then(|| view! { <DialogoReautenticar /> })}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::explorador::Valor;

    fn campo(etiqueta: &str) -> CampoCambio {
        CampoCambio {
            nombre: etiqueta.to_lowercase(),
            etiqueta: etiqueta.into(),
            antes: Valor::Ausente,
            despues: Valor::Secreto,
        }
    }

    #[test]
    fn resumen_corta_a_tres_campos() {
        assert_eq!(resumen(&[]), "Sin campos");
        assert_eq!(resumen(&[campo("Activado")]), "1 campo: Activado");
        let cuatro: Vec<_> = ["A", "B", "C", "D"].into_iter().map(campo).collect();
        assert_eq!(resumen(&cuatro), "4 campos: A, B, C, …");
        assert_eq!(resumen(&cuatro[..3]), "3 campos: A, B, C");
    }

    #[test]
    fn fecha_en_hora_peninsular() {
        // 2026-10-09 12:05:09 UTC es 14:05:09 en horario de verano.
        assert_eq!(fecha_hora(1_791_547_509, true), "09/10/2026 14:05:09");
        assert_eq!(fecha_hora(1_791_547_509, false), "09/10/2026 14:05");
    }

    #[test]
    fn paginas_y_cambios_en_curso() {
        let p = |total| PaginaHistorial {
            entradas: Vec::new(),
            pagina: 1,
            total,
            hay_mas: false,
        };
        assert_eq!(
            [0, 1, 25, 26, 50, 51].map(|t| p(t).paginas()),
            [1, 1, 1, 2, 2, 3]
        );
        assert!(en_curso(EstadoDeCambio::Vigilando));
        assert!(en_curso(EstadoDeCambio::Aplicando));
        assert!(!en_curso(EstadoDeCambio::Aplicado));
        assert!(!p(1).hay_en_curso());
    }
}
