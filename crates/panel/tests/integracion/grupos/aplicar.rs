//! Aplicar grupos de pasos: orden, pasos omitidos, acciones y migración.

use super::*;
use crate::comun::ConfirmarLigado;

#[tokio::test]
async fn grupo_aplica_en_orden_y_verifica() {
    let (p, m) = preparado().await;
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), lease0("192.168.1.80")]))
        .await
        .unwrap();
    assert_eq!(prep.riesgo, Riesgo::Bajo);
    assert_eq!(prep.pasos.len(), 2);
    assert_eq!(prep.pasos[0].indice, 0);
    assert_eq!(prep.pasos[0].funcion, UPNP);
    assert_eq!(prep.pasos[0].etiqueta.as_deref(), Some("ajuste-upnp"));
    assert!(prep.pasos.iter().all(|p| !p.sin_cambios));
    assert_eq!(prep.pasos[1].diff.campos[0].despues, "192.168.1.80");

    let id = m
        .confirmar_grupo_ligado(&prep.token, &[], "admin")
        .await
        .unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(escritas(&p), [UPNP, LEASES]);
    assert_eq!(upnp_actual(&p).await, "1");
    assert_eq!(ip_de_lease0(&p).await, "192.168.1.80");
    assert_eq!(
        estados_pasos(&c),
        [EstadoPaso::Aplicado, EstadoPaso::Aplicado]
    );
    assert_eq!(c.pasos[0].orden, 0);
    assert_eq!(c.pasos[1].funcion, LEASES);
    assert_eq!(c.pasos[1].instancia.as_deref(), Some("0"));
    assert_eq!(c.pasos[1].etiqueta.as_deref(), Some("ajuste-reserva"));
    assert_eq!(c.titulo.as_deref(), Some("ajuste-prueba"));
    // El diff del grupo junta los de los pasos.
    assert_eq!(c.diff.campos.len(), 2);
    esperar_libre_gestor(&m).await;
}

#[tokio::test]
async fn pasos_sin_cambio_y_desmarcados_se_omiten() {
    let (p, m) = preparado().await;
    // Ya está así.
    let r = m
        .preparar_grupo(grupo(vec![upnp("0"), lease0("192.168.1.68")]))
        .await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");

    let prep = m
        .preparar_grupo(grupo(vec![upnp("0"), lease0("192.168.1.80"), ssid("OTRA")]))
        .await
        .unwrap();
    assert_eq!(
        prep.pasos.iter().map(|p| p.sin_cambios).collect::<Vec<_>>(),
        [true, false, false]
    );
    assert!(prep.pasos[0].diff.campos.is_empty());
    // Sin el paso de riesgo medio (desmarcado) el riesgo no cambia aquí: es
    // el de lo preparado.
    let id = m
        .confirmar_grupo_ligado(&prep.token, &[2], "admin")
        .await
        .unwrap();
    let c = esperar_estado(&m, id, Estado::Aplicado).await;
    assert_eq!(escritas(&p), [LEASES]);
    assert_eq!(ssid_actual(&p).await, "RED-1");
    assert_eq!(
        estados_pasos(&c),
        [
            EstadoPaso::Omitido,
            EstadoPaso::Aplicado,
            EstadoPaso::Omitido
        ]
    );
    esperar_libre_gestor(&m).await;

    // Todo desmarcado: nada que hacer, nada guardado.
    let prep = m
        .preparar_grupo(grupo(vec![upnp("1"), ssid("OTRA")]))
        .await
        .unwrap();
    let r = m
        .confirmar_grupo_ligado(&prep.token, &[0, 1], "admin")
        .await;
    assert!(matches!(r, Err(CambioError::SinCambios)), "{r:?}");
    assert_eq!(m.historial(10).await.len(), 1);
    assert_eq!(escritas(&p).len(), 1);
}

/// Un grupo no admite acciones: no se podrían revertir si falla otro paso.
#[tokio::test]
async fn acciones_solo_en_grupos_de_un_paso() {
    let (_p, m) = preparado().await;
    let r = m
        .preparar_grupo(grupo(vec![
            upnp("1"),
            paso(req("cbReboot", None, &[]), "ajuste-reiniciar"),
        ]))
        .await;
    assert!(matches!(r, Err(CambioError::Invalida(_))), "{r:?}");
    // Y un grupo con una acción necesita la palabra (va por `confirmar`).
    let prep = m
        .preparar_grupo(grupo(vec![paso(
            req("cbReboot", None, &[]),
            "ajuste-reiniciar",
        )]))
        .await
        .unwrap();
    let r = m.confirmar_grupo_ligado(&prep.token, &[], "admin").await;
    assert!(matches!(r, Err(CambioError::PalabraIncorrecta)), "{r:?}");
}

/// Las filas de antes de los grupos pasan a ser grupos de un paso.
#[tokio::test]
async fn migracion_pasa_cambios_antiguos_a_un_paso() {
    use sqlx::{
        Row,
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    };
    let dir = tempfile::tempdir().unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(dir.path().join("p.db"))
                .create_if_missing(true)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let todas = sqlx::migrate!("./migrations");
    let mut previas = sqlx::migrate!("./migrations");
    previas.migrations = std::borrow::Cow::Owned(
        todas
            .migrations
            .iter()
            .filter(|m| m.version < 10)
            .cloned()
            .collect(),
    );
    previas.run(&pool).await.unwrap();
    for (id, estado) in [(1, "aplicado"), (2, "vigilando")] {
        sqlx::query(
            "INSERT INTO changes (id, ts, updated, funcion, instancia, riesgo, estado, diff, \
             anteriores, anteriores_cifrados, operacion, solicitud, nuevos, ids_antes) \
             VALUES (?, 1, 1, 'cbDhcpV4SvrPoolSetStatic', '0', 'bajo', ?, \
             '[{\"nombre\":\"ipv4_addr\",\"antes\":\"a\",\"despues\":\"b\",\"secreto\":false}]', \
             '{\"ipv4_addr\":\"a\"}', X'0102', 'editar', '{\"x\":\"1\"}', \
             '{\"ipv4_addr\":\"b\"}', '[\"3\"]')",
        )
        .bind(id)
        .bind(estado)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE changes SET deshecho_por = 2 WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    todas.run(&pool).await.unwrap();
    let deshecho: Option<i64> = sqlx::query_scalar("SELECT deshecho_por FROM changes WHERE id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(deshecho, Some(2), "se conserva el enlace de deshacer");
    let filas = sqlx::query("SELECT * FROM cambio_pasos ORDER BY cambio_id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(filas.len(), 2);
    let f = &filas[0];
    assert_eq!(f.get::<i64, _>("orden"), 0);
    assert_eq!(f.get::<String, _>("funcion"), LEASES);
    assert_eq!(
        f.get::<Option<String>, _>("instancia").as_deref(),
        Some("0")
    );
    assert_eq!(f.get::<String, _>("estado"), "aplicado");
    assert_eq!(
        f.get::<Vec<u8>, _>("valores_anteriores"),
        br#"{"ipv4_addr":"a"}"#
    );
    assert_eq!(f.get::<Vec<u8>, _>("valores_anteriores_cifrados"), [1, 2]);
    assert_eq!(
        f.get::<Vec<u8>, _>("valores_nuevos"),
        br#"{"ipv4_addr":"b"}"#
    );
    assert_eq!(
        f.get::<Option<String>, _>("ids_antes").as_deref(),
        Some(r#"["3"]"#)
    );
    // Un cambio en vigilancia ya escribió y verificó su paso.
    assert_eq!(filas[1].get::<String, _>("estado"), "aplicado");
    // El nuevo estado del grupo es válido.
    sqlx::query("UPDATE changes SET estado = 'revertido_parcial' WHERE id = 2")
        .execute(&pool)
        .await
        .unwrap();
}
