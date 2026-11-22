//! Pure KeyBinding editor model: behavior selection and keycode catalogs.
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

    pub(crate) fn from_value(v: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find_map(|(b, _)| (b.value() == v).then_some(*b))
    }

    pub(crate) fn from_binding(b: &KeyBinding) -> Self {
        match b {
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

pub(crate) fn choose_behavior(
    b: Behavior,
    layers: &[BindingLayerChoice],
    macros: &[BindingMacroChoice],
) -> KeyBinding {
    let layer_id = layers
        .get(1)
        .or_else(|| layers.first())
        .map_or_else(|| "base".into(), |l| l.id.clone());
    match b {
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
            macro_id: macros.first().map_or_else(String::new, |m| m.id.clone()),
        },
        Behavior::Transparent => KeyBinding::Transparent,
        Behavior::None => KeyBinding::None,
    }
}

pub(crate) fn with_keycode(b: &KeyBinding, value: String) -> Option<KeyBinding> {
    match b {
        KeyBinding::KeyPress { .. } => Some(KeyBinding::KeyPress { keycode: value }),
        KeyBinding::StickyKey { .. } => Some(KeyBinding::StickyKey { keycode: value }),
        KeyBinding::ModTap { hold, .. } => Some(KeyBinding::ModTap {
            hold: hold.clone(),
            tap: value,
        }),
        KeyBinding::LayerTap { layer_id, .. } => Some(KeyBinding::LayerTap {
            layer_id: layer_id.clone(),
            tap: value,
        }),
        _ => None,
    }
}

pub(crate) fn with_layer_id(b: &KeyBinding, value: String) -> Option<KeyBinding> {
    match b {
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
    fn behavior_round_trip_and_field_updates() {
        let layers = vec![
            BindingLayerChoice {
                id: "base".into(),
                name: "Base".into(),
            },
            BindingLayerChoice {
                id: "fn".into(),
                name: "Fn".into(),
            },
        ];
        let macros = vec![BindingMacroChoice {
            id: "m1".into(),
            name: "M1".into(),
        }];
        for (b, _) in Behavior::ALL {
            assert_eq!(Behavior::from_value(b.value()), Some(b));
            let binding = choose_behavior(b, &layers, &macros);
            assert_eq!(Behavior::from_binding(&binding), b);
        }
        let mt = choose_behavior(Behavior::ModTap, &layers, &macros);
        assert_eq!(
            with_keycode(&mt, "B".into()),
            Some(KeyBinding::ModTap {
                hold: "LSHIFT".into(),
                tap: "B".into()
            })
        );
        let lt = choose_behavior(Behavior::LayerTap, &layers, &macros);
        assert_eq!(
            with_layer_id(&lt, "base".into()),
            Some(KeyBinding::LayerTap {
                layer_id: "base".into(),
                tap: "SPACE".into()
            })
        );
    }
}
