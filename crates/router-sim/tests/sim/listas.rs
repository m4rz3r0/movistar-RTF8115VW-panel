//! Listas genéricas: crear, editar, borrar e ids de instancia.

use super::*;

fn ids_nat(n: &XmlNode) -> Vec<(String, String)> {
    n.all("rtm_cfg_nat_port_mapping")
        .into_iter()
        .map(|e| {
            (
                e.text_of("nat_port_mapping_id").unwrap().to_owned(),
                e.text_of("desc").unwrap().to_owned(),
            )
        })
        .collect()
}

#[tokio::test]
async fn lista_crear_editar_borrar() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let todas = [("all", "ALL")];
    let inicial = ids_nat(&c.call("cbGetNatPortMapping", &todas).await.unwrap());
    assert_eq!(inicial.len(), 1);
    let id0 = inicial[0].0.clone();

    // Crear: sin id.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("desc", "nueva"), ("int_ip_addr", "192.0.2.10")],
    )
    .await
    .unwrap();
    let tras = c.call("cbGetNatPortMapping", &todas).await.unwrap();
    let ids = ids_nat(&tras);
    assert_eq!(ids.len(), 2);
    let nuevo = ids.iter().find(|(_, d)| d == "nueva").unwrap().0.clone();
    assert_eq!(
        nuevo,
        (id0.parse::<u32>().unwrap() + 1).to_string(),
        "max + 1"
    );
    let por_indice = c
        .call("cbGetNatPortMapping", &[("nat_port_mapping_0i", "1")])
        .await
        .unwrap();
    assert_eq!(
        valor(&por_indice, "int_ip_addr").as_deref(),
        Some("192.0.2.10")
    );

    // Editar la existente.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("nat_port_mapping_id", &id0), ("desc", "editada")],
    )
    .await
    .unwrap();
    let ids = ids_nat(&c.call("cbGetNatPortMapping", &todas).await.unwrap());
    assert_eq!(ids.len(), 2, "editar no crea");
    assert!(ids.contains(&(id0.clone(), "editada".into())));

    // Borrar la primera.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("nat_port_mapping_id", &id0), ("del", "DEL")],
    )
    .await
    .unwrap();
    let ids = ids_nat(&c.call("cbGetNatPortMapping", &todas).await.unwrap());
    assert_eq!(ids, vec![(nuevo.clone(), "nueva".into())]);
    let r = c
        .call("cbGetNatPortMapping", &[("nat_port_mapping_id", &id0)])
        .await;
    assert!(r.is_err(), "una instancia borrada ya no se lee");
}

const NAT: &str = "rtm_cfg_nat_port_mapping";

/// Crea una redirección con ese `desc` (sin id) y devuelve el id asignado.
async fn crear_nat(c: &Client, desc: &str) -> String {
    c.write("natportmap.asp", "cbSetNatPortMapping", &[("desc", desc)])
        .await
        .unwrap();
    let todas = c
        .call("cbGetNatPortMapping", &[("all", "ALL")])
        .await
        .unwrap();
    todas
        .all(NAT)
        .into_iter()
        .find(|e| e.text_of("desc") == Some(desc))
        .and_then(|e| e.text_of("nat_port_mapping_id"))
        .unwrap()
        .to_owned()
}

/// `…_0i` es la posición en la lista, como en el router real: al borrar
/// una del medio, las de detrás se adelantan, y la posición ya no es el id.
#[tokio::test]
async fn lectura_por_0i_es_la_posicion_tras_borrar_en_medio() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    let leer = |posicion: &'static str| {
        let c = c.clone();
        async move {
            c.call("cbGetNatPortMapping", &[("nat_port_mapping_0i", posicion)])
                .await
        }
    };
    let primera = crear_nat(&c, "primera").await;
    let segunda = crear_nat(&c, "segunda").await;
    let tercera = crear_nat(&c, "tercera").await;
    let mut ids = [primera.clone(), segunda.clone(), tercera.clone()];
    ids.sort();
    assert_eq!(ids.len(), 3);
    // Borra la del medio y edita la tercera.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("nat_port_mapping_id", &segunda), ("del", "DEL")],
    )
    .await
    .unwrap();
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[
            ("nat_port_mapping_id", &tercera),
            ("desc", "tercera-editada"),
        ],
    )
    .await
    .unwrap();
    // Detrás de la de las fixtures (posición 0) van la primera y la tercera.
    let n = leer("1").await.unwrap();
    assert_eq!(campo_de(&n, NAT, "desc").as_deref(), Some("primera"));
    assert_eq!(
        campo_de(&n, NAT, "nat_port_mapping_0i").as_deref(),
        Some("1")
    );
    let n = leer("2").await.unwrap();
    assert_eq!(
        campo_de(&n, NAT, "nat_port_mapping_id"),
        Some(tercera.clone())
    );
    assert_eq!(
        campo_de(&n, NAT, "desc").as_deref(),
        Some("tercera-editada")
    );
    assert!(leer("3").await.is_err(), "no hay cuarta posición");
    // El 0i de la instancia de las fixtures no cambia; el de las nuevas es su id.
    let todas = c
        .call("cbGetNatPortMapping", &[("all", "ALL")])
        .await
        .unwrap();
    let indices: Vec<(String, String)> = todas
        .all(NAT)
        .into_iter()
        .map(|e| {
            (
                e.text_of("nat_port_mapping_id").unwrap().to_owned(),
                e.text_of("nat_port_mapping_0i").unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        indices,
        vec![
            ("0".into(), "0".into()),
            (primera.clone(), primera),
            (tercera.clone(), tercera)
        ]
    );
}

#[tokio::test]
async fn edicion_sin_instancia_solo_toca_los_hijos_directos() {
    let capa = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("capa-ntp-anidado");
    std::fs::create_dir_all(&capa).unwrap();
    std::fs::write(
        capa.join("cbGetTimeNtp.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\r\n<Response><cbGetTimeNtp><rtm_time_ntp>\
         <ntp_svr_1>a.example</ntp_svr_1><extra><ntp_svr_1>b.example</ntp_svr_1></extra>\
         </rtm_time_ntp><Result>0</Result></cbGetTimeNtp></Response>",
    )
    .unwrap();
    let sim = Sim::start(SimConfig::new(USER, PASS, fixtures_dir()).con_capa(&capa)).await;
    let c = client(&sim, Duration::from_secs(5));
    c.write("ntp.asp", "cbSetTimeNtp", &[("ntp_svr_1", "c.example")])
        .await
        .unwrap();
    let n = c.call("cbGetTimeNtp", &[]).await.unwrap();
    let ntp = n.get("rtm_time_ntp").unwrap();
    assert_eq!(ntp.text_of("ntp_svr_1"), Some("c.example"));
    assert_eq!(
        ntp.get("extra").unwrap().text_of("ntp_svr_1"),
        Some("b.example"),
        "el nodo anidado no se toca"
    );
}

#[tokio::test]
async fn id_que_no_existe_crea_con_max_mas_uno() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("nat_port_mapping_id", "42"), ("desc", "con-id-ajeno")],
    )
    .await
    .unwrap();
    let todas = c
        .call("cbGetNatPortMapping", &[("all", "ALL")])
        .await
        .unwrap();
    let ids: Vec<(&str, &str)> = todas
        .all(NAT)
        .into_iter()
        .map(|e| {
            (
                e.text_of("nat_port_mapping_id").unwrap(),
                e.text_of("desc").unwrap(),
            )
        })
        .collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[1], ("1", "con-id-ajeno"), "id max + 1, no el escrito");
}

#[tokio::test]
async fn creacion_doble_en_la_lista_generica() {
    let sim = start().await;
    let c = client(&sim, Duration::from_secs(5));
    sim.control.create_twice_next_write();
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("desc", "doble")],
    )
    .await
    .unwrap();
    let todas = c
        .call("cbGetNatPortMapping", &[("all", "ALL")])
        .await
        .unwrap();
    let dobles: Vec<&str> = todas
        .all(NAT)
        .into_iter()
        .filter(|e| e.text_of("desc") == Some("doble"))
        .map(|e| e.text_of("nat_port_mapping_id").unwrap())
        .collect();
    assert_eq!(dobles, ["1", "2"]);
    // Solo la próxima creación.
    c.write(
        "natportmap.asp",
        "cbSetNatPortMapping",
        &[("desc", "simple")],
    )
    .await
    .unwrap();
    let todas = c
        .call("cbGetNatPortMapping", &[("all", "ALL")])
        .await
        .unwrap();
    assert_eq!(todas.all(NAT).len(), 4);
}
