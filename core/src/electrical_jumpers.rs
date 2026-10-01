//! Per-part reversible footprint jumper recipes.
//!
//! Recipes are deliberately keyed by reviewed generator sources and explicit
//! parameters. Pad numbers are never used to infer a signal role.
use crate::model::{Part, PartDefinition, Side};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct JumperSite {
    pub id: String,
    pub face: String,
    pub local_net_id: String,
    pub signal_terminal: String,
    pub close: bool,
    pub routing_required: bool,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct JumperDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct JumperRecipe {
    pub part_id: String,
    pub source: String,
    pub sites: Vec<JumperSite>,
    pub terminal_aliases: Vec<(String, String)>,
    pub diagnostics: Vec<JumperDiagnostic>,
}

fn parameter<'a>(
    part: &'a Part,
    definition: &'a PartDefinition,
    name: &str,
) -> Option<&'a serde_json::Value> {
    let value = part
        .generator_parameters
        .as_ref()
        .and_then(|values| values.get(name))
        .or_else(|| {
            definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get(name))
        })?;
    Some(value.get("value").unwrap_or(value))
}
fn option(part: &Part, definition: &PartDefinition, name: &str, default: bool) -> bool {
    parameter(part, definition, name)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}
fn local_net(part: &str, suffix: usize) -> String {
    // Matches encodeURIComponent in the footprint renderer; references are editable.
    let encoded: String = part
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    format!("__boardstudio_local_{encoded}_{suffix}")
}
const MCU_ROWS: [[&str; 2]; 12] = [
    ["P1", "RAW"],
    ["P0", "GND"],
    ["GND", "RST"],
    ["GND", "VCC"],
    ["P2", "P21"],
    ["P3", "P20"],
    ["P4", "P19"],
    ["P5", "P18"],
    ["P6", "P15"],
    ["P7", "P14"],
    ["P8", "P16"],
    ["P9", "P10"],
];

pub fn describe(part: &Part, definition: &PartDefinition) -> Option<JumperRecipe> {
    let source = definition.generator.as_ref()?.source.as_str();
    let infused_kim_display = source == "infused-kim/nice_view";
    if !option(
        part,
        definition,
        if infused_kim_display {
            "reverse"
        } else {
            "reversible"
        },
        false,
    ) {
        return None;
    }
    let installed_front = parameter(part, definition, "side")
        .and_then(serde_json::Value::as_str)
        .map(|side| side == "F")
        .unwrap_or(part.side == Side::Front);
    // The older nice!view source keeps destination nets in header order; the
    // ceoloide footprints reverse them to put the bridges on the opposite face.
    let closing_face = if installed_front == infused_kim_display {
        "front"
    } else {
        "back"
    };
    let traces = !infused_kim_display && option(part, definition, "include_traces", true);
    let mut recipe = JumperRecipe {
        part_id: part.id.clone(),
        source: source.into(),
        sites: vec![],
        terminal_aliases: vec![],
        diagnostics: vec![],
    };
    let mut site = |face: &str, suffix: usize, signal: &str, x: f64, y: f64| {
        recipe.sites.push(JumperSite {
            id: format!("{}/{face}/{suffix}", part.id),
            face: face.into(),
            local_net_id: local_net(&part.id, suffix),
            signal_terminal: signal.into(),
            close: face == closing_face,
            routing_required: !traces,
            x,
            y,
        });
    };
    match source {
        "ceoloide/mcu_nice_nano" | "ceoloide/mcu_supermini_nrf52840" => {
            let reverse = option(part, definition, "reverse_mount", false);
            let reduced = option(part, definition, "only_required_jumpers", false);
            let rectangular = option(part, definition, "use_rectangular_jumpers", false);
            for (row, terminals) in MCU_ROWS.iter().enumerate() {
                let left = terminals[usize::from(!reverse)];
                let right = terminals[usize::from(reverse)];
                if reduced && row >= 4 {
                    // Lower socket rows have no bridges in the reduced variant.
                    // Turning the installed module over swaps their GPIO identities.
                    for index in 0..2 {
                        recipe.terminal_aliases.push((
                            terminals[index].into(),
                            terminals[if installed_front { 1 - index } else { index }].into(),
                        ));
                    }
                    continue;
                }
                let y = -12.7 + row as f64 * 2.54;
                let x = if rectangular { 5.03 } else { 5.1375 };
                site("front", 24 - row, left, -x, y);
                site("front", 1 + row, right, x, y);
                site("back", 24 - row, right, -x, y);
                site("back", 1 + row, left, x, y);
            }
            if option(part, definition, "invert_jumpers_position", false) {
                recipe.diagnostics.push(JumperDiagnostic {
                    code: "unsupported-jumper-variant".into(),
                    severity: "error".into(),
                    message: "This MCU footprint rejects inverted jumper positions".into(),
                });
            }
        }
        "ceoloide/display_nice_view" | "ceoloide/display_ssd1306" => {
            let nice_view = source == "ceoloide/display_nice_view";
            let original: &[&str] = if nice_view {
                &["MOSI", "SCK", "VCC", "GND", "CS"]
            } else {
                &["SDA", "SCL", "VCC", "GND"]
            };
            let count = original.len();
            let offset = if option(part, definition, "invert_jumpers_position", false) {
                4.4
            } else {
                0.0
            };
            for index in 0..count {
                if nice_view && index == 2 {
                    continue;
                }
                let x = (index as f64 - (count - 1) as f64 / 2.0) * 2.54;
                // Back pads reverse their x positions and local-net order.
                site(
                    "front",
                    index + 1,
                    original[count - 1 - index],
                    x,
                    14.5 + offset,
                );
                site(
                    "back",
                    count - index,
                    original[count - 1 - index],
                    -x,
                    14.5 + offset,
                );
            }
        }
        "ceoloide/battery_connector_jst_ph_2" => {
            site("front", 1, "BAT_P", -1.0, 2.308);
            site("front", 2, "BAT_N", 1.0, 2.308);
            site("back", 1, "BAT_N", -1.0, 2.308);
            site("back", 2, "BAT_P", 1.0, 2.308);
        }
        "infused-kim/nice_view" => {
            let signals = ["MOSI", "SCK", "VCC", "GND", "CS"];
            let y = if option(part, definition, "jumpers_at_bottom", false) {
                19.6
            } else {
                13.9
            };
            for (index, signal) in signals.iter().enumerate() {
                if index == 2 {
                    continue;
                }
                let x = (index as f64 - 2.0) * 2.54;
                site("front", index + 1, signal, x, y);
                site("back", 5 - index, signal, -x, y);
            }
        }
        _ => return None,
    }
    if !traces {
        recipe.diagnostics.push(JumperDiagnostic {
            code: "local-routing-required".into(),
            severity: "warning".into(),
            message: "Route socket pads to their jumper pads and signal vias before fabrication"
                .into(),
        });
    }
    Some(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::collections::BTreeMap;
    fn fixture(
        source: &str,
        side: Side,
        params: BTreeMap<String, serde_json::Value>,
    ) -> (Part, PartDefinition) {
        (
            Part {
                id: "u1".into(),
                definition_id: "d".into(),
                reference: "U1".into(),
                pose: Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                },
                side,
                locked: None,
                keycap: None,
                outline: None,
                properties: None,
                generator_parameters: Some(params),
            },
            PartDefinition {
                input_profile: None,
                id: "d".into(),
                name: "d".into(),
                kind: PartKind::Controller,
                keycap: None,
                envelope_source: None,
                kicad_source: None,
                terminals: BTreeMap::new(),
                matrix_terminals: None,
                envelope_notice: None,
                courtyard: vec![],
                pads: vec![],

                models: None,
                generator: Some(PartGenerator {
                    source: source.into(),
                    version: "test".into(),
                    parameters: BTreeMap::new(),
                }),
                mechanical_profile: None,
            },
        )
    }
    #[test]
    fn mcu_full_and_reduced_sites_are_deterministic() {
        let mut all = BTreeMap::new();
        all.insert("reversible".into(), serde_json::json!(true));
        let (p, d) = fixture("ceoloide/mcu_nice_nano", Side::Front, all);
        assert_eq!(describe(&p, &d).unwrap().sites.len(), 48);
        let mut q = BTreeMap::new();
        q.insert("reversible".into(), serde_json::json!(true));
        q.insert("only_required_jumpers".into(), serde_json::json!(true));
        let (p, d) = fixture("ceoloide/mcu_nice_nano", Side::Back, q);
        let r = describe(&p, &d).unwrap();
        assert_eq!(r.sites.len(), 16);
        assert!(r.sites.iter().any(|s| s.close));
    }
    #[test]
    fn extra_pins_are_never_fabricated_jumper_sites() {
        let mut q = BTreeMap::new();
        q.insert("reversible".into(), serde_json::json!(true));
        q.insert("include_extra_pins".into(), serde_json::json!(true));
        let (p, d) = fixture("ceoloide/mcu_supermini_nrf52840", Side::Front, q);
        assert!(
            describe(&p, &d)
                .unwrap()
                .sites
                .iter()
                .all(|s| s.signal_terminal != "P107")
        );
    }
    #[test]
    fn traces_disabled_is_routing_obligation() {
        let mut q = BTreeMap::new();
        q.insert("reversible".into(), serde_json::json!(true));
        q.insert("include_traces".into(), serde_json::json!(false));
        let (p, d) = fixture("ceoloide/display_nice_view", Side::Front, q);
        assert!(
            describe(&p, &d)
                .unwrap()
                .sites
                .iter()
                .all(|s| s.routing_required)
        );
    }
}
