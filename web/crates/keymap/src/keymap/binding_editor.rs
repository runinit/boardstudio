//! Private, typed editor for one accepted Keymap binding.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{EncoderDirection, KeyBinding};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingLayerChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingMacroChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingField {
    Behavior,
    Keycode,
    Tap,
    HoldModifier,
    Layer,
    Macro,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncoderInputIdentity {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub projection_generation: u64,
    pub electrical_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingTarget {
    Key {
        key_id: String,
    },
    EncoderRotation {
        encoder_id: String,
        direction: EncoderDirection,
    },
    EncoderPush {
        encoder_id: String,
        key_id: String,
    },
}

impl BindingTarget {
    /// Stable, collision-free component/DOM identity for this complete target.
    pub fn stable_key(&self) -> String {
        match self {
            Self::Key { key_id } => format!("key-{}", hex_bytes(key_id)),
            Self::EncoderRotation {
                encoder_id,
                direction,
            } => format!(
                "rotation-{}-{}",
                match direction {
                    EncoderDirection::Clockwise => "clockwise",
                    EncoderDirection::Counterclockwise => "counterclockwise",
                },
                hex_bytes(encoder_id),
            ),
            Self::EncoderPush { encoder_id, key_id } => {
                format!("push-{}-{}", hex_bytes(encoder_id), hex_bytes(key_id))
            }
        }
    }
}

fn hex_bytes(value: &str) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingEditRequest {
    pub scope: Scope,
    pub admission_token: SnapshotToken,
    pub admission_revision: u64,
    pub active_layer_id: String,
    pub target: BindingTarget,
    pub input_identity: Option<EncoderInputIdentity>,
    pub field: BindingField,
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub binding: KeyBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingEditStatus {
    Pending,
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingEditFeedback {
    pub scope: Scope,
    pub admission_token: SnapshotToken,
    pub admission_revision: u64,
    pub active_layer_id: String,
    pub target: BindingTarget,
    pub input_identity: Option<EncoderInputIdentity>,
    pub field: BindingField,
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub status: BindingEditStatus,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behavior {
    KeyPress,
    ModTap,
    LayerTap,
    MomentaryLayer,
    ToggleLayer,
    ToLayer,
    StickyLayer,
    StickyKey,
    Macro,
    Transparent,
    None,
}

impl Behavior {
    const ALL: [(Self, &'static str); 11] = [
        (Self::KeyPress, "Key press"),
        (Self::ModTap, "Mod tap"),
        (Self::LayerTap, "Layer tap"),
        (Self::MomentaryLayer, "Momentary layer"),
        (Self::ToggleLayer, "Toggle layer"),
        (Self::ToLayer, "Go to layer"),
        (Self::StickyLayer, "Sticky layer"),
        (Self::StickyKey, "Sticky key"),
        (Self::Macro, "Macro"),
        (Self::Transparent, "Transparent"),
        (Self::None, "Unassigned"),
    ];

    fn value(self) -> &'static str {
        match self {
            Self::KeyPress => "key-press",
            Self::ModTap => "mod-tap",
            Self::LayerTap => "layer-tap",
            Self::MomentaryLayer => "momentary-layer",
            Self::ToggleLayer => "toggle-layer",
            Self::ToLayer => "to-layer",
            Self::StickyLayer => "sticky-layer",
            Self::StickyKey => "sticky-key",
            Self::Macro => "macro",
            Self::Transparent => "transparent",
            Self::None => "none",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find_map(|(behavior, _)| (behavior.value() == value).then_some(*behavior))
    }

    fn from_binding(binding: &KeyBinding) -> Self {
        match binding {
            KeyBinding::KeyPress { .. } => Self::KeyPress,
            KeyBinding::ModTap { .. } => Self::ModTap,
            KeyBinding::LayerTap { .. } => Self::LayerTap,
            KeyBinding::MomentaryLayer { .. } => Self::MomentaryLayer,
            KeyBinding::ToggleLayer { .. } => Self::ToggleLayer,
            KeyBinding::ToLayer { .. } => Self::ToLayer,
            KeyBinding::StickyLayer { .. } => Self::StickyLayer,
            KeyBinding::StickyKey { .. } => Self::StickyKey,
            KeyBinding::Macro { .. } => Self::Macro,
            KeyBinding::Transparent => Self::Transparent,
            KeyBinding::None => Self::None,
        }
    }
}

fn default_layer_id(layers: &[BindingLayerChoice]) -> String {
    layers
        .get(1)
        .or_else(|| layers.first())
        .map_or_else(|| "base".into(), |layer| layer.id.clone())
}

fn choose_behavior(
    behavior: Behavior,
    layers: &[BindingLayerChoice],
    macros: &[BindingMacroChoice],
) -> KeyBinding {
    let layer_id = default_layer_id(layers);
    match behavior {
        Behavior::KeyPress => KeyBinding::KeyPress {
            keycode: "A".into(),
        },
        Behavior::ModTap => KeyBinding::ModTap {
            hold: "LSHIFT".into(),
            tap: "A".into(),
        },
        Behavior::LayerTap => KeyBinding::LayerTap {
            layer_id,
            tap: "SPACE".into(),
        },
        Behavior::MomentaryLayer => KeyBinding::MomentaryLayer { layer_id },
        Behavior::ToggleLayer => KeyBinding::ToggleLayer { layer_id },
        Behavior::ToLayer => KeyBinding::ToLayer { layer_id },
        Behavior::StickyLayer => KeyBinding::StickyLayer { layer_id },
        Behavior::StickyKey => KeyBinding::StickyKey {
            keycode: "A".into(),
        },
        Behavior::Macro => KeyBinding::Macro {
            macro_id: macros
                .first()
                .map_or_else(String::new, |item| item.id.clone()),
        },
        Behavior::Transparent => KeyBinding::Transparent,
        Behavior::None => KeyBinding::None,
    }
}

fn binding_keycode(binding: &KeyBinding) -> Option<(&'static str, &'static str, &str)> {
    match binding {
        KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => {
            Some(("Keycode", "keycode", keycode))
        }
        KeyBinding::ModTap { tap, .. } | KeyBinding::LayerTap { tap, .. } => {
            Some(("Tap keycode", "tap", tap))
        }
        _ => None,
    }
}

fn with_keycode(binding: &KeyBinding, field: &str, value: String) -> Option<KeyBinding> {
    match (binding, field) {
        (KeyBinding::KeyPress { .. }, "keycode") => Some(KeyBinding::KeyPress { keycode: value }),
        (KeyBinding::StickyKey { .. }, "keycode") => Some(KeyBinding::StickyKey { keycode: value }),
        (KeyBinding::ModTap { hold, .. }, "tap") => Some(KeyBinding::ModTap {
            hold: hold.clone(),
            tap: value,
        }),
        (KeyBinding::LayerTap { layer_id, .. }, "tap") => Some(KeyBinding::LayerTap {
            layer_id: layer_id.clone(),
            tap: value,
        }),
        _ => None,
    }
}

fn binding_layer_id(binding: &KeyBinding) -> Option<&str> {
    match binding {
        KeyBinding::LayerTap { layer_id, .. }
        | KeyBinding::MomentaryLayer { layer_id }
        | KeyBinding::ToggleLayer { layer_id }
        | KeyBinding::ToLayer { layer_id }
        | KeyBinding::StickyLayer { layer_id } => Some(layer_id),
        _ => None,
    }
}

fn field_exists(binding: &KeyBinding, field: BindingField) -> bool {
    match field {
        BindingField::Behavior => true,
        BindingField::Keycode => {
            matches!(
                binding,
                KeyBinding::KeyPress { .. } | KeyBinding::StickyKey { .. }
            )
        }
        BindingField::Tap => {
            matches!(
                binding,
                KeyBinding::ModTap { .. } | KeyBinding::LayerTap { .. }
            )
        }
        BindingField::HoldModifier => matches!(binding, KeyBinding::ModTap { .. }),
        BindingField::Layer => binding_layer_id(binding).is_some(),
        BindingField::Macro => matches!(binding, KeyBinding::Macro { .. }),
    }
}

fn with_layer_id(binding: &KeyBinding, value: String) -> Option<KeyBinding> {
    match binding {
        KeyBinding::LayerTap { tap, .. } => Some(KeyBinding::LayerTap {
            layer_id: value,
            tap: tap.clone(),
        }),
        KeyBinding::MomentaryLayer { .. } => Some(KeyBinding::MomentaryLayer { layer_id: value }),
        KeyBinding::ToggleLayer { .. } => Some(KeyBinding::ToggleLayer { layer_id: value }),
        KeyBinding::ToLayer { .. } => Some(KeyBinding::ToLayer { layer_id: value }),
        KeyBinding::StickyLayer { .. } => Some(KeyBinding::StickyLayer { layer_id: value }),
        _ => None,
    }
}

const HOLD_MODIFIERS: [&str; 8] = [
    "LSHIFT", "RSHIFT", "LCTRL", "RCTRL", "LALT", "RALT", "LGUI", "RGUI",
];

const KEYCODE_CHOICES: &[(&str, &str)] = &[
    ("A", "A"),
    ("B", "B"),
    ("C", "C"),
    ("D", "D"),
    ("E", "E"),
    ("F", "F"),
    ("G", "G"),
    ("H", "H"),
    ("I", "I"),
    ("J", "J"),
    ("K", "K"),
    ("L", "L"),
    ("M", "M"),
    ("N", "N"),
    ("O", "O"),
    ("P", "P"),
    ("Q", "Q"),
    ("R", "R"),
    ("S", "S"),
    ("T", "T"),
    ("U", "U"),
    ("V", "V"),
    ("W", "W"),
    ("X", "X"),
    ("Y", "Y"),
    ("Z", "Z"),
    ("N0", "0"),
    ("N1", "1"),
    ("N2", "2"),
    ("N3", "3"),
    ("N4", "4"),
    ("N5", "5"),
    ("N6", "6"),
    ("N7", "7"),
    ("N8", "8"),
    ("N9", "9"),
    ("F1", "F1"),
    ("F2", "F2"),
    ("F3", "F3"),
    ("F4", "F4"),
    ("F5", "F5"),
    ("F6", "F6"),
    ("F7", "F7"),
    ("F8", "F8"),
    ("F9", "F9"),
    ("F10", "F10"),
    ("F11", "F11"),
    ("F12", "F12"),
    ("SPACE", "Space"),
    ("ENTER", "Enter"),
    ("ESC", "Esc"),
    ("TAB", "Tab"),
    ("BSPC", "Backspace"),
    ("LSHFT", "Shift"),
    ("LCTRL", "Ctrl"),
    ("LALT", "Alt"),
    ("LGUI", "Super"),
    ("UP", "Up"),
    ("DOWN", "Down"),
    ("LEFT", "Left"),
    ("RIGHT", "Right"),
    ("MINUS", "-"),
    ("EQUAL", "="),
    ("LBKT", "["),
    ("RBKT", "]"),
    ("BSLH", "\\"),
    ("SEMI", ";"),
    ("SQT", "'"),
    ("COMMA", ","),
    ("DOT", "."),
    ("FSLH", "/"),
    ("GRAVE", "`"),
];

/// Edits a single projected binding and reports typed, target-correlated intents.
#[component]
pub fn BindingEditor(props: BindingEditorProps) -> Element {
    let request_sequence = props.request_sequence;
    let context = EditContext::from(&props);
    let value = super::binding_controller::draft_binding(
        &props.scope,
        &props.active_layer_id,
        &props.target,
        &props.value,
    );
    let behavior = Behavior::from_binding(&value);
    let target_key = props.target.stable_key();
    let datalist_id = format!(
        "m1-keymap-keycodes-{}-{target_key}",
        props.editor_instance_id
    );
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
    let pending =
        feedback.is_some_and(|feedback| matches!(feedback.status, BindingEditStatus::Pending));
    let failed_field = feedback.and_then(|feedback| match &feedback.status {
        BindingEditStatus::Failed(message) => Some((feedback.field, message.as_str())),
        BindingEditStatus::Pending => None,
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
        section { class: "m1-keymap-binding-editor", "aria-label": "Key binding",
            label { class: "m1-keymap-binding-field", "Behavior"
                select {
                    "aria-label": "{key_label} behavior",
                    value: "{behavior.value()}",
                    disabled,
                    onchange: move |event: FormEvent| {
                        if let Some(next_behavior) = Behavior::from_value(&event.value())
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
                    for (candidate, title) in Behavior::ALL {
                        option {
                            key: "{candidate.value()}",
                            value: "{candidate.value()}",
                            selected: candidate == behavior,
                            disabled: candidate == Behavior::Macro && props.macros.is_empty(),
                            "{title}"
                        }
                    }
                }
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
                            datalist_id: datalist_id.clone(),
                            error: code_error.clone(),
                            pending: feedback.is_some_and(|feedback| feedback.field == field && matches!(feedback.status, BindingEditStatus::Pending)),
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
                        label { class: "m1-keymap-binding-field", "Hold modifier"
                            select {
                                "aria-label": "{props.key_label} hold modifier",
                                value: "{current_hold}",
                                disabled,
                                onchange: move |event: FormEvent| {
                                    if let KeyBinding::ModTap { tap, .. } = &hold_binding {
                                        emit_change(
                                            &hold_context,
                                            &mut hold_sequence,
                                            hold_change,
                                            BindingField::HoldModifier,
                                            KeyBinding::ModTap {
                                                hold: event.value(),
                                                tap: tap.clone(),
                                            },
                                        );
                                    }
                                },
                                for modifier in HOLD_MODIFIERS {
                                    option { key: "{modifier}", value: "{modifier}", selected: modifier == current_hold.as_str(), "{modifier}" }
                                }
                            }
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
                        label { class: "m1-keymap-binding-field", "Layer"
                            select {
                                "aria-label": "{props.key_label} layer",
                                value: "{selected_layer_id}",
                                disabled,
                                onchange: move |event: FormEvent| {
                                    if let Some(binding) = with_layer_id(&layer_binding, event.value()) {
                                        emit_change(
                                            &layer_context,
                                            &mut layer_sequence,
                                            layer_change,
                                            BindingField::Layer,
                                            binding,
                                        );
                                    }
                                },
                                for layer in props.layers.iter() {
                                    option {
                                        key: "{layer.id}",
                                        value: "{layer.id}",
                                        selected: layer.id == selected_layer_id,
                                        "{layer.name}"
                                    }
                                }
                            }
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
                        label { class: "m1-keymap-binding-field", "Macro"
                            select {
                                "aria-label": "{props.key_label} macro",
                                value: "{selected_macro_id}",
                                disabled,
                                onchange: move |event: FormEvent| {
                                    emit_change(
                                        &macro_context,
                                        &mut macro_sequence,
                                        macro_change,
                                        BindingField::Macro,
                                        KeyBinding::Macro { macro_id: event.value() },
                                    );
                                },
                                for item in props.macros.iter() {
                                    option {
                                        key: "{item.id}",
                                        value: "{item.id}",
                                        selected: item.id == selected_macro_id,
                                        "{item.name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            datalist { id: "{datalist_id}",
                for (code, title) in KEYCODE_CHOICES {
                    option { value: "{code}", "{title}" }
                }
            }
            if pending { p { class: "m1-keymap-binding-pending", role: "status", "Saving binding…" } }
            if let Some(message) = error { p { class: "m1-keymap-binding-error", role: "alert", "{message}" } }
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
    datalist_id: String,
    error: Option<String>,
    pending: bool,
    disabled: bool,
}

#[component]
fn KeycodeField(props: KeycodeFieldProps) -> Element {
    let mut draft = use_signal(|| props.accepted_value.clone());
    let mut dirty = use_signal(|| false);
    use_effect(use_reactive(
        (&props.accepted_value, &props.pending, &props.error),
        move |(accepted, pending, _)| {
            if !dirty() && !pending {
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
    let datalist_id = props.datalist_id.clone();
    let show_error = !dirty() && props.error.is_some();

    rsx! {
        label { class: "m1-keymap-binding-field", "{props.field_label}"
            input {
                "aria-label": "{props.label}",
                r#type: "text",
                list: "{datalist_id}",
                placeholder: "Search or enter a ZMK keycode",
                "aria-invalid": "{show_error}",
                value: "{draft}",
                disabled: props.disabled,
                oninput: move |event: FormEvent| { dirty.set(true); draft.set(event.value()); },
                onblur: move |_| {
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
            }
        }
        if show_error {
            if let Some(message) = props.error.as_ref() {
                p { class: "m1-keymap-binding-error", role: "alert", "{message}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn binding_targets_have_distinct_stable_identities() {
        let targets = [
            BindingTarget::Key {
                key_id: "encoder:1".into(),
            },
            BindingTarget::EncoderRotation {
                encoder_id: "encoder:1".into(),
                direction: EncoderDirection::Clockwise,
            },
            BindingTarget::EncoderRotation {
                encoder_id: "encoder:1".into(),
                direction: EncoderDirection::Counterclockwise,
            },
            BindingTarget::EncoderPush {
                encoder_id: "encoder:1".into(),
                key_id: "push:1".into(),
            },
        ];
        let ids: std::collections::HashSet<_> =
            targets.iter().map(BindingTarget::stable_key).collect();

        assert_eq!(ids.len(), targets.len());
        assert_ne!(
            BindingTarget::Key { key_id: "a".into() }.stable_key(),
            BindingTarget::Key {
                key_id: "aa".into()
            }
            .stable_key()
        );
    }

    #[wasm_bindgen_test]
    fn layer_defaults_use_stable_non_base_then_first_ids() {
        let layers = [
            BindingLayerChoice {
                id: "primary-id".into(),
                name: "Base".into(),
            },
            BindingLayerChoice {
                id: "nav-id".into(),
                name: "Navigation".into(),
            },
        ];

        assert_eq!(
            choose_behavior(Behavior::LayerTap, &layers, &[]),
            KeyBinding::LayerTap {
                layer_id: "nav-id".into(),
                tap: "SPACE".into(),
            }
        );
        assert_eq!(
            choose_behavior(Behavior::ToLayer, &layers[..1], &[]),
            KeyBinding::ToLayer {
                layer_id: "primary-id".into(),
            }
        );
    }

    #[wasm_bindgen_test]
    fn macro_and_empty_bindings_keep_their_typed_distinction() {
        let macros = [BindingMacroChoice {
            id: "macro-id".into(),
            name: "Tap dance".into(),
        }];

        assert_eq!(
            choose_behavior(Behavior::Macro, &[], &macros),
            KeyBinding::Macro {
                macro_id: "macro-id".into(),
            }
        );
        assert_eq!(
            choose_behavior(Behavior::Macro, &[], &[]),
            KeyBinding::Macro {
                macro_id: String::new(),
            }
        );
        assert_ne!(KeyBinding::Transparent, KeyBinding::None);
    }
}
