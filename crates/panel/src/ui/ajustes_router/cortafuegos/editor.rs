//! El editor completo del cortafuegos: los filtros con sus reglas en orden, y alta, cambio y baja de cada regla y cada filtro ([`Editor`]).
//!
//! Lee los filtros con [`leer_cortafuegos`] y prepara cada cambio con
//! [`preparar_cortafuegos`]: se revisa en el diálogo de siempre, se aplica
//! por la cola y se deshace desde el aviso o el Historial. Lo que el
//! servidor no deja hacer (sin comprobar en el router real) sale con su
//! motivo y los botones desactivados. Va dentro del formulario del
//! detalle: solo botones `type="button"`, sin otro `<form>`. Sin lectura,
//! queda la tabla de siempre.

use super::super::aviso::Seguimiento;
use super::super::consulta;
use super::super::rejilla::Pagina;
use crate::ajustes_router::{
    AjusteVista, EditorCortafuegos, FiltroEditor, OpcionEditor, PeticionCortafuegos, ReglaEditor,
    ValoresCortafuegos, leer_cortafuegos, preparar_cortafuegos,
};
use crate::t;
use crate::ui::explorador::texto_error;
use crate::ui::i18n::traducir;
use leptos::{prelude::*, task::spawn_local};

/// Prefijo de las claves de los campos.
const ED: &str = "ajuste-seguridad-cortafuegos-ed";
/// Id de la nota con el motivo general (lo describen los botones).
const ID_MOTIVO: &str = "cortafuegos-editor-motivo";

/// Lo que se está editando.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Edicion {
    /// Una regla del filtro `fid` (de la versión `version`): nueva o la de
    /// ese RID (`antes`, sus valores leídos).
    Regla {
        fid: String,
        version: String,
        rid: Option<String>,
        antes: ValoresCortafuegos,
    },
    /// Un filtro: nuevo o el de ese FID (`con_reglas`: no cambia de
    /// versión).
    Filtro {
        fid: Option<String>,
        con_reglas: bool,
        antes: ValoresCortafuegos,
    },
}

/// Lo que comparten las piezas del editor.
#[derive(Clone, Copy)]
struct Ctx {
    edicion: RwSignal<Option<Edicion>>,
    /// Los valores del formulario abierto.
    form: RwSignal<ValoresCortafuegos>,
    error: RwSignal<Option<String>>,
    ocupado: RwSignal<bool>,
    enviar: Callback<PeticionCortafuegos>,
    solo_lectura: bool,
}

impl Ctx {
    fn abrir(self, e: Edicion, valores: ValoresCortafuegos) {
        self.error.set(None);
        self.form.set(valores);
        self.edicion.set(Some(e));
    }

    fn valor(self, k: &str) -> String {
        self.form.with(|f| f.get(k).cloned().unwrap_or_default())
    }

    fn poner(self, k: &str, v: String) {
        self.form.update(|f| {
            f.insert(k.to_owned(), v);
        });
    }
}

fn v<'a>(valores: &'a ValoresCortafuegos, k: &str) -> &'a str {
    valores.get(k).map_or("", String::as_str)
}

/// Nombre de la opción `valor` de `opciones` (o el valor).
fn nombre_de(opciones: &[OpcionEditor], valor: &str) -> String {
    opciones
        .iter()
        .find(|o| o.valor == valor)
        .map_or_else(|| valor.to_owned(), |o| o.nombre.clone())
}

/// Índice de la versión (`"1"` IPv6).
fn idx(version: &str) -> usize {
    usize::from(version == "1")
}

fn version_texto(version: &str) -> String {
    if version == "1" {
        t!("ajuste-seguridad-cortafuegos-version-ipv6")
    } else {
        t!("ajuste-seguridad-cortafuegos-version-ipv4")
    }
}

/// «dirección/máscara» o «Cualquiera».
fn lado(r: &ValoresCortafuegos, addr: &str, mask: &str) -> String {
    match (v(r, addr), v(r, mask)) {
        ("", _) => t!("ajuste-seguridad-cortafuegos-editor-cualquiera"),
        (a, "") => a.to_owned(),
        (a, m) => format!("{a}/{m}"),
    }
}

/// Un rango de puertos («22», «1000–2000») o vacío.
fn rango(r: &ValoresCortafuegos, desde: &str, hasta: &str) -> String {
    match (v(r, desde), v(r, hasta)) {
        ("", _) => String::new(),
        (a, b) if b.is_empty() || a == b => a.to_owned(),
        (a, b) => format!("{a}–{b}"),
    }
}

/// Los puertos de una regla TCP o UDP: «origen → destino» con lo que haya.
fn puertos(r: &ValoresCortafuegos) -> String {
    let (o, d) = (
        rango(r, "srcportstart", "srcportend"),
        rango(r, "dstportstart", "dstportend"),
    );
    let todos = || t!("ajuste-seguridad-cortafuegos-editor-cualquiera");
    match (o.is_empty(), d.is_empty()) {
        (true, true) if matches!(v(r, "protocol"), "2" | "3") => todos(),
        (true, true) => String::new(),
        (true, false) => d,
        (false, true) => format!("{o} → {}", todos()),
        (false, false) => format!("{o} → {d}"),
    }
}

/// El tráfico de una regla: el protocolo y, con ICMP, su tipo.
fn trafico(r: &ValoresCortafuegos, icmp: &[OpcionEditor]) -> String {
    match v(r, "protocol") {
        "1" => format!(
            "{} · {}",
            t!("ajuste-seguridad-cortafuegos-protocolo-icmp"),
            nombre_de(icmp, v(r, "icmpType"))
        ),
        "2" => t!("ajuste-seguridad-cortafuegos-protocolo-tcp"),
        "3" => t!("ajuste-seguridad-cortafuegos-protocolo-udp"),
        _ => t!("ajuste-seguridad-cortafuegos-editor-todo"),
    }
}

/// Lo que hace una regla y, si rechaza, cómo.
fn accion(r: &ValoresCortafuegos, rechazo: &[OpcionEditor]) -> String {
    match v(r, "specificAction") {
        "0" => t!("ajuste-seguridad-cortafuegos-accion-descartar"),
        "1" => format!(
            "{} · {}",
            t!("ajuste-seguridad-cortafuegos-accion-rechazar"),
            nombre_de(rechazo, v(r, "rejectType"))
        ),
        _ => t!("ajuste-seguridad-cortafuegos-accion-permitir"),
    }
}

/// Los datos de un filtro: sentido, versión, interfaz y qué hace por
/// defecto.
fn datos_filtro(f: &FiltroEditor) -> String {
    let sentido = if v(&f.valores, "type") == "1" {
        t!("ajuste-seguridad-cortafuegos-ed-type-salida")
    } else {
        t!("ajuste-seguridad-cortafuegos-ed-type-entrada")
    };
    let accion = if v(&f.valores, "default") == "1" {
        t!("ajuste-seguridad-cortafuegos-ed-default-permitir")
    } else {
        t!("ajuste-seguridad-cortafuegos-ed-default-descartar")
    };
    let defecto = t!(
        "ajuste-seguridad-cortafuegos-editor-por-defecto",
        accion = accion
    );
    [
        sentido,
        version_texto(v(&f.valores, "version")),
        f.interfaz.clone(),
        defecto,
    ]
    .join(" · ")
}

/// Un botón del editor: desactivado (con `aria-disabled`) si hay motivo
/// (`motivo`, que se lee en `title`) o el panel está en solo lectura.
fn boton(
    ctx: Ctx,
    texto: String,
    clase: &'static str,
    motivo: Option<String>,
    al_pulsar: impl Fn() + 'static,
) -> impl IntoView {
    let bloqueado = ctx.solo_lectura || motivo.is_some();
    view! {
        <button
            type="button"
            class=clase
            title=motivo
            aria-disabled=move || (bloqueado || ctx.ocupado.get()).then_some("true")
            on:click=move |_| {
                if !bloqueado && !ctx.ocupado.get_untracked() {
                    al_pulsar();
                }
            }
        >
            {texto}
        </button>
    }
}

/// Un campo de texto del formulario.
fn texto(ctx: Ctx, k: &'static str, ayuda: Option<String>) -> impl IntoView {
    let id = format!("cortafuegos-{k}");
    view! {
        <div class="campo">
            <label for=id.clone()>{traducir(&format!("{ED}-{k}"), None)}</label>
            <input
                id=id
                type="text"
                autocomplete="off"
                prop:value=move || ctx.valor(k)
                on:input=move |ev| ctx.poner(k, event_target_value(&ev))
            />
            {ayuda.map(|a| view! { <p class="ayuda">{a}</p> })}
        </div>
    }
}

/// Un desplegable del formulario con `opciones` (valor, texto).
fn lista(
    ctx: Ctx,
    k: &'static str,
    opciones: Vec<(String, String)>,
    desactivado: bool,
) -> impl IntoView {
    let id = format!("cortafuegos-{k}");
    let actual = ctx.valor(k);
    let sin_elegir = !opciones.iter().any(|(x, _)| *x == actual);
    let opciones = opciones
        .into_iter()
        .map(|(valor, nombre)| {
            let sel = valor.clone();
            view! {
                <option value=valor selected=move || ctx.valor(k) == sel>{nombre}</option>
            }
        })
        .collect_view();
    view! {
        <div class="campo">
            <label for=id.clone()>{traducir(&format!("{ED}-{k}"), None)}</label>
            <select
                id=id
                disabled=desactivado
                on:change=move |ev| ctx.poner(k, event_target_value(&ev))
            >
                {sin_elegir
                    .then(|| {
                        view! {
                            <option value="" selected=true disabled=true>
                                {t!("ajuste-seguridad-cortafuegos-editor-elige")}
                            </option>
                        }
                    })}
                {opciones}
            </select>
        </div>
    }
}

/// Los valores de la regla del formulario que viajan: lo que la web
/// deshabilita (el tipo de rechazo sin «rechazar», el ICMP sin ICMP, los
/// puertos sin TCP ni UDP, la máscara sin dirección) no va.
fn valores_regla(f: &ValoresCortafuegos, version: &str) -> ValoresCortafuegos {
    let protocolo = v(f, "protocol");
    let puertos = matches!(protocolo, "2" | "3");
    let mut out = ValoresCortafuegos::new();
    let mut poner = |k: &str, si: bool| {
        if si && !v(f, k).trim().is_empty() {
            out.insert(k.to_owned(), v(f, k).trim().to_owned());
        }
    };
    poner("enabled", true);
    poner("protocol", true);
    poner("specificAction", true);
    poner("rejectType", v(f, "specificAction") == "1");
    poner("icmpType", protocolo == "1");
    for (addr, mask) in [("srcaddr", "srcmask"), ("dstaddr", "dstmask")] {
        let con = !v(f, addr).trim().is_empty();
        poner(addr, true);
        poner(mask, con);
    }
    for k in ["srcportstart", "srcportend", "dstportstart", "dstportend"] {
        poner(k, puertos);
    }
    out.insert("version".into(), version.to_owned());
    out
}

/// El formulario de una regla del filtro `fid`.
fn form_regla(
    ctx: Ctx,
    e: &EditorCortafuegos,
    fid: String,
    version: String,
    rid: Option<String>,
    antes: ValoresCortafuegos,
) -> impl IntoView + use<> {
    let i = idx(&version);
    let par = |o: &OpcionEditor| (o.valor.clone(), o.nombre.clone());
    let icmp: Vec<(String, String)> = e.icmp[i].iter().map(par).collect();
    let rechazo: Vec<(String, String)> = e.rechazo[i].iter().map(par).collect();
    let opcion = |valor: &str, clave: &str| (valor.to_owned(), traducir(clave, None));
    let protocolos = vec![
        opcion("1", "ajuste-seguridad-cortafuegos-protocolo-icmp"),
        opcion("2", "ajuste-seguridad-cortafuegos-protocolo-tcp"),
        opcion("3", "ajuste-seguridad-cortafuegos-protocolo-udp"),
    ];
    let acciones = vec![
        opcion("0", "ajuste-seguridad-cortafuegos-accion-descartar"),
        opcion("1", "ajuste-seguridad-cortafuegos-accion-rechazar"),
        opcion("2", "ajuste-seguridad-cortafuegos-accion-permitir"),
    ];
    let titulo = if rid.is_some() {
        t!("ajuste-seguridad-cortafuegos-editor-form-regla")
    } else {
        t!("ajuste-seguridad-cortafuegos-editor-form-regla-nueva")
    };
    let nueva = rid.is_none();
    let protocolo = move || ctx.valor("protocol");
    let ayuda_dir = t!("ajuste-seguridad-cortafuegos-editor-ayuda-direccion");
    let ayuda_puertos = t!("ajuste-seguridad-cortafuegos-editor-ayuda-puertos");
    let (ayuda_dir2, ayuda_puertos2) = (ayuda_dir.clone(), ayuda_puertos.clone());
    let revisar = move || {
        let valores = ctx.form.with(|f| valores_regla(f, &version));
        let p = match rid.clone() {
            None => PeticionCortafuegos::ReglaNueva {
                filtro: fid.clone(),
                valores,
            },
            Some(regla) => PeticionCortafuegos::ReglaCambiada {
                filtro: fid.clone(),
                regla,
                valores,
                antes: antes.clone(),
            },
        };
        ctx.enviar.run(p);
    };
    view! {
        <fieldset class="cortafuegos-form">
            <legend>{titulo}</legend>
            {nueva.then(|| view! { <p class="ayuda">{t!("ajuste-seguridad-cortafuegos-editor-al-final")}</p> })}
            <div class="cortafuegos-form-campos">
                <div class="campo campo-casilla">
                    <input
                        id="cortafuegos-enabled"
                        type="checkbox"
                        prop:checked=move || ctx.valor("enabled") != "0"
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            ctx.poner("enabled", if on { "1" } else { "0" }.to_owned());
                        }
                    />
                    <label for="cortafuegos-enabled">{t!("ajuste-seguridad-cortafuegos-ed-enabled")}</label>
                </div>
                {lista(ctx, "protocol", protocolos, false)}
                {lista(ctx, "specificAction", acciones, false)}
                {move || (ctx.valor("specificAction") == "1").then(|| lista(ctx, "rejectType", rechazo.clone(), false))}
                {move || (protocolo() == "1").then(|| lista(ctx, "icmpType", icmp.clone(), false))}
            </div>
            <div class="cortafuegos-form-lados">
                <div class="cortafuegos-form-lado">
                    {texto(ctx, "srcaddr", Some(ayuda_dir))}
                    {texto(ctx, "srcmask", None)}
                    {move || matches!(protocolo().as_str(), "2" | "3").then(|| view! {
                        {texto(ctx, "srcportstart", Some(ayuda_puertos.clone()))}
                        {texto(ctx, "srcportend", None)}
                    })}
                </div>
                <div class="cortafuegos-form-lado">
                    {texto(ctx, "dstaddr", Some(ayuda_dir2))}
                    {texto(ctx, "dstmask", None)}
                    {move || matches!(protocolo().as_str(), "2" | "3").then(|| view! {
                        {texto(ctx, "dstportstart", Some(ayuda_puertos2.clone()))}
                        {texto(ctx, "dstportend", None)}
                    })}
                </div>
            </div>
            {botones_form(ctx, revisar)}
        </fieldset>
    }
}

/// El formulario de un filtro: nuevo o el de `fid`.
fn form_filtro(
    ctx: Ctx,
    e: &EditorCortafuegos,
    fid: Option<String>,
    con_reglas: bool,
    antes: ValoresCortafuegos,
) -> impl IntoView + use<> {
    let interfaces: Vec<(String, String)> = e
        .interfaces
        .iter()
        .map(|(id, nombre)| (id.clone(), format!("{nombre} ({id})")))
        .collect();
    let opcion = |valor: &str, clave: &str| (valor.to_owned(), traducir(clave, None));
    let sentidos = vec![
        opcion("0", "ajuste-seguridad-cortafuegos-ed-type-entrada"),
        opcion("1", "ajuste-seguridad-cortafuegos-ed-type-salida"),
    ];
    let versiones = vec![
        opcion("0", "ajuste-seguridad-cortafuegos-version-ipv4"),
        opcion("1", "ajuste-seguridad-cortafuegos-version-ipv6"),
    ];
    let defectos = vec![
        opcion("0", "ajuste-seguridad-cortafuegos-ed-default-descartar"),
        opcion("1", "ajuste-seguridad-cortafuegos-ed-default-permitir"),
    ];
    let titulo = if fid.is_some() {
        t!("ajuste-seguridad-cortafuegos-editor-form-filtro")
    } else {
        t!("ajuste-seguridad-cortafuegos-editor-form-filtro-nuevo")
    };
    let revisar = move || {
        let valores = ctx.form.get();
        let p = match fid.clone() {
            None => PeticionCortafuegos::FiltroNuevo { valores },
            Some(filtro) => PeticionCortafuegos::FiltroCambiado {
                filtro,
                valores,
                antes: antes.clone(),
            },
        };
        ctx.enviar.run(p);
    };
    view! {
        <fieldset class="cortafuegos-form">
            <legend>{titulo}</legend>
            <div class="cortafuegos-form-campos">
                {texto(ctx, "name", Some(t!("ajuste-seguridad-cortafuegos-editor-ayuda-nombre")))}
                {lista(ctx, "interface", interfaces, false)}
                {lista(ctx, "type", sentidos, false)}
                {lista(ctx, "version", versiones, con_reglas)}
                {lista(ctx, "default", defectos, false)}
            </div>
            {con_reglas.then(|| view! { <p class="ayuda">{t!("ajuste-seguridad-cortafuegos-editor-version-fija")}</p> })}
            {botones_form(ctx, revisar)}
        </fieldset>
    }
}

/// «Revisar el cambio» y «Cancelar» de un formulario.
fn botones_form(ctx: Ctx, revisar: impl Fn() + 'static) -> impl IntoView {
    view! {
        <div class="botones">
            <button
                type="button"
                class="b-pri"
                aria-disabled=move || ctx.ocupado.get().then_some("true")
                on:click=move |_| {
                    if !ctx.ocupado.get_untracked() {
                        revisar();
                    }
                }
            >
                {t!("ajuste-seguridad-cortafuegos-editor-revisar")}
            </button>
            <button type="button" class="b-sec" on:click=move |_| ctx.edicion.set(None)>
                {t!("ajuste-seguridad-cortafuegos-editor-cancelar")}
            </button>
        </div>
    }
}

/// Una fila de la tabla de reglas de `f`.
fn fila(
    ctx: Ctx,
    e: &EditorCortafuegos,
    f: &FiltroEditor,
    n: usize,
    r: &ReglaEditor,
) -> impl IntoView + use<> {
    let i = idx(v(&f.valores, "version"));
    let val = &r.valores;
    let activa = if v(val, "enabled") == "1" {
        t!("ajuste-seguridad-cortafuegos-editor-si")
    } else {
        t!("ajuste-seguridad-cortafuegos-editor-no")
    };
    let no_reescribible =
        (!r.reescribible).then(|| t!("ajuste-seguridad-cortafuegos-editor-no-reescribible"));
    let motivo = |m: &Option<String>| m.clone().or_else(|| no_reescribible.clone());
    let (fid, version, rid) = (
        f.fid.clone(),
        v(&f.valores, "version").to_owned(),
        r.rid.clone(),
    );
    let antes = r.valores.clone();
    let cambiar = {
        let (fid, version, rid, antes) = (fid.clone(), version, rid.clone(), antes.clone());
        move || {
            ctx.abrir(
                Edicion::Regla {
                    fid: fid.clone(),
                    version: version.clone(),
                    rid: Some(rid.clone()),
                    antes: antes.clone(),
                },
                antes.clone(),
            );
        }
    };
    let borrar = move || {
        ctx.enviar.run(PeticionCortafuegos::ReglaBorrada {
            filtro: fid.clone(),
            regla: rid.clone(),
            antes: antes.clone(),
        });
    };
    view! {
        <tr class:cortafuegos-inactiva=v(val, "enabled") != "1">
            <td>{n.to_string()}</td>
            <td>{activa}</td>
            <td>{trafico(val, &e.icmp[i])}</td>
            <td>{accion(val, &e.rechazo[i])}</td>
            <td>{lado(val, "srcaddr", "srcmask")}</td>
            <td>{lado(val, "dstaddr", "dstmask")}</td>
            <td>{puertos(val)}</td>
            <td class="cortafuegos-acciones">
                {boton(ctx, t!("ajuste-seguridad-cortafuegos-editor-cambiar"), "b-sec", motivo(&e.bloqueos.regla_cambiar), cambiar)}
                {boton(ctx, t!("ajuste-seguridad-cortafuegos-editor-borrar"), "b-pel", motivo(&e.bloqueos.regla_borrar), borrar)}
            </td>
        </tr>
    }
}

/// Un filtro con sus reglas, sus botones y, si se edita algo suyo, el
/// formulario.
fn filtro(ctx: Ctx, e: StoredValue<EditorCortafuegos>, f: FiltroEditor) -> impl IntoView {
    let titulo_id = format!("cortafuegos-filtro-{}", f.fid);
    let etiquetado = titulo_id.clone();
    let nombre = v(&f.valores, "name").to_owned();
    let datos = datos_filtro(&f);
    let filas = e.with_value(|e| {
        f.reglas
            .iter()
            .enumerate()
            .map(|(n, r)| fila(ctx, e, &f, n + 1, r))
            .collect_view()
    });
    let vacio = f.reglas.is_empty();
    let region = t!(
        "ajuste-seguridad-cortafuegos-editor-reglas-de",
        filtro = nombre.clone()
    );
    let bloqueos = e.with_value(|e| e.bloqueos.clone());
    let no_reescribible =
        (!f.reescribible).then(|| t!("ajuste-seguridad-cortafuegos-editor-filtro-no-reescribible"));
    let motivo_cambiar = bloqueos.filtro_cambiar.clone().or(no_reescribible.clone());
    let motivo_borrar = bloqueos
        .filtro_borrar
        .clone()
        .or_else(|| (!vacio).then(|| t!("ajuste-seguridad-cortafuegos-editor-filtro-con-reglas")))
        .or(no_reescribible);
    let (fid, version) = (f.fid.clone(), v(&f.valores, "version").to_owned());
    let antes = f.valores.clone();
    let cambiar = {
        let (fid, antes) = (fid.clone(), antes.clone());
        move || {
            ctx.abrir(
                Edicion::Filtro {
                    fid: Some(fid.clone()),
                    con_reglas: !vacio,
                    antes: antes.clone(),
                },
                antes.clone(),
            );
        }
    };
    let borrar = {
        let (fid, antes) = (fid.clone(), antes.clone());
        move || {
            ctx.enviar.run(PeticionCortafuegos::FiltroBorrado {
                filtro: fid.clone(),
                antes: antes.clone(),
            });
        }
    };
    let nueva = {
        let (fid, version) = (fid.clone(), version.clone());
        move || {
            let base = [("enabled", "1"), ("protocol", "2"), ("specificAction", "2")]
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .into();
            ctx.abrir(
                Edicion::Regla {
                    fid: fid.clone(),
                    version: version.clone(),
                    rid: None,
                    antes: ValoresCortafuegos::new(),
                },
                base,
            );
        }
    };
    let mio = fid.clone();
    let formulario = move || {
        let ed = ctx.edicion.get()?;
        let vista = match ed {
            Edicion::Regla {
                fid,
                version,
                rid,
                antes,
            } if fid == mio => e
                .with_value(|e| form_regla(ctx, e, fid, version, rid, antes))
                .into_any(),
            Edicion::Filtro {
                fid: Some(fid),
                con_reglas,
                antes,
            } if fid == mio => e
                .with_value(|e| form_filtro(ctx, e, Some(fid), con_reglas, antes))
                .into_any(),
            _ => return None,
        };
        Some(vista)
    };
    view! {
        <section class="cortafuegos-filtro" aria-labelledby=etiquetado>
            <header class="cortafuegos-filtro-cabecera">
                <div>
                    <h4 id=titulo_id>{nombre}</h4>
                    <p class="cortafuegos-filtro-datos">{datos}</p>
                </div>
                <div class="botones">
                    {boton(ctx, t!("ajuste-seguridad-cortafuegos-editor-filtro-cambiar"), "b-sec", motivo_cambiar, cambiar)}
                    {boton(ctx, t!("ajuste-seguridad-cortafuegos-editor-filtro-borrar"), "b-pel", motivo_borrar, borrar)}
                </div>
            </header>
            {if vacio {
                view! { <p class="ayuda">{t!("ajuste-seguridad-cortafuegos-editor-vacio")}</p> }.into_any()
            } else {
                view! {
                    <div class="tabla-scroll" tabindex="0" role="region" aria-label=region>
                        <table class="tabla cortafuegos-tabla">
                            <thead>
                                <tr>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-orden")}</th>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-activa")}</th>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-trafico")}</th>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-accion")}</th>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-desde")}</th>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-hacia")}</th>
                                    <th scope="col">{t!("ajuste-seguridad-cortafuegos-editor-col-puertos")}</th>
                                    <th scope="col">
                                        <span class="solo-lector">{t!("ajuste-seguridad-cortafuegos-editor-col-acciones")}</span>
                                    </th>
                                </tr>
                            </thead>
                            <tbody>{filas}</tbody>
                        </table>
                    </div>
                }
                .into_any()
            }}
            <div class="botones">
                {boton(ctx, t!("ajuste-seguridad-cortafuegos-editor-regla-nueva"), "b-sec", bloqueos.regla_nueva.clone(), nueva)}
            </div>
            {formulario}
        </section>
    }
}

/// El editor ya leído.
fn editor(ctx: Ctx, e: EditorCortafuegos) -> impl IntoView {
    // Los motivos generales (lo no comprobado), una vez cada uno.
    let mut motivos: Vec<String> = [
        &e.bloqueos.regla_nueva,
        &e.bloqueos.regla_cambiar,
        &e.bloqueos.regla_borrar,
        &e.bloqueos.filtro_nuevo,
        &e.bloqueos.filtro_cambiar,
        &e.bloqueos.filtro_borrar,
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect();
    motivos.dedup();
    let motivo_nuevo = e.bloqueos.filtro_nuevo.clone();
    let filtros = e.filtros.clone();
    let e = StoredValue::new(e);
    let filtros = filtros
        .into_iter()
        .map(|f| filtro(ctx, e, f))
        .collect_view();
    let nuevo = move || {
        let base = [("type", "0"), ("version", "0"), ("default", "0")]
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .into();
        ctx.abrir(
            Edicion::Filtro {
                fid: None,
                con_reglas: false,
                antes: ValoresCortafuegos::new(),
            },
            base,
        );
    };
    let form_nuevo = move || match ctx.edicion.get() {
        Some(Edicion::Filtro {
            fid: None,
            con_reglas,
            antes,
        }) => Some(e.with_value(|e| form_filtro(ctx, e, None, con_reglas, antes))),
        _ => None,
    };
    view! {
        <div class="cortafuegos-editor">
            <p class="ayuda">{t!("ajuste-seguridad-cortafuegos-editor-ayuda")}</p>
            {(!motivos.is_empty())
                .then(|| view! { <p class="nota-solo-lectura" id=ID_MOTIVO>{motivos.join(" ")}</p> })}
            <p class="error-formulario" role="alert">{move || ctx.error.get()}</p>
            {filtros}
            <div class="botones">
                {boton(ctx, t!("ajuste-seguridad-cortafuegos-editor-filtro-nuevo"), "b-sec", motivo_nuevo, nuevo)}
            </div>
            {form_nuevo}
        </div>
    }
}

/// El editor del cortafuegos del ajuste `a` (su tabla, mientras se lee o si
/// no se puede), dentro del detalle del elemento `elemento`.
#[component]
pub(super) fn Editor(a: AjusteVista, elemento: String) -> impl IntoView {
    let pagina = expect_context::<Pagina>();
    let s = expect_context::<Seguimiento>();
    let datos = Resource::new(|| (), |_| leer_cortafuegos());
    let edicion = RwSignal::new(None::<Edicion>);
    let error = RwSignal::new(None::<String>);
    let ocupado = RwSignal::new(false);
    let elemento = StoredValue::new(elemento);
    let enviar = Callback::new(move |p: PeticionCortafuegos| {
        if ocupado.get_untracked() {
            return;
        }
        error.set(None);
        ocupado.set(true);
        spawn_local(async move {
            match preparar_cortafuegos(p).await {
                Ok(g) => {
                    edicion.try_set(None);
                    s.revisar(g, vec![elemento.get_value()]);
                }
                Err(e) => {
                    error.try_set(Some(texto_error(&e)));
                }
            }
            ocupado.try_set(false);
        });
    });
    let ctx = Ctx {
        edicion,
        form: RwSignal::new(ValoresCortafuegos::new()),
        error,
        ocupado,
        enviar,
        solo_lectura: pagina.solo_lectura,
    };
    let a = StoredValue::new(a);
    let tabla = move || a.with_value(|a| consulta::lista(a).into_any());
    view! {
        <Suspense fallback=tabla>
            {move || {
                datos
                    .get()
                    .map(|r| match r {
                        Ok(e) => editor(ctx, e).into_any(),
                        Err(_) => {
                            view! {
                                {tabla()}
                                <p class="nota">{t!("ajuste-seguridad-cortafuegos-editor-sin-leer")}</p>
                            }
                                .into_any()
                        }
                    })
            }}
        </Suspense>
    }
}
