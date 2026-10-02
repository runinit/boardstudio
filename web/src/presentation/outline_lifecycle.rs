use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, TerminalOutcome,
};
use boardstudio_core::model::{Contour, EditCommand, EditOperation, EditPhase};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OutlineAction {
    Copy {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        board_id: String,
    },
    Delete {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        board_id: String,
        version_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PendingKind {
    Copy { version_id: String },
    Delete { version_id: String },
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
    pub(super) enabled: bool,
    pub(super) feedback: Option<OutlineFeedback>,
    pub(super) on_action: EventHandler<OutlineAction>,
    scope: Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
}

pub(super) fn use_outline_lifecycle(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> Option<OutlineInspectorProjection> {
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
                    let Some(snapshot) = model.accepted.as_ref() else {
                        return;
                    };
                    if snapshot.document.id != waiting.snapshot.document.id {
                        pending.set(None);
                        feedback.set(Some(OutlineFeedback {
                            scope: waiting.scope.clone(),
                            generation: waiting.generation,
                            board_id: waiting.scope.board_id.clone(),
                            state: "source-changed",
                            message: Some(
                                "The project changed before the outline result could be confirmed."
                                    .into(),
                            ),
                        }));
                        return;
                    }
                    if snapshot.session_epoch == waiting.snapshot.session_epoch
                        && (snapshot.token == waiting.snapshot.token
                            || snapshot.document.revision <= waiting.snapshot.document.revision)
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
                        PendingKind::Copy { version_id } => board_state.is_some_and(|state| {
                            state.active_version_id.as_deref() == Some(version_id)
                                && state
                                    .versions
                                    .iter()
                                    .any(|version| version.id == *version_id)
                        }),
                        PendingKind::Delete { version_id } => board_state.is_none_or(|state| {
                            state.active_version_id.is_none()
                                && state
                                    .versions
                                    .iter()
                                    .all(|version| version.id != *version_id)
                        }),
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
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map_or_else(Vec::new, |board| board.contours.clone());
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
        board_id,
        board_name,
        version_name,
        active_version_id,
        contours,
        enabled: editable,
        feedback: pending_feedback.or(visible_feedback),
        on_action,
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
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
    let (action_scope, expected_token, expected_revision, board_id) = match &action {
        OutlineAction::Copy {
            scope,
            token,
            revision,
            board_id,
        }
        | OutlineAction::Delete {
            scope,
            token,
            revision,
            board_id,
            ..
        } => (scope, token, *revision, board_id),
    };
    if runtime.scope().as_ref() != Some(action_scope) {
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    if snapshot.token != *expected_token
        || snapshot.document.revision != expected_revision
        || snapshot.document.id != action_scope.document_id
        || snapshot.session_epoch != action_scope.session_epoch
        || action_scope.board_id != *board_id
        || model.lifecycle != Lifecycle::Ready
        || model.durability
            != (Durability::Saved {
                revision: expected_revision,
            })
        || model.display_preview.is_some()
        || model.gesture.is_some()
    {
        return;
    }
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
    let (operation, kind) = match &action {
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
                let candidate = format!("outline-version-{}", runtime.operation().0);
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
                PendingKind::Copy { version_id },
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
            )
        }
    };
    let operation_id = runtime.operation();
    let outcome = runtime.observe_operation(operation_id);
    pending.set(Some(Pending {
        scope: action_scope.clone(),
        snapshot: snapshot.clone(),
        generation: captured_generation,
        kind,
        outcome,
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
            target_ids: vec![board_id.clone()],
            operation,
        },
    });
}

#[component]
pub(super) fn OutlineVersionInspector(projection: OutlineInspectorProjection) -> Element {
    let contour_view = contour_preview(&projection.contours);
    let copy = OutlineAction::Copy {
        scope: projection.scope.clone(),
        token: projection.token,
        revision: projection.revision,
        board_id: projection.board_id.clone(),
    };
    let delete = projection
        .active_version_id
        .as_ref()
        .map(|version_id| OutlineAction::Delete {
            scope: projection.scope.clone(),
            token: projection.token,
            revision: projection.revision,
            board_id: projection.board_id.clone(),
            version_id: version_id.clone(),
        });
    let copy_handler = projection.on_action;
    let delete_handler = projection.on_action;
    rsx! {
        section { class: "m1-outline-inspector", "aria-label": "Board outline",
            div { class: "m1-outline-inspector-heading", h2 { "Board outline" } span { class: "m1-outline-board-name", "{projection.board_name}" } }
            p { if projection.active_version_id.is_some() { "A fixed outline; component placement is shared with every version." } else { "Generated follows your keycaps and included components." } }
            p { "Active version: {projection.version_name}" }
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
