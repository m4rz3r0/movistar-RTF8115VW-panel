//! Enmascarado de secretos según `secretos.toml`.

use hgu::xml::XmlNode;
use serde::Deserialize;

use crate::CatalogoError;

/// Valor que sustituye a un secreto.
pub const MASCARA: &str = "••••••";

/// Texto de `secretos.toml`.
pub const SECRETOS_TOML: &str = include_str!("../secretos.toml");

/// Reglas para reconocer nombres de elementos o parámetros secretos.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mascara {
    fragmentos: Vec<String>,
    exactos: Vec<String>,
    sufijos_numericos: Vec<String>,
}

impl Mascara {
    /// Carga las reglas compiladas en el binario.
    pub fn load() -> Result<Mascara, CatalogoError> {
        Self::from_toml(SECRETOS_TOML)
    }

    /// Carga las reglas desde texto TOML.
    pub fn from_toml(text: &str) -> Result<Mascara, CatalogoError> {
        let mut m: Mascara =
            toml::from_str(text).map_err(|e| CatalogoError::Toml(format!("secretos.toml: {e}")))?;
        for v in [&mut m.fragmentos, &mut m.exactos, &mut m.sufijos_numericos] {
            for s in v.iter_mut() {
                *s = s.to_ascii_lowercase();
            }
            if v.iter().any(|s| s.is_empty()) {
                return Err(CatalogoError::Invalido(
                    "secretos.toml: entrada vacía".into(),
                ));
            }
        }
        Ok(m)
    }

    fn contiene_fragmento(&self, n: &str) -> bool {
        self.fragmentos.iter().any(|f| n.contains(f.as_str()))
    }

    fn sufijo_numerico(&self, n: &str) -> bool {
        self.sufijos_numericos
            .iter()
            .any(|s| n.ends_with(s.as_str()))
    }

    /// `true` si el valor del elemento o parámetro `nombre` es secreto.
    /// Los identificadores e indicadores (`*_id`, `*_type`…) no lo son.
    pub fn es_secreto(&self, nombre: &str) -> bool {
        let n = nombre.to_ascii_lowercase();
        self.exactos.contains(&n) || (self.contiene_fragmento(&n) && !self.sufijo_numerico(&n))
    }

    /// `true` si el elemento `nombre` con valor `valor` debe enmascararse:
    /// [`Mascara::es_secreto`], o un nombre sensible con sufijo numérico cuyo
    /// valor no es un entero.
    pub fn oculta(&self, nombre: &str, valor: &str) -> bool {
        if self.es_secreto(nombre) {
            return true;
        }
        let n = nombre.to_ascii_lowercase();
        self.contiene_fragmento(&n) && !es_entero(valor)
    }

    /// Copia del árbol con el texto de los elementos secretos sustituido por
    /// [`MASCARA`] (también si está vacío, para no revelar que no hay clave).
    pub fn enmascarar(&self, node: &XmlNode) -> XmlNode {
        XmlNode {
            tag: node.tag.clone(),
            text: if self.oculta(&node.tag, &node.text) {
                MASCARA.to_owned()
            } else {
                node.text.clone()
            },
            children: node.children.iter().map(|c| self.enmascarar(c)).collect(),
        }
    }

    /// Valor para mostrar o registrar un parámetro.
    pub fn valor_visible<'a>(&self, nombre: &str, valor: &'a str) -> &'a str {
        if self.oculta(nombre, valor) {
            MASCARA
        } else {
            valor
        }
    }
}

fn es_entero(v: &str) -> bool {
    let d = v.strip_prefix('-').unwrap_or(v);
    v.is_empty() || (!d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hgu::xml::parse_response;
    use std::path::Path;

    fn m() -> Mascara {
        Mascara::load().expect("secretos.toml válido")
    }

    #[test]
    fn nombres_secretos() {
        let m = m();
        for n in [
            "key_phrase",
            "pre_shared_key",
            "wep_key1_data",
            "wlWpaPsk",
            "sessionKey",
            "wifi_passphrase",
            "password",
            "Password",
            "auth_password",
            "auth_username",
            "username",
            "UserName",
            "loginPassword",
            "rqPasswd",
            "rqUser",
            "acs_URL",
            "acs_username",
            "acs_password",
            "connectionRequestURL",
            "connectionRequestUsername",
            "connectionRequestPassword",
            "GponSerialNumber",
            "gpon_password",
            "AcsSerialNumber",
            "directory_number",
            "call_forward_unconditional_number",
            "register_uri",
            "conference_call_domain_uri",
            "proxy_server_cfg",
            "outbound_proxy_cfg",
            "wps_device_pin",
            "pri_svr_secret",
            "radius_secret",
            "api_token",
            "admin_pwd",
        ] {
            assert!(m.es_secreto(n), "{n} debería ser secreto");
        }
        for n in [
            "dpm_cfg_wifi_ssid_id",
            "nat_port_mapping_id",
            "nat_port_mapping_0i",
            "wep_key1_type",
            "wep_key_idx",
            "regen_key_interval",
            "ssid",
            "ssid_hide",
            "authentication",
            "RevisionNumber",
            "ModelName",
            "adm_state",
            "ipv4_addr",
            "mac_addr",
            "SysUptime",
            "wps_adm_state",
            "proxy_server_port",
            "mapping",
            "description",
        ] {
            assert!(!m.es_secreto(n), "{n} no debería ser secreto");
        }
    }

    #[test]
    fn sufijo_numerico_con_texto_se_oculta() {
        let m = m();
        assert!(!m.oculta("wep_key1_type", "1"));
        assert!(m.oculta("wep_key1_type", "abc"));
        assert!(!m.oculta("dpm_cfg_wifi_ssid_id", "-1"));
        assert!(!m.oculta("ssid", "Casa"));
        assert_eq!(m.valor_visible("password", "x"), MASCARA);
        assert_eq!(m.valor_visible("ssid", "Casa"), "Casa");
    }

    #[test]
    fn enmascara_recursivo() {
        let m = m();
        let n = parse_response(
            "cbX",
            "<Response><cbX><Result>0</Result><a><ssid>Casa</ssid><key_phrase>supersecreta</key_phrase>\
             <wep_key1_type>1</wep_key1_type><password></password></a></cbX></Response>",
        )
        .unwrap();
        let e = m.enmascarar(&n);
        let a = e.get("a").unwrap();
        assert_eq!(a.text_of("ssid"), Some("Casa"));
        assert_eq!(a.text_of("key_phrase"), Some(MASCARA));
        assert_eq!(a.text_of("wep_key1_type"), Some("1"));
        assert_eq!(a.text_of("password"), Some(MASCARA));
        assert_eq!(e.text_of("Result"), Some("0"));
    }

    fn fixture(nombre: &str) -> XmlNode {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../hgu/fixtures");
        let raw = std::fs::read_to_string(dir.join(nombre)).expect("fixture");
        let call = nombre.split("__").next().unwrap().trim_end_matches(".xml");
        parse_response(call, &raw).expect("respuesta")
    }

    fn hojas<'a>(n: &'a XmlNode, out: &mut Vec<&'a XmlNode>) {
        if n.children.is_empty() {
            out.push(n);
        }
        for c in &n.children {
            hojas(c, out);
        }
    }

    /// Etiquetas que el saneado de fixtures redacta por privacidad pero que no
    /// son credenciales: nombre del concentrador PPPoE, servicio PPPoE,
    /// dominio y dominio SIP del operador.
    const NO_CREDENCIALES: &[&str] = &[
        "ac_name",
        "service_name",
        "domain_name",
        "user_agent_domain",
    ];

    #[test]
    fn mascara_oculta_psk_y_ppp() {
        let m = m();
        let mut vistos = std::collections::BTreeSet::new();
        for f in [
            "cbGetWifiAccPt__dpm_cfg_wifi_acc_pt_id=1.xml",
            "cbGetWifiAccPt__dpm_cfg_wifi_acc_pt_0i=1.xml",
            "cbGetWanIPIntf.xml",
            "cbGetWanIPIntf__ip_intf_0i=0.xml",
            "cbGetGPONInfo.xml",
            "cbGetTr69Config.xml",
            "cbGetVoiceLine.xml",
            "cbGetVoiceSip.xml",
            "cbGetUserList.xml",
        ] {
            let orig = fixture(f);
            let e = m.enmascarar(&orig);
            let (mut a, mut b) = (Vec::new(), Vec::new());
            hojas(&orig, &mut a);
            hojas(&e, &mut b);
            assert_eq!(a.len(), b.len());
            for (o, n) in a.iter().zip(&b) {
                if m.es_secreto(&o.tag) {
                    assert_eq!(n.text, MASCARA, "{f}: <{}> visible", o.tag);
                    vistos.insert(o.tag.to_ascii_lowercase());
                } else if !m.oculta(&o.tag, &o.text) {
                    assert_eq!(n.text, o.text, "{f}: <{}> cambiado", o.tag);
                }
                // Lo que el saneado de fixtures retiró como secreto no puede
                // quedar visible tras enmascarar (salvo datos del operador que
                // se retiraron por privacidad, no por ser credenciales).
                if o.text == "X-REDACTED-X" && !NO_CREDENCIALES.contains(&o.tag.as_str()) {
                    assert_eq!(n.text, MASCARA, "{f}: <{}> redactado pero visible", o.tag);
                }
            }
        }
        for t in [
            "key_phrase",
            "pre_shared_key",
            "wep_key1_data",
            "wps_device_pin",
            "username",
            "password",
            "gponserialnumber",
            "acs_url",
            "connectionrequestpassword",
            "directory_number",
        ] {
            assert!(
                vistos.contains(t),
                "<{t}> no aparece en las fixtures de WiFi/WAN"
            );
        }
    }
}
