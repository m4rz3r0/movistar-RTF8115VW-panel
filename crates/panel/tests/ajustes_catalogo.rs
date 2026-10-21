//! El catálogo de ajustes frente al gestor de cambios: lo que el catálogo
//! deja editar, el gestor lo admite (no lo bloquea por la IP LAN, por los
//! secretos de la web básica ni por otra regla de la v1).

mod comun;

use ajustes::{Campo, CatalogoAjustes};
use comun::PanelPrueba;
use panel::changes::{ChangeManager, Tiempos};

#[tokio::test]
async fn ajustes_escribibles_por_el_gestor() {
    let p = PanelPrueba::nuevo().sin_router().arrancar().await;
    let m = ChangeManager::new(p.state.clone(), Tiempos::default());
    let catalogo = CatalogoAjustes::load().unwrap();
    let mut campos = 0;
    let mut fallos = Vec::new();
    for a in catalogo.todos() {
        for (c, e) in a
            .campos
            .iter()
            .filter_map(|c| Campo::escribible(c).map(|e| (c, e)))
        {
            campos += 1;
            if let Err(err) = m.escribible(&e.funcion) {
                fallos.push(format!("{}.{} ({}): {err}", a.id, c.id, e.funcion));
            }
        }
    }
    assert!(campos > 0, "ningún campo editable en el catálogo");
    assert!(fallos.is_empty(), "el gestor no admite: {fallos:#?}");
}
