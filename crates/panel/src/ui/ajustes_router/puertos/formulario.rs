//! Formulario de una regla de puertos, nueva o existente.

use super::super::detalle::Registro;
use super::super::lista::seguir_lo_editado;
use super::*;

/// Formulario de una regla, nueva o existente (`editando`: la regla y sus
/// valores tal como los leyó la página). Llama a `preparado` con el resumen
/// del cambio y a `cancelar` al dejarlo. Su error general va a `error`, la
/// única región de alerta del detalle. Lo editado se apunta en `registro`
/// con el id de su fila (`fila`) y sobrevive a que la página vuelva a leer.
#[component]
pub(super) fn FormRegla(
    plantilla: Vec<CampoVista>,
    editando: Option<InstanciaVista>,
    existentes: Vec<ReglaPuertos>,
    red: RedPuertos,
    error: RwSignal<Option<String>>,
    dom: String,
    preparado: Callback<GrupoVista>,
    cancelar: Callback<()>,
    registro: Registro,
    fila: String,
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
    let olvidar = seguir_lo_editado(
        fila,
        vec![
            ("nombre".into(), nombre_v, ini.nombre.clone()),
            ("protocolo".into(), protocolo_v, ini.protocolo.clone()),
            ("puerto".into(), puerto_v, ini.puerto.clone()),
            ("dispositivo".into(), eleccion, eleccion_inicial.clone()),
            ("otra_ip".into(), otra_ip_v, otra_ip_v.get_untracked()),
            ("interno".into(), interno_v, ini.interno.clone()),
        ],
        registro,
    );
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
                    Ok(g) => {
                        olvidar.run(());
                        preparado.run(g);
                    }
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
                    class="b-sec"
                    type="button"
                    disabled=move || preparando.get()
                    on:click=move |_| cancelar.run(())
                >
                    {t!("ajustes-cancelar")}
                </button>
                <button
                    class="b-pri"
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
