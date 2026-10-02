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
    children: Element,
) -> Element {
    let mut query = use_signal(String::new);
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
        |key| key.reference.to_string(),
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
            section { class: "m1-keymap-layers", "aria-label": "Layers",
                h3 { "Layers" }
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
            }
            section { class: "m1-keymap-key-selection", "aria-label": "Selected key",
                h3 { "{heading}" }
                label { class: "m1-keymap-search-label", "Find a key"
                    input {
                        class: "m1-keymap-search",
                        type: "search",
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
                        option { value: "", "Choose on the layout…" }
                        for key in view.keys.iter().filter(|key| key.search_index.contains(&search)) {
                            option {
                                key: "{key.id}",
                                value: "{key.id}",
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
                    {children}
                } else if no_matches {
                    p { class: "m1-keymap-empty", role: "status", "No keys match this search." }
                } else {
                    p { class: "m1-keymap-empty", role: "status", "Select a switch on the layout to inspect its binding." }
                }
            }
        }
    }
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
            } else {
                p { class: "m1-keymap-layer-protected", role: "status", "The first layer cannot be removed." }
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
