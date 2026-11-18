use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeyBinding {
    KeyPress {
        keycode: String,
    },
    ModTap {
        hold: String,
        tap: String,
    },
    LayerTap {
        #[serde(rename = "layerId")]
        layer_id: String,
        tap: String,
    },
    MomentaryLayer {
        #[serde(rename = "layerId")]
        layer_id: String,
    },
    ToggleLayer {
        #[serde(rename = "layerId")]
        layer_id: String,
    },
    ToLayer {
        #[serde(rename = "layerId")]
        layer_id: String,
    },
    StickyLayer {
        #[serde(rename = "layerId")]
        layer_id: String,
    },
    StickyKey {
        keycode: String,
    },
    Macro {
        #[serde(rename = "macroId")]
        macro_id: String,
    },
    Transparent,
    None,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MacroStep {
    Tap { binding: KeyBinding },
    Press { binding: KeyBinding },
    Release { binding: KeyBinding },
    Wait { ms: u32 },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeymapMacro {
    pub id: String,
    pub name: String,
    pub steps: Vec<MacroStep>,
    pub tap_ms: u32,
    pub wait_ms: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncoderBinding {
    pub clockwise: KeyBinding,
    pub counterclockwise: KeyBinding,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeymapLayer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub bindings: BTreeMap<String, KeyBinding>,
    #[serde(default)]
    pub sensors: BTreeMap<String, EncoderBinding>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KeymapConfiguration {
    pub layers: Vec<KeymapLayer>,
    #[serde(default)]
    pub macros: Vec<KeymapMacro>,
}
impl Default for KeymapConfiguration {
    fn default() -> Self {
        Self {
            layers: vec![KeymapLayer {
                id: "base".into(),
                name: "Base".into(),
                bindings: BTreeMap::new(),
                sensors: BTreeMap::new(),
            }],
            macros: vec![],
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KeymapChange {
    EditMacro {
        #[serde(rename = "macroId")]
        macro_id: String,
        change: MacroChange,
    },
    Binding {
        #[serde(rename = "layerId")]
        layer_id: String,
        #[serde(rename = "keyId")]
        key_id: String,
        binding: KeyBinding,
    },
    AddLayer {
        id: String,
        name: String,
    },
    RenameLayer {
        id: String,
        name: String,
    },
    RemoveLayer {
        id: String,
    },
    SaveMacro {
        value: KeymapMacro,
    },
    RemoveMacro {
        id: String,
    },
    Encoder {
        #[serde(rename = "layerId")]
        layer_id: String,
        #[serde(rename = "encoderId")]
        encoder_id: String,
        direction: EncoderDirection,
        binding: KeyBinding,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncoderDirection {
    Clockwise,
    Counterclockwise,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MacroChange {
    Name { value: String },
    TapMs { value: u32 },
    WaitMs { value: u32 },
    AddStep { value: MacroStep },
    Step { index: usize, value: MacroStep },
    RemoveStep { index: usize },
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub fn name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\' || b == b' ')
}
/// Safe keycode/modifier expressions such as LC(LS(A)); no DTS delimiters.
pub fn keycode(value: &str) -> bool {
    fn expression(s: &str, depth: u8) -> bool {
        if depth > 8 || s.is_empty() {
            return false;
        }
        if let Some(i) = s.find('(') {
            let wrappers = ["LS", "RS", "LC", "RC", "LA", "RA", "LG", "RG"];
            s.ends_with(')')
                && wrappers.contains(&&s[..i])
                && expression(&s[i + 1..s.len() - 1], depth + 1)
        } else {
            s.len() <= 48
                && s.bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        }
    }
    expression(value, 0)
}
pub fn legacy_binding(value: &str) -> bool {
    matches!(value, "&none" | "&trans") || value.strip_prefix("&kp ").is_some_and(keycode)
}
fn binding(binding: &KeyBinding, map: &KeymapConfiguration) -> Result<String, String> {
    let code = |value: &str| {
        if keycode(value) {
            Ok(value.to_string())
        } else {
            Err(format!("Invalid keycode expression {value}"))
        }
    };
    let layer = |id: &str| {
        map.layers
            .iter()
            .position(|l| l.id == id)
            .ok_or_else(|| format!("Unknown layer {id}"))
    };
    Ok(match binding {
        KeyBinding::KeyPress { keycode } => format!("&kp {}", code(keycode)?),
        KeyBinding::ModTap { hold, tap } => format!("&mt {} {}", code(hold)?, code(tap)?),
        KeyBinding::LayerTap { layer_id, tap } => {
            format!("&lt {} {}", layer(layer_id)?, code(tap)?)
        }
        KeyBinding::MomentaryLayer { layer_id } => format!("&mo {}", layer(layer_id)?),
        KeyBinding::ToggleLayer { layer_id } => format!("&tog {}", layer(layer_id)?),
        KeyBinding::ToLayer { layer_id } => format!("&to {}", layer(layer_id)?),
        KeyBinding::StickyLayer { layer_id } => format!("&sl {}", layer(layer_id)?),
        KeyBinding::StickyKey { keycode } => format!("&sk {}", code(keycode)?),
        KeyBinding::Macro { macro_id } => format!(
            "&bs_macro_{}",
            map.macros
                .iter()
                .position(|m| m.id == *macro_id)
                .ok_or_else(|| format!("Unknown macro {macro_id}"))?
        ),
        KeyBinding::Transparent => "&trans".into(),
        KeyBinding::None => "&none".into(),
    })
}
pub fn validate(map: &KeymapConfiguration) -> Result<(), String> {
    if map.layers.is_empty() || map.layers.len() > 32 {
        return Err("Keymap needs 1–32 layers".into());
    }
    let mut ids = BTreeSet::new();
    for layer in &map.layers {
        if !identifier(&layer.id) || !name(&layer.name) || !ids.insert(&layer.id) {
            return Err("Layers need unique ids and valid names of 1–32 characters".into());
        }
        for value in layer.bindings.values() {
            binding(value, map)?;
        }
        for value in layer.sensors.values() {
            binding(&value.clockwise, map)?;
            binding(&value.counterclockwise, map)?;
        }
    }
    ids.clear();
    if map.macros.len() > 128 {
        return Err("At most 128 macros are supported".into());
    }
    for mac in &map.macros {
        if !identifier(&mac.id)
            || !name(&mac.name)
            || !ids.insert(&mac.id)
            || mac.steps.is_empty()
            || mac.steps.len() > 128
            || mac.tap_ms > 10000
            || mac.wait_ms > 10000
        {
            return Err(
                "Macros require unique ids, a name, 1–128 steps and timings within 10000 ms".into(),
            );
        }
        let cells: usize = mac
            .steps
            .iter()
            .map(|step| {
                if matches!(step, MacroStep::Wait { .. }) {
                    4
                } else {
                    2
                }
            })
            .sum();
        if cells > 256 {
            return Err("Macro exceeds ZMK's 256 expanded binding limit".into());
        }
        for step in &mac.steps {
            match step {
                MacroStep::Wait { ms } if *ms > 10000 => {
                    return Err("Macro wait exceeds 10000 ms".into());
                }
                MacroStep::Wait { .. } => {}
                MacroStep::Tap { binding: b }
                | MacroStep::Press { binding: b }
                | MacroStep::Release { binding: b } => {
                    if matches!(b, KeyBinding::Macro { .. }) {
                        return Err("Nested macro calls are not supported".into());
                    }
                    binding(b, map)?;
                }
            }
        }
    }
    Ok(())
}
pub fn source(
    map: &KeymapConfiguration,
    keys: &[String],
    legacy: &[String],
    encoders: &[String],
) -> Result<String, String> {
    validate(map)?;
    let mut nodes = String::new();
    for (i, mac) in map.macros.iter().enumerate() {
        let steps = mac
            .steps
            .iter()
            .map(|step| {
                Ok(match step {
                    MacroStep::Wait { ms } => format!(
                        "<&macro_wait_time {ms}>, <&macro_press &none>, <&macro_wait_time {}>",
                        mac.wait_ms
                    ),
                    MacroStep::Tap { binding: b } => {
                        format!("<&macro_tap>, <{}>", binding(b, map)?)
                    }
                    MacroStep::Press { binding: b } => {
                        format!("<&macro_press>, <{}>", binding(b, map)?)
                    }
                    MacroStep::Release { binding: b } => {
                        format!("<&macro_release>, <{}>", binding(b, map)?)
                    }
                })
            })
            .collect::<Result<Vec<_>, String>>()?
            .join(", ");
        nodes += &format!(
            "bs_macro_{i}: bs_macro_{i} {{ compatible = \"zmk,behavior-macro\"; #binding-cells = <0>; tap-ms = <{}>; wait-ms = <{}>; bindings = {steps}; }};\n",
            mac.tap_ms, mac.wait_ms
        );
    }
    let mut layers = String::new();
    for (i, layer) in map.layers.iter().enumerate() {
        let values = keys
            .iter()
            .enumerate()
            .map(|(k, id)| match layer.bindings.get(id) {
                Some(b) => binding(b, map),
                None => Ok(if i == 0 {
                    legacy.get(k).cloned().unwrap_or("&none".into())
                } else {
                    "&trans".into()
                }),
            })
            .collect::<Result<Vec<_>, String>>()?
            .join(" ");
        let mut sensors = vec![];
        for (j, id) in encoders.iter().enumerate() {
            let default = EncoderBinding {
                clockwise: KeyBinding::None,
                counterclockwise: KeyBinding::None,
            };
            let b = layer.sensors.get(id).unwrap_or(&default);
            let label = format!("bs_sensor_{i}_{j}");
            nodes += &format!(
                "{label}: {label} {{ compatible = \"zmk,behavior-sensor-rotate\"; #sensor-binding-cells = <0>; bindings = <{}>, <{}>; }};\n",
                binding(&b.clockwise, map)?,
                binding(&b.counterclockwise, map)?
            );
            sensors.push(format!("&{label}"));
        }
        let sensor = if sensors.is_empty() {
            String::new()
        } else {
            format!("sensor-bindings = <{}>;", sensors.join(" "))
        };
        layers += &format!(
            "layer_{i} {{ label = \"{}\"; bindings = <{values}>; {sensor} }};\n",
            layer.name
        );
    }
    Ok(format!(
        "#include <behaviors.dtsi>\n#include <dt-bindings/zmk/keys.h>\n/ {{ behaviors {{ {nodes} }}; keymap {{ compatible = \"zmk,keymap\"; {layers} }}; }};\n"
    ))
}
