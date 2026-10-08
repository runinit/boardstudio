//! Runtime and Dioxus adapter for outline lifecycle actions.

use super::planner::{EditablePerimeter, PerimeterAnchor, connection_point_world};
use super::planner::{OutlineAction, OutlinePointTarget, Skip, plan_action};
use crate::outline_settings::{
    OutlineEdit, generated_feature, generated_margin, generated_settings,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    Contour, EditPhase, Operation, OutlineConnection, OutlineFeature, OutlineGap,
    OutlineRepairSettings, OutlineSettings, Part, Side, Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlinePendingKey {
    OneShot,
    VersionName,
    Setting(OutlineSettingKey),
    Perimeter(OutlinePointTarget),
    Feature {
        feature_id: String,
        field: OutlineFeatureField,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct OutlineOwner {
    scope: Scope,
    context: crate::objects::TreeContext,
    active_version_id: Option<String>,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlineSettingKey {
    Corners,
    Size,
    Margin,
    BridgeWidth,
    RepairEnabled,
    MaximumGapSpan,
    MinimumConnectionWidth,
    EdgeClearance,
    ProtectedGap(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlineFeatureField {
    Whole,
    RectCenterX,
    RectCenterY,
    RectWidth,
    RectHeight,
    RectRadius,
    RectRotation,
    Connections,
    ConnectionWidth(String),
    ConnectionPoints(String),
    ConnectionPointX(String, usize),
    ConnectionPointY(String, usize),
    ConnectionPointAttachment(String, usize),
}

#[derive(Clone)]
struct ActionState {
    pending: PendingEditSignals<OutlinePendingKey>,
    feedback: Signal<Option<OutlineFeedback>>,
    one_shot_pending: Signal<bool>,
    version_name_draft: Signal<String>,
    version_name_failure: Signal<Option<String>>,
    pending_owner: Signal<Option<OutlineOwner>>,
    inspector_mounted: Signal<bool>,
    selected_point: Signal<usize>,
    editing_points: Signal<bool>,
    drawing_operation: Signal<Option<OutlineDrawTool>>,
    drawing_points: Signal<Vec<Vec2>>,
    selected_feature_id: Signal<Option<String>>,
    selected_connection_id: Signal<Option<String>>,
    selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    captured_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutlineDrawTool {
    Polygon(Operation),
    Connect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineFeedback {
    scope: Scope,
    pub generation: u64,
    pub board_id: String,
    pub message: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct OutlineInspectorProjection {
    pub board_id: String,
    pub board_name: String,
    pub version_name: String,
    pub active_version_id: Option<String>,
    pub contours: Vec<Contour>,
    pub(super) perimeter: Option<EditablePerimeter>,
    pub(super) snap_paths: Vec<Vec<Vec2>>,
    pub selected_point: Signal<usize>,
    pub editing_points: Signal<bool>,
    pub drawing_operation: Signal<Option<OutlineDrawTool>>,
    pub drawing_points: Signal<Vec<Vec2>>,
    pub(super) selected_feature_id: Signal<Option<String>>,
    pub(super) selected_connection_id: Signal<Option<String>>,
    pub geometry_features: Vec<OutlineFeature>,
    pub connections: Vec<OutlineConnection>,
    pub(super) connection_feature: Option<OutlineFeature>,
    pub(super) outline_parts: Vec<Part>,
    pub versions: Vec<OutlineVersionChoice>,
    pub settings: OutlineSettings,
    pub repair: OutlineRepairSettings,
    pub generated_margin: Option<f64>,
    pub has_generated: bool,
    pub gaps: Vec<OutlineGap>,
    pub(super) action_context: Rc<OutlineActionContext>,
    pub enabled: bool,
    /// A one-shot action (activate, copy, delete, add, remove) is pending. Field edits
    /// never read this.
    pub one_shot_pending: bool,
    pub(super) version_name_draft: Signal<String>,
    pub(super) version_name_failure: Signal<Option<String>>,
    pub(super) inspector_mounted: Signal<bool>,
    pub feedback: Option<OutlineFeedback>,
    pub on_action: EventHandler<OutlineAction>,
    pub(super) selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    pub scope: Scope,
    pub(super) token: boardstudio_application::SnapshotToken,
    pub(super) revision: u64,
    pub(super) generation: u64,
}

impl OutlineInspectorProjection {
    pub fn canvas_edit_key(&self) -> String {
        format!(
            "{:?}-{}-{:?}-{}-{}-{}",
            self.scope,
            self.board_id,
            self.active_version_id,
            self.token.0,
            self.revision,
            self.generation
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct OutlineActionContext {
    scope: Scope,
    token: boardstudio_application::SnapshotToken,
    pub(super) revision: u64,
    generation: u64,
    board_id: String,
    selection_context: crate::objects::TreeContext,
}

impl OutlineActionContext {
    pub(super) fn action(&self, edit: OutlineEdit) -> OutlineAction {
        OutlineAction::Update {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            edit,
        }
    }

    pub(super) fn activate_action(&self, version_id: Option<String>) -> OutlineAction {
        OutlineAction::Activate {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            context: self.selection_context.clone(),
            require_selected_version: false,
            board_id: self.board_id.clone(),
            version_id,
        }
    }

    pub(super) fn edit_perimeter(
        &self,
        target: OutlinePointTarget,
        points: Vec<Vec2>,
        phase: EditPhase,
        transaction_id: String,
    ) -> OutlineAction {
        OutlineAction::EditPerimeter {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            target,
            points,
            phase,
            transaction_id,
        }
    }

    pub(super) fn add_feature(&self, feature: OutlineFeature) -> OutlineAction {
        OutlineAction::AddFeature {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            feature,
        }
    }

    pub(super) fn add_connection(&self, points: Vec<Vec2>) -> OutlineAction {
        OutlineAction::AddConnection {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            points,
        }
    }

    pub(super) fn set_feature(
        &self,
        version_id: Option<String>,
        before: OutlineFeature,
        after: OutlineFeature,
    ) -> OutlineAction {
        OutlineAction::SetFeature {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            version_id,
            before,
            after,
        }
    }

    pub(super) fn remove_feature(&self, version_id: String, feature_id: String) -> OutlineAction {
        OutlineAction::RemoveFeature {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            version_id,
            feature_id,
        }
    }

    pub(super) fn focus_gap(&self, gap_id: String) -> OutlineAction {
        OutlineAction::FocusGap {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            gap_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineVersionChoice {
    pub id: String,
    pub name: String,
}

impl OutlineAction {
    pub fn is_current(&self, runtime: &Runtime, generation: u64) -> bool {
        let (scope, token, revision, captured_generation, board_id) = self.envelope();
        let model = runtime.model();
        let perimeter_edit = matches!(
            self,
            OutlineAction::EditPerimeter {
                phase: EditPhase::Preview | EditPhase::Commit,
                ..
            }
        );
        let lifecycle_current = model.lifecycle == Lifecycle::Ready
            || (perimeter_edit && model.lifecycle == Lifecycle::Applying);
        generation == captured_generation
            && runtime.scope().as_ref() == Some(scope)
            && model.accepted.as_ref().is_some_and(|snapshot| {
                snapshot.token == *token
                    && snapshot.document.revision == revision
                    && snapshot.document.id == scope.document_id
                    && snapshot.session_epoch == scope.session_epoch
            })
            && scope.board_id == *board_id
            && lifecycle_current
            && model.durability == (Durability::Saved { revision })
            && (model.display_preview.is_none()
                || matches!(
                    self,
                    OutlineAction::EditPerimeter {
                        phase: EditPhase::Preview | EditPhase::Commit,
                        ..
                    }
                ))
            && model.gesture.is_none()
    }
}

pub fn use_outline_lifecycle(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<crate::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> (
    Option<OutlineInspectorProjection>,
    EventHandler<OutlineAction>,
) {
    let version = use_context::<Signal<u64>>()();
    let captured_generation = scope_generation();
    let pending = use_hook(|| PendingEditSignals::<OutlinePendingKey>::new());
    let feedback = use_signal(|| None::<OutlineFeedback>);
    let one_shot_pending = use_signal(|| false);
    let version_name_draft = use_signal(String::new);
    let version_name_failure = use_signal(|| None::<String>);
    let pending_owner = use_signal(|| None::<OutlineOwner>);
    let inspector_mounted = use_signal(|| false);
    pending.bind_field(
        OutlinePendingKey::VersionName,
        version_name_draft,
        version_name_failure,
    );
    pending.bind_one_shot(OutlinePendingKey::OneShot, one_shot_pending);
    let selected_point = use_signal(|| 0usize);
    let editing_points = use_signal(|| false);
    let drawing_operation = use_signal(|| None::<OutlineDrawTool>);
    let drawing_points = use_signal(Vec::<Vec2>::new);
    let selected_feature_id = use_signal(|| None::<String>);
    let selected_connection_id = use_signal(|| None::<String>);
    let draft_owner = selected_context.read().clone();
    let draft_workspace = workspace();
    let draft_active_version = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|snapshot| {
            let board_id = draft_owner
                .as_ref()
                .and_then(|owner| match &owner.context {
                    crate::objects::TreeContext::Outline { board_id }
                    | crate::objects::TreeContext::OutlineVersion { board_id, .. } => {
                        Some(board_id)
                    }
                    _ => None,
                })?;
            snapshot
                .document
                .board_outlines
                .iter()
                .find(|state| &state.board_id == board_id)
        })
        .and_then(|state| state.active_version_id.clone());
    let draft_scope = runtime.scope();
    use_effect(use_reactive(
        (
            &version,
            &draft_owner,
            &draft_workspace,
            &captured_generation,
        ),
        {
            let mut drawing_operation = drawing_operation;
            let mut drawing_points = drawing_points;
            move |_| {
                drawing_operation.set(None);
                drawing_points.set(Vec::new());
            }
        },
    ));
    let draft_version_name = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|snapshot| {
            let owner_board = draft_owner
                .as_ref()
                .and_then(|owner| match &owner.context {
                    crate::objects::TreeContext::Outline { board_id }
                    | crate::objects::TreeContext::OutlineVersion { board_id, .. } => {
                        Some(board_id)
                    }
                    _ => None,
                })?;
            let outline = snapshot
                .document
                .board_outlines
                .iter()
                .find(|state| &state.board_id == owner_board)?;
            let active = outline.active_version_id.as_ref()?;
            Some(
                outline
                    .versions
                    .iter()
                    .find(|version| &version.id == active)?
                    .name
                    .clone(),
            )
        })
        .unwrap_or_default();
    use_effect(use_reactive(
        (
            &draft_owner,
            &draft_workspace,
            &draft_active_version,
            &draft_scope,
            &captured_generation,
        ),
        {
            let mut selected_feature_id = selected_feature_id;
            let mut selected_connection_id = selected_connection_id;
            let mut version_name_draft = version_name_draft;
            let mut version_name_failure = version_name_failure;
            move |_| {
                selected_feature_id.set(None);
                selected_connection_id.set(None);
                version_name_draft.set(draft_version_name.clone());
                version_name_failure.set(None);
            }
        },
    ));
    let action_state = ActionState {
        pending: pending.clone(),
        feedback,
        one_shot_pending,
        version_name_draft,
        version_name_failure,
        pending_owner,
        inspector_mounted,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
        selected_context,
        workspace,
        scope_generation,
        captured_generation,
    };
    let on_action = use_callback({
        let runtime = runtime.clone();
        let action_state = action_state.clone();
        move |action| {
            submit_action(&runtime, action_state.clone(), action);
        }
    });

    let live_owner = outline_owner(
        &runtime,
        selected_context.read().clone(),
        scope_generation(),
    );
    let observation_owner = pending_owner();
    use_effect(use_reactive(
        (
            &version,
            &workspace(),
            &scope_generation(),
            &live_owner,
            &observation_owner,
            &inspector_mounted(),
        ),
        {
            let runtime = runtime.clone();
            let pending = pending.clone();
            let mut feedback = feedback;
            let mut version_name_failure = version_name_failure;
            move |(
                _,
                current_workspace,
                current_generation,
                current_owner,
                submitted_owner,
                mounted,
            )| {
                let owner_is_live = mounted
                    && current_workspace == "Layout"
                    && current_generation == captured_generation
                    && submitted_owner
                        .as_ref()
                        .is_some_and(|owner| Some(owner) == current_owner.as_ref());
                let live_scope = runtime.scope();
                let accepted_name = runtime
                    .model()
                    .accepted
                    .as_ref()
                    .and_then(|snapshot| {
                        snapshot.document.board_outlines.iter().find(|outline| {
                            runtime
                                .scope()
                                .as_ref()
                                .is_some_and(|scope| outline.board_id == scope.board_id)
                        })
                    })
                    .and_then(|outline| {
                        outline.active_version_id.as_ref().and_then(|id| {
                            outline.versions.iter().find(|version| &version.id == id)
                        })
                    })
                    .map(|version| version.name.clone())
                    .unwrap_or_default();
                let results = pending.settle(owner_is_live, |key| match key {
                    OutlinePendingKey::VersionName => accepted_name.clone(),
                    _ => String::new(),
                });
                for result in results {
                    match result {
                        PendingEditResult::Failed { key, message, .. } => {
                            if key == OutlinePendingKey::VersionName {
                                version_name_failure.set(Some(message));
                                continue;
                            }
                            if let Some(scope) = live_scope.as_ref() {
                                feedback.set(Some(OutlineFeedback {
                                    scope: scope.clone(),
                                    generation: captured_generation,
                                    board_id: scope.board_id.clone(),
                                    message: Some(message),
                                }));
                            }
                        }
                        PendingEditResult::Landed { .. } | PendingEditResult::Retired { .. } => {
                            // Accepted state is the success confirmation; retirement is silent.
                        }
                    }
                }
            }
        },
    ));

    (
        project_inspector(&runtime, action_state, on_action),
        on_action,
    )
}

fn project_inspector(
    runtime: &Runtime,
    state: ActionState,
    on_action: EventHandler<OutlineAction>,
) -> Option<OutlineInspectorProjection> {
    let ActionState {
        pending: _,
        feedback,
        one_shot_pending,
        version_name_draft,
        version_name_failure,
        pending_owner: _,
        inspector_mounted,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
        selected_context,
        workspace,
        scope_generation,
        captured_generation,
    } = state;
    let model = runtime.model();
    let scope = runtime.scope()?;
    if workspace() != "Layout" || scope_generation() != captured_generation {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let selected = selected_context.read().clone()?;
    if selected.scope != scope
        || !matches!(
            &selected.context,
            crate::objects::TreeContext::Outline { .. }
                | crate::objects::TreeContext::OutlineVersion { .. }
        )
        || !crate::selection::context_is_current(&model, &scope, &selected.context)
    {
        return None;
    }
    let board_id = match &selected.context {
        crate::objects::TreeContext::Outline { board_id }
        | crate::objects::TreeContext::OutlineVersion { board_id, .. } => board_id.clone(),
        _ => return None,
    };
    if board_id != scope.board_id {
        return None;
    }
    let board_name = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)?
        .name
        .clone();
    let outline = snapshot
        .document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    let active_version_id = outline.and_then(|state| state.active_version_id.clone());
    let version_name = outline
        .and_then(|state| {
            state
                .versions
                .iter()
                .find(|version| Some(&version.id) == active_version_id.as_ref())
        })
        .map_or_else(|| "Generated".to_owned(), |version| version.name.clone());
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)?;
    let versions = outline
        .into_iter()
        .flat_map(|state| state.versions.iter())
        .map(|version| OutlineVersionChoice {
            id: version.id.clone(),
            name: version.name.clone(),
        })
        .collect();
    let generated = generated_feature(&snapshot.document, board);
    let settings = outline
        .and_then(|state| {
            active_version_id.as_ref().and_then(|active_id| {
                state
                    .versions
                    .iter()
                    .find(|version| &version.id == active_id)
                    .map(|version| version.geometry.settings.clone())
            })
        })
        .or_else(|| generated.and_then(generated_settings))
        .unwrap_or_default();
    let repair = settings.repair.clone().unwrap_or_default();
    let gaps = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .map_or_else(Vec::new, |scene| scene.gaps.clone());
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map_or_else(Vec::new, |board| board.contours.clone());
    let geometry_features = active_version_id
        .as_deref()
        .and_then(|version_id| {
            outline
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == version_id)
                .map(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                        .cloned()
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_else(|| {
            board
                .outline_ids
                .iter()
                .filter_map(|id| {
                    snapshot
                        .document
                        .outline
                        .iter()
                        .find(|feature| feature.id() == id)
                })
                .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .cloned()
                .collect()
        });
    let connections = active_version_id
        .as_deref()
        .and_then(|version_id| {
            outline
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == version_id)
                .and_then(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .find_map(|feature| match feature {
                            OutlineFeature::PartEnvelope { connections, .. } => {
                                Some(connections.clone())
                            }
                            _ => None,
                        })
                })
        })
        .or_else(|| {
            generated.and_then(|feature| match feature {
                OutlineFeature::PartEnvelope { connections, .. } => Some(connections.clone()),
                _ => None,
            })
        })
        .unwrap_or_default();
    let connection_feature = active_version_id
        .as_deref()
        .and_then(|version_id| {
            outline
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == version_id)
                .and_then(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
                        .cloned()
                })
        })
        .or_else(|| generated.cloned());
    let outline_parts = board
        .part_ids
        .iter()
        .filter_map(|id| snapshot.document.parts.iter().find(|part| &part.id == id))
        .cloned()
        .collect::<Vec<_>>();
    let perimeter = editable_perimeter(
        snapshot,
        &board_id,
        active_version_id.as_deref(),
        selected_feature_id.read().as_deref(),
    );
    let snap_paths = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .map(|scene| {
            scene
                .source_contours
                .iter()
                .map(|contour| contour.points.clone())
                .collect()
        })
        .unwrap_or_default();
    // Outline edits queue behind each other, so the panel stays editable while an earlier
    // edit is applying or saving.
    let editable = matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) && matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && model.display_preview.is_none()
        && model.gesture.is_none();
    let one_shot_pending = one_shot_pending();
    let version_name_draft = version_name_draft;
    let version_name_failure = version_name_failure;
    let visible_feedback = feedback
        .read()
        .as_ref()
        .filter(|state| {
            state.board_id == board_id
                && state.scope == scope
                && state.generation == captured_generation
        })
        .cloned();
    Some(OutlineInspectorProjection {
        board_id: board_id.clone(),
        board_name,
        version_name,
        active_version_id,
        contours,
        perimeter,
        snap_paths,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
        geometry_features,
        connections,
        connection_feature,
        outline_parts,
        versions,
        settings,
        repair,
        generated_margin: generated.and_then(generated_margin),
        has_generated: generated.is_some(),
        gaps,
        action_context: Rc::new(OutlineActionContext {
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation: captured_generation,
            board_id: board_id.clone(),
            selection_context: selected.context,
        }),
        enabled: editable,
        one_shot_pending,
        version_name_draft,
        version_name_failure,
        inspector_mounted,
        feedback: visible_feedback,
        on_action,
        selected_context,
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        generation: captured_generation,
    })
}

fn outline_owner(
    runtime: &Runtime,
    selected: Option<crate::objects::ScopedTreeContext>,
    generation: u64,
) -> Option<OutlineOwner> {
    let scope = runtime.scope()?;
    let selected = selected?;
    if selected.scope != scope
        || !matches!(
            &selected.context,
            crate::objects::TreeContext::Outline { .. }
                | crate::objects::TreeContext::OutlineVersion { .. }
        )
        || !crate::selection::context_is_current(&runtime.model(), &scope, &selected.context)
    {
        return None;
    }
    let board_id = match &selected.context {
        crate::objects::TreeContext::Outline { board_id }
        | crate::objects::TreeContext::OutlineVersion { board_id, .. } => board_id,
        _ => return None,
    };
    let active_version_id = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .document
                .board_outlines
                .iter()
                .find(|outline| &outline.board_id == board_id)
        })
        .and_then(|outline| outline.active_version_id.clone());
    Some(OutlineOwner {
        scope,
        context: selected.context,
        active_version_id,
        generation,
    })
}

pub(super) fn editable_perimeter(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    active_version_id: Option<&str>,
    selected_feature_id: Option<&str>,
) -> Option<EditablePerimeter> {
    if let Some(version_id) = active_version_id {
        let feature = snapshot
            .document
            .board_outlines
            .iter()
            .find(|state| state.board_id == board_id)?
            .versions
            .iter()
            .find(|version| version.id == version_id)?
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
                .map(|point| {
                    anchor_part_id
                        .as_deref()
                        .and_then(|id| snapshot.document.parts.iter().find(|part| part.id == id))
                        .map(|part| PerimeterAnchor {
                            at: part.pose.at,
                            rotation: part.pose.rotation,
                            back: part.side == Side::Back,
                        })
                        .map_or(*point, |anchor| anchor.world(*point))
                })
                .collect(),
            anchor: anchor_part_id
                .as_deref()
                .and_then(|id| snapshot.document.parts.iter().find(|part| part.id == id))
                .map(|part| PerimeterAnchor {
                    at: part.pose.at,
                    rotation: part.pose.rotation,
                    back: part.side == Side::Back,
                }),
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

fn submit_action(runtime: &Rc<Runtime>, state: ActionState, action: OutlineAction) {
    let pending = state.pending.clone();
    let mut pending_owner = state.pending_owner;
    let selected_context = state.selected_context;
    let mut feedback = state.feedback;
    let one_shot_pending = state.one_shot_pending;
    let version_name_draft = state.version_name_draft;
    let mut selected_context = selected_context;
    let workspace = state.workspace;
    let scope_generation = state.scope_generation;
    let captured_generation = state.captured_generation;
    if workspace() != "Layout" || scope_generation() != captured_generation {
        return;
    }
    if !action.is_current(runtime, captured_generation) {
        return;
    }
    let (action_scope, _, _, _, board_id) = action.envelope();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let selected = selected_context.read().clone();
    let selection_is_current = selected.as_ref().is_some_and(|selected| {
        selected.scope == *action_scope
            && matches!(&selected.context,
                crate::objects::TreeContext::Outline { board_id: selected_board }
                | crate::objects::TreeContext::OutlineVersion { board_id: selected_board, .. }
                if selected_board == board_id)
            && crate::selection::context_is_current(&model, action_scope, &selected.context)
    });
    if !selection_is_current {
        return;
    }
    if let OutlineAction::FocusGap { gap_id, .. } = &action {
        let Some(gap) = snapshot
            .scene
            .board_outline_scenes
            .iter()
            .find(|scene| scene.board_id == *board_id)
            .and_then(|scene| scene.gaps.iter().find(|gap| gap.id == *gap_id))
        else {
            return;
        };
        let points = gap
            .points
            .iter()
            .map(|point| connection_point_world(point, &snapshot.document.parts))
            .filter(|point| point.x.is_finite() && point.y.is_finite())
            .collect::<Vec<_>>();
        let Some((min_x, max_x, min_y, max_y)) =
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
                })
        else {
            return;
        };
        let Some((base_min_x, base_max_x, base_min_y, base_max_y)) =
            crate::keycaps_fit::layout_canvas_bounds(&snapshot.document, &snapshot.scene, board_id)
        else {
            return;
        };
        // Match React's canvas fit around a contour: include its 12 mm breathing room and
        // preserve the target offset relative to the stable Layout camera basis.
        let target_width = (max_x - min_x + 24.0).max(1.0);
        let target_height = (max_y - min_y + 24.0).max(1.0);
        let base_width = (base_max_x - base_min_x).max(50.0);
        let base_height = (base_max_y - base_min_y).max(50.0);
        let (surface_width, surface_height) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.query_selector("svg.m1-canvas").ok().flatten())
            .map(|surface| {
                let bounds = surface.get_bounding_client_rect();
                (bounds.width().max(1.0), bounds.height().max(1.0))
            })
            .unwrap_or((800.0, 600.0));
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
        runtime.submit(Event::SetCamera {
            operation_id: runtime.operation(),
            center,
            zoom,
        });
        return;
    }
    let seed = runtime.operation().0;
    // Admission at dispatch: an action the panel's snapshot cannot support is ignored early,
    // exactly as before. The resolver plans again against whatever is accepted at execution.
    if plan_action(snapshot, &action, seed).is_err() {
        return;
    }
    let pending_key = action_pending_key(&action);
    let one_shot = matches!(&pending_key, OutlinePendingKey::OneShot);
    let is_version_name = matches!(&pending_key, OutlinePendingKey::VersionName);
    if one_shot && one_shot_pending() {
        return;
    }
    if let OutlineAction::Delete { .. } = &action {
        selected_context.set(Some(crate::objects::ScopedTreeContext {
            scope: action_scope.clone(),
            context: crate::objects::TreeContext::Outline {
                board_id: board_id.clone(),
            },
        }));
    }
    if let OutlineAction::EditPerimeter {
        phase: EditPhase::Preview,
        transaction_id,
        ..
    } = &action
    {
        let Ok((operation, target_ids)) = plan_action(snapshot, &action, seed) else {
            return;
        };
        runtime.submit(Event::PreviewEdit {
            operation_id: runtime.operation(),
            transaction_id: transaction_id.clone(),
            target_ids,
            operation,
        });
        return;
    }
    let transaction_id = match &action {
        OutlineAction::EditPerimeter { transaction_id, .. } => transaction_id.clone(),
        _ => String::new(),
    };
    let resolver = action_resolver(action.clone(), seed, transaction_id);
    feedback.set(None);
    pending_owner.set(outline_owner(
        runtime,
        selected_context.read().clone(),
        captured_generation,
    ));
    if one_shot {
        pending.begin_one_shot(
            runtime,
            pending_key,
            "layout-outline",
            Some("outline".into()),
            resolver,
        );
    } else {
        let submitted_draft = version_name_draft.peek().clone();
        pending.begin_field(
            runtime,
            pending_key,
            "layout-outline",
            Some("outline".into()),
            resolver,
            if is_version_name {
                &submitted_draft
            } else {
                ""
            },
        );
    }
}

fn action_pending_key(action: &OutlineAction) -> OutlinePendingKey {
    match action {
        OutlineAction::Update { edit, .. } => match edit {
            OutlineEdit::RenameVersion { .. } => OutlinePendingKey::VersionName,
            OutlineEdit::CreateAutomatic | OutlineEdit::RemoveProtectedGap { .. } => {
                OutlinePendingKey::OneShot
            }
            OutlineEdit::SetMargin(_) => OutlinePendingKey::Setting(OutlineSettingKey::Margin),
            OutlineEdit::SetCorners(_) => OutlinePendingKey::Setting(OutlineSettingKey::Corners),
            OutlineEdit::SetSize(_) => OutlinePendingKey::Setting(OutlineSettingKey::Size),
            OutlineEdit::SetBridgeWidth(_) => {
                OutlinePendingKey::Setting(OutlineSettingKey::BridgeWidth)
            }
            OutlineEdit::SetRepairEnabled(_) => {
                OutlinePendingKey::Setting(OutlineSettingKey::RepairEnabled)
            }
            OutlineEdit::SetMaximumGapSpan(_) => {
                OutlinePendingKey::Setting(OutlineSettingKey::MaximumGapSpan)
            }
            OutlineEdit::SetMinimumConnectionWidth(_) => {
                OutlinePendingKey::Setting(OutlineSettingKey::MinimumConnectionWidth)
            }
            OutlineEdit::SetEdgeClearance(_) => {
                OutlinePendingKey::Setting(OutlineSettingKey::EdgeClearance)
            }
            OutlineEdit::SetProtectedGap { gap_id, .. } => {
                OutlinePendingKey::Setting(OutlineSettingKey::ProtectedGap(gap_id.clone()))
            }
        },
        OutlineAction::EditPerimeter { target, .. } => OutlinePendingKey::Perimeter(target.clone()),
        OutlineAction::SetFeature { before, after, .. } => {
            let field = feature_edit_field(before, after);
            if matches!(
                &field,
                OutlineFeatureField::Connections | OutlineFeatureField::ConnectionPoints(_)
            ) {
                OutlinePendingKey::OneShot
            } else {
                OutlinePendingKey::Feature {
                    feature_id: before.id().to_owned(),
                    field,
                }
            }
        }
        OutlineAction::Activate { .. }
        | OutlineAction::Copy { .. }
        | OutlineAction::Delete { .. }
        | OutlineAction::AddFeature { .. }
        | OutlineAction::AddConnection { .. }
        | OutlineAction::RemoveFeature { .. }
        | OutlineAction::FocusGap { .. } => OutlinePendingKey::OneShot,
    }
}

fn feature_edit_field(before: &OutlineFeature, after: &OutlineFeature) -> OutlineFeatureField {
    match (before, after) {
        (
            OutlineFeature::Rect {
                center: old_center,
                size: old_size,
                radius: old_radius,
                rotation: old_rotation,
                ..
            },
            OutlineFeature::Rect {
                center: new_center,
                size: new_size,
                radius: new_radius,
                rotation: new_rotation,
                ..
            },
        ) => {
            if old_center.x != new_center.x {
                OutlineFeatureField::RectCenterX
            } else if old_center.y != new_center.y {
                OutlineFeatureField::RectCenterY
            } else if old_size.x != new_size.x {
                OutlineFeatureField::RectWidth
            } else if old_size.y != new_size.y {
                OutlineFeatureField::RectHeight
            } else if old_radius != new_radius {
                OutlineFeatureField::RectRadius
            } else if old_rotation != new_rotation {
                OutlineFeatureField::RectRotation
            } else {
                OutlineFeatureField::Whole
            }
        }
        (
            OutlineFeature::PartEnvelope {
                connections: old_connections,
                ..
            },
            OutlineFeature::PartEnvelope {
                connections: new_connections,
                ..
            },
        ) => connection_edit_field(old_connections, new_connections),
        _ => OutlineFeatureField::Whole,
    }
}

fn connection_edit_field(
    before: &[OutlineConnection],
    after: &[OutlineConnection],
) -> OutlineFeatureField {
    if before.len() != after.len()
        || before
            .iter()
            .map(|item| &item.id)
            .ne(after.iter().map(|item| &item.id))
    {
        return OutlineFeatureField::Connections;
    }
    for (old, new) in before.iter().zip(after) {
        if old.width != new.width {
            return OutlineFeatureField::ConnectionWidth(old.id.clone());
        }
        if old.points.len() != new.points.len() {
            return OutlineFeatureField::ConnectionPoints(old.id.clone());
        }
        for (index, (old_point, new_point)) in old.points.iter().zip(&new.points).enumerate() {
            if old_point.at.x != new_point.at.x {
                return OutlineFeatureField::ConnectionPointX(old.id.clone(), index);
            }
            if old_point.at.y != new_point.at.y {
                return OutlineFeatureField::ConnectionPointY(old.id.clone(), index);
            }
            if old_point.part_id != new_point.part_id {
                return OutlineFeatureField::ConnectionPointAttachment(old.id.clone(), index);
            }
        }
    }
    OutlineFeatureField::Whole
}

/// Resolve one outline action against the accepted document at execution time.
pub(super) fn action_resolver(
    action: OutlineAction,
    seed: u64,
    transaction_id: String,
) -> EditResolver {
    EditResolver::new(
        "layout-outline",
        move |accepted: &AcceptedSnapshot| match plan_action(accepted, &action, seed) {
            Ok((operation, target_ids)) => Resolution::submit_with_transaction_id(
                target_ids,
                transaction_id.clone(),
                operation,
            ),
            Err(Skip::Unchanged) => Resolution::Unchanged,
            Err(Skip::Retire(reason)) => Resolution::Retire(reason),
        },
    )
}
impl OutlineInspectorProjection {
    pub(super) fn copy_action(&self) -> OutlineAction {
        OutlineAction::Copy {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
        }
    }

    pub(super) fn delete_action(&self) -> Option<OutlineAction> {
        self.active_version_id
            .as_ref()
            .map(|version_id| OutlineAction::Delete {
                scope: self.scope.clone(),
                token: self.token,
                revision: self.revision,
                generation: self.generation,
                board_id: self.board_id.clone(),
                version_id: version_id.clone(),
            })
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "hook_tests.rs"]
mod hook_tests;
