//! Pure outline edit conversion, admission, settlement, and inspector projection helpers.

use super::planner::{
    EditablePerimeter, OutlineAction, OutlineActionKind, OutlinePointTarget, PerimeterAnchor,
    connection_point_world, outline_connection_control, unique_outline_entity_id,
    unique_outline_version_id,
};
use crate::outline_settings::{OutlineEdit as OutlineSettingEdit, generated_feature};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, ReadModel, Resolution, Scope,
};
use boardstudio_core_kernel::outline_edit::{
    OutlineChange, OutlineEdit, OutlineEditDisposition, OutlineEditTarget,
};
use boardstudio_model::model::{
    Board, BoardOutline, Contour, EditOperation, EditPhase, Operation, OutlineConnection,
    OutlineFeature, OutlineVersion, ProjectDoc, Side, Vec2,
};
use boardstudio_web_ui_model::tree::TreeContext;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutlineDrawTool {
    Polygon(Operation),
    Connect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineFeedback {
    pub scope: Scope,
    pub generation: u64,
    pub board_id: String,
    pub message: Option<String>,
}

/// How one outline version came to exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineVersionOwnership {
    Generated,
    Linked,
    Edited,
}

impl OutlineVersionOwnership {
    pub fn label(self) -> &'static str {
        match self {
            Self::Generated => "Generated",
            Self::Linked => "Linked refinement",
            Self::Edited => "Edited outline",
        }
    }
}

/// One selectable outline: its Core-owned ownership plus the accepted scene's located problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineVersionChoice {
    pub id: String,
    pub name: String,
    pub ownership: OutlineVersionOwnership,
    pub problem_finding_id: Option<String>,
    pub problem_message: Option<String>,
    pub problem_selectable: bool,
    pub blocked_exports: Vec<String>,
}

impl OutlineVersionChoice {
    pub fn label(&self) -> String {
        format!("{} \u{b7} {}", self.ownership.label(), self.name)
    }
}

pub fn outline_state<'a>(
    snapshot: &'a AcceptedSnapshot,
    board_id: &str,
) -> Option<&'a BoardOutline> {
    snapshot
        .document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id)
}

pub fn active_outline<'a>(
    snapshot: &'a AcceptedSnapshot,
    board_id: &str,
) -> (Option<&'a BoardOutline>, Option<&'a OutlineVersion>) {
    let state = outline_state(snapshot, board_id);
    let version = state
        .and_then(|state| state.active_version_id.as_deref())
        .and_then(|id| state?.versions.iter().find(|version| version.id == id));
    (state, version)
}

pub fn outline_version<'a>(
    outline: Option<&'a BoardOutline>,
    version_id: &str,
) -> Option<&'a OutlineVersion> {
    outline?
        .versions
        .iter()
        .find(|version| version.id == version_id)
}

pub fn version_geometry(
    outline: Option<&BoardOutline>,
    active_version_id: Option<&str>,
    board: &Board,
    document: &ProjectDoc,
    generated: Option<&OutlineFeature>,
) -> (
    Vec<OutlineFeature>,
    Vec<OutlineConnection>,
    Option<OutlineFeature>,
) {
    let active = active_version_id.and_then(|id| outline_version(outline, id));
    let features = active
        .map(|version| {
            version
                .geometry
                .features
                .iter()
                .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .cloned()
                .collect()
        })
        .unwrap_or_else(|| {
            board
                .outline_ids
                .iter()
                .filter_map(|id| document.outline.iter().find(|feature| feature.id() == id))
                .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .cloned()
                .collect()
        });
    let connections = active
        .and_then(|version| {
            version
                .geometry
                .features
                .iter()
                .find_map(|feature| match feature {
                    OutlineFeature::PartEnvelope { connections, .. } => Some(connections.clone()),
                    _ => None,
                })
        })
        .or_else(|| {
            generated.and_then(|feature| match feature {
                OutlineFeature::PartEnvelope { connections, .. } => Some(connections.clone()),
                _ => None,
            })
        })
        .unwrap_or_default();
    let connection_feature = active
        .and_then(|version| {
            version
                .geometry
                .features
                .iter()
                .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .cloned()
        })
        .or_else(|| generated.cloned());
    (features, connections, connection_feature)
}

pub fn blocked_export_labels(snapshot: &AcceptedSnapshot, board_id: &str) -> Vec<String> {
    let Some(readiness) = snapshot
        .scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == board_id)
    else {
        return Vec::new();
    };
    let mut labels = Vec::new();
    if !readiness.outline {
        labels.push("Outline".to_owned());
    }
    if !readiness.pcb {
        labels.push("PCB".to_owned());
    }
    if !readiness.case_ready {
        labels.push("Case".to_owned());
    }
    labels
}

pub fn editable_perimeter(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    active_version_id: Option<&str>,
    selected_feature_id: Option<&str>,
) -> Option<EditablePerimeter> {
    if let Some(version_id) = active_version_id {
        let feature = outline_version(outline_state(snapshot, board_id), version_id)?
            .geometry
            .features
            .iter()
            .find(|feature| {
                matches!(feature, OutlineFeature::Polygon { .. })
                    && selected_feature_id.is_none_or(|id| feature.id() == id)
            })?;
        let OutlineFeature::Polygon {
            id,
            points,
            anchor_part_id,
            operation,
        } = feature
        else {
            return None;
        };
        let anchor = anchor_part_id
            .as_deref()
            .and_then(|id| snapshot.document.parts.iter().find(|part| part.id == id))
            .map(|part| PerimeterAnchor {
                at: part.pose.at,
                rotation: part.pose.rotation,
                back: part.side == Side::Back,
            });
        return Some(EditablePerimeter {
            target: OutlinePointTarget::Fixed {
                version_id: version_id.to_owned(),
                feature_id: id.clone(),
                anchor_part_id: anchor_part_id.clone(),
                operation: *operation,
            },
            points: points.clone(),
            canvas_points: points
                .iter()
                .map(|point| anchor.map_or(*point, |a| a.world(*point)))
                .collect(),
            anchor,
        });
    }

    let source = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)?
        .source_contours
        .first()?;
    Some(EditablePerimeter {
        target: OutlinePointTarget::Generated { contour: 0 },
        points: source.points.clone(),
        canvas_points: source.points.clone(),
        anchor: None,
    })
}

pub fn gap_focus_camera(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    gap_id: &str,
    (surface_width, surface_height): (f64, f64),
    (base_min_x, base_max_x, base_min_y, base_max_y): (f64, f64, f64, f64),
) -> Option<(Vec2, f64)> {
    let gap = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .and_then(|scene| scene.gaps.iter().find(|gap| gap.id == gap_id))?;
    let points = gap
        .points
        .iter()
        .map(|point| connection_point_world(point, &snapshot.document.parts))
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .collect::<Vec<_>>();
    let (min_x, max_x, min_y, max_y) =
        points
            .iter()
            .fold(None::<(f64, f64, f64, f64)>, |bounds, point| {
                Some(match bounds {
                    Some((min_x, max_x, min_y, max_y)) => (
                        min_x.min(point.x),
                        max_x.max(point.x),
                        min_y.min(point.y),
                        max_y.max(point.y),
                    ),
                    None => (point.x, point.x, point.y, point.y),
                })
            })?;
    let target_width = (max_x - min_x + 24.0).max(1.0);
    let target_height = (max_y - min_y + 24.0).max(1.0);
    let base_width = (base_max_x - base_min_x).max(50.0);
    let base_height = (base_max_y - base_min_y).max(50.0);
    let usable_width = (surface_width - 32.0).max(1.0);
    let usable_height = (surface_height - 72.0 - 24.0).max(1.0);
    let zoom = (base_width / target_width * usable_width / surface_width)
        .min(base_height / target_height * usable_height / surface_height)
        .clamp(0.15, 8.0);
    let center = Vec2 {
        x: (min_x + max_x - base_min_x - base_max_x) * 0.5,
        y: (min_y + max_y - base_min_y - base_max_y) * 0.5
            + (72.0 - 24.0) * base_height / zoom / surface_height * 0.5,
    };
    Some((center, zoom))
}

fn board<'a>(document: &'a ProjectDoc, board_id: &str) -> Option<&'a Board> {
    document.boards.iter().find(|board| board.id == board_id)
}

fn generated_envelope<'a>(document: &'a ProjectDoc, board_id: &str) -> Option<&'a OutlineFeature> {
    let board = board(document, board_id)?;
    generated_feature(document, board)
}

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

fn next_edited_outline_name(state: Option<&BoardOutline>) -> String {
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

fn reviewed_source(snapshot: &AcceptedSnapshot, board_id: &str) -> Option<Vec<Contour>> {
    snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .map(|scene| scene.source_contours.clone())
}

fn settings_target(board_id: &str, active_version: Option<&str>) -> OutlineEditTarget {
    match active_version {
        Some(version_id) => version_target(board_id, version_id),
        None => board_target(board_id),
    }
}

fn board_target(board_id: &str) -> OutlineEditTarget {
    OutlineEditTarget::Board {
        board_id: board_id.to_owned(),
    }
}

fn version_target(board_id: &str, version_id: &str) -> OutlineEditTarget {
    OutlineEditTarget::Version {
        board_id: board_id.to_owned(),
        version_id: version_id.to_owned(),
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
        if active_version.is_some() {
            return None;
        }
        Some((board_target(board_id), change, None))
    };
    let version_or_generated = |change: OutlineChange| -> Plan {
        (settings_target(board_id, active_version), change, None)
    };
    let linked_or_generated = |change: OutlineChange| -> Option<Plan> {
        match active_version {
            None => generated_only(change),
            Some(version_id) => document
                .board_outlines
                .iter()
                .find(|state| state.board_id == board_id)?
                .versions
                .iter()
                .find(|version| version.id == version_id)
                .is_some_and(|version| !version.ownership.is_fixed())
                .then(|| version_or_generated(change)),
        }
    };
    match edit {
        OutlineSettingEdit::RenameVersion { version_id, name } => Some((
            version_target(board_id, version_id),
            OutlineChange::Rename {
                name: name.trim().to_owned(),
            },
            None,
        )),
        OutlineSettingEdit::CreateAutomatic => {
            let board = board(document, board_id)?;
            Some((
                board_target(board_id),
                OutlineChange::CreateGenerated {
                    margin: 4.0,
                    settings: crate::outline_settings::reference_outline_settings(),
                    part_ids: board.part_ids.clone(),
                },
                None,
            ))
        }
        OutlineSettingEdit::SetMargin(value) => {
            linked_or_generated(OutlineChange::SetMargin { margin: *value })
        }
        OutlineSettingEdit::SetBridgeWidth(value) => {
            linked_or_generated(OutlineChange::SetBridgeWidth {
                bridge_width: *value,
            })
        }
        OutlineSettingEdit::SetRepairEnabled(value) => {
            linked_or_generated(OutlineChange::SetRepairEnabled { enabled: *value })
        }
        OutlineSettingEdit::SetMaximumGapSpan(value) => {
            linked_or_generated(OutlineChange::SetMaximumGapSpan { value: *value })
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
            let (change, reviewed) = if *protected {
                (
                    OutlineChange::SetProtectedGap {
                        gap_id: gap.id.clone(),
                        points: gap.points.clone(),
                    },
                    Some(reviewed_source(snapshot, board_id)?),
                )
            } else {
                (
                    OutlineChange::RemoveProtectedGap {
                        gap_id: gap.id.clone(),
                    },
                    None,
                )
            };
            Some((board_target(board_id), change, reviewed))
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
pub fn outline_edit(
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
    let (target, change, reviewed) = match &action.kind {
        OutlineActionKind::Activate { version_id } => {
            let target = settings_target(&board_id, version_id.as_deref());
            (target, OutlineChange::Activate, None)
        }
        OutlineActionKind::Copy => (
            board_target(&board_id),
            OutlineChange::Copy {
                version_id: unique_outline_version_id(seed, all_version_ids(document)),
                name: next_edited_outline_name(state),
            },
            None,
        ),
        OutlineActionKind::Delete { version_id } => (
            version_target(&board_id, version_id),
            OutlineChange::Delete,
            None,
        ),
        OutlineActionKind::Update { edit } => update(
            document,
            snapshot,
            &board_id,
            active_version.as_deref(),
            edit,
        )?,
        OutlineActionKind::Refine => (
            board_target(&board_id),
            OutlineChange::CreateLinkedRefinement {
                version_id: unique_outline_version_id(seed, all_version_ids(document)),
                name: next_edited_outline_name(state),
            },
            None,
        ),
        OutlineActionKind::Freeze { version_id } => (
            version_target(&board_id, version_id),
            OutlineChange::Freeze {
                name: next_edited_outline_name(state),
            },
            None,
        ),
        OutlineActionKind::EditPerimeter { target, points, .. } => {
            let reviewed = Some(reviewed_source(snapshot, &board_id)?);
            match target {
                OutlinePointTarget::Generated { contour } => (
                    board_target(&board_id),
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
                    version_target(&board_id, version_id),
                    OutlineChange::EditFeaturePerimeter {
                        feature_id: feature_id.clone(),
                        points: points.clone(),
                    },
                    reviewed,
                ),
            }
        }
        OutlineActionKind::AddFeature { context, feature } => {
            let target = match context {
                TreeContext::OutlineVersion {
                    version_id: Some(version_id),
                    ..
                } => version_target(&board_id, version_id),
                _ => board_target(&board_id),
            };
            (
                target,
                OutlineChange::AddFeature {
                    feature: feature.clone(),
                },
                None,
            )
        }
        OutlineActionKind::AddConnection { points } => {
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
            let id = unique_outline_entity_id(
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
                            outline_connection_control(*point, &eligible)
                        } else {
                            boardstudio_model::model::OutlineControlPoint {
                                at: *point,
                                part_id: None,
                            }
                        }
                    })
                    .collect(),
            };
            (
                board_target(&board_id),
                OutlineChange::AddConnection { connection },
                None,
            )
        }
        OutlineActionKind::SetFeature {
            version_id,
            before,
            after,
            ..
        } => {
            if before.id() != after.id() {
                return None;
            }
            let target = settings_target(&board_id, version_id.as_deref());
            (
                target,
                OutlineChange::SetFeature {
                    feature: after.clone(),
                },
                None,
            )
        }
        OutlineActionKind::RemoveFeature {
            version_id,
            feature_id,
            ..
        } => (
            version_target(&board_id, version_id),
            OutlineChange::RemoveFeature {
                feature_id: feature_id.clone(),
            },
            None,
        ),
        OutlineActionKind::FocusGap { .. } => return None,
    };
    Some(OutlineEdit {
        target,
        change,
        seed,
        reviewed,
    })
}

/// The durable ids the submitted edit addresses, for transaction bookkeeping.
pub fn target_ids(action: &OutlineAction) -> Vec<String> {
    let board_id = action.envelope().4.clone();
    let mut ids = vec![board_id];
    match &action.kind {
        OutlineActionKind::Activate {
            version_id: Some(version_id),
            ..
        }
        | OutlineActionKind::Delete { version_id, .. }
        | OutlineActionKind::Freeze { version_id, .. } => ids.push(version_id.clone()),
        OutlineActionKind::EditPerimeter {
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
        OutlineActionKind::SetFeature {
            version_id, after, ..
        } => {
            if let Some(version_id) = version_id {
                ids.push(version_id.clone());
            }
            ids.push(after.id().to_owned());
        }
        OutlineActionKind::RemoveFeature {
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

pub fn retire_reason(action: &OutlineAction) -> &'static str {
    match &action.kind {
        OutlineActionKind::Activate {
            version_id: Some(_),
            ..
        }
        | OutlineActionKind::Delete { .. }
        | OutlineActionKind::Freeze { .. } => "That outline version no longer exists.",
        OutlineActionKind::SetFeature { .. } | OutlineActionKind::RemoveFeature { .. } => {
            "That outline feature no longer exists."
        }
        OutlineActionKind::EditPerimeter { .. } => "That outline perimeter no longer exists.",
        OutlineActionKind::Update { edit, .. } => match edit {
            OutlineSettingEdit::RenameVersion { .. } => "That outline version no longer exists.",
            OutlineSettingEdit::SetProtectedGap { .. }
            | OutlineSettingEdit::RemoveProtectedGap { .. } => "That outline gap no longer exists.",
            OutlineSettingEdit::CreateAutomatic => "The generated outline already exists.",
            _ => "The outline changed before this edit was applied.",
        },
        OutlineActionKind::Copy => "The outline changed before this copy was applied.",
        OutlineActionKind::Refine => {
            "Resolve the generated outline before making a linked refinement."
        }
        OutlineActionKind::AddFeature { .. } | OutlineActionKind::AddConnection { .. } => {
            "The outline changed before this addition was applied."
        }
        _ => "The outline changed before this edit was applied.",
    }
}

pub fn settle(
    accepted: &AcceptedSnapshot,
    edit: &OutlineEdit,
    action: &OutlineAction,
    transaction_id: String,
) -> Resolution {
    match boardstudio_core_kernel::outline_edit::evaluate(accepted.document.as_ref(), edit) {
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

pub fn admission(accepted: &AcceptedSnapshot, edit: &OutlineEdit) -> Admission {
    match boardstudio_core_kernel::outline_edit::evaluate(accepted.document.as_ref(), edit) {
        Ok(OutlineEditDisposition::Changed) => Admission::Submit,
        Ok(OutlineEditDisposition::Unchanged) | Ok(OutlineEditDisposition::Retire) => {
            Admission::Ignore
        }
        Err(_) => Admission::Malformed,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    Submit,
    Ignore,
    Malformed,
}

pub fn outline_action_matches_saved_owner(
    action: &OutlineAction,
    model: &ReadModel,
    current_scope: Option<&Scope>,
    generation: u64,
) -> bool {
    let (scope, token, revision, captured_generation, board_id) = action.envelope();
    let perimeter_edit = matches!(
        &action.kind,
        OutlineActionKind::EditPerimeter {
            phase: EditPhase::Preview | EditPhase::Commit,
            ..
        }
    );
    let lifecycle_current = model.lifecycle == Lifecycle::Ready;
    generation == captured_generation
        && current_scope == Some(scope)
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.token == *token
                && snapshot.document.revision == revision
                && snapshot.document.id == scope.document_id
                && snapshot.session_epoch == scope.session_epoch
        })
        && scope.board_id == *board_id
        && lifecycle_current
        && model.durability == (Durability::Saved { revision })
        && (model.display_preview.is_none() || perimeter_edit)
        && model.gesture.is_none()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::outline::OutlineActionEnvelope;
    use boardstudio_application::{Event, SessionEpoch, SnapshotToken};
    use boardstudio_web_test_runtime::new_native_runtime;

    #[test]
    fn outline_copy_requires_the_captured_saved_owner() {
        let runtime = new_native_runtime();
        let mut document = ProjectDoc::empty("outline", "Outline");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        let mut model = runtime.model();
        let scope = runtime.scope().unwrap();
        let snapshot = model.accepted.as_ref().unwrap();
        let revision = snapshot.document.revision;
        let token = snapshot.token;
        let action = OutlineAction {
            envelope: OutlineActionEnvelope {
                scope: scope.clone(),
                token,
                revision,
                generation: 2,
                board_id: scope.board_id.clone(),
            },
            kind: OutlineActionKind::Copy,
        };
        let admitted = |model: &ReadModel, scope: Option<&Scope>, generation| {
            outline_action_matches_saved_owner(&action, model, scope, generation)
        };
        assert!(admitted(&model, Some(&scope), 2));
        assert!(!admitted(&model, Some(&scope), 3));
        assert!(!admitted(&model, None, 2));
        let replacement = Scope {
            session_epoch: SessionEpoch(scope.session_epoch.0 + 1),
            ..scope.clone()
        };
        assert!(!admitted(&model, Some(&replacement), 2));
        model.durability = Durability::Saving { revision };
        assert!(!admitted(&model, Some(&scope), 2));
        model.durability = Durability::Saved { revision };
        model.accepted.as_mut().unwrap().token = SnapshotToken(token.0 + 1);
        assert!(!admitted(&model, Some(&scope), 2));
    }
}
