use super::view::KeymapView;
use dioxus::prelude::*;
use std::rc::Rc;

/// Read-only layer browser and shared selected-key control for the Inspector slot.
#[component]
pub(in crate::presentation) fn KeymapPanel(
    view: Rc<KeymapView>,
    active_layer_id: String,
    selected_key_id: Option<String>,
    on_layer: EventHandler<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    let mut query = use_signal(|| String::new());
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
                            let selected_layer = active_layer.is_some_and(|active| active.id == layer.id.as_ref());
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
            }
            section { class: "m1-keymap-key-selection", "aria-label": "Selected key"
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
                } else if no_matches {
                    p { class: "m1-keymap-empty", role: "status", "No keys match this search." }
                } else if let Some(key) = selected {
                    p { class: "m1-keymap-selected-label", "{key.binding_title}" }
                } else {
                    p { class: "m1-keymap-empty", role: "status", "Select a switch on the layout to inspect its binding." }
                }
            }
        }
    }
}
