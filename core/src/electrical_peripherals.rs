//! Source-reviewed peripheral terminal requirements.
//!
//! This module intentionally uses generator source and declared parameters;
//! pad numbers are never treated as semantic electrical roles.
use crate::model::ProjectDoc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PeripheralRequirement {
    pub part_id: String,
    pub source: String,
    pub kind: String,
    pub gpio_terminals: Vec<(String, String)>,
    pub fixed_terminals: Vec<(String, String)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub rotary: Option<crate::inputs::RotaryProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "export-types", ts(optional))]
    pub press_key_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PeripheralRequirementError {
    pub part_id: String,
    pub code: String,
    pub message: String,
}

fn bool_parameter(
    part: &crate::model::Part,
    definition: &crate::model::PartDefinition,
    name: &str,
) -> Option<bool> {
    part.generator_parameters
        .as_ref()
        .and_then(|parameters| parameters.get(name))
        .or_else(|| {
            definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get(name))
        })
        .and_then(|value| {
            value
                .as_bool()
                .or_else(|| value.get("value").and_then(|v| v.as_bool()))
        })
}

/// Resolve known peripheral roles populated on a board. Unknown sources are
/// omitted so callers can present an explicit "needs reviewed profile" state.
pub fn describe(document: &ProjectDoc, board_id: &str) -> Vec<PeripheralRequirement> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return vec![];
    };
    let result: Vec<_> = document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            let input = crate::inputs::profile(definition);
            if let Some(rotary) = input.rotary {
                let enabled = definition.input_profile.is_some()
                    || bool_parameter(part, definition, "include_momentary_switch_pads")
                        != Some(false);
                let press_key_id = input.press.as_ref().filter(|_| enabled).map(|_| {
                    if crate::inputs::matrix_member(document, part) {
                        part.id.clone()
                    } else {
                        format!("{}/push", part.id)
                    }
                });
                let press = input.press.filter(|_| {
                    enabled
                        && crate::inputs::scan_mode(document, part)
                            == crate::inputs::PressScanMode::Direct
                });
                let mut gpio: Vec<(String, String)> = vec![
                    (rotary.a.clone(), "encoder-a".into()),
                    (rotary.b.clone(), "encoder-b".into()),
                ];
                let mut fixed = vec![(rotary.common.clone(), "GND".into())];
                if let Some(press) = press {
                    gpio.push((press.row, "encoder-push".into()));
                    fixed.push((press.column, "GND".into()));
                }
                return Some(PeripheralRequirement {
                    part_id: part.id.clone(),
                    source: definition
                        .generator
                        .as_ref()
                        .map(|g| g.source.clone())
                        .unwrap_or_else(|| format!("input-profile:{}", definition.id)),
                    kind: "encoder".into(),
                    gpio_terminals: gpio
                        .into_iter()
                        .map(|(terminal, function)| (terminal, format!("{}/{function}", part.id)))
                        .collect(),
                    fixed_terminals: fixed,
                    rotary: Some(rotary),
                    press_key_id,
                });
            }
            if let Some(press) = input.press.filter(|_| {
                definition.input_profile.is_some()
                    && crate::inputs::scan_mode(document, part)
                        == crate::inputs::PressScanMode::Direct
            }) {
                return Some(PeripheralRequirement {
                    part_id: part.id.clone(),
                    source: format!("input-profile:{}", definition.id),
                    kind: "press".into(),
                    gpio_terminals: vec![(press.row, format!("{}/input-push", part.id))],
                    fixed_terminals: vec![(press.column, "GND".into())],
                    rotary: None,
                    press_key_id: Some(if crate::inputs::matrix_member(document, part) {
                        part.id.clone()
                    } else {
                        format!("{}/push", part.id)
                    }),
                });
            }
            let source = definition.generator.as_ref()?.source.clone();
            if !source.starts_with("ceoloide/") && source != "infused-kim/nice_view" {
                return None;
            }
            let (kind, gpio_terminals, fixed_terminals): (
                &str,
                Vec<(&str, &str)>,
                Vec<(&str, &str)>,
            ) = if source.ends_with("/display_ssd1306") {
                (
                    "display-i2c",
                    vec![("SDA", "i2c/SDA"), ("SCL", "i2c/SCL")],
                    vec![("VCC", "VCC"), ("GND", "GND")],
                )
            } else if source.ends_with("/display_nice_view") || source == "infused-kim/nice_view" {
                (
                    "display-spi",
                    vec![("MOSI", "spi/MOSI"), ("SCK", "spi/SCK"), ("CS", "CS")],
                    vec![("VCC", "VCC"), ("GND", "GND")],
                )
            } else if source.ends_with("/reset_switch_smd_side")
                || source.ends_with("/reset_switch_tht_top")
            {
                ("reset", vec![], vec![("from", "GND"), ("to", "RST")])
            } else if source.ends_with("/power_switch_smd_side") {
                (
                    "power-switch",
                    vec![],
                    vec![("from", "BAT_P"), ("to", "RAW")],
                )
            } else if source.ends_with("/battery_connector_jst_ph_2")
                || source.ends_with("/battery_connector_molex_pico_ezmate_1x02")
            {
                (
                    "battery",
                    vec![],
                    vec![("BAT_P", "BAT_P"), ("BAT_N", "GND")],
                )
            } else if source.ends_with("/led_sk6812mini-e") {
                (
                    "rgb",
                    vec![("P4", "rgb-in")],
                    vec![("P1", "VCC"), ("P3", "GND")],
                )
            } else if source.ends_with("/trrs_pj320a") {
                (
                    "split",
                    vec![("TP", "split-tx"), ("R2", "split-rx")],
                    vec![("SL", "GND")],
                )
            } else {
                return None;
            };
            Some(PeripheralRequirement {
                rotary: None,
                press_key_id: None,
                part_id: part.id.clone(),
                source,
                kind: kind.into(),
                gpio_terminals: gpio_terminals
                    .into_iter()
                    .map(|(a, b)| {
                        (
                            a.into(),
                            if b.starts_with("spi/") || b.starts_with("i2c/") {
                                b.into()
                            } else {
                                format!("peripheral/{}/{}", part.id, b)
                            },
                        )
                    })
                    .collect(),
                fixed_terminals: fixed_terminals
                    .into_iter()
                    .map(|(a, b)| (a.into(), b.into()))
                    .collect(),
            })
        })
        .collect();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::collections::BTreeMap;
    fn doc(source: &str, params: BTreeMap<String, serde_json::Value>) -> ProjectDoc {
        let mut d = ProjectDoc::empty("p", "p");
        d.definitions.push(PartDefinition {
            input_profile: None,
            id: "peripheral".into(),
            name: "Peripheral".into(),
            kind: PartKind::Utility,
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
        });
        d.parts.push(Part {
            id: "p1".into(),
            definition_id: "peripheral".into(),
            reference: "J1".into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: Some(params),
        });
        d.boards.push(Board {
            id: "b".into(),
            name: "b".into(),
            outline_ids: vec![],
            part_ids: vec!["p1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        d
    }
    #[test]
    fn encoder_push_can_be_disabled() {
        let mut p = BTreeMap::new();
        p.insert(
            "include_momentary_switch_pads".into(),
            serde_json::json!(false),
        );
        let r = describe(&doc("ceoloide/rotary_encoder_ec11_ec12", p), "b");
        assert_eq!(r[0].gpio_terminals.len(), 2);
    }
    #[test]
    fn battery_is_fixed_power_not_gpio() {
        let r = describe(
            &doc("ceoloide/battery_connector_jst_ph_2", BTreeMap::new()),
            "b",
        );
        assert!(r[0].gpio_terminals.is_empty());
        assert!(
            r[0].fixed_terminals
                .contains(&("BAT_P".into(), "BAT_P".into()))
        );
    }
    #[test]
    fn unknown_source_is_omitted_for_review() {
        assert!(describe(&doc("custom/unknown", BTreeMap::new()), "b").is_empty());
    }
}
