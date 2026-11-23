use super::preview::changed_ids;
use super::*;

pub(super) fn validate_project_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Project name cannot be empty".into());
    }
    Ok(())
}

pub(crate) fn require_board<'a>(doc: &'a ProjectDoc, board_id: &str) -> Result<&'a Board, String> {
    doc.boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| format!("Unknown board {board_id}"))
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
            | EditOperation::PlaceAssembly { .. }
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

fn merge_assembly_assets(doc: &mut ProjectDoc, assets: &[Asset]) -> Result<(), String> {
    for asset in assets {
        match doc.assets.iter().find(|existing| existing.id == asset.id) {
            Some(existing) if existing != asset => {
                return Err(format!(
                    "The model asset identity '{}' conflicts with the accepted project.",
                    asset.id
                ));
            }
            Some(_) => {}
            None => doc.assets.push(asset.clone()),
        }
    }
    Ok(())
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
    match op {
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
            require_board(doc, board_id)?;
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
                if let Some(part) = mirrored_half::local_part_mut(doc, &position.id) {
                    if part.locked == Some(true) {
                        return Err(format!("Part {} is locked", position.id));
                    }
                    if !position.at.x.is_finite() || !position.at.y.is_finite() {
                        return Err("Position must be finite".into());
                    }
                    part.pose.at = position.at;
                    changed.push(part.id.clone());
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
                let at = position.at;
                // Ticket 13: an anchored part re-solves Spread/Stagger from
                // the clamped drop instead of writing the pose.
                if doc.parts[index].anchor.is_some() {
                    changed.extend(matrix::part_anchors::move_anchored(doc, &position.id, at)?);
                    continue;
                }
                match constraints::retune_offset(doc, &position.id, at, None)? {
                    Some(constraints::Retune::Mirror) => {
                        return Err(constraints::mirror_drag_refusal(&position.id));
                    }
                    Some(constraints::Retune::Applied) => {
                        changed.push(position.id.clone());
                        continue;
                    }
                    None => {}
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
        EditOperation::AddPart {
            part,
            board_id,
            definition,
            layout_id,
        } => add_part_placement(
            doc,
            part,
            board_id.as_deref(),
            definition.as_ref(),
            layout_id.as_deref(),
        ),
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
        EditOperation::SetMirroredPart { .. } => {
            Err("Use a mirror exception to change derived part ownership".into())
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
            if let Some(part) = mirrored_half::local_part_mut(doc, part_id) {
                if !rotation.is_finite() || part.locked == Some(true) {
                    return Err(
                        "Local mirror rotation requires an unlocked part and finite angle".into(),
                    );
                }
                part.pose.rotation = rotation.rem_euclid(360.0);
                return Ok(vec![part_id.clone()]);
            }
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
            let mut authored = Vec::new();
            let mut omitted = Vec::new();
            for id in ids {
                if let Some(pair_id) = mirrored_half::local_pair_id(doc, id) {
                    omitted.extend(mirrored_half::set_exception(
                        doc,
                        &pair_id,
                        id,
                        HalfExceptionState::Omitted,
                    )?);
                } else {
                    authored.push(id.clone());
                }
            }
            let ids = &authored;
            let affected: Vec<_> = doc
                .matrices
                .iter()
                .filter(|matrix| matrix.part_ids.iter().any(|id| ids.contains(id)))
                .map(|matrix| matrix.id.clone())
                .collect();
            let ids = ids.clone();
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
            let mut changed = ids;
            changed.extend(omitted);
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
            let mut changed = apply_unscripted(
                doc,
                &EditOperation::SetMatrix {
                    matrix: matrix.as_ref().clone(),
                    definitions: definitions.clone(),
                },
            )?;
            let mut target = right.clone();
            target.mirror_link = None;
            target.matrix_ids.clear();
            doc.layouts.extend([left.clone(), target]);
            changed.extend(mirrored_half::create_pair(
                doc,
                mirrored_half::MirroredPair {
                    id: right.id.clone(),
                    source: mirrored_half::MirrorDestination::SameBoardLayout(left.id.clone()),
                    destination: mirrored_half::MirrorDestination::SameBoardLayout(
                        right.id.clone(),
                    ),
                    mapping: layout::axis_mapping(link.axis_x),
                    overrides: vec![],
                },
            )?);
            Ok(changed)
        }
        EditOperation::CreateMirroredHalf { .. } => {
            apply_create_mirrored_half(doc, board_contours, op, identities)
        }
        EditOperation::SetLayout { layout: incoming } => {
            if incoming.mirror_link.is_some() {
                return Err("Layout mirror links are unsupported; use a mirrored pair".into());
            }
            let current = doc
                .layouts
                .iter_mut()
                .find(|layout| layout.id == incoming.id)
                .ok_or("Layout does not exist")?;
            if current.board_id != incoming.board_id || current.matrix_ids != incoming.matrix_ids {
                return Err("Layout ownership cannot be reassigned".into());
            }
            *current = incoming.clone();
            layout::validate(doc)?;
            Ok(std::iter::once(incoming.id.clone())
                .chain(incoming.matrix_ids.clone())
                .collect())
        }
        EditOperation::SetSplitGap { .. } => {
            Err("Layout mirror links are unsupported; use a mirrored pair".into())
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
        EditOperation::SetHalfException {
            pair_id,
            mirror_id,
            state,
            ..
        } => mirrored_half::set_exception(doc, pair_id, mirror_id, *state),
        EditOperation::UnlinkHalves { pair_id } => mirrored_half::unlink_pair(doc, pair_id),
        EditOperation::SetCompanionPlacement {
            pair_id,
            targets,
            tweak,
            ..
        } => {
            let mut changed = Vec::new();
            for target in targets {
                if let (CompanionPlacementTarget::Item { mirror_id, .. }, Some(tweak)) =
                    (target, tweak)
                {
                    changed.extend(mirrored_half::set_companion_pose(
                        doc,
                        pair_id,
                        mirror_id,
                        Pose2 {
                            at: Vec2 {
                                x: tweak.x,
                                y: tweak.y,
                            },
                            rotation: tweak.rotation,
                        },
                    )?);
                }
            }
            Ok(changed)
        }
        EditOperation::UpsertAssembly {
            assembly,
            definitions,
            assets,
        } => {
            if assembly.id.trim().is_empty()
                || assembly.name.trim().is_empty()
                || assembly.members.is_empty()
            {
                return Err("Name the assembly and add at least one member".into());
            }
            let mut members = BTreeSet::new();
            if assembly
                .members
                .iter()
                .any(|member| member.id.trim().is_empty() || !members.insert(&member.id))
            {
                return Err("Assembly members must have unique, non-empty identities.".into());
            }
            merge_assembly_assets(doc, assets)?;
            for definition in definitions {
                if !doc
                    .definitions
                    .iter()
                    .any(|existing| existing.id == definition.id)
                {
                    doc.definitions.push(definition.clone());
                }
            }
            if let Some(existing) = doc.assemblies.iter_mut().find(|a| a.id == assembly.id) {
                existing.clone_from(assembly);
            } else {
                doc.assemblies.push(assembly.as_ref().clone());
            }
            Ok(vec![assembly.id.clone()])
        }
        EditOperation::PlaceAssembly {
            board_id,
            placement_id,
            definitions,
            parts,
            assets,
        } => {
            require_board(doc, board_id)?;
            if placement_id.trim().is_empty() || parts.is_empty() {
                return Err("An assembly placement needs an identity and at least one part".into());
            }
            merge_assembly_assets(doc, assets)?;
            for definition in definitions {
                if definition.id.trim().is_empty()
                    || doc
                        .definitions
                        .iter()
                        .any(|existing| existing.id == definition.id)
                {
                    return Err(format!("Duplicate assembly definition {}", definition.id));
                }
                doc.definitions.push(definition.clone());
            }
            let mut changed = Vec::with_capacity(parts.len() + 1);
            for part in parts {
                if part.id.trim().is_empty() || part.reference.trim().is_empty() {
                    return Err("Assembly parts need an identity and component reference".into());
                }
                if doc
                    .parts
                    .iter()
                    .any(|existing| existing.reference == part.reference)
                {
                    return Err(format!(
                        "The component reference '{}' is already in use.",
                        part.reference
                    ));
                }
                let mut part = part.clone();
                part.properties.get_or_insert_with(Default::default).insert(
                    "assemblyId".into(),
                    serde_json::Value::String(placement_id.clone()),
                );
                changed.extend(add_part_placement(doc, &part, Some(board_id), None, None)?);
            }
            changed.push(board_id.clone());
            Ok(changed)
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
            document_format::require_current_version(document.format_version)?;
            if document
                .layouts
                .iter()
                .any(|layout| layout.mirror_link.is_some())
            {
                return Err("Layout mirror links are unsupported; use a mirrored pair".into());
            }
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
    }
}

fn feature_correspondences(
    configuration: &MechanicalConfiguration,
) -> Vec<crate::mechanical::change::MechanicalFeatureCorrespondence> {
    let same = |kind: MechanicalFeatureKind, id: &str| {
        crate::mechanical::change::MechanicalFeatureCorrespondence {
            kind,
            first_feature_id: id.into(),
            second_feature_id: id.into(),
            source_instance_id: None,
        }
    };
    configuration
        .mounts
        .iter()
        .map(|mount| same(MechanicalFeatureKind::Mount, &mount.id))
        .chain(
            configuration
                .closure_mounts
                .iter()
                .flatten()
                .map(|mount| same(MechanicalFeatureKind::ClosureMount, &mount.id)),
        )
        .chain(
            configuration
                .openings
                .iter()
                .flatten()
                .map(|opening| same(MechanicalFeatureKind::Opening, &opening.id)),
        )
        .chain(
            configuration
                .gasket_layout
                .iter()
                .flat_map(|layout| &layout.supports)
                .map(|support| same(MechanicalFeatureKind::GasketSupport, &support.id)),
        )
        .collect()
}

fn apply_create_mirrored_half(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
    identities: &mechanical::edit::MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    let EditOperation::CreateMirroredHalf {
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
    } = op
    else {
        unreachable!("apply_create_mirrored_half requires CreateMirroredHalf");
    };
    let MirroredIdRule::BoardPrefix = id_rule;
    crate::mechanical::halves::validate_companion(*companion_mode, *companion_tweak)?;
    if !(mapping.at.x.is_finite() && mapping.at.y.is_finite() && mapping.rotation.is_finite()) {
        return Err("The mirrored half's mapping must be finite.".into());
    }
    if matches!(mode, MirroredHalfMode::ReuseBoard { .. }) {
        if companion_tweak.is_some() {
            return Err(
                "reuse-board copies no companions; companionTweak requires new-board".into(),
            );
        }
        if mapping.at.x != 0.0 || mapping.at.y != 0.0 || mapping.rotation != 0.0 {
            return Err(
                "A flipped instance of a reused board must use zero translation and zero rotation."
                    .into(),
            );
        }
    }
    if let Some(setup) = physical_setup {
        if setup.topology != HardwareTopology::Split
            || !matches!(
                setup.transport,
                HardwareTransport::Wired | HardwareTransport::Wireless
            )
        {
            return Err(
                "Mirrored physical setup requires split topology and wired or wireless transport."
                    .into(),
            );
        }
        let source = &setup.source_instance;
        if source.id != *source_instance_id
            || source.board_id != setup.shared_construction.board_id
            || !doc.boards.iter().any(|board| board.id == source.board_id)
        {
            return Err(
                "Physical setup source and shared construction must belong to a configured board."
                    .into(),
            );
        }
    }
    let mut next = doc.clone();
    let mut changed = mirrored_half::create_half(
        &mut next,
        pair_id,
        source_instance_id,
        mode,
        labels.as_ref(),
        physical_setup.as_ref(),
        *mapping,
    )?;
    let source_board_id = next
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .instances
                .iter()
                .find(|instance| instance.id == *source_instance_id)
        })
        .map(|instance| instance.board_id.clone())
        .ok_or("Unknown source physical instance")?;
    let (mirror_instance_id, mirror_board_id, is_new_board) = match mode {
        MirroredHalfMode::NewBoard {
            board_id,
            instance_id,
        } => (instance_id.as_str(), board_id.clone(), true),
        MirroredHalfMode::ReuseBoard { instance_id } => {
            (instance_id.as_str(), source_board_id.clone(), false)
        }
    };
    if let Ok(source_inputs) = crate::mechanical::inputs::effective_inputs(
        &next,
        &[],
        &crate::mechanical::MechanicalTarget::Instance {
            instance_id: source_instance_id.to_owned(),
        },
    ) {
        if let Some(mirror_instance) = next.hardware.as_mut().and_then(|hardware| {
            hardware
                .instances
                .iter_mut()
                .find(|instance| instance.id == mirror_instance_id)
        }) && mirror_instance.mechanical.is_none()
        {
            let mut cfg = source_inputs.configuration;
            cfg.board_id.clone_from(&mirror_board_id);
            if is_new_board {
                cfg.hardware = None;
                cfg.critical_fits = None;
                cfg.battery = None;
            }
            mirror_instance.mechanical = Some(cfg);
        }
        let mirror_inputs = crate::mechanical::inputs::effective_inputs(
            &next,
            &[],
            &crate::mechanical::MechanicalTarget::Instance {
                instance_id: mirror_instance_id.to_owned(),
            },
        )?;
        let edit = crate::mechanical::MechanicalEdit {
            target: crate::mechanical::MechanicalTarget::Instance {
                instance_id: source_instance_id.to_owned(),
            },
            change: crate::mechanical::MechanicalChange::LinkMirrorPair {
                pair_id: pair_id.to_owned(),
                members: [source_instance_id.to_owned(), mirror_instance_id.to_owned()],
                mapping: *mapping,
                construction_source: source_instance_id.to_owned(),
                correspondences: feature_correspondences(&mirror_inputs.configuration),
            },
            mounting_hole: mounting_hole.clone(),
        };
        changed.extend(crate::mechanical::edit::apply_with_ledger(
            &mut next,
            board_contours,
            &edit,
            identities,
        )?);
        if !is_new_board
            && let Some(pair) = next
                .hardware
                .as_mut()
                .and_then(|hardware| hardware.pairs.iter_mut().find(|pair| pair.id == *pair_id))
        {
            pair.halves = Some(MirroredHalves {
                companion_mode: *companion_mode,
                source_board_id,
                mirror_board_id,
                correspondences: vec![],
                exceptions: vec![],
            });
        }
    }
    layout::validate(&next)?;
    crate::mechanical::halves::validate_document(&next)?;
    changed.sort();
    changed.dedup();
    *doc = next;
    Ok(changed)
}
