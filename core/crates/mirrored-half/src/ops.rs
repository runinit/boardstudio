use crate::model::{EditOperation, HalfExceptionState, Pose2, ProjectDoc};
use crate::{MirrorOverride, MirroredPair, PartOverride, project_document};
use boardstudio_model::mirrored_half::{
    mirrored_entity_id, mirrored_part_id, owns_matrix, owns_part,
};

#[derive(Debug, PartialEq, Eq)]
pub enum MirrorEditRefused {
    LinkedTarget {
        target_id: String,
        source_id: String,
        pair_id: String,
    },
}

impl std::fmt::Display for MirrorEditRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self::LinkedTarget {
            target_id,
            source_id,
            pair_id,
        } = self;
        write!(
            f,
            "Mirror {target_id} in pair {pair_id} is linked: edit {source_id} or make the mirror local"
        )
    }
}

pub fn admit_edit(doc: &ProjectDoc, operation: &EditOperation) -> Result<(), MirrorEditRefused> {
    use EditOperation::*;

    let mut part_targets = Vec::new();
    let mut matrix_targets = Vec::new();
    match operation {
        MoveParts { positions } => {
            part_targets.extend(positions.iter().map(|position| position.id.as_str()))
        }
        RemoveParts { ids } => part_targets.extend(ids.iter().map(String::as_str)),
        SetInputScanMode { part_id, .. }
        | SetMirroredPart { part_id, .. }
        | SetPartAnchor { part_id, .. }
        | SetPartAnchorFollow { part_id, .. }
        | DetachPartAnchor { part_id }
        | SetPartAnchorValues { part_id, .. }
        | SetPartRotation { part_id, .. } => part_targets.push(part_id.as_str()),
        SetKeyBinding { key_id, .. } | SetKeycapKey { key_id, .. } => {
            part_targets.push(key_id.as_str())
        }
        AddPart { part, .. } | AddMirrorOnlyPart { part, .. } => {
            part_targets.push(part.id.as_str())
        }
        SetKeycapSize {
            sizes,
            moves,
            offsets,
        } => {
            part_targets.extend(sizes.keys().map(String::as_str));
            part_targets.extend(moves.iter().map(|position| position.id.as_str()));
            matrix_targets.extend(offsets.iter().map(|offset| offset.matrix_id.as_str()));
        }
        SetMatrix { matrix, .. } => matrix_targets.push(matrix.id.as_str()),
        SetMatrixCellsEnabled { cells, .. } | RemoveSelectedMatrixCells { cells } => {
            matrix_targets.extend(cells.keys().map(String::as_str))
        }
        RemoveMatrix { id: matrix_id }
        | SetMatrixKeycaps { matrix_id, .. }
        | RemoveMatrixCells { matrix_id, .. }
        | RemoveMatrixRow { matrix_id, .. }
        | RemoveMatrixColumn { matrix_id, .. }
        | SetMatrixSplay { matrix_id, .. }
        | SetMatrixPlacement { matrix_id, .. }
        | SetMatrixAnchor { matrix_id, .. }
        | DetachMatrixAnchor { matrix_id, .. }
        | DetachMatrixAnchorConfirmed { matrix_id, .. } => matrix_targets.push(matrix_id.as_str()),
        ApplyMechanicalEdit { .. }
        | ApplyOutlineEdit { .. }
        | SetMountedModule { .. }
        | RemoveMountedModule { .. } => {
            return admit_geometry_edit(doc, operation);
        }
        _ => return Ok(()),
    }
    for mirror in &doc.mirrored_pairs {
        for source in &doc.parts {
            if part_targets.is_empty() || !owns_part(doc, &mirror.source, &source.id) {
                continue;
            }
            let target_id = mirrored_part_id(doc, mirror, &source.id);
            if part_targets.contains(&target_id.as_str())
                && !mirror.overrides.iter().any(|item| {
                    item.source_id == source.id && matches!(item.state, MirrorOverride::Local(_))
                })
            {
                return Err(MirrorEditRefused::LinkedTarget {
                    target_id,
                    source_id: source.id.clone(),
                    pair_id: mirror.id.clone(),
                });
            }
        }
        for source in &doc.matrices {
            if matrix_targets.is_empty() || !owns_matrix(doc, &mirror.source, source) {
                continue;
            }
            let target_id = mirrored_entity_id(mirror, "matrix", &source.id);
            if matrix_targets.contains(&target_id.as_str()) {
                return Err(MirrorEditRefused::LinkedTarget {
                    target_id,
                    source_id: source.id.clone(),
                    pair_id: mirror.id.clone(),
                });
            }
        }
    }
    Ok(())
}

pub fn create_pair(doc: &mut ProjectDoc, pair: MirroredPair) -> Result<Vec<String>, String> {
    if doc
        .mirrored_pairs
        .iter()
        .any(|existing| existing.id == pair.id)
    {
        return Err(format!("Duplicate mirror pair {}", pair.id));
    }
    if pair.source == pair.destination {
        return Err("Mirror source and destination must differ".into());
    }
    let changed = vec![pair.id.clone()];
    doc.mirrored_pairs.push(pair);
    Ok(changed)
}

pub fn set_exception(
    doc: &mut ProjectDoc,
    pair_id: &str,
    target_id: &str,
    state: HalfExceptionState,
) -> Result<Vec<String>, String> {
    let index = doc
        .mirrored_pairs
        .iter()
        .position(|pair| pair.id == pair_id)
        .ok_or("Unknown mirror pair")?;
    let mirror = &doc.mirrored_pairs[index];
    let source_id = doc
        .parts
        .iter()
        .map(|part| &part.id)
        .chain(mirror.overrides.iter().map(|item| &item.source_id))
        .find(|source_id| {
            mirrored_part_id(doc, mirror, source_id) == target_id
            || mirror.overrides.iter().any(|item| &item.source_id == *source_id
                && matches!(&item.state, MirrorOverride::Local(part) if part.id == target_id))
        })
        .cloned()
        .ok_or("Unknown mirror target")?;
    let replacement = match state {
        HalfExceptionState::Local => {
            let resolved = project_document(doc);
            let part = resolved
                .geometry()
                .parts
                .iter()
                .find(|part| part.id == target_id)
                .ok_or("Missing projected part")?;
            Some(MirrorOverride::Local(Box::new(part.clone())))
        }
        HalfExceptionState::Omitted => {
            let resolved = project_document(doc);
            let view = resolved.geometry();
            if let Some(constraint) = doc.constraints.iter().find(|constraint| {
                constraint.source() == target_id || constraint.target() == target_id
            }) {
                return Err(format!(
                    "Mirror target is used by constraint {}",
                    constraint.id()
                ));
            }
            for net in view
                .nets
                .iter()
                .filter(|net| net.pins.iter().any(|pin| pin.part_id == target_id))
            {
                if let Some(trace) = view
                    .boards
                    .iter()
                    .flat_map(|board| &board.traces)
                    .find(|trace| trace.net_id.as_ref() == Some(&net.id))
                {
                    return Err(format!("Mirror target is used by trace {}", trace.id));
                }
                if let Some(via) = view
                    .boards
                    .iter()
                    .flat_map(|board| &board.vias)
                    .find(|via| via.net_id.as_ref() == Some(&net.id))
                {
                    return Err(format!("Mirror target is used by via {}", via.id));
                }
            }
            Some(MirrorOverride::Omitted)
        }
        HalfExceptionState::Linked => None,
    };
    let overrides = &mut doc.mirrored_pairs[index].overrides;
    overrides.retain(|item| item.source_id != source_id);
    if let Some(state) = replacement {
        overrides.push(PartOverride { source_id, state });
    }
    Ok(vec![target_id.into()])
}

pub fn set_companion_pose(
    doc: &mut ProjectDoc,
    pair_id: &str,
    target_id: &str,
    pose: Pose2,
) -> Result<Vec<String>, String> {
    let index = doc
        .mirrored_pairs
        .iter()
        .position(|pair| pair.id == pair_id)
        .ok_or("Unknown mirror pair")?;
    let mirror = &doc.mirrored_pairs[index];
    let source_id = doc
        .parts
        .iter()
        .map(|part| &part.id)
        .find(|id| mirrored_part_id(doc, mirror, id) == target_id)
        .cloned()
        .ok_or("Unknown mirror target")?;
    let mirror = &mut doc.mirrored_pairs[index];
    if mirror.overrides.iter().any(|item| {
        item.source_id == source_id
            && matches!(
                item.state,
                MirrorOverride::Local(_) | MirrorOverride::Omitted
            )
    }) {
        return Err("The mirror companion must be linked".into());
    }
    mirror.overrides.retain(|item| item.source_id != source_id);
    mirror.overrides.push(PartOverride {
        source_id,
        state: MirrorOverride::CompanionPose(pose),
    });
    Ok(vec![target_id.into()])
}

pub fn unlink_pair(doc: &mut ProjectDoc, pair_id: &str) -> Result<Vec<String>, String> {
    let index = doc
        .mirrored_pairs
        .iter()
        .position(|pair| pair.id == pair_id)
        .ok_or("Unknown mirror pair")?;
    let mut isolated = doc.clone();
    isolated.mirrored_pairs = vec![doc.mirrored_pairs[index].clone()];
    let resolved = project_document(&isolated);
    let projected = resolved.geometry();
    doc.parts.clone_from(&projected.parts);
    doc.matrices.clone_from(&projected.matrices);
    doc.nets.clone_from(&projected.nets);
    doc.outline.clone_from(&projected.outline);
    doc.keycaps.clone_from(&projected.keycaps);
    doc.modules.clone_from(&projected.modules);
    doc.hardware.clone_from(&projected.hardware);
    doc.boards.clone_from(&projected.boards);
    doc.layouts.clone_from(&projected.layouts);
    doc.mirrored_pairs.remove(index);
    Ok(vec![pair_id.into()])
}

pub fn create_half(
    doc: &mut ProjectDoc,
    pair_id: &str,
    source_instance_id: &str,
    mode: &crate::model::MirroredHalfMode,
    labels: Option<&crate::model::MirroredHalfLabels>,
    setup: Option<&crate::model::MirroredHalfPhysicalSetup>,
    mapping: Pose2,
) -> Result<Vec<String>, String> {
    use crate::MirrorDestination;
    use crate::model::MirroredHalfMode;

    let mut hardware = doc.hardware.clone().unwrap_or_default();
    if let Some(setup) = setup {
        hardware.topology = setup.topology;
        hardware.transport = setup.transport;
        hardware.shared_construction = Some(setup.shared_construction.clone());
        hardware
            .instances
            .retain(|instance| instance.id != setup.source_instance.id);
        hardware.instances.push(setup.source_instance.clone());
    }
    let source = hardware
        .instances
        .iter_mut()
        .find(|instance| instance.id == source_instance_id)
        .ok_or("Unknown source physical instance")?;
    if let Some(label) = labels.and_then(|labels| labels.source.as_ref()) {
        label_instance(source, label);
    }
    let mut destination = source.clone();
    let source_board = source.board_id.clone();
    let target_instance_id = match mode {
        MirroredHalfMode::NewBoard { instance_id, .. }
        | MirroredHalfMode::ReuseBoard { instance_id } => instance_id,
    };
    if hardware
        .instances
        .iter()
        .any(|instance| instance.id == *target_instance_id)
    {
        return Err(format!("Duplicate physical instance {target_instance_id}"));
    }
    let mut changed = vec![];
    match mode {
        MirroredHalfMode::NewBoard {
            board_id,
            instance_id,
        } => {
            destination.id.clone_from(instance_id);
            destination.board_id.clone_from(board_id);
            destination.mechanical = None;
            destination.controller_part_id = None;
            if doc.boards.iter().any(|board| board.id == *board_id) {
                return Err(format!("Duplicate board {board_id}"));
            }
            let mut board = doc
                .boards
                .iter()
                .find(|board| board.id == source_board)
                .cloned()
                .ok_or("Unknown source board")?;
            board.id.clone_from(board_id);
            board.part_ids.clear();
            board.outline_ids.clear();
            board.net_ids.clear();
            board.traces.clear();
            board.vias.clear();
            changed.extend(create_pair(
                doc,
                MirroredPair {
                    id: pair_id.into(),
                    source: MirrorDestination::SecondBoard(source_board),
                    destination: MirrorDestination::SecondBoard(board_id.clone()),
                    mapping,
                    overrides: vec![],
                },
            )?);
            doc.boards.push(board);
            changed.push(board_id.clone());
        }
        MirroredHalfMode::ReuseBoard { instance_id } => {
            destination.id.clone_from(instance_id);
            destination.flipped = !destination.flipped;
            destination.controller_part_id = None;
            destination.construction_ownership =
                crate::model::MechanicalConstructionOwnership::Local;
        }
    }
    if let Some(label) = labels.and_then(|labels| labels.mirror.as_ref()) {
        label_instance(&mut destination, label);
    }
    changed.push(destination.id.clone());
    hardware.instances.push(destination);
    doc.hardware = Some(hardware);
    Ok(changed)
}

fn label_instance(
    instance: &mut crate::model::PhysicalBoardInstance,
    label: &crate::model::MirroredHalfLabel,
) {
    if let Some(name) = &label.name {
        instance.name.clone_from(name);
    }
    if let Some(half) = &label.half {
        instance.half.clone_from(half);
    }
    if let Some(role) = &label.role {
        instance.role.clone_from(role);
    }
}

pub fn local_part_mut<'a>(
    doc: &'a mut ProjectDoc,
    target_id: &str,
) -> Option<&'a mut crate::model::Part> {
    doc.mirrored_pairs
        .iter_mut()
        .flat_map(|pair| &mut pair.overrides)
        .find_map(|item| match &mut item.state {
            MirrorOverride::Local(part) if part.id == target_id => Some(part.as_mut()),
            _ => None,
        })
}

pub fn companion_placement_disabled_reason(
    doc: &ProjectDoc,
    pair_id: &str,
    item_id: &str,
) -> Option<&'static str> {
    let Some(pair) = doc.mirrored_pairs.iter().find(|pair| pair.id == pair_id) else {
        return Some("Unknown derived mirror pair");
    };
    let source = doc.parts.iter().find(|part| {
        owns_part(doc, &pair.source, &part.id)
            && (part.id == item_id || mirrored_part_id(doc, pair, &part.id) == item_id)
    });
    let Some(source) = source else {
        return Some("Unknown derived mirror companion");
    };
    if pair.overrides.iter().any(|item| {
        item.source_id == source.id
            && matches!(
                item.state,
                MirrorOverride::Local(_) | MirrorOverride::Omitted
            )
    }) {
        return Some("The mirror companion must be linked");
    }
    None
}

fn admit_geometry_edit(
    doc: &ProjectDoc,
    operation: &EditOperation,
) -> Result<(), MirrorEditRefused> {
    use crate::MirrorDestination;
    use boardstudio_model::mechanical::change::MechanicalTarget;
    use boardstudio_model::outline_edit::OutlineChange;

    let (board, instance, module, outline) = match operation {
        EditOperation::ApplyMechanicalEdit { edit } => match &edit.target {
            MechanicalTarget::Board { board_id } => (Some(board_id.as_str()), None, None, None),
            MechanicalTarget::Instance { instance_id } => (
                doc.hardware
                    .iter()
                    .flat_map(|hw| &hw.instances)
                    .find(|item| &item.id == instance_id)
                    .map(|item| item.board_id.as_str()),
                Some(instance_id.as_str()),
                None,
                None,
            ),
        },
        EditOperation::ApplyOutlineEdit { edit } => {
            let outline = match &edit.change {
                OutlineChange::AddFeature { feature } | OutlineChange::SetFeature { feature } => {
                    Some(feature.id())
                }
                OutlineChange::RemoveFeature { feature_id } => Some(feature_id.as_str()),
                _ => None,
            };
            (Some(edit.target.board_id()), None, None, outline)
        }
        EditOperation::SetMountedModule { instance, .. } => (
            Some(instance.host_board_id.as_str()),
            None,
            Some(instance.id.as_str()),
            None,
        ),
        EditOperation::RemoveMountedModule { id } => (None, None, Some(id.as_str()), None),
        _ => return Ok(()),
    };

    for pair in &doc.mirrored_pairs {
        let (MirrorDestination::SecondBoard(source), MirrorDestination::SecondBoard(target)) =
            (&pair.source, &pair.destination)
        else {
            continue;
        };
        let refusal = |target_id: &str, source_id: &str| {
            Err(MirrorEditRefused::LinkedTarget {
                target_id: target_id.into(),
                source_id: source_id.into(),
                pair_id: pair.id.clone(),
            })
        };
        if let Some(id) = module
            && let Some(authored) = doc.modules.iter().find(|item| {
                &item.host_board_id == source && mirrored_entity_id(pair, "module", &item.id) == id
            })
        {
            return refusal(id, &authored.id);
        }
        if let Some(id) = outline
            && let Some(authored) =
                doc.boards
                    .iter()
                    .find(|item| &item.id == source)
                    .and_then(|item| {
                        item.outline_ids
                            .iter()
                            .find(|authored| mirrored_entity_id(pair, "outline", authored) == id)
                    })
        {
            return refusal(id, authored);
        }
        if board == Some(target.as_str()) {
            if let Some(id) = instance {
                let source_instance = doc
                    .hardware
                    .iter()
                    .flat_map(|hw| &hw.instances)
                    .find(|item| &item.board_id == source);
                return refusal(
                    id,
                    source_instance.map_or(source.as_str(), |item| item.id.as_str()),
                );
            }
            return refusal(target, source);
        }
    }
    Ok(())
}

pub fn local_pair_id(doc: &ProjectDoc, target_id: &str) -> Option<String> {
    doc.mirrored_pairs
        .iter()
        .find(|pair| {
            pair.overrides.iter().any(
                |item| matches!(&item.state, MirrorOverride::Local(part) if part.id == target_id),
            )
        })
        .map(|pair| pair.id.clone())
}
