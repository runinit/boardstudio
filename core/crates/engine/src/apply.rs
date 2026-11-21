use super::preview::changed_ids;
use super::*;

pub(super) fn validate_project_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Project name cannot be empty".into());
    }
    Ok(())
}

pub(crate) fn uses_prepared_transaction(document: &ProjectDoc, operation: &EditOperation) -> bool {
    if matches!(
        operation,
        EditOperation::AddMirrorOnlyPart { .. }
            | EditOperation::SetCompanionPlacement { .. }
            | EditOperation::SetHalfException { .. }
            | EditOperation::UnlinkHalves { .. }
    ) {
        return true;
    }
    has_native_mirror_pair(document)
}

pub(super) fn has_native_mirror_pair(document: &ProjectDoc) -> bool {
    document.hardware.as_ref().is_some_and(|hardware| {
        hardware.pairs.iter().any(|pair| {
            pair.halves
                .as_ref()
                .is_some_and(|halves| halves.source_board_id != halves.mirror_board_id)
        })
    })
}

pub(crate) fn affects_outline(op: &EditOperation) -> bool {
    if let EditOperation::ApplyMechanicalEdit { edit } = op {
        return crate::mechanical::edit::affects_outline(&edit.change);
    }
    if let EditOperation::ApplyOutlineEdit { edit } = op {
        return crate::outline_edit::affects_outline(&edit.change);
    }
    matches!(
        op,
        EditOperation::SetMountedModule {
            host_connector_definition: Some(_),
            ..
        } | EditOperation::MoveParts { .. }
            | EditOperation::SetKeycapSize { .. }
            | EditOperation::AddPart { .. }
            | EditOperation::AddMirrorOnlyPart { .. }
            | EditOperation::RemoveParts { .. }
            | EditOperation::RemoveMatrix { .. }
            | EditOperation::SetMatrixCellsEnabled { .. }
            | EditOperation::RemoveSelectedMatrixCells { .. }
            | EditOperation::RemoveMatrixCells { .. }
            | EditOperation::RemoveMatrixRow { .. }
            | EditOperation::RemoveMatrixColumn { .. }
            | EditOperation::SetMatrix { .. }
            | EditOperation::SetMatrixSplay { .. }
            | EditOperation::SetMatrixPlacement { .. }
            | EditOperation::CreateMirroredPair { .. }
            | EditOperation::CreateMirroredHalf { .. }
            | EditOperation::SetLayout { .. }
            | EditOperation::SetSplitGap { .. }
            | EditOperation::SetMirroredPart { .. }
            | EditOperation::SetPartAnchor { .. }
            | EditOperation::SetPartAnchorFollow { .. }
            | EditOperation::DetachPartAnchor { .. }
            | EditOperation::SetPartAnchorValues { .. }
            | EditOperation::SetPartRotation { .. }
            | EditOperation::SetConstraint { .. }
            | EditOperation::RemoveConstraint { .. }
            | EditOperation::ReplaceDocument { .. }
    )
}

fn add_part_placement(
    document: &mut ProjectDoc,
    part: &Part,
    board_id: Option<&str>,
    definition: Option<&PartDefinition>,
    layout_id: Option<&str>,
) -> Result<Vec<String>, String> {
    if document.parts.iter().any(|existing| existing.id == part.id) {
        return Err(format!("Duplicate part {}", part.id));
    }
    if let Some(definition) = definition {
        if definition.id != part.definition_id {
            return Err(format!(
                "Definition {:?} does not match part definition {:?}",
                definition.id, part.definition_id
            ));
        }
        let mut existing = document
            .definitions
            .iter()
            .filter(|item| item.id == definition.id);
        match (existing.next(), existing.next()) {
            (Some(current), None) if current == definition => {}
            (None, None) => document.definitions.push(definition.clone()),
            _ => {
                return Err(format!(
                    "Definition {:?} already exists with different or ambiguous contents",
                    definition.id
                ));
            }
        }
    }
    if !geometry::part_valid(part, document) {
        return Err(format!("Unknown definition {}", part.definition_id));
    }

    let board_id = board_id
        .map(str::to_owned)
        .or_else(|| document.boards.first().map(|board| board.id.clone()));
    let board_index = board_id
        .as_deref()
        .map(|id| {
            let mut boards = document
                .boards
                .iter()
                .enumerate()
                .filter(|(_, board)| board.id == id);
            let (index, _) = boards.next().ok_or_else(|| format!("Unknown board {id}"))?;
            if boards.next().is_some() {
                return Err(format!("Board {id:?} is ambiguous"));
            }
            Ok(index)
        })
        .transpose()?;
    let board_outline = board_index
        .and_then(|index| document.boards.get(index))
        .map(|board| board.outline_ids.clone())
        .unwrap_or_default();
    if let Some(layout_id) = layout_id {
        let mut layouts = document
            .layouts
            .iter()
            .filter(|layout| layout.id == layout_id);
        let layout = layouts
            .next()
            .ok_or_else(|| format!("Unknown layout {layout_id}"))?;
        if layouts.next().is_some() {
            return Err(format!("Layout {layout_id:?} is ambiguous"));
        }
        if Some(layout.board_id.as_str()) != board_id.as_deref() {
            return Err(format!(
                "Layout {layout_id:?} does not belong to selected board"
            ));
        }
        if layout.part_ids.contains(&part.id) {
            return Err(format!(
                "Layout {layout_id:?} already contains part {:?}",
                part.id
            ));
        }
        if layout.matrix_ids.is_empty()
            || layout.matrix_ids.iter().any(|id| {
                !document.matrices.iter().any(|matrix| {
                    &matrix.id == id && matrix.board_id.as_ref() == Some(&layout.board_id)
                })
            })
        {
            return Err(format!(
                "Layout {layout_id:?} has inconsistent matrix ownership"
            ));
        }
    }

    if let Some(index) = board_index {
        document.boards[index].part_ids.push(part.id.clone());
    }
    let mut attached = false;
    for feature in &mut document.outline {
        if let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature
            && board_outline.contains(id)
        {
            part_ids.push(part.id.clone());
            attached = true;
        }
    }
    if !attached
        && document.boards.is_empty()
        && let Some(OutlineFeature::PartEnvelope { part_ids, .. }) = document
            .outline
            .iter_mut()
            .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
    {
        part_ids.push(part.id.clone());
    }
    if let Some(layout_id) = layout_id {
        document
            .layouts
            .iter_mut()
            .find(|layout| layout.id == layout_id)
            .ok_or_else(|| format!("Unknown layout {layout_id}"))?
            .part_ids
            .push(part.id.clone());
    }
    // Ticket 12: drops onto a stored-gap half arrive board-space; storage is
    // half-frame, so convert at insertion. Rotations are frame-free.
    let mut stored = part.clone();
    // Ticket 13: an independent drop without an anchor takes the nearest key
    // of its half, keeping the drop point upright. Members, companions and
    // driven parts stay unanchored; a preset anchor is validated on use.
    if stored.anchor.is_none()
        && let Some(layout_id) = layout_id
        && !matrix::layout::is_member_or_companion(document, &part.id)
        && document
            .constraint_driving_part(&part.id)
            .ok()
            .flatten()
            .is_none()
    {
        stored.anchor = matrix::part_anchors::drop_anchor(document, layout_id, part.pose);
    }
    // A preset anchor is validated like a Pick: a stale key refuses instead
    // of silently falling back to the stored pose.
    if let Some(anchor) = stored.anchor.as_ref() {
        matrix::part_anchors::validate_target(document, &stored, anchor)?;
    }
    if let Some(layout_id) = layout_id
        && let Some(offset) = layout::layout_half_offset(document, layout_id)
        && !layout::is_member_or_companion(document, &part.id)
    {
        stored.pose.at.x -= offset.x;
        stored.pose.at.y -= offset.y;
    }
    document.parts.push(stored);
    Ok(vec![part.id.clone()])
}

#[cfg(test)]
pub(crate) fn apply(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
) -> Result<Vec<String>, String> {
    apply_with_ledger(
        doc,
        board_contours,
        op,
        &mechanical::edit::MechanicalIdentityLedger::default(),
    )
}

pub(crate) fn edit_dispatch() -> mirrored_half::EditDispatch {
    mirrored_half::EditDispatch {
        apply_with_ledger,
        apply_unscripted,
    }
}

/// Staged-edit dispatch for the engine integration tests.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn apply_with_ledger_for_test(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
    identities: &mechanical::edit::MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    apply_with_ledger(doc, board_contours, op, identities)
}

#[doc(hidden)]
pub fn apply_unscripted_for_test(
    doc: &mut ProjectDoc,
    op: &EditOperation,
) -> Result<Vec<String>, String> {
    apply_unscripted(doc, op)
}

/// Applies one operation under a fresh identity ledger for sibling crates' tests.
#[doc(hidden)]
pub fn apply_for_test(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
) -> Result<Vec<String>, String> {
    apply_with_ledger(
        doc,
        board_contours,
        op,
        &mechanical::edit::MechanicalIdentityLedger::default(),
    )
}

// Recursive and staged edits dispatch known operations and cannot replace the document.
fn apply_unscripted(doc: &mut ProjectDoc, op: &EditOperation) -> Result<Vec<String>, String> {
    apply_with_ledger(
        doc,
        &[],
        op,
        &mechanical::edit::MechanicalIdentityLedger::default(),
    )
}

pub(crate) fn apply_with_ledger(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
    identities: &mechanical::edit::MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    let mut changed = match op {
        EditOperation::SetModuleDefinition { definition } => {
            modules::set_definition(doc, definition)
        }
        EditOperation::SetMountedModule {
            instance,
            definition,
            host_connector_definition,
        } => modules::set(
            doc,
            instance,
            definition.as_deref(),
            host_connector_definition.as_deref(),
        ),
        EditOperation::RemoveMountedModule { id } => modules::remove(doc, id),
        EditOperation::EmbedModuleCircuit {
            id,
            definition,
            host_board_id,
            pose,
            side,
            joins,
        } => modules::embed(
            doc,
            id,
            definition,
            host_board_id,
            *pose,
            side.clone(),
            joins,
        ),
        EditOperation::RemoveEmbeddedCircuit { id } => modules::remove_circuit(doc, id),
        EditOperation::SetInputScanMode { part_id, mode } => {
            inputs::set_scan_mode(doc, part_id, *mode)
        }
        EditOperation::SetWiringMode { board_id, mode } => {
            if !doc.boards.iter().any(|board| board.id == *board_id) {
                return Err(format!("Unknown board {board_id}"));
            }
            if doc
                .hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .boards
                        .iter()
                        .find(|configuration| configuration.board_id == *board_id)
                })
                .map_or(electrical::ElectricalMode::Matrix, |configuration| {
                    configuration.mode
                })
                == *mode
            {
                return Ok(Vec::new());
            }
            let hardware = doc.hardware.get_or_insert_with(Default::default);
            if let Some(configuration) = hardware
                .boards
                .iter_mut()
                .find(|configuration| configuration.board_id == *board_id)
            {
                configuration.mode = *mode;
            } else {
                hardware.boards.push(ElectricalBoardConfiguration {
                    board_id: board_id.clone(),
                    mode: *mode,
                    ..Default::default()
                });
            }
            Ok(vec![board_id.clone()])
        }
        EditOperation::EditKeymap { change } => keymap::apply_edit(doc, change),
        EditOperation::SetKeyBinding { .. }
        | EditOperation::SetKeycapBoard { .. }
        | EditOperation::SetMatrixKeycaps { .. }
        | EditOperation::SetKeycapKey { .. } => keycaps::apply_edit(doc, op),
        EditOperation::SetKeycapSize {
            sizes,
            moves,
            offsets,
        } => {
            // Validate every entry before writing anything: one atomic typed
            // entry or nothing.
            for (id, size) in sizes {
                doc.parts
                    .iter()
                    .find(|part| &part.id == id)
                    .ok_or_else(|| format!("Unknown key {id}"))?;
                if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
                    return Err(format!("Keycap size for {id} must be positive and finite"));
                }
            }
            for position in moves {
                doc.parts
                    .iter()
                    .find(|part| part.id == position.id)
                    .ok_or_else(|| format!("Unknown key {}", position.id))?;
                if !position.at.x.is_finite() || !position.at.y.is_finite() {
                    return Err(format!("Keycap pose for {} must be finite", position.id));
                }
            }
            for cell in offsets {
                doc.matrices
                    .iter()
                    .find(|matrix| matrix.id == cell.matrix_id)
                    .ok_or_else(|| format!("Unknown matrix {}", cell.matrix_id))?;
                if let Some(offset) = &cell.offset
                    && (!offset.x.is_finite() || !offset.y.is_finite())
                {
                    return Err(format!(
                        "Keycap cell offset for {} is not finite",
                        cell.matrix_id
                    ));
                }
            }
            let mut changed = BTreeSet::new();
            for (id, size) in sizes {
                let part = doc
                    .parts
                    .iter_mut()
                    .find(|part| &part.id == id)
                    .expect("validated");
                part.keycap = Some(*size);
                changed.insert(id.clone());
            }
            for position in moves {
                let part = doc
                    .parts
                    .iter_mut()
                    .find(|part| part.id == position.id)
                    .expect("validated");
                part.pose.at = position.at;
                changed.insert(position.id.clone());
            }
            for cell in offsets {
                let matrix = doc
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == cell.matrix_id)
                    .expect("validated");
                if let Some(existing) = matrix
                    .cells
                    .iter_mut()
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                {
                    existing.offset = cell.offset;
                } else {
                    matrix.cells.push(MatrixCell {
                        shift: None,
                        bend: None,
                        deleted: false,
                        row: cell.row,
                        column: cell.column,
                        enabled: true,
                        definition_id: None,
                        variant: None,
                        offset: cell.offset,
                        rotation: None,
                        assemblies: vec![],
                        assemblies_local: None,
                    });
                }
                changed.insert(cell.matrix_id.clone());
            }
            Ok(changed.into_iter().collect())
        }
        EditOperation::MoveParts { positions } => {
            let (mut changed, handled) = layout::move_keys(doc, positions)?;
            for position in positions {
                if handled.contains(&position.id) {
                    continue;
                }
                let Some(index) = doc.parts.iter().position(|part| part.id == position.id) else {
                    return Err(format!("Unknown part {}", position.id));
                };
                if doc.parts[index].locked == Some(true) {
                    return Err(format!("Part {} is locked", position.id));
                }
                if !position.at.x.is_finite() || !position.at.y.is_finite() {
                    return Err("Position must be finite".into());
                }
                // Ticket 12: pair parts cannot cross the split axis; the drag
                // stops at it. The clamp is the identity off a mirrored half.
                // Drags arrive board-space while storage is half-frame, so the
                // retune delta measures against the evaluated pose.
                let at = layout::clamp_part_to_half(doc, &position.id, position.at);
                // Ticket 13: an anchored part re-solves Spread/Stagger from
                // the clamped drop instead of writing the pose.
                if doc.parts[index].anchor.is_some() {
                    changed.extend(matrix::part_anchors::move_anchored(doc, &position.id, at)?);
                    continue;
                }
                let current = layout::evaluated_part_pose(doc, &doc.parts[index]);
                let delta = Vec2 {
                    x: at.x - current.at.x,
                    y: at.y - current.at.y,
                };
                if let Some(constraint) = doc
                    .constraints
                    .iter_mut()
                    .find(|constraint| constraint.target() == position.id)
                {
                    if constraints::retune_offset(&doc.parts, constraint, delta, None)?
                        == constraints::Retune::Mirror
                    {
                        return Err(constraints::mirror_drag_refusal(&position.id));
                    }
                    changed.push(position.id.clone());
                    continue;
                }
                // Ticket 12: a correspondence-mirrored part on a stored-gap
                // pair writes both copies through, so either copy edits both.
                if let Some(mirrored) = layout::move_pair_part(doc, &position.id, at)? {
                    changed.extend(mirrored);
                    continue;
                }
                if let Some(linked) = layout::move_linked_component(doc, &position.id, at)? {
                    changed.extend(linked);
                    continue;
                }
                // Ticket 12: the drop commits board-space; storage converts.
                let stored = layout::stored_part_pose(
                    doc,
                    &position.id,
                    Pose2 {
                        at,
                        rotation: doc.parts[index].pose.rotation,
                    },
                );
                let part = &mut doc.parts[index];
                part.pose.at = stored.at;
                if doc
                    .matrices
                    .iter()
                    .any(|matrix| matrix.part_ids.contains(&part.id))
                {
                    matrix::mark_override(part);
                }
                changed.push(part.id.clone());
            }
            Ok(changed)
        }
        EditOperation::AddPart { .. } | EditOperation::AddMirrorOnlyPart { .. } => match op {
            EditOperation::AddPart {
                part,
                board_id,
                definition,
                layout_id,
            } => {
                let mut changed = add_part_placement(
                    doc,
                    part,
                    board_id.as_deref(),
                    definition.as_ref(),
                    layout_id.as_deref(),
                )?;
                // Ticket 12: a part added to a stored-gap half mirrors by
                // correspondence in the same Undo entry; legacy pairs mirror
                // through the resolve sync. Mirror-only adds stay single.
                changed.extend(layout::mirror_new_pair_part(doc, &part.id)?);
                Ok(changed)
            }
            EditOperation::AddMirrorOnlyPart {
                part,
                board_id,
                definition,
                layout_id,
            } => add_part_placement(
                doc,
                part,
                Some(board_id),
                definition.as_ref(),
                layout_id.as_deref(),
            ),
            _ => unreachable!("the placement arm matches both Part addition operations"),
        },
        EditOperation::SetMirroredPart { part_id, mirrored } => {
            layout::set_part_mirrored(doc, part_id, *mirrored)
        }
        EditOperation::SetPartAnchor { part_id, anchor } => {
            matrix::part_anchors::set_anchor(doc, part_id, anchor)
        }
        EditOperation::SetPartAnchorFollow { part_id, follow } => {
            matrix::part_anchors::set_follow(doc, part_id, follow)
        }
        EditOperation::DetachPartAnchor { part_id } => matrix::part_anchors::detach(doc, part_id),
        EditOperation::SetPartAnchorValues { part_id, change } => {
            matrix::part_anchors::set_value(doc, part_id, change)
        }
        EditOperation::SetPartRotation { part_id, rotation } => {
            layout::set_detached_part_rotation(doc, part_id, *rotation)
        }
        EditOperation::RemoveMatrix { id } => {
            let ids = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == *id)
                .ok_or("Matrix does not exist")?
                .part_ids
                .clone();
            // Dependants keep their current world frame instead of dangling.
            let mut detached = matrix::anchors::detach_dependants(doc, id)?;
            // Ticket 21: linked partners follow the deletion or survive it
            // complete; only defunct pair records retire.
            let follow = layout::plan_linked_matrix_removal(doc, id, &mut detached)?;
            for gone in std::iter::once(id.clone())
                .chain(follow.matrices.clone())
                .chain(follow.retired.clone())
            {
                layout::remove_matrix(doc, &gone);
            }
            let mut ids = ids;
            ids.extend(follow.part_ids);
            let mut changed = apply_unscripted(doc, &EditOperation::RemoveParts { ids })?;
            changed.append(&mut detached);
            doc.matrices.retain(|matrix| {
                matrix.id != *id
                    && !follow.matrices.contains(&matrix.id)
                    && !follow.retired.contains(&matrix.id)
            });
            let mut empty_owned: std::collections::BTreeSet<_> = std::collections::BTreeSet::new();
            for gone in std::iter::once(id.clone()).chain(follow.matrices.clone()) {
                let prefix = format!("matrix/{gone}/net/");
                empty_owned.extend(
                    doc.nets
                        .iter()
                        .filter(|net| net.id.starts_with(&prefix) && net.pins.is_empty())
                        .map(|net| net.id.clone()),
                );
            }
            doc.nets.retain(|net| !empty_owned.contains(&net.id));
            for board in &mut doc.boards {
                board.net_ids.retain(|id| !empty_owned.contains(id));
            }
            changed.push(id.clone());
            changed.extend(follow.matrices);
            changed.extend(follow.retired);
            Ok(changed)
        }
        EditOperation::SetMatrixCellsEnabled { cells, enabled } => {
            matrix::deletion::edit_selected_cells(doc, cells, Some(*enabled))
        }
        EditOperation::RemoveSelectedMatrixCells { cells } => {
            matrix::deletion::edit_selected_cells(doc, cells, None)
        }
        EditOperation::RemoveMatrixCells { matrix_id, cells } => {
            matrix::deletion::remove_cells(doc, matrix_id, cells)
        }
        EditOperation::RemoveMatrixRow { matrix_id, row } => {
            matrix::deletion::remove_axis(doc, matrix_id, *row, true, apply_unscripted)
        }
        EditOperation::RemoveMatrixColumn { matrix_id, column } => {
            matrix::deletion::remove_axis(doc, matrix_id, *column, false, apply_unscripted)
        }
        EditOperation::RemoveParts { ids } => {
            let affected: Vec<_> = doc
                .matrices
                .iter()
                .filter(|matrix| matrix.part_ids.iter().any(|id| ids.contains(id)))
                .map(|matrix| matrix.id.clone())
                .collect();
            let ids = layout::mirrored_component_ids(doc, ids);
            let ids = matrix::removed_members(doc, &ids);
            doc.constraints.retain(|constraint| {
                !ids.iter()
                    .any(|id| id == constraint.source() || id == constraint.target())
            });
            doc.parts.retain(|part| !ids.contains(&part.id));
            for board in &mut doc.boards {
                board.part_ids.retain(|id| !ids.contains(id));
            }
            for net in &mut doc.nets {
                net.pins.retain(|pin| !ids.contains(&pin.part_id));
            }
            for feature in &mut doc.outline {
                if let OutlineFeature::PartEnvelope { part_ids, .. } = feature {
                    part_ids.retain(|id| !ids.contains(id));
                }
            }
            for matrix in &mut doc.matrices {
                matrix.part_ids.retain(|id| !ids.contains(id));
            }
            for layout in &mut doc.layouts {
                layout.part_ids.retain(|id| !ids.contains(id));
            }
            // A linked refinement drops exactly the authored choices that required a
            // removed component, inside this same accepted edit, so the cleanup and the
            // removal share one Undo step and never retarget onto another component.
            // Ticket 12: removing either copy of a mirrored part drops its Part
            // correspondence, so no dangling record survives the removal.
            layout::retire_part_correspondences(doc, &ids);
            let mut changed = ids;
            let cleanup = outline_versions::prune_removed_sources(doc, &changed);
            changed.extend(cleanup);
            for matrix_id in affected {
                changed.extend(layout::sync(doc, &matrix_id)?);
            }
            Ok(changed)
        }
        EditOperation::SetCase { body } => {
            if let Some(current) = doc.case_bodies.iter_mut().find(|item| item.id == body.id) {
                *current = body.clone();
            } else {
                doc.case_bodies.push(body.clone());
            }
            Ok(vec![body.id.clone()])
        }
        EditOperation::SetMatrix {
            matrix,
            definitions,
        } => matrix::placement::carrying_authored_companions(
            doc,
            matrix::placement::Reported::PreserveEditReport,
            |doc| {
                let mut changed = vec![];
                let mut provided = BTreeSet::new();
                for definition in definitions.iter().flatten() {
                    if definition.id.trim().is_empty() || !provided.insert(&definition.id) {
                        return Err("Matrix definition ID is invalid or duplicated".into());
                    }
                    if let Some(existing) =
                        doc.definitions.iter().find(|item| item.id == definition.id)
                    {
                        if existing != definition {
                            return Err(format!("Conflicting definition {}", definition.id));
                        }
                        continue;
                    }
                    doc.definitions.push(definition.clone());
                    changed.push(definition.id.clone());
                }
                // Ticket 21: an absent matrix has no independent storage
                // state; refuse before any detach side effect.
                layout::refuse_omitted_storage_edit(doc, &matrix.id, &[])?;
                if let Some(previous) = doc
                    .matrices
                    .iter()
                    .find(|item| item.id == matrix.id)
                    .cloned()
                {
                    // Shrinking never retargets an anchor onto a new key at an old
                    // coordinate: dependants of removed slots detach with their frame.
                    changed.extend(matrix::anchors::detach_out_of_range(
                        doc,
                        &matrix.id,
                        matrix.rows,
                        matrix.columns,
                    )?);
                    matrix::anchors::check_anchored_frame_change(doc, &previous, matrix)?;
                }
                changed.extend(matrix::set_matrix(doc, matrix)?);
                changed.extend(layout::sync(doc, &matrix.id)?);
                layout::sync_member_pairs(doc, &matrix.id)?;
                Ok(changed)
            },
        ),
        EditOperation::SetMatrixSplay {
            matrix_id,
            column,
            change,
        } => {
            // Ticket 21: an absent matrix has no independent storage state.
            layout::refuse_omitted_storage_edit(doc, matrix_id, &[])?;
            let current = doc
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)
                .ok_or("Matrix not found")?;
            let matrix = matrix::splay::update(current, *column, change, doc)?;
            if matrix == *current {
                return Ok(Vec::new());
            }
            // A splay reports every member it touched, not only those that moved.
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Touched,
                |doc| {
                    let mut changed = matrix::set_matrix(doc, &matrix)?;
                    changed.extend(layout::sync(doc, matrix_id)?);
                    Ok(changed)
                },
            )
        }
        EditOperation::SetMatrixPlacement { matrix_id, change } => {
            // Ticket 21: an absent matrix has no independent storage state.
            layout::refuse_omitted_storage_edit(doc, matrix_id, &[])?;
            let current = doc
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)
                .cloned()
                .ok_or("Matrix not found")?;
            let matrix = matrix::placement::update(&current, change, doc)?;
            // Setting the current value is a valid no-op.
            if matrix == current {
                return Ok(Vec::new());
            }
            // Report only members whose projection actually moved.
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| {
                    let mut changed = matrix::set_matrix(doc, &matrix)?;
                    changed.extend(layout::sync(doc, matrix_id)?);
                    Ok(changed)
                },
            )
        }
        EditOperation::AddAnchoredMatrix { id, anchor } => {
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| matrix::anchors::add(doc, id, anchor),
            )
        }
        EditOperation::SetMatrixAnchor { matrix_id, anchor } => {
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| matrix::anchors::set_anchor(doc, matrix_id, anchor),
            )
        }
        EditOperation::DetachMatrixAnchor { matrix_id } => {
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| matrix::anchors::detach(doc, matrix_id),
            )
        }
        EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id,
            local_matrix_id,
        } => matrix::placement::carrying_authored_companions(
            doc,
            matrix::placement::Reported::Moved,
            |doc| matrix::anchors::detach_confirmed(doc, matrix_id, local_matrix_id),
        ),
        EditOperation::CreateMirroredPair {
            left,
            right,
            matrix,
            definitions,
        } => {
            let link = right
                .mirror_link
                .as_ref()
                .ok_or("A new mirrored pair must be linked")?;
            if left.mirror_link.is_some()
                || link.source_id != left.id
                || left.id == right.id
                || left.matrix_ids != [matrix.id.clone()]
                || right.matrix_ids.len() != 1
                || left.matrix_ids == right.matrix_ids
                || matrix.board_id.as_ref() != Some(&left.board_id)
                || left.board_id != right.board_id
                || !left.part_ids.is_empty()
                || !right.part_ids.is_empty()
                || !matrix.part_ids.is_empty()
                || doc
                    .layouts
                    .iter()
                    .any(|layout| layout.id == left.id || layout.id == right.id)
                || doc.matrices.iter().any(|item| {
                    left.matrix_ids.contains(&item.id) || right.matrix_ids.contains(&item.id)
                })
            {
                return Err(
                    "Mirrored pair requires two new layouts and distinct matrices on one board"
                        .into(),
                );
            }
            // New pairs store the half frame directly: the source origin is
            // re-expressed half-local and the mirror reflects it, so a later
            // Split gap change moves both halves without rewriting storage.
            // Anchored matrices stay legacy (no stored gap); their first edit
            // adopts only when an unanchored source matrix exists.
            // Ticket 11: the conversion still uses the requested placement,
            // while the stored gap is max(requested, 40 mm, minimum), so the
            // halves widen symmetrically to the stored gap on evaluation.
            let requested_gap = if matrix.anchor.is_none() {
                let gap = layout::implied_gap(link.axis_x, matrix);
                if !gap.is_finite() {
                    return Err("Split gap must be finite".into());
                }
                Some(gap)
            } else {
                None
            };
            let stored_gap = match requested_gap {
                Some(_) => Some(layout::proposed_creation_gap(doc, matrix, link.axis_x)?),
                None => None,
            };
            let mut source_half = matrix.as_ref().clone();
            if let Some(gap) = requested_gap {
                source_half.origin.x -= link.axis_x - gap / 2.0;
            }
            let mut changed = apply_unscripted(
                doc,
                &EditOperation::SetMatrix {
                    matrix: source_half.clone(),
                    definitions: definitions.clone(),
                },
            )?;
            let mut target = source_half.clone();
            target.id = right.matrix_ids[0].clone();
            target.name = Some(right.name.clone());
            for cell in &mut target.cells {
                for assembly in &mut cell.assemblies {
                    assembly.offset.x = -assembly.offset.x;
                    assembly.rotation = assembly.rotation.map(|angle| -angle);
                }
            }
            let target = layout::reflected(
                &source_half,
                &target,
                layout::axis_mapping(link.axis_x),
                false,
                stored_gap.is_some(),
            )?;
            changed.extend(matrix::set_matrix(doc, &target)?);
            let mut follower = right.clone();
            let link = follower
                .mirror_link
                .as_mut()
                .ok_or("A new mirrored pair must be linked")?;
            link.split_gap = stored_gap;
            link.correspondences.clear();
            link.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Matrix,
                source_id: matrix.id.clone(),
                mirror_id: target.id.clone(),
                companion_mode: None,
                companion_tweak: None,
            });
            link.correspondences
                .extend(layout::part_correspondences(layout::member_pairs(
                    matrix,
                    &target.id,
                    Some(&target),
                )));
            doc.layouts.extend([left.clone(), follower]);
            layout::validate(doc)?;
            changed.extend([left.id.clone(), right.id.clone()]);
            Ok(changed)
        }
        EditOperation::CreateMirroredHalf {
            pair_id,
            source_instance_id,
            mode,
            labels,
            physical_setup,
            companion_mode,
            companion_tweak,
            mapping,
            id_rule,
            mounting_hole,
        } => match physical_setup {
            Some(setup) => mirrored_half::create_with_physical_setup(
                doc,
                pair_id,
                source_instance_id,
                mode,
                *mapping,
                *id_rule,
                mounting_hole.as_deref(),
                *companion_mode,
                *companion_tweak,
                labels.as_ref(),
                setup,
            ),
            None => mirrored_half::create(
                doc,
                pair_id,
                source_instance_id,
                mode,
                *mapping,
                *id_rule,
                mounting_hole.as_deref(),
                *companion_mode,
                *companion_tweak,
                labels.as_ref(),
            ),
        },
        EditOperation::SetLayout { layout: incoming } => {
            let mut incoming = incoming.clone();
            if incoming.mirror_link.is_some()
                && doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == incoming.id)
                    .is_some_and(|layout| layout.mirror_link.is_none())
            {
                layout::populate_new_link(doc, &mut incoming)?;
            }
            let (old_gap, was_linked_target, source_layout_id) = {
                let current = doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == incoming.id)
                    .ok_or("Layout does not exist")?;
                if current.board_id != incoming.board_id
                    || current.matrix_ids != incoming.matrix_ids
                {
                    return Err("Layout ownership cannot be reassigned".into());
                }
                let was_linked_target =
                    current.mirror_link.is_some() && incoming.mirror_link.is_none();
                let source_layout_id = was_linked_target
                    .then(|| {
                        current
                            .mirror_link
                            .as_ref()
                            .map(|link| link.source_id.clone())
                    })
                    .flatten();
                (
                    current.mirror_link.as_ref().and_then(|link| link.split_gap),
                    was_linked_target,
                    source_layout_id,
                )
            };
            // Half-frame gap transitions run before the record is replaced so
            // the conversion still sees the previous gap: adding a gap adopts
            // the board placement first, clearing one restores it.
            let new_gap = incoming
                .mirror_link
                .as_ref()
                .and_then(|link| link.split_gap);
            match (old_gap, new_gap) {
                (None, Some(_)) => {
                    layout::adopt_half_frames(doc)?;
                }
                (Some(_), None) => {
                    layout::unadopt_pair(doc, &incoming.id)?;
                }
                _ => {}
            }
            // Ticket 12: parts joining or leaving a stored-gap half reframe
            // with it, preserving evaluated poses. The unlink transition
            // skips leaving parts: unadopt above already restored them.
            let unlinking = matches!((old_gap, new_gap), (Some(_), None));
            if let Some(previous) = doc
                .layouts
                .iter()
                .find(|layout| layout.id == incoming.id)
                .cloned()
            {
                layout::reframe_moved_parts(doc, &previous, &incoming, unlinking)?;
            }
            let current = doc
                .layouts
                .iter_mut()
                .find(|layout| layout.id == incoming.id)
                .ok_or("Layout does not exist")?;
            let previous = current.clone();
            *current = incoming.clone();
            // Ticket 21: run the same-board exception lifecycle (Local/Omitted
            // transitions, Rejoin materialization) before resolve adopts the
            // current source state.
            let mut transitioned = layout::apply_exception_transitions(doc, &previous, &incoming)?;
            let mut detached = if was_linked_target {
                layout::unlink_components(doc, &incoming.id)
            } else {
                Vec::new()
            };
            if let Some(source_layout_id) = source_layout_id {
                detached.extend(layout::unlink_components(doc, &source_layout_id));
            }
            layout::resolve(doc)?;
            if !matches!((old_gap, new_gap), (Some(_), None)) {
                transitioned.extend(layout::adopt_half_frames(doc)?);
            }
            transitioned.append(&mut detached);
            Ok([
                [vec![incoming.id.clone()], incoming.matrix_ids.clone()].concat(),
                transitioned,
            ]
            .concat())
        }
        EditOperation::SetSplitGap {
            layout_id,
            split_gap,
        } => {
            if !split_gap.is_finite() || *split_gap < 0.0 {
                return Err("The Split gap must be a finite non-negative distance".into());
            }
            let paired = doc.layouts.iter().any(|layout| {
                layout.id == *layout_id && layout.mirror_link.is_some()
                    || layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == *layout_id)
            });
            if !paired {
                return Err(format!("Layout {layout_id} is not part of a mirrored pair"));
            }
            // A legacy board-coordinate pair adopts the half frame first, so
            // the gap change itself moves the halves without rewriting them.
            let mut changed = layout::adopt_half_frames(doc)?;
            changed.extend(layout::set_pair_gap(doc, layout_id, *split_gap)?);
            layout::resolve(doc)?;
            let root = doc
                .layouts
                .iter()
                .find(|layout| layout.id == *layout_id)
                .and_then(|layout| {
                    layout
                        .mirror_link
                        .as_ref()
                        .map(|link| link.source_id.clone())
                })
                .unwrap_or(layout_id.clone());
            for layout in doc.layouts.iter() {
                let in_pair = layout.id == root
                    || layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == root);
                if in_pair {
                    changed.push(layout.id.clone());
                    for matrix_id in &layout.matrix_ids {
                        changed.push(matrix_id.clone());
                        if let Some(matrix) =
                            doc.matrices.iter().find(|matrix| matrix.id == *matrix_id)
                        {
                            changed.extend(matrix.part_ids.iter().cloned());
                        }
                    }
                }
            }
            changed.sort();
            changed.dedup();
            Ok(changed)
        }
        EditOperation::ConvertOffsetConstraint { constraint_id, to } => {
            constraints::convert_offset(doc, constraint_id, *to)
        }
        EditOperation::SetConstraint { constraint } => {
            if let Some(reason) = matrix::anchors::constraining_refusal(doc, constraint) {
                return Err(reason);
            }
            if let Some(current) = doc
                .constraints
                .iter_mut()
                .find(|item| item.id() == constraint.id())
            {
                *current = constraint.clone();
            } else {
                doc.constraints.push(constraint.clone());
            }
            Ok(vec![constraint.id().into()])
        }
        EditOperation::RemoveConstraint { id } => {
            if !doc.constraints.iter().any(|item| item.id() == id) {
                return Err(format!("Unknown constraint {id}"));
            }
            doc.constraints.retain(|item| item.id() != id);
            Ok(vec![id.clone()])
        }
        EditOperation::ApplyMechanicalEdit { edit } => {
            crate::mechanical::edit::apply_with_ledger(doc, board_contours, edit, identities)
        }
        EditOperation::ApplyOutlineEdit { edit } => crate::outline_edit::apply(doc, edit),
        EditOperation::SetHalfException { .. }
        | EditOperation::SetCompanionPlacement { .. }
        | EditOperation::UnlinkHalves { .. } => {
            Err("Mirrored-half lifecycle edits require prepared transaction dispatch".into())
        }
        EditOperation::RenameProject { name } => {
            validate_project_name(name)?;
            doc.name.clone_from(name);
            Ok(vec![])
        }
        EditOperation::ReplaceDocument { document } => {
            if document.format != "boardstudio/v2" {
                return Err("Unsupported document format".into());
            }
            migrate::require_current_version(document.format_version)?;
            let accepted = doc.clone();
            let mut changed = changed_ids(doc, document);
            let mut prepared = document.as_ref().clone();
            // Replacements, like Open, discard legacy scripts without executing them.
            prepared.scripts.clear();
            layout::resolve(&mut prepared)?;
            generators::document_error(&prepared)?;
            *doc = prepared;
            // A wholesale replacement can add or remove a physical member without the
            // typed pair operations. Reconcile ownership against the accepted state so
            // the replacement can never leave a dangling pair or lose the survivor's
            // visible geometry and construction.
            changed.extend(crate::mechanical::edit::reconcile_physical_topology(
                doc, &accepted,
            )?);
            changed.sort();
            changed.dedup();
            Ok(changed)
        }
    }?;
    // The first committed geometry edit adopts the half frame: legacy
    // board-coordinate pairs store their implied Split gap and re-express
    // unanchored origins as half-frame values, preserving every world pose.
    // SetLayout and SetSplitGap manage their own gap transitions;
    // CreateMirroredPair stores the frame directly; parts adopt with 12.
    if matches!(
        op,
        EditOperation::SetMatrix { .. }
            | EditOperation::SetMatrixSplay { .. }
            | EditOperation::SetMatrixPlacement { .. }
            | EditOperation::AddAnchoredMatrix { .. }
            | EditOperation::SetMatrixAnchor { .. }
            | EditOperation::DetachMatrixAnchor { .. }
            | EditOperation::DetachMatrixAnchorConfirmed { .. }
            | EditOperation::RemoveMatrix { .. }
            | EditOperation::SetMatrixCellsEnabled { .. }
            | EditOperation::RemoveMatrixCells { .. }
            | EditOperation::RemoveMatrixRow { .. }
            | EditOperation::RemoveMatrixColumn { .. }
            | EditOperation::RemoveSelectedMatrixCells { .. }
    ) {
        changed.extend(layout::adopt_half_frames(doc)?);
    }
    Ok(changed)
}
