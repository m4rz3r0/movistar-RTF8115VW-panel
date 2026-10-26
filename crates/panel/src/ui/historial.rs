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
//! Los revelados no llevan el valor: solo quién, cuándo y qué campo.

#[cfg(feature = "ssr")]
pub(crate) mod servidor;

use crate::t;
use crate::ui::{
    charts::{formato_fecha, hora_local},
    comun::{EstadoVisual, Insignia},
    explorador::{
        CampoCambio, DialogoCambio, EstadoDeCambio, PreparadoVista, RiesgoCambio, TablaDiff, Valor,
        texto_error,
    },
    panel_ajustes::{DialogoReautenticar, Reautenticacion, repetible},
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
    /// Por qué falló, en lenguaje claro y ya traducido.
    pub error: Option<String>,
    /// Lo técnico del fallo o, sin él, de la nota (el texto del router),
    /// para los detalles técnicos.
    #[serde(default)]
    pub error_tecnico: Option<String>,
    /// Cambio que lo deshizo.
    pub deshecho_por: Option<Vinculo>,
    /// Cambio que este deshace.
    pub deshace: Option<Vinculo>,
    /// El gestor aceptaría deshacerlo ahora.
    pub puede_deshacer: bool,
    /// Por qué no se puede deshacer un cambio aplicado (solo cambió una
    /// clave que el router no deja leer), ya traducido.
    #[serde(default)]
    pub sin_deshacer: Option<String>,
    /// Resumen humano del grupo, si lo tiene.
    pub titulo: Option<String>,
    /// Pasos del grupo (vacío si es de un solo paso).
    pub pasos: Vec<PasoFila>,
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

    pub fn estado(self) -> EstadoVisual {
        match self {
            EstadoPasoVista::Aplicado => EstadoVisual::Emite,
            EstadoPasoVista::Revertido => EstadoVisual::Atencion,
            EstadoPasoVista::Fallido => EstadoVisual::Peligro,
            _ => EstadoVisual::Proceso,
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
    /// Por qué falló, en lenguaje claro y ya traducido.
    pub error: Option<String>,
    /// Lo técnico del fallo, para los detalles técnicos.
    #[serde(default)]
    pub error_tecnico: Option<String>,
}

/// Un secreto revelado (sin su valor).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReveladoFila {
    pub id: i64,
    pub ts: i64,
    /// `None` si el usuario ya no existe.
    pub usuario: Option<String>,
    pub funcion: String,
    pub ruta: Option<String>,
    pub campo: String,
    pub instancia: Option<String>,
    /// El nombre legible del campo, en el idioma de la página, si el
    /// catálogo de funciones o el de ajustes lo trae.
    #[serde(default)]
    pub etiqueta: Option<String>,
}

/// Un secreto de un cambio con su valor anterior y el nuevo, para el detalle
/// desplegado del cambio (se pide al desplegarlo). Su `Debug` no enseña los
/// valores.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretoDelCambio {
    /// Orden del paso (desde 0).
    pub orden: i64,
    /// Parámetro del paso.
    pub nombre: String,
    pub antes: Option<String>,
    pub despues: Option<String>,
    /// El router no deja leerlo: no hay «antes» que enseñar.
    #[serde(default)]
    pub tapado: bool,
}

impl std::fmt::Debug for SecretoDelCambio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretoDelCambio")
            .field("orden", &self.orden)
            .field("nombre", &self.nombre)
            .finish_non_exhaustive()
    }
}

/// Entrada del historial.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntradaVista {
    /// En una caja: es mucho más grande que un revelado.
    Cambio(Box<CambioFila>),
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
    // Lleva los datos sensibles de los cambios en claro.
    crate::ui::explorador::servidor::sin_guardar();
    crate::auth::require_user().await?;
    let state = crate::ui::explorador::servidor::app_state()?;
    servidor::pagina(&state, pagina).await
}

/// Prepara el cambio inverso de `change_id` (solo uno aplicado que nadie
/// haya deshecho). Se confirma con `confirmar_cambio`, como cualquier otro.
/// El de un alta o una baja de usuarios del router exige reautenticación.
#[server(endpoint = "deshacer", input = Json)]
pub async fn deshacer(change_id: i64) -> Result<PreparadoVista, ServerFnError> {
    // Lleva los datos sensibles de los cambios en claro.
    crate::ui::explorador::servidor::sin_guardar();
    crate::auth::require_user().await?;
    let state = crate::ui::explorador::servidor::app_state()?;
    servidor::exigir_reauth_si_usuarios(&state, change_id).await?;
    let v = servidor::deshacer(&state, change_id).await?;
    crate::ui::explorador::servidor::ligar_a_la_sesion(&v.token).await?;
    Ok(v)
}

/// Los secretos del cambio `cambio` con su valor anterior y el nuevo, para
/// enseñarlos censurados con el ojo en su detalle desplegado. Solo descifra
/// ese cambio. Exige sesión; la respuesta va con `no-store`. Vacío si el
/// cambio no existe o no cambió ningún secreto.
#[server(endpoint = "secretos_del_cambio", input = Json)]
pub async fn secretos_del_cambio(cambio: i64) -> Result<Vec<SecretoDelCambio>, ServerFnError> {
    crate::ui::explorador::servidor::sin_guardar();
    crate::auth::require_user().await?;
    let state = crate::ui::explorador::servidor::app_state()?;
    servidor::secretos_del_cambio(&state, cambio).await
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

/// Un secreto mostrado (filas de la 0.2.x: la 0.3 ya no los apunta). La
/// función y el parámetro, solo en los detalles técnicos.
#[component]
fn FilaRevelado(r: ReveladoFila) -> impl IntoView {
    let instancia = r.instancia.map(|i| t!("historial-instancia-par", i = i));
    let que = match r.etiqueta {
        Some(campo) => t!("historial-revelado-campo", campo = campo),
        None => t!("historial-revelado-clave"),
    };
    view! {
        <li class="hist-entrada hist-revelado" id=format!("revelado-{}", r.id)>
            <div class="hist-cabeza">
                <span class="hist-hora cifras">{fecha_hora(r.ts, false)}</span>
                <span class="hist-quien">{r.usuario.unwrap_or_else(|| "—".to_owned())}</span>
                <span class="hist-que">
                    <IconoOjo />
                    <span>{que} {instancia}</span>
                </span>
                <span class="chip">{t!("historial-secreto")}</span>
            </div>
            <details class="detalles-tecnicos">
                <summary>{t!("historial-tecnico")}</summary>
                <p>
                    <code class="mono">{r.campo}</code>
                    " "
                    {t!("historial-revelado-de")}
                    " "
                    {funcion_enlazada(r.funcion, r.ruta, None)}
                </p>
            </details>
        </li>
    }
}

/// Los campos `campos` del paso `orden` con el valor de sus secretos, si ya
/// llegaron (`secretos`), para enseñarlos censurados con el ojo.
fn con_secretos(
    campos: &[CampoCambio],
    orden: i64,
    secretos: Option<&[SecretoDelCambio]>,
) -> Vec<CampoCambio> {
    let buscar = |nombre: &str| {
        secretos
            .into_iter()
            .flatten()
            .find(|s| s.orden == orden && s.nombre == nombre)
    };
    campos
        .iter()
        .map(|c| {
            let mut c = c.clone();
            if let Some(s) = buscar(&c.nombre) {
                match (&c.antes, &s.antes) {
                    (Valor::Secreto, Some(v)) => c.antes = Valor::Clave(v.clone()),
                    // Lo que había no se sabe: el router no deja leerlo.
                    (Valor::Secreto, None) if s.tapado => {
                        c.antes = Valor::Texto(t!("historial-clave-sin-leer"));
                    }
                    _ => {}
                }
                if let (Valor::Secreto, Some(v)) = (&c.despues, &s.despues) {
                    c.despues = Valor::Clave(v.clone());
                }
            }
            c
        })
        .collect()
}

/// Los nombres técnicos de un paso (la función, su instancia y cada
/// parámetro con su etiqueta), solo dentro de los detalles técnicos.
fn tecnico_de(
    funcion: String,
    ruta: Option<String>,
    instancia: Option<String>,
    campos: &[CampoCambio],
) -> impl IntoView + use<> {
    let parametros = campos
        .iter()
        .map(|c| {
            view! {
                <li>
                    {c.etiqueta.clone()}
                    ": "
                    <code class="mono">{c.nombre.clone()}</code>
                </li>
            }
        })
        .collect_view();
    view! {
        <p>{funcion_enlazada(funcion, ruta, instancia)}</p>
        <ul class="hist-tecnico">{parametros}</ul>
    }
}

/// Lista desplegable de los pasos de un grupo, con el valor de sus secretos
/// cuando llegan (`secretos`).
fn lista_pasos(
    pasos: Vec<PasoFila>,
    secretos: Signal<Option<Vec<SecretoDelCambio>>>,
) -> impl IntoView {
    let filas = pasos
        .into_iter()
        .map(|p| {
            let instancia = p
                .instancia
                .clone()
                .map(|i| view! { <span class="hist-instancia">{t!("historial-instancia-sep", i = i)}</span> });
            let error = p.error.map(|e| view! { <p class="hist-error">{e}</p> });
            let orden = p.numero - 1;
            let campos = p.campos.clone();
            let tabla = move || {
                let campos = secretos.with(|s| con_secretos(&campos, orden, s.as_deref()));
                view! { <TablaDiff campos sin_nombres=true /> }
            };
            let tecnico = tecnico_de(p.funcion.clone(), None, p.instancia.clone(), &p.campos);
            let error_tecnico = p
                .error_tecnico
                .map(|e| view! { <p class="mono hist-error-tecnico">{e}</p> });
            view! {
                <li class="hist-paso">
                    <details>
                        <summary class="hist-paso-cabeza">
                            <span class="hist-paso-n cifras">
                                {t!("historial-paso-num", n = p.numero)}
                            </span>
                            <span>{p.etiqueta}</span>
                            {instancia}
                            <span class="hist-paso-estado">
                                <Insignia estado=p.estado.estado() texto=super::mayuscula_inicial(&p.estado.etiqueta()) />
                            </span>
                        </summary>
                        <div class="hist-paso-cuerpo">
                            {tabla}
                            <details class="detalles-tecnicos">
                                <summary>{t!("historial-tecnico")}</summary>
                                {tecnico}
                                {error_tecnico}
                            </details>
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
            EstadoVisual::Proceso
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
    // Un grupo o un cambio hecho desde Ajustes: su título (y en un grupo, el
    // número de pasos); uno de la Biblioteca, su función. El detalle, los
    // pasos o el diff, con los nombres técnicos plegados aparte.
    let n_pasos = c.pasos.len();
    // Sin título (un cambio de la Biblioteca), un texto genérico: el nombre
    // de la función solo va en los detalles técnicos.
    let titulo = c
        .titulo
        .clone()
        .unwrap_or_else(|| t!("historial-cambio-router"));
    let que = if n_pasos > 0 {
        view! {
            <span>
                {titulo}
                <span class="hist-instancia">{t!("historial-pasos", n = n_pasos)}</span>
            </span>
        }
        .into_any()
    } else {
        view! { <span>{titulo}</span> }.into_any()
    };
    // Los valores de sus secretos, al desplegarlo (solo si cambió alguno).
    let con_secreto = c
        .campos
        .iter()
        .any(|x| x.antes == Valor::Secreto || x.despues == Valor::Secreto);
    // Solo esta fila: abrir o cerrar otra no vuelve a pedir nada ni repinta.
    let abierta = Memo::new(move |_| abiertos.with(|a| a.contains(&id)));
    let pedidos = LocalResource::new(move || {
        let abierto = con_secreto && abierta.get();
        async move {
            if abierto {
                secretos_del_cambio(id).await.ok()
            } else {
                None
            }
        }
    });
    // Solo se lee al desplegarlo (nunca al pintar en el servidor, donde
    // está plegado: si no, la página esperaría a un recurso que allí no
    // se carga).
    let secretos = Memo::new(move |_| {
        if abierta.get() {
            pedidos.get().flatten()
        } else {
            None
        }
    });
    let secretos = Signal::from(secretos);
    let detalle = if n_pasos > 0 {
        // Lo técnico del grupo entero (p. ej. lo que dijo el router en una
        // escritura sin respuesta); el de cada paso va en el suyo.
        let tecnico = c.error_tecnico.clone().map(|e| {
            view! {
                <details class="detalles-tecnicos">
                    <summary>{t!("historial-tecnico")}</summary>
                    <p class="mono hist-error-tecnico">{e}</p>
                </details>
            }
        });
        view! {
            {lista_pasos(c.pasos, secretos)}
            {tecnico}
        }
        .into_any()
    } else {
        let campos = c.campos.clone();
        let tabla = move || {
            let campos = secretos.with(|s| con_secretos(&campos, 0, s.as_deref()));
            view! { <TablaDiff campos sin_nombres=true /> }
        };
        let tecnico = tecnico_de(
            c.funcion.clone(),
            c.ruta.clone(),
            c.instancia.clone(),
            &c.campos,
        );
        let error_tecnico = c
            .error_tecnico
            .clone()
            .map(|e| view! { <p class="mono hist-error-tecnico">{e}</p> });
        view! {
            {tabla}
            <details class="detalles-tecnicos">
                <summary>{t!("historial-tecnico")}</summary>
                {tecnico}
                {error_tecnico}
            </details>
        }
        .into_any()
    };
    // Por qué no se puede deshacer (solo cambió una clave que el router no
    // deja leer).
    let sin_deshacer = c
        .sin_deshacer
        .clone()
        .map(|m| view! { <p class="hist-nota hist-sin-deshacer">{m}</p> });
    let boton = c.puede_deshacer.then(|| {
        view! {
            <button
                class="b-sec hist-deshacer"
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
                        <Insignia estado texto=super::mayuscula_inicial(&c.estado.etiqueta()) />
                    </span>
                </summary>
                <div class="hist-detalle">
                    {detalle}
                    <dl class="hist-tiempos">{tiempos}</dl>
                    {enlaces}
                </div>
            </details>
            {nota.map(|n| view! { <p class=nota_clase>{n}</p> })}
            {sin_deshacer}
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

/// Lo que la página enseña alrededor de las entradas: solo si cambia esto se
/// vuelve a pintar todo; las filas que no cambian se quedan (ver
/// [`ListaHistorial`]).
#[derive(Clone, Debug, PartialEq)]
enum Forma {
    Error(String),
    Vacio,
    /// Con entradas o sin ellas en esta página. La paginación (que cambia
    /// con cada entrada nueva) va aparte.
    Lista {
        sin_entradas: bool,
    },
}

/// Clave de una fila para `<For>`: todo lo que la fila enseña. Con el
/// refresco de 2 s (un cambio en curso), las filas iguales no se recrean, y
/// así no se pierde un secreto mostrado ni la operación que espera a la
/// reautenticación.
fn clave_fila(e: &EntradaVista) -> String {
    format!("{e:?}")
}

/// Las entradas de la página del historial, en filas que sobreviven al
/// refresco mientras no cambien.
#[component]
fn ListaHistorial(
    datos: Resource<Result<PaginaHistorial, ServerFnError>>,
    abiertos: RwSignal<BTreeSet<i64>>,
    ocupado: Signal<bool>,
    #[prop(into)] on_deshacer: Callback<i64>,
) -> impl IntoView {
    let forma = Memo::new(move |_| {
        datos.get().map(|r| match r {
            Err(e) => Forma::Error(texto_error(&e)),
            Ok(p) if p.total == 0 => Forma::Vacio,
            Ok(p) => Forma::Lista {
                sin_entradas: p.entradas.is_empty(),
            },
        })
    });
    let paginacion = Memo::new(move |_| {
        datos
            .get()
            .and_then(Result::ok)
            .map(|p| (p.pagina, p.paginas(), p.hay_mas, p.total))
    });
    let entradas = move || {
        datos
            .get()
            .and_then(Result::ok)
            .map(|p| p.entradas)
            .unwrap_or_default()
    };
    move || {
        forma.get().map(|f| match f {
            Forma::Error(e) => {
                view! { <p class="error-formulario" role="alert">{e}</p> }.into_any()
            }
            Forma::Vacio => view! {
                <section class="tarjeta hist-vacio">
                    <p>{t!("historial-vacio")}</p>
                    <p class="pagina-sub">{t!("historial-vacio-sub")}</p>
                </section>
            }
            .into_any(),
            Forma::Lista { sin_entradas } => {
                let lista = if sin_entradas {
                    view! {
                        <p class="pagina-sub">
                            {t!("historial-pagina-vacia")}
                            " "
                            <A href="/historial">{t!("historial-volver-recientes")}</A>
                        </p>
                    }
                    .into_any()
                } else {
                    view! {
                        <ol class="hist-lista">
                            <For each=entradas key=clave_fila let:e>
                                {match e {
                                    EntradaVista::Cambio(c) => view! {
                                        <FilaCambio c=*c abiertos ocupado on_deshacer />
                                    }
                                    .into_any(),
                                    EntradaVista::Revelado(r) => view! { <FilaRevelado r /> }.into_any(),
                                }}
                            </For>
                        </ol>
                    }
                    .into_any()
                };
                view! {
                    {lista}
                    {move || {
                        paginacion.get().map(|(pagina, paginas, hay_mas, total)| {
                            view! { <Paginacion pagina paginas hay_mas total /> }
                        })
                    }}
                }
                .into_any()
            }
        })
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

    // Deshacer un alta o una baja de usuarios del router pide la contraseña
    // del panel y se repite al darla.
    let (reauth, dialogo_reauth) = Reautenticacion::de_la_pagina();
    let deshacer_cambio = move |id: i64| {
        if preparando.get_untracked() {
            return;
        }
        let pedir = repetible(move |yo| {
            error.set(None);
            preparando.set(true);
            spawn_local(async move {
                match deshacer(id).await {
                    Ok(p) => preparado.set(Some((id, p))),
                    Err(e) => reauth.tratar(&e, error, yo),
                }
                preparando.set(false);
            });
        });
        pedir.run(());
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
        {move || {
            (dialogo_reauth && reauth.abierto()).then(|| view! { <DialogoReautenticar /> })
        }}
        <Transition fallback=|| view! { <p class="pagina-sub">{t!("comun-cargando")}</p> }>
            <ListaHistorial
                datos
                abiertos
                ocupado=preparando.into()
                on_deshacer=deshacer_cambio
            />
        </Transition>
        {dialogo}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
