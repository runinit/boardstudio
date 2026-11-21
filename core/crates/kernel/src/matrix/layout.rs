use crate::{matrix, model::*};
use std::collections::BTreeSet;

mod adoption;
mod components;
mod keys;
mod parts;
mod reflection;
mod spacing;

use adoption::strip_substitute_flag;
pub use adoption::{
    adopt_half_frames, adopt_unrecorded_mirrors, evaluated_part, evaluated_part_pose,
    is_member_or_companion, layout_half_offset, part_half_offset, set_pair_gap, unadopt_pair,
};
pub use components::{
    mirrored_component_ids, move_linked_component, remove_matrix, retune_driven_constraint,
    sync_components, unlink_components,
};
use components::{remove_component, set_component_pose, sync_component_nets};
use keys::primary_member;
pub use keys::{
    AssemblyPlacement, LinkedRemovalFollow, add_local_matrix_exception,
    apply_exception_transitions, assembly_placements, companion_mode, edits_alone,
    has_linked_positions, is_matrix_key, move_keys, plan_linked_matrix_removal,
    reconcile_constraint_keys, refuse_omitted_storage_edit,
};
use parts::materialize_unrecorded_partners;
pub use parts::{
    clamp_part_to_half, ensure_part_mirror, mirror_new_pair_part, move_pair_part,
    reframe_moved_parts, retire_part_correspondences, set_detached_part_rotation,
    set_part_mirrored, stored_part_pose,
};
use reflection::{carrier_offset, pair_sides, remap_anchor_target};
pub use reflection::{
    exception_state, omitted, part_correspondence, reflect_half_part, reflected, refresh_existing,
    resolve, sync,
};
pub use spacing::{
    DEFAULT_CREATION_GAP_MM, DEFAULT_SPACING_MARGIN_MM, adapt_spacing, part_extent_offsets,
    part_extents_at, proposed_creation_gap,
};

const MIRRORED_COMPONENT_SOURCE: &str = "boardstudio.mirroredComponentSource";
const MIRRORED_COMPONENT_SUBSTITUTE: &str = "boardstudio.mirroredComponentSubstitute";

pub fn validate(doc: &ProjectDoc) -> Result<(), String> {
    // Bound every matrix before any walk over its rows and columns below.
    for matrix in &doc.matrices {
        matrix::valid_dimensions(matrix)?;
    }
    if doc.layouts.is_empty() {
        return Ok(());
    }
    let mut ids = BTreeSet::new();
    let mut matrices = BTreeSet::new();
    let mut parts = BTreeSet::new();
    let mut linked = BTreeSet::new();
    for layout in &doc.layouts {
        if layout.id.trim().is_empty() || layout.name.trim().is_empty() || !ids.insert(&layout.id) {
            return Err("Layout names and identities must be present and unique".into());
        }
        let board = doc
            .boards
            .iter()
            .find(|board| board.id == layout.board_id)
            .ok_or("Layout board is missing")?;
        if layout.matrix_ids.is_empty() {
            return Err("Layout matrix membership must be nonempty".into());
        }
        for id in &layout.matrix_ids {
            let matrix = matrix::evaluation::logical_matrix(id, doc)?;
            if !matrices.insert(matrix.id.clone()) || matrix.board_id.as_ref() != Some(&board.id) {
                return Err("Each layout must own distinct matrices on its board".into());
            }
        }
        for id in &layout.part_ids {
            if !parts.insert(id)
                || !board.part_ids.contains(id)
                || !doc.parts.iter().any(|part| &part.id == id)
                || doc
                    .matrices
                    .iter()
                    .any(|matrix| matrix.part_ids.contains(id))
            {
                return Err("Layout components must be independent parts on the same board".into());
            }
        }
        if let Some(link) = &layout.mirror_link {
            let source = doc
                .layouts
                .iter()
                .find(|other| other.id == link.source_id)
                .ok_or("Linked layout is missing")?;
            if !link.axis_x.is_finite()
                || source.id == layout.id
                || source.board_id != layout.board_id
                || source.mirror_link.is_some()
                || !linked.insert(&source.id)
                || !linked.insert(&layout.id)
            {
                return Err("A mirrored pair must link two distinct layouts on one board".into());
            }
            if link.split_gap.is_some_and(|gap| !gap.is_finite()) {
                return Err("Split gap must be finite".into());
            }
            if link.spacing.is_some_and(|spacing| !spacing.is_finite()) {
                return Err("Split spacing must be finite".into());
            }
            validate_link(doc, source, layout, link)?;
        }
    }
    for part in &doc.parts {
        if primary_member(doc, &part.id)
            .is_some_and(|(matrix, _, _)| partner(doc, &matrix.id).is_some())
            && doc.constraint_driving_part(&part.id)?.is_some()
        {
            return Err("Unlink the halves before constraining a linked key's position".into());
        }
    }
    Ok(())
}

fn owns_member(doc: &ProjectDoc, layout: &Layout, id: &str) -> bool {
    layout.part_ids.iter().any(|member| member == id)
        || layout.matrix_ids.iter().any(|matrix_id| {
            let Ok(matrix) = matrix::evaluation::logical_matrix(matrix_id, doc) else {
                return false;
            };
            let Some(slot) = id.strip_prefix(&format!("matrix/{matrix_id}/r")) else {
                return false;
            };
            let (slot, assembly) = slot
                .split_once('/')
                .map_or((slot, None), |(slot, assembly)| (slot, Some(assembly)));
            let Some((row, column)) = slot.split_once('c') else {
                return false;
            };
            let (Ok(row), Ok(column)) = (row.parse::<u32>(), column.parse::<u32>()) else {
                return false;
            };
            row < matrix.rows
                && column < matrix.columns
                && assembly.is_none_or(|id| {
                    matrix.cells.iter().any(|cell| {
                        cell.row == row
                            && cell.column == column
                            && cell.assemblies.iter().any(|assembly| assembly.id == id)
                    })
                })
        })
}

fn validate_link(
    doc: &ProjectDoc,
    source: &Layout,
    target: &Layout,
    link: &LayoutMirrorLink,
) -> Result<(), String> {
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for record in &link.correspondences {
        let source_key = (record.kind, record.source_id.as_str());
        let target_key = (record.kind, record.mirror_id.as_str());
        if sources.contains(&source_key) || targets.contains(&target_key) {
            return Err(
                "Layout correspondences must have unique source and mirror identities".into(),
            );
        }
        sources.push(source_key);
        targets.push(target_key);
        let owned = match record.kind {
            MirroredItemKind::Matrix => {
                source.matrix_ids.contains(&record.source_id)
                    && target.matrix_ids.contains(&record.mirror_id)
            }
            MirroredItemKind::Part => {
                owns_member(doc, source, &record.source_id)
                    && owns_member(doc, target, &record.mirror_id)
            }
            _ => false,
        };
        if !owned {
            return Err("Layout correspondence has invalid ownership".into());
        }
    }
    if source
        .matrix_ids
        .iter()
        .any(|id| !sources.contains(&(MirroredItemKind::Matrix, id.as_str())))
    {
        return Err(
            "Linked Layout requires an explicit correspondence for every source matrix".into(),
        );
    }
    let mut overlays = Vec::new();
    for exception in &link.exceptions {
        let key = (exception.kind, exception.mirror_id.as_str());
        if overlays.contains(&key) {
            return Err("Layout exceptions must have unique identities".into());
        }
        overlays.push(key);
        let owned = match exception.kind {
            MirroredItemKind::Matrix => target.matrix_ids.contains(&exception.mirror_id),
            MirroredItemKind::Part => owns_member(doc, target, &exception.mirror_id),
            _ => false,
        };
        if !owned || (!targets.contains(&key) && exception.state != MirroredExceptionState::Local) {
            return Err("Layout exception has invalid ownership or no correspondence".into());
        }
    }
    Ok(())
}

/// The slots both halves still own, paired by slot. A mirror without a physical matrix (an
/// omitted group) pairs by the source shape.
pub fn member_pairs(
    source: &Matrix,
    mirror_id: &str,
    mirror: Option<&Matrix>,
) -> Vec<(String, String)> {
    let mirror_members = mirror.map(matrix::cell_members);
    matrix::cell_members(source)
        .into_iter()
        .filter_map(|(slot, source_id)| {
            Some((
                source_id,
                match &mirror_members {
                    Some(members) => members.get(&slot).cloned()?,
                    None => matrix::member_id(mirror_id, slot.0, slot.1),
                },
            ))
        })
        .collect()
}

/// Part correspondences for explicit member pairs.
pub fn part_correspondences(
    pairs: impl IntoIterator<Item = (String, String)>,
) -> impl Iterator<Item = MirroredCorrespondence> {
    pairs
        .into_iter()
        .map(|(source_id, mirror_id)| MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id,
            mirror_id,
            companion_mode: None,
            companion_tweak: None,
        })
}

/// A Local Part exception excuses its mirror member from pairing.
fn is_excepted(exceptions: &[MirroredException], mirror_id: &str) -> bool {
    exceptions.iter().any(|exception| {
        exception.kind == MirroredItemKind::Part && exception.mirror_id == mirror_id
    })
}

pub fn populate_new_link(doc: &ProjectDoc, target: &mut Layout) -> Result<(), String> {
    let Some(link) = &mut target.mirror_link else {
        return Ok(());
    };
    if !link.correspondences.is_empty() {
        return Ok(());
    }
    let source = doc
        .layouts
        .iter()
        .find(|layout| layout.id == link.source_id)
        .ok_or("Linked layout is missing")?;
    if source.matrix_ids.len() != 1 || target.matrix_ids.len() != 1 {
        return Err("Linking matrix groups requires explicit matrix correspondences".into());
    }
    let source_id = &source.matrix_ids[0];
    let mirror_id = &target.matrix_ids[0];
    link.correspondences.push(MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: source_id.clone(),
        mirror_id: mirror_id.clone(),
        companion_mode: None,
        companion_tweak: None,
    });
    let matrix = matrix::evaluation::logical_matrix(source_id, doc)?;
    let exceptions = link.exceptions.clone();
    link.correspondences.extend(part_correspondences(
        member_pairs(&matrix, mirror_id, None)
            .into_iter()
            .filter(|(_, mirror_id)| !is_excepted(&exceptions, mirror_id)),
    ));
    Ok(())
}

/// Membership edits maintain explicit identities; admission never repairs them.
pub fn sync_member_pairs(doc: &mut ProjectDoc, matrix_id: &str) -> Result<(), String> {
    for index in 0..doc.layouts.len() {
        let Some(link) = doc.layouts[index].mirror_link.as_ref() else {
            continue;
        };
        let pairs: Vec<_> = link
            .correspondences
            .iter()
            .filter(|record| {
                record.kind == MirroredItemKind::Matrix
                    && (record.source_id == matrix_id || record.mirror_id == matrix_id)
            })
            .cloned()
            .collect();
        if pairs.is_empty() {
            continue;
        }
        let source_index = doc
            .layouts
            .iter()
            .position(|layout| layout.id == link.source_id)
            .ok_or("Linked layout is missing")?;
        let pairs_of_slots: Vec<_> = pairs
            .iter()
            .map(|pair| {
                let source_matrix = matrix::evaluation::logical_matrix(&pair.source_id, doc)?;
                let mirror_matrix = matrix::evaluation::logical_matrix(&pair.mirror_id, doc)?;
                Ok(member_pairs(
                    &source_matrix,
                    &pair.mirror_id,
                    Some(mirror_matrix.as_ref()),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let Some(mut link) = doc.layouts[index].mirror_link.take() else {
            continue;
        };
        let source = &doc.layouts[source_index];
        let target = &doc.layouts[index];
        link.correspondences.retain(|record| {
            record.kind != MirroredItemKind::Part
                || (owns_member(doc, source, &record.source_id)
                    && owns_member(doc, target, &record.mirror_id))
        });
        link.exceptions.retain(|record| {
            record.kind != MirroredItemKind::Part || owns_member(doc, target, &record.mirror_id)
        });
        for slots in pairs_of_slots {
            for (source_id, mirror_id) in slots {
                if is_excepted(&link.exceptions, &mirror_id)
                    || link.correspondences.iter().any(|record| {
                        record.kind == MirroredItemKind::Part
                            && (record.source_id == source_id || record.mirror_id == mirror_id)
                    })
                {
                    continue;
                }
                link.correspondences
                    .extend(part_correspondences([(source_id, mirror_id)]));
            }
        }
        doc.layouts[index].mirror_link = Some(link);
    }
    Ok(())
}

/// The relationship is stored once, but geometry can be edited from either half.
pub fn has_link(doc: &ProjectDoc, matrix_id: &str) -> bool {
    doc.layouts
        .iter()
        .filter(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .any(|owner| {
            owner.mirror_link.is_some()
                || doc.layouts.iter().any(|layout| {
                    layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == owner.id)
                })
        })
}

pub fn partner(doc: &ProjectDoc, matrix_id: &str) -> Option<(String, f64)> {
    doc.layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .find_map(|link| {
            link.correspondences.iter().find_map(|record| {
                if record.kind != MirroredItemKind::Matrix {
                    return None;
                }
                if record.source_id == matrix_id {
                    Some((record.mirror_id.clone(), link.axis_x))
                } else if record.mirror_id == matrix_id {
                    Some((record.source_id.clone(), link.axis_x))
                } else {
                    None
                }
            })
        })
}

/// The pair map of a vertical mirror line at `axis_x`: the reflection `x' = 2 * axis_x - x`.
pub fn axis_mapping(axis_x: f64) -> Pose2 {
    Pose2 {
        at: Vec2 {
            x: 2.0 * axis_x,
            y: 0.0,
        },
        rotation: 0.0,
    }
}

/// Key-edge to key-edge distance implied by board-coordinate geometry, measured
/// at the half-origin column: the source origin's key edge to its reflection.
/// This is the stored Split gap a legacy pair adopts on its first edit.
pub fn implied_gap(axis_x: f64, matrix: &Matrix) -> f64 {
    gap_from_origin(axis_x, matrix.origin.x, matrix.pitch.x, matrix.edge_gap)
}

fn gap_from_origin(axis_x: f64, origin_x: f64, pitch_x: f64, edge_gap: Option<Vec2>) -> f64 {
    let edge = edge_gap
        .map(|gap| gap.x)
        .unwrap_or(super::DEFAULT_EDGE_GAP_MM);
    2.0 * (axis_x - origin_x) - (pitch_x - edge)
}

/// The link, axis, stored gap, current spacing and side of the pair a matrix
/// belongs to: `true` when the matrix lives on the mirror layout.
/// Record-absent mirror members resolve through their correspondence.
fn pair_frame(doc: &ProjectDoc, matrix_id: &str) -> Option<(f64, Option<f64>, Option<f64>, bool)> {
    for layout in &doc.layouts {
        if let Some(link) = layout.mirror_link.as_ref()
            && layout.matrix_ids.iter().any(|id| id == matrix_id)
        {
            return Some((link.axis_x, link.split_gap, link.spacing, true));
        }
    }
    for layout in &doc.layouts {
        if layout.mirror_link.is_some() || !layout.matrix_ids.iter().any(|id| id == matrix_id) {
            continue;
        }
        let link = doc
            .layouts
            .iter()
            .filter_map(|other| other.mirror_link.as_ref())
            .find(|link| link.source_id == layout.id)?;
        return Some((link.axis_x, link.split_gap, link.spacing, false));
    }
    for layout in &doc.layouts {
        if let Some(link) = layout.mirror_link.as_ref()
            && link.correspondences.iter().any(|record| {
                record.kind == MirroredItemKind::Matrix && record.mirror_id == matrix_id
            })
        {
            return Some((link.axis_x, link.split_gap, link.spacing, true));
        }
    }
    None
}

/// The stored Split gap of the pair a matrix belongs to, if the pair has one.
pub fn pair_gap(doc: &ProjectDoc, matrix_id: &str) -> Option<f64> {
    pair_frame(doc, matrix_id).and_then(|(_, gap, _, _)| gap)
}

/// Board-space position of the half frame a matrix is placed in: the split
/// axis offset by half the current spacing (the adapted `max(stored gap,
/// minimum)`, or the stored gap before the first adaptive pass). `None` for
/// unlinked matrices and legacy pairs without a stored gap, whose origins
/// stay board-coordinate.
///
/// Ticket 17: the Layout canvas anchor badge reads this for the unanchored
/// matrix anchor (the half origin); evaluation applies the same offset.
pub fn half_offset(doc: &ProjectDoc, matrix_id: &str) -> Option<Vec2> {
    let (axis_x, gap, spacing, mirror_side) = pair_frame(doc, matrix_id)?;
    let gap = spacing.or(gap)?;
    Some(Vec2 {
        x: if mirror_side {
            axis_x + gap / 2.0
        } else {
            axis_x - gap / 2.0
        },
        y: 0.0,
    })
}

#[cfg(test)]
mod case_hardware_tests {
    use super::components::adopt_unassigned_components;
    use super::*;
    use boardstudio_model::mechanical::pair;

    #[test]
    fn case_clearance_holes_are_not_adopted_by_linked_layouts() {
        let mut doc = ProjectDoc::empty("case", "Case");
        doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["case-closure/board/10/10"],"netIds":[],"thickness":1.6}])).unwrap();
        doc.parts = serde_json::from_value(serde_json::json!([{"id":"case-closure/board/10/10","definitionId":"hole","reference":"MH1","pose":{"at":{"x":10,"y":10},"rotation":0},"side":"front"}])).unwrap();
        doc.matrices = serde_json::from_value(serde_json::json!([{"id":"matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}])).unwrap();
        doc.layouts = serde_json::from_value(serde_json::json!([{"id":"left","name":"Left","boardId":"board","matrixIds":["matrix"],"partIds":[]}])).unwrap();
        adopt_unassigned_components(&mut doc, "left", 100.0, &mut vec![]).unwrap();
        assert!(doc.layouts[0].part_ids.is_empty());
    }

    #[test]
    fn linked_controller_mirror_keeps_world_heading() {
        // Setup "Place controller": one controller source part at rotation 0 on a linked
        // source layout. The derived mirror must share world rotation 0 (plain reflection
        // `-r = 0`), not ADR-0010's `180 - r` half turn.
        let mut doc = ProjectDoc::empty("linked-controller", "Linked controller");
        doc.definitions = serde_json::from_value(serde_json::json!([
            {"id":"mcu","name":"MCU","kind":"controller","courtyard":[],"pads":[]}
        ]))
        .unwrap();
        doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["source/U1"],"netIds":[],"thickness":1.6}])).unwrap();
        doc.parts = serde_json::from_value(serde_json::json!([{"id":"source/U1","definitionId":"mcu","reference":"U1","pose":{"at":{"x":10.0,"y":5.0},"rotation":0.0},"side":"front"}])).unwrap();
        doc.matrices = serde_json::from_value(serde_json::json!([
            {"id":"source-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]},
            {"id":"mirror-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}
        ])).unwrap();
        doc.layouts = serde_json::from_value(serde_json::json!([
            {"id":"source-layout","name":"Source","boardId":"board","matrixIds":["source-matrix"],"partIds":["source/U1"]},
            {"id":"mirror-layout","name":"Mirror","boardId":"board","matrixIds":["mirror-matrix"],"partIds":[],"mirrorLink":{"sourceId":"source-layout","axisX":50.0}}
        ])).unwrap();
        sync_components(&mut doc).unwrap();
        let mirrored = doc
            .parts
            .iter()
            .find(|part| {
                part.properties
                    .as_ref()
                    .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                    .and_then(serde_json::Value::as_str)
                    == Some("source/U1")
            })
            .expect("the linked controller mirror is materialized");
        assert_eq!(mirrored.pose.at, Vec2 { x: 90.0, y: 5.0 });
        assert!(
            (mirrored.pose.rotation - 0.0).abs() < 1e-9,
            "controller mirror keeps world heading 0, got {}",
            mirrored.pose.rotation
        );
        for heading in [90.0f64, 180.0, 270.0] {
            let mut doc = ProjectDoc::empty("linked-controller", "Linked controller");
            doc.definitions = serde_json::from_value(serde_json::json!([
                {"id":"mcu","name":"MCU","kind":"controller","courtyard":[],"pads":[]}
            ]))
            .unwrap();
            doc.boards = serde_json::from_value(serde_json::json!([{"id":"board","name":"Board","outlineIds":[],"partIds":["source/U1"],"netIds":[],"thickness":1.6}])).unwrap();
            doc.parts = serde_json::from_value(serde_json::json!([{"id":"source/U1","definitionId":"mcu","reference":"U1","pose":{"at":{"x":10.0,"y":5.0},"rotation":heading},"side":"front"}])).unwrap();
            doc.matrices = serde_json::from_value(serde_json::json!([
                {"id":"source-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]},
                {"id":"mirror-matrix","boardId":"board","rows":1,"columns":1,"pitch":{"x":19,"y":19},"origin":{"x":0,"y":0},"definitionId":"key","partIds":[]}
            ])).unwrap();
            doc.layouts = serde_json::from_value(serde_json::json!([
                {"id":"source-layout","name":"Source","boardId":"board","matrixIds":["source-matrix"],"partIds":["source/U1"]},
                {"id":"mirror-layout","name":"Mirror","boardId":"board","matrixIds":["mirror-matrix"],"partIds":[],"mirrorLink":{"sourceId":"source-layout","axisX":50.0}}
            ])).unwrap();
            sync_components(&mut doc).unwrap();
            let mirrored = doc
                .parts
                .iter()
                .find(|part| {
                    part.properties
                        .as_ref()
                        .and_then(|properties| properties.get(MIRRORED_COMPONENT_SOURCE))
                        .and_then(serde_json::Value::as_str)
                        == Some("source/U1")
                })
                .expect("the linked controller mirror is materialized");
            let expected = (0.0 - heading).rem_euclid(360.0);
            assert!(
                (mirrored.pose.rotation - expected).abs() < 1e-9,
                "controller heading {heading} mirrors to {expected}, got {}",
                mirrored.pose.rotation
            );
        }
    }

    fn matrix_with_companion() -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "source", "rows": 1, "columns": 1, "pitch": {"x": 19, "y": 19},
            "origin": {"x": 10, "y": 5}, "rotation": 20, "definitionId": "key", "partIds": [],
            "cells": [{"row": 0, "column": 0, "enabled": true, "rotation": 15,
                "assemblies": [{"id": "diode", "definitionId": "d", "offset": {"x": 3, "y": 4}, "rotation": 30}]}]
        }))
        .unwrap()
    }

    fn empty_target() -> Matrix {
        let mut target = matrix_with_companion();
        target.id = "target".into();
        target.cells.clear();
        target
    }

    #[test]
    fn a_vertical_axis_reflects_the_origin_turns_and_companions() {
        let reflected = reflected(
            &matrix_with_companion(),
            &empty_target(),
            axis_mapping(50.0),
            true,
            false,
        )
        .unwrap();
        assert_eq!(reflected.origin, Vec2 { x: 90.0, y: 5.0 });
        assert_eq!(reflected.rotation, Some(-20.0));
        assert_eq!(reflected.mirror, Some(Mirror::X));
        let cell = &reflected.cells[0];
        assert_eq!(cell.rotation, Some(15.0));
        assert_eq!(cell.assemblies[0].offset, Vec2 { x: -3.0, y: 4.0 });
        assert_eq!(cell.assemblies[0].rotation, Some(-30.0));
    }

    #[test]
    fn a_rotated_map_turns_the_matrix_heading_once_and_leaves_relative_values_alone() {
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: 10.0 },
            rotation: 40.0,
        };
        let reflected = reflected(
            &matrix_with_companion(),
            &empty_target(),
            mapping,
            true,
            false,
        )
        .unwrap();
        // The origin goes through the map and the heading is `rotation - angle`.
        let expected = pair::forward(mapping, Vec2 { x: 10.0, y: 5.0 });
        assert_eq!(reflected.origin, expected);
        assert_eq!(reflected.rotation, Some(20.0));
        // The cell turn and the companion are relative to that frame and to the part, which
        // already carry the pair rotation: only their handedness flips.
        let cell = &reflected.cells[0];
        assert_eq!(cell.rotation, Some(15.0));
        assert_eq!(cell.assemblies[0].offset, Vec2 { x: -3.0, y: 4.0 });
        assert_eq!(cell.assemblies[0].rotation, Some(-30.0));
    }
}
