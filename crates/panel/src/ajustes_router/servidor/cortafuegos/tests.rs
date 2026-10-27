//! Pruebas del cortafuegos de la tarjeta: el ping por versión, lo pendiente (solo lo abierto a cualquiera) y la regla del panel.

use super::*;

fn filtros() -> Vec<Filtro> {
    cf::analizar(include_str!(
        "../../../../../hgu/fixtures/paginas/scbidirectionalflt_ls.asp"
    ))
    .unwrap()
}

/// La regla del panel en el filtro IPv4 de la copia saneada.
fn con_regla(mut f: Vec<Filtro>) -> Vec<Filtro> {
    let mut r = f[0].reglas[8].clone();
    (r.rid, r.action, r.icmp_type) = ("16".into(), "0".into(), "25".into());
    f[0].reglas.push(r);
    f
}

#[test]
fn el_ping_por_version() {
    let f = filtros();
    let p = ping(&f);
    assert_eq!((p.v4, p.v6, p.regla), (true, true, false));
    assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-ping");
    // Con la regla, la del operador sigue dejando pasar el ping de IPv4
    // (la del panel va detrás): la tarjeta nunca dice que no responde, y el
    // motivo y lo pendiente dicen que la regla está puesta pero no basta.
    let con = con_regla(f.clone());
    let p = ping(&con);
    assert_eq!((p.v4, p.v6, p.regla), (true, true, true));
    assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-ping");
    assert!(motivo_regla(&con).unwrap().contains("sigue respondiendo"));
    let mut sin_v6 = con_regla(f.clone());
    sin_v6[1].reglas.retain(|r| r.icmp_type != "55");
    let p = ping(&sin_v6);
    assert_eq!((p.v4, p.v6, p.regla), (true, false, true));
    assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-ping-ipv4");
    // Sin ninguna otra regla que abra el ping, solo «regla puesta», sin
    // afirmar su efecto.
    let mut solo_regla = con_regla(f);
    solo_regla[1].reglas.retain(|r| r.icmp_type != "55");
    solo_regla[0].reglas.remove(8);
    let p = ping(&solo_regla);
    assert_eq!((p.v4, p.v6, p.regla), (false, false, true));
    assert_eq!(estado(p).1, "ajustes-estado-cortafuegos-regla-puesta");
}

/// Con la regla del panel puesta y activa y el ping de IPv4 aún abierto por
/// el operador, lo pendiente lo dice y la recomendación no vuelve a
/// ofrecer la regla: queda pedírselo al operador.
#[test]
fn con_la_regla_puesta_el_ping_ipv4_sigue_pendiente() {
    let mut f = con_regla(filtros());
    f[1].reglas.retain(|r| r.icmp_type != "55");
    let p = ping(&f);
    let pr = problema(&f, p);
    assert_eq!(pr.len(), 1, "{pr:?}");
    assert!(pr[0].contains("otra regla del operador"), "{pr:?}");
    let t = texto(&f, p, &pr, true);
    assert!(!t.contains("pase más desapercibido"), "{t}");
    assert!(!t.contains("desactivada"), "{t}");
    assert!(t.contains("operador"), "{t}");
}

/// Una regla con la huella pero desactivada no cuenta para el estado
/// (el ping de IPv4 sigue abierto), aunque sí impide otra alta.
#[test]
fn la_regla_desactivada_no_cuenta() {
    let mut f = con_regla(filtros());
    f[0].reglas.last_mut().unwrap().enable = "0".into();
    let p = ping(&f);
    assert_eq!((p.v4, p.regla), (true, false));
    assert!(hay_regla(&f));
}

/// El ping de las reglas 8 (IPv4) y 13 (IPv6) solo desde redes
/// concretas.
fn ping_de_redes(f: &mut [Filtro], v4: bool, v6: bool) {
    if v4 {
        f[0].reglas[8].orig_addr = "192.0.2.1".into();
    }
    if v6 {
        let r = f[1].reglas.iter_mut().find(|r| r.icmp_type == "55");
        r.unwrap().orig_addr = "2001:db8::1".into();
    }
}

/// Solo es pendiente lo abierto a cualquiera; la web, el SSH
/// y la gestión del operador de serie (solo desde sus redes) son un
/// dato, no un problema.
#[test]
fn solo_lo_abierto_a_cualquiera_es_pendiente() {
    let mut f = filtros();
    let pr = problema(&f, ping(&f));
    assert_eq!(pr.len(), 1, "{pr:?}");
    assert!(pr[0].contains("responde al ping"), "{pr:?}");
    assert_eq!(
        restringidos(&f),
        [
            "la web del router",
            "el SSH",
            "la gestión remota del operador"
        ]
    );
    ping_de_redes(&mut f, true, true);
    assert!(problema(&f, ping(&f)).is_empty());
    assert_eq!(restringidos(&f)[0], "el ping");
}

/// El ping de IPv6 solo desde redes concretas no se pierde porque el de
/// IPv4 esté abierto a cualquiera.
#[test]
fn el_ping_restringido_por_version() {
    let mut f = filtros();
    ping_de_redes(&mut f, false, true);
    let p = ping(&f);
    assert_eq!((p.v4, p.v6), (true, false));
    assert_eq!(restringidos(&f)[0], "el ping por IPv6");
}

/// Con la regla del panel desactivada, su motivo propio y sin ofrecer
/// ponerla en el texto.
#[test]
fn la_regla_desactivada_tiene_su_motivo() {
    let mut f = con_regla(filtros());
    f[0].reglas.last_mut().unwrap().enable = "0".into();
    let m = motivo_regla(&f).unwrap();
    assert!(m.contains("desactivada"), "{m}");
    let p = ping(&f);
    let t = texto(&f, p, &problema(&f, p), true);
    assert!(!t.contains("pase más desapercibido"), "{t}");
    // Activa y con el ping del operador abierto, que no basta; sin él,
    // «ya puesta».
    let f = con_regla(filtros());
    assert!(motivo_regla(&f).unwrap().contains("sigue respondiendo"));
    let mut solo_regla = con_regla(filtros());
    solo_regla[0].reglas.remove(8);
    assert!(motivo_regla(&solo_regla).unwrap().contains("ya puesta"));
    assert!(motivo_regla(&filtros()).is_none());
}

/// Con la regla del panel desactivada y solo el ping de IPv4 pendiente, la
/// recomendación no sale vacía: dice que se active, con sus pros y
/// contras.
#[test]
fn regla_desactivada_y_solo_ping_ipv4_con_texto() {
    let mut f = con_regla(filtros());
    f[0].reglas.last_mut().unwrap().enable = "0".into();
    f[1].reglas.retain(|r| r.icmp_type != "55");
    let p = ping(&f);
    assert_eq!((p.v4, p.v6), (true, false));
    let pr = problema(&f, p);
    assert_eq!(pr.len(), 1, "{pr:?}");
    let t = texto(&f, p, &pr, true);
    assert!(t.starts_with("Recomendado si"), "{t}");
    assert!(t.contains("desactivada"), "{t}");
    assert!(t.contains("A favor:") && t.contains("En contra:"), "{t}");
}

/// Con la regla del panel puesta, el ping de IPv4 desde redes concretas
/// sigue saliendo como abierto: la regla va detrás y no lo quita.
#[test]
fn con_la_regla_puesta_se_lista_el_ping_ipv4_restringido() {
    let mut f = con_regla(filtros());
    ping_de_redes(&mut f, true, false);
    assert!(puesta(&f));
    let r = restringidos(&f);
    assert!(r.iter().any(|s| s.contains("ping por IPv4")), "{r:?}");
}

/// Sin «sin ping» en este router, la recomendación no ofrece la regla del
/// panel: lo pendiente, ping incluido, es pedir al operador que lo cierre.
#[test]
fn sin_la_funcion_no_ofrece_la_regla() {
    let f = filtros();
    let p = ping(&f);
    assert!(p.v4);
    let t = texto(&f, p, &problema(&f, p), false);
    assert!(!t.contains("pase más desapercibido"), "{t}");
    assert!(t.contains("operador"), "{t}");
    let mut f = con_regla(filtros());
    f[0].reglas.last_mut().unwrap().enable = "0".into();
    f[1].reglas.retain(|r| r.icmp_type != "55");
    let p = ping(&f);
    let t = texto(&f, p, &problema(&f, p), false);
    assert!(!t.contains("desactivada"), "{t}");
    assert!(t.contains("operador"), "{t}");
}
