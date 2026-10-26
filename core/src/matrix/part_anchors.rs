//! Ticket 13: independent parts placed with the column vocabulary.
//!
//! A part carries an optional [`PartAnchor`]: an anchor key with a Follow
//! subset, Spread/Stagger/Splay carried from that anchor, and Offset/Rotation
//! for the part alone. While anchored, the evaluated pose derives from the
//! anchor key's board-space pose, so editing the anchor column or key moves
//! the part without rewriting its stored values:
//!
//! ```text
//! base = A + rot({spread, stagger}, A.r)
//! pose = step({base.x, base.y, A.r + splay}, offset, rotation)
//! ```
//!
//! where `A` is the anchor key pose filtered by Follow (or the half origin
//! for a detached part, which keeps the stored pose instead). Detached parts
//! keep ticket-12 behavior exactly: the stored half-frame pose plus the half
//! offset.
//!
//! Mirror copies share the anchor through a mapped target: the partner
//! anchors to the counterpart key in its own layout with mirrored values
//! (spread, splay, offset.x and rotation negate; stagger and offset.y stay —
//! the matrix relative-negate rule), so each half derives its mirrored pose
//! from its own anchor. A part whose anchor matrix has no counterpart in the
//! partner layout cannot mirror; naming that key for a mirrored part refuses
//! instead of silently retargeting, and the drop/mirror fallback leaves the
//! partner detached at the reflected pose.
//!
//! Readers must keep using [`evaluated`](super::layout::evaluated_part_pose):
//! the stored pose of an anchored part is only refreshed by part ops, so it
//! goes stale when an anchor column or key moves the part.

use super::splay::rotate;
use super::{evaluation, layout, member_id};
use crate::model::{
    Follow, MirroredItemKind, Part, PartAnchor, PartAnchorValueChange, Pose2, ProjectDoc, Vec2,
};

fn zero() -> Vec2 {
    Vec2::default()
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}

fn transform(pose: Pose2, v: Vec2) -> Vec2 {
    add(pose.at, rotate(v, pose.rotation, zero()))
}

/// The layout homing an independent part, if any.
fn part_layout_id(doc: &ProjectDoc, part_id: &str) -> Option<String> {
    doc.layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == part_id))
        .map(|layout| layout.id.clone())
}

/// The board-space pose of an anchor key. A Local (or explicitly overridden)
/// member holds its final pose while carried fields stay shared, so the part
/// follows that frozen member — it never moves with the gap, which also keeps
/// the spacing search rigid. Otherwise the part follows the evaluated cell,
/// which rides its half through spacing: member stored poses go stale inside
/// the adaptive minimum search, where only the temporary gap moves.
fn anchor_key_pose(doc: &ProjectDoc, anchor: &PartAnchor, layout_id: &str) -> Option<Pose2> {
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == anchor.matrix_id)?;
    let layout = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    if !layout.matrix_ids.contains(&target.id) {
        return None;
    }
    if target.board_id.as_deref() != Some(layout.board_id.as_str()) {
        return None;
    }
    if anchor.row >= target.rows || anchor.column >= target.columns {
        return None;
    }
    let evaluated = evaluation::evaluate(target, doc).ok()?;
    let cell = evaluated
        .cells
        .iter()
        .find(|cell| cell.row == anchor.row as usize && cell.column == anchor.column as usize)?;
    let key_id = member_id(&target.id, anchor.row, anchor.column);
    let frozen = doc
        .parts
        .iter()
        .find(|part| part.id == key_id)
        .filter(|member| {
            layout::exception_state(doc, MirroredItemKind::Part, &member.id)
                == Some(crate::model::MirroredExceptionState::Local)
                || member.properties.as_ref().is_some_and(|properties| {
                    properties.get(super::OVERRIDE) == Some(&serde_json::json!(1))
                })
        });
    Some(match frozen {
        Some(member) => layout::evaluated_part_pose(doc, member),
        None => cell.pose,
    })
}

fn filtered(anchor: &PartAnchor, key: Pose2) -> Pose2 {
    Pose2 {
        at: Vec2 {
            x: if anchor.follow.x { key.at.x } else { 0.0 },
            y: if anchor.follow.y { key.at.y } else { 0.0 },
        },
        rotation: if anchor.follow.rotation {
            key.rotation
        } else {
            0.0
        },
    }
}

/// The derived board-space pose of an anchored part, or `None` when detached
/// (or unresolvable). Members and companions never derive: their poses belong
/// to the matrix refresh.
pub(crate) fn evaluate(doc: &ProjectDoc, part: &Part) -> Option<Pose2> {
    let anchor = part.anchor.as_ref()?;
    if layout::is_member_or_companion(doc, &part.id) {
        return None;
    }
    // A separately driven part keeps its solver-owned pose: constraints win
    // over the anchor, like they win over a companion's attachment.
    if !doc.constraints.is_empty()
        && doc
            .constraint_driving_part(&part.id)
            .ok()
            .flatten()
            .is_some()
    {
        return None;
    }
    let layout_id = part_layout_id(doc, &part.id)?;
    let key = anchor_key_pose(doc, anchor, &layout_id)?;
    let base_pose = filtered(anchor, key);
    let base = add(
        base_pose.at,
        rotate(
            Vec2 {
                x: anchor.spread,
                y: anchor.stagger,
            },
            base_pose.rotation,
            zero(),
        ),
    );
    let frame = Pose2 {
        at: base,
        rotation: base_pose.rotation + anchor.splay,
    };
    Some(Pose2 {
        at: transform(frame, anchor.offset),
        rotation: frame.rotation + anchor.rotation,
    })
}

/// Solve the carried and local values that realize `board` from `anchor`:
/// the anchor-frame offset becomes Spread/Stagger, Splay and Offset reset,
/// and Rotation absorbs the heading difference. The pose-preserving answer
/// for Pick, Follow changes and the drop anchor.
fn solve_for_pose(anchor: &PartAnchor, key: Pose2, board: Pose2) -> PartAnchor {
    let base = filtered(anchor, key);
    let local = rotate(sub(board.at, base.at), -base.rotation, zero());
    PartAnchor {
        matrix_id: anchor.matrix_id.clone(),
        row: anchor.row,
        column: anchor.column,
        follow: anchor.follow,
        spread: local.x,
        stagger: local.y,
        splay: 0.0,
        offset: zero(),
        rotation: board.rotation - base.rotation,
    }
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

fn finite_anchor(anchor: &PartAnchor) -> bool {
    finite(anchor.spread)
        && finite(anchor.stagger)
        && finite(anchor.splay)
        && finite(anchor.offset.x)
        && finite(anchor.offset.y)
        && finite(anchor.rotation)
}

/// Validate an anchor target for an unlocked, undriven independent part homed
/// on a layout, naming a live key in its own layout that no constraint
/// drives. The part need not be in the document yet, so drops validate their
/// preset anchor before insertion. Returns the part's layout id and the
/// anchor key's board-space pose.
pub(crate) fn validate_target(
    doc: &ProjectDoc,
    part: &Part,
    anchor: &PartAnchor,
) -> Result<(String, Pose2), String> {
    let part_id = part.id.as_str();
    if layout::is_member_or_companion(doc, part_id) {
        return Err(format!(
            "Part {part_id} follows its matrix and cannot anchor"
        ));
    }
    if part.locked == Some(true) {
        return Err(format!("Part {part_id} is locked"));
    }
    if doc.constraint_driving_part(part_id)?.is_some() {
        return Err(format!(
            "Part {part_id} is driven by a constraint and cannot anchor"
        ));
    }
    let layout_id = part_layout_id(doc, part_id)
        .ok_or_else(|| format!("Part {part_id} is not on a layout and cannot anchor"))?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == anchor.matrix_id)
        .ok_or_else(|| {
            format!(
                "Part {part_id} anchor target {} is missing",
                anchor.matrix_id
            )
        })?;
    let layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == layout_id)
        .expect("part layout");
    let same_board = target
        .board_id
        .as_deref()
        .is_none_or(|board| board == layout.board_id);
    if !layout.matrix_ids.contains(&target.id) || !same_board {
        return Err(format!(
            "Part {part_id} anchor must stay on the same board and Layout"
        ));
    }
    if anchor.row >= target.rows || anchor.column >= target.columns {
        return Err(format!(
            "Part {part_id} anchor key {} is missing",
            member_id(&target.id, anchor.row, anchor.column)
        ));
    }
    let key_id = member_id(&target.id, anchor.row, anchor.column);
    if doc.constraint_driving_part(&key_id)?.is_some() {
        return Err(format!(
            "Part {part_id} anchor key {key_id} is constraint-driven"
        ));
    }
    let key = anchor_key_pose(doc, anchor, &layout_id)
        .ok_or_else(|| format!("Part {part_id} anchor key {key_id} has no pose"))?;
    Ok((layout_id, key))
}

/// Refresh the stored pose from the evaluated board pose so storage-level
/// readers (clamp sides, mirror reflection, legacy sync) keep working while
/// the anchor owns the placement.
pub(crate) fn refresh_stored_pose(doc: &mut ProjectDoc, part_id: &str) {
    let Some(index) = doc.parts.iter().position(|part| part.id == part_id) else {
        return;
    };
    let evaluated = layout::evaluated_part_pose(doc, &doc.parts[index].clone());
    let stored = layout::stored_part_pose(doc, part_id, evaluated);
    doc.parts[index].pose = stored;
}

/// Mirror-map carried and local values across the split axis. Spread, Splay,
/// Offset.x and Rotation negate while Stagger and Offset.y stay (the matrix
/// relative-negate rule); non-controllers gain the shared mirrored-part half
/// turn so anchored copies agree with ticket-12 reflection. An involution.
fn mirror_values(values: &PartAnchor, is_controller: bool) -> PartAnchor {
    PartAnchor {
        matrix_id: values.matrix_id.clone(),
        row: values.row,
        column: values.column,
        follow: values.follow,
        spread: -values.spread,
        stagger: values.stagger,
        splay: -values.splay,
        offset: Vec2 {
            x: -values.offset.x,
            y: values.offset.y,
        },
        rotation: -values.rotation + if is_controller { 0.0 } else { 180.0 },
    }
}

pub(crate) fn is_controller_for(doc: &ProjectDoc, part: &Part) -> bool {
    doc.definitions.iter().any(|definition| {
        definition.id == part.definition_id && definition.kind == crate::model::PartKind::Controller
    })
}

/// The counterpart of `matrix_id` in `target_layout_id` through the pair's
/// Matrix correspondences: the mirror record when the target is the mirror
/// side, the source record when it is the source side.
pub(crate) fn counterpart_matrix_id(
    doc: &ProjectDoc,
    matrix_id: &str,
    target_layout_id: &str,
) -> Option<String> {
    for layout in &doc.layouts {
        let Some(link) = layout.mirror_link.as_ref() else {
            continue;
        };
        for record in &link.correspondences {
            if record.kind != MirroredItemKind::Matrix {
                continue;
            }
            if layout.id == target_layout_id && record.source_id == matrix_id {
                return Some(record.mirror_id.clone());
            }
            if link.source_id == target_layout_id && record.mirror_id == matrix_id {
                return Some(record.source_id.clone());
            }
        }
    }
    None
}

/// Map an authority (source-layout) anchor onto the partner layout: the
/// target becomes the counterpart matrix and the values mirror. `None` when
/// the anchor matrix has no counterpart there.
pub(crate) fn map_anchor_for_mirror(
    doc: &ProjectDoc,
    anchor: &PartAnchor,
    partner_layout_id: &str,
    is_controller: bool,
) -> Option<PartAnchor> {
    let matrix_id = counterpart_matrix_id(doc, &anchor.matrix_id, partner_layout_id)?;
    let mut mapped = mirror_values(anchor, is_controller);
    mapped.matrix_id = matrix_id;
    Some(mapped)
}

/// Write an anchor for the edited copy and fan it out to its mirror partner:
/// the source-layout copy is the authority, so an edit naming the mirror
/// copy converts through the mirror map first and the partner re-derives from
/// the authority. Refuses when the mapped target is missing instead of
/// silently retargeting.
fn write_anchor(
    doc: &mut ProjectDoc,
    part_id: &str,
    anchor: &PartAnchor,
) -> Result<Vec<String>, String> {
    let Some((_, record)) = layout::part_correspondence(doc, part_id) else {
        let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) else {
            return Err(format!("Unknown part {part_id}"));
        };
        part.anchor = Some(anchor.clone());
        refresh_stored_pose(doc, part_id);
        return Ok(vec![part_id.to_string()]);
    };
    let source_id = record.source_id.clone();
    // The authority anchor always lives in the source layout: an edit naming
    // the mirror copy converts its target and values back first.
    let authority_anchor = if part_id == source_id {
        anchor.clone()
    } else {
        let source_layout = part_layout_id(doc, &source_id)
            .ok_or_else(|| format!("Mirrored part {source_id} is not on a layout"))?;
        let target =
            counterpart_matrix_id(doc, &anchor.matrix_id, &source_layout).ok_or_else(|| {
                format!(
                    "Part {part_id} anchor needs the counterpart of {} in layout {source_layout}; name the missing counterpart instead of silently retargeting",
                    anchor.matrix_id
                )
            })?;
        let mut converted =
            mirror_values(anchor, is_controller_for(doc, &anchor_owner(doc, part_id)?));
        converted.matrix_id = target;
        converted
    };
    write_anchor_source(doc, part_id, &authority_anchor)
}

fn anchor_owner(doc: &ProjectDoc, part_id: &str) -> Result<Part, String> {
    doc.parts
        .iter()
        .find(|part| part.id == part_id)
        .cloned()
        .ok_or_else(|| format!("Unknown part {part_id}"))
}

/// Pick: retarget the anchor key (and Follow subset), re-solving the carried
/// and local values so the evaluated pose is unchanged. One Undo entry.
pub(crate) fn set_anchor(
    doc: &mut ProjectDoc,
    part_id: &str,
    anchor: &PartAnchor,
) -> Result<Vec<String>, String> {
    // Incoming carried/local values are re-solved below, so only the target
    // and Follow subset matter here; values that do not reproduce the pose
    // are never stored.
    let part = anchor_owner(doc, part_id)?;
    let (_, key) = validate_target(doc, &part, anchor)?;
    let current = layout::evaluated_part_pose(doc, &part);
    let solved = solve_for_pose(anchor, key, current);
    write_anchor(doc, part_id, &solved)
}

/// Change the Follow subset, re-solving the carried and local values so the
/// evaluated pose is unchanged. One Undo entry.
pub(crate) fn set_follow(
    doc: &mut ProjectDoc,
    part_id: &str,
    follow: &Follow,
) -> Result<Vec<String>, String> {
    let part = anchor_owner(doc, part_id)?;
    let existing = part
        .anchor
        .as_ref()
        .ok_or_else(|| format!("Part {part_id} has no anchor"))?
        .clone();
    if layout::is_member_or_companion(doc, part_id) {
        return Err(format!(
            "Part {part_id} follows its matrix and cannot anchor"
        ));
    }
    if part.locked == Some(true) {
        return Err(format!("Part {part_id} is locked"));
    }
    if doc.constraint_driving_part(part_id)?.is_some() {
        return Err(format!(
            "Part {part_id} is driven by a constraint and cannot anchor"
        ));
    }
    let layout_id = part_layout_id(doc, part_id)
        .ok_or_else(|| format!("Part {part_id} is not on a layout and cannot anchor"))?;
    let key = anchor_key_pose(doc, &existing, &layout_id).ok_or_else(|| {
        format!(
            "Part {part_id} anchor key {} is missing",
            member_id(&existing.matrix_id, existing.row, existing.column)
        )
    })?;
    let mut anchor = existing;
    anchor.follow = *follow;
    let current = layout::evaluated_part_pose(doc, &part);
    let solved = solve_for_pose(&anchor, key, current);
    write_anchor(doc, part_id, &solved)
}

/// Detach: clear the anchor, keeping the evaluated pose from the half origin.
/// One Undo entry.
pub(crate) fn detach(doc: &mut ProjectDoc, part_id: &str) -> Result<Vec<String>, String> {
    let part = anchor_owner(doc, part_id)?;
    if part.anchor.is_none() {
        return Err(format!("Part {part_id} has no anchor"));
    }
    if part.locked == Some(true) {
        return Err(format!("Part {part_id} is locked"));
    }
    let evaluated = layout::evaluated_part_pose(doc, &part);
    let stored = layout::stored_part_pose(doc, part_id, evaluated);
    let Some(live) = doc.parts.iter_mut().find(|part| part.id == part_id) else {
        return Err(format!("Unknown part {part_id}"));
    };
    live.anchor = None;
    live.pose = stored;
    Ok(vec![part_id.to_string()])
}

/// Edit one carried or local value. Spread/Stagger moves come from drags and
/// the Inspector alike; Splay turns about the carried base, Offset/ Rotation
/// apply for the part alone. One Undo entry.
pub(crate) fn set_value(
    doc: &mut ProjectDoc,
    part_id: &str,
    change: &PartAnchorValueChange,
) -> Result<Vec<String>, String> {
    let part = anchor_owner(doc, part_id)?;
    let existing = part
        .anchor
        .as_ref()
        .ok_or_else(|| format!("Part {part_id} has no anchor"))?
        .clone();
    if layout::is_member_or_companion(doc, part_id) {
        return Err(format!(
            "Part {part_id} follows its matrix and cannot anchor"
        ));
    }
    if part.locked == Some(true) {
        return Err(format!("Part {part_id} is locked"));
    }
    let mut anchor = existing;
    match change {
        PartAnchorValueChange::Spread(value) => anchor.spread = *value,
        PartAnchorValueChange::Stagger(value) => anchor.stagger = *value,
        PartAnchorValueChange::Splay(value) => anchor.splay = *value,
        PartAnchorValueChange::Offset(value) => anchor.offset = *value,
        PartAnchorValueChange::Rotation(value) => anchor.rotation = *value,
    }
    if !finite_anchor(&anchor) {
        return Err("Part anchor values must be finite".into());
    }
    if doc.constraint_driving_part(part_id)?.is_some() {
        return Err(format!(
            "Part {part_id} is driven by a constraint and cannot anchor"
        ));
    }
    // The change names values in the edited copy's frame; write_anchor
    // converts a mirror-side edit onto the authority copy.
    write_anchor(doc, part_id, &anchor)
}

/// Write an authority-framed anchor through the mirror fan-out. `part_id`
/// names either copy; the authority values win on both.
fn write_anchor_source(
    doc: &mut ProjectDoc,
    part_id: &str,
    authority_anchor: &PartAnchor,
) -> Result<Vec<String>, String> {
    let Some((_, record)) = layout::part_correspondence(doc, part_id) else {
        let Some(part) = doc.parts.iter_mut().find(|part| part.id == part_id) else {
            return Err(format!("Unknown part {part_id}"));
        };
        part.anchor = Some(authority_anchor.clone());
        refresh_stored_pose(doc, part_id);
        return Ok(vec![part_id.to_string()]);
    };
    let source_id = record.source_id.clone();
    let mirror_id = record.mirror_id.clone();
    let mirror_layout = part_layout_id(doc, &mirror_id)
        .ok_or_else(|| format!("Mirrored part {mirror_id} is not on a layout"))?;
    let authority = anchor_owner(doc, &source_id)?;
    let partner_anchor = map_anchor_for_mirror(
        doc,
        authority_anchor,
        &mirror_layout,
        is_controller_for(doc, &authority),
    )
    .ok_or_else(|| {
        format!(
            "Part {part_id} anchor needs the counterpart of {} in layout {mirror_layout}; name the missing counterpart instead of silently retargeting",
            authority_anchor.matrix_id
        )
    })?;
    for (id, next) in [
        (&source_id, authority_anchor),
        (&mirror_id, &partner_anchor),
    ] {
        let Some(part) = doc.parts.iter_mut().find(|part| &part.id == id) else {
            return Err(format!("Mirrored part {id} is missing"));
        };
        part.anchor = Some(next.clone());
    }
    refresh_stored_pose(doc, &source_id);
    refresh_stored_pose(doc, &mirror_id);
    let mut changed = vec![source_id, mirror_id];
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// The split axis reflecting `part_id`'s drags: the carrier link's axis for
/// a recorded mirror pair, the target layout link's axis for a legacy
/// property-linked copy.
fn mirror_axis_for(doc: &ProjectDoc, part_id: &str) -> Option<f64> {
    if let Some((carrier_id, _)) = layout::part_correspondence(doc, part_id) {
        return doc
            .layouts
            .iter()
            .find(|layout| layout.id == carrier_id)?
            .mirror_link
            .as_ref()
            .map(|link| link.axis_x);
    }
    let source_id = doc
        .parts
        .iter()
        .find(|part| part.id == part_id)?
        .properties
        .as_ref()?
        .get("boardstudio.mirroredComponentSource")?
        .as_str()?;
    let source_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.part_ids.iter().any(|id| id == source_id))?;
    doc.layouts.iter().find_map(|layout| {
        layout
            .mirror_link
            .as_ref()
            .filter(|link| link.source_id == source_layout.id)
            .map(|link| link.axis_x)
    })
}

/// Drag an anchored part: the clamped board delta re-solves Spread/Stagger in
/// the dragged copy's anchor frame (rotation is untouched by drags), then the
/// mirror fan-out keeps both copies on the same values. A legacy
/// property-linked partner follows through the component sync.
pub(crate) fn move_anchored(
    doc: &mut ProjectDoc,
    part_id: &str,
    board_at: Vec2,
) -> Result<Vec<String>, String> {
    let part = anchor_owner(doc, part_id)?;
    let anchor = part
        .anchor
        .as_ref()
        .ok_or_else(|| format!("Part {part_id} has no anchor"))?
        .clone();
    let layout_id = part_layout_id(doc, part_id)
        .ok_or_else(|| format!("Part {part_id} is not on a layout and cannot anchor"))?;
    let key = anchor_key_pose(doc, &anchor, &layout_id).ok_or_else(|| {
        format!(
            "Part {part_id} anchor key {} is missing",
            member_id(&anchor.matrix_id, anchor.row, anchor.column)
        )
    })?;
    let clamped = layout::clamp_part_to_half(doc, part_id, board_at);
    // Retune a correspondence partner before writing, mirroring the
    // unanchored move arm: the partner's board pose is the split-axis
    // reflection of the clamped drag and its rotation never changes here.
    if let Some((_, record)) = layout::part_correspondence(doc, part_id) {
        let partner_id = if record.source_id == part_id {
            record.mirror_id.clone()
        } else {
            record.source_id.clone()
        };
        let partner_rotation = doc
            .parts
            .iter()
            .find(|part| part.id == partner_id)
            .map(|partner| layout::evaluated_part_pose(doc, partner).rotation)
            .unwrap_or(0.0);
        if let Some(axis_x) = mirror_axis_for(doc, part_id) {
            layout::retune_driven_constraint(
                doc,
                &partner_id,
                Pose2 {
                    at: Vec2 {
                        x: 2.0 * axis_x - clamped.x,
                        y: clamped.y,
                    },
                    rotation: partner_rotation,
                },
                Some(0.0),
            )?;
        }
    }
    let base = filtered(&anchor, key);
    let delta = rotate(sub(clamped, base.at), -base.rotation, zero());
    let mut edited = anchor;
    edited.spread = delta.x;
    edited.stagger = delta.y;
    let mut changed = write_anchor(doc, part_id, &edited)?;
    // Legacy property-linked partners resolve through the component sync,
    // which reflects the clamped drop the dragged copy now realizes.
    if layout::part_correspondence(doc, part_id).is_none()
        && let Some(more) = layout::move_linked_component(doc, part_id, clamped)?
    {
        changed.extend(more);
        changed.sort();
        changed.dedup();
    }
    Ok(changed)
}

/// Anchor a drop to the nearest live key of the drop half's layout, keeping
/// the drop point upright: the drop pose becomes Spread/Stagger in the anchor
/// frame with zero Splay, Offset and a counter-rotation. `None` when the
/// layout names no live key. Members, companions and driven parts never
/// auto-anchor; callers check.
pub(crate) fn drop_anchor(doc: &ProjectDoc, layout_id: &str, board: Pose2) -> Option<PartAnchor> {
    let layout = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    let mut best: Option<(PartAnchor, f64)> = None;
    for matrix_id in &layout.matrix_ids {
        let Some(matrix) = doc.matrices.iter().find(|matrix| matrix.id == *matrix_id) else {
            continue;
        };
        if matrix.board_id.as_deref() != Some(layout.board_id.as_str()) {
            continue;
        }
        let Ok(evaluated) = evaluation::evaluate(matrix, doc) else {
            continue;
        };
        for cell in &evaluated.cells {
            let row = cell.row as u32;
            let column = cell.column as u32;
            if matrix
                .cells
                .iter()
                .any(|entry| entry.row == row && entry.column == column && entry.deleted)
            {
                continue;
            }
            let key_id = member_id(&matrix.id, row, column);
            let pose = doc
                .parts
                .iter()
                .find(|part| part.id == key_id)
                .map(|member| layout::evaluated_part_pose(doc, member))
                .unwrap_or(cell.pose);
            let distance = sub(board.at, pose.at);
            let distance = distance.x.hypot(distance.y);
            if best.as_ref().is_none_or(|(_, known)| distance < *known) {
                let anchor = PartAnchor {
                    matrix_id: matrix.id.clone(),
                    row,
                    column,
                    follow: Follow {
                        x: true,
                        y: true,
                        rotation: true,
                    },
                    spread: 0.0,
                    stagger: 0.0,
                    splay: 0.0,
                    offset: zero(),
                    rotation: 0.0,
                };
                best = Some((solve_for_pose(&anchor, pose, board), distance));
            }
        }
    }
    best.map(|(anchor, _)| anchor)
}
