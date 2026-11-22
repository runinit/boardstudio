//! Private, typed editor for one accepted Keymap binding.
//!
//! The pure binding model (identities, behavior selection, field helpers)
//! lives in `crate::binding` so it compiles on every target; this module
//! keeps the mounted components and re-exports the shared names.
use crate::binding::{
    Behavior, HOLD_MODIFIERS, KEYCODE_CHOICES, binding_keycode, binding_layer_id, choose_behavior,
    field_exists, with_keycode, with_layer_id,
};
pub use crate::binding::{
    BindingEditFeedback, BindingEditRequest, BindingEditStatus, BindingField, BindingLayerChoice,
    BindingMacroChoice, BindingTarget, EncoderInputIdentity,
};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::KeyBinding;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::combobox::{Combobox, ComboboxOption};
use boardstudio_web_ui_shared::ui::field::{Field, FieldError, FieldLabel};
use boardstudio_web_ui_shared::ui::select::{Select, SelectOption};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Props, Clone, PartialEq)]
pub struct BindingEditorProps {
    pub scope: Scope,
    pub admission_token: SnapshotToken,
    pub admission_revision: u64,
    pub active_layer_id: String,
    pub target: BindingTarget,
    pub input_identity: Option<EncoderInputIdentity>,
    pub key_label: String,
    pub editor_instance_id: u64,
    pub request_sequence: Signal<u64>,
    pub value: KeyBinding,
    pub layers: Rc<[BindingLayerChoice]>,
    pub macros: Rc<[BindingMacroChoice]>,
    pub enabled: bool,
    pub feedback: Option<BindingEditFeedback>,
    pub on_change: EventHandler<BindingEditRequest>,
}

#[derive(Clone, PartialEq)]
struct EditContext {
    scope: Scope,
    admission_token: SnapshotToken,
    admission_revision: u64,
    active_layer_id: String,
    target: BindingTarget,
    input_identity: Option<EncoderInputIdentity>,
    editor_instance_id: u64,
    enabled: bool,
}

impl From<&BindingEditorProps> for EditContext {
    fn from(props: &BindingEditorProps) -> Self {
        Self {
            scope: props.scope.clone(),
            admission_token: props.admission_token,
            admission_revision: props.admission_revision,
            active_layer_id: props.active_layer_id.clone(),
            target: props.target.clone(),
            input_identity: props.input_identity.clone(),
            editor_instance_id: props.editor_instance_id,
            enabled: props.enabled,
        }
    }
}

fn emit_change(
    context: &EditContext,
    sequence: &mut Signal<u64>,
    on_change: EventHandler<BindingEditRequest>,
    field: BindingField,
    binding: KeyBinding,
) {
    if !context.enabled {
        return;
    }
    let Some(request_id) = sequence().checked_add(1) else {
        return;
    };
    sequence.set(request_id);
    on_change.call(BindingEditRequest {
        scope: context.scope.clone(),
        admission_token: context.admission_token,
        admission_revision: context.admission_revision,
        active_layer_id: context.active_layer_id.clone(),
        target: context.target.clone(),
        input_identity: context.input_identity.clone(),
        field,
        editor_instance_id: context.editor_instance_id,
        request_id,
        binding,
    });
}

/// Edits a single projected binding and reports typed, target-correlated intents.
#[component]
pub fn BindingEditor(props: BindingEditorProps) -> Element {
    let request_sequence = props.request_sequence;
    let context = EditContext::from(&props);
    let value = props.value.clone();
    let behavior = Behavior::from_binding(&value);
    let target_key = props.target.stable_key();
    let keycode_field = binding_keycode(&value);
    let layer_value = binding_layer_id(&value).map(str::to_owned);
    let macro_value = match &value {
        KeyBinding::Macro { macro_id } => Some(macro_id.clone()),
        _ => None,
    };
    let feedback = props.feedback.as_ref().filter(|feedback| {
        feedback.scope == props.scope
            && feedback.active_layer_id == props.active_layer_id
            && feedback.target == props.target
            && feedback.editor_instance_id == props.editor_instance_id
            && field_exists(&value, feedback.field)
    });
    let failed_field = feedback.map(|feedback| match &feedback.status {
        BindingEditStatus::Failed(message) => (feedback.field, message.as_str()),
    });
    let code_error = failed_field
        .filter(|(field, _)| matches!(field, BindingField::Keycode | BindingField::Tap))
        .map(|(_, message)| message.to_owned());
    let error = failed_field
        .filter(|(field, _)| !matches!(field, BindingField::Keycode | BindingField::Tap))
        .map(|(_, message)| message);
    let disabled = !props.enabled;
    let key_label = props.key_label.clone();
    let mut behavior_sequence = request_sequence;
    let behavior_context = context.clone();
    let behavior_change = props.on_change;
    let layers = props.layers.clone();
    let macros = props.macros.clone();

    rsx! {
        section { class: "grid gap-2.5 mt-3 pt-3 border-t border-border", "aria-label": "Key binding",
            BindingChoice {
                label: "Behavior",
                aria_label: "{key_label} behavior",
                value: "{behavior.value()}",
                disabled,
                options: Behavior::ALL.iter().map(|(candidate, title)| (candidate.value().to_owned(), (*title).to_owned(), *candidate == Behavior::Macro && props.macros.is_empty())).collect(),
                    on_change: move |next: String| {
                        if let Some(next_behavior) = Behavior::from_value(&next)
                            && (next_behavior != Behavior::Macro || !macros.is_empty()) {
                                let binding = choose_behavior(next_behavior, &layers, &macros);
                                emit_change(
                                    &behavior_context,
                                    &mut behavior_sequence,
                                    behavior_change,
                                    BindingField::Behavior,
                                    binding,
                                );
                        }
                    },
            }
            if let Some((field_label, field, accepted_value)) = keycode_field {
                {
                    let field = if field == "tap" { BindingField::Tap } else { BindingField::Keycode };
                    let draft_key = format!(
                        "{:?}:{}:{}:{:?}:{}",
                        props.scope,
                        props.editor_instance_id,
                        props.active_layer_id,
                        field,
                        target_key,
                    );
                    rsx! {
                        KeycodeField {
                            key: "{draft_key}",
                            context: context.clone(),
                            request_sequence,
                            on_change: props.on_change,
                            binding: value.clone(),
                            field,
                            label: format!(
                                "{} {}",
                                props.key_label,
                                if field == BindingField::Tap { "tap" } else { "keycode" },
                            ),
                            field_label: field_label.to_string(),
                            accepted_value: accepted_value.to_string(),
                            error: code_error.clone(),
                            disabled,
                        }
                    }
                }
            }
            if let KeyBinding::ModTap { hold, .. } = &value {
                {
                    let current_hold = hold.clone();
                    let mut hold_sequence = request_sequence;
                    let hold_context = context.clone();
                    let hold_change = props.on_change;
                    let hold_binding = value.clone();
                    rsx! {
                        BindingChoice {
                            label: "Hold modifier",
                            aria_label: "{props.key_label} hold modifier",
                            value: "{current_hold}",
                            disabled,
                            options: HOLD_MODIFIERS.iter().map(|modifier| ((*modifier).to_owned(), (*modifier).to_owned(), false)).collect(),
                                on_change: move |next: String| {
                                    if let KeyBinding::ModTap { tap, .. } = &hold_binding {
                                        emit_change(
                                            &hold_context,
                                            &mut hold_sequence,
                                            hold_change,
                                            BindingField::HoldModifier,
                                            KeyBinding::ModTap {
                                                hold: next,
                                                tap: tap.clone(),
                                            },
                                        );
                                    }
                                },
                        }
                    }
                }
            }
            if let Some(layer_id) = layer_value {
                {
                    let selected_layer_id = layer_id.clone();
                    let layer_binding = value.clone();
                    let mut layer_sequence = request_sequence;
                    let layer_context = context.clone();
                    let layer_change = props.on_change;
                    rsx! {
                        BindingChoice {
                            label: "Layer",
                            aria_label: "{props.key_label} layer",
                            value: "{selected_layer_id}",
                            disabled,
                            options: props.layers.iter().map(|layer| (layer.id.clone(), layer.name.clone(), false)).collect(),
                                on_change: move |next: String| {
                                    if let Some(binding) = with_layer_id(&layer_binding, next) {
                                        emit_change(
                                            &layer_context,
                                            &mut layer_sequence,
                                            layer_change,
                                            BindingField::Layer,
                                            binding,
                                        );
                                    }
                                },
                        }
                    }
                }
            }
            if let Some(macro_id) = macro_value {
                {
                    let selected_macro_id = macro_id.clone();
                    let mut macro_sequence = request_sequence;
                    let macro_context = context.clone();
                    let macro_change = props.on_change;
                    rsx! {
                        BindingChoice {
                            label: "Macro",
                            aria_label: "{props.key_label} macro",
                            value: "{selected_macro_id}",
                            disabled,
                            options: props.macros.iter().map(|item| (item.id.clone(), item.name.clone(), false)).collect(),
                                on_change: move |next: String| {
                                    emit_change(
                                        &macro_context,
                                        &mut macro_sequence,
                                        macro_change,
                                        BindingField::Macro,
                                        KeyBinding::Macro { macro_id: next },
                                    );
                                },
                        }
                    }
                }
            }
            if let Some(message) = error { Alert { variant: AlertVariant::Destructive, "{message}" } }
        }
    }
}

/// Preserve typed binding values while choosing a short Select or a searchable long list.
#[component]
fn BindingChoice(
    label: String,
    aria_label: String,
    value: String,
    disabled: bool,
    options: Vec<(String, String, bool)>,
    on_change: EventHandler<String>,
) -> Element {
    let id = format!("keymap-choice-{}", dioxus::core::current_scope_id().0);
    let selected = use_memo(use_reactive((&value,), |(value,)| Some(value)));
    rsx! {
        Field { class: "my-0 gap-1.5", disabled,
            FieldLabel { html_for: id.clone(), "{label}" }
            if options.len() > 12 {
                Combobox::<String> {
                    id, aria_label: aria_label.clone(), list_aria_label: aria_label,
                    class: "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                    value: Some(selected.into()), disabled,
                    on_value_change: move |value: Option<String>| if let Some(value) = value { on_change.call(value); },
                    for (index, (value, text, unavailable)) in options.into_iter().enumerate() {
                        ComboboxOption::<String> { index, value, text_value: text.clone(), disabled: unavailable,
                            class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{text}" }
                    }
                }
            } else {
                Select::<String> {
                    id, aria_label,
                    class: "max-[960px]:[&_[data-slot=select-trigger]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=select-trigger]]:min-h-control-touch",
                    value: Some(selected.into()), disabled,
                    on_value_change: move |value: Option<String>| if let Some(value) = value { on_change.call(value); },
                    for (index, (value, text, unavailable)) in options.into_iter().enumerate() {
                        SelectOption::<String> { index, value, text_value: text.clone(), disabled: unavailable,
                            class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{text}" }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct KeycodeFieldProps {
    context: EditContext,
    request_sequence: Signal<u64>,
    on_change: EventHandler<BindingEditRequest>,
    binding: KeyBinding,
    field: BindingField,
    label: String,
    field_label: String,
    accepted_value: String,
    error: Option<String>,
    disabled: bool,
}

#[component]
fn KeycodeField(props: KeycodeFieldProps) -> Element {
    let mut draft = use_signal(|| props.accepted_value.clone());
    let mut dirty = use_signal(|| false);
    use_effect(use_reactive(
        (&props.accepted_value, &props.error),
        move |(accepted, _)| {
            if !dirty() {
                draft.set(accepted);
            }
        },
    ));
    let mut request_sequence = props.request_sequence;
    let context = props.context.clone();
    let binding = props.binding.clone();
    let field = props.field;
    let on_change = props.on_change;
    let request_field = field;
    let show_error = !dirty() && props.error.is_some();
    let id = format!("keymap-keycode-{}", dioxus::core::current_scope_id().0);
    let error_id = format!("{id}-error");
    let selected = use_memo(move || Some(draft()));
    let custom = draft();
    let custom_option =
        !custom.is_empty() && !KEYCODE_CHOICES.iter().any(|(code, _)| *code == custom);

    rsx! {
        Field { class: "my-0 gap-1.5", disabled: props.disabled, invalid: show_error,
            FieldLabel { html_for: id.clone(), "{props.field_label}" }
            div {
                oninput: move |event: FormEvent| { dirty.set(true); draft.set(event.value()); },
                onfocusout: move |_| {
                    let next = draft().trim().to_owned();
                    if dirty()
                        && let Some(binding) = with_keycode(&binding, if request_field == BindingField::Tap { "tap" } else { "keycode" }, next) {
                            dirty.set(false);
                            emit_change(
                                &context,
                                &mut request_sequence,
                                on_change,
                                field,
                                binding,
                            );
                    }
                },
                Combobox::<String> {
                    id, aria_label: props.label.clone(), list_aria_label: props.label.clone(),
                    class: "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                    placeholder: "Search or enter a ZMK keycode", disabled: props.disabled,
                    value: Some(selected.into()), aria_invalid: show_error,
                    aria_describedby: show_error.then_some(error_id.clone()),
                    on_value_change: move |value: Option<String>| if let Some(value) = value { dirty.set(true); draft.set(value); },
                    filter: move |(query, code): (String, String)| {
                        let query = query.to_lowercase();
                        code.to_lowercase().contains(&query)
                            || KEYCODE_CHOICES.iter().any(|(candidate, title)| *candidate == code && title.to_lowercase().contains(&query))
                    },
                    for (index, (code, title)) in KEYCODE_CHOICES.iter().enumerate() {
                        ComboboxOption::<String> { index, value: *code, text_value: *code,
                            class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{code} · {title}" }
                    }
                    if custom_option {
                        ComboboxOption::<String> { index: KEYCODE_CHOICES.len(), value: custom.clone(), text_value: custom.clone(), class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{custom}" }
                    }
                }
            }
            if show_error && let Some(message) = props.error.as_ref() {
                div { id: error_id, FieldError { "{message}" } }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod choice_floor_tests {
    use super::*;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn fixture() -> Element {
        let mut short = use_signal(|| "a".to_owned());
        let mut long = use_signal(|| "0".to_owned());
        rsx! {
            BindingChoice { label: "Behavior", aria_label: "Behavior", value: short(), disabled: false,
                options: vec![("a".into(), "Alpha".into(), false), ("b".into(), "Beta".into(), false), ("c".into(), "Cherry".into(), false)],
                on_change: move |value| short.set(value), }
            BindingChoice { label: "Layer", aria_label: "Layer", value: long(), disabled: false,
                options: (0..13).map(|i| (i.to_string(), format!("Layer {i}"), false)).collect(),
                on_change: move |value| long.set(value), }
        }
    }

    fn key(element: &web_sys::HtmlElement, value: &str) {
        let event = web_sys::KeyboardEventInit::new();
        event.set_key(value);
        event.set_bubbles(true);
        event.set_cancelable(true);
        element
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                    .unwrap(),
            )
            .unwrap();
    }

    #[wasm_bindgen_test]
    async fn short_and_long_choices_keep_their_keyboard_floor_and_names() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
        wait_until("choices render", || {
            by_role_name(&root, "button", "Behavior").is_some()
                && by_role_name(&root, "combobox", "Layer").is_some()
        })
        .await;
        let trigger = by_role_name(&root, "button", "Behavior").unwrap();
        let field = trigger.closest("[data-slot=field]").unwrap().unwrap();
        assert!(
            field
                .get_attribute("class")
                .unwrap_or_default()
                .split_whitespace()
                .any(|class| class == "my-0"),
            "binding fields retain zero vertical margins without legacy CSS"
        );
        trigger.focus().unwrap();
        key(&trigger, "Enter");
        wait_until("behavior list opens", || {
            by_role_name(&root, "listbox", "Behavior").is_some()
        })
        .await;
        let list = by_role_name(&root, "listbox", "Behavior").unwrap();
        key(&list, "End");
        wait_until("last behavior focused", || {
            by_role_name(&root, "option", "Cherry")
                .is_some_and(|option| option.matches(":focus").unwrap())
        })
        .await;
        key(&by_role_name(&root, "option", "Cherry").unwrap(), "Home");
        wait_until("first behavior focused", || {
            by_role_name(&root, "option", "Alpha")
                .is_some_and(|option| option.matches(":focus").unwrap())
        })
        .await;
        key(&by_role_name(&root, "option", "Alpha").unwrap(), "b");
        wait_until("behavior typeahead focuses Beta", || {
            by_role_name(&root, "option", "Beta")
                .is_some_and(|option| option.matches(":focus").unwrap())
        })
        .await;
        key(&by_role_name(&root, "option", "Beta").unwrap(), "Enter");
        wait_until("selection returns focus", || {
            trigger.matches(":focus").unwrap()
                && trigger
                    .text_content()
                    .is_some_and(|text| text.trim() == "Beta")
        })
        .await;
        key(&trigger, "Enter");
        wait_until("behavior reopens", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        key(
            &by_role_name(&root, "listbox", "Behavior").unwrap(),
            "Escape",
        );
        wait_until("behavior escape returns focus", || {
            trigger.matches(":focus").unwrap()
                && trigger.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;

        let input = by_role_name(&root, "combobox", "Layer").unwrap();
        input.focus().unwrap();
        key(&input, "ArrowDown");
        wait_until("layer list opens", || {
            by_role_name(&root, "listbox", "Layer").is_some()
        })
        .await;
        let native: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
        native.set_value("Layer 12");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        native
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        wait_until("layer list filters", || {
            by_role_name(&root, "option", "Layer 12").is_some()
                && by_role_name(&root, "option", "Layer 0").is_none()
        })
        .await;
        key(&input, "ArrowDown");
        wait_until("layer active descendant", || {
            by_role_name(&root, "option", "Layer 12").is_some_and(|option| {
                input.get_attribute("aria-activedescendant").as_deref()
                    == Some(option.id().as_str())
            })
        })
        .await;
        key(&input, "Enter");
        wait_until("layer is selected", || native.value() == "Layer 12").await;
        key(&input, "ArrowDown");
        wait_until("layer list reopens", || {
            input.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        key(&input, "Escape");
        wait_until("layer escape retains focus", || {
            input.matches(":focus").unwrap()
                && input.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;
        root.remove();
    }
}
