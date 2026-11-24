use super::inspector_types as inspector;
use boardstudio_application::ReadModel;
use boardstudio_model::mirrored_half::{self, MirroredPair};
use boardstudio_model::model::{
    CompanionPlacementTarget, MirroredCompanionMode, MirroredCompanionTweak, MirroredItemKind,
    Part, ProjectDoc,
};
use boardstudio_web_ui_model::tree as objects;

/// One linked companion through a derived pair: the authored source id and
/// its derived mirror id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanionLink {
    pub source_id: String,
    pub mirror_id: String,
}

/// The pair's default companion placement: the explicit halves record when
/// the mechanical pair carries one, otherwise a literal reflection.
fn pair_default_mode(document: &ProjectDoc, pair: &MirroredPair) -> MirroredCompanionMode {
    document
        .hardware
        .iter()
        .flat_map(|hardware| &hardware.pairs)
        .find(|candidate| candidate.id == pair.id)
        .and_then(|candidate| candidate.halves.as_ref())
        .map(|halves| halves.companion_mode)
        .unwrap_or_default()
}

/// The sparse per-item tweak covering one source, if its override carries a
/// companion pose.
fn item_tweak(pair: &MirroredPair, source_id: &str) -> Option<MirroredCompanionTweak> {
    match mirrored_half::override_of(pair, source_id) {
        Some(boardstudio_model::mirrored_half::MirrorOverride::CompanionPose(pose)) => {
            Some(MirroredCompanionTweak {
                x: pose.at.x,
                y: pose.at.y,
                rotation: pose.rotation,
            })
        }
        _ => None,
    }
}

pub fn companion_placement_projection(
    model: &ReadModel,
    selected: Option<&objects::ScopedTreeContext>,
    context_generation: u64,
    scope_generation: u64,
) -> Option<inspector::CompanionPlacementProjection> {
    let selected = selected?;
    if !objects::selection_context_is_current(model, &selected.scope, &selected.context) {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let document = &snapshot.document;
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
    for pair in &document.mirrored_pairs {
        if let Some(matrix_id) = matrix_id.filter(|_| !component_context) {
            // The selection may name the derived mirror side: resolve to the
            // authored source matrix and keep pairs that project it.
            let source_matrix_id =
                match mirrored_half::pair_matrix_derivation(document, pair, matrix_id) {
                    Some((source, _)) => source.to_owned(),
                    None => matrix_id.to_owned(),
                };
            let Some(source_matrix) = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == source_matrix_id)
            else {
                continue;
            };
            if !mirrored_half::owns_matrix(document, &pair.source, source_matrix) {
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
            for part_id in selected_parts {
                let Some((source_id, _)) = mirrored_half::pair_derivation(document, pair, part_id)
                else {
                    continue;
                };
                if let Some(reason) =
                    boardstudio_core_mirrored_half::companion_placement_disabled_reason(
                        document, &pair.id, source_id,
                    )
                {
                    disabled_reason = Some(reason);
                    continue;
                }
                if !is_matrix_companion(document, pair, source_id) {
                    continue;
                }
                let mirror_id = mirrored_half::mirrored_part_id(document, pair, source_id);
                if !eligible
                    .iter()
                    .any(|existing: &CompanionLink| existing.mirror_id == mirror_id)
                {
                    eligible.push(CompanionLink {
                        source_id: source_id.to_owned(),
                        mirror_id: mirror_id.clone(),
                    });
                    targets.push(CompanionPlacementTarget::Item {
                        item_kind: MirroredItemKind::Part,
                        mirror_id,
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
    // Derived pairs carry one pair-wide default; per-item state is the sparse
    // companion-pose tweak. Modes never mix across the eligible set.
    let default_mode = pair_default_mode(document, pair);
    let first_mode = Some(default_mode);
    let mixed_mode = false;
    let tweaks: Vec<_> = eligible
        .iter()
        .map(|link| item_tweak(pair, &link.source_id))
        .collect();
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

pub fn companion_links_for_selection(
    document: &boardstudio_model::model::ProjectDoc,
    pair: &MirroredPair,
    selected_matrix_id: &str,
    row: Option<u32>,
    column: Option<u32>,
) -> (Vec<CompanionLink>, Option<&'static str>) {
    // The selection may name the derived mirror side: resolve to the
    // authored source matrix and keep pairs that project it.
    let source_matrix_id =
        match mirrored_half::pair_matrix_derivation(document, pair, selected_matrix_id) {
            Some((source, _)) => source.to_owned(),
            None => selected_matrix_id.to_owned(),
        };
    let Some(matrix) = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == source_matrix_id)
        .filter(|matrix| mirrored_half::owns_matrix(document, &pair.source, matrix))
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
            if let Some(reason) =
                boardstudio_core_mirrored_half::companion_placement_disabled_reason(
                    document, &pair.id, &source_id,
                )
            {
                disabled_reason = Some(reason);
            } else if !links
                .iter()
                .any(|existing: &CompanionLink| existing.source_id == source_id)
            {
                links.push(CompanionLink {
                    source_id: source_id.clone(),
                    mirror_id: mirrored_half::mirrored_part_id(document, pair, &source_id),
                });
            }
        }
    }
    (links, disabled_reason)
}

pub fn is_matrix_companion(
    document: &boardstudio_model::model::ProjectDoc,
    pair: &MirroredPair,
    source_id: &str,
) -> bool {
    let Some(source_matrix_id) = source_matrix_id(document, pair, source_id) else {
        return false;
    };
    companion_links_for_selection(document, pair, &source_matrix_id, None, None)
        .0
        .iter()
        .any(|link| link.source_id == source_id)
}

/// The authored source matrix owning one companion source part, if any pair
/// matrix projects it.
pub fn source_matrix_id(
    document: &boardstudio_model::model::ProjectDoc,
    pair: &MirroredPair,
    source_part_id: &str,
) -> Option<String> {
    document
        .matrices
        .iter()
        .filter(|matrix| mirrored_half::owns_matrix(document, &pair.source, matrix))
        .find(|matrix| {
            matrix
                .part_ids
                .iter()
                .any(|part_id| part_id == source_part_id)
        })
        .map(|matrix| matrix.id.clone())
}

pub fn layout_relationship_summary(
    document: &boardstudio_model::model::ProjectDoc,
    board: &boardstudio_model::model::Board,
    active_layout: Option<&boardstudio_model::model::Layout>,
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
pub fn part_anchor_summary(
    document: &boardstudio_model::model::ProjectDoc,
    part: &boardstudio_model::model::Part,
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
