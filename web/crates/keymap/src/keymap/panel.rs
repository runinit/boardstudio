use super::view::KeymapView;
use crate::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation};
use boardstudio_application::Scope;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::combobox::{Combobox, ComboboxOption};
use boardstudio_web_ui_shared::ui::field::{Field, FieldError, FieldLabel};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupInput};
use boardstudio_web_ui_shared::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
use dioxus::prelude::*;
use std::rc::Rc;

/// Read-only layer browser and shared selected-key control for the Inspector slot.
#[component]
pub fn KeymapPanel(
    view: Rc<KeymapView>,
    #[props(default = true)] show_navigation: bool,
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
    let mut keys_section_open = use_signal(|| true);
    let mut macros_section_open = use_signal(|| true);
    let mut encoders_section_open = use_signal(|| true);
    // React keys the Inspector by its accepted selection owner. Keep query and tab
    // presentation local to that owner, without changing shared Session selection.
    use_effect(use_reactive((&scope, &selected_key_id), move |_| {
        // React keys the Inspector by its accepted selection owner. A key/scope
        // change remounts the disclosure expanded; layer-only updates keep this
        // signal untouched and preserve a deliberate collapse.
        keys_section_open.set(true);
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
    let encoder_heading = format!(
        "Encoders · {}",
        active_layer.map_or("Base", |layer| layer.name.as_ref())
    );
    let no_matches = !search.is_empty() && matching_key_count == 0;
    let layer_count = view.layers.len();
    let key_count = view.keys.len();
    let search_id = format!("keymap-panel-search-{}", dioxus::core::current_scope_id().0);
    let key_picker_id = format!("keymap-panel-key-{}", dioxus::core::current_scope_id().0);
    let selected_key_value = use_memo(use_reactive((&selected_key_id,), |(value,)| value));

    rsx! {
        section { class: "flex flex-1 flex-col gap-0 min-w-0 min-h-0 overflow-auto py-2 px-6 text-foreground bg-card [&_h3]:m-0 [&_h3]:mb-[7px] [&_h3]:text-foreground [&_h3]:text-base", "aria-label": "Keymap",
            header { class: "flex items-baseline justify-between gap-2 mb-4",
                h2 { class: "m-0 text-foreground text-[17px]", "Keymap editor" }
                span { class: "text-muted-foreground text-sm", "{layer_count} layers · {key_count} keys" }
            }
            if show_navigation {
            KeymapLayersSection {
                compact: false,
                view: view.clone(),
                scope: scope.clone(),
                active_layer_id: active_layer_id.clone(),
                layer_operations_enabled,
                layer_feedback: layer_feedback.clone(),
                on_layer,
                on_layer_operation,
            }
            }
            ToggleGroup { class: "flex-wrap gap-1.5! mb-1", aria_label: "Keymap editors", horizontal: true,
                pressed: Some([match selected_editor() { KeymapEditor::Keys => 0usize, KeymapEditor::Macros => 1, KeymapEditor::Encoders => 2 }].into_iter().collect()),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    let editor = if next.contains(&0) { Some(KeymapEditor::Keys) }
                        else if next.contains(&1) { Some(KeymapEditor::Macros) }
                        else if next.contains(&2) { Some(KeymapEditor::Encoders) }
                        else { None };
                    if let Some(editor) = editor && selected_editor() != editor {
                        match editor {
                            KeymapEditor::Keys => keys_section_open.set(true),
                            KeymapEditor::Macros => macros_section_open.set(true),
                            KeymapEditor::Encoders => encoders_section_open.set(true),
                        }
                        selected_editor.set(editor);
                    }
                },
                ToggleItem { index: 0usize, aria_label: "Keys", class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Keys" }
                ToggleItem { index: 1usize, aria_label: "Macros", class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Macros" }
                ToggleItem { index: 2usize, aria_label: "Encoders", class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Encoders" }
            }
            if selected_editor() == KeymapEditor::Keys {
                Collapsible { class: "min-w-0 border-t border-border", open: keys_section_open(), keep_mounted: true,
                    on_open_change: move |open| keys_section_open.set(open),
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{heading}" }
                    CollapsibleContent { hidden: !keys_section_open(), inert: (!keys_section_open()).then_some(""),
                    div { class: "grid gap-3 min-w-0 pb-5",
                        if show_navigation {
                        Field { class: "gap-1.5",
                            FieldLabel { html_for: search_id.clone(), "Find a key" }
                            InputGroup { InputGroupInput {
                                id: search_id.clone(), r#type: InputType::Search,
                                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                aria_label: "Find a key",
                                value: "{query}",
                                oninput: move |event: FormEvent| query.set(event.value()),
                            } }
                        }
                        Field { class: "gap-1.5",
                            FieldLabel { html_for: key_picker_id.clone(), "Selected key" }
                            Combobox::<String> {
                                id: key_picker_id.clone(),
                                class: "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                                aria_label: "Selected key", list_aria_label: "Selected key",
                                placeholder: "Choose on the layout…",
                                value: Some(selected_key_value.into()),
                                on_value_change: move |value: Option<String>| if let Some(value) = value { on_select_key.call(value); },
                                ComboboxOption::<String> { index: 0usize, value: "", text_value: "Choose on the layout…", class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Choose on the layout…" }
                                for (index, key) in view.keys.iter().filter(|key| key.search_index.contains(&search)).enumerate() {
                                    {
                                        let label = format!("{} · {}", key.reference, key.binding_title);
                                        rsx! { ComboboxOption::<String> { key: "{key.id}", index: index + 1, value: key.id.to_string(), text_value: label.clone(), class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{label}" } }
                                    }
                                }
                            }
                        }
                        }
                        if key_count == 0 {
                            p { class: "m-0 text-muted-foreground text-base leading-normal", role: "status", "No keys are available on this board." }
                        } else if let Some(key) = selected {
                            if no_matches {
                                p { class: "m-0 text-muted-foreground text-base leading-normal", role: "status", "No keys match this search." }
                            }
                            p { class: "m-0 text-foreground text-base", "{key.binding_title}" }
                        } else if no_matches {
                            p { class: "m-0 text-muted-foreground text-base leading-normal", role: "status", "No keys match this search." }
                        } else {
                            p { class: "m-0 text-muted-foreground text-base leading-normal", role: "status", "Select a switch on the layout to assign its behavior." }
                        }
                        {keys_editor}
                    }
                    }
                }
            } else if selected_editor() == KeymapEditor::Macros {
                Collapsible { class: "min-w-0 border-t border-border", open: macros_section_open(), keep_mounted: true,
                    on_open_change: move |open| macros_section_open.set(open),
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Macros" }
                    CollapsibleContent { hidden: !macros_section_open(), inert: (!macros_section_open()).then_some(""),
                    div { class: "grid gap-3 min-w-0 pb-5 [&>section]:border-t-0! [&>section]:pt-0! [&>section>h3]:hidden!",
                        {macros_editor}
                    }
                    }
                }
            } else {
                Collapsible { class: "min-w-0 border-t border-border", open: encoders_section_open(), keep_mounted: true,
                    on_open_change: move |open| encoders_section_open.set(open),
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{encoder_heading}" }
                    CollapsibleContent { hidden: !encoders_section_open(), inert: (!encoders_section_open()).then_some(""),
                    div { class: "grid gap-3 min-w-0 pb-5 [&>section>header]:hidden!",
                        {encoders_editor}
                    }
                    }
                }
            }
            Button {
                class: "w-full my-1 max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                disabled: !firmware_export_enabled,
                onclick: move |_| on_export.call(()),
                "Export ZMK source"
            }
            p { class: "m-0 mb-4 text-muted-foreground text-xs leading-normal", "Local source export. Configure controller and wiring in PCB before building firmware." }
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

/// Layer and key navigation for the dock; edits retain the existing layer controller.
#[component]
pub fn KeymapNavigation(
    view: Rc<KeymapView>,
    scope: Scope,
    active_layer_id: String,
    selected_key_id: Option<String>,
    layer_operations_enabled: bool,
    layer_feedback: Option<KeymapLayerFeedback>,
    on_layer: EventHandler<String>,
    on_layer_operation: EventHandler<KeymapLayerOperation>,
    on_select_key: EventHandler<String>,
) -> Element {
    let mut query = use_signal(String::new);
    let search = query().to_lowercase();
    let search_id = format!(
        "keymap-navigation-search-{}",
        dioxus::core::current_scope_id().0
    );
    let matching_keys: Vec<_> = view
        .keys
        .iter()
        .filter(|key| key.search_index.contains(&search))
        .collect();
    let matching_ids: Vec<String> = matching_keys.iter().map(|key| key.id.to_string()).collect();
    let selected_index = matching_keys
        .iter()
        .position(|key| selected_key_id.as_deref() == Some(key.id.as_ref()));
    rsx! {
        section { class: "flex-1 min-h-0 overflow-auto p-4", aria_label: "Keymap layers and keys",
            KeymapLayersSection {
                compact: true,
                view: view.clone(),
                scope: scope.clone(),
                active_layer_id: active_layer_id.clone(),
                layer_operations_enabled,
                layer_feedback: layer_feedback.clone(),
                on_layer,
                on_layer_operation,
            }

            Field { class: "gap-1.5",
                FieldLabel { html_for: search_id.clone(), "Find a key" }
                InputGroup { InputGroupInput { id: search_id, r#type: InputType::Search, aria_label: "Find a key", class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    value: query(), oninput: move |event: FormEvent| query.set(event.value()) }
                }
            }
            ToggleGroup { class: "grid! gap-0.5! mt-3 w-full", aria_label: "Keys on active layer", horizontal: false,
                pressed: Some(selected_index.into_iter().collect()),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    if let Some(index) = next.into_iter().next().or(selected_index)
                        && let Some(id) = matching_ids.get(index) {
                        on_select_key.call(id.clone());
                    }
                },
                for (index, key) in matching_keys.iter().enumerate() {
                    ToggleItem { key: "{key.id}", index, class: "w-full justify-start max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        aria_label: format!("{} {}", key.reference, key.binding_title),
                        strong { "{key.reference}" } span { "{key.binding_title}" }
                    }
                }
            }
            if matching_keys.is_empty() {
                p { role: "status", "No keys match this search." }
            }
        }
    }
}

/// Layer disclosure shared by the Inspector panel and the dock navigation.
#[component]
fn KeymapLayersSection(
    compact: bool,
    view: Rc<KeymapView>,
    scope: Scope,
    active_layer_id: String,
    layer_operations_enabled: bool,
    layer_feedback: Option<KeymapLayerFeedback>,
    on_layer: EventHandler<String>,
    on_layer_operation: EventHandler<KeymapLayerOperation>,
) -> Element {
    let active_layer = view
        .layers
        .iter()
        .find(|layer| layer.id.as_ref() == active_layer_id)
        .or_else(|| view.layers.first());
    let layer_count = view.layers.len();
    let mut layers_open = use_signal(|| true);
    let tabs_id = format!("keymap-layer-tabs-{}", dioxus::core::current_scope_id().0);
    rsx! {
        Collapsible { class: "min-w-0 border-t border-border", open: layers_open(), on_open_change: move |open| layers_open.set(open), keep_mounted: true,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Keymap layers" }
            CollapsibleContent { hidden: !layers_open(), inert: (!layers_open()).then_some(""),
                div { class: "grid gap-3 min-w-0 pb-5",
                    Tabs { class: "flex-col!", value: active_layer.map(|layer| layer.id.to_string()),
                        on_value_change: move |id: String| on_layer.call(id),
                        TabsList { class: "flex! flex-col! gap-1!", aria_label: "Keymap layers", aria_orientation: "vertical",
                            for (index, layer) in view.layers.iter().enumerate() {
                                TabsTrigger { key: "{layer.id}", value: layer.id.to_string(), index, id: Some(format!("{tabs_id}-tab-{index}")),
                                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                                    aria_label: layer.name.to_string(),
                                    span { class: "flex-none min-w-5 text-muted-foreground tabular-nums", aria_hidden: "true", "{index}" }
                                    "{layer.name}"
                                }
                            }
                        }
                        for (index, layer) in view.layers.iter().enumerate() {
                            TabsContent { key: "{layer.id}", value: layer.id.to_string(), index,
                                aria_label: layer.name.to_string(),
                                aria_labelledby: format!("{tabs_id}-tab-{index}"),
                                KeymapLayerControls {
                                    key: "{scope:?}:{layer.id}",
                                    compact,
                                    layer_id: layer.id.to_string(),
                                    layer_name: layer.name.to_string(),
                                    is_base: index == 0,
                                    layer_count,
                                    enabled: layer_operations_enabled,
                                    feedback: layer_feedback.clone(),
                                    on_operation: on_layer_operation,
                                }
                            }
                        }
                    }
                    p { class: "m-0 text-muted-foreground text-base leading-normal", "Higher layers take precedence. Transparent keys fall through to the layer below." }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct KeymapLayerControlsProps {
    compact: bool,
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
    let context = use_context::<super::layer_controller::LayerEditsContext>();
    let mut name_draft = context.name_draft;
    let mut name_failure = context.name_failure;
    let mut dirty = use_signal(|| false);
    use_effect(use_reactive(
        (&props.layer_name, &props.feedback),
        move |(name, feedback)| {
            if !dirty() && !matches!(feedback, Some(KeymapLayerFeedback::Pending)) {
                name_draft.set(name);
            }
        },
    ));
    let add_disabled = !props.enabled
        || props.layer_count >= 32
        || super::layer_controller::action_pending(&KeymapLayerOperation::Add);
    let remove_disabled = !props.enabled
        || props.is_base
        || super::layer_controller::action_pending(&KeymapLayerOperation::Remove {
            layer_id: props.layer_id.clone(),
        });
    let layer_id = props.layer_id.clone();
    let name_id = format!("keymap-layer-name-{}", dioxus::core::current_scope_id().0);
    let error_id = format!("{name_id}-error");
    let feedback_error = props.feedback.as_ref().and_then(|feedback| match feedback {
        KeymapLayerFeedback::Failed(message) => Some(message.as_str()),
        KeymapLayerFeedback::Pending => None,
    });
    let field_error = if feedback_error.is_none() {
        name_failure()
    } else {
        None
    };

    rsx! {
        div { class: if props.compact {
                "grid gap-2 pt-0 border-t-0 [&_p]:my-[3px] [&_p]:text-muted-foreground [&_p]:text-sm [&_p]:leading-normal [&_[role=alert]]:text-destructive"
            } else {
                "grid gap-3 pt-0 border-t-0 [&_p]:my-[3px] [&_p]:text-muted-foreground [&_p]:text-sm [&_p]:leading-normal [&_[role=alert]]:text-destructive"
            }, role: "group", "aria-label": "Layer operations",
            Button {
                variant: ButtonVariant::Secondary,
                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                disabled: add_disabled,
                onclick: move |_| props.on_operation.call(KeymapLayerOperation::Add),
                "Add layer"
            }
            Field { class: if props.compact { "grid gap-2" } else { "gap-1.5" }, disabled: !props.enabled, invalid: field_error.is_some(),
                FieldLabel { html_for: name_id.clone(), "Layer name" }
                InputGroup {
                InputGroupInput {
                    id: name_id,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    aria_label: "Layer name",
                    aria_invalid: field_error.is_some(),
                    aria_describedby: field_error.as_ref().map(|_| error_id.clone()),
                    value: "{name_draft}",
                    disabled: !props.enabled,
                    oninput: move |event: FormEvent| { name_failure.set(None); name_draft.set(event.value()); dirty.set(true); },
                    attributes: {
                        let name_draft = name_draft;
                        let on_operation = props.on_operation;
                        let layer_id = layer_id.clone();
                        vec![Attribute::new("maxlength", "32", None, false), dioxus::events::onblur(move |_| {
                            let name = name_draft();
                            if dirty() && props.enabled {
                                dirty.set(false);
                                on_operation.call(KeymapLayerOperation::Rename {
                                    layer_id: layer_id.clone(),
                                    name,
                                });
                            }
                        })]
                    },
                }
                }
                if let Some(message) = field_error.as_deref() {
                    div { id: error_id, FieldError { "{message}" } }
                }
            }
            if !props.is_base {
                Button {
                    variant: ButtonVariant::Secondary,
                    class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
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
                p { role: "status", "Layer changes are paused while another edit or save is in progress." }
            }

            if let Some(message) = feedback_error { Alert { variant: AlertVariant::Destructive, "{message}" } }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod disclosure_mounted_tests {
    use super::super::view::{KeymapKey, KeymapLayerLabel};
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch};
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
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
        selected_key: Rc<RefCell<Option<String>>>,
    }

    fn sample_view() -> Rc<KeymapView> {
        Rc::new(KeymapView {
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
            keys: [("key-a", "K1", "Alpha"), ("key-b", "K2", "Beta")]
                .into_iter()
                .map(|(id, reference, binding)| KeymapKey {
                    id: Rc::from(id),
                    reference: Rc::from(reference),
                    pose: boardstudio_model::model::Pose2 {
                        at: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
                        rotation: 0.0,
                    },
                    binding_title: Rc::from(binding),
                    color: Rc::from("#fff"),
                    size: boardstudio_model::model::Vec2 { x: 1.0, y: 1.0 },
                    search_index: format!("{reference} {binding}").to_lowercase(),
                })
                .collect(),
        })
    }

    fn sample_scope() -> Scope {
        Scope {
            session_epoch: SessionEpoch(1),
            document_id: "keymap-disclosure-fixture".into(),
            board_id: "board".into(),
            instance_id: None,
        }
    }

    #[component]
    fn fixture() -> Element {
        super::super::layer_controller::provide_idle_context();
        let probe = use_context::<Probe>();
        let version = use_signal(|| 0_u64);
        *probe.render.borrow_mut() = Some(version);
        let _ = version();
        let active_layer_id = probe.active_layer.borrow().clone();
        let scope = sample_scope();
        let view = sample_view();
        let layer = probe.active_layer.clone();
        let operations = probe.operations.clone();
        let selected_key = probe.selected_key.clone();
        rsx! {
            style { {include_str!("../../../../assets/workbench-base.css")} }
            KeymapPanel {
                view,
                scope,
                active_layer_id,
                selected_key_id: None,
                layer_operations_enabled: true,
                layer_feedback: None,
                on_layer: move |id| *layer.borrow_mut() = id,
                on_layer_operation: move |operation| operations.borrow_mut().push(operation),
                on_select_key: move |id| *selected_key.borrow_mut() = Some(id),
                on_export: |_| {},
                firmware_export_enabled: true,
                keys_editor: rsx! { div { "Key editor" } },
                macros_editor: rsx! { section { "data-editor-probe": "macros", h3 { "Macros" } p { "Macro editor" } } },
                encoders_editor: rsx! { section { "data-editor-probe": "encoders", header { "Encoders" } p { "Encoder editor" } } },
            }
        }
    }

    #[component]
    fn navigation_fixture() -> Element {
        super::super::layer_controller::provide_idle_context();
        let probe = use_context::<Probe>();
        let selected = probe.selected_key.clone();
        rsx! {
            KeymapNavigation {
                view: sample_view(), scope: sample_scope(), active_layer_id: "base".to_owned(),
                selected_key_id: None, layer_operations_enabled: true, layer_feedback: None,
                on_layer: |_| {}, on_layer_operation: |_| {},
                on_select_key: move |id| *selected.borrow_mut() = Some(id),
            }
        }
    }

    fn mount() -> (Probe, WebElement, MountedFixture) {
        mount_component(fixture)
    }

    fn mount_component(component: fn() -> Element) -> (Probe, WebElement, MountedFixture) {
        let probe = Probe {
            active_layer: Rc::new(RefCell::new("base".into())),
            render: Rc::new(RefCell::new(None)),
            operations: Rc::new(RefCell::new(Vec::new())),
            selected_key: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("keymap-layers-disclosure-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(component);
        dom.provide_root_context(probe.clone());
        let mounted = MountedFixture::new(root.clone(), dom);
        (probe, root, mounted)
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(40).await;
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

    #[wasm_bindgen_test]
    async fn editor_bodies_keep_layout_and_nested_heading_rules_without_legacy_selectors() {
        let (_, root, _mounted) = mount();
        rendered().await;
        let key_input = by_role_name(&root, "searchbox", "Find a key").expect("key search");
        let key_body = key_input
            .closest("[data-slot=collapsible-content]")
            .unwrap()
            .unwrap()
            .first_element_child()
            .unwrap();
        for expected in ["grid", "gap-3", "min-w-0", "pb-5"] {
            assert!(
                key_body
                    .get_attribute("class")
                    .unwrap_or_default()
                    .split_whitespace()
                    .any(|class| class == expected),
                "keys: missing {expected}"
            );
        }
        let editors = by_role_name(&root, "group", "Keymap editors").unwrap();
        for (label, probe, nested) in [
            (
                "Macros",
                "macros",
                vec![
                    "[&>section]:border-t-0!",
                    "[&>section]:pt-0!",
                    "[&>section>h3]:hidden!",
                ],
            ),
            ("Encoders", "encoders", vec!["[&>section>header]:hidden!"]),
        ] {
            by_role_name(&editors, "button", label).unwrap().click();
            wait_until("chosen editor mounts", || {
                root.query_selector(&format!("[data-editor-probe={probe}]"))
                    .unwrap()
                    .is_some()
            })
            .await;
            let body = root
                .query_selector(&format!("[data-editor-probe={probe}]"))
                .unwrap()
                .unwrap()
                .parent_element()
                .unwrap();
            let classes = body.get_attribute("class").unwrap_or_default();
            for expected in ["grid", "gap-3", "min-w-0", "pb-5"]
                .into_iter()
                .chain(nested)
            {
                assert!(
                    classes.split_whitespace().any(|class| class == expected),
                    "{label}: missing {expected}"
                );
            }
        }
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn layers_are_accessible_tabs_and_select_the_requested_layer() {
        let (probe, root, _mounted) = mount();
        rendered().await;
        let main = by_role_name(&root, "tab", "Main").expect("Main layer tab");
        let function = by_role_name(&root, "tab", "Function").expect("Function layer tab");
        let controls = by_role_name(&root, "group", "Layer operations").unwrap();
        let classes = controls.get_attribute("class").unwrap_or_default();
        for expected in ["grid", "gap-3"] {
            assert!(classes.split_whitespace().any(|class| class == expected));
        }
        assert!(by_role_name(&root, "tablist", "Keymap layers").is_some());
        let panel = by_role_name(&root, "tabpanel", "Main").expect("named active layer panel");
        let disclosure = by_role_name(&root, "button", "Keymap layers").unwrap();
        let layers = disclosure
            .closest("[data-slot=collapsible]")
            .unwrap()
            .unwrap();
        let layer_classes = layers.get_attribute("class").unwrap_or_default();
        for expected in ["min-w-0", "border-t", "border-border"] {
            assert!(
                layer_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let tabs = main.closest("[data-slot=tabs]").unwrap().unwrap();
        assert!(
            tabs.get_attribute("class")
                .unwrap_or_default()
                .split_whitespace()
                .any(|class| class == "flex-col!")
        );
        let tablist = by_role_name(&root, "tablist", "Keymap layers").unwrap();
        let tablist_classes = tablist.get_attribute("class").unwrap_or_default();
        for expected in ["flex!", "flex-col!", "gap-1!"] {
            assert!(
                tablist_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let controlled = root
            .owner_document()
            .unwrap()
            .get_element_by_id(&disclosure.get_attribute("aria-controls").unwrap())
            .unwrap();
        let content = controlled.first_element_child().unwrap();
        let content_classes = content.get_attribute("class").unwrap_or_default();
        for expected in ["grid", "gap-3", "min-w-0", "pb-5"] {
            assert!(
                content_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        assert_eq!(
            main.get_attribute("aria-controls").as_deref(),
            Some(panel.id().as_str())
        );
        assert_eq!(
            panel.get_attribute("aria-labelledby").as_deref(),
            Some(main.id().as_str())
        );
        assert_eq!(main.get_attribute("aria-selected").as_deref(), Some("true"));
        assert_eq!(
            function.get_attribute("aria-selected").as_deref(),
            Some("false")
        );
        function.click();
        rendered().await;
        assert_eq!(&*probe.active_layer.borrow(), "fn");
        rerender(&probe);
        rendered().await;
        assert_eq!(
            by_role_name(&root, "tab", "Function")
                .unwrap()
                .get_attribute("aria-selected")
                .as_deref(),
            Some("true")
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn editor_choice_roves_and_keeps_its_selection() {
        let (_, root, _mounted) = mount();
        rendered().await;
        let panel = root
            .query_selector("[aria-label='Keymap']")
            .unwrap()
            .unwrap();
        let panel_classes = panel.get_attribute("class").unwrap_or_default();
        for expected in [
            "flex",
            "flex-1",
            "flex-col",
            "gap-0",
            "min-w-0",
            "min-h-0",
            "overflow-auto",
            "py-2",
            "px-6",
            "text-foreground",
            "bg-card",
        ] {
            assert!(
                panel_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let heading = panel.query_selector("header").unwrap().unwrap();
        let heading_classes = heading.get_attribute("class").unwrap_or_default();
        for expected in ["flex", "items-baseline", "justify-between", "gap-2", "mb-4"] {
            assert!(
                heading_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let empty = by_role_name(
            &root,
            "status",
            "Select a switch on the layout to assign its behavior.",
        )
        .expect("key selection guidance");
        let classes = empty.get_attribute("class").unwrap_or_default();
        for expected in [
            "m-0",
            "text-muted-foreground",
            "text-base",
            "leading-normal",
        ] {
            assert!(
                classes.split_whitespace().any(|class| class == expected),
                "key selection guidance retains its presentation: {expected}"
            );
        }
        let group = by_role_name(&root, "group", "Keymap editors").unwrap();
        let classes = group.get_attribute("class").unwrap_or_default();
        for expected in ["flex-wrap", "gap-1.5!", "mb-1"] {
            assert!(classes.split_whitespace().any(|class| class == expected));
        }
        let keys = by_role_name(&root, "button", "Keys").unwrap();
        let macros = by_role_name(&root, "button", "Macros").unwrap();
        let section = by_role_name(&root, "button", "Select a key")
            .expect("key editor disclosure")
            .closest("[data-slot=collapsible]")
            .unwrap()
            .unwrap();
        let classes = section.get_attribute("class").unwrap_or_default();
        for expected in ["min-w-0", "border-t", "border-border"] {
            assert!(classes.split_whitespace().any(|class| class == expected));
        }
        assert_eq!(keys.get_attribute("aria-pressed").as_deref(), Some("true"));
        keys.focus().unwrap();
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("ArrowRight");
        init.set_bubbles(true);
        keys.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
        wait_until("the next editor receives focus", || {
            macros.matches(":focus").unwrap()
        })
        .await;
        macros.click();
        rendered().await;
        assert_eq!(
            macros.get_attribute("aria-pressed").as_deref(),
            Some("true")
        );
        assert!(root.text_content().unwrap().contains("Macro editor"));
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn key_picker_filters_and_selects_the_named_key() {
        let (probe, root, _mounted) = mount();
        rendered().await;
        let picker = by_role_name(&root, "combobox", "Selected key").unwrap();
        picker.focus().unwrap();
        let key = |value: &str| {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(value);
            init.set_bubbles(true);
            init.set_cancelable(true);
            picker
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap(),
                )
                .unwrap();
        };
        key("ArrowDown");
        wait_until("key picker opens a named listbox", || {
            by_role_name(&root, "listbox", "Selected key").is_some()
        })
        .await;
        let input: HtmlInputElement = picker.clone().dyn_into().unwrap();
        input.set_value("beta");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        wait_until("key picker filters", || {
            by_role_name(&root, "option", "K2 · Beta").is_some()
                && by_role_name(&root, "option", "K1 · Alpha").is_none()
        })
        .await;
        key("ArrowDown");
        wait_until("filtered key is active", || {
            by_role_name(&root, "option", "K2 · Beta").is_some_and(|option| {
                picker.get_attribute("aria-activedescendant").as_deref()
                    == Some(option.id().as_str())
            })
        })
        .await;
        key("Enter");
        wait_until("key selection is emitted", || {
            probe.selected_key.borrow().as_deref() == Some("key-b")
        })
        .await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn navigation_keys_roving_focus_and_selection() {
        let (probe, root, _mounted) = mount_component(navigation_fixture);
        rendered().await;
        let group = by_role_name(&root, "group", "Keys on active layer").unwrap();
        let navigation = group
            .closest("[aria-label='Keymap layers and keys']")
            .unwrap()
            .unwrap();
        let navigation_classes = navigation.get_attribute("class").unwrap_or_default();
        for expected in ["flex-1", "min-h-0", "overflow-auto", "p-4"] {
            assert!(
                navigation_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let classes = group.get_attribute("class").unwrap_or_default();
        for expected in ["grid!", "gap-0.5!", "mt-3"] {
            assert!(
                classes.split_whitespace().any(|class| class == expected),
                "key navigation retains its layout without legacy CSS: {expected}"
            );
        }
        let name = by_role_name(&root, "textbox", "Layer name").unwrap();
        let controls = by_role_name(&root, "group", "Layer operations").unwrap();
        let control_classes = controls.get_attribute("class").unwrap_or_default();
        for expected in ["grid", "gap-2"] {
            assert!(
                control_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let input_classes = name.get_attribute("class").unwrap_or_default();
        for expected in ["w-full", "min-w-0"] {
            assert!(
                input_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let field = name.closest("[data-slot=field]").unwrap().unwrap();
        let field_classes = field.get_attribute("class").unwrap_or_default();
        for expected in ["grid", "gap-2"] {
            assert!(
                field_classes
                    .split_whitespace()
                    .any(|class| class == expected)
            );
        }
        let first = by_role_name(&root, "button", "K1 Alpha").unwrap();
        let second = by_role_name(&root, "button", "K2 Beta").unwrap();
        first.focus().unwrap();
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("ArrowDown");
        init.set_bubbles(true);
        first
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        wait_until("next key receives focus", || {
            second.matches(":focus").unwrap()
        })
        .await;
        second.click();
        wait_until("key navigation emits selection", || {
            probe.selected_key.borrow().as_deref() == Some("key-b")
        })
        .await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn layer_tabs_roving_focus_wraps_and_supports_home_and_end() {
        let (_, root, _mounted) = mount();
        rendered().await;
        let main = by_role_name(&root, "tab", "Main").unwrap();
        let function = by_role_name(&root, "tab", "Function").unwrap();
        let key = |target: &HtmlElement, value: &str| {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(value);
            init.set_bubbles(true);
            init.set_cancelable(true);
            target
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                        .unwrap(),
                )
                .unwrap();
        };
        assert_eq!(main.get_attribute("tabindex").as_deref(), Some("0"));
        assert_eq!(function.get_attribute("tabindex").as_deref(), Some("-1"));
        main.focus().unwrap();
        for (source, value, destination) in [
            (&main, "ArrowUp", &function),
            (&function, "ArrowDown", &main),
            (&main, "End", &function),
            (&function, "Home", &main),
        ] {
            key(source, value);
            wait_until("layer tab receives focus", || {
                destination.matches(":focus").unwrap()
            })
            .await;
        }
        assert_eq!(
            main.get_attribute("aria-selected").as_deref(),
            Some("true"),
            "arrow navigation does not activate a layer"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn layer_name_keeps_its_length_limit_and_commits_on_blur() {
        let (probe, root, _mounted) = mount();
        rendered().await;
        let input: HtmlInputElement = by_role_name(&root, "textbox", "Layer name")
            .expect("layer name")
            .dyn_into()
            .unwrap();
        assert_eq!(input.get_attribute("maxlength").as_deref(), Some("32"));
        input.focus().unwrap();
        input.set_value("Renamed");
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
        input.blur().unwrap();
        wait_until("the layer name is committed on blur", || {
            !probe.operations.borrow().is_empty()
        })
        .await;
        assert_eq!(
            &*probe.operations.borrow(),
            &[KeymapLayerOperation::Rename {
                layer_id: "base".into(),
                name: "Renamed".into(),
            }]
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn layers_disclosure_is_open_accessible_persistent_and_view_only() {
        let (probe, root, _mounted) = mount();
        rendered().await;

        let disclosure = by_role_name(&root, "button", "Keymap layers").expect("layers disclosure");
        assert_eq!(
            disclosure.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        assert!(by_role_name(&root, "button", "Remove layer").is_none());

        disclosure.click();
        rendered().await;
        assert_eq!(
            disclosure.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        let content = root
            .owner_document()
            .unwrap()
            .get_element_by_id(
                &disclosure
                    .get_attribute("aria-controls")
                    .expect("disclosure controls content"),
            )
            .expect("controlled content exists");
        assert!(
            content.has_attribute("hidden"),
            "collapsed controls are not exposed"
        );
        *probe.active_layer.borrow_mut() = "fn".into();
        rerender(&probe);
        rendered().await;
        assert_eq!(
            disclosure.get_attribute("aria-expanded").as_deref(),
            Some("false"),
            "rerender retains disclosure choice"
        );
        assert_eq!(
            by_role_name(&root, "textbox", "Layer name")
                .unwrap()
                .dyn_into::<HtmlInputElement>()
                .unwrap()
                .value(),
            "Function"
        );

        disclosure.click();
        rendered().await;
        assert_eq!(
            disclosure.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        assert!(by_role_name(&root, "button", "Remove layer").is_some());
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
