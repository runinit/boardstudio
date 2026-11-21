//! Capture and restoration of omitted copper, nets and rejoined anchors.

use super::super::{MembershipContext, TransitionState};
use super::{effective, parent};
use crate::model::{AggregateAnchor, AnchorPart, MirroredItemKind};
use std::collections::BTreeSet;

/// Copper is matched to a pad by position; both come from authored geometry, so a hair of
/// tolerance is enough and anything coarser would grab a neighbouring pad's trace.
pub(super) const COPPER_TOLERANCE: f64 = 1e-6;

pub(super) fn near(a: crate::model::Vec2, b: crate::model::Vec2) -> bool {
    (a.x - b.x).abs() <= COPPER_TOLERANCE && (a.y - b.y).abs() <= COPPER_TOLERANCE
}

/// The world position of every pad of a part, from the part's pose and its definition.
pub(super) fn pad_positions(
    document: &crate::model::ProjectDoc,
    part_id: &str,
) -> Vec<crate::model::OmittedPad> {
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return Vec::new();
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return Vec::new();
    };
    // Ticket 12: omitted pads report the evaluated board pose of half-frame
    // parts.
    let pose = crate::matrix::layout::evaluated_part_pose(document, part);
    definition
        .pads
        .iter()
        .map(|pad| crate::model::OmittedPad {
            part_id: part.id.clone(),
            pad_id: pad.id.clone(),
            at: crate::outline_controls::world(pad.at, &pose, part.side.clone()),
        })
        .collect()
}

/// The explicitly selected item an inherited item takes its state from.
pub(super) fn explicit_root<'a>(
    context: &'a MembershipContext,
    association: &'a super::super::TransitionAssociation,
) -> &'a super::super::TransitionAssociation {
    let mut current = association;
    for _ in 0..context.associations.len() {
        if current.explicit_state {
            break;
        }
        match parent(context, current) {
            Ok(Some(owner)) => current = owner,
            _ => break,
        }
    }
    current
}

/// Removes the mirror-board copper that belonged only to the items this transition omits and
/// returns it by the explicitly omitted item that owns it: copper on a net the omission leaves
/// private, and any copper that ends on an omitted pad. The source half is never touched.
pub(super) fn capture_omitted_copper(
    draft: &mut crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<std::collections::BTreeMap<String, crate::model::OmittedCopper>, String> {
    use std::collections::BTreeMap;
    let mut newly = Vec::new();
    for association in &context.associations {
        if association.state == TransitionState::Omitted
            && effective(context, association, false, 0)? != TransitionState::Omitted
        {
            newly.push(association);
        }
    }
    if newly.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut part_owner: BTreeMap<String, String> = BTreeMap::new();
    let mut net_owner: BTreeMap<String, String> = BTreeMap::new();
    for association in &newly {
        let root = explicit_root(context, association).mirror_id.clone();
        match association.kind {
            MirroredItemKind::Part => {
                part_owner.insert(association.mirror_id.clone(), root);
            }
            MirroredItemKind::Net => {
                net_owner.insert(association.mirror_id.clone(), root);
            }
            _ => {}
        }
    }
    let omitted_parts: BTreeSet<String> = part_owner.keys().cloned().collect();

    let mut saved: BTreeMap<String, crate::model::OmittedCopper> = BTreeMap::new();
    let mut pad_points = Vec::new();
    for (part_id, owner) in &part_owner {
        for pad in pad_positions(draft, part_id) {
            pad_points.push((pad.at, owner.clone()));
            saved
                .entry(owner.clone())
                .or_insert_with(|| crate::model::OmittedCopper {
                    traces: Vec::new(),
                    vias: Vec::new(),
                    pads: Vec::new(),
                })
                .pads
                .push(pad);
        }
    }

    // A net is private to the omission when every non-controller pin of its source net is on an
    // omitted part. It goes with them, unless something the mirror authored still uses it.
    let source_controller = post_source
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == context.source_board_id)
        })
        .and_then(|board| board.controller_part_id.as_deref());
    let mirror_controller = source_controller
        .and_then(|id| {
            context
                .pair_right
                .correspondences
                .iter()
                .find(|record| record.kind == MirroredItemKind::Part && record.source_id == id)
        })
        .map(|record| record.mirror_id.clone());
    let local_mirror_controller = mirror_controller.as_ref().is_some_and(|mirror_id| {
        context.associations.iter().any(|association| {
            association.kind == MirroredItemKind::Part
                && association.mirror_id == *mirror_id
                && association.state == TransitionState::Local
        })
    });
    let mut rule_nets = BTreeSet::new();
    for record in context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Net)
    {
        if net_owner.contains_key(&record.mirror_id)
            || record.mirror_id.starts_with("embedded/")
            || context.pair_right.overlay_of(record).is_some()
        {
            continue;
        }
        let Some(mirror_net) = draft.nets.iter().find(|net| net.id == record.mirror_id) else {
            continue;
        };
        if !crate::mechanical::halves::private_net_is_absent_with(
            post_source,
            draft,
            &context.pair_right,
            &record.source_id,
            &omitted_parts,
        ) {
            continue;
        }
        let Some(source_net) = post_source
            .nets
            .iter()
            .find(|net| net.id == record.source_id)
        else {
            continue;
        };
        let owner = source_net
            .pins
            .iter()
            .filter_map(|pin| {
                context
                    .pair_right
                    .correspondences
                    .iter()
                    .find(|c| c.kind == MirroredItemKind::Part && c.source_id == pin.part_id)
            })
            .filter_map(|c| part_owner.get(&c.mirror_id))
            .min()
            .cloned();
        let Some(owner) = owner else {
            continue;
        };
        let still_used = mirror_net.pins.iter().any(|pin| {
            !omitted_parts.contains(&pin.part_id)
                && (Some(&pin.part_id) != mirror_controller.as_ref() || local_mirror_controller)
                && draft.parts.iter().any(|part| part.id == pin.part_id)
        }) || draft.embedded_circuits.iter().any(|circuit| {
            circuit.ports.values().any(|net| *net == record.mirror_id)
                || circuit.net_ids.contains(&record.mirror_id)
        });
        if still_used {
            continue;
        }
        net_owner.insert(record.mirror_id.clone(), owner);
        rule_nets.insert(record.mirror_id.clone());
    }

    let board = draft
        .boards
        .iter_mut()
        .find(|board| board.id == context.mirror_board_id)
        .ok_or("The Omit mirror board is unavailable")?;
    let owner_at = |at: crate::model::Vec2| {
        pad_points
            .iter()
            .find(|(pad, _)| near(*pad, at))
            .map(|(_, owner)| owner.clone())
    };
    let mut kept_traces = Vec::new();
    for trace in std::mem::take(&mut board.traces) {
        let owner = trace
            .net_id
            .as_ref()
            .and_then(|net| net_owner.get(net).cloned())
            .or_else(|| owner_at(trace.start))
            .or_else(|| owner_at(trace.end));
        match owner {
            Some(owner) => saved
                .entry(owner)
                .or_insert_with(|| crate::model::OmittedCopper {
                    traces: Vec::new(),
                    vias: Vec::new(),
                    pads: Vec::new(),
                })
                .traces
                .push(trace),
            None => kept_traces.push(trace),
        }
    }
    board.traces = kept_traces;
    let mut kept_vias = Vec::new();
    for via in std::mem::take(&mut board.vias) {
        let owner = via
            .net_id
            .as_ref()
            .and_then(|net| net_owner.get(net).cloned())
            .or_else(|| owner_at(via.at));
        match owner {
            Some(owner) => saved
                .entry(owner)
                .or_insert_with(|| crate::model::OmittedCopper {
                    traces: Vec::new(),
                    vias: Vec::new(),
                    pads: Vec::new(),
                })
                .vias
                .push(via),
            None => kept_vias.push(via),
        }
    }
    board.vias = kept_vias;
    board.net_ids.retain(|id| !rule_nets.contains(id));
    draft.nets.retain(|net| !rule_nets.contains(&net.id));
    // Pads without copper say nothing worth restoring.
    saved.retain(|_, copper| !copper.traces.is_empty() || !copper.vias.is_empty());
    Ok(saved)
}

/// A Rejoin brings back the nets the omission had left private, and the rejoined parts' pins on
/// the nets that stayed, from the current source.
pub(super) fn restore_rejoined_nets(
    draft: &mut crate::model::ProjectDoc,
    post_source: &crate::model::ProjectDoc,
    context: &MembershipContext,
) -> Result<(), String> {
    let rejoined: BTreeSet<String> = context
        .associations
        .iter()
        .filter(|a| {
            a.kind == MirroredItemKind::Part
                && a.state == TransitionState::Linked
                && context.pending_rejoins.get(&a.source_id) == Some(&TransitionState::Omitted)
        })
        .map(|a| a.mirror_id.clone())
        .collect();
    if rejoined.is_empty() {
        return Ok(());
    }
    let part_map: std::collections::BTreeMap<_, _> = context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Part)
        .map(|record| (record.source_id.clone(), record.mirror_id.clone()))
        .collect();
    for record in context
        .pair_right
        .correspondences
        .iter()
        .filter(|record| record.kind == MirroredItemKind::Net)
    {
        if record.mirror_id.starts_with("embedded/")
            || context.pair_right.overlay_of(record).is_some()
        {
            continue;
        }
        let Some(source_net) = post_source
            .nets
            .iter()
            .find(|net| net.id == record.source_id)
        else {
            continue;
        };
        let mapped: Vec<crate::model::Pin> = source_net
            .pins
            .iter()
            .filter_map(|pin| {
                let mirror = part_map.get(&pin.part_id)?;
                draft
                    .parts
                    .iter()
                    .any(|part| part.id == *mirror)
                    .then(|| crate::model::Pin {
                        part_id: mirror.clone(),
                        pad_id: pin.pad_id.clone(),
                    })
            })
            .collect();
        if !mapped.iter().any(|pin| rejoined.contains(&pin.part_id)) {
            continue;
        }
        if let Some(net) = draft.nets.iter_mut().find(|net| net.id == record.mirror_id) {
            for pin in mapped
                .into_iter()
                .filter(|pin| rejoined.contains(&pin.part_id))
            {
                if !net.pins.contains(&pin) {
                    net.pins.push(pin);
                }
            }
        } else {
            let board = draft
                .boards
                .iter_mut()
                .find(|board| board.id == context.mirror_board_id)
                .ok_or("The Rejoin mirror board is unavailable")?;
            if board.net_ids.contains(&record.mirror_id) {
                return Err("An absent Rejoin net retains a board registration".into());
            }
            board.net_ids.push(record.mirror_id.clone());
            draft.nets.push(crate::model::Net {
                id: record.mirror_id.clone(),
                name: super::super::super::mirror_net_name(
                    &context.source_board_id,
                    &context.mirror_board_id,
                    &source_net.name,
                ),
                pins: mapped,
            });
        }
    }
    Ok(())
}

/// Puts back the copper an Omit saved, but only if every saved pad stands exactly where it did.
/// Moved pads require fresh routing. In either case, the saved copper is gone from
/// the pair, because Rejoin clears the overlay that held it.
pub(super) fn restore_saved_copper(
    output: &mut super::super::MembershipTransitionOutput,
    context: &MembershipContext,
) -> Result<(), String> {
    for overlay in &context.pair_right.exceptions {
        let Some(copper) = &overlay.removed_copper else {
            continue;
        };
        let Some(record) = context.pair_right.overlaid_correspondence(overlay) else {
            continue;
        };
        let Some(association) = context.associations.iter().find(|a| {
            a.kind == record.kind
                && a.source_id == record.source_id
                && a.mirror_id == record.mirror_id
        }) else {
            continue;
        };
        if association.state != TransitionState::Linked
            || !context.pending_rejoins.contains_key(&association.source_id)
        {
            continue;
        }
        let unmoved = copper.pads.iter().all(|pad| {
            pad_positions(&output.draft, &pad.part_id)
                .iter()
                .any(|now| now.pad_id == pad.pad_id && near(now.at, pad.at))
        });
        if !unmoved {
            continue;
        }
        let board = output
            .draft
            .boards
            .iter_mut()
            .find(|board| board.id == context.mirror_board_id)
            .ok_or("The Rejoin mirror board is unavailable")?;
        if copper
            .traces
            .iter()
            .any(|trace| board.traces.iter().any(|live| live.id == trace.id))
            || copper
                .vias
                .iter()
                .any(|via| board.vias.iter().any(|live| live.id == via.id))
        {
            return Err(format!(
                "Saved copper for {:?} collides with live copper",
                overlay.mirror_id
            ));
        }
        board.traces.extend(copper.traces.iter().cloned());
        board.vias.extend(copper.vias.iter().cloned());
    }
    Ok(())
}

pub(super) fn copy_part_payload(target: &mut crate::model::Part, source: &crate::model::Part) {
    target.side = source.side.clone();
    target.locked = source.locked;
    target.keycap = source.keycap;
    target.outline = source.outline.clone();
    target.properties = source.properties.clone();
    target.generator_parameters = source.generator_parameters.clone();
}

/// Coordinate durable exceptions with the existing forward projector. Membership-changing
/// matrix edits continue through `prepare_membership_transition`; this seam handles fixed
/// membership and selective independent/hosted exceptions.
/// Ticket 29: a Rejoin adopts the source anchor onto the mirror half. The
/// projection copies the anchor verbatim, so remap every target through the
/// pair's matrix correspondences and refuse — naming the missing key —
/// instead of materializing a cross-half anchor or silently retargeting.
pub(in crate::mirrored_half::reconcile) fn adopt_rejoined_anchor(
    projected: &mut crate::model::Matrix,
    draft: &crate::model::ProjectDoc,
    context: &MembershipContext,
    matrix_pair: &super::super::MatrixPairContext,
) -> Result<(), String> {
    let Some(anchor) = projected.anchor.clone() else {
        return Ok(());
    };
    let counterpart = |matrix_id: &str| {
        context
            .pair_right
            .correspondences
            .iter()
            .find(|record| record.kind == MirroredItemKind::Matrix && record.source_id == matrix_id)
            .map(|record| record.mirror_id.clone())
    };
    // A counterpart slot is missing when its matrix record is gone, the
    // slot is out of range, or the slot is explicitly removed.
    let slot_missing = |mirror_id: &str, row: u32, column: u32| -> bool {
        let Some(record) = draft.matrices.iter().find(|matrix| matrix.id == mirror_id) else {
            return true;
        };
        if row >= record.rows || column >= record.columns {
            return true;
        }
        record
            .cells
            .iter()
            .any(|cell| cell.row == row && cell.column == column && (cell.deleted || !cell.enabled))
    };
    let mut next = anchor.clone();
    match &anchor.aggregate {
        Some(aggregate) => {
            let mut parts = Vec::with_capacity(aggregate.parts.len());
            for part in &aggregate.parts {
                let Some(mirror_id) = counterpart(&part.matrix_id) else {
                    return Err(format!(
                        "Cannot Rejoin {}: the adopted source anchor targets {:?}, which has no mirrored counterpart in this pair",
                        matrix_pair.mirror_id, part.matrix_id
                    ));
                };
                if slot_missing(&mirror_id, part.row, part.column) {
                    return Err(format!(
                        "Cannot Rejoin {}: the adopted source anchor targets matrix/{}/r{}c{}, whose mirror counterpart matrix/{}/r{}c{} is missing",
                        matrix_pair.mirror_id,
                        part.matrix_id,
                        part.row,
                        part.column,
                        mirror_id,
                        part.row,
                        part.column
                    ));
                }
                parts.push(AnchorPart {
                    matrix_id: mirror_id,
                    row: part.row,
                    column: part.column,
                });
            }
            next.aggregate = Some(AggregateAnchor {
                method: aggregate.method,
                parts,
            });
        }
        None => {
            let Some(mirror_id) = counterpart(&anchor.matrix_id) else {
                return Err(format!(
                    "Cannot Rejoin {}: the adopted source anchor targets {:?}, which has no mirrored counterpart in this pair",
                    matrix_pair.mirror_id, anchor.matrix_id
                ));
            };
            if slot_missing(&mirror_id, anchor.row, anchor.column) {
                return Err(format!(
                    "Cannot Rejoin {}: the adopted source anchor targets matrix/{}/r{}c{}, whose mirror counterpart matrix/{}/r{}c{} is missing",
                    matrix_pair.mirror_id,
                    anchor.matrix_id,
                    anchor.row,
                    anchor.column,
                    mirror_id,
                    anchor.row,
                    anchor.column
                ));
            }
            next.matrix_id = mirror_id;
        }
    }
    projected.anchor = Some(next);
    Ok(())
}
