//! Core-owned single-key anchor transactions: Add, change and Detach.
//!
//! Evaluation order, Follow filtering and pose materialization live in
//! `evaluation`/`kernel`; this module owns the stored graph. Validation refuses
//! self-references, cycles, cross-board/cross-Layout targets, missing keys and
//! constraint-driven anchor keys before any mutation, so a refused edit leaves
//! the document, revision and history unchanged. Removal and compaction keep
//! dependants attached to their physical key or detach them with their frame.

use super::{evaluation, layout, member_id};
use crate::model::*;

fn anchor_key_id(anchor: &MatrixAnchor) -> String {
    member_id(&anchor.matrix_id, anchor.row, anchor.column)
}

fn dependants(doc: &ProjectDoc, target_id: &str) -> Vec<(String, MatrixAnchor)> {
    doc.matrices
        .iter()
        .filter(|matrix| {
            matrix
                .anchor
                .as_ref()
                .is_some_and(|anchor| anchor.matrix_id == target_id)
        })
        .map(|matrix| {
            (
                matrix.id.clone(),
                matrix.anchor.clone().expect("filtered anchor"),
            )
        })
        .collect()
}

fn owning_layout<'a>(doc: &'a ProjectDoc, owner_id: &str) -> Option<&'a Layout> {
    doc.layouts
        .iter()
        .find(|layout| layout.matrix_ids.iter().any(|id| id == owner_id))
}

/// Refuse anything that is not a same-board, same-Layout, live, non-driven key
/// outside the owner's own anchor chain. `owner_layout` is `None` while an Add
/// has not joined a Layout yet; the owner then joins its target's Layout.
fn validate_target(
    doc: &ProjectDoc,
    owner_id: &str,
    owner_layout: Option<&str>,
    anchor: &MatrixAnchor,
) -> Result<(), String> {
    if anchor.matrix_id == owner_id {
        return Err(format!("Matrix {owner_id} cannot anchor to itself"));
    }
    let target = evaluation::logical_matrix(&anchor.matrix_id, doc).map_err(|_| {
        format!(
            "Matrix {owner_id} anchor target {} is missing",
            anchor.matrix_id
        )
    })?;
    match owner_layout {
        Some(layout_id) => {
            let owner_board = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == owner_id)
                .and_then(|matrix| matrix.board_id.clone());
            let same_layout = doc
                .layouts
                .iter()
                .find(|layout| layout.id == layout_id)
                .is_some_and(|layout| layout.matrix_ids.contains(&target.id));
            if !same_layout || target.board_id != owner_board {
                return Err(format!(
                    "Matrix {owner_id} anchor must stay on the same board and Layout"
                ));
            }
        }
        None => {
            if owning_layout(doc, &target.id).is_none() {
                return Err(format!(
                    "Anchored matrix {owner_id} must belong to a Layout"
                ));
            }
        }
    }
    if anchor.row >= target.rows || anchor.column >= target.columns {
        return Err(format!(
            "Matrix {owner_id} anchor key {} is missing",
            anchor_key_id(anchor)
        ));
    }
    let key_id = anchor_key_id(anchor);
    if doc.constraint_driving_part(&key_id)?.is_some() {
        return Err(format!(
            "Matrix {owner_id} anchor key {key_id} is constraint-driven"
        ));
    }
    let mut cursor = anchor.matrix_id.clone();
    for _ in 0..doc.matrices.len() + 1 {
        if cursor == owner_id {
            return Err(format!("Matrix anchor cycle at {owner_id}"));
        }
        let Some(next) = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == cursor)
            .and_then(|matrix| matrix.anchor.as_ref())
        else {
            break;
        };
        cursor = next.matrix_id.clone();
    }
    Ok(())
}

/// All layouts in one same-board linked group: the source plus every mirror
/// that points at it. The authored layout is one of them.
fn linked_group(doc: &ProjectDoc, layout_id: &str) -> Option<(String, Vec<String>)> {
    let authored = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    let source_id = authored
        .mirror_link
        .as_ref()
        .map(|link| link.source_id.clone())
        .unwrap_or_else(|| layout_id.to_string());
    if !doc.layouts.iter().any(|layout| {
        layout.mirror_link
            .as_ref()
            .is_some_and(|link| link.source_id == source_id)
    }) {
        return None;
    }
    let mut mirrors: Vec<String> = doc
        .layouts
        .iter()
        .filter(|layout| {
            layout
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.source_id == source_id)
        })
        .map(|layout| layout.id.clone())
        .collect();
    mirrors.sort();
    Some((source_id, mirrors))
}

/// The matrix in `to_layout` that corresponds to `matrix_id` in the authored
/// layout, travelling via the source when the two are sibling mirrors.
fn counterpart_in_layout(
    doc: &ProjectDoc,
    matrix_id: &str,
    authored_layout_id: &str,
    to_layout_id: &str,
    source_id: &str,
) -> Result<String, String> {
    if authored_layout_id == to_layout_id {
        return Ok(matrix_id.to_string());
    }
    let via_source = |layout_id: &str, matrix: &str| -> Option<String> {
        doc.layouts.iter().find_map(|layout| {
            if layout.id != layout_id {
                return None;
            }
            let link = layout.mirror_link.as_ref()?;
            link.correspondences.iter().find_map(|record| {
                if record.kind != MirroredItemKind::Matrix {
                    return None;
                }
                if layout_id == source_id {
                    // `layout_id` is the source: `matrix` is a source id.
                    (record.source_id == matrix).then(|| record.mirror_id.clone())
                } else {
                    // `layout_id` is a mirror: `matrix` is a mirror id.
                    (record.mirror_id == matrix).then(|| record.source_id.clone())
                }
            })
        })
    };
    // Authored -> source (unless authored is already the source).
    let in_source = if authored_layout_id == source_id {
        matrix_id.to_string()
    } else {
        via_source(authored_layout_id, matrix_id).ok_or_else(|| {
            format!(
                "Linked counterpart of matrix {matrix_id:?} is missing in source layout {source_id:?}; name the missing counterpart instead of silently retargeting"
            )
        })?
    };
    // Source -> target (unless target is the source).
    if to_layout_id == source_id {
        return Ok(in_source);
    }
    doc.layouts
        .iter()
        .find(|layout| layout.id == to_layout_id)
        .and_then(|layout| layout.mirror_link.as_ref())
        .and_then(|link| {
            link.correspondences.iter().find_map(|record| {
                (record.kind == MirroredItemKind::Matrix && record.source_id == in_source)
                    .then(|| record.mirror_id.clone())
            })
        })
        .ok_or_else(|| {
            format!(
                "Linked counterpart of matrix {matrix_id:?} is missing in layout {to_layout_id:?}; name the missing counterpart instead of silently retargeting"
            )
        })
}

/// Create a 1x3 matrix in the anchor target's Layout and board at relative
/// Origin (0, -pitch.y) with every Follow component, then materialize it.
pub(crate) fn add(
    doc: &mut ProjectDoc,
    id: &str,
    anchor: &MatrixAnchor,
) -> Result<Vec<String>, String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("Matrix ID must use letters, digits, _ or -".into());
    }
    if doc.matrices.iter().any(|matrix| matrix.id == id) {
        return Err(format!("Duplicate matrix identity {id:?}"));
    }
    validate_target(doc, id, None, anchor)?;
    let target = evaluation::logical_matrix(&anchor.matrix_id, doc)
        .map_err(|_| format!("Matrix {id} anchor target {} is missing", anchor.matrix_id))?;
    let layout_id = owning_layout(doc, &target.id)
        .expect("validated target layout")
        .id
        .clone();
    // Ticket 20: a linked group creates the mirrored counterpart in the same
    // edit. New local records copy unchanged; anchored relative Origin X and
    // Rotation negate while Y stays, via the stored relative values below.
    if let Some((source_id, mirrors)) = linked_group(doc, &layout_id) {
        let mut group_layouts = vec![source_id.clone()];
        group_layouts.extend(mirrors.clone());
        group_layouts.sort();
        group_layouts.dedup();
        // Map the anchor target into every member layout before mutating.
        let mut anchors_by_layout = std::collections::BTreeMap::new();
        for member in &group_layouts {
            let mapped = counterpart_in_layout(
                doc,
                &anchor.matrix_id,
                &layout_id,
                member,
                &source_id,
            )?;
            anchors_by_layout.insert(
                member.clone(),
                MatrixAnchor {
                    matrix_id: mapped,
                    row: anchor.row,
                    column: anchor.column,
                    follow: anchor.follow,
                },
            );
        }
        // Reserve every counterpart identity before any mutation so a clash
        // leaves the document unchanged.
        let mut ids_by_layout = std::collections::BTreeMap::new();
        // The authored layout keeps the requested id; mirrors take a derived
        // `-mirror` identity with numeric dedup.
        ids_by_layout.insert(layout_id.clone(), id.to_string());
        for member in &group_layouts {
            if member == &layout_id {
                continue;
            }
            let base = format!("{id}-mirror");
            let mut candidate = base.clone();
            let mut index = 2;
            while doc.matrices.iter().any(|matrix| matrix.id == candidate)
                || ids_by_layout.values().any(|taken| taken == &candidate)
            {
                candidate = format!("{base}-{index}");
                index += 1;
            }
            ids_by_layout.insert(member.clone(), candidate);
        }
        for taken in ids_by_layout.values() {
            if doc.matrices.iter().any(|matrix| &matrix.id == taken) {
                return Err(format!("Duplicate matrix identity {taken:?}"));
            }
        }
        // Validate every counterpart anchor while the document is unchanged.
        // New identities join their anchor target's Layout, so validate as an
        // Add (no owner layout yet) rather than as a retarget of an existing
        // matrix whose board lookup would be empty.
        for member in &group_layouts {
            let counterpart_id = &ids_by_layout[member];
            let counterpart_anchor = &anchors_by_layout[member];
            validate_target(doc, counterpart_id, None, counterpart_anchor)?;
            // The mapped anchor must live in the same member layout the new
            // matrix joins; otherwise the group would silently retarget.
            let anchor_layout = owning_layout(doc, &counterpart_anchor.matrix_id)
                .map(|layout| layout.id.clone());
            if anchor_layout.as_deref() != Some(member.as_str()) {
                return Err(format!(
                    "Matrix {counterpart_id} anchor must stay on the same board and Layout"
                ));
            }
        }
        let pitch = target.pitch;
        let definition_id = target.definition_id.clone();
        let board_id = target.board_id.clone();
        let edge_gap = target.edge_gap;
        let diode_direction = target.diode_direction;
        for member in &group_layouts {
            let counterpart_id = ids_by_layout[member].clone();
            let is_source = member == &source_id;
            doc.layouts
                .iter_mut()
                .find(|layout| &layout.id == member)
                .expect("linked group layout")
                .matrix_ids
                .push(counterpart_id.clone());
            // Record the matrix correspondence on the mirror link.
            if !is_source {
                let source_matrix_id = ids_by_layout[&source_id].clone();
                if let Some(layout) = doc
                    .layouts
                    .iter_mut()
                    .find(|layout| &layout.id == member)
                    && let Some(link) = layout.mirror_link.as_mut()
                {
                    if !link.correspondences.iter().any(|record| {
                        record.kind == MirroredItemKind::Matrix
                            && record.source_id == source_matrix_id
                            && record.mirror_id == counterpart_id
                    }) {
                        link.correspondences.push(MirroredCorrespondence {
                            kind: MirroredItemKind::Matrix,
                            source_id: source_matrix_id,
                            mirror_id: counterpart_id.clone(),
                            companion_mode: None,
                            companion_tweak: None,
                        });
                    }
                }
            }
        }
        // Materialize every new matrix, then sync pairs and member pairs.
        // Collect the structs first so each `set_matrix` sees all identities.
        let mut pending: Vec<Matrix> = Vec::new();
        for member in &group_layouts {
            let counterpart_id = ids_by_layout[member].clone();
            let is_source = member == &source_id;
            pending.push(Matrix {
                id: counterpart_id,
                name: None,
                rows: 1,
                columns: 3,
                pitch,
                origin: Vec2 {
                    x: 0.0,
                    y: -pitch.y,
                },
                definition_id: definition_id.clone(),
                part_ids: vec![],
                board_id: board_id.clone(),
                mirror: if is_source { None } else { Some(Mirror::X) },
                rotation: None,
                edge_gap,
                diode_direction,
                row_layout: vec![],
                column_layout: vec![],
                anchor: Some(anchors_by_layout[member].clone()),
                cells: vec![],
            });
        }
        let mut changed = Vec::new();
        for matrix in &pending {
            changed.extend(super::set_matrix(doc, matrix)?);
        }
        for matrix in &pending {
            changed.extend(layout::sync(doc, &matrix.id)?);
        }
        for matrix in &pending {
            layout::sync_member_pairs(doc, &matrix.id)?;
        }
        changed.sort();
        changed.dedup();
        return Ok(changed);
    }
    let matrix = Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: target.pitch,
        origin: Vec2 {
            x: 0.0,
            y: -target.pitch.y,
        },
        definition_id: target.definition_id.clone(),
        part_ids: vec![],
        board_id: target.board_id.clone(),
        mirror: None,
        rotation: None,
        edge_gap: target.edge_gap,
        diode_direction: target.diode_direction,
        row_layout: vec![],
        column_layout: vec![],
        anchor: Some(anchor.clone()),
        cells: vec![],
    };
    doc.layouts
        .iter_mut()
        .find(|layout| layout.id == layout_id)
        .expect("validated layout")
        .matrix_ids
        .push(id.into());
    let mut changed = super::set_matrix(doc, &matrix)?;
    changed.extend(layout::sync(doc, id)?);
    layout::sync_member_pairs(doc, id)?;
    Ok(changed)
}

/// Re-target one matrix's anchor and Follow subset atomically.
pub(crate) fn set_anchor(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    anchor: &MatrixAnchor,
) -> Result<Vec<String>, String> {
    let current = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Matrix not found")?;
    if current.anchor.as_ref() == Some(anchor) {
        return Ok(vec![]);
    }
    let layout_id = owning_layout(doc, matrix_id)
        .ok_or_else(|| format!("Anchored matrix {matrix_id} must belong to a Layout"))?
        .id
        .clone();
    validate_target(doc, matrix_id, Some(&layout_id), anchor)?;
    let mut next = current.clone();
    next.anchor = Some(anchor.clone());
    let mut changed = super::set_matrix(doc, &next)?;
    changed.extend(layout::sync(doc, matrix_id)?);
    layout::sync_member_pairs(doc, matrix_id)?;
    Ok(changed)
}

/// Clear one matrix's anchor after writing its current effective world frame to
/// Origin/Rotation, so every pose is unchanged. A linked partner follows
/// through the normal pair sync in the same edit; a sync that cannot keep both
/// poses is refused and names the Local exception instead of changing one side.
pub(crate) fn detach(doc: &mut ProjectDoc, matrix_id: &str) -> Result<Vec<String>, String> {
    let current = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Matrix not found")?
        .clone();
    if current.anchor.is_none() {
        return Err(format!("Matrix {matrix_id} has no anchor to detach"));
    }
    let frame = evaluation::evaluate(&current, doc)
        .map_err(|message| message.to_string())?
        .frame;
    let mut next = current.clone();
    next.anchor = None;
    next.origin = frame.origin;
    next.rotation = if frame.rotation == 0.0 {
        None
    } else {
        Some(frame.rotation)
    };
    let mut changed = super::set_matrix(doc, &next)?;
    changed.extend(
        layout::sync(doc, matrix_id).map_err(|message| {
            format!("{message}; detaching a linked group that cannot keep both poses needs an explicit Local exception (ticket 21)")
        })?,
    );
    layout::sync_member_pairs(doc, matrix_id)?;
    Ok(changed)
}

/// Detach every matrix anchored to `target_id`, keeping each current world
/// frame. Call before the target matrix disappears.
pub(crate) fn detach_dependants(
    doc: &mut ProjectDoc,
    target_id: &str,
) -> Result<Vec<String>, String> {
    let mut changed = vec![];
    for (id, _) in dependants(doc, target_id) {
        changed.extend(detach(doc, &id)?);
    }
    Ok(changed)
}

/// Detach dependants whose anchor slot leaves the grid. A wholesale replacement
/// never retargets an anchor onto a new key at an old coordinate.
pub(crate) fn detach_out_of_range(
    doc: &mut ProjectDoc,
    matrix_id: &str,
    rows: u32,
    columns: u32,
) -> Result<Vec<String>, String> {
    let mut changed = vec![];
    for (id, anchor) in dependants(doc, matrix_id) {
        if anchor.row >= rows || anchor.column >= columns {
            changed.extend(detach(doc, &id)?);
        }
    }
    Ok(changed)
}

/// Remap anchors into a compacted matrix: the removed slot detaches with its
/// frame first, then later slots follow their surviving physical key.
pub(crate) fn remap_compacted(
    doc: &mut ProjectDoc,
    matrix_ids: &[String],
    index: u32,
    row_axis: bool,
) -> Result<Vec<String>, String> {
    let mut changed = vec![];
    for target in matrix_ids {
        for (id, anchor) in dependants(doc, target) {
            let axis = if row_axis { anchor.row } else { anchor.column };
            if axis == index {
                changed.extend(detach(doc, &id)?);
            }
        }
    }
    Ok(changed)
}

/// Shift stored anchors past a removed axis onto the surviving key, then
/// refresh their poses. Call after the axis removal is stored.
pub(crate) fn shift_compacted(
    doc: &mut ProjectDoc,
    matrix_ids: &[String],
    index: u32,
    row_axis: bool,
) -> Result<Vec<String>, String> {
    let mut changed = vec![];
    let mut moved: Vec<(String, MatrixAnchor)> = vec![];
    for target in matrix_ids {
        for (id, mut anchor) in dependants(doc, target) {
            let axis = if row_axis {
                &mut anchor.row
            } else {
                &mut anchor.column
            };
            if *axis > index {
                *axis -= 1;
                moved.push((id, anchor));
            }
        }
    }
    for (id, anchor) in moved {
        let mut next = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == id)
            .ok_or("Anchored matrix is missing")?
            .clone();
        next.anchor = Some(anchor);
        changed.extend(super::set_matrix(doc, &next)?);
        changed.extend(layout::sync(doc, &id)?);
    }
    Ok(changed)
}

/// Refuse a constraint that would drive a key a matrix anchors to.
pub(crate) fn constraining_refusal(doc: &ProjectDoc, constraint: &Constraint) -> Option<String> {
    let target = constraint.target();
    for matrix in &doc.matrices {
        if let Some(anchor) = &matrix.anchor
            && anchor_key_id(anchor) == target
        {
            return Some(format!(
                "Cannot constrain {target} because matrix {} anchors to it; detach the matrix first",
                matrix.id
            ));
        }
    }
    None
}

/// Refuse a stored Origin/Rotation change on an anchored matrix that its Follow
/// subset cannot express. The evaluated frame would be identical, so accepting
/// would silently keep geometry while recording a new value.
pub(crate) fn check_anchored_frame_change(
    doc: &ProjectDoc,
    previous: &Matrix,
    incoming: &Matrix,
) -> Result<(), String> {
    let Some(anchor) = &incoming.anchor else {
        return Ok(());
    };
    if previous.anchor.as_ref() != Some(anchor) {
        return Ok(());
    }
    if previous.origin == incoming.origin && previous.rotation == incoming.rotation {
        return Ok(());
    }
    let before = evaluation::evaluate(previous, doc).map_err(|message| message.to_string())?;
    let after = evaluation::evaluate(incoming, doc).map_err(|message| message.to_string())?;
    if before.frame == after.frame {
        return Err(
            "Matrix Origin/Rotation is not followed; enable the missing Follow component or Detach first".into(),
        );
    }
    Ok(())
}
