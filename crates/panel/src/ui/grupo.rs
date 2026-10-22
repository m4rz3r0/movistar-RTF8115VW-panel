//! Diálogo para guardar un grupo de cambios de Ajustes ([`DialogoGrupo`]): pasos «antes → después», «Aplicar» y «Deshacer».
//!
//! Resumen legible «antes → después» por pasos (agrupados por pestaña en un
//! lote de Mejoras), casillas para no aplicar un paso, riesgo, efecto y
//! plazo de vigilancia, «Aplicar», seguimiento del cambio y «Deshacer». Lo
//! usan las páginas de Ajustes y la de Mejoras.
//!
//! Es un `<dialog>` nativo abierto con `showModal()`: el resto de la página
//! queda inerte. Al abrirse, el foco va al título; al cerrarse, vuelve al
//! botón que lo abrió. Sin nombres de la API: solo etiquetas de los ajustes.

use crate::ajustes_router::{CambioLegible, GrupoVista, confirmar_ajuste, deshacer_ajuste};
use crate::t;
use crate::ui::comun::{EstadoVisual, Insignia};
use crate::ui::explorador::{
    CambioVista, EstadoDeCambio, RiesgoCambio, duracion, estado_cambio, texto_error,
};
use crate::ui::i18n::{idioma_actual, traducir};
use crate::ui::{Estado, IconoEstado};
use ajustes::TipoCampo;
use leptos::{ev::SubmitEvent, html, prelude::*, task::spawn_local};
use pasos::{desmarcados, marcados_de, marcas_conservadas};
use std::time::Duration;

mod pasos;

/// Cada cuánto se pregunta por el estado del cambio.
const SONDEO: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fase {
    /// Resumen a la vista, esperando «Aplicar».
    Revisar,
    /// `confirmar_ajuste` en marcha: no se puede cerrar.
    Confirmando,
    /// Siguiendo el cambio aplicado.
    Seguimiento(i64),
    /// Preparando el cambio que deshace el aplicado.
    Preparando,
}

/// Texto de un valor legible del resumen: una opción por su etiqueta, un
/// sí/no traducido, un valor que el panel no reconoce como tal y el resto
/// tal cual.
pub(crate) fn texto_valor(c: &CambioLegible, v: &str) -> String {
    let desconocido = || t!("ajustes-valor-desconocido", valor = v);
    // Sin valor (antes de crear una instancia o tras borrarla): vacío, en
    // cualquier tipo, no «valor no reconocido».
    if v.is_empty() {
        return t!("ajustes-grupo-vacio");
    }
    match c.tipo {
        TipoCampo::Opcion => {
            let clave = format!("{}-{v}", c.campo);
            if i18n::existe(idioma_actual(), &clave) {
                traducir(&clave, None)
            } else {
                desconocido()
            }
        }
        TipoCampo::Booleano => match v {
            "true" => texto_booleano(&c.campo, true),
            "false" => texto_booleano(&c.campo, false),
            _ => desconocido(),
        },
        _ => v.to_owned(),
    }
}

/// Texto de un sí/no del campo `clave`: sus textos de valor si el catálogo
/// los tiene («Encendida», «Apagada») o, si no, «Sí» y «No».
pub(crate) fn texto_booleano(clave: &str, v: bool) -> String {
    let propio = format!("{clave}-{v}");
    if i18n::existe(idioma_actual(), &propio) {
        traducir(&propio, None)
    } else if v {
        t!("comun-si")
    } else {
        t!("comun-no")
    }
}

fn riesgo(r: RiesgoCambio) -> (String, String) {
    match r {
        RiesgoCambio::Bajo => (t!("ajustes-riesgo-bajo"), t!("ajustes-riesgo-bajo-desc")),
        RiesgoCambio::Medio => (t!("ajustes-riesgo-medio"), t!("ajustes-riesgo-medio-desc")),
        RiesgoCambio::Alto => (t!("ajustes-riesgo-alto"), t!("ajustes-riesgo-alto-desc")),
    }
}

/// Cambio que se deshace tras pulsar «Deshacer» sobre el cambio `id`
/// cuando ya se estaba deshaciendo `actual`: deshacer un deshacer vuelve a
/// ser un cambio normal (título y botón de siempre).
fn siguiente_deshace(actual: Option<i64>, id: i64) -> Option<i64> {
    match actual {
        Some(_) => None,
        None => Some(id),
    }
}

/// Diálogo de un grupo preparado con [`preparar_ajuste`]
/// (`crate::ajustes_router::preparar_ajuste`). Al cerrarse llama a
/// `on_cerrar` con el id del último cambio confirmado desde él, haya
/// acabado aplicado, revertido o fallido (`None` si no se confirmó
/// ninguno), para que la página vuelva a leer los ajustes.
///
/// Con `al_aplicar`, el diálogo solo sirve para revisar: «Aplicar» lo
/// cierra y le pasa el token y los pasos desmarcados, y la página confirma
/// y sigue el cambio (la rejilla, con la insignia de cada tarjeta y el
/// aviso con «Deshacer»). `deshacer`: el grupo deshace otro cambio (título
/// y botón de deshacer).
///
/// Para una revisión con opciones (el proveedor de DNS de un lote):
/// `antes_de_pasos` se pinta encima de los pasos; cada grupo que llega por
/// `nuevo_grupo` sustituye al revisado, conservando las casillas por la
/// `clave` de cada paso; mientras `ocupado` (se prepara el nuevo), ni Esc ni
/// «Cancelar» cierran y no se aplica.
#[component]
pub fn DialogoGrupo(
    grupo: GrupoVista,
    #[prop(into)] on_cerrar: Callback<Option<i64>>,
    #[prop(optional, into)] al_aplicar: Option<Callback<(String, Vec<usize>)>>,
    #[prop(optional)] deshacer: bool,
    #[prop(optional_no_strip)] antes_de_pasos: Option<ViewFn>,
    #[prop(optional_no_strip)] nuevo_grupo: Option<Signal<Option<GrupoVista>>>,
    #[prop(optional_no_strip)] ocupado: Option<Signal<bool>>,
) -> impl IntoView {
    let ocupado_ahora = move || ocupado.is_some_and(|o| o.get_untracked());
    let ocupado = move || ocupado.is_some_and(|o| o.get());
    let dlg = NodeRef::<html::Dialog>::new();
    let titulo_ref = NodeRef::<html::H2>::new();
    let marcados = RwSignal::new(marcados_de(&grupo));
    let grupo = RwSignal::new(grupo);
    // Cambio que se está deshaciendo (cambia el título y el botón). Con
    // `deshacer`, el grupo ya es el que deshace otro (lo preparó la página).
    let deshace = RwSignal::new(deshacer.then_some(0));
    let fase = RwSignal::new(Fase::Revisar);
    let error = RwSignal::new(None::<String>);
    let cambio = RwSignal::new(None::<CambioVista>);
    let terminado =
        Memo::new(move |_| cambio.with(|c| c.as_ref().is_some_and(|c| c.estado.final_())));
    let ultimo_confirmado = StoredValue::new(None::<i64>);
    let quien_abrio = StoredValue::new_local(None::<leptos::web_sys::HtmlElement>);

    // Un grupo nuevo (otra opción elegida) sustituye al revisado, con las
    // casillas que se habían tocado.
    if let Some(nuevo) = nuevo_grupo {
        Effect::new(move |_| {
            let Some(g) = nuevo.get() else {
                return;
            };
            let marcas = grupo.with_untracked(|viejo| {
                marcados.with_untracked(|m| marcas_conservadas(viejo, m, &g))
            });
            marcados.set(marcas);
            grupo.set(g);
        });
    }

    let enfocar_titulo = move || {
        if let Some(h) = titulo_ref.get_untracked() {
            let _ = h.focus();
        }
    };

    // Abre el diálogo en cuanto existe, guardando quién tenía el foco.
    Effect::new(move |_| {
        use leptos::wasm_bindgen::JsCast;
        if let Some(d) = dlg.get()
            && !d.open()
        {
            quien_abrio.set_value(
                document()
                    .active_element()
                    .and_then(|e| e.dyn_into::<leptos::web_sys::HtmlElement>().ok()),
            );
            let _ = d.show_modal();
            enfocar_titulo();
        }
    });

    let aplicar = move |ev: SubmitEvent| {
        ev.prevent_default();
        if fase.get_untracked() != Fase::Revisar || ocupado_ahora() {
            return;
        }
        let quitar = grupo.with_untracked(|g| desmarcados(&g.pasos, &marcados.get_untracked()));
        if marcados.with_untracked(|m| !m.iter().any(|x| *x)) {
            error.set(Some(t!("ajustes-grupo-ninguno")));
            return;
        }
        error.set(None);
        let token = grupo.with_untracked(|g| g.token.clone());
        if let Some(al_aplicar) = al_aplicar {
            al_aplicar.run((token, quitar));
            if let Some(d) = dlg.get_untracked() {
                d.close();
            }
            return;
        }
        fase.set(Fase::Confirmando);
        spawn_local(async move {
            match confirmar_ajuste(token, quitar).await {
                Ok(id) => {
                    ultimo_confirmado.set_value(Some(id));
                    cambio.set(None);
                    fase.set(Fase::Seguimiento(id));
                }
                Err(e) => {
                    error.set(Some(texto_error(&e)));
                    fase.set(Fase::Revisar);
                }
            }
        });
    };

    // Seguimiento: ahora y cada 2 s hasta un estado final.
    Effect::new(move |_| {
        let Fase::Seguimiento(id) = fase.get() else {
            return;
        };
        if terminado.get() {
            return;
        }
        let consultar = move || {
            spawn_local(async move {
                match estado_cambio(id).await {
                    Ok(c) => cambio.set(Some(c)),
                    Err(e) => error.set(Some(texto_error(&e))),
                }
            });
        };
        consultar();
        let h = set_interval_with_handle(consultar, SONDEO).ok();
        on_cleanup(move || {
            if let Some(h) = h {
                h.clear();
            }
        });
    });

    let deshacer = move |_| {
        let Fase::Seguimiento(id) = fase.get_untracked() else {
            return;
        };
        error.set(None);
        fase.set(Fase::Preparando);
        spawn_local(async move {
            match deshacer_ajuste(id).await {
                Ok(g) => {
                    marcados.set(marcados_de(&g));
                    grupo.set(g);
                    deshace.update(|d| *d = siguiente_deshace(*d, id));
                    cambio.set(None);
                    fase.set(Fase::Revisar);
                    enfocar_titulo();
                }
                Err(e) => {
                    error.set(Some(texto_error(&e)));
                    fase.set(Fase::Seguimiento(id));
                }
            }
        });
    };

    let cerrar = move || {
        if !matches!(fase.get_untracked(), Fase::Confirmando | Fase::Preparando)
            && !ocupado_ahora()
            && let Some(d) = dlg.get_untracked()
        {
            d.close();
        }
    };

    let titulo = move || {
        if deshace.get().is_some() {
            t!("ajustes-grupo-deshacer-titulo")
        } else {
            t!("ajustes-grupo-revisar")
        }
    };
    // El ajuste y, si el título no basta, a qué se refiere («IP fijas:
    // host-2»).
    let ajuste = move || {
        grupo.with(|g| {
            let titulo = traducir(&g.titulo, None);
            match &g.detalle {
                Some(d) => t!(
                    "ajustes-grupo-ajuste-detalle",
                    ajuste = titulo,
                    detalle = d.clone()
                ),
                None => titulo,
            }
        })
    };
    let cabecera_riesgo = move || {
        let (texto, desc) = riesgo(grupo.with(|g| g.riesgo));
        view! {
            <Insignia estado=grupo.with(|g| g.riesgo.estado()) texto=texto />
            <span class="solo-lector">{desc}</span>
        }
    };
    let efecto = move || {
        grupo.with(|g| g.efecto.clone()).map(|e| {
            view! {
                <p>
                    <strong>{t!("ajustes-efecto")}</strong>
                    ": "
                    {traducir(&e, None)}
                </p>
            }
        })
    };
    let aviso = move || {
        grupo.with(|g| g.aviso.clone()).map(|a| {
            view! {
                <p class="aviso estado-aviso" role="status">
                    <IconoEstado estado=Estado::Aviso />
                    <span>{a}</span>
                </p>
            }
        })
    };
    let vigilancia = move || {
        let plazo = duracion(grupo.with(|g| g.vigilancia_s));
        t!("ajustes-grupo-vigilancia", plazo = plazo.as_str())
    };

    let bloqueado = Signal::derive(move || fase.get() != Fase::Revisar);
    let pasos = move || grupo.with(|g| pasos::lista(g, marcados, bloqueado));

    let estado = move || {
        if !matches!(fase.get(), Fase::Seguimiento(_) | Fase::Confirmando) {
            return None;
        }
        let Some(c) = cambio.get() else {
            return Some(
                view! { <p class="estado-cambio">{t!("ajustes-grupo-estado-aplicando")}</p> }
                    .into_any(),
            );
        };
        let (estado, texto) = match c.estado {
            EstadoDeCambio::Vigilando => (
                EstadoVisual::Proceso,
                t!(
                    "ajustes-grupo-estado-vigilando",
                    resto = duracion(c.restante_s.unwrap_or(0).max(0) as u64)
                ),
            ),
            EstadoDeCambio::Aplicado => (EstadoVisual::Emite, t!("ajustes-grupo-estado-aplicado")),
            EstadoDeCambio::Revertido => {
                (EstadoVisual::Atencion, t!("ajustes-grupo-estado-revertido"))
            }
            EstadoDeCambio::Fallido => (EstadoVisual::Peligro, t!("ajustes-grupo-estado-fallido")),
            EstadoDeCambio::RevertidoParcial => {
                (EstadoVisual::Peligro, t!("ajustes-grupo-estado-parcial"))
            }
            EstadoDeCambio::Preparado | EstadoDeCambio::Aplicando => {
                (EstadoVisual::Proceso, t!("ajustes-grupo-estado-aplicando"))
            }
        };
        Some(view! { <div class="estado-cambio"><Insignia estado texto /></div> }.into_any())
    };

    let pie = move || match fase.get() {
        Fase::Seguimiento(_) | Fase::Preparando => {
            let puede_deshacer = cambio.with(|c| {
                c.as_ref()
                    .is_some_and(|c| c.estado == EstadoDeCambio::Aplicado)
            });
            let preparando = fase.get() == Fase::Preparando;
            view! {
                {puede_deshacer.then(|| view! {
                    <button
                        class="b-sec"
                        type="button"
                        disabled=preparando
                        on:click=deshacer
                    >
                        {if preparando { t!("ajustes-grupo-preparando") } else { t!("ajustes-grupo-deshacer") }}
                    </button>
                })}
                <button class="b-pri" type="button" disabled=preparando on:click=move |_| cerrar()>
                    {t!("ajustes-grupo-cerrar")}
                </button>
            }
            .into_any()
        }
        f => {
            let borra = grupo.with(|g| g.borra);
            view! {
            <button
                class="b-sec"
                type="button"
                disabled=move || f == Fase::Confirmando || ocupado()
                on:click=move |_| cerrar()
            >
                {t!("ajustes-grupo-cancelar")}
            </button>
            // Lo que borra se confirma con el botón de peligro.
            <button
                class=if borra { "b-pel" } else { "b-pri" }
                type="submit"
                disabled=move || f == Fase::Confirmando || ocupado()
            >
                {if f == Fase::Confirmando {
                    t!("ajustes-grupo-aplicando")
                } else if deshace.get().is_some() {
                    t!("ajustes-grupo-deshacer")
                } else if borra {
                    t!("ajustes-grupo-borrar")
                } else {
                    t!("ajustes-grupo-aplicar")
                }}
            </button>
            }
            .into_any()
        }
    };

    view! {
        <dialog
            class="dialogo dialogo-grupo"
            node_ref=dlg
            aria-labelledby="grupo-titulo"
            aria-modal="true"
            on:cancel=move |ev: leptos::ev::Event| {
                if matches!(fase.get_untracked(), Fase::Confirmando | Fase::Preparando)
                    || ocupado_ahora()
                {
                    ev.prevent_default();
                }
            }
            on:close=move |_| {
                quien_abrio.with_value(|e| {
                    if let Some(e) = e {
                        let _ = e.focus();
                    }
                });
                on_cerrar.run(ultimo_confirmado.get_value());
            }
        >
            <form class="dialogo-cuerpo" on:submit=aplicar>
                <header class="dialogo-cabecera">
                    <h2 id="grupo-titulo" tabindex="-1" node_ref=titulo_ref>{titulo}</h2>
                    {cabecera_riesgo}
                </header>
                <p>
                    <strong>{ajuste}</strong>
                    " · "
                    {t!("ajustes-grupo-intro")}
                </p>
                {antes_de_pasos.map(|v| v.run())}
                {pasos}
                {aviso}
                {efecto}
                <p class="pagina-sub">{vigilancia}</p>
                <p class="error-formulario" role="alert">
                    {move || error.get()}
                </p>
                <div class="dialogo-estado" aria-live="polite">
                    {estado}
                </div>
                <div class="dialogo-pie">{pie}</div>
            </form>
        </dialog>
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::pasos::por_pestana;
    use super::*;
    use crate::ajustes_router::PasoVista;
    use crate::ui::explorador::MASCARA;

    fn cambio(campo: &str, antes: &str, despues: &str, tipo: TipoCampo) -> CambioLegible {
        CambioLegible {
            campo: campo.into(),
            antes: antes.into(),
            despues: despues.into(),
            secreto: tipo == TipoCampo::Secreto,
            tipo,
        }
    }

    fn grupo(pasos: Vec<PasoVista>) -> GrupoVista {
        GrupoVista {
            token: "t".into(),
            titulo: "ajuste-wifi-red_24-titulo".into(),
            pasos,
            riesgo: RiesgoCambio::Medio,
            efecto: Some("efecto-wifi_red".into()),
            vigilancia_s: 120,
            detalle: None,
            aviso: None,
            borra: false,
        }
    }

    fn html(g: GrupoVista) -> String {
        Owner::new().with(|| view! { <DialogoGrupo grupo=g on_cerrar=|_| {} /> }.to_html())
    }

    /// Una MAC tal cual lleva un `<wbr>` tras cada «:» (se parte solo por
    /// ahí); un sí/no traducido, ninguno.
    #[test]
    fn valor_tal_cual_se_parte_por_sus_separadores() {
        let paso = PasoVista {
            campos: vec![
                cambio(
                    "ajuste-red_local-reservas-mac",
                    "",
                    "02:00:00:00:00:03",
                    TipoCampo::Texto,
                ),
                cambio(
                    "ajuste-red_local-reservas-activa",
                    "false",
                    "true",
                    TipoCampo::Booleano,
                ),
            ],
            ..PasoVista::default()
        };
        let h = html(grupo(vec![paso]));
        assert!(h.contains("02:<wbr>"), "{h}");
        assert!(h.contains("03<wbr>"), "{h}");
        assert_eq!(h.matches("<wbr>").count(), 6, "solo los de la MAC: {h}");
    }

    /// El resumen nombra a qué se refiere el cambio si el título no basta
    /// (el dispositivo de una IP fija).
    #[test]
    fn el_resumen_lleva_el_detalle() {
        let mut g = grupo(Vec::new());
        g.titulo = "ajuste-red_local-reservas-titulo".into();
        g.detalle = Some("host-2".into());
        let h = html(g);
        let titulo = traducir("ajuste-red_local-reservas-titulo", None);
        assert!(h.contains(&format!("{titulo}: host-2")), "{h}");
    }

    /// Al crear o borrar una instancia, un sí/no o una opción sin valor se
    /// ve vacío, no como «valor no reconocido».
    #[test]
    fn sin_valor_es_vacio_en_cualquier_tipo() {
        for tipo in [TipoCampo::Booleano, TipoCampo::Opcion, TipoCampo::Ip] {
            let c = cambio("ajuste-red_local-reservas-activa", "", "true", tipo);
            assert_eq!(texto_valor(&c, ""), t!("ajustes-grupo-vacio"), "{tipo:?}");
        }
    }

    #[test]
    fn resumen_con_etiquetas_y_valores_legibles() {
        let g = grupo(vec![PasoVista {
            indice: 0,
            sin_cambios: false,
            campos: vec![
                cambio(
                    "ajuste-wifi-red_24-nombre",
                    "casa",
                    "casa-nueva",
                    TipoCampo::Texto,
                ),
                cambio(
                    "ajuste-wifi-red_24-seguridad",
                    "99",
                    "abierta",
                    TipoCampo::Opcion,
                ),
                cambio(
                    "ajuste-wifi-red_24-visible",
                    "true",
                    "false",
                    TipoCampo::Booleano,
                ),
                cambio(
                    "ajuste-wifi-red_24-contrasena",
                    MASCARA,
                    MASCARA,
                    TipoCampo::Secreto,
                ),
            ],
            titulo: None,
            nota: None,
            ..Default::default()
        }]);
        let h = html(g);
        assert!(h.contains("Revisa el cambio"), "{h}");
        assert!(h.contains("tabindex=\"-1\""), "título enfocable: {h}");
        // Etiquetas del ajuste, no parámetros.
        assert!(
            h.contains(&traducir("ajuste-wifi-red_24-nombre", None)),
            "{h}"
        );
        assert!(
            h.contains(&traducir("ajuste-wifi-red_24-seguridad-abierta", None)),
            "{h}"
        );
        assert!(
            h.contains(&t!("ajustes-valor-desconocido", valor = "99")),
            "{h}"
        );
        assert!(
            h.contains(&t!("comun-si")) && h.contains(&t!("comun-no")),
            "{h}"
        );
        assert!(h.contains(MASCARA), "{h}");
        assert!(h.contains(&traducir("efecto-wifi_red", None)), "{h}");
        assert!(h.contains("2 min"), "plazo de vigilancia: {h}");
        assert!(h.contains(&t!("ajustes-riesgo-medio")), "{h}");
        // Un solo paso: sin casillas.
        assert!(!h.contains("checkbox"), "{h}");
        for api in ["ssid", "authentication", "key_phrase", "cbSet"] {
            assert!(!h.contains(api), "{api}: {h}");
        }
        assert!(!h.contains("style="), "sin estilos en línea: {h}");
    }

    #[test]
    fn pasos_con_casillas_y_ya_esta_asi_deshabilitado() {
        let g = grupo(vec![
            PasoVista {
                indice: 0,
                sin_cambios: false,
                campos: vec![cambio(
                    "ajuste-wifi-red_24-nombre",
                    "a",
                    "b",
                    TipoCampo::Texto,
                )],
                titulo: None,
                nota: None,
                ..Default::default()
            },
            PasoVista {
                indice: 1,
                sin_cambios: true,
                campos: vec![],
                titulo: None,
                nota: None,
                ..Default::default()
            },
        ]);
        let h = html(g);
        assert_eq!(h.matches("type=\"checkbox\"").count(), 2, "{h}");
        assert!(h.contains(&t!("ajustes-grupo-ya-esta")), "{h}");
        assert_eq!(
            h.matches(" disabled").count(),
            1,
            "solo el paso sin cambios: {h}"
        );
        assert!(h.contains(&t!("ajustes-grupo-paso", n = 2)), "{h}");
    }

    /// Un paso con título («TCP», «UDP») lo enseña en vez de «Paso N», con
    /// su nota debajo; sin título, sigue siendo «Paso N».
    #[test]
    fn pasos_con_titulo_y_nota() {
        let paso = |indice, titulo: Option<&str>, nota: Option<&str>| PasoVista {
            indice,
            sin_cambios: false,
            campos: vec![cambio(
                "ajuste-puertos-reglas-protocolo",
                "",
                "udp",
                TipoCampo::Opcion,
            )],
            titulo: titulo.map(str::to_owned),
            nota: nota.map(str::to_owned),
            ..Default::default()
        };
        let h = html(grupo(vec![
            paso(0, Some("ajuste-puertos-reglas-protocolo-tcp"), None),
            paso(
                1,
                Some("ajuste-puertos-reglas-protocolo-udp"),
                Some("ajuste-puertos-reglas-paso-igual"),
            ),
        ]));
        assert!(h.contains(">TCP</label>"), "{h}");
        assert!(h.contains(">UDP</label>"), "{h}");
        assert!(!h.contains(&t!("ajustes-grupo-paso", n = 1)), "{h}");
        assert!(
            h.contains(&traducir("ajuste-puertos-reglas-paso-igual", None)),
            "{h}"
        );
        let h = html(grupo(vec![paso(0, None, None), paso(1, None, None)]));
        assert!(h.contains(&t!("ajustes-grupo-paso", n = 2)), "{h}");
    }

    /// Encender o apagar una red dice «Apagada → Encendida», no «No → Sí»:
    /// los textos de valor del catálogo; un sí/no sin ellos, «Sí» y «No».
    #[test]
    fn un_si_no_con_sus_textos_de_valor() {
        let c = cambio(
            "ajuste-wifi-red_24-activa",
            "false",
            "true",
            TipoCampo::Booleano,
        );
        assert_eq!(texto_valor(&c, "false"), "Apagada");
        assert_eq!(texto_valor(&c, "true"), "Encendida");
        let c = cambio(
            "ajuste-wifi-red_24-wps",
            "true",
            "false",
            TipoCampo::Booleano,
        );
        assert_eq!(texto_valor(&c, "true"), t!("comun-si"));
        assert_eq!(texto_valor(&c, "false"), t!("comun-no"));
    }

    #[test]
    fn deshacer_un_deshacer_vuelve_a_ser_un_cambio_normal() {
        assert_eq!(siguiente_deshace(None, 5), Some(5));
        assert_eq!(siguiente_deshace(Some(5), 6), None);
        assert_eq!(siguiente_deshace(None, 7), Some(7));
    }

    #[test]
    fn desmarcados_solo_los_pasos_con_cambios() {
        let paso = |indice, sin_cambios| PasoVista {
            indice,
            sin_cambios,
            campos: vec![],
            titulo: None,
            nota: None,
            ..Default::default()
        };
        let pasos = vec![paso(0, false), paso(1, true), paso(2, false)];
        assert_eq!(desmarcados(&pasos, &[true, false, false]), vec![2]);
        assert_eq!(
            desmarcados(&pasos, &[true, false, true]),
            Vec::<usize>::new()
        );
        assert_eq!(marcados_de(&grupo(pasos)), vec![true, false, true]);
    }

    #[test]
    fn marcados_iniciales_respetan_desmarcado() {
        let paso = |indice, sin_cambios, desmarcado| PasoVista {
            indice,
            sin_cambios,
            desmarcado,
            ..Default::default()
        };
        let g = grupo(vec![
            paso(0, false, false),
            paso(1, false, true),
            paso(2, true, false),
        ]);
        assert_eq!(marcados_de(&g), vec![true, false, false]);
    }

    /// Al cambiar de proveedor, el grupo nuevo conserva lo que se había
    /// tocado, paso a paso por su clave, aunque cambie de posición.
    #[test]
    fn marcas_se_conservan_por_clave() {
        let paso = |indice, clave: &str, desmarcado| PasoVista {
            indice,
            clave: Some(clave.into()),
            desmarcado,
            ..Default::default()
        };
        let viejo = grupo(vec![
            paso(0, "dns_casa", false),
            paso(1, "aviso_operador", false),
            paso(2, "hora_neutra", true),
        ]);
        // El aviso, desmarcado a mano; la hora, marcada a mano.
        let marcas = [true, false, true];
        let mut nuevo = grupo(vec![
            paso(0, "aviso_operador", false),
            paso(1, "gestion_remota", false),
            paso(2, "dns_casa", false),
            paso(3, "hora_neutra", true),
        ]);
        assert_eq!(
            marcas_conservadas(&viejo, &marcas, &nuevo),
            vec![false, true, true, true]
        );
        // Lo que ya no cambia nada, desmarcado.
        nuevo.pasos[2].sin_cambios = true;
        assert_eq!(
            marcas_conservadas(&viejo, &marcas, &nuevo),
            vec![false, true, false, true]
        );
    }

    #[test]
    fn por_pestana_agrupa_seguidos() {
        let paso = |indice, pestana: Option<&str>| PasoVista {
            indice,
            pestana: pestana.map(str::to_owned),
            ..Default::default()
        };
        let red = Some("ajustes-tema-red_local");
        let seg = Some("ajustes-tema-seguridad");
        let pasos = [
            paso(0, red),
            paso(1, seg),
            paso(2, seg),
            paso(3, None),
            paso(4, None),
            paso(5, red),
        ];
        let s = |x: Option<&str>| x.map(str::to_owned);
        assert_eq!(
            por_pestana(&pasos),
            vec![
                (s(red), vec![0]),
                (s(seg), vec![1, 2]),
                (None, vec![3, 4]),
                (s(red), vec![5]),
            ]
        );
    }

    /// Los pasos de un lote van bajo el nombre de su pestaña; los de un
    /// ajuste (sin pestaña), sin encabezado.
    #[test]
    fn encabezado_por_pestana() {
        let paso = |indice, pestana: Option<&str>| PasoVista {
            indice,
            pestana: pestana.map(str::to_owned),
            ..Default::default()
        };
        let h = html(grupo(vec![
            paso(0, Some("ajustes-tema-red_local")),
            paso(1, Some("ajustes-tema-seguridad")),
            paso(2, Some("ajustes-tema-seguridad")),
        ]));
        assert_eq!(h.matches("<h3").count(), 2, "{h}");
        let red = h.find(&t!("ajustes-tema-red_local")).expect("Red local");
        let seg = h.find(&t!("ajustes-tema-seguridad")).expect("Seguridad");
        assert!(red < seg, "{h}");
        let h = html(grupo(vec![paso(0, None), paso(1, None)]));
        assert!(!h.contains("<h3"), "{h}");
    }
}
