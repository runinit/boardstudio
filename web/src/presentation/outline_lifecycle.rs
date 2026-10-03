use crate::outline_settings::{
    OutlineEdit, OutlineExpectation, apply_outline_edit, expectation_applied, generated_feature,
    generated_margin, generated_settings,
};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, TerminalOutcome,
};
use boardstudio_core::model::{
    Contour, CornerStyle, EditCommand, EditOperation, EditPhase, Operation, OutlineContourEdit,
    OutlineFeature, OutlineGap, OutlineRepairSettings, OutlineSettings, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlineAction {
    Activate {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        context: super::objects::TreeContext,
        require_selected_version: bool,
        board_id: String,
        version_id: Option<String>,
    },
    Copy {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
    },
    Delete {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        version_id: String,
    },
    Update {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        edit: OutlineEdit,
    },
    EditPerimeter {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        target: OutlinePointTarget,
        points: Vec<Vec2>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlinePointTarget {
    Generated {
        contour: u32,
    },
    Fixed {
        version_id: String,
        feature_id: String,
        anchor_part_id: Option<String>,
        operation: Operation,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct EditablePerimeter {
    target: OutlinePointTarget,
    points: Vec<Vec2>,
}

#[derive(Clone, Debug, PartialEq)]
enum PendingKind {
    Activate {
        version_id: Option<String>,
    },
    Copy {
        version_id: String,
        edited_contour: Option<(u32, Vec<Vec2>)>,
    },
    Delete {
        version_id: String,
    },
    Edit(OutlineExpectation),
}

#[derive(Clone)]
struct Pending {
    scope: Scope,
    snapshot: AcceptedSnapshot,
    generation: u64,
    kind: PendingKind,
    outcome: OutcomeSlot,
}

#[derive(Clone, Copy)]
struct ActionState {
    pending: Signal<Option<Pending>>,
    feedback: Signal<Option<OutlineFeedback>>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    captured_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OutlineFeedback {
    pub(super) scope: Scope,
    pub(super) generation: u64,
    pub(super) board_id: String,
    pub(super) state: &'static str,
    pub(super) message: Option<String>,
}

#[derive(Clone, PartialEq)]
pub(super) struct OutlineInspectorProjection {
    pub(super) board_id: String,
    pub(super) board_name: String,
    pub(super) version_name: String,
    pub(super) active_version_id: Option<String>,
    pub(super) contours: Vec<Contour>,
    perimeter: Option<EditablePerimeter>,
    pub(super) versions: Vec<OutlineVersionChoice>,
    pub(super) settings: OutlineSettings,
    pub(super) repair: OutlineRepairSettings,
    pub(super) generated_margin: Option<f64>,
    pub(super) has_generated: bool,
    pub(super) gaps: Vec<OutlineGap>,
    action_context: Rc<OutlineActionContext>,
    pub(super) enabled: bool,
    pub(super) feedback: Option<OutlineFeedback>,
    pub(super) on_action: EventHandler<OutlineAction>,
    scope: Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct OutlineActionContext {
    scope: Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
    generation: u64,
    board_id: String,
    selection_context: super::objects::TreeContext,
}

impl OutlineActionContext {
    fn action(&self, edit: OutlineEdit) -> OutlineAction {
        OutlineAction::Update {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            edit,
        }
    }

    fn activate_action(&self, version_id: Option<String>) -> OutlineAction {
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

    fn edit_perimeter(&self, target: OutlinePointTarget, points: Vec<Vec2>) -> OutlineAction {
        OutlineAction::EditPerimeter {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            target,
            points,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OutlineVersionChoice {
    pub(super) id: String,
    pub(super) name: String,
}

impl OutlineAction {
    fn envelope(
        &self,
    ) -> (
        &Scope,
        &boardstudio_application::SnapshotToken,
        u64,
        u64,
        &String,
    ) {
        match self {
            OutlineAction::Activate {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Copy {
                scope,
                token,
                revision,
                generation,
                board_id,
            }
            | OutlineAction::Delete {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Update {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::EditPerimeter {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            } => (scope, token, *revision, *generation, board_id),
        }
    }

    pub(super) fn is_current(&self, runtime: &Runtime, generation: u64) -> bool {
        let (scope, token, revision, captured_generation, board_id) = self.envelope();
        let model = runtime.model();
        generation == captured_generation
            && runtime.scope().as_ref() == Some(scope)
            && model.accepted.as_ref().is_some_and(|snapshot| {
                snapshot.token == *token
                    && snapshot.document.revision == revision
                    && snapshot.document.id == scope.document_id
                    && snapshot.session_epoch == scope.session_epoch
            })
            && scope.board_id == *board_id
            && model.lifecycle == Lifecycle::Ready
            && model.durability == (Durability::Saved { revision })
            && model.display_preview.is_none()
            && model.gesture.is_none()
    }

    pub(super) fn for_tree(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        generation: u64,
        context: &super::objects::TreeContext,
    ) -> Option<Self> {
        let super::objects::TreeContext::OutlineVersion {
            board_id,
            version_id,
        } = context
        else {
            return None;
        };
        Some(Self::Activate {
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation,
            context: context.clone(),
            require_selected_version: true,
            board_id: board_id.clone(),
            version_id: version_id.clone(),
        })
    }
}

pub(super) fn use_outline_lifecycle(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> (
    Option<OutlineInspectorProjection>,
    EventHandler<OutlineAction>,
) {
    let version = use_context::<Signal<u64>>()();
    let captured_generation = scope_generation();
    let pending = use_signal(|| None::<Pending>);
    let feedback = use_signal(|| None::<OutlineFeedback>);
    let action_state = ActionState {
        pending,
        feedback,
        selected_context,
        workspace,
        scope_generation,
        captured_generation,
    };
    let on_action = use_callback({
        let runtime = runtime.clone();
        move |action| {
            submit_action(&runtime, action_state, action);
        }
    });

    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow().clone() else {
                return;
            };
            match outcome {
                TerminalOutcome::Completed => {
                    let model = runtime.model();
                    let Some(snapshot) = model.accepted.as_ref().filter(|snapshot| {
                        snapshot.document.id == waiting.snapshot.document.id
                            && snapshot.session_epoch == waiting.snapshot.session_epoch
                            && runtime.scope().as_ref() == Some(&waiting.scope)
                    }) else {
                        pending.set(None);
                        feedback.set(Some(OutlineFeedback {
                            scope: waiting.scope.clone(),
                            generation: waiting.generation,
                            board_id: waiting.scope.board_id.clone(),
                            state: "source-changed",
                            message: Some(
                                "The outline source changed before its result could be confirmed."
                                    .into(),
                            ),
                        }));
                        return;
                    };
                    if matches!(model.durability, Durability::Failed { .. })
                        || matches!(
                            model.lifecycle,
                            Lifecycle::RecoveryRequired | Lifecycle::Closed
                        )
                    {
                        pending.set(None);
                        feedback.set(Some(OutlineFeedback {
                            scope: waiting.scope.clone(),
                            generation: waiting.generation,
                            board_id: waiting.scope.board_id.clone(),
                            state: "recovery-required",
                            message: Some("The outline operation completed, but the accepted document is not durably available. Retry after recovery.".into()),
                        }));
                        return;
                    }
                    if snapshot.token == waiting.snapshot.token
                        || snapshot.document.revision <= waiting.snapshot.document.revision
                    {
                        return;
                    }
                    if model.lifecycle != Lifecycle::Ready
                        || model.durability
                            != (Durability::Saved {
                                revision: snapshot.document.revision,
                            })
                    {
                        return;
                    }
                    let board_state = snapshot
                        .document
                        .board_outlines
                        .iter()
                        .find(|state| state.board_id == waiting.scope.board_id);
                    let applied = match &waiting.kind {
                        PendingKind::Activate { version_id } => {
                            board_state.and_then(|state| state.active_version_id.as_ref())
                                == version_id.as_ref()
                        }
                        PendingKind::Copy {
                            version_id,
                            edited_contour,
                        } => board_state.is_some_and(|state| {
                            state.active_version_id.as_deref() == Some(version_id)
                                && state.versions.iter().any(|version| {
                                    version.id == *version_id
                                        && edited_contour.as_ref().is_none_or(
                                            |(contour, points)| {
                                                matches!(
                                                    version.geometry.features.get(*contour as usize),
                                                    Some(OutlineFeature::Polygon {
                                                        points: accepted,
                                                        ..
                                                    }) if accepted == points
                                                )
                                            },
                                        )
                                })
                        }),
                        PendingKind::Delete { version_id } => board_state.is_none_or(|state| {
                            state.active_version_id.is_none()
                                && state
                                    .versions
                                    .iter()
                                    .all(|version| version.id != *version_id)
                        }),
                        PendingKind::Edit(expectation) => expectation_applied(
                            &snapshot.document,
                            &waiting.scope.board_id,
                            expectation,
                        ),
                    };
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: if applied { "saved" } else { "failed" },
                        message: (!applied).then(|| "The saved outline no longer matches this action. Review the accepted version and retry.".into()),
                    }));
                }
                TerminalOutcome::Rejected(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "rejected",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::PersistenceFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "persistence-failed",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::BlockedByRecovery(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "recovery-required",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::ExecutorFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "executor-failed",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "cancelled",
                        message: Some(
                            "The outline change did not complete in the active session.".into(),
                        ),
                    }));
                }
            }
        }
    }));

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
        pending,
        feedback,
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
            selected.context,
            super::objects::TreeContext::Outline { .. }
                | super::objects::TreeContext::OutlineVersion { .. }
        )
        || !super::selection::context_is_current(&model, &scope, &selected.context)
    {
        return None;
    }
    let board_id = match &selected.context {
        super::objects::TreeContext::Outline { board_id }
        | super::objects::TreeContext::OutlineVersion { board_id, .. } => board_id.clone(),
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
    let perimeter = editable_perimeter(snapshot, &board_id, active_version_id.as_deref());
    let editable = model.durability
        == Durability::Saved {
            revision: snapshot.document.revision,
        }
        && model.lifecycle == Lifecycle::Ready
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && pending.read().is_none();
    let visible_feedback = feedback
        .read()
        .as_ref()
        .filter(|state| {
            state.board_id == board_id
                && state.scope == scope
                && state.generation == captured_generation
        })
        .cloned();
    let pending_feedback = pending
        .read()
        .as_ref()
        .filter(|waiting| waiting.scope == scope && waiting.generation == captured_generation)
        .map(|waiting| OutlineFeedback {
            scope: waiting.scope.clone(),
            generation: waiting.generation,
            board_id: waiting.scope.board_id.clone(),
            state: "pending",
            message: None,
        });
    Some(OutlineInspectorProjection {
        board_id: board_id.clone(),
        board_name,
        version_name,
        active_version_id,
        contours,
        perimeter,
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
        feedback: pending_feedback.or(visible_feedback),
        on_action,
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        generation: captured_generation,
    })
}

fn editable_perimeter(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    active_version_id: Option<&str>,
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
            .first()?;
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
    })
}

fn submit_action(runtime: &Rc<Runtime>, state: ActionState, action: OutlineAction) {
    let mut pending = state.pending;
    let mut feedback = state.feedback;
    let mut selected_context = state.selected_context;
    let workspace = state.workspace;
    let scope_generation = state.scope_generation;
    let captured_generation = state.captured_generation;
    if pending.read().is_some()
        || workspace() != "Layout"
        || scope_generation() != captured_generation
    {
        return;
    }
    if !action.is_current(runtime, captured_generation) {
        return;
    }
    let (action_scope, _, expected_revision, _, board_id) = action.envelope();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let selected = selected_context.read().clone();
    let selection_is_current = selected.as_ref().is_some_and(|selected| {
        selected.scope == *action_scope
            && matches!(&selected.context,
                super::objects::TreeContext::Outline { board_id: selected_board }
                | super::objects::TreeContext::OutlineVersion { board_id: selected_board, .. }
                if selected_board == board_id)
            && super::selection::context_is_current(&model, action_scope, &selected.context)
    });
    if !selection_is_current {
        return;
    }
    let state = snapshot
        .document
        .board_outlines
        .iter()
        .find(|state| state.board_id == *board_id);
    let operation_id = runtime.operation();
    let (operation, kind, target_ids) = match &action {
        OutlineAction::Activate {
            version_id,
            context,
            require_selected_version,
            ..
        } => {
            if !selected.as_ref().is_some_and(|selected| {
                if selected.context != *context {
                    return false;
                }
                match (&selected.context, version_id) {
                    (
                        super::objects::TreeContext::Outline {
                            board_id: selected_board,
                        },
                        _,
                    ) => selected_board == board_id,
                    (
                        super::objects::TreeContext::OutlineVersion {
                            board_id: selected_board,
                            version_id: selected_version,
                        },
                        version_id,
                    ) => {
                        selected_board == board_id
                            && (!require_selected_version || selected_version == version_id)
                    }
                    _ => false,
                }
            }) || version_id.as_ref().is_some_and(|id| {
                !state.is_some_and(|state| state.versions.iter().any(|version| version.id == *id))
            }) || state.and_then(|state| state.active_version_id.as_ref()) == version_id.as_ref()
            {
                return;
            }
            (
                EditOperation::SelectOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                PendingKind::Activate {
                    version_id: version_id.clone(),
                },
                vec![board_id.clone()],
            )
        }
        OutlineAction::Copy { .. } => {
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
            let name = format!("Edited outline {next_number}");
            let version_id = loop {
                let candidate = format!("outline-version-{}", operation_id.0);
                if snapshot
                    .document
                    .board_outlines
                    .iter()
                    .flat_map(|state| &state.versions)
                    .all(|version| version.id != candidate)
                {
                    break candidate;
                }
            };
            (
                EditOperation::CopyOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                    name,
                    edit: None,
                    feature: None,
                },
                PendingKind::Copy {
                    version_id,
                    edited_contour: None,
                },
                vec![board_id.clone()],
            )
        }
        OutlineAction::Delete { version_id, .. } => {
            if state.and_then(|state| state.active_version_id.as_deref())
                != Some(version_id.as_str())
                || !state
                    .into_iter()
                    .flat_map(|state| &state.versions)
                    .any(|version| version.id == *version_id)
            {
                return;
            }
            selected_context.set(Some(super::objects::ScopedTreeContext {
                scope: action_scope.clone(),
                context: super::objects::TreeContext::Outline {
                    board_id: board_id.clone(),
                },
            }));
            (
                EditOperation::RemoveOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                PendingKind::Delete {
                    version_id: version_id.clone(),
                },
                vec![board_id.clone()],
            )
        }
        OutlineAction::Update { edit, .. } => {
            let Some((operation, expectation, target_ids)) = apply_outline_edit(
                &snapshot.document,
                &snapshot.scene,
                board_id,
                edit,
                operation_id,
            ) else {
                return;
            };
            (operation, PendingKind::Edit(expectation), target_ids)
        }
        OutlineAction::EditPerimeter {
            context,
            target,
            points,
            ..
        } => {
            if !selected
                .as_ref()
                .is_some_and(|selected| selected.context == *context)
            {
                return;
            }
            if points.len() < 3
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return;
            }
            match target {
                OutlinePointTarget::Generated { contour } => {
                    let Some(source_points) = snapshot
                        .scene
                        .board_outline_scenes
                        .iter()
                        .find(|scene| scene.board_id == *board_id)
                        .and_then(|scene| scene.source_contours.get(*contour as usize))
                        .map(|source| &source.points)
                    else {
                        return;
                    };
                    if state
                        .and_then(|state| state.active_version_id.as_ref())
                        .is_some()
                        || source_points == points
                    {
                        return;
                    }
                    let version_id = format!("outline-version-{}", operation_id.0);
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
                    let name = format!("Edited outline {next_number}");
                    (
                        EditOperation::CopyOutline {
                            board_id: board_id.clone(),
                            version_id: version_id.clone(),
                            name,
                            edit: Some(OutlineContourEdit {
                                contour: *contour,
                                points: points.clone(),
                            }),
                            feature: None,
                        },
                        PendingKind::Copy {
                            version_id,
                            edited_contour: Some((*contour, points.clone())),
                        },
                        vec![board_id.clone()],
                    )
                }
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    anchor_part_id,
                    operation,
                } => {
                    if state.and_then(|state| state.active_version_id.as_deref())
                        != Some(version_id.as_str())
                    {
                        return;
                    }
                    let Some(OutlineFeature::Polygon {
                        id,
                        points: current,
                        anchor_part_id: current_anchor,
                        operation: current_operation,
                    }) = state
                        .into_iter()
                        .flat_map(|state| &state.versions)
                        .find(|version| version.id == *version_id)
                        .and_then(|version| {
                            version
                                .geometry
                                .features
                                .iter()
                                .find(|feature| feature.id() == *feature_id)
                        })
                    else {
                        return;
                    };
                    if current == points
                        || current_anchor != anchor_part_id
                        || current_operation != operation
                    {
                        return;
                    }
                    let feature = OutlineFeature::Polygon {
                        id: id.clone(),
                        points: points.clone(),
                        anchor_part_id: anchor_part_id.clone(),
                        operation: *operation,
                    };
                    (
                        EditOperation::SetOutline {
                            feature: feature.clone(),
                        },
                        PendingKind::Edit(OutlineExpectation::VersionFeature {
                            version_id: version_id.clone(),
                            feature,
                        }),
                        vec![board_id.clone(), feature_id.clone()],
                    )
                }
            }
        }
    };
    let outcome = runtime.observe_operation(operation_id);
    pending.set(Some(Pending {
        scope: action_scope.clone(),
        snapshot: snapshot.clone(),
        generation: captured_generation,
        kind,
        outcome: outcome.clone(),
    }));
    feedback.set(Some(OutlineFeedback {
        scope: action_scope.clone(),
        generation: captured_generation,
        board_id: board_id.clone(),
        state: "pending",
        message: None,
    }));
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: expected_revision,
            transaction_id: format!("outline-lifecycle-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        },
    });
    // Keep exact observation alive independently of the Editor's signal lifetime.
    // This task never reads or writes a component signal after an await.
    wasm_bindgen_futures::spawn_local(async move {
        while outcome.borrow().is_none() {
            gloo_timers::future::TimeoutFuture::new(16).await;
        }
    });
}

impl OutlineInspectorProjection {
    fn copy_action(&self) -> OutlineAction {
        OutlineAction::Copy {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
        }
    }

    fn delete_action(&self) -> Option<OutlineAction> {
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

#[component]
pub(super) fn OutlineVersionInspector(projection: OutlineInspectorProjection) -> Element {
    let contour_view = contour_preview(&projection.contours);
    let copy = projection.copy_action();
    let delete = projection.delete_action();
    let copy_handler = projection.on_action;
    let delete_handler = projection.on_action;
    let mut version_name = use_signal(|| None::<(String, String, String)>);
    let mut perimeter_open = use_signal(|| false);
    let mut selected_point = use_signal(|| 0usize);
    let name_draft = version_name()
        .filter(|(id, baseline, _)| {
            Some(id) == projection.active_version_id.as_ref()
                && baseline == &projection.version_name
        })
        .map(|(_, _, draft)| draft)
        .unwrap_or_else(|| projection.version_name.clone());
    let active_version = projection.active_version_id.clone();
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let active_value = projection.active_version_id.clone().unwrap_or_default();
    let perimeter = projection.perimeter.clone();
    let corner_value = match projection.settings.corners {
        CornerStyle::Sharp => "sharp",
        CornerStyle::Fillet => "fillet",
        CornerStyle::Chamfer => "chamfer",
    };
    let size_label: &'static str = match projection.settings.corners {
        CornerStyle::Fillet => "Fillet radius",
        _ => "Chamfer size",
    };
    let point_index = perimeter
        .as_ref()
        .map(|perimeter| selected_point().min(perimeter.points.len().saturating_sub(1)))
        .unwrap_or_default();
    let point = perimeter
        .as_ref()
        .and_then(|perimeter| perimeter.points.get(point_index).copied())
        .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
    let point_count = perimeter
        .as_ref()
        .map(|perimeter| perimeter.points.len())
        .unwrap_or_default();
    rsx! {
        if perimeter_open() {
            if let Some(perimeter) = perimeter.as_ref() {
                section { class: "m1-outline-inspector m1-outline-point-editor", "aria-label": "Perimeter",
                    div { class: "m1-outline-inspector-heading",
                        h2 { "Perimeter" }
                        button { r#type: "button", disabled: !enabled, onclick: move |_| perimeter_open.set(false), "Done" }
                    }
                    p { if projection.active_version_id.is_some() { "This outline stays fixed when components move. Changes save as you edit." } else { "The first point change creates and activates a fixed copy. Generated stays available." } }
                    h3 { class: "m1-outline-point-heading", "Point {point_index + 1} of {point_count}" }
                    div { class: "m1-outline-coordinate-fields",
                        OutlineCoordinate {
                            label: format!("Point {} X mm", point_index + 1),
                            value: point.x,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |value| {
                                    if let Some(point) = points.get_mut(point_index) { point.x = value; }
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone()));
                                }
                            },
                        }
                        OutlineCoordinate {
                            label: format!("Point {} Y mm", point_index + 1),
                            value: point.y,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |value| {
                                    if let Some(point) = points.get_mut(point_index) { point.y = value; }
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone()));
                                }
                            },
                        }
                    }
                    div { class: "m1-outline-point-actions",
                        button {
                            class: "m1-outline-insert-point",
                            r#type: "button",
                            disabled: !enabled,
                            aria_label: "Insert after {point_index + 1}",
                            onclick: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |_| {
                                    if point_count < 3 { return; }
                                    let next = (point_index + 1) % point_count;
                                    let a = points[point_index];
                                    let b = points[next];
                                    points.insert(point_index + 1, Vec2 { x: (a.x + b.x) / 2.0, y: (a.y + b.y) / 2.0 });
                                    selected_point.set(point_index + 1);
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone()));
                                }
                            },
                            "Insert after"
                        }
                        button {
                            class: "m1-outline-remove-point",
                            r#type: "button",
                            disabled: !enabled || point_count <= 3,
                            aria_label: "Remove point {point_index + 1}",
                            title: if point_count <= 3 { "Keep at least three points." } else { "Remove the selected point" },
                            onclick: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |_| {
                                    if point_count <= 3 { return; }
                                    points.remove(point_index);
                                    selected_point.set(point_index.saturating_sub(1));
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone()));
                                }
                            },
                            "Remove point"
                        }
                    }
                    div { class: "m1-outline-point-list", role: "group", aria_label: "Outline points",
                        div { class: "m1-outline-point-columns", aria_hidden: "true",
                            span { "Point" } span { "X · mm" } span { "Y · mm" }
                        }
                        for (index, point) in perimeter.points.iter().enumerate() {
                            button {
                                key: "outline-point-{index}",
                                r#type: "button",
                                aria_label: "Select outline point {index + 1}",
                                aria_pressed: "{index == point_index}",
                                onclick: move |_| selected_point.set(index),
                                span { "{index + 1}" }
                                span { "{point.x:.3}" }
                                span { "{point.y:.3}" }
                            }
                        }
                    }
                    if let Some(feedback) = projection.feedback.as_ref() {
                        p { role: if feedback.state == "pending" || feedback.state == "saved" { "status" } else { "alert" }, "data-state": feedback.state,
                            if feedback.state == "pending" { "Saving outline…" }
                            else if feedback.state == "saved" { "Saved" }
                            else { "Outline change failed: {feedback.message.as_deref().unwrap_or_default()}" }
                        }
                    }
                }
            }
        } else {
        section { class: "m1-outline-inspector", "aria-label": "Board outline",
            div { class: "m1-outline-inspector-heading", h2 { "Board outline" } span { class: "m1-outline-board-name", "{projection.board_name}" } }
            p { if projection.active_version_id.is_some() { "A fixed outline; component placement is shared with every version." } else { "Generated follows your keycaps and included components." } }
            p { "{projection.contours.len()} accepted contours" }
            if let Some((view_box, paths)) = contour_view {
                svg { class: "m1-outline-preview", role: "img", "aria-label": "Accepted active board outline", view_box: "{view_box}",
                    g { transform: "scale(1,-1)",
                        for (index, (points, hole)) in paths.iter().enumerate() {
                            polygon { key: "outline-preview-{index}", class: if *hole { "m1-outline is-hole" } else { "m1-outline" }, points: "{points}" }
                        }
                    }
                }
            }
            div { class: "m1-outline-settings",
                label { class: "m1-outline-field",
                    span { "Active outline" }
                    select {
                        aria_label: "Active outline",
                        value: "{active_value}",
                        disabled: !enabled,
                        onchange: {
                            let action_context = action_context.clone();
                            move |event: FormEvent| {
                                let value = event.value();
                                on_action.call(action_context.activate_action((!value.is_empty()).then_some(value)));
                            }
                        },
                        option { value: "", "Generated" }
                        for item in projection.versions.iter() {
                            option { key: "{item.id}", value: "{item.id}", "{item.name}" }
                        }
                    }
                }
                if let Some(version_id) = active_version.as_ref() {
                    label { class: "m1-outline-field",
                        span { "Version name" }
                        input {
                            aria_label: "Outline version name",
                            maxlength: "120",
                            value: "{name_draft}",
                            disabled: !enabled,
                            aria_invalid: name_draft.trim().is_empty(),
                            oninput: {
                                let version_id = version_id.clone();
                                let baseline = projection.version_name.clone();
                                move |event: FormEvent| version_name.set(Some((version_id.clone(), baseline.clone(), event.value())))
                            },
                            onblur: {
                                let version_id = version_id.clone();
                                let accepted_name = projection.version_name.clone();
                                let action_context = action_context.clone();
                                move |_| {
                                    let Some((draft_id, baseline, draft)) = version_name() else { return; };
                                    if draft_id != version_id || baseline != accepted_name { return; }
                                    let name = draft.trim().to_owned();
                                    if !name.is_empty() && name.len() <= 120 && name != accepted_name {
                                        on_action.call(action_context.action(OutlineEdit::RenameVersion { version_id: version_id.clone(), name }));
                                    }
                                }
                            },
                            onkeydown: {
                                move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                    "Enter" => {
                                        event.prevent_default();
                                        if let Some(input) = event.data().try_as_web_event()
                                            .and_then(|event| event.target())
                                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                        { let _ = input.blur(); }
                                    }
                                    "Escape" => {
                                        event.prevent_default();
                                        version_name.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if !projection.has_generated && projection.active_version_id.is_none() {
                    button {
                        class: "m1-outline-action",
                        disabled: !enabled,
                        onclick: {
                            let action_context = action_context.clone();
                            move |_| on_action.call(action_context.action(OutlineEdit::CreateAutomatic))
                        },
                        "Generate automatic outline"
                    }
                }
                if projection.has_generated || projection.active_version_id.is_some() {
                    label { class: "m1-outline-field",
                        span { "Corners" }
                        select {
                            aria_label: "Outline corners",
                            value: "{corner_value}",
                            disabled: !enabled,
                            onchange: {
                                let action_context = action_context.clone();
                                move |event: FormEvent| {
                                let corner = match event.value().as_str() {
                                    "fillet" => CornerStyle::Fillet,
                                    "chamfer" => CornerStyle::Chamfer,
                                    _ => CornerStyle::Sharp,
                                };
                                on_action.call(action_context.action(OutlineEdit::SetCorners(corner)));
                            }},
                            option { value: "sharp", "Sharp" }
                            option { value: "fillet", "Fillet" }
                            option { value: "chamfer", "Chamfer" }
                        }
                    }
                    if projection.settings.corners != CornerStyle::Sharp {
                        OutlineDimension {
                            label: size_label,
                            value: projection.settings.size,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetSize(value)))
                            },
                        }
                    }
                    if projection.active_version_id.is_none() {
                        if let Some(margin) = projection.generated_margin {
                            OutlineDimension {
                                label: "Outline margin",
                                value: margin,
                                minimum: 0.0,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    move |value| on_action.call(action_context.action(OutlineEdit::SetMargin(value)))
                                },
                            }
                        }
                        OutlineDimension {
                            label: "Bridge width",
                            value: projection.settings.bridge_width,
                            minimum: 0.001,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetBridgeWidth(value)))
                            },
                        }
                    }
                    if projection.active_version_id.is_none() && !projection.gaps.is_empty() {
                        fieldset { class: "m1-outline-controls", disabled: !enabled,
                            legend { "Gap repair" }
                            p { "Keep gap preserves an intentional recess and follows its source components." }
                            for (index, gap) in projection.gaps.iter().enumerate() {
                                label { class: "m1-outline-field m1-outline-gap",
                                    input {
                                        r#type: "checkbox",
                                        aria_label: "Keep gap {index + 1}",
                                        checked: gap.protected,
                                onchange: {
                                    let gap_id = gap.id.clone();
                                    let action_context = action_context.clone();
                                    move |event: FormEvent| on_action.call(action_context.action(OutlineEdit::SetProtectedGap { gap_id: gap_id.clone(), protected: event.checked() }))
                                        }
                                    }
                                    span { "Gap {index + 1} · {gap.span:.1} mm span" }
                                }
                            }
                        }
                    }
                    fieldset { class: "m1-outline-controls", disabled: !enabled,
                        legend { "Advanced cleanup and clearance" }
                        if projection.active_version_id.is_none() {
                            label { class: "m1-outline-field m1-outline-gap",
                                input {
                                    r#type: "checkbox",
                                    aria_label: "Automatic gap cleanup",
                                    checked: projection.repair.enabled,
                                    onchange: {
                                        let action_context = action_context.clone();
                                        move |event: FormEvent| on_action.call(action_context.action(OutlineEdit::SetRepairEnabled(event.checked())))
                                    },
                                }
                                span { "Automatic cleanup" }
                            }
                            OutlineDimension {
                                label: "Maximum gap span",
                                value: projection.repair.maximum_gap_span,
                                minimum: 0.0,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    move |value| on_action.call(action_context.action(OutlineEdit::SetMaximumGapSpan(value)))
                                },
                            }
                        }
                        OutlineDimension {
                            label: "Minimum connection width",
                            value: projection.repair.minimum_connection_width,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetMinimumConnectionWidth(value)))
                            },
                        }
                        OutlineDimension {
                            label: "PCB edge clearance",
                            value: projection.repair.edge_clearance,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetEdgeClearance(value)))
                            },
                        }
                        p { "Support and clearance findings block affected fabrication exports. Editing and project saving stay available." }
                        if projection.active_version_id.is_none() {
                            for (index, gap) in projection.repair.keep_gaps.iter().enumerate() {
                                button {
                                    class: "m1-outline-remove-gap",
                                    aria_label: "Remove protected gap {index + 1}",
                                    onclick: {
                                        let gap_id = gap.id.clone();
                                        let action_context = action_context.clone();
                                        move |_| on_action.call(action_context.action(OutlineEdit::RemoveProtectedGap { gap_id: gap_id.clone() }))
                                    },
                                    "Remove protected gap {index + 1}"
                                }
                            }
                        }
                    }
                }
            }
            if projection.perimeter.as_ref().is_some_and(|perimeter| perimeter.points.len() >= 3) {
                button {
                    class: "m1-outline-action",
                    r#type: "button",
                    disabled: !enabled,
                    onclick: move |_| {
                        selected_point.set(0);
                        perimeter_open.set(true);
                    },
                    "Edit perimeter points"
                }
            }
            div { class: "m1-outline-actions",
                button { r#type: "button", disabled: !projection.enabled, onclick: move |_| copy_handler.call(copy.clone()), "Copy outline" }
                if let Some(delete) = delete {
                    button { r#type: "button", disabled: !projection.enabled, onclick: move |_| delete_handler.call(delete.clone()), "Delete outline" }
                }
            }
            if let Some(feedback) = projection.feedback.as_ref() {
                p { role: if feedback.state == "pending" || feedback.state == "saved" { "status" } else { "alert" }, "data-state": feedback.state,
                    if feedback.state == "pending" { "Saving outline…" }
                    else if feedback.state == "saved" { "Saved" }
                    else { "Outline change failed: {feedback.message.as_deref().unwrap_or_default()}" }
                }
            }
        }
        }
    }
}

#[component]
fn OutlineDimension(
    label: &'static str,
    value: f64,
    minimum: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    let mut field = use_signal(|| (value, value.to_string()));
    let draft = if field().0 == value {
        field().1
    } else {
        value.to_string()
    };
    let parsed = draft.trim().parse::<f64>();
    let valid = !draft.trim().is_empty()
        && parsed.is_ok_and(|parsed| parsed.is_finite() && parsed >= minimum);
    let error = !valid;
    rsx! {
        label { class: "m1-outline-field",
            span { "{label}" }
            span { class: "m1-outline-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    min: "{minimum}",
                    value: "{draft}",
                    disabled: !editable,
                    aria_label: label,
                    aria_invalid: error,
                    oninput: move |event: FormEvent| field.set((value, event.value())),
                    onblur: move |_| {
                        if field().0 != value {
                            field.set((value, value.to_string()));
                        } else if let Ok(next) = field().1.trim().parse::<f64>()
                            && next.is_finite() && next >= minimum && next != value
                        { on_commit.call(next); }
                    },
                    onkeydown: move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                        "Enter" => {
                            event.prevent_default();
                            if let Some(input) = event.data().try_as_web_event()
                                .and_then(|event| event.target())
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            { let _ = input.blur(); }
                        }
                        "Escape" => {
                            event.prevent_default();
                            field.set((value, value.to_string()));
                        }
                        _ => {}
                    }
                }
                small { "mm" }
            }
            if error { small { role: "alert", "Enter a finite value greater than or equal to {minimum} mm." } }
        }
    }
}

#[component]
fn OutlineCoordinate(
    label: String,
    value: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    let mut field = use_signal(|| (value, value.to_string()));
    let draft = if field().0 == value {
        field().1
    } else {
        value.to_string()
    };
    let valid = !draft.trim().is_empty()
        && draft
            .trim()
            .parse::<f64>()
            .is_ok_and(|parsed| parsed.is_finite());
    rsx! {
        label { class: "m1-outline-field",
            span { "{label}" }
            span { class: "m1-outline-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    value: "{draft}",
                    disabled: !editable,
                    aria_label: "{label}",
                    aria_invalid: !valid,
                    oninput: move |event: FormEvent| field.set((value, event.value())),
                    onblur: move |_| {
                        if field().0 != value {
                            field.set((value, value.to_string()));
                        } else if let Ok(next) = field().1.trim().parse::<f64>()
                            && next.is_finite() && next != value
                        { on_commit.call(next); }
                    },
                    onkeydown: move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                        "Enter" => {
                            event.prevent_default();
                            if let Some(input) = event.data().try_as_web_event()
                                .and_then(|event| event.target())
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            { let _ = input.blur(); }
                        }
                        "Escape" => {
                            event.prevent_default();
                            field.set((value, value.to_string()));
                        }
                        _ => {}
                    }
                }
                span { "mm" }
            }
        }
    }
}

fn contour_preview(contours: &[Contour]) -> Option<(String, Vec<(String, bool)>)> {
    let mut points = contours.iter().flat_map(|contour| &contour.points);
    let first = points.next()?;
    let (min_x, max_x, min_y, max_y) = points.fold(
        (first.x, first.x, first.y, first.y),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            )
        },
    );
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let padding = width.max(height) * 0.08;
    let view_box = format!(
        "{} {} {} {}",
        min_x - padding,
        -(max_y + padding),
        width + padding * 2.0,
        height + padding * 2.0,
    );
    let paths = contours
        .iter()
        .map(|contour| {
            let points = contour
                .points
                .iter()
                .map(|point| format!("{},{}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(" ");
            (points, contour.hole)
        })
        .collect();
    Some((view_box, paths))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "outline_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "outline_lifecycle_browser_tests.rs"]
mod browser_tests;
