//! Reglas de puertos: partes de una regla y su comprobación con el router.

use super::lectura::lista;
use super::red_local::sin_leer;

use super::*;

/// Peticiones en que se parte `p`: una regla de puertos nueva con
/// «Ambos» ([`AMBOS`]) son dos, una TCP y una UDP, porque el router
/// admite un solo protocolo por regla; van en el mismo grupo (un
/// diálogo, un deshacer). El resto, `p` sola.
pub(crate) fn partes(a: &Ajuste, p: &PeticionAjuste) -> Vec<PeticionAjuste> {
    let ambos = p.valores.get("protocolo").map(|v| v.trim()) == Some(AMBOS);
    if a.componente != Some(Componente::Puertos) || p.borrar || !ambos {
        return vec![p.clone()];
    }
    Protocolo::Ambos
        .claves()
        .iter()
        .map(|clave| {
            let mut q = p.clone();
            q.valores.insert("protocolo".into(), (*clave).to_owned());
            q
        })
        .collect()
}

/// Comprueba una regla de puertos de `p` con lo que tiene ahora el
/// router y devuelve su nombre (para el resumen).
/// - «Ambos» solo vale al crear (400).
/// - Al editar o borrar, la regla tiene que existir (400), la página
///   tiene que decir lo que la identifica ([`IDENTIFICAN_REGLA`] en
///   `esperados`; si falta algo, 400) y tiene que coincidir (si no, 409:
///   otra sesión la cambió).
/// - Si hay que validarla ([`valida_la_regla`]), como la página
///   ([`validar_regla`]), con la red y las reglas del router; lo que no
///   se cambia es lo de la regla. Una nueva necesita todos sus campos.
///   Error 400 junto al campo. Los formatos ya están validados.
pub(super) async fn comprobar_regla(
    state: &AppState,
    a: &Ajuste,
    p: &PeticionAjuste,
) -> Result<Option<String>, ServerFnError> {
    let prefijo = prefijo_i18n(a);
    let invalida = |motivo: &str| {
        tracing::warn!("ajustes: {}: {motivo}", a.id);
        fallo(StatusCode::BAD_REQUEST, t!("ajustes-error-invalida"))
    };
    let ambos = p.valores.get("protocolo").map(|v| v.trim()) == Some(AMBOS);
    if ambos && p.instancia.is_some() {
        return Err(invalida("«ambos» solo al crear una regla"));
    }
    let params: Vec<(&str, &str)> = a
        .leer
        .params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let n = state
        .hgu
        .call(&a.leer.funcion, &params)
        .await
        .map_err(|e| sin_leer("las reglas de puertos", e))?;
    let instancias = lista(&state.catalogo, a, &prefijo, &n).ok_or_else(|| {
        tracing::warn!("ajustes: {}: reglas sin su id", a.id);
        fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
    })?;
    let actual = match &p.instancia {
        None => None,
        Some(id) => Some(
            instancias
                .iter()
                .find(|i| i.id == *id)
                .ok_or_else(|| invalida("la regla no existe"))?,
        ),
    };
    // Al editar o borrar, lo que identifica la regla es obligatorio:
    // sin ello no se sabe si sigue siendo la misma.
    if let Some(i) = actual {
        for campo in IDENTIFICAN_REGLA {
            let esperado = p
                .esperados
                .get(campo)
                .ok_or_else(|| invalida("editar o borrar sin lo que identifica la regla"))?;
            if esperado.trim() != valor_de(&i.campos, campo) {
                return Err(fallo(
                    StatusCode::CONFLICT,
                    t!("ajuste-puertos-reglas-error-cambiada"),
                ));
            }
        }
    }
    let valor = |campo: &str| {
        p.valores
            .get(campo)
            .filter(|_| !p.borrar)
            .map(|v| v.trim().to_owned())
            .or_else(|| actual.map(|i| valor_de(&i.campos, campo)))
            .filter(|v| !v.is_empty())
    };
    let detalle = valor("nombre");
    if !valida_la_regla(p) {
        return Ok(detalle);
    }
    let campo = |id: &str| a.campo(id).expect("campo de la regla");
    let exigir =
        |id: &str| valor(id).ok_or_else(|| error_valor(&prefijo, campo(id), &ErrorValor::Vacio));
    let puerto = |id: &str| {
        exigir(id)?.parse::<u16>().map_err(|_| {
            let c = campo(id);
            let (min, max) = (c.validar.and_then(|v| v.min), c.validar.and_then(|v| v.max));
            error_valor(&prefijo, c, &ErrorValor::FueraDeRango { min, max })
        })
    };
    exigir("nombre")?;
    let regla = ReglaPuertos {
        id: p.instancia.clone(),
        nombre: detalle.clone().unwrap_or_default(),
        protocolo: Protocolo::de_clave(&exigir("protocolo")?)
            .ok_or_else(|| error_valor(&prefijo, campo("protocolo"), &ErrorValor::Opcion))?,
        externo: (puerto("externo_desde")?, puerto("externo_hasta")?),
        interno: (puerto("interno_desde")?, puerto("interno_hasta")?),
        ip: exigir("destino")?
            .parse()
            .map_err(|_| error_valor(&prefijo, campo("destino"), &ErrorValor::Ip))?,
        activa: valor("activa").as_deref() == Some("true"),
    };
    let existentes: Vec<ReglaPuertos> = instancias.iter().filter_map(regla_de).collect();
    let lan = hgu::read::lan_ipv4(&state.hgu)
        .await
        .map_err(|e| sin_leer("la red de casa", e))?
        .ok_or_else(|| {
            tracing::warn!("ajustes: el router no da su IP en la red de casa");
            fallo(StatusCode::BAD_GATEWAY, t!("ajustes-error-lectura"))
        })?;
    validar_regla(&regla, &existentes, lan).map_err(|e| {
        let etiqueta = crate::ui::i18n::traducir(&format!("{prefijo}-{}", e.campo()), None);
        fallo(
            StatusCode::BAD_REQUEST,
            format!("{etiqueta}: {}", e.texto(lan.0)),
        )
    })?;
    Ok(detalle)
}
