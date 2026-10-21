//! Pruebas internas del saneado de las fixtures.

use super::*;

#[test]
fn tag_list_is_valid_and_case_insensitive() {
    assert_eq!(tag_class("ssid"), Some(TagClass::Ssid));
    assert_eq!(tag_class("HostName"), Some(TagClass::Host));
    assert_eq!(tag_class("pri_svr_secret"), Some(TagClass::Redact));
    assert_eq!(tag_class("no_existe"), None);
    assert!(parse_tag_list("a keep\nb rara").is_err());
    assert!(parse_tag_list("a keep\nA redact").is_err());
}

#[test]
fn sensitive_names() {
    for (t, v) in [
        ("key_phrase", "x"),
        ("acs_password", "x"),
        ("GponSerialNumber", "x"),
        ("wps_device_pin", "55556666"),
        ("directory_number", "555"),
        ("register_uri", "555"),
        ("connectionRequestURL", "x"),
        ("UserName", "x"),
        ("dhcpv4_opt", "0c 01"),
        ("pri_svr_secret", "x"),
        ("wifi_passphrase", "x"),
        ("admin_pwd", "x"),
        ("api_token", "x"),
        ("contact_email", "x"),
        ("Description", "x"),
        ("user_agent_domain", "x"),
        ("wifi_ssid_id", "Casa"),
        ("wep_key1_type", "abc"),
    ] {
        assert!(is_sensitive(t, v), "{t}");
    }
    for (t, v) in [
        ("nat_port_mapping_id", "3"),
        ("wifi_ssid_id", "1"),
        ("ssid_hide", "0"),
        ("regen_key_interval", "3600"),
        ("wep_key1_type", "-1"),
        ("bssid", "aa:bb:cc:00:11:22"),
        ("igmp_snooping", "1"),
        ("RevisionNumber", "7"),
        ("authentication", "4"),
        ("dhcpv4_opt60", "0"),
        ("name", "ip2"),
    ] {
        assert!(!is_sensitive(t, v), "{t}");
    }
}

#[test]
fn sanitizes_consistently() {
    let mut s = Sanitizer::new();
    let a = s
        .sanitize(
            "<R><ssid>Casa</ssid><key_phrase>secreta1</key_phrase><mac_addr>AA:BB:CC:00:11:22</mac_addr>\
             <ipaddr>9.9.9.9</ipaddr><gw_addr>192.168.1.5</gw_addr><pre_shared_key></pre_shared_key><desc>LAN</desc></R>",
        )
        .unwrap();
    assert_eq!(
        a,
        "<R><ssid>RED-1</ssid><key_phrase>X-REDACTED-X</key_phrase><mac_addr>02:00:00:00:00:01</mac_addr>\
         <ipaddr>203.0.113.1</ipaddr><gw_addr>192.168.1.5</gw_addr><pre_shared_key></pre_shared_key><desc>LAN</desc></R>"
    );
    let b = s
        .sanitize(
            "<R><bssid>aa:bb:cc:00:11:22</bssid><acs_URL>http://9.9.9.9:7547/</acs_URL>\
             <ntp_svr_1>http://9.9.9.9:7547/</ntp_svr_1><desc>nas</desc><HostName>pc</HostName></R>",
        )
        .unwrap();
    assert_eq!(
        b,
        "<R><bssid>02:00:00:00:00:01</bssid><acs_URL>X-REDACTED-X</acs_URL>\
         <ntp_svr_1>http://203.0.113.1:7547/</ntp_svr_1><desc>desc-1</desc><HostName>host-1</HostName></R>"
    );
    let removed: Vec<_> = s.removed_values().collect();
    assert!(removed.contains(&"secreta1") && removed.contains(&"9.9.9.9"));
}

#[test]
fn mac_forms_and_global_ipv6() {
    let mut s = Sanitizer::new();
    let out = s
        .sanitize(
            "<R><hwaddr>AA-BB-CC-00-11-22</hwaddr><mac_addr>aabbcc001122</mac_addr>\
             <rx_byte>987654321098</rx_byte><ipv6_addr>2620:fe::fe</ipv6_addr>\
             <ss_ipv6_prefix>fd00::</ss_ipv6_prefix><dst_addr>2620:fe::fe/64</dst_addr></R>",
        )
        .unwrap();
    assert_eq!(
        out,
        "<R><hwaddr>02-00-00-00-00-01</hwaddr><mac_addr>020000000001</mac_addr>\
         <rx_byte>987654321098</rx_byte><ipv6_addr>2001:db8::1</ipv6_addr>\
         <ss_ipv6_prefix>fd00::</ss_ipv6_prefix><dst_addr>2001:db8::1/64</dst_addr></R>"
    );
}

#[test]
fn seudonimos_de_mac_no_cambian() {
    // Volver a sanear fixtures ya saneadas no renumera sus MAC.
    let mut s = Sanitizer::new();
    let xml = "<R><mac_addr>02:00:00:00:01:01</mac_addr><bssid>02:00:00:00:00:0d</bssid></R>";
    assert_eq!(s.sanitize(xml).unwrap(), xml);
}

#[test]
fn unknown_tags_and_sensitive_names_fail() {
    let mut s = Sanitizer::new();
    let e = s
        .sanitize("<R><nuevo_campo></nuevo_campo></R>")
        .unwrap_err();
    assert!(e.contains("nuevo_campo"), "{e}");
    assert!(s.check_file_name("cbGetX__dmz=1_all=ALL").is_ok());
    assert!(s.check_file_name("cbGetX__raro=1").is_err());
    assert!(s.check_file_name("cbGetX__ssid=Casa").is_err());
}

#[test]
fn learned_values_are_scrubbed_inside_other_values() {
    let mut s = Sanitizer::new();
    let status =
        "<R><wps_status_string>WPS en Casa-Pepe y operadora1.example</wps_status_string></R>";
    let ap = "<R><ssid>Casa-Pepe</ssid><UserName>operadora1</UserName></R>";
    s.learn(status);
    s.learn(ap);
    assert_eq!(
        s.sanitize(status).unwrap(),
        "<R><wps_status_string>WPS en RED-1 y X-REDACTED-X.example</wps_status_string></R>"
    );
    assert_eq!(
        s.sanitize(ap).unwrap(),
        "<R><ssid>RED-1</ssid><UserName>X-REDACTED-X</UserName></R>"
    );
    let removed: Vec<_> = s.removed_values().collect();
    assert_eq!(removed, ["Casa-Pepe", "operadora1"]);
}

#[test]
fn fixture_names_split_on_underscore_after_value() {
    assert_eq!(
        split_fixture_name("cbX__rtm_cfg_dhcpv4_svr_pool_id=0_all=ALL"),
        (
            "cbX",
            vec![
                ("rtm_cfg_dhcpv4_svr_pool_id".to_owned(), "0".to_owned()),
                ("all".to_owned(), "ALL".to_owned())
            ]
        )
    );
    assert_eq!(split_fixture_name("cbX"), ("cbX", vec![]));
}

#[test]
fn renumera_lan_en_orden_y_conserva_router() {
    let mut n = Neutralizador::new();
    let a = n
        .neutralizar(
            "cbGetArpCache.xml",
            "<R><ip>192.168.1.1</ip><ip>192.168.1.77</ip><ip>192.168.1.0</ip></R>",
        )
        .unwrap();
    assert_eq!(
        a,
        "<R><ip>192.168.1.1</ip><ip>192.168.1.60</ip><ip>192.168.1.0</ip></R>"
    );
    let b = n
        .neutralizar(
            "cbGetLanHostList.xml",
            "<R><ip>192.168.1.5</ip><rango>192.168.1.77-192.168.1.255</rango></R>",
        )
        .unwrap();
    assert_eq!(
        b,
        "<R><ip>192.168.1.61</ip><rango>192.168.1.60-192.168.1.255</rango></R>"
    );
}

#[test]
fn renumerar_mas_de_40_equipos_es_error_sin_el_valor() {
    let mut n = Neutralizador::new();
    let xml: String = (2..=42)
        .map(|x| format!("<ip>192.168.1.{x}</ip>"))
        .collect();
    let e = n.neutralizar("cbGetArpCache.xml", &xml).unwrap_err();
    assert!(!e.contains("192.168.1.42"), "{e}");
}

#[test]
fn operador_a_10_255() {
    let mut n = Neutralizador::new();
    let out = n
        .neutralizar(
            "cbGetWanIPIntf.xml",
            "<R><ip>10.31.0.9</ip><gw>192.168.144.1</gw><r>172.20.1.2</r>\
             <c>100.70.0.1</c><ip2>10.31.0.9</ip2><m>255.255.224.0</m></R>",
        )
        .unwrap();
    assert_eq!(
        out,
        "<R><ip>10.255.0.1</ip><gw>192.168.144.1</gw><r>10.255.0.2</r>\
         <c>10.255.0.3</c><ip2>10.255.0.1</ip2><m>255.255.224.0</m></R>"
    );
}

#[test]
fn pool_y_reenvio_neutros() {
    let mut n = Neutralizador::new();
    let pool = n
        .neutralizar(
            "cbGetDhcpv4ServerPool__rtm_cfg_dhcpv4_svr_pool_0i=0.xml",
            "<R><ipv4_addr_min>192.168.1.2</ipv4_addr_min><ipv4_addr_max>192.168.1.250</ipv4_addr_max>\
             <ipv4_dns_addr_0>192.168.1.10</ipv4_dns_addr_0><ipv4_dns_addr_1>203.0.113.2</ipv4_dns_addr_1>\
             <ipv4_dns_addr_2>0.0.0.0</ipv4_dns_addr_2><ipv4_dns_addr_3></ipv4_dns_addr_3>\
             <ipv4_gw_addr>192.168.1.1</ipv4_gw_addr></R>",
        )
        .unwrap();
    assert_eq!(
        pool,
        "<R><ipv4_addr_min>192.168.1.100</ipv4_addr_min><ipv4_addr_max>192.168.1.199</ipv4_addr_max>\
         <ipv4_dns_addr_0>192.168.1.1</ipv4_dns_addr_0><ipv4_dns_addr_1>192.168.1.1</ipv4_dns_addr_1>\
         <ipv4_dns_addr_2>0.0.0.0</ipv4_dns_addr_2><ipv4_dns_addr_3></ipv4_dns_addr_3>\
         <ipv4_gw_addr>192.168.1.1</ipv4_gw_addr></R>"
    );
    let nat = n
        .neutralizar(
            "cbGetNatPortMapping.xml",
            "<R><ext_l4_port_min>1</ext_l4_port_min><ext_l4_port_min_value>2222</ext_l4_port_min_value>\
             <ext_l4_port_max_value>2223</ext_l4_port_max_value><int_ip_addr>192.168.1.30</int_ip_addr>\
             <int_l4_port_min_value>22</int_l4_port_min_value><int_l4_port_max_value>23</int_l4_port_max_value></R>",
        )
        .unwrap();
    assert_eq!(
        nat,
        "<R><ext_l4_port_min>1</ext_l4_port_min><ext_l4_port_min_value>8080</ext_l4_port_min_value>\
         <ext_l4_port_max_value>8080</ext_l4_port_max_value><int_ip_addr>192.168.1.60</int_ip_addr>\
         <int_l4_port_min_value>8080</int_l4_port_min_value><int_l4_port_max_value>8080</int_l4_port_max_value></R>"
    );
    let resumen = n
        .neutralizar(
            "cbGetSummary.xml",
            "<R><SysUptime>26263</SysUptime><SysTime>1700000000</SysTime></R>",
        )
        .unwrap();
    assert_eq!(
        resumen,
        "<R><SysUptime>26263</SysUptime><SysTime>1767225600</SysTime></R>"
    );
}

#[test]
fn neutralizar_es_idempotente() {
    let fixtures = [
        (
            "cbDhcpV4SvrPoolGetStatic.xml",
            "<R><ip>192.168.1.21</ip><ip>192.168.1.77</ip></R>",
        ),
        (
            "cbGetArpCache.xml",
            "<R><ip>192.168.1.11</ip><ip>192.168.1.21</ip><gw>10.1.0.1</gw></R>",
        ),
        (
            "cbGetDhcpv4ServerPool.xml",
            "<R><ipv4_addr_min>192.168.1.2</ipv4_addr_min><ipv4_addr_max>192.168.1.250</ipv4_addr_max>\
             <ipv4_dns_addr_0>192.168.1.10</ipv4_dns_addr_0></R>",
        ),
        (
            "cbGetNatPortMapping.xml",
            "<R><int_ip_addr>192.168.1.30</int_ip_addr><ext_l4_port_min_value>2222</ext_l4_port_min_value></R>",
        ),
        ("cbGetSummary.xml", "<R><SysTime>1700000000</SysTime></R>"),
        (
            "cbGetWanIPIntf.xml",
            "<R><ip>172.16.5.5</ip><gw>10.1.0.1</gw></R>",
        ),
    ];
    let pasar = |entrada: &[(&str, String)]| -> Vec<(String, String)> {
        let mut n = Neutralizador::new();
        entrada
            .iter()
            .map(|(f, x)| ((*f).to_owned(), n.neutralizar(f, x).unwrap()))
            .collect()
    };
    let una: Vec<_> = fixtures
        .iter()
        .map(|(f, x)| (*f, (*x).to_owned()))
        .collect();
    let primera = pasar(&una);
    let otra: Vec<_> = primera
        .iter()
        .map(|(f, x)| (f.as_str(), x.clone()))
        .collect();
    assert_eq!(pasar(&otra), primera);
    assert_ne!(primera[0].1, fixtures[0].1);
}
