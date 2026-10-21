//! Construcción y retoque de las respuestas XML.

use crate::XML_HEADER;
use hgu::xml::XmlNode;

pub(crate) fn response(node: &XmlNode) -> String {
    format!("{XML_HEADER}<Response>{}</Response>", node.to_xml())
}

pub(crate) fn leaf(tag: &str, text: &str) -> XmlNode {
    XmlNode {
        tag: tag.to_owned(),
        text: text.to_owned(),
        children: Vec::new(),
    }
}

pub(crate) fn result_node(
    name: &str,
    mut children: Vec<XmlNode>,
    result: &str,
    error: &str,
) -> XmlNode {
    children.push(leaf("Result", result));
    if !error.is_empty() {
        children.push(leaf("ErrorString", error));
    }
    XmlNode {
        tag: name.to_owned(),
        text: String::new(),
        children,
    }
}

pub(crate) fn failure(name: &str, error: &str) -> String {
    response(&result_node(name, Vec::new(), "1", error))
}

pub(crate) fn ok(name: &str) -> String {
    response(&result_node(name, Vec::new(), "0", ""))
}

/// Aplica `f` a `n` y a todos sus descendientes.
pub(crate) fn for_each_node(n: &mut XmlNode, f: &mut impl FnMut(&mut XmlNode)) {
    f(n);
    for c in &mut n.children {
        for_each_node(c, f);
    }
}

/// Cambia el texto del hijo `tag`, si existe.
pub(crate) fn set_child(n: &mut XmlNode, tag: &str, text: &str) {
    if let Some(c) = n.children.iter_mut().find(|c| c.tag == tag) {
        c.text = text.to_owned();
    }
}
