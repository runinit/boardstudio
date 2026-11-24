//! Half-frame adoption, unadoption and evaluated part poses for linked matrix pairs.

use super::{
    MIRRORED_COMPONENT_SOURCE, MIRRORED_COMPONENT_SUBSTITUTE, carrier_offset, gap_from_origin,
};
use crate::{matrix, model::*};
use std::borrow::Cow;

/// Adopt the half frame for every legacy pair (no stored gap) that has an
/// unanchored source matrix: store the implied gap and re-express unanchored
/// World origins as half-frame values. Every evaluated world pose is
/// preserved, so adoption moves nothing. Fully anchored pairs stay legacy;
/// spacing owns their envelopes (ticket 10).
pub fn adopt_half_frames(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let targets: Vec<String> = doc
        .layouts
        .iter()
        .filter(|layout| {
            layout
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.split_gap.is_none())
        })
        .map(|layout| layout.id.clone())
        .collect();
    let mut changed = Vec::new();
    for target_id in targets {
        let (source_id, axis_x) = {
            let link = doc
                .layouts
                .iter()
                .find(|layout| layout.id == target_id)
                .and_then(|layout| layout.mirror_link.as_ref())
                .ok_or("Linked layout is missing")?;
            (link.source_id.clone(), link.axis_x)
        };
        let Some(reference) = doc
            .layouts
            .iter()
            .find(|layout| layout.id == source_id)
            .map(|layout| layout.matrix_ids.clone())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| doc.matrices.iter().find(|matrix| matrix.id == id))
            .find(|matrix| matrix.anchor.is_none())
            .map(|matrix| (matrix.origin.x, matrix.pitch.x, matrix.edge_gap))
        else {
            continue;
        };
        let gap = gap_from_origin(axis_x, reference.0, reference.1, reference.2);
        if !gap.is_finite() {
            return Err("Split gap must be finite".into());
        }
        let members: Vec<(String, bool)> = {
            let source_ids = doc
                .layouts
                .iter()
                .find(|layout| layout.id == source_id)
                .map(|layout| layout.matrix_ids.clone())
                .unwrap_or_default();
            let target_ids = doc
                .layouts
                .iter()
                .find(|layout| layout.id == target_id)
                .map(|layout| layout.matrix_ids.clone())
                .unwrap_or_default();
            source_ids
                .into_iter()
                .map(|id| (id, false))
                .chain(target_ids.into_iter().map(|id| (id, true)))
                .collect()
        };
        for (matrix_id, mirror_side) in members {
            if let Some(matrix) = doc
                .matrices
                .iter_mut()
                .find(|matrix| matrix.id == matrix_id)
                && matrix.anchor.is_none()
            {
                matrix.origin.x -= if mirror_side {
                    axis_x + gap / 2.0
                } else {
                    axis_x - gap / 2.0
                };
                changed.push(matrix_id);
            }
        }
        if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == target_id)
            && let Some(link) = layout.mirror_link.as_mut()
        {
            link.split_gap = Some(gap);
            changed.push(target_id.clone());
        }
        changed.extend(adopt_pair_parts(doc, &source_id, &target_id, axis_x, gap)?);
    }
    Ok(changed)
}

/// Ticket 12: adopt the half frame for a pair's independent parts: re-express
/// board-space poses as half-frame values (rotation is frame-free), preserving
/// every evaluated world pose, and record each properties-linked mirror pair
/// as a Part correspondence on the mirror link so the toggle (ticket 12) and
/// the mirror sync own them by record. Unlinked parts stay local by absence.
pub(super) fn adopt_pair_parts(
    doc: &mut ProjectDoc,
    source_id: &str,
    target_id: &str,
    axis_x: f64,
    gap: f64,
) -> Result<Vec<String>, String> {
    let mut changed = Vec::new();
    let sides: Vec<(String, bool)> = [source_id, target_id]
        .into_iter()
        .map(|id| {
            let mirror_side = id == target_id;
            (id.to_string(), mirror_side)
        })
        .collect();
    for (layout_id, mirror_side) in &sides {
        let part_ids = doc
            .layouts
            .iter()
            .find(|layout| layout.id == *layout_id)
            .map(|layout| layout.part_ids.clone())
            .unwrap_or_default();
        for part_id in part_ids {
            if is_member_or_companion(doc, &part_id) {
                continue;
            }
            if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) {
                let offset = if *mirror_side {
                    axis_x + gap / 2.0
                } else {
                    axis_x - gap / 2.0
                };
                part.pose.at.x -= offset;
                changed.push(part_id.clone());
            }
        }
    }
    // Record the properties-linked mirrors the legacy sync maintained, so the
    // pair keeps mirroring them by correspondence under the half frame.
    for (source_part, mirror_part) in unrecorded_mirror_pairs(doc, source_id, target_id) {
        strip_substitute_flag(doc, &source_part);
        strip_substitute_flag(doc, &mirror_part);
        if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == target_id)
            && let Some(link) = layout.mirror_link.as_mut()
        {
            link.correspondences.push(LayoutLinkPair {
                kind: MirroredItemKind::Part,
                source_id: source_part.clone(),
                mirror_id: mirror_part.clone(),
            });
            changed.push(source_part);
            changed.push(mirror_part);
        }
    }
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// Ticket 12: properties-linked mirrors on a pair that carry no Part
/// correspondence yet: legacy sync output (board-space) that never passed
/// adoption, e.g. pairs created with their gap stored. Local-substitute parts
/// stay out: their flag pins them local.
pub(super) fn unrecorded_mirror_pairs(
    doc: &ProjectDoc,
    source_id: &str,
    target_id: &str,
) -> Vec<(String, String)> {
    let has_substitute = |id: &str| {
        doc.parts.iter().any(|part| {
            part.id == id
                && part.properties.as_ref().is_some_and(|properties| {
                    properties.contains_key(MIRRORED_COMPONENT_SUBSTITUTE)
                })
        })
    };
    let recorded = |source_part: &str, mirror_part: &str| {
        doc.layouts.iter().any(|layout| {
            layout.mirror_link.as_ref().is_some_and(|link| {
                link.correspondences.iter().any(|record| {
                    record.kind == MirroredItemKind::Part
                        && record.source_id == source_part
                        && record.mirror_id == mirror_part
                })
            })
        })
    };
    doc.parts
        .iter()
        .filter_map(|part| {
            let source = part
                .properties
                .as_ref()?
                .get(MIRRORED_COMPONENT_SOURCE)?
                .as_str()?;
            Some((source.to_string(), part.id.clone()))
        })
        .filter(|(source_part, mirror_part)| {
            !recorded(source_part, mirror_part)
                && !has_substitute(source_part)
                && !has_substitute(mirror_part)
                && doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == source_id)
                    .is_some_and(|layout| layout.part_ids.contains(source_part))
                && doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == target_id)
                    .is_some_and(|layout| layout.part_ids.contains(mirror_part))
        })
        .collect()
}

/// Clear the local-substitute flag from a correspondence-managed part: the
/// Part correspondence on the mirror link owns its mirroring now.
pub(super) fn strip_substitute_flag(doc: &mut ProjectDoc, part_id: &str) {
    if let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id)
        && let Some(properties) = part.properties.as_mut()
    {
        properties.remove(MIRRORED_COMPONENT_SUBSTITUTE);
        if properties.is_empty() {
            part.properties = None;
        }
    }
}

/// Set the stored Split gap on every link of the pair a layout belongs to:
/// the carrier's own link, or every mirror link pointing at a source layout.
pub fn set_pair_gap(
    doc: &mut ProjectDoc,
    layout_id: &str,
    gap: f64,
) -> Result<Vec<String>, String> {
    let carriers: Vec<String> = match doc.layouts.iter().find(|layout| layout.id == layout_id) {
        Some(layout) if layout.mirror_link.is_some() => vec![layout_id.to_string()],
        Some(_) => doc
            .layouts
            .iter()
            .filter(|layout| {
                layout
                    .mirror_link
                    .as_ref()
                    .is_some_and(|link| link.source_id == layout_id)
            })
            .map(|layout| layout.id.clone())
            .collect(),
        None => vec![],
    };
    if carriers.is_empty() {
        return Err(format!("Layout {layout_id} is not part of a mirrored pair"));
    }
    for id in &carriers {
        if let Some(layout) = doc.layouts.iter_mut().find(|layout| layout.id == *id)
            && let Some(link) = layout.mirror_link.as_mut()
        {
            link.split_gap = Some(gap);
        }
    }
    Ok(carriers)
}

/// Ticket 12: board-space position of the half frame an independent part is
/// placed in: the split axis offset by half the current spacing (the adapted
/// `max(stored gap, minimum)`, or the stored gap before the first adaptive
/// pass). `None` for ungrouped parts, parts on unlinked layouts, matrix
/// members/companions (whose world poses the refresh owns), and legacy pairs
/// without a stored gap, whose parts stay board-coordinate.
pub fn part_half_offset(doc: &ProjectDoc, part_id: &str) -> Option<Vec2> {
    let layout = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))?;
    let offset = layout_half_offset(doc, &layout.id)?;
    // Membership only matters when the owning layout stores a half-frame offset.
    // In a flat layout there is no need to enumerate every matrix's cells for each part.
    (!is_member_or_companion(doc, part_id)).then_some(offset)
}

/// Ticket 12: evaluation offset of one layout in a stored-gap pair: the split
/// axis offset by half the current spacing on the layout's side. `None` for
/// unlinked layouts and legacy pairs without a stored gap.
pub fn layout_half_offset(doc: &ProjectDoc, layout_id: &str) -> Option<Vec2> {
    let layout = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    carrier_offset(doc, layout)
}

/// Whether `part_id` is a matrix member or a generated companion: their world
/// poses belong to the matrix refresh, never to the half frame.
pub fn is_member_or_companion(doc: &ProjectDoc, part_id: &str) -> bool {
    for matrix in &doc.matrices {
        let members = matrix::cell_members(matrix);
        if members.values().any(|member| member == part_id) {
            return true;
        }
        if part_id
            .rsplit_once('/')
            .is_some_and(|(head, _)| members.values().any(|member| member == head))
        {
            return true;
        }
    }
    false
}

/// Ticket 12: evaluated (board-space) pose of a part: the stored half-frame
/// pose plus the half offset on stored-gap pairs; the stored pose everywhere
/// else. Rotations are frame-free, so only the position shifts.
///
/// Ticket 13: an anchored independent part derives its pose from its anchor
/// key instead (the anchor column or key moving moves the part). A part whose
/// anchor no longer resolves holds its stored pose; members and companions
/// never derive.
pub fn evaluated_part_pose(doc: &ProjectDoc, part: &Part) -> Pose2 {
    if let Some(derived) = super::super::part_anchors::evaluate(doc, part) {
        return derived;
    }
    match part_half_offset(doc, &part.id) {
        Some(offset) => Pose2 {
            at: Vec2 {
                x: part.pose.at.x + offset.x,
                y: part.pose.at.y + offset.y,
            },
            rotation: part.pose.rotation,
        },
        None => part.pose,
    }
}

/// Ticket 12: borrow a part viewed at its evaluated board pose. Readers that
/// need the whole [`Part`] (not just its pose) take this instead of cloning:
/// the owned shadow only materializes while a half-frame offset applies.
pub fn evaluated_part<'a>(doc: &ProjectDoc, part: &'a Part) -> Cow<'a, Part> {
    let evaluated = evaluated_part_pose(doc, part);
    if evaluated == part.pose {
        Cow::Borrowed(part)
    } else {
        Cow::Owned(Part {
            pose: evaluated,
            ..part.clone()
        })
    }
}
