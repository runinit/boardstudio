//! Private, deterministic projection of accepted Case closure mounts into PCB parts.
//!
//! This is document planning only. Callers provide the already normalized
//! mounting-hole catalogue definition; geometry resolution, validation and edit
//! admission remain with their existing owners.

use boardstudio_core::model::{
    MountKind, Part, PartDefinition, PartGenerator, PartOutline, Pose2, ProjectDoc, Side, Vec2,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

const PART_PREFIX: &str = "case-closure/";
const DEFINITION_PREFIX: &str = "assembly-closure/definition/";
const MOUNTING_HOLE_SOURCE: &str = "ceoloide/mounting_hole_npth";

#[derive(Clone, Debug)]
struct Hole {
    key: String,
    board_id: String,
    at: Vec2,
    diameter: f64,
}

/// Rebuilds only the document-owned closure clearance projection.
///
/// `mounting_hole` must be the existing normalized bundled catalogue entry for
/// `ceoloide/mounting_hole_npth`. Its source identity and unrelated normalized
/// footprint data are retained while the generator parameters and pad drill
/// dimensions are updated to the projected diameter.
pub(super) fn project_closure_clearance(
    document: &ProjectDoc,
    mounting_hole: &PartDefinition,
) -> ProjectDoc {
    let old_part_ids: HashSet<&str> = document
        .parts
        .iter()
        .filter(|part| part.id.starts_with(PART_PREFIX))
        .map(|part| part.id.as_str())
        .collect();

    let mut holes = Vec::<Hole>::new();
    let mut index_by_key = HashMap::<String, usize>::new();
    let mut add_configuration = |configuration: &boardstudio_core::model::MechanicalConfiguration,
                                 flipped: bool| {
        for mount in configuration.closure_mounts.as_deref().unwrap_or_default() {
            let diameter = match &mount.kind {
                MountKind::Boss => mount.boss_diameter.unwrap_or(mount.hole_diameter)
                    + 2.0 * configuration.clearance,
                MountKind::Hole => mount.hole_diameter,
            };
            let at = Vec2 {
                x: if flipped { -mount.at.x } else { mount.at.x },
                y: mount.at.y,
            };
            let key = format!(
                "{}/{}/{}",
                configuration.board_id,
                js_to_fixed_5(at.x),
                js_to_fixed_5(at.y),
            );
            if let Some(index) = index_by_key.get(&key).copied() {
                holes[index].diameter = holes[index].diameter.max(diameter);
                holes[index].at = at;
            } else {
                index_by_key.insert(key.clone(), holes.len());
                holes.push(Hole {
                    key,
                    board_id: configuration.board_id.clone(),
                    at,
                    diameter,
                });
            }
        }
    };

    if let Some(configuration) = document.mechanical.as_ref() {
        add_configuration(configuration, false);
    }
    if let Some(hardware) = document.hardware.as_ref() {
        for instance in &hardware.instances {
            if let Some(configuration) = instance.mechanical.as_ref() {
                add_configuration(configuration, instance.flipped);
            }
        }
    }

    let definitions = holes
        .iter()
        .map(|hole| normalized_mounting_hole(mounting_hole, hole))
        .collect::<Vec<_>>();
    let next_reference = document
        .parts
        .iter()
        .filter(|part| !old_part_ids.contains(part.id.as_str()))
        .filter_map(|part| mounting_hole_reference_number(&part.reference))
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let parts = holes
        .iter()
        .enumerate()
        .map(|(index, hole)| Part {
            id: format!("{PART_PREFIX}{}", hole.key),
            definition_id: definitions[index].id.clone(),
            reference: format!("MH{}", next_reference.saturating_add(index as u64)),
            pose: Pose2 {
                at: hole.at,
                rotation: 0.0,
            },
            side: Side::Front,
            outline: Some(PartOutline {
                excluded: true,
                ..PartOutline::default()
            }),
            keycap: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        })
        .collect::<Vec<_>>();

    let mut result = document.clone();
    result
        .definitions
        .retain(|definition| !definition.id.starts_with(DEFINITION_PREFIX));
    result.definitions.extend(definitions);
    result
        .parts
        .retain(|part| !old_part_ids.contains(part.id.as_str()));
    result.parts.extend(parts.iter().cloned());
    for layout in &mut result.layouts {
        layout
            .part_ids
            .retain(|part_id| !old_part_ids.contains(part_id.as_str()));
    }
    for board in &mut result.boards {
        board
            .part_ids
            .retain(|part_id| !old_part_ids.contains(part_id.as_str()));
        board.part_ids.extend(
            holes
                .iter()
                .zip(&parts)
                .filter(|(hole, _)| hole.board_id == board.id)
                .map(|(_, part)| part.id.clone()),
        );
    }
    result
}

fn normalized_mounting_hole(template: &PartDefinition, hole: &Hole) -> PartDefinition {
    let mut definition = template.clone();
    definition.id = format!("{DEFINITION_PREFIX}{}", hole.key);
    if let Some(generator) = definition.generator.as_mut() {
        generator
            .parameters
            .insert("hole_drill".into(), Value::String(hole.diameter.to_string()));
        generator
            .parameters
            .insert("hole_size".into(), Value::String(hole.diameter.to_string()));
        generator.source = MOUNTING_HOLE_SOURCE.into();
    } else {
        definition.generator = Some(PartGenerator {
            source: MOUNTING_HOLE_SOURCE.into(),
            version: "bundled-1".into(),
            parameters: BTreeMap::from([
                ("hole_drill".into(), Value::String(hole.diameter.to_string())),
                ("hole_size".into(), Value::String(hole.diameter.to_string())),
            ]),
        });
    }
    for pad in &mut definition.pads {
        if pad.drill.is_some() {
            pad.drill = Some(hole.diameter);
            pad.size = Vec2 {
                x: hole.diameter,
                y: hole.diameter,
            };
            pad.plated = Some(false);
        }
    }
    // The bundled source consists of one circular NPTH pad and no courtyard
    // graphics. Ergogen's normalizer therefore uses the generated pad bounds as
    // the definition courtyard; keep that derived envelope in sync with its
    // updated hole-size parameter.
    let corners = definition
        .pads
        .iter()
        .filter(|pad| pad.drill.is_some())
        .flat_map(|pad| {
            let angle = pad.rotation.unwrap_or(0.0).to_radians();
            let half_x = pad.size.x / 2.0;
            let half_y = pad.size.y / 2.0;
            [
                (-half_x, -half_y),
                (half_x, -half_y),
                (half_x, half_y),
                (-half_x, half_y),
            ]
            .map(|(x, y)| Vec2 {
                x: pad.at.x + x * angle.cos() - y * angle.sin(),
                y: pad.at.y - x * angle.sin() - y * angle.cos(),
            })
        })
        .collect::<Vec<_>>();
    if !corners.is_empty() {
        let min_x = corners.iter().map(|point| point.x).fold(f64::INFINITY, f64::min);
        let max_x = corners.iter().map(|point| point.x).fold(f64::NEG_INFINITY, f64::max);
        let min_y = corners.iter().map(|point| point.y).fold(f64::INFINITY, f64::min);
        let max_y = corners.iter().map(|point| point.y).fold(f64::NEG_INFINITY, f64::max);
        definition.courtyard = if max_x - min_x < 0.01 || max_y - min_y < 0.01 {
            vec![]
        } else {
            vec![
                Vec2 { x: min_x, y: min_y },
                Vec2 { x: max_x, y: min_y },
                Vec2 { x: max_x, y: max_y },
                Vec2 { x: min_x, y: max_y },
            ]
        };
    }
    definition
}

fn mounting_hole_reference_number(reference: &str) -> Option<u64> {
    let suffix = reference.strip_prefix("MH")?;
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    suffix.parse::<u64>().ok()
}

/// Matches Number#toFixed(5) for accepted finite board coordinates.
///
/// The binary float is rounded as an exact rational and midpoint ties go up,
/// matching ECMAScript's rule. Negative zero renders as zero, while a small
/// negative nonzero value can still render as -0.00000.
fn js_to_fixed_5(value: f64) -> String {
    const DECIMAL_SCALE: u128 = 100_000;
    const FRACTION_DIGITS: usize = 5;

    if value.is_nan() {
        return "NaN".into();
    }
    if value == f64::INFINITY {
        return "Infinity".into();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".into();
    }
    if value == 0.0 {
        return "0.00000".into();
    }
    if value.abs() >= 1e21 {
        return js_number_to_string(value);
    }

    let bits = value.abs().to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, exponent) = if exponent_bits == 0 {
        (fraction as u128, -1074)
    } else {
        (
            ((1_u64 << 52) | fraction) as u128,
            exponent_bits - 1023 - 52,
        )
    };
    let scaled = significand * DECIMAL_SCALE;
    let rounded = if exponent >= 0 {
        scaled
            .checked_shl(exponent as u32)
            .expect("accepted board coordinate scaling fits u128")
    } else {
        let shift = (-exponent) as u32;
        if shift >= 128 {
            0
        } else {
            let quotient = scaled >> shift;
            let remainder = scaled & ((1_u128 << shift) - 1);
            let midpoint = 1_u128 << (shift - 1);
            quotient + if remainder >= midpoint { 1 } else { 0 }
        }
    };
    let mut digits = rounded.to_string();
    if digits.len() <= FRACTION_DIGITS {
        digits.insert_str(0, &"0".repeat(FRACTION_DIGITS + 1 - digits.len()));
    }
    let decimal = digits.len() - FRACTION_DIGITS;
    let sign = if value.is_sign_negative() { "-" } else { "" };
    format!("{sign}{}.{}", &digits[..decimal], &digits[decimal..])
}

fn js_number_to_string(value: f64) -> String {
    let raw = value.to_string();
    let negative = raw.starts_with('-');
    let unsigned = raw.strip_prefix('-').unwrap_or(&raw);
    let (mantissa, exponent) = unsigned
        .split_once('e')
        .map(|(mantissa, exponent)| (mantissa, exponent.parse::<i32>().unwrap_or(0)))
        .unwrap_or((unsigned, 0));
    let decimal_index = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let mut digits = mantissa
        .chars()
        .filter(|character| *character != '.')
        .collect::<String>();
    let leading_zeros = digits.bytes().take_while(|digit| *digit == b'0').count();
    digits.drain(..leading_zeros);
    if digits.is_empty() {
        return "0".into();
    }
    let power = decimal_index - leading_zeros as i32 - 1 + exponent;
    while digits.ends_with('0') {
        digits.pop();
    }
    let fraction = digits.get(1..).unwrap_or_default();
    let mantissa = if fraction.is_empty() {
        digits[..1].to_string()
    } else {
        format!("{}.{}", &digits[..1], fraction)
    };
    format!("{}{mantissa}e+{power}", if negative { "-" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        Board, HardwareConfiguration, Layout, MechanicalConfiguration, Mount,
        PhysicalBoardInstance,
    };

    fn configuration(board_id: &str, mounts: Vec<Mount>, clearance: f64) -> MechanicalConfiguration {
        let mut configuration = serde_json::from_value::<MechanicalConfiguration>(
            serde_json::json!({
                "boardId": board_id, "method": "printed", "mount": "rigid",
                "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
                "bottomFoamThickness": 0.5, "batteryHeight": 0.0, "bottomThickness": 2.0,
                "plateToPcb": 3.0, "wallThickness": 2.0, "clearance": clearance,
                "profiles": [], "mounts": [], "closureMounts": mounts
            }),
        )
        .unwrap();
        configuration.clearance = clearance;
        configuration
    }

    fn mount(at: Vec2, kind: MountKind, hole: f64, boss: Option<f64>) -> Mount {
        Mount {
            id: "mount".into(),
            at,
            kind,
            hole_diameter: hole,
            boss_diameter: boss,
            height: None,
        }
    }

    fn hole_template() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"ergogen:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        })).unwrap()
    }

    #[test]
    fn key_rounding_matches_javascript_fixed_decimal_behavior() {
        assert_eq!(js_to_fixed_5(-0.0), "0.00000");
        assert_eq!(js_to_fixed_5(-0.0000001), "-0.00000");
        assert_eq!(js_to_fixed_5(0.015625), "0.01563");
        assert_eq!(js_to_fixed_5(-0.015625), "-0.01563");
        assert_eq!(mounting_hole_reference_number("MH4294967295"), Some(4_294_967_295));
    }

    #[test]
    fn projects_canonical_and_flipped_instance_union_and_preserves_other_parts() {
        let mut document = ProjectDoc::empty("doc", "Doc");
        document.boards.push(Board {
            id: "board".into(), name: "Board".into(), outline_ids: vec![],
            part_ids: vec!["authored".into(), "case-closure/stale".into()],
            net_ids: vec![], thickness: 1.6, traces: vec![], vias: vec![],
        });
        document.parts.push(Part {
            id: "authored".into(), definition_id: "authored-def".into(), reference: "MH4294967295".into(),
            pose: Pose2 { at: Vec2 { x: 4.0, y: 5.0 }, rotation: 0.0 }, side: Side::Front,
            outline: None, keycap: None, locked: None, properties: None, generator_parameters: None,
        });
        document.layouts.push(Layout {
            id: "layout".into(), name: "Layout".into(), board_id: "board".into(),
            matrix_id: "matrix".into(), part_ids: vec!["authored".into(), "case-closure/stale".into()],
            mirror_link: None,
        });
        document.parts.push(Part {
            id: "case-closure/stale".into(), definition_id: "stale".into(), reference: "MH88".into(),
            pose: Pose2 { at: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 }, side: Side::Front,
            outline: None, keycap: None, locked: None, properties: None, generator_parameters: None,
        });
        document.mechanical = Some(configuration(
            "board",
            vec![mount(Vec2 { x: 10.0, y: 20.0 }, MountKind::Boss, 2.2, Some(5.0))],
            0.3,
        ));
        document.hardware = Some(HardwareConfiguration {
            instances: vec![PhysicalBoardInstance {
                id: "right".into(), name: "Right".into(), board_id: "board".into(),
                half: "right".into(), role: "peripheral".into(), flipped: true,
                controller_part_id: None, construction_linked: true,
                mechanical: Some(configuration(
                    "board",
                    vec![mount(Vec2 { x: -10.000001, y: 20.000001 }, MountKind::Boss, 2.2, Some(7.0))],
                    0.3,
                )),
            }],
            ..HardwareConfiguration::default()
        });

        let projected = project_closure_clearance(&document, &hole_template());
        let generated = projected.parts.iter().find(|part| part.id.starts_with(PART_PREFIX)).unwrap();
        assert_eq!(generated.pose.at, Vec2 { x: 10.000001, y: 20.000001 });
        assert_eq!(generated.reference, "MH4294967296");
        let definition = projected.definitions.iter().find(|definition| definition.id == generated.definition_id).unwrap();
        assert_eq!(definition.generator.as_ref().unwrap().source, MOUNTING_HOLE_SOURCE);
        assert_eq!(definition.generator.as_ref().unwrap().parameters["hole_drill"], "7.6");
        assert_eq!(definition.pads[0].drill, Some(7.6));
        assert_eq!(definition.pads[0].plated, Some(false));
        assert_eq!(definition.envelope_notice.as_deref(), Some("No closed courtyard is available; the outline uses physical graphics and pad extents."));
        assert_eq!(definition.courtyard[0], Vec2 { x: -3.8, y: -3.8 });
        assert_eq!(definition.courtyard[2], Vec2 { x: 3.8, y: 3.8 });
        assert_eq!(projected.boards[0].part_ids.len(), 2);
        assert_eq!(projected.boards[0].part_ids[0], "authored");
        assert_eq!(projected.boards[0].part_ids[1], generated.id);
        assert_eq!(projected.layouts[0].part_ids.len(), 1);
        assert_eq!(projected.layouts[0].part_ids[0], "authored");
        assert!(document.parts.iter().any(|part| part.id == "case-closure/stale"));
        assert_eq!(projected.parts.iter().filter(|part| part.id.starts_with(PART_PREFIX)).count(), 1);
    }

    #[test]
    fn screw_clearance_uses_hole_diameter_and_empty_mounts_remove_owned_projection() {
        let mut document = ProjectDoc::empty("doc", "Doc");
        document.boards.push(Board {
            id: "board".into(), name: "Board".into(), outline_ids: vec![], part_ids: vec![],
            net_ids: vec![], thickness: 1.6, traces: vec![], vias: vec![],
        });
        document.mechanical = Some(configuration(
            "board",
            vec![mount(Vec2 { x: 1.0, y: 2.0 }, MountKind::Hole, 2.7, Some(9.0))],
            4.0,
        ));
        let first = project_closure_clearance(&document, &hole_template());
        let part = first.parts.iter().find(|part| part.id.starts_with(PART_PREFIX)).unwrap();
        assert_eq!(first.definitions.iter().find(|definition| definition.id == part.definition_id).unwrap().pads[0].drill, Some(2.7));
        let mut explicit_empty = first.clone();
        explicit_empty.mechanical.as_mut().unwrap().closure_mounts = Some(vec![]);
        let empty = project_closure_clearance(&explicit_empty, &hole_template());
        assert!(empty.parts.iter().all(|part| !part.id.starts_with(PART_PREFIX)));
        assert!(empty.definitions.iter().all(|definition| !definition.id.starts_with(DEFINITION_PREFIX)));
    }

    #[test]
    fn positive_and_negative_zero_deduplicate_to_one_js_compatible_key() {
        let mut document = ProjectDoc::empty("doc", "Doc");
        document.boards.push(Board {
            id: "board".into(), name: "Board".into(), outline_ids: vec![], part_ids: vec![],
            net_ids: vec![], thickness: 1.6, traces: vec![], vias: vec![],
        });
        document.mechanical = Some(configuration(
            "board",
            vec![mount(Vec2 { x: -0.0, y: 0.0 }, MountKind::Hole, 2.2, None)],
            0.0,
        ));
        document.hardware = Some(HardwareConfiguration {
            instances: vec![PhysicalBoardInstance {
                id: "right".into(), name: "Right".into(), board_id: "board".into(),
                half: "right".into(), role: "peripheral".into(), flipped: true,
                controller_part_id: None, construction_linked: true,
                mechanical: Some(configuration(
                    "board",
                    vec![mount(Vec2 { x: 0.0, y: 0.0 }, MountKind::Hole, 2.2, None)],
                    0.0,
                )),
            }],
            ..HardwareConfiguration::default()
        });

        let projected = project_closure_clearance(&document, &hole_template());
        let generated = projected.parts.iter().filter(|part| part.id.starts_with(PART_PREFIX)).collect::<Vec<_>>();
        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0].id, "case-closure/board/0.00000/0.00000");
    }
}
