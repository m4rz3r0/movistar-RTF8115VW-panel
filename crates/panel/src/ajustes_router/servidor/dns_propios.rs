//! DNS propios en la red de casa: valen como lo recomendado en su tarjeta.

use super::*;
use ajustes::CatalogoAjustes;
use std::net::Ipv4Addr;

/// En cada campo con resolutores del operador (`del_operador`), un valor
/// propio ([`ajustes::dns_propio`]) se acepta como el recomendado: la
/// tarjeta no recomienda cambiarlo. Sin la IP del router no se acepta nada.
pub(super) fn aceptar(
    cat: &CatalogoAjustes,
    vistas: &mut [AjusteVista],
    ip_router: Option<Ipv4Addr>,
) {
    for v in vistas.iter_mut() {
        let Some(a) = cat.ajuste(&v.id) else {
            continue;
        };
        for c in &mut v.campos {
            let ValorLegible::Texto(t) = &c.valor else {
                continue;
            };
            let propio = a
                .campo(&c.id)
                .and_then(|def| ajustes::dns_propio(def, t, ip_router));
            if propio == Some(true) && !c.aceptados.contains(t) {
                c.aceptados.push(t.clone());
            }
        }
    }
}
