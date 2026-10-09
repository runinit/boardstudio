//! Adapter from presentation outline actions to the typed Core outline edit.
//!
//! One [`OutlineAction`] describes what the user did. This module turns it into exactly
//! one [`CoreOutlineEdit`] - the target, the named change, the captured creation seed and
//! the source shape the user reviewed - so Core owns identity allocation, validation,
//! version mutation, linked resolution and the no-op/retirement decision. Drafts,
//! selection, focus and lifecycle admission stay in the hook, and no adapter prepares a
//! replacement document.
use super::planner::{OutlineAction, OutlinePointTarget};
use crate::outline_settings::{OutlineEdit as OutlineSettingEdit, generated_feature};
use boardstudio_application::{AcceptedSnapshot, Resolution};
use boardstudio_core::model::{
    Board, Contour, EditOperation, OutlineConnection, OutlineFeature, ProjectDoc,
};
use boardstudio_core::outline_edit::{
    OutlineChange, OutlineEdit, OutlineEditDisposition, OutlineEditTarget,
};
use boardstudio_web_ui_model::tree::TreeContext;

fn board<'a>(document: &'a ProjectDoc, board_id: &str) -> Option<&'a Board> {
    document.boards.iter().find(|board| board.id == board_id)
}

/// The board's generated envelope feature, so the adapter can read the board's bridge
/// width and connection identities without re-deriving Core's rules.
fn generated_envelope<'a>(document: &'a ProjectDoc, board_id: &str) -> Option<&'a OutlineFeature> {
    let board = board(document, board_id)?;
    generated_feature(document, board)
}

/// Every identity a new outline version must avoid: the persisted versions plus the
/// retired ones, whose identities stay reserved forever (Core refuses to reissue them).
fn all_version_ids(document: &ProjectDoc) -> Vec<String> {
    document
        .board_outlines
        .iter()
        .flat_map(|state| {
            state
                .versions
                .iter()
                .map(|version| version.id.clone())
                .chain(state.retired_version_ids.iter().cloned())
        })
        .collect()
}

/// The next non-duplicating `Edited outline N` name on this board.
fn next_edited_outline_name(state: Option<&boardstudio_core::model::BoardOutline>) -> String {
    let next_number = state
        .into_iter()
        .flat_map(|state| &state.versions)
        .filter_map(|version| {
            version
                .name
                .strip_prefix("Edited outline ")
                .and_then(|number| number.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    format!("Edited outline {next_number}")
}

/// The source shape the user reviewed for a source-shape edit: the board's current
/// source contours, which are the Generated source while Generated is active and the
/// active version's resolved projection while that version is active.
fn reviewed_source(snapshot: &AcceptedSnapshot, board_id: &str) -> Option<Vec<Contour>> {
    snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .map(|scene| scene.source_contours.clone())
}

/// The version a settings field addresses: the active version when one is selected,
/// otherwise the Generated envelope that owns the field.
fn settings_target(board_id: &str, active_version: Option<&str>) -> OutlineEditTarget {
    match active_version {
        Some(version_id) => OutlineEditTarget::Version {
            board_id: board_id.to_owned(),
            version_id: version_id.to_owned(),
        },
        None => OutlineEditTarget::Board {
            board_id: board_id.to_owned(),
        },
    }
}

type Plan = (OutlineEditTarget, OutlineChange, Option<Vec<Contour>>);

fn update(
    document: &ProjectDoc,
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    active_version: Option<&str>,
    edit: &OutlineSettingEdit,
) -> Option<Plan> {
    let generated_only = |change: OutlineChange| -> Option<Plan> {
        // The Generated envelope owns these fields; a selected version owns its own copy.
        if active_version.is_some() {
            return None;
        }
        Some((
            OutlineEditTarget::Board {
                board_id: board_id.to_owned(),
            },
            change,
            None,
        ))
    };
    let version_or_generated = |change: OutlineChange| -> Plan {
        (settings_target(board_id, active_version), change, None)
    };
    match edit {
        OutlineSettingEdit::RenameVersion { version_id, name } => Some((
            OutlineEditTarget::Version {
                board_id: board_id.to_owned(),
                version_id: version_id.clone(),
            },
            OutlineChange::Rename {
                name: name.trim().to_owned(),
            },
            None,
        )),
        // One typed Generated-envelope creation on the board: Core authors the envelope
        // and its board membership in one accepted edit, and settles Unchanged when the
        // board already has one.
        OutlineSettingEdit::CreateAutomatic => {
            let board = board(document, board_id)?;
            Some((
                OutlineEditTarget::Board {
                    board_id: board_id.to_owned(),
                },
                OutlineChange::CreateGenerated {
                    margin: 4.0,
                    settings: crate::outline_settings::reference_outline_settings(),
                    part_ids: board.part_ids.clone(),
                },
                None,
            ))
        }
        OutlineSettingEdit::SetMargin(value) => {
            generated_only(OutlineChange::SetMargin { margin: *value })
        }
        OutlineSettingEdit::SetBridgeWidth(value) => {
            generated_only(OutlineChange::SetBridgeWidth {
                bridge_width: *value,
            })
        }
        OutlineSettingEdit::SetRepairEnabled(value) => {
            generated_only(OutlineChange::SetRepairEnabled { enabled: *value })
        }
        OutlineSettingEdit::SetMaximumGapSpan(value) => {
            generated_only(OutlineChange::SetMaximumGapSpan { value: *value })
        }
        OutlineSettingEdit::SetCorners(value) => {
            Some(version_or_generated(OutlineChange::SetCorners {
                corners: *value,
            }))
        }
        OutlineSettingEdit::SetSize(value) => Some(version_or_generated(OutlineChange::SetSize {
            size: *value,
        })),
        OutlineSettingEdit::SetMinimumConnectionWidth(value) => Some(version_or_generated(
            OutlineChange::SetMinimumConnectionWidth { value: *value },
        )),
        OutlineSettingEdit::SetEdgeClearance(value) => {
            Some(version_or_generated(OutlineChange::SetEdgeClearance {
                value: *value,
            }))
        }
        OutlineSettingEdit::SetProtectedGap { gap_id, protected } => {
            if active_version.is_some() {
                return None;
            }
            let gap = snapshot
                .scene
                .board_outline_scenes
                .iter()
                .find(|scene| scene.board_id == board_id)?
                .gaps
                .iter()
                .find(|gap| gap.id == *gap_id)?;
            if generated_envelope(document, board_id)?.id() != gap.feature_id {
                return None;
            }
            let change = if *protected {
                OutlineChange::SetProtectedGap {
                    gap_id: gap.id.clone(),
                    points: gap.points.clone(),
                }
            } else {
                OutlineChange::RemoveProtectedGap {
                    gap_id: gap.id.clone(),
                }
            };
            Some((
                OutlineEditTarget::Board {
                    board_id: board_id.to_owned(),
                },
                change,
                None,
            ))
        }
        OutlineSettingEdit::RemoveProtectedGap { gap_id } => {
            generated_only(OutlineChange::RemoveProtectedGap {
                gap_id: gap_id.clone(),
            })
        }
    }
}

/// One typed Core outline edit for this presentation action, or `None` when the action
/// cannot be expressed (including work that is not an edit at all, like focusing a gap).
pub(crate) fn outline_edit(
    snapshot: &AcceptedSnapshot,
    action: &OutlineAction,
    seed: u64,
) -> Option<OutlineEdit> {
    let document = snapshot.document.as_ref();
    let board_id = action.envelope().4.clone();
    board(document, &board_id)?;
    let state = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    let active_version = state.and_then(|state| state.active_version_id.clone());
    let (target, change, reviewed) = match action {
        OutlineAction::Activate { version_id, .. } => {
            let target = match version_id {
                Some(version_id) => OutlineEditTarget::Version {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                None => OutlineEditTarget::Board {
                    board_id: board_id.clone(),
                },
            };
            (target, OutlineChange::Activate, None)
        }
        OutlineAction::Copy { .. } => (
            OutlineEditTarget::Board {
                board_id: board_id.clone(),
            },
            OutlineChange::Copy {
                version_id: super::planner::unique_outline_version_id(
                    seed,
                    all_version_ids(document),
                ),
                name: next_edited_outline_name(state),
            },
            None,
        ),
        OutlineAction::Delete { version_id, .. } => (
            OutlineEditTarget::Version {
                board_id: board_id.clone(),
                version_id: version_id.clone(),
            },
            OutlineChange::Delete,
            None,
        ),
        OutlineAction::Update { edit, .. } => update(
            document,
            snapshot,
            &board_id,
            active_version.as_deref(),
            edit,
        )?,
        // Accepting the Generated outline as an alternative creates a linked refinement:
        // Core seeds it from the board's current generated projections, so it follows the
        // components and its settings stay editable while linked.
        OutlineAction::Refine { .. } => (
            OutlineEditTarget::Board {
                board_id: board_id.clone(),
            },
            OutlineChange::CreateLinkedRefinement {
                version_id: super::planner::unique_outline_version_id(
                    seed,
                    all_version_ids(document),
                ),
                name: next_edited_outline_name(state),
            },
            None,
        ),
        OutlineAction::Freeze { version_id, .. } => (
            OutlineEditTarget::Version {
                board_id: board_id.clone(),
                version_id: version_id.clone(),
            },
            OutlineChange::Freeze {
                name: next_edited_outline_name(state),
            },
            None,
        ),
        OutlineAction::EditPerimeter { target, points, .. } => {
            let reviewed = Some(reviewed_source(snapshot, &board_id)?);
            match target {
                OutlinePointTarget::Generated { contour } => (
                    OutlineEditTarget::Board {
                        board_id: board_id.clone(),
                    },
                    OutlineChange::EditGeneratedPerimeter {
                        contour: *contour,
                        points: points.clone(),
                    },
                    reviewed,
                ),
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    ..
                } => (
                    OutlineEditTarget::Version {
                        board_id: board_id.clone(),
                        version_id: version_id.clone(),
                    },
                    OutlineChange::EditFeaturePerimeter {
                        feature_id: feature_id.clone(),
                        points: points.clone(),
                    },
                    reviewed,
                ),
            }
        }
        OutlineAction::AddFeature {
            context, feature, ..
        } => {
            let target = match context {
                TreeContext::OutlineVersion {
                    version_id: Some(version_id),
                    ..
                } => OutlineEditTarget::Version {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                _ => OutlineEditTarget::Board {
                    board_id: board_id.clone(),
                },
            };
            (
                target,
                OutlineChange::AddFeature {
                    feature: feature.clone(),
                },
                None,
            )
        }
        OutlineAction::AddConnection { points, .. } => {
            // A connection belongs to the Generated outline. The adapter supplies the
            // board's bridge width explicitly: Core refuses a non-positive or
            // non-finite width instead of silently defaulting it.
            let feature = generated_envelope(document, &board_id)?;
            let OutlineFeature::PartEnvelope {
                connections,
                settings,
                part_ids,
                ..
            } = feature
            else {
                return None;
            };
            let eligible = document
                .parts
                .iter()
                .filter(|part| {
                    part_ids.contains(&part.id)
                        && !part
                            .outline
                            .as_ref()
                            .is_some_and(|outline| outline.excluded)
                })
                .collect::<Vec<_>>();
            let id = super::planner::unique_outline_entity_id(
                "outline-connection",
                seed,
                connections.iter().map(|connection| connection.id.clone()),
            );
            let connection = OutlineConnection {
                id,
                width: settings.bridge_width,
                points: points
                    .iter()
                    .enumerate()
                    .map(|(index, point)| {
                        if index == 0 || index + 1 == points.len() {
                            super::planner::outline_connection_control(*point, &eligible)
                        } else {
                            boardstudio_core::model::OutlineControlPoint {
                                at: *point,
                                part_id: None,
                            }
                        }
                    })
                    .collect(),
            };
            (
                OutlineEditTarget::Board {
                    board_id: board_id.clone(),
                },
                OutlineChange::AddConnection { connection },
                None,
            )
        }
        OutlineAction::SetFeature {
            version_id,
            before,
            after,
            ..
        } => {
            if before.id() != after.id() {
                return None;
            }
            let target = match version_id {
                Some(version_id) => OutlineEditTarget::Version {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                None => OutlineEditTarget::Board {
                    board_id: board_id.clone(),
                },
            };
            (
                target,
                OutlineChange::SetFeature {
                    feature: after.clone(),
                },
                None,
            )
        }
        OutlineAction::RemoveFeature {
            version_id,
            feature_id,
            ..
        } => (
            OutlineEditTarget::Version {
                board_id: board_id.clone(),
                version_id: version_id.clone(),
            },
            OutlineChange::RemoveFeature {
                feature_id: feature_id.clone(),
            },
            None,
        ),
        // Focusing a gap is presentation only and never creates document history.
        OutlineAction::FocusGap { .. } => return None,
    };
    Some(OutlineEdit {
        target,
        change,
        seed,
        reviewed,
    })
}

/// The durable ids the submitted edit addresses, for transaction bookkeeping.
pub(crate) fn target_ids(action: &OutlineAction) -> Vec<String> {
    let board_id = action.envelope().4.clone();
    let mut ids = vec![board_id];
    match action {
        OutlineAction::Activate {
            version_id: Some(version_id),
            ..
        }
        | OutlineAction::Delete { version_id, .. } => ids.push(version_id.clone()),
        OutlineAction::Freeze { version_id, .. } => ids.push(version_id.clone()),
        OutlineAction::EditPerimeter {
            target:
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    ..
                },
            ..
        } => {
            ids.push(version_id.clone());
            ids.push(feature_id.clone());
        }
        OutlineAction::SetFeature {
            version_id, after, ..
        } => {
            if let Some(version_id) = version_id {
                ids.push(version_id.clone());
            }
            ids.push(after.id().to_owned());
        }
        OutlineAction::RemoveFeature {
            version_id,
            feature_id,
            ..
        } => {
            ids.push(version_id.clone());
            ids.push(feature_id.clone());
        }
        _ => {}
    }
    ids
}

/// Why a retired outline edit is reported. Core decides retirement; the adapter owns the
/// wording of the explanation the user sees.
pub(crate) fn retire_reason(action: &OutlineAction) -> &'static str {
    match action {
        OutlineAction::Activate {
            version_id: Some(_),
            ..
        }
        | OutlineAction::Delete { .. } => "That outline version no longer exists.",
        OutlineAction::SetFeature { .. } | OutlineAction::RemoveFeature { .. } => {
            "That outline feature no longer exists."
        }
        OutlineAction::EditPerimeter { .. } => "That outline perimeter no longer exists.",
        OutlineAction::Update { edit, .. } => match edit {
            OutlineSettingEdit::RenameVersion { .. } => "That outline version no longer exists.",
            OutlineSettingEdit::SetProtectedGap { .. }
            | OutlineSettingEdit::RemoveProtectedGap { .. } => "That outline gap no longer exists.",
            OutlineSettingEdit::CreateAutomatic => "The generated outline already exists.",
            _ => "The outline changed before this edit was applied.",
        },
        OutlineAction::Copy { .. } => "The outline changed before this copy was applied.",
        OutlineAction::Refine { .. } => {
            "Resolve the generated outline before making a linked refinement."
        }
        OutlineAction::Freeze { .. } => "That outline version no longer exists.",
        OutlineAction::AddFeature { .. } | OutlineAction::AddConnection { .. } => {
            "The outline changed before this addition was applied."
        }
        _ => "The outline changed before this edit was applied.",
    }
}

/// Settle one captured typed edit against the accepted document at execution.
///
/// `Changed` submits the one typed operation Core applies and reports its changed owners
/// for; `Unchanged` lands without a revision step; a vanished or ineligible target
/// retires with an explanation; malformed input surfaces Core's own error.
pub(crate) fn settle(
    accepted: &AcceptedSnapshot,
    edit: &OutlineEdit,
    action: &OutlineAction,
    transaction_id: String,
) -> Resolution {
    match boardstudio_core::outline_edit::evaluate(accepted.document.as_ref(), edit) {
        Ok(OutlineEditDisposition::Changed) => Resolution::submit_with_transaction_id(
            target_ids(action),
            transaction_id,
            EditOperation::ApplyOutlineEdit {
                edit: Box::new(edit.clone()),
            },
        ),
        Ok(OutlineEditDisposition::Unchanged) => Resolution::Unchanged,
        Ok(OutlineEditDisposition::Retire) => Resolution::Retire(retire_reason(action).to_owned()),
        Err(message) => Resolution::Retire(message),
    }
}

/// Admission at dispatch: whether the action is worth submitting a pending ticket for.
pub(crate) fn admission(accepted: &AcceptedSnapshot, edit: &OutlineEdit) -> Admission {
    match boardstudio_core::outline_edit::evaluate(accepted.document.as_ref(), edit) {
        Ok(OutlineEditDisposition::Changed) => Admission::Submit,
        Ok(OutlineEditDisposition::Unchanged) | Ok(OutlineEditDisposition::Retire) => {
            Admission::Ignore
        }
        Err(_) => Admission::Malformed,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Admission {
    /// The edit changes state; submit it as intent.
    Submit,
    /// The edit is equal to the accepted value or its target is gone: drop it silently.
    Ignore,
    /// The submitted geometry is malformed: submit so Core's own error surfaces.
    Malformed,
}
