use super::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation};
use super::view::KeymapView;
use boardstudio_application::Scope;
use dioxus::prelude::*;
use std::rc::Rc;

/// Read-only layer browser and shared selected-key control for the Inspector slot.
#[component]
pub(in crate::presentation) fn KeymapPanel(
    view: Rc<KeymapView>,
    scope: Scope,
    active_layer_id: String,
    selected_key_id: Option<String>,
    layer_operations_enabled: bool,
    layer_feedback: Option<KeymapLayerFeedback>,
    on_layer: EventHandler<String>,
    on_layer_operation: EventHandler<KeymapLayerOperation>,
    on_select_key: EventHandler<String>,
    on_export: EventHandler<()>,
    firmware_export_enabled: bool,
    keys_editor: Element,
    macros_editor: Element,
    encoders_editor: Element,
) -> Element {
    let mut query = use_signal(String::new);
    let mut selected_editor = use_signal(|| KeymapEditor::Keys);
    // React keys the Inspector by its accepted selection owner. Keep query and tab
    // presentation local to that owner, without changing shared Session selection.
    use_effect(use_reactive((&scope, &selected_key_id), move |_| {
        if !query.peek().is_empty() {
            query.set(String::new());
        }
        if *selected_editor.peek() != KeymapEditor::Keys {
            selected_editor.set(KeymapEditor::Keys);
        }
    }));
    let active_layer = view
        .layers
        .iter()
        .find(|layer| layer.id.as_ref() == active_layer_id.as_str())
        .or_else(|| view.layers.first());
    let selected = selected_key_id
        .as_deref()
        .and_then(|id| view.keys.iter().find(|key| key.id.as_ref() == id));
    let search = query().to_lowercase();
    let matching_key_count = view
        .keys
        .iter()
        .filter(|key| key.search_index.contains(&search))
        .count();
    let heading = selected.map_or_else(
        || "Select a key".to_owned(),
        |key| {
            format!(
                "{} · {}",
                key.reference,
                active_layer.map_or("Base", |layer| layer.name.as_ref())
            )
        },
    );
    let no_matches = !search.is_empty() && matching_key_count == 0;
    let layer_count = view.layers.len();
    let key_count = view.keys.len();
    let layer_controls_key =
        active_layer.map(|layer| format!("{:?}:{}:{}", scope, layer.id, layer.name));

    rsx! {
        section { class: "m1-keymap-panel", "aria-label": "Keymap",
            header { class: "m1-keymap-heading",
                h2 { "Keymap" }
                span { "{layer_count} layers · {key_count} keys" }
            }
            details { class: "m1-keymap-layers", open: true,
                summary { span { "Layers" } }
                div { class: "m1-keymap-layers-content",
                    div { role: "group", "aria-label": "Keymap layers",
                        for (index, layer) in view.layers.iter().enumerate() {
                            {
                                let id = layer.id.clone();
                                let selected_layer = active_layer.is_some_and(|active| active.id.as_ref() == layer.id.as_ref());
                                rsx! {
                                    button {
                                        key: "{id}",
                                        class: if selected_layer { "m1-keymap-layer is-active" } else { "m1-keymap-layer" },
                                        type: "button",
                                        "aria-pressed": "{selected_layer}",
                                        onclick: move |_| on_layer.call(id.to_string()),
                                        span { class: "m1-keymap-layer-index", "{index}" }
                                        "{layer.name}"
                                    }
                                }
                            }
                        }
                    }
                    if let (Some(layer), Some(layer_controls_key)) = (active_layer, layer_controls_key) {
                        {
                            let layer_id = layer.id.to_string();
                            let layer_name = layer.name.to_string();
                            let is_base = view.layers.first().is_some_and(|base| base.id == layer.id);
                            rsx! {
                                KeymapLayerControls {
                                    key: "{layer_controls_key}",
                                    layer_id,
                                    layer_name,
                                    is_base,
                                    layer_count,
                                    enabled: layer_operations_enabled,
                                    feedback: layer_feedback.clone(),
                                    on_operation: on_layer_operation,
                                }
                            }
                        }
                    }
                    p { class: "m1-keymap-empty", "Higher layers take precedence. Transparent keys fall through to the layer below." }
                }
            }
            div { class: "m1-keymap-actions", role: "group", "aria-label": "Keymap editors",
                for (editor, label) in [
                    (KeymapEditor::Keys, "Keys"),
                    (KeymapEditor::Macros, "Macros"),
                    (KeymapEditor::Encoders, "Encoders"),
                ] {
                    {
                        let pressed = selected_editor() == editor;
                        rsx! {
                            button {
                                class: if pressed { "m1-keymap-editor-tab is-active" } else { "m1-keymap-editor-tab" },
                                r#type: "button",
                                "aria-pressed": "{pressed}",
                                onclick: move |_| selected_editor.set(editor),
                                "{label}"
                            }
                        }
                    }
                }
            }
            button {
                class: "m1-keymap-export",
                r#type: "button",
                disabled: !firmware_export_enabled,
                onclick: move |_| on_export.call(()),
                "Export ZMK source"
            }
            if selected_editor() == KeymapEditor::Keys {
                section { class: "m1-keymap-key-selection", "aria-label": "Selected key",
                    h3 { "{heading}" }
                    label { class: "m1-keymap-search-label", "Find a key"
                        input {
                            class: "m1-keymap-search",
                            r#type: "search",
                            "aria-label": "Find a key",
                            value: "{query}",
                            oninput: move |event: FormEvent| query.set(event.value()),
                        }
                    }
                    label { class: "m1-keymap-select-label", "Selected key"
                        select {
                            class: "m1-keymap-select",
                            "aria-label": "Selected key",
                            value: selected.map_or("", |key| key.id.as_ref()),
                            onchange: move |event: FormEvent| on_select_key.call(event.value()),
                            option { value: "", selected: selected.is_none(), "Choose on the layout…" }
                            for key in view.keys.iter().filter(|key| key.search_index.contains(&search)) {
                                option {
                                    key: "{key.id}",
                                    value: "{key.id}",
                                    selected: selected.is_some_and(|current| current.id == key.id),
                                    "{key.reference} · {key.binding_title}"
                                }
                            }
                        }
                    }
                    if key_count == 0 {
                        p { class: "m1-keymap-empty", role: "status", "No keys are available on this board." }
                    } else if let Some(key) = selected {
                        if no_matches {
                            p { class: "m1-keymap-empty", role: "status", "No keys match this search." }
                        }
                        p { class: "m1-keymap-selected-label", "{key.binding_title}" }
                    } else if no_matches {
                        p { class: "m1-keymap-empty", role: "status", "No keys match this search." }
                    } else {
                        p { class: "m1-keymap-empty", role: "status", "Select a switch on the layout to assign its behavior." }
                    }
                    {keys_editor}
                }
            } else if selected_editor() == KeymapEditor::Macros {
                {macros_editor}
            } else {
                {encoders_editor}
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum KeymapEditor {
    #[default]
    Keys,
    Macros,
    Encoders,
}

#[derive(Props, Clone, PartialEq)]
struct KeymapLayerControlsProps {
    layer_id: String,
    layer_name: String,
    is_base: bool,
    layer_count: usize,
    enabled: bool,
    feedback: Option<KeymapLayerFeedback>,
    on_operation: EventHandler<KeymapLayerOperation>,
}

#[component]
fn KeymapLayerControls(props: KeymapLayerControlsProps) -> Element {
    let mut name_draft = use_signal(|| props.layer_name.clone());
    let add_disabled = !props.enabled || props.layer_count >= 32;
    let remove_disabled = !props.enabled || props.is_base;
    let layer_id = props.layer_id.clone();
    let accepted_name = props.layer_name.clone();
    let feedback_pending = matches!(props.feedback.as_ref(), Some(KeymapLayerFeedback::Pending));
    let feedback_saved = matches!(props.feedback.as_ref(), Some(KeymapLayerFeedback::Saved));
    let feedback_error = props.feedback.as_ref().and_then(|feedback| match feedback {
        KeymapLayerFeedback::Failed(message) => Some(message.as_str()),
        KeymapLayerFeedback::Pending | KeymapLayerFeedback::Saved => None,
    });

    rsx! {
        div { class: "m1-keymap-layer-controls", role: "group", "aria-label": "Layer operations",
            button {
                r#type: "button",
                disabled: add_disabled,
                onclick: move |_| props.on_operation.call(KeymapLayerOperation::Add),
                "Add layer"
            }
            label { class: "m1-keymap-layer-name-label", "Layer name"
                input {
                    class: "m1-keymap-layer-name",
                    r#type: "text",
                    "aria-label": "Layer name",
                    maxlength: 32,
                    value: "{name_draft}",
                    disabled: !props.enabled,
                    oninput: move |event: FormEvent| name_draft.set(event.value()),
                    onblur: {
                        let name_draft = name_draft;
                        let on_operation = props.on_operation;
                        let layer_id = layer_id.clone();
                        let accepted_name = accepted_name.clone();
                        move |_| {
                            let name = name_draft();
                            if name != accepted_name && props.enabled {
                                on_operation.call(KeymapLayerOperation::Rename {
                                    layer_id: layer_id.clone(),
                                    name,
                                });
                            }
                        }
                    },
                }
            }
            if !props.is_base {
                button {
                    r#type: "button",
                    disabled: remove_disabled,
                    onclick: {
                        let on_operation = props.on_operation;
                        let layer_id = props.layer_id.clone();
                        move |_| on_operation.call(KeymapLayerOperation::Remove {
                            layer_id: layer_id.clone(),
                        })
                    },
                    "Remove layer"
                }
            }
            if !props.enabled && props.feedback.is_none() {
                p { class: "m1-keymap-layer-paused", role: "status", "Layer changes are paused while another edit or save is in progress." }
            }
            if feedback_pending { p { role: "status", "Saving layer changes…" } }
            if feedback_saved { p { role: "status", "Layer changes saved." } }
            if let Some(message) = feedback_error { p { role: "alert", "{message}" } }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod disclosure_mounted_tests {
    use super::super::view::KeymapLayerLabel;
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch};
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::{Element as WebElement, HtmlElement, HtmlInputElement};

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct Probe {
        active_layer: Rc<RefCell<String>>,
        render: Rc<RefCell<Option<Signal<u64>>>>,
        operations: Rc<RefCell<Vec<KeymapLayerOperation>>>,
    }

    #[component]
    fn fixture() -> Element {
        let probe = use_context::<Probe>();
        let version = use_signal(|| 0_u64);
        *probe.render.borrow_mut() = Some(version);
        let _ = version();
        let active_layer_id = probe.active_layer.borrow().clone();
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "keymap-disclosure-fixture".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let view = Rc::new(KeymapView {
            layers: vec![
                KeymapLayerLabel {
                    id: Rc::from("base"),
                    name: Rc::from("Main"),
                },
                KeymapLayerLabel {
                    id: Rc::from("fn"),
                    name: Rc::from("Function"),
                },
            ],
            keys: Vec::new(),
        });
        let layer = probe.active_layer.clone();
        let operations = probe.operations.clone();
        rsx! {
            style { {include_str!("../../../assets/m1.css")} }
            KeymapPanel {
                view,
                scope,
                active_layer_id,
                selected_key_id: None,
                layer_operations_enabled: true,
                layer_feedback: None,
                on_layer: move |id| *layer.borrow_mut() = id,
                on_layer_operation: move |operation| operations.borrow_mut().push(operation),
                on_select_key: |_| {},
                on_export: |_| {},
                firmware_export_enabled: true,
                keys_editor: rsx! { div { "Key editor" } },
                macros_editor: rsx! { div { "Macro editor" } },
                encoders_editor: rsx! { div { "Encoder editor" } },
            }
        }
    }

    fn mount() -> (Probe, WebElement) {
        let probe = Probe {
            active_layer: Rc::new(RefCell::new("base".into())),
            render: Rc::new(RefCell::new(None)),
            operations: Rc::new(RefCell::new(Vec::new())),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("keymap-layers-disclosure-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(fixture);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        (probe, root)
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }

    fn layers(root: &WebElement) -> WebElement {
        root.query_selector("details.m1-keymap-layers")
            .unwrap()
            .expect("the Keymap Layers section is a native details disclosure")
    }

    fn click(element: WebElement) {
        element.dyn_into::<HtmlElement>().unwrap().click();
    }

    fn rerender(probe: &Probe) {
        let mut signal = probe
            .render
            .borrow_mut()
            .as_mut()
            .copied()
            .expect("fixture exposes its production render signal");
        signal.set(signal() + 1);
    }

    fn contains_button(root: &WebElement, expected: &str) -> bool {
        let buttons = root.query_selector_all("button").unwrap();
        (0..buttons.length()).any(|index| {
            buttons
                .item(index)
                .and_then(|button| button.text_content())
                .is_some_and(|label| label.trim() == expected)
        })
    }

    #[wasm_bindgen_test]
    async fn layers_disclosure_is_open_accessible_persistent_and_view_only() {
        let (probe, root) = mount();
        rendered().await;

        let disclosure = layers(&root);
        assert!(disclosure.has_attribute("open"), "Layers starts expanded");
        let summary = disclosure.query_selector("summary").unwrap().unwrap();
        assert_eq!(summary.text_content().unwrap().trim(), "Layers");
        assert!(!contains_button(&disclosure, "Remove layer"));
        assert!(
            disclosure
                .query_selector(".m1-keymap-layer-protected")
                .unwrap()
                .is_none(),
            "Base has no Dioxus-only protection sentence"
        );

        click(summary.clone());
        rendered().await;
        assert!(!layers(&root).has_attribute("open"));
        *probe.active_layer.borrow_mut() = "fn".into();
        rerender(&probe);
        rendered().await;
        let collapsed = layers(&root);
        assert!(
            !collapsed.has_attribute("open"),
            "rerender retains disclosure choice"
        );
        assert_eq!(
            collapsed
                .query_selector("input[aria-label='Layer name']")
                .unwrap()
                .unwrap()
                .dyn_into::<HtmlInputElement>()
                .unwrap()
                .value(),
            "Function"
        );

        click(collapsed.query_selector("summary").unwrap().unwrap());
        rendered().await;
        let expanded = layers(&root);
        assert!(expanded.has_attribute("open"));
        assert!(contains_button(&expanded, "Remove layer"));
        assert!(
            expanded
                .query_selector(".m1-keymap-layer-protected")
                .unwrap()
                .is_none()
        );
        assert!(
            probe.operations.borrow().is_empty(),
            "disclosure interaction is view-only"
        );

        let _ = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .remove_child(&root);
    }
}
