//! Enmascarado de secretos y datos sensibles según `secretos.toml`.
//!
//! Dos niveles ([`Nivel`]): los secretos (contraseñas, claves, PIN) van
//! censurados con el ojo en pantalla; los datos sensibles (usuarios,
//! números, serie, servidores) se ven en claro en pantalla. Los dos se
//! enmascaran en los registros, los errores y lo que se guarda sin cifrar:
//! para eso, [`Mascara::es_protegido`], [`Mascara::oculta`] y
//! [`Mascara::enmascarar`]; para la pantalla, [`Mascara::es_secreto`] y
//! [`Mascara::oculta_en_pantalla`].

use hgu::xml::XmlNode;
use serde::Deserialize;

use crate::CatalogoError;

/// Valor que sustituye a un secreto.
pub const MASCARA: &str = "••••••";

/// Texto de `secretos.toml`.
pub const SECRETOS_TOML: &str = include_str!("../secretos.toml");

/// Nivel de protección de un valor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Nivel {
    /// Dato personal o identificador: en claro en pantalla, enmascarado en
    /// los registros.
    Sensible,
    /// Contraseña, clave, PIN o token: censurado con el ojo en pantalla y
    /// enmascarado en los registros.
    Secreto,
}

/// Reglas de un nivel.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reglas {
    fragmentos: Vec<String>,
    exactos: Vec<String>,
}

/// Reglas para reconocer nombres de elementos o parámetros secretos o
/// sensibles.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mascara {
    sufijos_numericos: Vec<String>,
    secreto: Reglas,
    sensible: Reglas,
}

impl Reglas {
    fn contiene_fragmento(&self, n: &str) -> bool {
        self.fragmentos.iter().any(|f| n.contains(f.as_str()))
    }
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
        for v in [
            &mut m.secreto.fragmentos,
            &mut m.secreto.exactos,
            &mut m.sensible.fragmentos,
            &mut m.sensible.exactos,
            &mut m.sufijos_numericos,
        ] {
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

    fn sufijo_numerico(&self, n: &str) -> bool {
        self.sufijos_numericos
            .iter()
            .any(|s| n.ends_with(s.as_str()))
    }

    fn reglas(&self) -> [(Nivel, &Reglas); 2] {
        [
            (Nivel::Secreto, &self.secreto),
            (Nivel::Sensible, &self.sensible),
        ]
    }

    /// Nivel del elemento o parámetro `nombre`, o `None` si se ve tal cual.
    /// Los identificadores e indicadores (`*_id`, `*_type`…) no tienen.
    pub fn nivel(&self, nombre: &str) -> Option<Nivel> {
        let n = nombre.to_ascii_lowercase();
        let numerico = self.sufijo_numerico(&n);
        self.reglas()
            .into_iter()
            .find(|(_, r)| r.exactos.contains(&n) || (!numerico && r.contiene_fragmento(&n)))
            .map(|(nivel, _)| nivel)
    }

    /// Nivel del elemento `nombre` con valor `valor`: [`Mascara::nivel`] o,
    /// con un nombre de un nivel terminado en un sufijo numérico, ese nivel
    /// si el valor no es un entero.
    pub fn nivel_de(&self, nombre: &str, valor: &str) -> Option<Nivel> {
        self.nivel(nombre).or_else(|| {
            if es_entero(valor) {
                return None;
            }
            let n = nombre.to_ascii_lowercase();
            self.reglas()
                .into_iter()
                .find(|(_, r)| r.contiene_fragmento(&n))
                .map(|(nivel, _)| nivel)
        })
    }

    /// `true` si `nombre` es un secreto de verdad (contraseña, clave, PIN o
    /// token): en pantalla, censurado con el ojo.
    pub fn es_secreto(&self, nombre: &str) -> bool {
        self.nivel(nombre) == Some(Nivel::Secreto)
    }

    /// `true` si `nombre` es secreto o sensible: su valor no sale en los
    /// registros, los errores ni lo que se guarda sin cifrar.
    pub fn es_protegido(&self, nombre: &str) -> bool {
        self.nivel(nombre).is_some()
    }

    /// `true` si el elemento `nombre` con valor `valor` se enmascara en los
    /// registros, los errores y lo guardado sin cifrar (los dos niveles).
    pub fn oculta(&self, nombre: &str, valor: &str) -> bool {
        self.nivel_de(nombre, valor).is_some()
    }

    /// `true` si el elemento `nombre` con valor `valor` va censurado en
    /// pantalla (solo los secretos: los sensibles se ven en claro).
    pub fn oculta_en_pantalla(&self, nombre: &str, valor: &str) -> bool {
        self.nivel_de(nombre, valor) == Some(Nivel::Secreto)
    }

    /// Copia del árbol con el texto de los elementos secretos y sensibles
    /// sustituido por [`MASCARA`] (también si está vacío, para no revelar
    /// que no hay clave). Para registros y capturas.
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
            "loginPassword",
            "rqPasswd",
            "acs_password",
            "connectionRequestPassword",
            "gpon_password",
            "wps_device_pin",
            "pri_svr_secret",
            "radius_secret",
            "api_token",
            "admin_pwd",
            "wanppp",
        ] {
            assert_eq!(m.nivel(n), Some(Nivel::Secreto), "{n} debería ser secreto");
            assert!(m.es_secreto(n) && m.es_protegido(n), "{n}");
        }
        for n in [
            "auth_username",
            "username",
            "UserName",
            "rqUser",
            "acs_URL",
            "acs_username",
            "connectionRequestURL",
            "connectionRequestUsername",
            "GponSerialNumber",
            "AcsSerialNumber",
            "directory_number",
            "call_forward_unconditional_number",
            "register_uri",
            "conference_call_domain_uri",
            "proxy_server_cfg",
            "outbound_proxy_cfg",
            "user_agent_domain",
            "rqDelList",
            "account",
        ] {
            assert_eq!(
                m.nivel(n),
                Some(Nivel::Sensible),
                "{n} debería ser sensible"
            );
            assert!(!m.es_secreto(n) && m.es_protegido(n), "{n}");
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
            assert_eq!(m.nivel(n), None, "{n} no debería estar protegido");
            assert!(!m.es_protegido(n), "{n}");
        }
    }

    /// En pantalla solo se censuran los secretos; en los registros, los dos
    /// niveles.
    #[test]
    fn pantalla_solo_secretos_registros_los_dos() {
        let m = m();
        assert!(m.oculta_en_pantalla("acs_password", "x"));
        assert!(m.oculta("acs_password", "x"));
        assert!(!m.oculta_en_pantalla("acs_username", "usuario"));
        assert!(m.oculta("acs_username", "usuario"));
        assert!(m.oculta("acs_URL", "https://acs.example"));
        // Un sufijo numérico con texto: su nivel.
        assert_eq!(m.nivel_de("wep_key1_type", "abc"), Some(Nivel::Secreto));
        assert_eq!(m.nivel_de("serial_type", "abc"), Some(Nivel::Sensible));
        assert_eq!(m.nivel_de("serial_type", "2"), None);
        assert!(!m.oculta_en_pantalla("serial_type", "abc"));
        let n = parse_response(
            "cbX",
            "<Response><cbX><Result>0</Result><acs_URL>https://acs.example</acs_URL>\
             <acs_password>p</acs_password></cbX></Response>",
        )
        .unwrap();
        let e = m.enmascarar(&n);
        assert_eq!(e.text_of("acs_URL"), Some(MASCARA));
        assert_eq!(e.text_of("acs_password"), Some(MASCARA));
    }

    #[test]
    fn sufijo_numerico_con_texto_se_oculta() {
        let m = m();
        assert!(!m.oculta("wep_key1_type", "1"));
        assert!(m.oculta("wep_key1_type", "abc"));
        assert!(!m.oculta("dpm_cfg_wifi_ssid_id", "-1"));
        assert!(!m.oculta("ssid", "Casa"));
        assert!(m.oculta("password", "x"));
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
    /// son credenciales: nombre del concentrador PPPoE, servicio PPPoE y
    /// dominio del operador. (`user_agent_domain` sí se enmascara: lleva la
    /// cuenta SIP incrustada.)
    const NO_CREDENCIALES: &[&str] = &["ac_name", "service_name", "domain_name"];

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
                if m.es_protegido(&o.tag) {
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
