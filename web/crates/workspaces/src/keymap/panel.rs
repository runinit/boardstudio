//! Declarative Keymap navigation (layers & key picker) and tabbed inspector panel.
use super::view::KeymapView;
use crate::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation};
use boardstudio_application::Scope;
use boardstudio_web_ui_shared::inspector_fields::{SelectField, TextCommitField, TextField};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
    input::InputType,
    tabs::{Tabs, TabsContent, TabsList, TabsTrigger},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn KeymapNavigation(
    view: Rc<KeymapView>,
    scope: Scope,
    active_layer_id: String,
    selected_key_id: Option<String>,
    layer_operations_enabled: bool,
    layer_feedback: Option<KeymapLayerFeedback>,
    on_layer_operation: EventHandler<KeymapLayerOperation>,
    on_layer: EventHandler<String>,
    on_select_key: EventHandler<String>,
) -> Element {
    let _ = scope;
    let active_idx = view
        .layers
        .iter()
        .position(|l| l.id.as_ref() == active_layer_id)
        .unwrap_or(0);
    let active = view.layers.get(active_idx);
    let mut query = use_signal(String::new);
    let q = query().to_lowercase();
    let mut key_opts = vec![(String::new(), "Select a key…".to_owned())];
    for k in view.keys.iter().filter(|k| k.search_index.contains(&q)) {
        key_opts.push((
            k.id.to_string(),
            format!("{} · {}", k.reference, k.binding_title),
        ));
    }
    rsx! {
        section { class: "grid min-w-0 gap-3 p-4 text-foreground", "aria-label": "Keymap layers",
            div { class: "flex items-center justify-between gap-2",
                h2 { class: "m-0 text-sm font-semibold", "Layers" }
                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, disabled: !layer_operations_enabled || view.layers.len() >= 32,
                    onclick: move |_| on_layer_operation.call(KeymapLayerOperation::Add), "Add layer" }
            }
            div { class: "flex flex-wrap gap-1.5",
                for layer in &view.layers {
                    {
                        let (id, is_active) = (layer.id.to_string(), active.is_some_and(|a| a.id == layer.id));
                        rsx! {
                            Button { key: "{id}", variant: if is_active { ButtonVariant::Default } else { ButtonVariant::Secondary }, size: ButtonSize::Sm,
                                onclick: move |_| on_layer.call(id.clone()), "{layer.name}" }
                        }
                    }
                }
            }
            if let Some(layer) = active {
                {
                    let (id_ren, id_rem) = (layer.id.to_string(), layer.id.to_string());
                    rsx! {
                        TextCommitField {
                            label: "Layer name".to_owned(), aria_label: "Layer name", value: layer.name.to_string(),
                            disabled_reason: (!layer_operations_enabled).then(|| "Layer editing unavailable.".to_owned()),
                            on_commit: move |name: String| on_layer_operation.call(KeymapLayerOperation::Rename { layer_id: id_ren.clone(), name }),
                        }
                        if active_idx > 0 {
                            Button { variant: ButtonVariant::Destructive, size: ButtonSize::Sm, disabled: !layer_operations_enabled,
                                onclick: move |_| on_layer_operation.call(KeymapLayerOperation::Remove { layer_id: id_rem.clone() }), "Remove layer" }
                        }
                    }
                }
            }
            if let Some(KeymapLayerFeedback::Failed(msg)) = layer_feedback {
                Alert { variant: AlertVariant::Destructive, "{msg}" }
            }
            TextField {
                id: format!("keymap-key-search-{}", dioxus::core::current_scope_id().0),
                label: "Find a key".to_owned(), aria_label: "Find a key", r#type: InputType::Search,
                value: query(), oninput: move |e: FormEvent| query.set(e.value()),
            }
            SelectField::<String> {
                label: "Key".to_owned(), aria_label: "Selected key",
                value: selected_key_id.unwrap_or_default(), options: key_opts,
                on_commit: move |v: String| if !v.is_empty() { on_select_key.call(v); },
            }
        }
    }
}

#[component]
pub fn KeymapPanel(
    view: Rc<KeymapView>,
    selected_key_id: Option<String>,
    on_export: EventHandler<()>,
    firmware_export_enabled: bool,
    binding_error: Option<String>,
    keys_editor: Element,
    macros_editor: Element,
    encoders_editor: Element,
) -> Element {
    let mut tab = use_signal(|| "keys".to_owned());
    let assigned = view
        .keys
        .iter()
        .filter(|k| k.binding_title.as_ref() != "Unassigned")
        .count();
    rsx! {
        section { class: "flex flex-1 flex-col gap-3 min-w-0 min-h-0 overflow-auto px-6 py-4 text-foreground bg-card", "aria-label": "Keymap",
            header { class: "flex items-baseline justify-between gap-2",
                h2 { class: "m-0 text-xl font-semibold", "Keymap" }
                span { class: "text-xs text-muted-foreground", "{assigned}/{view.keys.len()} assigned" }
            }
            if let Some(msg) = binding_error { Alert { variant: AlertVariant::Destructive, "{msg}" } }
            Tabs { value: tab(), on_value_change: move |v: String| tab.set(v),
                TabsList {
                    TabsTrigger { value: "keys".to_owned(), index: 0usize, "Keys" }
                    TabsTrigger { value: "encoders".to_owned(), index: 1usize, "Encoders" }
                    TabsTrigger { value: "macros".to_owned(), index: 2usize, "Macros" }
                }
                TabsContent { value: "keys".to_owned(), index: 0usize,
                    if selected_key_id.is_some() { {keys_editor} }
                    else { p { class: "text-sm text-muted-foreground", "Select a key on the layout to edit its binding." } }
                }
                TabsContent { value: "encoders".to_owned(), index: 1usize, {encoders_editor} }
                TabsContent { value: "macros".to_owned(), index: 2usize, {macros_editor} }
            }
            Button { variant: ButtonVariant::Secondary, disabled: !firmware_export_enabled, onclick: move |_| on_export.call(()), "Export firmware" }
        }
    }
}
