//! Lectura de las respuestas XML del router.

use thiserror::Error;

/// Errores al interpretar una respuesta XML.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum XmlError {
    /// El documento no es XML bien formado.
    #[error("XML no válido: {0}")]
    Parse(String),
    /// La raíz no es `Response` (p. ej. sesión caducada).
    #[error("la respuesta no es un <Response>")]
    NotResponse,
    /// Falta el nodo esperado dentro de `Response`.
    #[error("falta el nodo <{0}>")]
    MissingNode(String),
}

/// Nodo XML simplificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlNode {
    /// Nombre de la etiqueta.
    pub tag: String,
    /// Texto directo del nodo, sin espacios en los extremos.
    pub text: String,
    /// Nodos hijos en orden de aparición.
    pub children: Vec<XmlNode>,
}

impl XmlNode {
    fn from_roxml(n: roxmltree::Node) -> Self {
        XmlNode {
            tag: n.tag_name().name().to_owned(),
            text: n
                .children()
                .filter(|c| c.is_text())
                .filter_map(|c| c.text())
                .collect::<String>()
                .trim()
                .to_owned(),
            children: n
                .children()
                .filter(|c| c.is_element())
                .map(Self::from_roxml)
                .collect(),
        }
    }

    /// Primer hijo con esa etiqueta.
    pub fn get(&self, tag: &str) -> Option<&XmlNode> {
        self.children.iter().find(|c| c.tag == tag)
    }

    /// Texto del primer hijo con esa etiqueta.
    pub fn text_of(&self, tag: &str) -> Option<&str> {
        self.get(tag).map(|c| c.text.as_str())
    }

    /// Todos los hijos con esa etiqueta.
    pub fn all(&self, tag: &str) -> Vec<&XmlNode> {
        self.children.iter().filter(|c| c.tag == tag).collect()
    }

    /// `true` si `Result` vale `0` o no existe.
    pub fn result_ok(&self) -> bool {
        self.text_of("Result").is_none_or(|t| t == "0")
    }

    /// El nodo como XML compacto (`<tag>texto</tag>` en las hojas), con `&`,
    /// `<` y `>` escapados. Vuelve a dar el mismo nodo con [`parse_response`]
    /// dentro de un `<Response>`.
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        self.escribir(&mut out);
        out
    }

    fn escribir(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.tag);
        out.push('>');
        if self.children.is_empty() {
            out.push_str(
                &self
                    .text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            );
        }
        for c in &self.children {
            c.escribir(out);
        }
        out.push_str("</");
        out.push_str(&self.tag);
        out.push('>');
    }
}

/// Interpreta `<Response><name>...</name></Response>` y devuelve el nodo `<name>`.
pub fn parse_response(name: &str, raw: &str) -> Result<XmlNode, XmlError> {
    let doc = roxmltree::Document::parse(raw).map_err(|e| XmlError::Parse(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "Response" {
        return Err(XmlError::NotResponse);
    }
    root.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
        .map(XmlNode::from_roxml)
        .ok_or_else(|| XmlError::MissingNode(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str =
        "<Response><cbX><Result>0</Result><s><id>0</id></s><s><id>1</id></s></cbX></Response>";

    #[test]
    fn parsea_respuesta() {
        let n = parse_response("cbX", OK).unwrap();
        assert_eq!(n.tag, "cbX");
        assert_eq!(n.all("s").len(), 2);
        assert!(n.result_ok());
        assert_eq!(n.get("s").unwrap().text_of("id"), Some("0"));
        assert_eq!(n.text_of("Result"), Some("0"));
        assert_eq!(n.text_of("nada"), None);
    }

    #[test]
    fn result_distinto_de_cero_es_error() {
        let n =
            parse_response("cbX", "<Response><cbX><Result>1</Result></cbX></Response>").unwrap();
        assert!(!n.result_ok());
    }

    #[test]
    fn sin_result_es_ok() {
        let n = parse_response("cbX", "<Response><cbX><a>1</a></cbX></Response>").unwrap();
        assert!(n.result_ok());
    }

    #[test]
    fn sesion_caducada_no_es_respuesta() {
        let r = parse_response("cbX", "<script>location = 'te_acceso_router.asp'</script>");
        assert!(matches!(r, Err(XmlError::NotResponse)));
    }

    #[test]
    fn a_xml_y_vuelta() {
        let n = parse_response(
            "cbX",
            "<Response><cbX><a>1 &amp; 2 &lt;3&gt;</a><s><id>0</id></s><Result>0</Result></cbX></Response>",
        )
        .unwrap();
        let xml = n.to_xml();
        assert_eq!(
            xml,
            "<cbX><a>1 &amp; 2 &lt;3&gt;</a><s><id>0</id></s><Result>0</Result></cbX>"
        );
        let otra = parse_response("cbX", &format!("<Response>{xml}</Response>")).unwrap();
        assert_eq!(otra, n);
    }

    #[test]
    fn xml_roto_y_nodo_ausente() {
        assert!(matches!(
            parse_response("cbX", "<Response><cbX>"),
            Err(XmlError::Parse(_))
        ));
        assert!(matches!(
            parse_response("cbY", OK),
            Err(XmlError::MissingNode(_))
        ));
    }
}
