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
                "{}/{:.5}/{:.5}",
                configuration.board_id, at.x, at.y
            );
            if let Some(index) = index_by_key.get(&key).copied() {
                holes[index].diameter = holes[index].diameter.max(diameter);
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
        + 1;
    let parts = holes
        .iter()
        .enumerate()
        .map(|(index, hole)| Part {
            id: format!("{PART_PREFIX}{}", hole.key),
            definition_id: definitions[index].id.clone(),
            reference: format!("MH{}", next_reference + index),
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
    result.definitions = document
        .definitions
        .iter()
        .filter(|definition| !definition.id.starts_with(DEFINITION_PREFIX))
        .cloned()
        .chain(definitions)
        .collect();
    result.parts = document
        .parts
        .iter()
        .filter(|part| !old_part_ids.contains(part.id.as_str()))
        .cloned()
        .chain(parts.iter().cloned())
        .collect();
    result.layouts = document
        .layouts
        .iter()
        .map(|layout| {
            let mut layout = layout.clone();
            layout
                .part_ids
                .retain(|part_id| !old_part_ids.contains(part_id.as_str()));
            layout
        })
        .collect();
    result.boards = document
        .boards
        .iter()
        .map(|board| {
            let mut board = board.clone();
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
            board
        })
        .collect();
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

fn mounting_hole_reference_number(reference: &str) -> Option<usize> {
    let suffix = reference.strip_prefix("MH")?;
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
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
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        })).unwrap()
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
            id: "authored".into(), definition_id: "authored-def".into(), reference: "MH9".into(),
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
        assert_eq!(generated.pose.at, Vec2 { x: 10.0, y: 20.0 });
        assert_eq!(generated.reference, "MH10");
        let definition = projected.definitions.iter().find(|definition| definition.id == generated.definition_id).unwrap();
        assert_eq!(definition.generator.as_ref().unwrap().source, MOUNTING_HOLE_SOURCE);
        assert_eq!(definition.generator.as_ref().unwrap().parameters["hole_drill"], "7.6");
        assert_eq!(definition.pads[0].drill, Some(7.6));
        assert_eq!(definition.pads[0].plated, Some(false));
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
        document.mechanical.as_mut().unwrap().closure_mounts = Some(vec![]);
        let empty = project_closure_clearance(&first, &hole_template());
        assert!(empty.parts.iter().all(|part| !part.id.starts_with(PART_PREFIX)));
        assert!(empty.definitions.iter().all(|definition| !definition.id.starts_with(DEFINITION_PREFIX)));
    }
}
