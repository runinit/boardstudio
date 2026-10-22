//! Finds controls by accessible role and name, so browser tests do not depend on `.m1-*` or
//! `.board-*` classes or on tag names. Test-support only, like [`crate::test_wait`].
//!
//! shortcut: names cover ARIA labels, native `label[for]` and text content (no `title` or
//! `alt`), and hidden controls are not excluded from lookup; extend
//! it when a migrated test needs more.
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

/// CSS selector for the elements whose role is `role`: the explicit attribute, plus the native
/// elements that carry the role implicitly.
fn role_selector(role: &str) -> String {
    let implicit = match role {
        "button" => ", button, input[type=button], input[type=submit]",
        "link" => ", a[href]",
        "checkbox" => ", input[type=checkbox]",
        "textbox" => ", input:not([type]), input[type=text], textarea",
        "combobox" => ", select",
        "spinbutton" => ", input[type=number]",
        _ => "",
    };
    format!("[role={role}]{implicit}")
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn content_text(node: &web_sys::Node) -> String {
    if node
        .dyn_ref::<Element>()
        .is_some_and(|element| element.get_attribute("aria-hidden").as_deref() == Some("true"))
    {
        return String::new();
    }
    if node.node_type() == web_sys::Node::TEXT_NODE {
        return node.node_value().unwrap_or_default();
    }
    let children = node.child_nodes();
    (0..children.length())
        .filter_map(|index| children.item(index))
        .map(|child| content_text(&child))
        .collect()
}

/// The accessible name of `element`.
pub fn accessible_name(element: &Element) -> String {
    if let Some(label) = element.get_attribute("aria-label") {
        return collapse(&label);
    }
    if let Some(ids) = element.get_attribute("aria-labelledby") {
        let document = element.owner_document().expect("element is in a document");
        let names: Vec<String> = ids
            .split_whitespace()
            .filter_map(|id| document.get_element_by_id(id))
            .map(|labelling| collapse(&labelling.text_content().unwrap_or_default()))
            .collect();
        return names.join(" ");
    }
    if let Some(id) = element.get_attribute("id")
        && let Some(document) = element.owner_document()
    {
        let labels = document
            .query_selector_all("label[for]")
            .expect("label selector");
        let names: Vec<_> = (0..labels.length())
            .filter_map(|index| labels.item(index)?.dyn_into::<Element>().ok())
            .filter(|label| label.get_attribute("for").as_ref() == Some(&id))
            .map(|label| collapse(&content_text(label.as_ref())))
            .collect();
        if !names.is_empty() {
            return names.join(" ");
        }
    }
    collapse(&content_text(element.as_ref()))
}

/// The elements under `root` with `role` whose accessible name is exactly `name`.
pub fn all_by_role_name(root: &Element, role: &str, name: &str) -> Vec<HtmlElement> {
    let found = root
        .query_selector_all(&role_selector(role))
        .expect("role selector is valid");
    (0..found.length())
        .filter_map(|index| found.item(index)?.dyn_into::<HtmlElement>().ok())
        .filter(|element| accessible_name(element) == name)
        .collect()
}

/// The only element under `root` with `role` and `name`; `None` when there is none yet, so a test
/// can wait for it. Panics when the role and name are ambiguous.
pub fn by_role_name(root: &Element, role: &str, name: &str) -> Option<HtmlElement> {
    let mut found = all_by_role_name(root, role, name);
    assert!(
        found.len() <= 1,
        "{} elements have role {role:?} and name {name:?}",
        found.len()
    );
    found.pop()
}
