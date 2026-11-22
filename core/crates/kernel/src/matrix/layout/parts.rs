//! Pair-part placement, mirroring and rotation edits.

use super::{
    MIRRORED_COMPONENT_SOURCE, MIRRORED_COMPONENT_SUBSTITUTE, carrier_offset, evaluated_part_pose,
    is_member_or_companion, layout_half_offset, pair_sides, part_correspondence,
    part_extent_offsets, part_half_offset, reflect_half_part, remove_component,
    retune_driven_constraint, set_component_pose, strip_substitute_flag, sync_component_nets,
};
use crate::model::*;
use boardstudio_model::mechanical::pair;
use std::collections::BTreeMap;

/// Ticket 12: reframe parts whose layout membership changes under SetLayout,
/// preserving every evaluated world pose. Entering parts convert into the
/// incoming half; leaving parts convert back to board-space. The unlink
/// transition skips leaving parts: unadopt already restored them. Matrix
/// members and companions never reframe: the matrix refresh owns them.
pub fn reframe_moved_parts(
    doc: &mut ProjectDoc,
    previous: &Layout,
    incoming: &Layout,
    skip_leaving: bool,
) -> Result<Vec<String>, String> {
    let entering: Vec<String> = incoming
        .part_ids
        .iter()
        .filter(|id| !previous.part_ids.contains(id))
        .cloned()
        .collect();
    let leaving: Vec<String> = if skip_leaving {
        Vec::new()
    } else {
        previous
            .part_ids
            .iter()
            .filter(|id| !incoming.part_ids.contains(id))
            .cloned()
            .collect()
    };
    // Live offsets: gap adoption earlier in the op already stored the final
    // gap, so siblings and newcomers convert alike. The unlink transition
    // reads the incoming record instead: its halves are board-space now.
    let incoming_offset = if skip_leaving {
        carrier_offset(doc, incoming)
    } else {
        layout_half_offset(doc, &incoming.id).or_else(|| carrier_offset(doc, incoming))
    };
    let mut moves: Vec<(String, Option<Vec2>, Option<Vec2>)> = Vec::new();
    for part_id in entering {
        if is_member_or_companion(doc, &part_id) {
            continue;
        }
        let old = doc
            .layouts
            .iter()
            .find(|layout| layout.part_ids.iter().any(|id| id == &part_id))
            .and_then(|layout| layout_half_offset(doc, &layout.id));
        moves.push((part_id, old, incoming_offset));
    }
    for part_id in leaving {
        if is_member_or_companion(doc, &part_id) {
            continue;
        }
        let old = layout_half_offset(doc, &previous.id);
        let new = doc
            .layouts
            .iter()
            .find(|layout| {
                layout.id != previous.id && layout.part_ids.iter().any(|id| id == &part_id)
            })
            .and_then(|layout| layout_half_offset(doc, &layout.id));
        moves.push((part_id, old, new));
    }
    let mut changed = Vec::new();
    for (part_id, old, new) in moves {
        if old == new {
            continue;
        }
        let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) else {
            continue;
        };
        let board_x = part.pose.at.x + old.map(|offset| offset.x).unwrap_or(0.0);
        let board_y = part.pose.at.y + old.map(|offset| offset.y).unwrap_or(0.0);
        part.pose.at.x = board_x - new.map(|offset| offset.x).unwrap_or(0.0);
        part.pose.at.y = board_y - new.map(|offset| offset.y).unwrap_or(0.0);
        changed.push(part_id);
    }
    Ok(changed)
}

/// Ticket 12: store a board-space pose in the half frame: independent parts on
/// a stored-gap half keep within-half values while the canvas, constraints
/// and spacing all work board-space. Frame-free rotation passes through.
pub fn stored_part_pose(doc: &ProjectDoc, part_id: &str, board: Pose2) -> Pose2 {
    match part_half_offset(doc, part_id) {
        Some(offset) => Pose2 {
            at: Vec2 {
                x: board.at.x - offset.x,
                y: board.at.y - offset.y,
            },
            rotation: board.rotation,
        },
        None => board,
    }
}

/// Ticket 12: clamp a board-space drag position so the part's evaluated
/// extent stays on its own side of the split axis: a part cannot cross to the
/// other half. Non-pair parts and poses already clear pass through unchanged;
/// the side follows the part's current evaluated center, never the request.
pub fn clamp_part_to_half(doc: &ProjectDoc, part_id: &str, at: Vec2) -> Vec2 {
    let Some(part) = doc.parts.iter().find(|part| part.id == part_id) else {
        return at;
    };
    let Some(layout) = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
    else {
        return at;
    };
    let Some(axis_x) = doc.layouts.iter().find_map(|other| {
        let link = other.mirror_link.as_ref()?;
        if other.id == layout.id || link.source_id == layout.id {
            Some(link.axis_x)
        } else {
            None
        }
    }) else {
        return at;
    };
    // The request arrives board-space, like the canvas that sent it; sides and
    // extents compare against the evaluated center. Legacy sides compare
    // board-space parts against the axis directly. Ticket 13: the evaluated
    // pose is anchor-derived for anchored parts, whose stored pose goes stale
    // when an anchor column or key moves them.
    let evaluated = evaluated_part_pose(doc, part);
    let on_left = evaluated.at.x < axis_x;
    let (min_rel, max_rel) = match doc
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    {
        Some(definition) => {
            part_extent_offsets(&evaluated, part.side.clone(), part.keycap, definition)
        }
        None => (0.0, 0.0),
    };
    let evaluated_at = at.x;
    if on_left {
        // The upper extent may at most touch the axis.
        let over = evaluated_at + max_rel - axis_x;
        if over > 0.0 {
            return Vec2 {
                x: at.x - over,
                y: at.y,
            };
        }
    } else {
        // The lower extent may at most touch the axis.
        let over = axis_x - (evaluated_at + min_rel);
        if over > 0.0 {
            return Vec2 {
                x: at.x + over,
                y: at.y,
            };
        }
    }
    at
}

/// Ticket 12: create or refresh the mirror partner of `part_id` through its
/// Part correspondence. The edited copy is the authority: the partner takes
/// the reflected half-frame pose, gains layout/board/outline/net membership,
/// and keeps the properties link the legacy paths (removal, orphans) read.
/// Missing partners materialize, so undo and replacement converge. Returns
/// the changed ids. Parts without a correspondence on a stored-gap link, and
/// legacy pairs without a stored gap, resolve through the legacy sync instead.
pub fn ensure_part_mirror(doc: &mut ProjectDoc, part_id: &str) -> Result<Vec<String>, String> {
    let Some((carrier_id, record)) = part_correspondence(doc, part_id) else {
        return Ok(vec![]);
    };
    let (source_layout_id, gapped) = {
        let layout = doc
            .layouts
            .iter()
            .find(|layout| layout.id == carrier_id)
            .ok_or("Linked layout is missing")?;
        let link = layout
            .mirror_link
            .as_ref()
            .ok_or("Linked layout is missing")?;
        (
            link.source_id.clone(),
            link.split_gap.or(link.spacing).is_some(),
        )
    };
    if !gapped {
        return Ok(vec![]);
    }
    let authority_on_source = record.source_id == part_id;
    let (authority_id, partner_id, partner_layout_id) = if authority_on_source {
        (
            record.source_id.clone(),
            record.mirror_id.clone(),
            carrier_id.clone(),
        )
    } else {
        (
            record.mirror_id.clone(),
            record.source_id.clone(),
            source_layout_id.clone(),
        )
    };
    let authority = doc
        .parts
        .iter()
        .find(|part| part.id == authority_id)
        .cloned()
        .ok_or("Mirrored part is missing")?;
    strip_substitute_flag(doc, &authority_id);
    strip_substitute_flag(doc, &partner_id);
    // Ticket 12: a constraint-driven partner owns its pose (the solver sets
    // it), so the sync back-solves the authority from the partner, like the
    // legacy sync did. Otherwise the authority wins and the partner follows.
    if doc.constraint_driving_part(&partner_id)?.is_some() {
        let partner = doc.parts.iter().find(|part| part.id == partner_id).cloned();
        if let Some(partner) = partner {
            // Ticket 13: mirror the fresh evaluated pose, like the forward
            // path below; the stored pose goes stale while anchored.
            let partner_half =
                stored_part_pose(doc, &partner_id, evaluated_part_pose(doc, &partner));
            let source_half = reflect_half_part(doc, &authority_id, partner_half);
            let source_board = match part_half_offset(doc, &authority_id) {
                Some(offset) => Pose2 {
                    at: Vec2 {
                        x: source_half.at.x + offset.x,
                        y: source_half.at.y + offset.y,
                    },
                    rotation: source_half.rotation,
                },
                None => source_half,
            };
            set_component_pose(doc, &authority_id, source_board)?;
            sync_component_nets(doc, &record.source_id, &record.mirror_id);
            return Ok(vec![authority_id, partner_id]);
        }
    }
    // Ticket 13: the authority's stored pose goes stale while anchored (anchor
    // edits move it without rewriting storage), so the reflection starts from
    // the fresh evaluated pose. Unanchored parts evaluate to their stored
    // pose, which keeps this identical.
    let authority_half = stored_part_pose(doc, &authority_id, evaluated_part_pose(doc, &authority));
    let partner_pose = reflect_half_part(doc, &authority_id, authority_half);
    // Ticket 12: a following partner carries its own held offset with it, so
    // its drivers retune, like the legacy sync's pose write did. The drag
    // path already retuned both copies; the delta is zero there and this is a
    // no-op.
    let partner_board = match layout_half_offset(doc, &partner_layout_id) {
        Some(offset) => Pose2 {
            at: Vec2 {
                x: partner_pose.at.x + offset.x,
                y: partner_pose.at.y + offset.y,
            },
            rotation: partner_pose.rotation,
        },
        None => partner_pose,
    };
    let partner_old_rotation = doc
        .parts
        .iter()
        .find(|part| part.id == partner_id)
        .map(|part| part.pose.rotation)
        .unwrap_or(partner_pose.rotation);
    retune_driven_constraint(
        doc,
        &partner_id,
        partner_board,
        Some(partner_pose.rotation - partner_old_rotation),
    )?;
    let partner_exists = doc.parts.iter().any(|part| part.id == partner_id);
    if !partner_exists {
        let mut mirrored = authority.clone();
        mirrored.id = partner_id.clone();
        mirrored.reference = if authority_on_source {
            format!("{}_M", authority.reference)
        } else {
            authority
                .reference
                .strip_suffix("_M")
                .unwrap_or(&authority.reference)
                .to_string()
        };
        mirrored.pose = partner_pose;
        // Ticket 13: the partner anchors to the counterpart key in its own
        // layout with mirrored values, so each half derives its mirrored pose
        // from its own anchor. No counterpart keeps the partner detached at
        // the reflected pose.
        mirrored.anchor = authority.anchor.as_ref().and_then(|anchor| {
            super::super::part_anchors::map_anchor_for_mirror(
                doc,
                anchor,
                &partner_layout_id,
                super::super::part_anchors::is_controller_for(doc, &authority),
            )
        });
        mirrored
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert(
                MIRRORED_COMPONENT_SOURCE.into(),
                serde_json::json!(record.source_id),
            );
        doc.parts.push(mirrored);
        if let Some(layout) = doc
            .layouts
            .iter_mut()
            .find(|layout| layout.id == partner_layout_id)
            && !layout.part_ids.contains(&partner_id)
        {
            layout.part_ids.push(partner_id.clone());
        }
        let board_id = doc
            .layouts
            .iter()
            .find(|layout| layout.id == partner_layout_id)
            .map(|layout| layout.board_id.clone())
            .ok_or("Linked component target layout is missing")?;
        for board in &mut doc.boards {
            if board.id == board_id && !board.part_ids.contains(&partner_id) {
                board.part_ids.push(partner_id.clone());
            }
        }
    }
    if let Some(partner) = doc.parts.iter_mut().find(|part| part.id == partner_id) {
        partner.pose = partner_pose;
        if partner_layout_id == carrier_id {
            partner.properties.get_or_insert_with(BTreeMap::new).insert(
                MIRRORED_COMPONENT_SOURCE.into(),
                serde_json::json!(record.source_id),
            );
        } else if let Some(properties) = partner.properties.as_mut() {
            properties.remove(MIRRORED_COMPONENT_SOURCE);
            if properties.is_empty() {
                partner.properties = None;
            }
        }
    }
    sync_component_nets(doc, &record.source_id, &record.mirror_id);
    for feature in &mut doc.outline {
        if let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = feature
            && part_ids.contains(&record.source_id)
            && !part_ids.contains(&record.mirror_id)
        {
            part_ids.push(record.mirror_id.clone());
        }
    }
    let mut changed = vec![authority_id, partner_id];
    if !partner_exists {
        changed.push(partner_layout_id);
    }
    Ok(changed)
}

/// Ticket 12: mirror a newly added part on a stored-gap half by
/// correspondence in the same edit. The drop point arrives board-space and is
/// already half-local; the partner materializes at the reflected pose.
/// Legacy pairs and ungrouped parts resolve through the existing sync.
pub fn mirror_new_pair_part(doc: &mut ProjectDoc, part_id: &str) -> Result<Vec<String>, String> {
    let layout_id = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
        .map(|layout| layout.id.clone());
    let Some(layout_id) = layout_id else {
        return Ok(vec![]);
    };
    let Some((source_layout_id, target_layout_id, on_source_side, gapped)) =
        pair_sides(doc, &layout_id)
    else {
        return Ok(vec![]);
    };
    if !gapped {
        return Ok(vec![]);
    }
    if part_correspondence(doc, part_id).is_some() {
        return ensure_part_mirror(doc, part_id);
    }
    let (source_part_id, mirror_part_id) = if on_source_side {
        (
            part_id.to_string(),
            mirror_part_id_for(&target_layout_id, part_id, doc),
        )
    } else {
        (
            mirror_part_id_for(&source_layout_id, part_id, doc),
            part_id.to_string(),
        )
    };
    if let Some(layout) = doc
        .layouts
        .iter_mut()
        .find(|layout| layout.id == target_layout_id)
        && let Some(link) = layout.mirror_link.as_mut()
    {
        link.correspondences.push(MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id: source_part_id,
            mirror_id: mirror_part_id,
            companion_mode: None,
            companion_tweak: None,
        });
    }
    let mut changed = ensure_part_mirror(doc, part_id)?;
    changed.push(layout_id);
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// Ticket 12: move-arm write-through for a correspondence-mirrored part on a
/// stored-gap pair: clamp the request to its half, write it, and reflect it
/// onto the partner so either copy edits both. Returns `None` for parts the
/// legacy component sync owns.
pub fn move_pair_part(
    doc: &mut ProjectDoc,
    part_id: &str,
    at: Vec2,
) -> Result<Option<Vec<String>>, String> {
    if is_member_or_companion(doc, part_id) {
        return Ok(None);
    }
    let Some((_, record)) = part_correspondence(doc, part_id) else {
        return Ok(None);
    };
    let layout_id = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
        .map(|layout| layout.id.clone());
    let Some(layout_id) = layout_id else {
        return Ok(None);
    };
    let Some((_, _, _, gapped)) = pair_sides(doc, &layout_id) else {
        return Ok(None);
    };
    if !gapped {
        return Ok(None);
    }
    // The drag arrives board-space; storage is half-frame, so the dragged copy
    // converts and the partner reflects the half-frame pose. Either copy can
    // carry a held offset, so both retune before the write, like a direct
    // drag of each copy would.
    let clamped = clamp_part_to_half(doc, part_id, at);
    let partner_id = if record.source_id == part_id {
        record.mirror_id.clone()
    } else {
        record.source_id.clone()
    };
    if doc.parts.iter().all(|part| part.id != partner_id) {
        return Err("Mirrored part is missing".into());
    }
    let dragged_rotation = doc
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .map(|part| part.pose.rotation)
        .unwrap_or(0.0);
    let stored = stored_part_pose(
        doc,
        part_id,
        Pose2 {
            at: clamped,
            rotation: dragged_rotation,
        },
    );
    let partner_pose = reflect_half_part(doc, part_id, stored);
    let axis_x = doc.layouts.iter().find_map(|other| {
        let link = other.mirror_link.as_ref()?;
        if other.id == layout_id || link.source_id == layout_id {
            Some(link.axis_x)
        } else {
            None
        }
    });
    retune_driven_constraint(
        doc,
        part_id,
        Pose2 {
            at: clamped,
            rotation: dragged_rotation,
        },
        None,
    )?;
    if let Some(axis_x) = axis_x {
        let partner_old_rotation = doc
            .parts
            .iter()
            .find(|part| part.id == partner_id)
            .map(|part| part.pose.rotation)
            .unwrap_or(0.0);
        retune_driven_constraint(
            doc,
            &partner_id,
            Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x - clamped.x,
                    y: clamped.y,
                },
                rotation: partner_pose.rotation,
            },
            Some(partner_pose.rotation - partner_old_rotation),
        )?;
    }
    if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) {
        part.pose.at = stored.at;
    } else {
        return Err(format!("Unknown part {part_id}"));
    }
    if let Some(partner) = doc.parts.iter_mut().find(|part| part.id == partner_id) {
        partner.pose = partner_pose;
    }
    Ok(Some(vec![part_id.to_string(), partner_id]))
}

/// Ticket 07: set a detached independent part's board rotation, fanning out
/// to a mirrored partner in the same Undo entry. Either copy is the
/// authority: the partner takes the reflected half-frame heading per the
/// shared mirrored-part rule. Locked, member/companion, anchored and
/// constraint-driven parts refuse; the caller checks finiteness.
pub fn set_detached_part_rotation(
    doc: &mut ProjectDoc,
    part_id: &str,
    rotation: f64,
) -> Result<Vec<String>, String> {
    if !rotation.is_finite() {
        return Err("Part rotation must be finite".into());
    }
    let Some(index) = doc.parts.iter().position(|part| part.id == part_id) else {
        return Err(format!("Unknown part {part_id}"));
    };
    if doc.parts[index].locked == Some(true) {
        return Err(format!("Part {part_id} is locked"));
    }
    if is_member_or_companion(doc, part_id) {
        return Err(format!(
            "Part {part_id} follows its matrix and cannot rotate"
        ));
    }
    if doc.parts[index].anchor.is_some() {
        return Err(format!(
            "Part {part_id} is anchored; use SetPartAnchorValues"
        ));
    }
    if doc.constraint_driving_part(part_id)?.is_some() {
        return Err(format!(
            "Part {part_id} is driven by a constraint and cannot rotate"
        ));
    }
    // Board rotation is frame-free, so the stored rotation matches.
    let current_board = evaluated_part_pose(doc, &doc.parts[index].clone()).rotation;
    let turn = rotation - current_board;
    // Correspondence-mirrored part on a stored-gap pair writes both copies,
    // so either copy edits both.
    if let Some((_, record)) = part_correspondence(doc, part_id) {
        let layout_id = doc
            .layouts
            .iter()
            .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
            .map(|layout| layout.id.clone());
        if let Some(layout_id) = layout_id
            && let Some((_, _, _, gapped)) = pair_sides(doc, &layout_id)
            && gapped
        {
            let partner_id = if record.source_id == part_id {
                record.mirror_id.clone()
            } else {
                record.source_id.clone()
            };
            if doc.parts.iter().all(|part| part.id != partner_id) {
                return Err("Mirrored part is missing".into());
            }
            let stored = stored_part_pose(
                doc,
                part_id,
                Pose2 {
                    at: evaluated_part_pose(doc, &doc.parts[index].clone()).at,
                    rotation,
                },
            );
            let partner_pose = reflect_half_part(doc, part_id, stored);
            let partner_old = doc
                .parts
                .iter()
                .find(|part| part.id == partner_id)
                .map(|part| evaluated_part_pose(doc, part).rotation)
                .unwrap_or(0.0);
            retune_driven_constraint(
                doc,
                part_id,
                Pose2 {
                    at: evaluated_part_pose(doc, &doc.parts[index].clone()).at,
                    rotation,
                },
                Some(turn),
            )?;
            let partner_at = evaluated_part_pose(
                doc,
                doc.parts
                    .iter()
                    .find(|part| part.id == partner_id)
                    .expect("partner checked"),
            )
            .at;
            retune_driven_constraint(
                doc,
                &partner_id,
                Pose2 {
                    at: partner_at,
                    rotation: partner_pose.rotation,
                },
                Some(partner_pose.rotation - partner_old),
            )?;
            if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) {
                part.pose = stored;
            } else {
                return Err(format!("Unknown part {part_id}"));
            }
            if let Some(partner) = doc.parts.iter_mut().find(|part| part.id == partner_id) {
                partner.pose = partner_pose;
            }
            return Ok(vec![part_id.to_string(), partner_id]);
        }
    }
    // Legacy property-linked copy: editing the mirror copy back-drives the
    // source so the resolve sync keeps both. Editing the source relies on the
    // post-apply component sync for the partner.
    if let Some(target) = doc.parts.iter().find(|part| part.id == part_id).cloned()
        && let Some(properties) = target.properties.as_ref()
        && let Some(source_id) = properties
            .get(MIRRORED_COMPONENT_SOURCE)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        && !properties
            .get(MIRRORED_COMPONENT_SUBSTITUTE)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    {
        let Some(source_layout) = doc
            .layouts
            .iter()
            .find(|layout| layout.part_ids.contains(&source_id))
            .cloned()
        else {
            // Orphaned link: write the edited copy alone; sync cleans up.
            let board_at = evaluated_part_pose(
                doc,
                doc.parts
                    .iter()
                    .find(|part| part.id == part_id)
                    .expect("edited part checked"),
            )
            .at;
            let stored = stored_part_pose(
                doc,
                part_id,
                Pose2 {
                    at: board_at,
                    rotation,
                },
            );
            if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) {
                part.pose = stored;
            }
            return Ok(vec![part_id.to_string()]);
        };
        let Some(target_layout) = doc.layouts.iter().find(|layout| {
            layout
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.source_id == source_layout.id)
        }) else {
            let board_at = evaluated_part_pose(
                doc,
                doc.parts
                    .iter()
                    .find(|part| part.id == part_id)
                    .expect("edited part checked"),
            )
            .at;
            let stored = stored_part_pose(
                doc,
                part_id,
                Pose2 {
                    at: board_at,
                    rotation,
                },
            );
            if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) {
                part.pose = stored;
            }
            return Ok(vec![part_id.to_string()]);
        };
        let axis_x = target_layout
            .mirror_link
            .as_ref()
            .map(|link| link.axis_x)
            .unwrap_or(0.0);
        let is_controller = doc.definitions.iter().any(|definition| {
            definition.id == target.definition_id && definition.kind == PartKind::Controller
        });
        let axis_mapping = Pose2 {
            at: Vec2 {
                x: 2.0 * axis_x,
                y: 0.0,
            },
            rotation: 0.0,
        };
        let source_rotation = pair::mirrored_part_heading(is_controller, axis_mapping, rotation);
        let source_board = evaluated_part_pose(
            doc,
            doc.parts
                .iter()
                .find(|part| part.id == source_id)
                .ok_or("Mirrored component source is missing")?,
        );
        retune_driven_constraint(
            doc,
            part_id,
            Pose2 {
                at: evaluated_part_pose(doc, &target).at,
                rotation,
            },
            Some(turn),
        )?;
        retune_driven_constraint(
            doc,
            &source_id,
            Pose2 {
                at: source_board.at,
                rotation: source_rotation,
            },
            Some(
                source_rotation
                    - doc
                        .parts
                        .iter()
                        .find(|part| part.id == source_id)
                        .map(|part| evaluated_part_pose(doc, part).rotation)
                        .unwrap_or(source_rotation),
            ),
        )?;
        set_component_pose(
            doc,
            &source_id,
            Pose2 {
                at: source_board.at,
                rotation: source_rotation,
            },
        )?;
        let edited_at = evaluated_part_pose(doc, &target).at;
        let stored = stored_part_pose(
            doc,
            part_id,
            Pose2 {
                at: edited_at,
                rotation,
            },
        );
        if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) {
            part.pose = stored;
        }
        return Ok(vec![part_id.to_string(), source_id]);
    }
    retune_driven_constraint(
        doc,
        part_id,
        Pose2 {
            at: evaluated_part_pose(doc, &doc.parts[index].clone()).at,
            rotation,
        },
        Some(turn),
    )?;
    let board = evaluated_part_pose(doc, &doc.parts[index].clone());
    let stored = stored_part_pose(
        doc,
        part_id,
        Pose2 {
            at: board.at,
            rotation,
        },
    );
    doc.parts[index].pose = stored;
    Ok(vec![part_id.to_string()])
}

/// Ticket 12: drop Part correspondences naming any removed part, so removal
/// of a mirrored copy (or its source) never leaves a dangling record behind.
pub fn retire_part_correspondences(doc: &mut ProjectDoc, ids: &[String]) {
    for layout in &mut doc.layouts {
        if let Some(link) = layout.mirror_link.as_mut() {
            link.correspondences.retain(|record| {
                record.kind != MirroredItemKind::Part
                    || (!ids.contains(&record.source_id) && !ids.contains(&record.mirror_id))
            });
        }
    }
}

/// Ticket 12: the Inspector "Mirror on both halves" toggle. `mirrored =
/// false` removes the partner and drops the correspondence, keeping the named
/// copy local; `mirrored = true` re-materializes the partner at the reflected
/// pose. Each direction lands in one Undo entry; already-there is a no-op.
pub fn set_part_mirrored(
    doc: &mut ProjectDoc,
    part_id: &str,
    mirrored: bool,
) -> Result<Vec<String>, String> {
    if !doc.parts.iter().any(|part| part.id == part_id) {
        return Err(format!("Unknown part {part_id}"));
    }
    if is_member_or_companion(doc, part_id) {
        return Err(format!(
            "Part {part_id} follows its matrix; mirror the matrix instead"
        ));
    }
    let layout_id = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
        .map(|layout| layout.id.clone())
        .ok_or_else(|| format!("Part {part_id} is not on a mirrored half"))?;
    let Some((source_layout_id, target_layout_id, on_source_side, gapped)) =
        pair_sides(doc, &layout_id)
    else {
        return Err(format!("Part {part_id} is not on a mirrored half"));
    };
    if mirrored {
        if part_correspondence(doc, part_id).is_some() {
            return Ok(vec![]);
        }
        if !gapped {
            // Legacy pairs mirror every source part on resolve: clearing the
            // local flag re-mirrors through the properties without new state.
            if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id)
                && let Some(properties) = part.properties.as_mut()
            {
                properties.remove(MIRRORED_COMPONENT_SUBSTITUTE);
                if properties.is_empty() {
                    part.properties = None;
                }
            }
            return Ok(vec![part_id.to_string()]);
        }
        // The record always names the source-layout copy as the source.
        // Re-mirroring clears the local pin, if any.
        strip_substitute_flag(doc, part_id);
        let (source_part_id, mirror_part_id) = if on_source_side {
            (
                part_id.to_string(),
                mirror_part_id_for(&target_layout_id, part_id, doc),
            )
        } else {
            (
                mirror_part_id_for(&source_layout_id, part_id, doc),
                part_id.to_string(),
            )
        };
        if let Some(layout) = doc
            .layouts
            .iter_mut()
            .find(|layout| layout.id == target_layout_id)
            && let Some(link) = layout.mirror_link.as_mut()
        {
            link.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Part,
                source_id: source_part_id,
                mirror_id: mirror_part_id,
                companion_mode: None,
                companion_tweak: None,
            });
        }
        let changed = ensure_part_mirror(doc, part_id)?;
        let mut changed = changed;
        changed.push(layout_id);
        changed.sort();
        changed.dedup();
        return Ok(changed);
    }
    // Making the part local removes the partner, keeping the named copy. The
    // substitute flag pins it local so the stored-gap sync does not
    // re-materialize a default mirror for it.
    if gapped {
        let Some((_, record)) = part_correspondence(doc, part_id) else {
            return Ok(vec![]);
        };
        let partner_id = if record.source_id == part_id {
            record.mirror_id.clone()
        } else {
            record.source_id.clone()
        };
        retire_part_correspondences(doc, std::slice::from_ref(&partner_id));
        remove_component(doc, &partner_id);
        if let Some(kept) = doc.parts.iter_mut().find(|part| part.id == part_id) {
            let properties = kept.properties.get_or_insert_with(BTreeMap::new);
            properties.insert(
                MIRRORED_COMPONENT_SUBSTITUTE.into(),
                serde_json::json!(true),
            );
            properties.remove(MIRRORED_COMPONENT_SOURCE);
        }
        return Ok(vec![part_id.to_string(), partner_id]);
    }
    // Legacy pairs mirror through the properties: remove the partner and pin
    // the kept copy local so resolve does not recreate it.
    let partner_id = doc.parts.iter().find_map(|candidate| {
        let source = candidate
            .properties
            .as_ref()?
            .get(MIRRORED_COMPONENT_SOURCE)?
            .as_str()?;
        if candidate.id == part_id {
            Some(source.to_string())
        } else if source == part_id {
            Some(candidate.id.clone())
        } else {
            None
        }
    });
    let mut changed = vec![part_id.to_string()];
    if let Some(partner_id) = partner_id {
        remove_component(doc, &partner_id);
        changed.push(partner_id);
    }
    if let Some(kept) = doc.parts.iter_mut().find(|part| part.id == part_id) {
        let properties = kept.properties.get_or_insert_with(BTreeMap::new);
        properties.insert(
            MIRRORED_COMPONENT_SUBSTITUTE.into(),
            serde_json::json!(true),
        );
        properties.remove(MIRRORED_COMPONENT_SOURCE);
    }
    Ok(changed)
}

/// Fresh partner id on `other_layout_id` mirroring `part_id`, colliding past
/// existing parts like the legacy component sync.
pub(super) fn mirror_part_id_for(other_layout_id: &str, part_id: &str, doc: &ProjectDoc) -> String {
    let base = format!("{other_layout_id}::mirror::{part_id}");
    if doc.parts.iter().all(|part| part.id != base) {
        return base;
    }
    let mut suffix = 1;
    while doc
        .parts
        .iter()
        .any(|part| part.id == format!("{base}::{suffix}"))
    {
        suffix += 1;
    }
    format!("{base}::{suffix}")
}
