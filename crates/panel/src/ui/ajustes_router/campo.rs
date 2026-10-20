//! Un campo de una tarjeta de Ajustes: su valor en la vista resumida y su
//! control al editar ([`Campo`]), con etiqueta, descripción, recomendación,
//! aviso de la opción elegida y error junto al campo.
//!
//! El valor en edición es siempre el legible como texto: la clave de una
//! opción, `true`/`false` en un sí/no, el texto tal cual. Un valor del
//! router que el panel no reconoce empieza como [`DESCONOCIDO`] y no se envía
//! mientras no se elija otro; un secreto empieza con su valor leído (y se
//! envía solo si cambia); uno que el router no deja leer, vacío.

use crate::ajustes_router::{CampoVista, OpcionVista, texto_error_valor};
use crate::t;
use crate::ui::i18n::traducir;
use crate::ui::secreto::{Secreto, SecretoLeido};
use crate::ui::{Estado, IconoEstado};
use ajustes::{Limites, SIN_ELECCION, TipoCampo, ValorLegible, validar_legible};
use leptos::prelude::*;

/// Valor en edición de un campo cuyo valor del router no se reconoce. No es
/// la clave de ninguna opción (solo llevan minúsculas, cifras y `_`).
pub(crate) const DESCONOCIDO: &str = "?";

/// Valor con que empieza la edición de un campo (sin el valor leído de un
/// secreto: para eso, [`inicial_campo`]).
pub(crate) fn inicial(v: &ValorLegible) -> String {
    match v {
        ValorLegible::Texto(t) | ValorLegible::Opcion(t) => t.clone(),
        ValorLegible::Booleano(b) => b.to_string(),
        ValorLegible::Desconocido(_) => DESCONOCIDO.to_owned(),
        ValorLegible::Secreto => String::new(),
    }
}

/// Valor con que empieza la edición del campo `c`: un secreto, con el que se
/// leyó del router (vacío si no deja leerlo).
pub(crate) fn inicial_campo(c: &CampoVista) -> String {
    match c.valor {
        ValorLegible::Secreto => c.valor_secreto.clone().unwrap_or_default(),
        ref v => inicial(v),
    }
}

/// ¿Se ha cambiado el campo? Un valor no reconocido, si se ha elegido otro;
/// el resto (también un secreto), si difiere del que se leyó.
pub(crate) fn cambiado(c: &CampoVista, v: &str) -> bool {
    match c.tipo {
        TipoCampo::Secreto => v != inicial_campo(c),
        _ => v != DESCONOCIDO && v != inicial(&c.valor),
    }
}

/// ¿El valor en edición sirve como valor de un campo exigido por otro (la
/// toma del espejo)? Uno no reconocido, vacío o «sin elegir», no.
pub(crate) fn elegido(c: &CampoVista, v: &str) -> bool {
    v != DESCONOCIDO && !v.trim().is_empty() && !(c.sin_eleccion && v == SIN_ELECCION)
}

/// Comprueba el valor como lo hará el servidor (mismos límites, en bytes) y
/// devuelve el mensaje en el idioma activo.
pub(crate) fn validar(c: &CampoVista, v: &str) -> Result<(), String> {
    let l = Limites {
        min: c.min,
        max: c.max,
        ascii: c.ascii,
        opcional: c.opcional,
    };
    validar_legible(c.tipo, l, |k| c.opciones.iter().any(|o| o.clave == k), v)
        .map_err(|e| texto_error_valor(&e))
}

/// Etiqueta del campo.
pub(crate) fn etiqueta(c: &CampoVista) -> String {
    traducir(&c.clave, None)
}

/// Texto de la opción `clave` del campo.
fn texto_opcion(c: &CampoVista, clave: &str) -> String {
    traducir(&format!("{}-{clave}", c.clave), None)
}

/// Texto de un valor legible del campo `c` (`v` como en edición).
pub(crate) fn texto_legible(c: &CampoVista, v: &str) -> String {
    match c.tipo {
        TipoCampo::Opcion => texto_opcion(c, v),
        TipoCampo::Booleano if v == "true" => t!("comun-si"),
        TipoCampo::Booleano => t!("comun-no"),
        _ => v.to_owned(),
    }
}

/// Valor actual del campo en la vista resumida. Un secreto, censurado con el
/// ojo (de solo lectura; vacío, con el motivo, si el router no deja leerlo).
pub(crate) fn valor_resumido(c: &CampoVista) -> AnyView {
    match &c.valor {
        ValorLegible::Secreto => {
            view! { <SecretoLeido valor=c.valor_secreto.clone() nombre=etiqueta(c) /> }.into_any()
        }
        ValorLegible::Desconocido(x) => {
            view! { <span>{t!("ajustes-valor-desconocido", valor = x.clone())}</span> }.into_any()
        }
        ValorLegible::Texto(t) if t.is_empty() => {
            view! { <span class="ajuste-vacio">{t!("ajustes-grupo-vacio")}</span> }.into_any()
        }
        v => view! { <span>{texto_legible(c, &inicial(v))}</span> }.into_any(),
    }
}

/// ¿El valor actual es el recomendado?
pub(crate) fn es_recomendado(c: &CampoVista) -> Option<bool> {
    let r = c.recomendado.as_ref()?;
    let reconocido = !matches!(
        c.valor,
        ValorLegible::Secreto | ValorLegible::Desconocido(_)
    );
    Some(reconocido && inicial(&c.valor) == *r)
}

/// Parte el motivo de una recomendación («Recomendado si … A favor: …
/// En contra: …») en la frase de para quién es y el resto (a favor y en
/// contra), que va plegado. Sin la marca, todo es la primera parte.
pub(crate) fn partir_motivo(motivo: &str, marca: &str) -> (String, Option<String>) {
    match motivo.find(&format!(" {marca}")) {
        Some(i) => (
            motivo[..i].trim().to_owned(),
            Some(motivo[i..].trim().to_owned()),
        ),
        None => (motivo.trim().to_owned(), None),
    }
}

/// Recomendación de un campo: el valor recomendado, para quién es y, plegado,
/// a favor y en contra. El usuario decide: nada se aplica solo.
fn recomendacion(c: &CampoVista, id: &str) -> Option<AnyView> {
    let r = c.recomendado.clone()?;
    let valor = texto_legible(c, &r);
    let (para_quien, pros) = c
        .motivo_recomendado
        .as_ref()
        .map(|m| partir_motivo(&traducir(m, None), &t!("ajustes-recomendado-marca")))
        .unwrap_or_default();
    Some(
        view! {
            <div class="recomendacion" id=id.to_owned()>
                <p>
                    <span class="chip chip-recomendado">
                        {t!("ajustes-recomendado-valor", valor = valor)}
                    </span>
                    " "
                    {para_quien}
                </p>
                {pros.map(|p| {
                    view! {
                        <details class="recomendacion-pros">
                            <summary>{t!("ajustes-recomendado-pros")}</summary>
                            <p>{p}</p>
                        </details>
                    }
                })}
            </div>
        }
        .into_any(),
    )
}

/// Aviso de la opción elegida (p. ej. una red WiFi abierta).
fn aviso_de(opciones: &[OpcionVista], v: &str) -> Option<String> {
    opciones
        .iter()
        .find(|o| o.clave == v)
        .and_then(|o| o.aviso.clone())
}

/// Un campo editable: control según su tipo, con etiqueta, descripción,
/// recomendación, aviso de la opción elegida y error. `dom` es el prefijo
/// único de sus ids en la página. Un secreto va con [`CampoSecreto`].
#[component]
pub(crate) fn Campo(
    c: CampoVista,
    dom: String,
    valor: RwSignal<String>,
    error: RwSignal<Option<String>>,
    #[prop(into)] bloqueado: Signal<bool>,
) -> impl IntoView {
    if c.tipo == TipoCampo::Secreto {
        return view! { <CampoSecreto c dom valor error bloqueado /> }.into_any();
    }
    let control_id = format!("{dom}-control");
    let desc_id = format!("{dom}-desc");
    let error_id = format!("{dom}-error");
    let reco_id = format!("{dom}-reco");
    let opcion_id = format!("{dom}-opcion");
    let tiene_reco = c.recomendado.is_some();
    let descrito = {
        let mut ids = vec![desc_id.clone()];
        if c.tipo == TipoCampo::Opcion {
            ids.push(opcion_id.clone());
        }
        if tiene_reco {
            ids.push(reco_id.clone());
        }
        ids.push(error_id.clone());
        ids.join(" ")
    };
    let invalido = move || error.with(|e| e.is_some()).then_some("true");
    let al_cambiar = move |ev: leptos::ev::Event| {
        valor.set(event_target_value(&ev));
        error.set(None);
    };
    let desc = traducir(&format!("{}-desc", c.clave), None);
    let etiqueta = etiqueta(&c);
    let control = match c.tipo {
        TipoCampo::Booleano => view! {
            <input
                id=control_id.clone()
                class="interruptor"
                type="checkbox"
                role="switch"
                aria-describedby=descrito.clone()
                aria-invalid=invalido
                disabled=move || bloqueado.get()
                prop:checked=move || valor.with(|v| v == "true")
                on:change=move |ev| {
                    valor.set(event_target_checked(&ev).to_string());
                    error.set(None);
                }
            />
        }
        .into_any(),
        TipoCampo::Opcion => {
            // Opciones extra, solo si es el valor actual: uno que el panel no
            // reconoce o «sin elegir» (la toma del espejo apagado).
            let extra = match &c.valor {
                ValorLegible::Desconocido(x) => Some((
                    DESCONOCIDO.to_owned(),
                    t!("ajustes-valor-desconocido", valor = x.clone()),
                )),
                ValorLegible::Opcion(k) if c.sin_eleccion && k == SIN_ELECCION => {
                    Some((SIN_ELECCION.to_owned(), texto_opcion(&c, SIN_ELECCION)))
                }
                _ => None,
            };
            // El atributo `selected` da el valor del HTML del servidor; luego
            // manda `prop:value` (una opción que la persona ha elegido ya no
            // hace caso al atributo, y «Cancelar» tiene que poder volver).
            let inicial = inicial(&c.valor);
            let opcion = |clave: String, texto: String| {
                let elegida = clave == inicial;
                view! { <option value=clave selected=elegida>{texto}</option> }
            };
            let lista = extra
                .into_iter()
                .chain(
                    c.opciones
                        .iter()
                        .map(|o| (o.clave.clone(), texto_opcion(&c, &o.clave))),
                )
                .map(|(clave, texto)| opcion(clave, texto))
                .collect_view();
            view! {
                <select
                    id=control_id.clone()
                    aria-describedby=descrito.clone()
                    aria-invalid=invalido
                    disabled=move || bloqueado.get()
                    prop:value=move || valor.get()
                    on:change=al_cambiar
                >
                    {lista}
                </select>
            }
            .into_any()
        }
        tipo => {
            // (Un secreto va con `CampoSecreto`.)
            let (tipo_html, modo, autocompletar) = match tipo {
                TipoCampo::Numero => ("number", Some("numeric"), "off"),
                TipoCampo::Ip => ("text", Some("decimal"), "off"),
                _ => ("text", None, "off"),
            };
            let (min, max) = if tipo == TipoCampo::Numero {
                (c.min.map(|m| m.to_string()), c.max.map(|m| m.to_string()))
            } else {
                (None, None)
            };
            view! {
                <input
                    id=control_id.clone()
                    type=tipo_html
                    inputmode=modo
                    autocomplete=autocompletar
                    autocapitalize="none"
                    spellcheck="false"
                    min=min
                    max=max
                    aria-describedby=descrito.clone()
                    aria-invalid=invalido
                    disabled=move || bloqueado.get()
                    prop:value=move || valor.get()
                    on:input=al_cambiar
                />
            }
            .into_any()
        }
    };
    let es_interruptor = c.tipo == TipoCampo::Booleano;
    let clave_opcion = c.clave.clone();
    let opciones = c.opciones.clone();
    // Lo que se ve de la opción elegida: su descripción y su aviso.
    let opcion_elegida = (c.tipo == TipoCampo::Opcion).then_some({
        move || {
            let v = valor.get();
            if v == DESCONOCIDO {
                return view! {
                    <p class="ayuda" id=opcion_id.clone()>{t!("ajustes-valor-desconocido-desc")}</p>
                }
                .into_any();
            }
            let desc = traducir(&format!("{clave_opcion}-{v}-desc"), None);
            let aviso = aviso_de(&opciones, &v).map(|a| {
                view! {
                    <p class="aviso estado-aviso" role="status">
                        <IconoEstado estado=Estado::Aviso />
                        <span>{traducir(&a, None)}</span>
                    </p>
                }
            });
            view! {
                <p class="ayuda opcion-elegida" id=opcion_id.clone()>{desc}</p>
                {aviso}
            }
            .into_any()
        }
    });
    view! {
        <div class="campo campo-ajuste" class:campo-interruptor-ajuste=es_interruptor>
            {if es_interruptor {
                view! {
                    <div class="campo-interruptor">
                        {control}
                        <label for=control_id.clone()>{etiqueta}</label>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <label for=control_id.clone()>{etiqueta}</label>
                    {control}
                }
                    .into_any()
            }}
            <p class="ayuda" id=desc_id>{desc}</p>
            {opcion_elegida}
            {recomendacion(&c, &reco_id)}
            <p class="campo-error" id=error_id aria-live="polite">{move || error.get()}</p>
        </div>
    }
    .into_any()
}

/// Un secreto al editar: su campo de contraseña con el valor leído,
/// censurado y con el ojo, que se edita ahí mismo. Si el router no deja
/// leerlo, sale vacío y lo dice; uno tapado al leer se pide al guardar.
#[component]
fn CampoSecreto(
    c: CampoVista,
    dom: String,
    valor: RwSignal<String>,
    error: RwSignal<Option<String>>,
    bloqueado: Signal<bool>,
) -> impl IntoView {
    let control_id = format!("{dom}-control");
    let desc_id = format!("{dom}-desc");
    let error_id = format!("{dom}-error");
    let etiqueta = etiqueta(&c);
    let sin_leer = c.valor_secreto.is_none();
    let mut desc = traducir(&format!("{}-desc", c.clave), None);
    // La de uno tapado ya dice que hay que escribirlo.
    if sin_leer && !c.tapado {
        desc = format!("{desc} {}", t!("ajustes-secreto-sin-leer-desc"));
    }
    // Corto, para que quepa a 360 px; la explicación va en la ayuda.
    let vacio = sin_leer.then(|| t!("secreto-sin-leer"));
    let invalido = Signal::derive(move || error.with(|e| e.is_some()));
    let campo = match vacio {
        Some(vacio) => view! {
            <Secreto
                valor
                nombre=etiqueta.clone()
                editable=true
                id=control_id.clone()
                vacio
                descrito=format!("{desc_id} {error_id}")
                invalido
                bloqueado
                error
            />
        }
        .into_any(),
        None => view! {
            <Secreto
                valor
                nombre=etiqueta.clone()
                editable=true
                id=control_id.clone()
                descrito=format!("{desc_id} {error_id}")
                invalido
                bloqueado
                error
            />
        }
        .into_any(),
    };
    view! {
        <div class="campo campo-ajuste campo-secreto">
            <label for=control_id>{etiqueta}</label>
            {campo}
            <p class="ayuda" id=desc_id>{desc}</p>
            <p class="campo-error" id=error_id aria-live="polite">{move || error.get()}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ajustes_router::OpcionVista;

    fn campo(tipo: TipoCampo, valor: ValorLegible) -> CampoVista {
        CampoVista {
            id: "c".into(),
            clave: "ajuste-wifi-red_24-nombre".into(),
            tipo,
            valor,
            opciones: vec![
                OpcionVista {
                    clave: "wpa2".into(),
                    recomendada: true,
                    aviso: None,
                },
                OpcionVista {
                    clave: "abierta".into(),
                    recomendada: false,
                    aviso: Some("aviso-wifi_abierta".into()),
                },
            ],
            min: Some(8),
            max: Some(63),
            ascii: true,
            opcional: false,
            solo_lectura: None,
            tapado: false,
            valor_secreto: None,
            recomendado: Some("wpa2".into()),
            motivo_recomendado: None,
            requiere: None,
            sin_eleccion: false,
        }
    }

    #[test]
    fn cambiado_segun_el_tipo() {
        let secreto = campo(TipoCampo::Secreto, ValorLegible::Secreto);
        assert!(!cambiado(&secreto, ""), "vacío = sin tocar");
        assert!(cambiado(&secreto, "nueva-clave"));
        let opcion = campo(TipoCampo::Opcion, ValorLegible::Desconocido("7".into()));
        assert_eq!(inicial(&opcion.valor), DESCONOCIDO);
        assert!(
            !cambiado(&opcion, DESCONOCIDO),
            "sin elegir otro no se envía"
        );
        assert!(cambiado(&opcion, "wpa2"));
        let si_no = campo(TipoCampo::Booleano, ValorLegible::Booleano(true));
        assert!(!cambiado(&si_no, "true"));
        assert!(cambiado(&si_no, "false"));
    }

    #[test]
    fn valida_con_los_mismos_limites_en_bytes() {
        let secreto = campo(TipoCampo::Secreto, ValorLegible::Secreto);
        assert!(validar(&secreto, "12345678").is_ok());
        // Con tildes no vale en una contraseña WiFi (solo ASCII).
        let e = validar(&secreto, "contraseña-larga").unwrap_err();
        assert_eq!(e, t!("ajustes-error-ascii"));
        let e = validar(&secreto, "corta").unwrap_err();
        assert!(e.contains('8'), "{e}");
        let opcion = campo(TipoCampo::Opcion, ValorLegible::Opcion("wpa2".into()));
        assert_eq!(
            validar(&opcion, DESCONOCIDO).unwrap_err(),
            t!("ajustes-error-opcion")
        );
    }

    #[test]
    fn motivo_partido_por_la_marca() {
        let (para, pros) = partir_motivo(
            "Recomendado si X. A favor: uno. En contra: dos.",
            "A favor:",
        );
        assert_eq!(para, "Recomendado si X.");
        assert_eq!(pros.as_deref(), Some("A favor: uno. En contra: dos."));
        let (para, pros) = partir_motivo("Sin marca.", "A favor:");
        assert_eq!((para.as_str(), pros), ("Sin marca.", None));
    }

    #[test]
    fn recomendado_si_coincide() {
        let c = campo(TipoCampo::Opcion, ValorLegible::Opcion("wpa2".into()));
        assert_eq!(es_recomendado(&c), Some(true));
        let c = campo(TipoCampo::Opcion, ValorLegible::Opcion("abierta".into()));
        assert_eq!(es_recomendado(&c), Some(false));
        let c = campo(TipoCampo::Opcion, ValorLegible::Desconocido("wpa2".into()));
        assert_eq!(es_recomendado(&c), Some(false));
    }
}
