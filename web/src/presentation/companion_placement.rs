use super::*;

pub(crate) fn companion_placement_projection(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
    context_generation: u64,
    scope_generation: u64,
) -> Option<inspector::CompanionPlacementProjection> {
    let selected = selected?;
    if !selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
    let hardware = document.hardware.as_ref()?;
    let component_context = matches!(&selected.context, objects::TreeContext::Component { .. });
    let (matrix_id, row, column) = match &selected.context {
        objects::TreeContext::Matrix { matrix_id } => (Some(matrix_id.as_str()), None, None),
        objects::TreeContext::Row { matrix_id, row } => {
            (Some(matrix_id.as_str()), Some(*row), None)
        }
        objects::TreeContext::Column { matrix_id, column } => {
            (Some(matrix_id.as_str()), None, Some(*column))
        }
        objects::TreeContext::Key {
            matrix_id,
            row,
            column,
        } => (Some(matrix_id.as_str()), Some(*row), Some(*column)),
        objects::TreeContext::Component {
            matrix_id,
            row,
            column,
            ..
        } => (matrix_id.as_deref(), *row, *column),
        _ => return None,
    };

    let mut matches = Vec::new();
    for pair in &hardware.pairs {
        let Some(halves) = pair.halves.as_ref() else {
            continue;
        };
        if let Some(matrix_id) = matrix_id.filter(|_| !component_context) {
            let linked_matrix = halves.correspondences.iter().any(|link| {
                link.kind == MirroredItemKind::Matrix
                    && (link.source_id == matrix_id || link.mirror_id == matrix_id)
            });
            if !linked_matrix {
                continue;
            }
            let (target, key) = match (row, column) {
                (Some(row), Some(column)) => (None, Some((row, column))),
                (Some(row), None) => (
                    Some(CompanionPlacementTarget::Row {
                        matrix_id: matrix_id.to_owned(),
                        row,
                    }),
                    None,
                ),
                (None, Some(column)) => (
                    Some(CompanionPlacementTarget::Column {
                        matrix_id: matrix_id.to_owned(),
                        column,
                    }),
                    None,
                ),
                (None, None) => (
                    Some(CompanionPlacementTarget::Matrix {
                        matrix_id: matrix_id.to_owned(),
                    }),
                    None,
                ),
            };
            let (eligible, disabled_reason) =
                companion_links_for_selection(document, pair, matrix_id, row, column);
            if eligible.is_empty() && disabled_reason.is_none() {
                continue;
            }
            let targets = if key.is_some() {
                eligible
                    .iter()
                    .map(|link| CompanionPlacementTarget::Item {
                        item_kind: MirroredItemKind::Part,
                        mirror_id: link.mirror_id.clone(),
                    })
                    .collect()
            } else {
                target.into_iter().collect()
            };
            matches.push((pair, targets, eligible, disabled_reason));
        } else {
            let selected_parts = &model.selected_part_ids;
            let mut eligible = Vec::new();
            let mut targets = Vec::new();
            let mut disabled_reason = None;
            for link in halves
                .correspondences
                .iter()
                .filter(|link| link.kind == MirroredItemKind::Part)
            {
                if !selected_parts
                    .iter()
                    .any(|part_id| part_id == &link.source_id || part_id == &link.mirror_id)
                {
                    continue;
                }
                if let Some(reason) = boardstudio_core::companion_placement_disabled_reason(
                    document,
                    &pair.id,
                    &link.source_id,
                ) {
                    disabled_reason = Some(reason);
                    continue;
                }
                if !is_matrix_companion(document, pair, &link.source_id) {
                    continue;
                }
                if !eligible.iter().any(
                    |existing: &&boardstudio_core::model::MirroredCorrespondence| {
                        existing.mirror_id == link.mirror_id
                    },
                ) {
                    eligible.push(link);
                    targets.push(CompanionPlacementTarget::Item {
                        item_kind: MirroredItemKind::Part,
                        mirror_id: link.mirror_id.clone(),
                    });
                }
            }
            if let (Some(matrix_id), Some(row), Some(column)) = (matrix_id, row, column)
                && targets.is_empty()
            {
                let (cell_links, cell_reason) = companion_links_for_selection(
                    document,
                    pair,
                    matrix_id,
                    Some(row),
                    Some(column),
                );
                eligible = cell_links;
                targets.extend(eligible.iter().map(|link| CompanionPlacementTarget::Item {
                    item_kind: MirroredItemKind::Part,
                    mirror_id: link.mirror_id.clone(),
                }));
                disabled_reason = disabled_reason.or(cell_reason);
            }
            if !targets.is_empty() || disabled_reason.is_some() {
                matches.push((pair, targets, eligible, disabled_reason));
            }
        }
    }
    let [(pair, targets, eligible, disabled_reason)] = matches.as_slice() else {
        return None;
    };
    let halves = pair.halves.as_ref()?;
    let modes: Vec<_> = if eligible.is_empty() {
        vec![halves.companion_mode]
    } else {
        eligible
            .iter()
            .map(|link| link.companion_mode.unwrap_or(halves.companion_mode))
            .collect()
    };
    let first_mode = modes.first().copied();
    let mixed_mode = modes.iter().any(|mode| Some(*mode) != first_mode);
    let tweaks: Vec<_> = eligible.iter().map(|link| link.companion_tweak).collect();
    let first_tweak = tweaks.first().copied().unwrap_or(None);
    let mixed_tweak = tweaks.iter().any(|tweak| *tweak != first_tweak);
    let snapshot_token = snapshot.token;
    Some(inspector::CompanionPlacementProjection {
        owner: inspector::CompanionPlacementOwner {
            scope: selected.scope.clone(),
            context: selected.clone(),
            selected_part_ids: model.selected_part_ids.clone(),
            snapshot_token,
            revision: document.revision,
            context_generation,
            scope_generation,
        },
        pair_id: pair.id.clone(),
        targets: targets.clone(),
        mode: if mixed_mode { None } else { first_mode },
        mixed_mode,
        tweak: if mixed_tweak { None } else { first_tweak },
        mixed_tweak,
        disabled_reason: if eligible.is_empty() {
            *disabled_reason
        } else {
            None
        },
    })
}

pub(crate) fn companion_links_for_selection<'a>(
    document: &'a boardstudio_core::model::ProjectDoc,
    pair: &'a boardstudio_core::model::MechanicalMirrorPair,
    selected_matrix_id: &str,
    row: Option<u32>,
    column: Option<u32>,
) -> (
    Vec<&'a boardstudio_core::model::MirroredCorrespondence>,
    Option<&'static str>,
) {
    let Some(halves) = pair.halves.as_ref() else {
        return (Vec::new(), None);
    };
    let matrix_links: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|link| {
            link.kind == MirroredItemKind::Matrix
                && (link.source_id == selected_matrix_id || link.mirror_id == selected_matrix_id)
        })
        .collect();
    let [matrix_link] = matrix_links.as_slice() else {
        return (Vec::new(), None);
    };
    let Some(matrix) = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_link.source_id)
    else {
        return (Vec::new(), None);
    };
    let mut links = Vec::new();
    let mut disabled_reason = None;
    for cell in matrix.cells.iter().filter(|cell| {
        cell.enabled
            && !cell.deleted
            && row.is_none_or(|row| cell.row == row)
            && column.is_none_or(|column| cell.column == column)
    }) {
        let prefix = format!("matrix/{}/r{}c{}/", matrix.id, cell.row, cell.column);
        for assembly in &cell.assemblies {
            let source_id = format!("{prefix}{}", assembly.id);
            let Some(link) = halves
                .correspondences
                .iter()
                .find(|link| link.kind == MirroredItemKind::Part && link.source_id == source_id)
            else {
                continue;
            };
            if let Some(reason) = boardstudio_core::companion_placement_disabled_reason(
                document, &pair.id, &source_id,
            ) {
                disabled_reason = Some(reason);
            } else if !links.iter().any(
                |existing: &&boardstudio_core::model::MirroredCorrespondence| {
                    existing.mirror_id == link.mirror_id
                },
            ) {
                links.push(link);
            }
        }
    }
    (links, disabled_reason)
}

pub(crate) fn is_matrix_companion(
    document: &boardstudio_core::model::ProjectDoc,
    pair: &boardstudio_core::model::MechanicalMirrorPair,
    source_id: &str,
) -> bool {
    companion_links_for_selection(
        document,
        pair,
        &source_matrix_id(pair, source_id).unwrap_or_default(),
        None,
        None,
    )
    .0
    .iter()
    .any(|link| link.source_id == source_id)
}

pub(crate) fn source_matrix_id(
    pair: &boardstudio_core::model::MechanicalMirrorPair,
    source_part_id: &str,
) -> Option<String> {
    let halves = pair.halves.as_ref()?;
    halves
        .correspondences
        .iter()
        .find(|matrix| {
            matrix.kind == MirroredItemKind::Matrix
                && source_part_id.starts_with(&format!("matrix/{}/", matrix.source_id))
        })
        .map(|matrix| matrix.source_id.clone())
}

/// Resolve a Relations reference against the live accepted document. A component opens
/// its Inspector context, as for a finding. A target that has gone, or a request for a
/// session that has moved on, resolves to `None` and nothing is selected.
pub(crate) fn relation_target_request(
    runtime: &Runtime,
    mut request: objects::TreeSelectRequest,
) -> Option<objects::TreeSelectRequest> {
    if runtime.scope().as_ref() != Some(&request.scope) {
        return None;
    }
    let model = runtime.model();
    if let objects::TreeContext::Component {
        part_id: Some(part_id),
        ..
    } = &request.context
    {
        request.context = objects::component_context_for_finding_part(&model, part_id)?;
    }
    if !selection::context_is_current(&model, &request.scope, &request.context) {
        return None;
    }
    // A removed key keeps a resolvable (empty) context, so check the cell itself.
    if let objects::TreeContext::Key {
        matrix_id,
        row,
        column,
    } = &request.context
    {
        let document = &model.accepted.as_ref()?.document;
        let removed = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)?
            .cells
            .iter()
            .any(|cell| cell.row == *row && cell.column == *column && cell.deleted);
        if removed {
            return None;
        }
    }
    Some(request)
}

/// The Relations projection for the selected matrix context, from the accepted document and
/// Core's matrix scene. `None` when nothing is selected, the selection is stale, or the
/// context names no live object of a matrix.
pub(crate) fn matrix_context_relations(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
) -> Option<boardstudio_web_layout::matrix_relations::MatrixRelations> {
    let selected = selected?;
    if !selection::context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let matrix_id = match &selected.context {
        objects::TreeContext::Matrix { matrix_id }
        | objects::TreeContext::Row { matrix_id, .. }
        | objects::TreeContext::Column { matrix_id, .. }
        | objects::TreeContext::Key { matrix_id, .. } => matrix_id,
        objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_id,
        _ => return None,
    };
    let snapshot = model.accepted.as_ref()?;
    let scene = snapshot
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == *matrix_id)?;
    let mut relations = boardstudio_web_layout::matrix_relations::matrix_relations(
        &snapshot.document,
        scene,
        &selected.context,
        &snapshot.scene.matrix_scenes,
    )?;
    // The chain covers one matrix context only: selected parts in other
    // matrices get an explanatory note, never a merged frame.
    relations.selection_note = boardstudio_web_layout::matrix_relations::cross_matrix_note(
        &snapshot.scene.matrix_scenes,
        &selected.context,
        &model.selected_part_ids,
    );
    Some(relations)
}

pub(crate) fn layout_relationship_summary(
    document: &boardstudio_core::model::ProjectDoc,
    board: &boardstudio_core::model::Board,
    active_layout: Option<&boardstudio_core::model::Layout>,
    active_part: Option<&Part>,
) -> String {
    let paired_layout = active_layout.and_then(|layout| {
        layout
            .mirror_link
            .as_ref()
            .and_then(|link| {
                document.layouts.iter().find(|candidate| {
                    candidate.board_id == board.id && candidate.id == link.source_id
                })
            })
            .or_else(|| {
                document.layouts.iter().find(|candidate| {
                    candidate.board_id == board.id
                        && candidate
                            .mirror_link
                            .as_ref()
                            .is_some_and(|link| link.source_id == layout.id)
                })
            })
    });
    if let Some(partner) = paired_layout {
        return format!(
            "Key assemblies, diodes and components mirror with {}. Replace a component on one half to keep it local.",
            partner.name
        );
    }
    if active_layout.is_some() {
        return "This layout is independent. Its geometry and components can be edited separately."
            .to_owned();
    }
    let driven =
        active_part.and_then(
            |target| match document.constraint_driving_part(&target.id) {
                Ok(Some(constraint)) => {
                    let source = document
                        .parts
                        .iter()
                        .find(|part| part.id == constraint.source())
                        .map(|part| part.reference.as_str())
                        .unwrap_or(constraint.source());
                    let driven = document
                        .parts
                        .iter()
                        .find(|part| part.id == constraint.target())
                        .map(|part| part.reference.as_str())
                        .unwrap_or(constraint.target());
                    Some(format!("{source} drives {driven}."))
                }
                Ok(None) => None,
                Err(_) => {
                    Some("The saved placement relationship could not be verified.".to_owned())
                }
            },
        );
    driven.unwrap_or_else(|| "No saved placement relationship on this selection.".to_owned())
}

/// Ticket 13: the Relationships lead for an anchored part: its anchor key and
/// Follow subset, ahead of the mirror-pairing summary.
pub(crate) fn part_anchor_summary(
    document: &boardstudio_core::model::ProjectDoc,
    part: &boardstudio_core::model::Part,
) -> Option<String> {
    let anchor = part.anchor.as_ref()?;
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == anchor.matrix_id)?;
    let name = matrix.name.clone().unwrap_or(matrix.id.clone());
    let mut follows = Vec::new();
    if anchor.follow.x {
        follows.push("X");
    }
    if anchor.follow.y {
        follows.push("Y");
    }
    if anchor.follow.rotation {
        follows.push("rotation");
    }
    let follows = if follows.is_empty() {
        "none".to_owned()
    } else {
        follows.join(", ")
    };
    Some(format!(
        "Anchored to {} R{}C{}, following {}.",
        name,
        anchor.row + 1,
        anchor.column + 1,
        follows
    ))
}
