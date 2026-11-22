//! Pure KeyBinding editor model: identities, behavior selection, and field helpers.
//!
//! This module compiles on every target so its synchronous tests run as
//! ordinary host `#[test]`. The wasm-only `keymap::binding_editor` keeps the
//! mounted components and re-exports these names for its existing callers.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{EncoderDirection, KeyBinding};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Behavior {
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
    pub(crate) const ALL: [(Self, &'static str); 11] = [
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

    pub(crate) fn value(self) -> &'static str {
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

    pub(crate) fn from_value(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find_map(|(behavior, _)| (behavior.value() == value).then_some(*behavior))
    }

    pub(crate) fn from_binding(binding: &KeyBinding) -> Self {
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

pub(crate) fn default_layer_id(layers: &[BindingLayerChoice]) -> String {
    layers
        .get(1)
        .or_else(|| layers.first())
        .map_or_else(|| "base".into(), |layer| layer.id.clone())
}

pub(crate) fn choose_behavior(
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

pub(crate) fn binding_keycode(binding: &KeyBinding) -> Option<(&'static str, &'static str, &str)> {
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

pub(crate) fn with_keycode(binding: &KeyBinding, field: &str, value: String) -> Option<KeyBinding> {
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

pub(crate) fn binding_layer_id(binding: &KeyBinding) -> Option<&str> {
    match binding {
        KeyBinding::LayerTap { layer_id, .. }
        | KeyBinding::MomentaryLayer { layer_id }
        | KeyBinding::ToggleLayer { layer_id }
        | KeyBinding::ToLayer { layer_id }
        | KeyBinding::StickyLayer { layer_id } => Some(layer_id),
        _ => None,
    }
}

pub(crate) fn field_exists(binding: &KeyBinding, field: BindingField) -> bool {
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

pub(crate) fn with_layer_id(binding: &KeyBinding, value: String) -> Option<KeyBinding> {
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

/// A binding field's accepted value, distinguishing typed variants that share a shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AcceptedFieldValue {
    Behavior(&'static str),
    Keycode(&'static str, String),
    Tap(String),
    HoldModifier(String),
    Layer(String),
    Macro(String),
}

pub(crate) fn field_value(binding: &KeyBinding, field: BindingField) -> Option<AcceptedFieldValue> {
    Some(match (field, binding) {
        (BindingField::Behavior, KeyBinding::KeyPress { .. }) => {
            AcceptedFieldValue::Behavior("key-press")
        }
        (BindingField::Behavior, KeyBinding::ModTap { .. }) => {
            AcceptedFieldValue::Behavior("mod-tap")
        }
        (BindingField::Behavior, KeyBinding::LayerTap { .. }) => {
            AcceptedFieldValue::Behavior("layer-tap")
        }
        (BindingField::Behavior, KeyBinding::MomentaryLayer { .. }) => {
            AcceptedFieldValue::Behavior("momentary-layer")
        }
        (BindingField::Behavior, KeyBinding::ToggleLayer { .. }) => {
            AcceptedFieldValue::Behavior("toggle-layer")
        }
        (BindingField::Behavior, KeyBinding::ToLayer { .. }) => {
            AcceptedFieldValue::Behavior("to-layer")
        }
        (BindingField::Behavior, KeyBinding::StickyLayer { .. }) => {
            AcceptedFieldValue::Behavior("sticky-layer")
        }
        (BindingField::Behavior, KeyBinding::StickyKey { .. }) => {
            AcceptedFieldValue::Behavior("sticky-key")
        }
        (BindingField::Behavior, KeyBinding::Macro { .. }) => AcceptedFieldValue::Behavior("macro"),
        (BindingField::Behavior, KeyBinding::Transparent) => {
            AcceptedFieldValue::Behavior("transparent")
        }
        (BindingField::Behavior, KeyBinding::None) => AcceptedFieldValue::Behavior("none"),
        (BindingField::Keycode, KeyBinding::KeyPress { keycode }) => {
            AcceptedFieldValue::Keycode("key-press", keycode.clone())
        }
        (BindingField::Keycode, KeyBinding::StickyKey { keycode }) => {
            AcceptedFieldValue::Keycode("sticky-key", keycode.clone())
        }
        (BindingField::Tap, KeyBinding::ModTap { tap, .. })
        | (BindingField::Tap, KeyBinding::LayerTap { tap, .. }) => {
            AcceptedFieldValue::Tap(tap.clone())
        }
        (BindingField::HoldModifier, KeyBinding::ModTap { hold, .. }) => {
            AcceptedFieldValue::HoldModifier(hold.clone())
        }
        (BindingField::Layer, KeyBinding::LayerTap { layer_id, .. })
        | (BindingField::Layer, KeyBinding::MomentaryLayer { layer_id })
        | (BindingField::Layer, KeyBinding::ToggleLayer { layer_id })
        | (BindingField::Layer, KeyBinding::ToLayer { layer_id })
        | (BindingField::Layer, KeyBinding::StickyLayer { layer_id }) => {
            AcceptedFieldValue::Layer(layer_id.clone())
        }
        (BindingField::Macro, KeyBinding::Macro { macro_id }) => {
            AcceptedFieldValue::Macro(macro_id.clone())
        }
        _ => return None,
    })
}

/// Apply only the requested semantic field to the freshly admitted binding.
pub(crate) fn apply_requested_field(
    current: &KeyBinding,
    requested: &KeyBinding,
    field: BindingField,
) -> Option<KeyBinding> {
    Some(match (field, current, requested) {
        (BindingField::Behavior, _, next) if field_value(next, field).is_some() => next.clone(),
        (BindingField::Keycode, KeyBinding::KeyPress { .. }, KeyBinding::KeyPress { keycode }) => {
            KeyBinding::KeyPress {
                keycode: keycode.clone(),
            }
        }
        (
            BindingField::Keycode,
            KeyBinding::StickyKey { .. },
            KeyBinding::StickyKey { keycode },
        ) => KeyBinding::StickyKey {
            keycode: keycode.clone(),
        },
        (BindingField::Tap, KeyBinding::ModTap { hold, .. }, KeyBinding::ModTap { tap, .. }) => {
            KeyBinding::ModTap {
                hold: hold.clone(),
                tap: tap.clone(),
            }
        }
        (
            BindingField::Tap,
            KeyBinding::LayerTap { layer_id, .. },
            KeyBinding::LayerTap { tap, .. },
        ) => KeyBinding::LayerTap {
            layer_id: layer_id.clone(),
            tap: tap.clone(),
        },
        (
            BindingField::HoldModifier,
            KeyBinding::ModTap { tap, .. },
            KeyBinding::ModTap { hold, .. },
        ) => KeyBinding::ModTap {
            hold: hold.clone(),
            tap: tap.clone(),
        },
        (
            BindingField::Layer,
            KeyBinding::LayerTap { tap, .. },
            KeyBinding::LayerTap { layer_id, .. },
        ) => KeyBinding::LayerTap {
            layer_id: layer_id.clone(),
            tap: tap.clone(),
        },
        (
            BindingField::Layer,
            KeyBinding::MomentaryLayer { .. },
            KeyBinding::MomentaryLayer { layer_id },
        ) => KeyBinding::MomentaryLayer {
            layer_id: layer_id.clone(),
        },
        (
            BindingField::Layer,
            KeyBinding::ToggleLayer { .. },
            KeyBinding::ToggleLayer { layer_id },
        ) => KeyBinding::ToggleLayer {
            layer_id: layer_id.clone(),
        },
        (BindingField::Layer, KeyBinding::ToLayer { .. }, KeyBinding::ToLayer { layer_id }) => {
            KeyBinding::ToLayer {
                layer_id: layer_id.clone(),
            }
        }
        (
            BindingField::Layer,
            KeyBinding::StickyLayer { .. },
            KeyBinding::StickyLayer { layer_id },
        ) => KeyBinding::StickyLayer {
            layer_id: layer_id.clone(),
        },
        (BindingField::Macro, KeyBinding::Macro { .. }, KeyBinding::Macro { macro_id }) => {
            KeyBinding::Macro {
                macro_id: macro_id.clone(),
            }
        }
        _ => return None,
    })
}

pub(crate) const HOLD_MODIFIERS: [&str; 8] = [
    "LSHIFT", "RSHIFT", "LCTRL", "RCTRL", "LALT", "RALT", "LGUI", "RGUI",
];

pub(crate) const KEYCODE_CHOICES: &[(&str, &str)] = &[
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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

    #[test]
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

    #[test]
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

    #[test]
    fn field_keys_cover_all_typed_variants_without_collapsing_empty_kinds() {
        let layer_id = "nav-stable-id".to_owned();
        let values = [
            KeyBinding::KeyPress {
                keycode: "A".into(),
            },
            KeyBinding::ModTap {
                hold: "LSHIFT".into(),
                tap: "A".into(),
            },
            KeyBinding::LayerTap {
                layer_id: layer_id.clone(),
                tap: "SPACE".into(),
            },
            KeyBinding::MomentaryLayer {
                layer_id: layer_id.clone(),
            },
            KeyBinding::ToggleLayer {
                layer_id: layer_id.clone(),
            },
            KeyBinding::ToLayer {
                layer_id: layer_id.clone(),
            },
            KeyBinding::StickyLayer { layer_id },
            KeyBinding::StickyKey {
                keycode: "LC(A)".into(),
            },
            KeyBinding::Macro {
                macro_id: "macro-stable-id".into(),
            },
            KeyBinding::Transparent,
            KeyBinding::None,
        ];

        assert!(
            values
                .iter()
                .all(|binding| field_value(binding, BindingField::Behavior).is_some())
        );
        assert!(field_value(&values[0], BindingField::Keycode).is_some());
        assert!(field_value(&values[7], BindingField::Keycode).is_some());
        assert!(field_value(&values[1], BindingField::Tap).is_some());
        assert!(field_value(&values[2], BindingField::Tap).is_some());
        assert!(field_value(&values[1], BindingField::HoldModifier).is_some());
        for binding in &values[2..7] {
            assert!(field_value(binding, BindingField::Layer).is_some());
        }
        assert!(field_value(&values[8], BindingField::Macro).is_some());
        assert_ne!(
            field_value(&KeyBinding::Transparent, BindingField::Behavior),
            field_value(&KeyBinding::None, BindingField::Behavior)
        );
    }

    #[test]
    fn field_updates_preserve_sibling_values_from_fresh_binding() {
        let current = KeyBinding::ModTap {
            hold: "LCTRL".into(),
            tap: "A".into(),
        };
        let stale_payload = KeyBinding::ModTap {
            hold: "LSHIFT".into(),
            tap: "B".into(),
        };
        assert_eq!(
            apply_requested_field(&current, &stale_payload, BindingField::Tap),
            Some(KeyBinding::ModTap {
                hold: "LCTRL".into(),
                tap: "B".into(),
            })
        );
        assert_eq!(
            apply_requested_field(&current, &stale_payload, BindingField::HoldModifier),
            Some(KeyBinding::ModTap {
                hold: "LSHIFT".into(),
                tap: "A".into(),
            })
        );
        assert_eq!(
            apply_requested_field(&current, &KeyBinding::None, BindingField::Tap),
            None
        );
    }
}
