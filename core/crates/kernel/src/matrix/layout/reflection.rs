//! Reflection of companions and the pair sync/resolve pass.

use super::{axis_mapping, companion_mode, pair_gap, partner, sync_components, validate};
use crate::{matrix, model::*};
use boardstudio_model::mechanical::pair;
use std::collections::BTreeMap;

/// A companion's offset and turn are relative to its part, so the reflection only flips their
/// handedness. The pair rotation is carried once, by the part's own heading.
pub(super) fn reflected_companion(mut assembly: MatrixAssembly) -> MatrixAssembly {
    assembly.offset.x = -assembly.offset.x;
    assembly.rotation = assembly.rotation.map(|rotation| -rotation);
    assembly
}

/// Reflects `source` through a pair's `mapping` onto `target`, which keeps its identity, board and
/// part ids. Layout records and all key values copy unchanged; the kernel's local mirror
/// reflects the whole local heading once. Companions retain their separate part-local
/// placement rule. Anchored Origin/Rotation are relative, not reflected world points.
///
/// With `half_frame`, both origins are already half-local: the mirror keeps the
/// negated half-frame Origin/Rotation instead of a board-space reflection, so a
/// later Split gap change moves the evaluated halves without rewriting storage.
pub fn reflected(
    source: &Matrix,
    target: &Matrix,
    mapping: Pose2,
    local_companions: bool,
    half_frame: bool,
) -> Result<Matrix, String> {
    // The pair uses a vertical reflection of the local None/X frames.
    // Switching to a horizontal local frame requires an independent layout.
    if source.mirror == Some(Mirror::Y) {
        return Err("Unlink the halves before changing to a horizontal matrix mirror".into());
    }
    let mut result = target.clone();
    result.rows = source.rows;
    result.columns = source.columns;
    result.pitch = source.pitch;
    result.edge_gap = source.edge_gap;
    if half_frame || source.anchor.is_some() {
        // Half-frame and anchored origins share one rule: relative X negates.
        result.origin = Vec2 {
            x: -source.origin.x,
            y: source.origin.y,
        };
        result.rotation = source.rotation.map(|rotation| -rotation);
    } else {
        result.origin = pair::forward(mapping, source.origin);
        result.rotation = if source.rotation.is_some() || mapping.rotation != 0.0 {
            Some(pair::forward_angle(mapping, source.rotation.unwrap_or(0.0)))
        } else {
            None
        };
    }
    result.mirror = Some(if source.mirror == Some(Mirror::X) {
        Mirror::None
    } else {
        Mirror::X
    });
    result.diode_direction = source.diode_direction;
    result.row_layout = source.row_layout.clone();
    result.column_layout = source.column_layout.clone();
    result.anchor = source.anchor.clone();
    if let Some(anchor) = &mut result.anchor {
        // Relative Origin X and Rotation negate; Y stays. When the receiver
        // already carries a mapped anchor (Omitted logical matrices), keep
        // it; `sync` remaps via correspondences immediately after and
        // overwrites any stale identity there.
        if let Some(mapped) = target.anchor.as_ref().map(|a| a.matrix_id.clone()) {
            anchor.matrix_id = mapped;
        }
        result.origin = Vec2 {
            x: -source.origin.x,
            y: source.origin.y,
        };
        result.rotation = source.rotation.map(|r| -r);
    }
    let source_cells: BTreeMap<_, _> = source
        .cells
        .iter()
        .map(|cell| ((cell.row, cell.column), cell))
        .collect();
    let mut cells: BTreeMap<_, _> = target
        .cells
        .iter()
        .filter(|cell| cell.row < source.rows && cell.column < source.columns)
        .map(|cell| ((cell.row, cell.column), cell.clone()))
        .collect();
    let local_preset = target.cells.iter().find(|cell| {
        cell.assemblies_local == Some(true)
            && cell.definition_id.as_deref() == Some(target.definition_id.as_str())
            && cell
                .variant
                .as_deref()
                .is_some_and(|variant| variant.starts_with("preset/"))
    });
    for (coordinate, source_cell) in &source_cells {
        cells.entry(*coordinate).or_insert_with(|| {
            // New keys inherit this half's saved hardware, not the other half's
            // local companion assignments. Existing per-key overrides stay intact.
            if (source_cell.row >= target.rows || source_cell.column >= target.columns)
                && let Some(template) = local_preset
            {
                return MatrixCell {
                    row: source_cell.row,
                    column: source_cell.column,
                    enabled: true,
                    offset: None,
                    rotation: None,
                    ..template.clone()
                };
            }
            MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: source_cell.row,
                column: source_cell.column,
                enabled: true,

                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }
        });
    }
    for (coordinate, cell) in &mut cells {
        let source_cell = source_cells.get(coordinate);
        cell.enabled = source_cell.is_none_or(|cell| cell.enabled);
        cell.deleted = source_cell.is_some_and(|cell| cell.deleted);
        cell.offset = source_cell.and_then(|cell| cell.offset);
        // Cell turns and companion offsets are relative to the matrix frame and to the part, which
        // already carry the pair rotation, so only their handedness flips here.
        cell.rotation = source_cell.and_then(|cell| cell.rotation);
        cell.shift = source_cell.and_then(|cell| cell.shift);
        cell.bend = source_cell.and_then(|cell| cell.bend);
        // A sync keeps the other half's local companions; a new half copies them, since it has none.
        if cell.assemblies_local != Some(true)
            && (local_companions
                || !source_cell.is_some_and(|cell| cell.assemblies_local == Some(true)))
        {
            cell.definition_id = source_cell.and_then(|cell| cell.definition_id.clone());
            cell.variant = source_cell.and_then(|cell| cell.variant.clone());
            cell.assemblies = source_cell
                .map(|cell| {
                    cell.assemblies
                        .iter()
                        .cloned()
                        .map(reflected_companion)
                        .collect()
                })
                .unwrap_or_default();
            cell.assemblies_local = if local_companions {
                source_cell.and_then(|cell| cell.assemblies_local)
            } else {
                None
            };
        }
    }
    result.cells = cells.into_values().collect();
    Ok(result)
}

/// Ticket 31: remap a whole anchor through matrix correspondences without
/// silently retargeting: the single key, or every aggregate part by identity
/// (row/column stay the physical key). Method and Follow cross unchanged; the
/// relative-negate rules apply to the resolved aggregate pose, never per
/// part. A missing counterpart for any part refuses naming the layout.
pub(super) fn remap_anchor(
    correspondences: &[MirroredCorrespondence],
    anchor: &MatrixAnchor,
    to_mirror: bool,
    layout_id: &str,
) -> Result<MatrixAnchor, String> {
    let mut next = anchor.clone();
    if let Some(aggregate) = &anchor.aggregate {
        let mut parts = Vec::with_capacity(aggregate.parts.len());
        for part in &aggregate.parts {
            parts.push(AnchorPart {
                matrix_id: remap_anchor_target(
                    correspondences,
                    &part.matrix_id,
                    to_mirror,
                    layout_id,
                )?,
                row: part.row,
                column: part.column,
            });
        }
        next.aggregate = Some(AggregateAnchor {
            method: aggregate.method,
            parts,
        });
    } else {
        next.matrix_id =
            remap_anchor_target(correspondences, &anchor.matrix_id, to_mirror, layout_id)?;
    }
    Ok(next)
}

/// Ticket 21: remap an anchor target through matrix correspondences without
/// silently retargeting. `to_mirror` selects source→mirror (true) or
/// mirror→source (false); a missing counterpart refuses naming the layout.
pub(super) fn remap_anchor_target(
    correspondences: &[MirroredCorrespondence],
    anchor_matrix_id: &str,
    to_mirror: bool,
    layout_id: &str,
) -> Result<String, String> {
    correspondences
        .iter()
        .find(|record| {
            record.kind == MirroredItemKind::Matrix
                && (if to_mirror {
                    record.source_id == anchor_matrix_id
                } else {
                    record.mirror_id == anchor_matrix_id
                })
        })
        .map(|record| {
            if to_mirror {
                record.mirror_id.clone()
            } else {
                record.source_id.clone()
            }
        })
        .ok_or_else(|| {
            format!(
                "Linked counterpart of anchor target {:?} is missing in layout {:?}; name the missing counterpart instead of silently retargeting",
                anchor_matrix_id, layout_id
            )
        })
}

pub fn sync(doc: &mut ProjectDoc, matrix_id: &str) -> Result<Vec<String>, String> {
    let Some((target_id, axis_x)) = partner(doc, matrix_id) else {
        return Ok(vec![]);
    };
    // Ticket 21: an independent (Local) or absent (Omitted) edited matrix
    // never authors its partner; its own record already holds its state.
    // A Local partner keeps its own dimensions/layout on source resize.
    // An Omitted partner stays live instead: its record follows the source
    // (no parts are ever materialized for it) so anchors through it keep a
    // live logical pose. Neither state blocks the source edit.
    if matches!(
        exception_state(doc, MirroredItemKind::Matrix, matrix_id),
        Some(MirroredExceptionState::Local | MirroredExceptionState::Omitted)
    ) {
        return Ok(vec![]);
    }
    if exception_state(doc, MirroredItemKind::Matrix, &target_id)
        == Some(MirroredExceptionState::Local)
    {
        return Ok(vec![]);
    }
    let target_omitted = exception_state(doc, MirroredItemKind::Matrix, &target_id)
        == Some(MirroredExceptionState::Omitted);
    // Ticket 21: a record-absent Omitted member stays absent; its logical
    // matrix synthesizes from the source during evaluation.
    if target_omitted && !doc.matrices.iter().any(|matrix| matrix.id == target_id) {
        return Ok(vec![]);
    }
    let source = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .ok_or("Linked matrix is missing")?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == target_id)
        .ok_or("Linked matrix is missing")?;
    let source_is_target = doc
        .layouts
        .iter()
        .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .is_some_and(|layout| layout.mirror_link.is_some());
    let sizes: BTreeMap<_, _> = matrix::cell_members(source)
        .into_iter()
        .filter_map(|(cell, id)| {
            doc.parts
                .iter()
                .find(|part| part.id == id)
                .and_then(|part| part.keycap)
                .map(|size| (cell, size))
        })
        .collect();
    let mapping = axis_mapping(axis_x);
    let half_frame = pair_gap(doc, matrix_id).is_some();
    let mut reflected_matrix = reflected(source, target, mapping, false, half_frame)?;
    // Ticket 20: map anchor targets by identity, not list position or the
    // counterpart's stale anchor. Row/column stay the physical key; Follow
    // keeps its subset with source-world transport in the kernel.
    // Ticket 31: aggregate parts map one by one through the same
    // correspondence; the mirror resolves the whole candidate, never a
    // per-part reflection.
    // Ticket 21: an Omitted partner follows source re-anchors when the
    // counterpart exists, keeps its last valid anchor when it does not, and
    // never blocks the source edit while absent; Rejoin restores the
    // current source anchor anyway. `reflected` already preserved the old
    // anchor identity; only a successful remap overwrites it here.
    if let Some(anchor) = &source.anchor {
        let edited_layout = doc
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.iter().any(|id| id == &source.id))
            .map(|layout| layout.id.clone());
        let target_layout = doc
            .layouts
            .iter()
            .find(|layout| layout.matrix_ids.iter().any(|id| id == &target.id))
            .map(|layout| layout.id.clone());
        if let (Some(edited_layout), Some(target_layout)) = (edited_layout, target_layout) {
            let mut mapped: Option<MatrixAnchor> = None;
            for layout in &doc.layouts {
                let Some(link) = &layout.mirror_link else {
                    continue;
                };
                let edited_is_source =
                    link.source_id == edited_layout && layout.id == target_layout;
                let edited_is_mirror =
                    layout.id == edited_layout && link.source_id == target_layout;
                if !edited_is_source && !edited_is_mirror {
                    continue;
                }
                // Forward wins with a reverse fallback (a self-mirror cannot
                // pass `validate`); the last matching layout still wins.
                if edited_is_source {
                    if let Ok(next) =
                        remap_anchor(&link.correspondences, anchor, true, &target_layout)
                    {
                        mapped = Some(next);
                        continue;
                    }
                    if !edited_is_mirror {
                        continue;
                    }
                }
                if edited_is_mirror
                    && let Ok(next) =
                        remap_anchor(&link.correspondences, anchor, false, &target_layout)
                {
                    mapped = Some(next);
                }
            }
            if let Some(mapped) = mapped {
                if let Some(result_anchor) = reflected_matrix.anchor.as_mut() {
                    result_anchor.matrix_id = mapped.matrix_id;
                    result_anchor.row = mapped.row;
                    result_anchor.column = mapped.column;
                    result_anchor.aggregate = mapped.aggregate;
                }
            } else if target_omitted {
                // No counterpart while absent: keep the last valid anchor
                // (or none) instead of a cross-layout pointer. Rejoin
                // restores the current source anchor and revalidates.
                reflected_matrix.anchor = target.anchor.clone();
            } else {
                return Err(format!(
                    "Linked counterpart of anchor target {:?} is missing in layout {:?}; name the missing counterpart instead of silently retargeting",
                    anchor.matrix_id, target_layout
                ));
            }
        }
    }
    if target_omitted {
        // An absent record must never carry an anchor it cannot resolve:
        // compaction may have diverged its slots from the source side.
        // Keep the newest valid anchor (or none); Rejoin restores the
        // current source anchor and revalidates.
        let valid = |anchor: &MatrixAnchor| {
            matrix::evaluation::logical_matrix(&anchor.matrix_id, doc)
                .ok()
                .is_some_and(|target| anchor.row < target.rows && anchor.column < target.columns)
        };
        if !reflected_matrix.anchor.as_ref().is_some_and(&valid) {
            reflected_matrix.anchor = target.anchor.clone().filter(|anchor| valid(anchor));
        }
    }
    // Ticket 20: Local final-pose edits remain local. Independent key
    // offset/Key rotation stay on their own half; carried Shift/Bend and
    // layout records still sync. Preserve either side's Local slot, defaulting
    // to no independent pose when the counterpart has no explicit record.
    {
        let target_cells: BTreeMap<_, _> = target
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut reflected_matrix.cells {
            let source_part = crate::matrix::member_id(&source.id, cell.row, cell.column);
            let target_part = crate::matrix::member_id(&target.id, cell.row, cell.column);
            let local = exception_state(doc, MirroredItemKind::Part, &source_part)
                == Some(MirroredExceptionState::Local)
                || exception_state(doc, MirroredItemKind::Part, &target_part)
                    == Some(MirroredExceptionState::Local);
            if local {
                if let Some(previous) = target_cells.get(&(cell.row, cell.column)) {
                    cell.offset = previous.offset;
                    cell.rotation = previous.rotation;
                } else {
                    cell.offset = None;
                    cell.rotation = None;
                }
            }
        }
    }
    if source_is_target {
        let canonical_cells: BTreeMap<_, _> = target
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut reflected_matrix.cells {
            if let Some(canonical) = canonical_cells.get(&(cell.row, cell.column))
                && cell.assemblies_local != Some(true)
            {
                cell.definition_id = canonical.definition_id.clone();
                cell.variant = canonical.variant.clone();
                cell.assemblies = canonical.assemblies.clone();
                cell.assemblies_local = canonical.assemblies_local;
            }
        }
    }
    let mut changed = matrix::set_matrix(doc, &reflected_matrix)?;
    let target = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == target_id)
        .ok_or("Linked matrix is missing")?;
    for (cell, id) in matrix::cell_members(target) {
        if let Some(size) = sizes.get(&cell)
            && let Some(part) = doc.parts.iter_mut().find(|part| part.id == id)
        {
            part.keycap = Some(*size);
        }
    }
    if source_is_target {
        let canonical = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == target_id)
            .ok_or("Linked matrix is missing")?;
        let mut local = doc
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .ok_or("Linked matrix is missing")?
            .clone();
        let canonical_cells: BTreeMap<_, _> = canonical
            .cells
            .iter()
            .map(|cell| ((cell.row, cell.column), cell))
            .collect();
        for cell in &mut local.cells {
            if cell.assemblies_local == Some(true) {
                continue;
            }
            if let Some(canonical) = canonical_cells.get(&(cell.row, cell.column)) {
                cell.assemblies = canonical
                    .assemblies
                    .iter()
                    .cloned()
                    .map(reflected_companion)
                    .collect();
            } else {
                cell.assemblies.clear();
            }
            cell.assemblies_local = None;
        }
        changed.extend(matrix::set_matrix(doc, &local)?);
    }
    Ok(changed)
}

pub fn resolve(doc: &mut ProjectDoc) -> Result<(), String> {
    validate(doc)?;
    for matrix in &doc.matrices {
        matrix::valid_projection_matrix(matrix)?;
        // World evaluation repeats the validated projection unless the document
        // contributes an anchor context or a half-frame offset.
        if matrix.anchor.is_some() || super::half_offset(doc, &matrix.id).is_some() {
            matrix::evaluated(matrix, doc)?;
        }
    }
    let sources: Vec<_> = doc
        .layouts
        .iter()
        .filter_map(|layout| {
            let link = layout.mirror_link.as_ref()?;
            doc.layouts
                .iter()
                .find(|source| source.id == link.source_id)
                .map(|source| source.matrix_ids.clone())
        })
        .collect();
    for source in sources.into_iter().flatten() {
        sync(doc, &source)?;
    }
    refresh_existing(doc)?;
    sync_components(doc)?;
    Ok(())
}

pub fn refresh_existing(doc: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let refresh = |doc: &mut ProjectDoc| {
        for index in matrix::evaluation::order(doc)? {
            let matrix = &doc.matrices[index];
            let evaluated = matrix::evaluated(matrix, doc)?;
            let mut poses = BTreeMap::new();
            for geometry in &evaluated.cells {
                let id = matrix::member_id(&matrix.id, geometry.row as u32, geometry.column as u32);
                if exception_state(doc, MirroredItemKind::Part, &id)
                    == Some(MirroredExceptionState::Local)
                {
                    continue;
                }
                poses.insert(id.clone(), geometry.pose);
                if let Some(cell) = matrix.cells.iter().find(|cell| {
                    cell.row == geometry.row as u32 && cell.column == geometry.column as u32
                }) {
                    for assembly in &cell.assemblies {
                        let id = matrix::companion_id(&id, &assembly.id);
                        if matches!(companion_mode(doc, &id), MatrixCompanionMode::Follows) {
                            poses.insert(id, matrix::companion_pose(geometry.pose, assembly));
                        }
                    }
                }
            }
            for part in &mut doc.parts {
                if let Some(pose) = poses.get(&part.id)
                    && !matrix::overridden(part)
                {
                    part.pose = *pose;
                }
            }
        }
        Ok(vec![])
    };
    if doc
        .matrices
        .iter()
        .flat_map(|matrix| &matrix.cells)
        .any(|cell| !cell.assemblies.is_empty())
    {
        matrix::placement::carrying_authored_companions(
            doc,
            matrix::placement::Reported::PreserveEditReport,
            refresh,
        )
    } else {
        // Refresh only writes poses. Without assembly declarations it cannot
        // introduce a companion that needs the pre-refresh snapshot.
        refresh(doc)
    }
}

pub fn exception_state(
    doc: &ProjectDoc,
    kind: MirroredItemKind,
    id: &str,
) -> Option<MirroredExceptionState> {
    doc.layouts
        .iter()
        .filter_map(|layout| layout.mirror_link.as_ref())
        .flat_map(|link| &link.exceptions)
        .chain(
            doc.hardware
                .iter()
                .flat_map(|hardware| &hardware.pairs)
                .filter_map(|pair| pair.halves.as_ref())
                .flat_map(|halves| &halves.exceptions),
        )
        .find(|exception| exception.kind == kind && exception.mirror_id == id)
        .map(|exception| exception.state)
}

pub fn omitted(doc: &ProjectDoc, matrix_id: &str, key_id: &str) -> bool {
    // Native reconciliation owns its staged membership plan; filter same-board overlays here.
    let state = |kind, id: &str| {
        doc.layouts
            .iter()
            .filter_map(|layout| layout.mirror_link.as_ref())
            .flat_map(|link| &link.exceptions)
            .find(|exception| exception.kind == kind && exception.mirror_id == id)
            .map(|exception| exception.state)
    };
    state(MirroredItemKind::Part, key_id).or_else(|| state(MirroredItemKind::Matrix, matrix_id))
        == Some(MirroredExceptionState::Omitted)
}

/// Ticket 12: the pair a layout belongs to: the source layout id, the link
/// carrier (mirror layout) id, whether `layout_id` is the source side, and
/// whether the pair stores a Split gap. `None` for unlinked layouts.
pub(super) fn pair_sides(
    doc: &ProjectDoc,
    layout_id: &str,
) -> Option<(String, String, bool, bool)> {
    let layout = doc.layouts.iter().find(|layout| layout.id == layout_id)?;
    if let Some(link) = layout.mirror_link.as_ref() {
        return Some((
            link.source_id.clone(),
            layout.id.clone(),
            false,
            link.split_gap.or(link.spacing).is_some(),
        ));
    }
    let carrier = doc.layouts.iter().find(|other| {
        other
            .mirror_link
            .as_ref()
            .is_some_and(|link| link.source_id == layout_id)
    })?;
    let link = carrier.mirror_link.as_ref()?;
    Some((
        layout_id.to_string(),
        carrier.id.clone(),
        true,
        link.split_gap.or(link.spacing).is_some(),
    ))
}

/// Ticket 12: the Part correspondence naming `part_id` on a same-board
/// mirror link, with the carrier layout id. Either copy carries the link: the
/// correspondence is the shared record while both copies exist; its absence
/// means the part is local to its half.
/// Ticket 12: the live Part correspondence naming `part_id` on a same-board
/// mirrored pair, with the carrier layout id. The Inspector toggle and the
/// mirror-side edit path both resolve through here; the hardware-halves store
/// belongs to the two-board flow and never carries these records.
pub fn part_correspondence(
    doc: &ProjectDoc,
    part_id: &str,
) -> Option<(String, MirroredCorrespondence)> {
    for layout in &doc.layouts {
        if let Some(link) = layout.mirror_link.as_ref()
            && let Some(record) = link.correspondences.iter().find(|record| {
                record.kind == MirroredItemKind::Part
                    && (record.source_id == part_id || record.mirror_id == part_id)
            })
        {
            return Some((layout.id.clone(), record.clone()));
        }
    }
    None
}

/// Whether `definition_id` names a controller: controllers keep the plain
/// reflected heading so both halves share the world rotation.
pub(super) fn is_controller_definition(doc: &ProjectDoc, definition_id: &str) -> bool {
    doc.definitions
        .iter()
        .any(|definition| definition.id == definition_id && definition.kind == PartKind::Controller)
}

/// Ticket 12: reflect a half-frame part pose across the split axis. Stored x
/// negates (both halves' frames grow from the axis); the heading follows the
/// shared mirrored-part rule. An involution: reflecting twice restores the pose.
pub fn reflect_half_part(doc: &ProjectDoc, part_id: &str, pose: Pose2) -> Pose2 {
    let is_controller = doc
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .map(|part| is_controller_definition(doc, &part.definition_id))
        .unwrap_or(false);
    Pose2 {
        at: Vec2 {
            x: -pose.at.x,
            y: pose.at.y,
        },
        rotation: pair::mirrored_part_heading(is_controller, axis_mapping(0.0), pose.rotation),
    }
}

/// Ticket 12: evaluation offset of a layout record: the split axis offset by
/// half the current spacing on the record's side. `None` for unlinked layouts
/// and legacy pairs without a stored gap. Neither record needs to be in the
/// document yet, so SetLayout can reframe against the incoming record.
pub(super) fn carrier_offset(doc: &ProjectDoc, layout: &Layout) -> Option<Vec2> {
    // A layout carrying the mirror link is always the mirror side: a link
    // source never carries a link of its own.
    if let Some(link) = layout.mirror_link.as_ref() {
        let gap = link.spacing.or(link.split_gap)?;
        return Some(Vec2 {
            x: link.axis_x + gap / 2.0,
            y: 0.0,
        });
    }
    let link = doc
        .layouts
        .iter()
        .filter_map(|other| other.mirror_link.as_ref())
        .find(|link| link.source_id == layout.id)?;
    let gap = link.spacing.or(link.split_gap)?;
    Some(Vec2 {
        x: link.axis_x - gap / 2.0,
        y: 0.0,
    })
}
