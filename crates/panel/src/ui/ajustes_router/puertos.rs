//! «Puertos abiertos» ([`Puertos`]): las reglas de apertura de puertos en
//! una lista (apilada en pantallas estrechas) con su nombre, el puerto de
//! internet y su protocolo, el dispositivo al que van, si está activa,
//! «Editar» y «Borrar». Una regla cuyo dispositivo de destino ya no está en
//! la red lleva un aviso ([`destino_inexistente`]).
//!
//! «Abrir un puerto» es un formulario corto: nombre, protocolo (también
//! «TCP y UDP», que son dos reglas en un solo cambio), el puerto o el rango
//! de internet, el dispositivo (de la lista, u otra IP) y, plegado, el
//! puerto del dispositivo si no es el mismo. Antes de enviar, la página
//! comprueba la regla como el servidor ([`validar_regla`]) y dice el error
//! junto al campo. Todo se guarda con el diálogo de grupo.

use super::Foco;
use super::campo::{Campo, DESCONOCIDO, validar};
use super::dispositivos::{aviso_error, nombre};
use super::tarjeta::{ayuda, cabecera, detalles_tecnicos, dom, enfocar};
use crate::ajustes_router::{
    AMBOS, AjusteVista, CampoVista, DispositivoVista, EstadoLectura, GrupoVista, IDENTIFICAN_REGLA,
    InstanciaVista, OpcionVista, PeticionAjuste, Protocolo, ReglaPuertos, destino_inexistente,
    leer_dispositivos, leer_red_casa, leer_reservas, preparar_ajuste, regla_de, validar_regla,
    valor_de,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::grupo::DialogoGrupo;
use crate::ui::i18n::traducir;
use crate::ui::{Estado, Insignia};
use ajustes::{TipoCampo, ValorLegible};
use leptos::{ev::SubmitEvent, prelude::*, task::spawn_local};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Prefijo de los textos propios del componente.
const CLAVE: &str = "ajuste-puertos-reglas";

/// Valor del `<select>` de dispositivos que pide escribir la IP.
const OTRA_IP: &str = "otra";

/// Las IP de las IP fijas.
type IpFijas = Vec<String>;

/// Lo que lee de la red de casa la tarjeta de Puertos: los dispositivos
/// (para elegir el destino y ponerle nombre), las IP fijas (para saber si
/// el destino sigue en la red) y la red (para validar antes de enviar). Cada
/// lectura es su propio recurso: ninguna función del servidor va detrás de
/// otra dentro de un mismo recurso.
#[derive(Clone, Copy)]
pub(super) struct DatosPuertos {
    dispositivos: Resource<Result<Vec<DispositivoVista>, ServerFnError>>,
    reservas: Resource<Result<IpFijas, ServerFnError>>,
    red: Resource<Result<Option<(Ipv4Addr, Ipv4Addr)>, ServerFnError>>,
}

/// Las tres lecturas; `None` en la que falló.
#[derive(Clone, Debug, Default)]
struct RedPuertos {
    dispositivos: Option<Vec<DispositivoVista>>,
    reservas: Option<IpFijas>,
    lan: Option<(Ipv4Addr, Ipv4Addr)>,
}

impl RedPuertos {
    /// ¿Hay que avisar de que el destino `ip` ya no está en la red? Solo si
    /// se pudieron leer los dispositivos y las IP fijas: sin ellos no se sabe.
    fn destino_perdido(&self, ip: &str) -> bool {
        match (&self.dispositivos, &self.reservas) {
            (Some(d), Some(r)) => {
                let fijas: Vec<(String, String, String)> = r
                    .iter()
                    .map(|ip| (String::new(), String::new(), ip.clone()))
                    .collect();
                destino_inexistente(ip, d, &fijas)
            }
            _ => false,
        }
    }

    /// Nombre del dispositivo con la IP `ip`, si se conoce.
    fn nombre_de(&self, ip: &str) -> Option<String> {
        self.dispositivos
            .as_ref()?
            .iter()
            .find(|d| d.ip.trim() == ip.trim())
            .map(|d| nombre(&d.nombre))
    }
}

impl DatosPuertos {
    /// Se crea con cada lectura del tema: tras aplicar un cambio se vuelve
    /// a leer.
    pub(super) fn nuevo() -> Self {
        DatosPuertos {
            dispositivos: Resource::new(|| (), |()| leer_dispositivos()),
            reservas: Resource::new(|| (), |()| leer_reservas()),
            red: Resource::new(|| (), |()| leer_red_casa()),
        }
    }

    async fn leer(self) -> RedPuertos {
        let lan = self.red.await.ok().flatten();
        let reservas = self.reservas.await.ok();
        let dispositivos = self.dispositivos.await.ok();
        RedPuertos {
            dispositivos,
            reservas,
            lan,
        }
    }
}

/// Puerto o rango escrito por la persona: `8080` o `27015-27030` (con
/// guion o raya, con o sin espacios). `None` si no son números de 0 a
/// 65535 (el 0 y el orden los dice [`validar_regla`]).
fn leer_puertos(texto: &str) -> Option<(u16, u16)> {
    let texto = texto.trim();
    let (desde, hasta) = match texto.split_once(['-', '–']) {
        Some((d, h)) => (d.trim(), h.trim()),
        None => (texto, texto),
    };
    Some((desde.parse().ok()?, hasta.parse().ok()?))
}

/// Puerto o rango para leerlo: `8080` o `27015–27030`.
fn rango(r: (u16, u16)) -> String {
    if r.0 == r.1 {
        r.0.to_string()
    } else {
        format!("{}–{}", r.0, r.1)
    }
}

/// Puerto o rango para editarlo: `8080` o `27015-27030`.
fn rango_editable(r: (u16, u16)) -> String {
    if r.0 == r.1 {
        r.0.to_string()
    } else {
        format!("{}-{}", r.0, r.1)
    }
}

/// Puertos del dispositivo: los mismos que los de internet o, con
/// `primero`, desde él y del mismo tamaño. `None` si se sale de 65535.
fn puertos_internos(externo: (u16, u16), primero: Option<u16>) -> Option<(u16, u16)> {
    match primero {
        None => Some(externo),
        Some(p) => Some((p, p.checked_add(externo.1.saturating_sub(externo.0))?)),
    }
}

/// Campo del formulario que no es del catálogo (el puerto de internet o el
/// del dispositivo), con sus textos propios.
fn campo_propio(id: &str, tipo: TipoCampo, opcional: bool) -> CampoVista {
    CampoVista {
        id: id.into(),
        clave: format!("{CLAVE}-{id}"),
        tipo,
        valor: ValorLegible::Texto(String::new()),
        opciones: Vec::new(),
        min: None,
        max: None,
        ascii: false,
        opcional,
        solo_lectura: None,
        tapado: false,
        recomendado: None,
        motivo_recomendado: None,
        requiere: None,
        sin_eleccion: false,
    }
}

/// Qué se está editando: una regla nueva o la de ese id.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Modo {
    Nueva,
    Editar(String),
}

/// Valores con que empieza el formulario.
#[derive(Clone, Debug, Default)]
struct Inicial {
    nombre: String,
    protocolo: String,
    puerto: String,
    destino: String,
    interno: String,
}

impl Inicial {
    /// De la regla que se edita (`r`, leída de `i`); sin ella, una nueva.
    /// Un protocolo que el panel no conoce empieza «sin elegir»
    /// ([`DESCONOCIDO`]): hay que elegir TCP o UDP para guardarla.
    fn de(r: Option<&ReglaPuertos>, i: Option<&InstanciaVista>) -> Inicial {
        let Some(r) = r else {
            return Inicial {
                protocolo: "tcp".into(),
                ..Inicial::default()
            };
        };
        let protocolo = i
            .map(|i| valor_de(&i.campos, "protocolo"))
            .filter(|p| {
                matches!(
                    Protocolo::de_clave(p),
                    Some(Protocolo::Tcp | Protocolo::Udp)
                )
            })
            .unwrap_or_else(|| DESCONOCIDO.to_owned());
        Inicial {
            nombre: r.nombre.clone(),
            protocolo,
            puerto: rango_editable(r.externo),
            destino: r.ip.to_string(),
            interno: if r.interno == r.externo {
                String::new()
            } else {
                r.interno.0.to_string()
            },
        }
    }
}

/// Formulario de una regla, nueva o existente (`editando`: la regla y sus
/// valores tal como los leyó la página). Llama a `preparado` con el resumen
/// del cambio y a `cancelar` al dejarlo. Su error general va a `error`, la
/// única región de alerta de la tarjeta.
#[component]
fn FormRegla(
    plantilla: Vec<CampoVista>,
    editando: Option<InstanciaVista>,
    existentes: Vec<ReglaPuertos>,
    red: RedPuertos,
    error: RwSignal<Option<String>>,
    dom: String,
    preparado: Callback<GrupoVista>,
    cancelar: Callback<()>,
) -> impl IntoView {
    let regla = editando.as_ref().and_then(regla_de);
    let instancia = editando.as_ref().map(|i| i.id.clone());
    let nueva = instancia.is_none();
    let ini = Inicial::de(regla.as_ref(), editando.as_ref());
    let plantilla_de = |id: &str| plantilla.iter().find(|c| c.id == id).cloned();
    let Some((c_nombre, mut c_protocolo, c_destino)) = plantilla_de("nombre")
        .zip(plantilla_de("protocolo"))
        .zip(plantilla_de("destino"))
        .map(|((n, p), d)| (n, p, d))
    else {
        return ().into_any();
    };
    // Al editar, el protocolo con el valor del router: si no lo conoce, el
    // control lo enseña como «valor no reconocido».
    if let Some(v) = editando
        .as_ref()
        .and_then(|i| i.campos.iter().find(|c| c.id == "protocolo"))
        .map(|c| c.valor.clone())
    {
        c_protocolo.valor = v;
    }
    // «TCP y UDP» solo al crear: el router guarda un protocolo por regla.
    if nueva {
        c_protocolo.opciones.push(OpcionVista {
            clave: AMBOS.into(),
            recomendada: false,
            aviso: None,
        });
    }
    let c_puerto = campo_propio("puerto", TipoCampo::Texto, false);
    let c_interno = CampoVista {
        min: Some(1),
        max: Some(65535),
        ..campo_propio("interno", TipoCampo::Numero, true)
    };

    // Los dispositivos para elegir: los que tienen IPv4, sin repetir IP.
    let mut elegibles: Vec<(String, String)> = Vec::new();
    for d in red.dispositivos.iter().flatten() {
        let ip = d.ip.trim();
        if ip.parse::<Ipv4Addr>().is_ok() && !elegibles.iter().any(|(i, _)| i == ip) {
            elegibles.push((ip.to_owned(), nombre(&d.nombre)));
        }
    }
    let en_lista = elegibles.iter().any(|(ip, _)| *ip == ini.destino);
    let eleccion_inicial = if ini.destino.is_empty() || en_lista {
        ini.destino.clone()
    } else {
        OTRA_IP.to_owned()
    };

    let nombre_v = RwSignal::new(ini.nombre.clone());
    let protocolo_v = RwSignal::new(ini.protocolo.clone());
    let puerto_v = RwSignal::new(ini.puerto.clone());
    let eleccion = RwSignal::new(eleccion_inicial.clone());
    let otra_ip_v = RwSignal::new(if en_lista {
        String::new()
    } else {
        ini.destino.clone()
    });
    let interno_v = RwSignal::new(ini.interno.clone());
    let [
        e_nombre,
        e_protocolo,
        e_puerto,
        e_dispositivo,
        e_otra_ip,
        e_interno,
    ] = std::array::from_fn(|_| RwSignal::new(None::<String>));
    let preparando = RwSignal::new(false);
    let id = {
        let dom = dom.clone();
        move |campo: &str| format!("{dom}-{campo}")
    };
    let control = {
        let id = id.clone();
        move |campo: &str| format!("{}-control", id(campo))
    };
    // El foco, al primer campo al abrirse.
    {
        let primero = control("nombre");
        request_animation_frame(move || enfocar(&primero));
    }
    let titulo = if nueva {
        t!("ajuste-puertos-reglas-nueva")
    } else {
        t!("ajuste-puertos-reglas-cambiar")
    };
    let campos = StoredValue::new((c_nombre.clone(), c_protocolo.clone(), c_destino.clone()));
    let c_interno_v = StoredValue::new(c_interno.clone());
    let editando = StoredValue::new(editando);
    let lan = red.lan;

    // El plegable del puerto del dispositivo se abre si su campo tiene un
    // error: el error y el foco no pueden quedar ocultos.
    let id_interno = id("plegable-interno");
    let abrir_interno = {
        let id_interno = id_interno.clone();
        move || {
            if let Some(e) = document().get_element_by_id(&id_interno) {
                let _ = e.set_attribute("open", "");
            }
        }
    };
    let guardar = {
        let control = control.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if preparando.get_untracked() {
                return;
            }
            error.set(None);
            for e in [
                e_nombre,
                e_protocolo,
                e_puerto,
                e_dispositivo,
                e_otra_ip,
                e_interno,
            ] {
                e.set(None);
            }
            let (c_nombre, c_protocolo, c_destino) = campos.get_value();
            let otra = eleccion.get_untracked() == OTRA_IP;
            // El primer campo con error recibe el foco.
            let mal = std::cell::Cell::new(None::<&'static str>);
            let marcar = |e: RwSignal<Option<String>>, campo: &'static str, m: String| {
                e.set(Some(m));
                if mal.get().is_none() {
                    mal.set(Some(campo));
                }
            };
            let nombre = nombre_v.get_untracked().trim().to_owned();
            if let Err(m) = validar(&c_nombre, &nombre) {
                marcar(e_nombre, "nombre", m);
            }
            let protocolo = protocolo_v.get_untracked();
            if let Err(m) = validar(&c_protocolo, &protocolo) {
                marcar(e_protocolo, "protocolo", m);
            }
            let externo = leer_puertos(&puerto_v.get_untracked());
            if externo.is_none() {
                marcar(e_puerto, "puerto", t!("ajuste-puertos-reglas-error-puerto"));
            }
            let destino = if otra {
                let ip = otra_ip_v.get_untracked().trim().to_owned();
                if let Err(m) = validar(&c_destino, &ip) {
                    marcar(e_otra_ip, "destino", m);
                }
                ip
            } else {
                let ip = eleccion.get_untracked();
                if ip.is_empty() {
                    marcar(
                        e_dispositivo,
                        "dispositivo",
                        t!("ajuste-puertos-reglas-error-dispositivo"),
                    );
                }
                ip
            };
            let interno_texto = interno_v.get_untracked().trim().to_owned();
            let primero_interno = if interno_texto.is_empty() {
                None
            } else {
                match validar(&c_interno_v.get_value(), &interno_texto) {
                    Ok(()) => interno_texto.parse::<u16>().ok(),
                    Err(m) => {
                        marcar(e_interno, "interno", m);
                        abrir_interno();
                        None
                    }
                }
            };
            let interno = externo.and_then(|ext| {
                let int = puertos_internos(ext, primero_interno);
                if int.is_none() {
                    marcar(
                        e_interno,
                        "interno",
                        t!("ajuste-puertos-reglas-error-distintos"),
                    );
                    abrir_interno();
                }
                int
            });
            // Lo mismo que comprobará el servidor, junto al campo.
            if mal.get().is_none()
                && let (Some(ext), Some(int), Ok(ip), Some(p)) = (
                    externo,
                    interno,
                    destino.parse::<Ipv4Addr>(),
                    Protocolo::de_clave(&protocolo),
                )
                && let Some(lan) = lan
            {
                let r = ReglaPuertos {
                    id: instancia.clone(),
                    nombre: nombre.clone(),
                    protocolo: p,
                    externo: ext,
                    interno: int,
                    ip,
                    activa: true,
                };
                if let Err(e) = validar_regla(&r, &existentes, lan) {
                    match e.campo() {
                        "destino" if otra => marcar(e_otra_ip, "destino", e.texto(lan.0)),
                        "destino" => marcar(e_dispositivo, "dispositivo", e.texto(lan.0)),
                        _ => marcar(e_puerto, "puerto", e.texto(lan.0)),
                    }
                }
            }
            if let Some(campo) = mal.get() {
                enfocar(&control(campo));
                return;
            }
            let (Some(ext), Some(int)) = (externo, interno) else {
                return;
            };
            let mut pedir: BTreeMap<String, String> = [
                ("nombre", nombre),
                ("protocolo", protocolo),
                ("externo_desde", ext.0.to_string()),
                ("externo_hasta", ext.1.to_string()),
                ("destino", destino),
                ("interno_desde", int.0.to_string()),
                ("interno_hasta", int.1.to_string()),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
            // Al editar, solo lo que cambia, y lo que identifica la regla
            // leída para que el servidor compruebe que sigue siendo la misma.
            let mut esperados = BTreeMap::new();
            if let Some(i) = editando.get_value() {
                pedir.retain(|k, v| *v != valor_de(&i.campos, k));
                esperados = IDENTIFICAN_REGLA
                    .iter()
                    .map(|k| ((*k).to_owned(), valor_de(&i.campos, k)))
                    .collect();
                if pedir.is_empty() {
                    error.set(Some(t!("ajustes-error-sin-cambios")));
                    return;
                }
            } else {
                pedir.insert("activa".into(), "true".into());
            }
            preparando.set(true);
            let p = PeticionAjuste {
                ajuste: "puertos.reglas".into(),
                instancia: instancia.clone(),
                borrar: false,
                valores: pedir,
                esperados,
            };
            spawn_local(async move {
                match preparar_ajuste(p).await {
                    Ok(g) => preparado.run(g),
                    Err(e) => error.set(Some(texto_error(&e))),
                }
                preparando.set(false);
            });
        }
    };

    // El selector de dispositivos, con la misma forma que un campo.
    let selector = {
        let id_d = id("dispositivo");
        let control_d = format!("{id_d}-control");
        let desc_d = format!("{id_d}-desc");
        let error_d = format!("{id_d}-error");
        let descrito = format!("{desc_d} {error_d}");
        let opciones = elegibles
            .iter()
            .map(|(ip, quien)| {
                let elegida = *ip == eleccion_inicial;
                view! { <option value=ip.clone() selected=elegida>{format!("{quien} ({ip})")}</option> }
            })
            .collect_view();
        let otra_elegida = eleccion_inicial == OTRA_IP;
        let ninguna = eleccion_inicial.is_empty();
        view! {
            <div class="campo campo-ajuste">
                <label for=control_d.clone()>{t!("ajuste-puertos-reglas-dispositivo")}</label>
                <select
                    id=control_d
                    aria-describedby=descrito
                    aria-invalid=move || e_dispositivo.with(|e| e.is_some()).then_some("true")
                    disabled=move || preparando.get()
                    prop:value=move || eleccion.get()
                    on:change=move |ev| {
                        eleccion.set(event_target_value(&ev));
                        e_dispositivo.set(None);
                    }
                >
                    <option value="" selected=ninguna disabled=true>
                        {t!("ajuste-puertos-reglas-dispositivo-elegir")}
                    </option>
                    {opciones}
                    <option value=OTRA_IP selected=otra_elegida>{t!("ajuste-puertos-reglas-otra-ip")}</option>
                </select>
                <p class="ayuda" id=desc_d>{t!("ajuste-puertos-reglas-dispositivo-desc")}</p>
                <p class="campo-error" id=error_d aria-live="polite">{move || e_dispositivo.get()}</p>
            </div>
        }
    };
    let otra_ip = {
        let dom_ip = id("destino");
        let c = c_destino.clone();
        move || {
            (eleccion.get() == OTRA_IP).then(|| {
                view! {
                    <Campo c=c.clone() dom=dom_ip.clone() valor=otra_ip_v error=e_otra_ip bloqueado=preparando />
                }
            })
        }
    };
    let interno_abierto = !ini.interno.is_empty();
    view! {
        <form class="ajuste-formulario regla-formulario" novalidate=true on:submit=guardar>
            <h3>{titulo}</h3>
            <div class="ajuste-campos">
                <Campo c=c_nombre dom=id("nombre") valor=nombre_v error=e_nombre bloqueado=preparando />
                <Campo c=c_protocolo dom=id("protocolo") valor=protocolo_v error=e_protocolo bloqueado=preparando />
                <Campo c=c_puerto dom=id("puerto") valor=puerto_v error=e_puerto bloqueado=preparando />
                {selector}
                {otra_ip}
                <details class="regla-interno" id=id_interno.clone() open=interno_abierto>
                    <summary>{t!("ajuste-puertos-reglas-puerto-distinto")}</summary>
                    <Campo c=c_interno dom=id("interno") valor=interno_v error=e_interno bloqueado=preparando />
                </details>
            </div>
            <div class="ajuste-acciones">
                <button
                    class="boton-sutil"
                    type="button"
                    disabled=move || preparando.get()
                    on:click=move |_| cancelar.run(())
                >
                    {t!("ajustes-cancelar")}
                </button>
                <button
                    class="boton"
                    type="submit"
                    aria-disabled=move || preparando.get().then_some("true")
                >
                    {move || {
                        if preparando.get() { t!("ajustes-grupo-preparando") } else { t!("ajustes-guardar") }
                    }}
                </button>
            </div>
        </form>
    }
    .into_any()
}

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
                        class="boton-sutil"
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
        view! { <Insignia estado=Estado::Aviso texto=t!("ajuste-puertos-reglas-destino-perdido") /> }
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
                        class="boton-mini"
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
                    class="boton-mini boton-peligro"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puerto_o_rango_escrito() {
        assert_eq!(leer_puertos("8080"), Some((8080, 8080)));
        assert_eq!(leer_puertos(" 27015-27030 "), Some((27015, 27030)));
        assert_eq!(leer_puertos("27015 – 27030"), Some((27015, 27030)));
        assert_eq!(leer_puertos("0"), Some((0, 0)));
        for mal in ["", "http", "70000", "80-", "-80", "1-2-3", "8080,8081"] {
            assert_eq!(leer_puertos(mal), None, "{mal}");
        }
    }

    #[test]
    fn puertos_del_dispositivo_del_mismo_tamano() {
        assert_eq!(puertos_internos((8080, 8080), None), Some((8080, 8080)));
        assert_eq!(puertos_internos((9000, 9010), Some(80)), Some((80, 90)));
        assert_eq!(puertos_internos((9000, 9010), Some(65530)), None);
    }

    #[test]
    fn rango_para_leer_y_para_editar() {
        assert_eq!(rango((8080, 8080)), "8080");
        assert_eq!(rango((1, 2)), "1–2");
        assert_eq!(rango_editable((1, 2)), "1-2");
    }

    fn red(d: Option<Vec<DispositivoVista>>, r: Option<IpFijas>) -> RedPuertos {
        RedPuertos {
            dispositivos: d,
            reservas: r,
            lan: None,
        }
    }

    /// Sin los dispositivos o sin las IP fijas no se sabe si el destino
    /// sigue: no se avisa.
    #[test]
    fn sin_leer_no_avisa() {
        assert!(red(Some(vec![]), Some(vec![])).destino_perdido("192.168.1.75"));
        assert!(!red(None, Some(vec![])).destino_perdido("192.168.1.75"));
        assert!(!red(Some(vec![]), None).destino_perdido("192.168.1.75"));
    }
}
