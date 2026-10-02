//! Editor-lifetime admission and acknowledgement for private Keymap layer edits.
use super::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, KeymapChange, KeymapConfiguration, KeymapLayer,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct PendingLayerEdit {
    editor_instance_id: u64,
    scope: Scope,
    snapshot_token: SnapshotToken,
    revision: u64,
    scope_generation: u64,
    operation_id: OperationId,
    target_layer_id: String,
    outcome: crate::operation_outcomes::OutcomeSlot,
    before: KeymapConfiguration,
    expected: KeymapConfiguration,
    select_after: Option<String>,
}

#[derive(Clone)]
struct LayerFeedbackState {
    editor_instance_id: u64,
    scope: Scope,
    snapshot_token: SnapshotToken,
    scope_generation: u64,
    operation_id: OperationId,
    target_layer_id: String,
    before: KeymapConfiguration,
    expected: KeymapConfiguration,
    feedback: KeymapLayerFeedback,
}

#[derive(Clone)]
pub(in crate::presentation) struct LayerSource {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) token: SnapshotToken,
    pub(in crate::presentation) revision: u64,
}

struct LayerEditCommit {
    editor_instance_id: u64,
    scope: Scope,
    snapshot: AcceptedSnapshot,
    scope_generation: u64,
    operation_id: OperationId,
    target_layer_id: String,
    before: KeymapConfiguration,
    expected: KeymapConfiguration,
    select_after: Option<String>,
    change: KeymapChange,
}

/// Hook-owned state passed directly into the private Keymap panel.
pub(in crate::presentation) struct LayerActions {
    pub(in crate::presentation) enabled: bool,
    pub(in crate::presentation) feedback: Option<KeymapLayerFeedback>,
    pub(in crate::presentation) on_operation: EventHandler<KeymapLayerOperation>,
}

/// Owns pending work at the Editor lifetime so hiding the Keymap workspace does
/// not abandon its single-flight guard or the Session terminal observer.
pub(in crate::presentation) fn use_layer_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    active_layer: Signal<String>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> LayerActions {
    let version = use_context::<Signal<u64>>()();
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let mut pending = use_signal(|| None::<PendingLayerEdit>);
    let mut feedback = use_signal(|| None::<LayerFeedbackState>);
    let mut effect_active_layer = active_layer;

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

            // Retire the operation slot only after its Session terminal result is
            // visible. A scope/workspace switch never cancels or replays it.
            pending.set(None);
            let live_scope = runtime.scope();
            if waiting.editor_instance_id != editor_instance_id
                || live_scope.as_ref() != Some(&waiting.scope)
                || scope_generation() != waiting.scope_generation
            {
                return;
            }

            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let identity = LayerFeedbackState {
                editor_instance_id,
                scope: waiting.scope.clone(),
                snapshot_token: waiting.snapshot_token,
                scope_generation: waiting.scope_generation,
                operation_id: waiting.operation_id,
                target_layer_id: waiting.target_layer_id.clone(),
                before: waiting.before.clone(),
                expected: waiting.expected.clone(),
                feedback: KeymapLayerFeedback::Pending,
            };
            match outcome {
                TerminalOutcome::Completed => {
                    let saved_current = model.lifecycle == Lifecycle::Ready
                        && snapshot.document.revision > waiting.revision
                        && model.durability
                            == (Durability::Saved {
                                revision: snapshot.document.revision,
                            });
                    let accepted_map = snapshot.document.keymap.clone().unwrap_or_default();
                    if !saved_current || snapshot.token == waiting.snapshot_token {
                        feedback.set(Some(failed_layer_edit_feedback(
                            identity,
                            &waiting,
                            &mut effect_active_layer,
                            "The accepted Keymap edit is not saved yet. Check the current layer values before retrying.".into(),
                        )));
                    } else if accepted_map == waiting.expected {
                        if let Some(select_after) = waiting.select_after.as_ref()
                            && active_layer() == waiting.target_layer_id
                        {
                            effect_active_layer.set(select_after.clone());
                        }
                        let mut identity = identity;
                        if let Some(select_after) = waiting.select_after {
                            identity.target_layer_id = select_after;
                        }
                        feedback.set(Some(LayerFeedbackState {
                            feedback: KeymapLayerFeedback::Saved,
                            ..identity
                        }));
                    } else {
                        feedback.set(Some(failed_layer_edit_feedback(
                            identity,
                            &waiting,
                            &mut effect_active_layer,
                            "The accepted Keymap differs from this layer edit. Review the current values and retry.".into(),
                        )));
                    }
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => {
                    feedback.set(Some(failed_layer_edit_feedback(
                        identity,
                        &waiting,
                        &mut effect_active_layer,
                        message,
                    )));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    feedback.set(Some(failed_layer_edit_feedback(
                        identity,
                        &waiting,
                        &mut effect_active_layer,
                        "The Keymap edit did not complete in the active session.".into(),
                    )));
                }
            }
        }
    }));

    let current_source = current_saved_source(
        &runtime,
        source.as_ref(),
        scope_generation(),
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    );
    let no_pending = pending.read().is_none();
    let enabled = no_pending && current_source.is_some();
    let current_feedback_snapshot = current_feedback_snapshot(&runtime, source.as_ref());
    let visible_feedback = feedback.read().as_ref().and_then(|state| {
        let current = current_feedback_snapshot.as_ref()?;
        (state.editor_instance_id == editor_instance_id
            && source.as_ref().map(|source| &source.scope) == Some(&state.scope)
            && scope_generation() == state.scope_generation
            && workspace() == "Keymap"
            && active_layer() == state.target_layer_id)
            .then(|| state.feedback.clone())
    });
    // Saved feedback is useful after the commit advanced the token, while a failed
    // request only describes the unchanged base snapshot. Filter either by the
    // current projected Keymap and target layer below.
    let visible_feedback = visible_feedback.filter(|shown| {
        let Some(state) = feedback.read().as_ref() else {
            return false;
        };
        let Some(current) = current_feedback_snapshot.as_ref() else {
            return false;
        };
        let accepted_map = current.document.keymap.clone().unwrap_or_default();
        match shown {
            KeymapLayerFeedback::Saved => {
                current.token != state.snapshot_token && accepted_map == state.expected
            }
            KeymapLayerFeedback::Failed(_) => {
                accepted_map == state.before && current.token == state.snapshot_token
            }
            KeymapLayerFeedback::Pending => pending.read().as_ref().is_some_and(|waiting| {
                waiting.editor_instance_id == state.editor_instance_id
                    && waiting.scope == state.scope
                    && waiting.operation_id == state.operation_id
                    && current.token == state.snapshot_token
            }),
        }
    });

    let on_operation = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let mut active_layer = active_layer;
        let source = source.clone();
        move |request: KeymapLayerOperation| {
            if pending.read().is_some() {
                return;
            }
            let requested_generation = scope_generation();
            let Some(snapshot) = current_saved_source(
                &runtime,
                source.as_ref(),
                requested_generation,
                scope_generation,
                workspace(),
                admission_current.as_ref(),
            ) else {
                return;
            };
            let Some(scope) = source.as_ref().map(|source| source.scope.clone()) else {
                return;
            };
            let mut expected = snapshot.document.keymap.clone().unwrap_or_default();
            let before = expected.clone();
            let (change, target_layer_id, select_after) = match request {
                KeymapLayerOperation::Add => {
                    if expected.layers.len() >= 32 {
                        return;
                    }
                    let operation_id = runtime.operation();
                    let mut id = format!("keymap-layer-{editor_instance_id}-{}", operation_id.0);
                    let existing_ids = expected
                        .layers
                        .iter()
                        .map(|layer| layer.id.clone())
                        .collect::<std::collections::BTreeSet<_>>();
                    let mut suffix = 0u32;
                    while existing_ids.contains(id.as_str()) {
                        suffix = suffix.saturating_add(1);
                        id = format!(
                            "keymap-layer-{editor_instance_id}-{}-{suffix}",
                            operation_id.0
                        );
                    }
                    let name = format!("Layer {}", expected.layers.len());
                    expected.layers.push(KeymapLayer {
                        id: id.clone(),
                        name: name.clone(),
                        bindings: Default::default(),
                        sensors: Default::default(),
                    });
                    active_layer.set(id.clone());
                    submit_layer_edit(
                        &runtime,
                        &mut pending,
                        &mut feedback,
                        LayerEditCommit {
                            editor_instance_id,
                            scope,
                            snapshot,
                            scope_generation: requested_generation,
                            operation_id,
                            target_layer_id: id.clone(),
                            before,
                            expected,
                            select_after: Some(id.clone()),
                            change: KeymapChange::AddLayer {
                                id: id.clone(),
                                name,
                            },
                        },
                    );
                    return;
                }
                KeymapLayerOperation::Rename { layer_id, name } => {
                    let Some(layer) = expected
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == layer_id)
                    else {
                        return;
                    };
                    layer.name = name.clone();
                    (
                        KeymapChange::RenameLayer {
                            id: layer_id.clone(),
                            name,
                        },
                        layer_id,
                        None,
                    )
                }
                KeymapLayerOperation::Remove { layer_id } => {
                    let Some(index) = expected
                        .layers
                        .iter()
                        .position(|layer| layer.id == layer_id)
                    else {
                        return;
                    };
                    if index == 0 || expected.layers.len() <= 1 {
                        return;
                    }
                    expected.layers.remove(index);
                    let fallback = expected.layers[0].id.clone();
                    (
                        KeymapChange::RemoveLayer {
                            id: layer_id.clone(),
                        },
                        layer_id.clone(),
                        (active_layer() == layer_id).then_some(fallback),
                    )
                }
            };
            let operation_id = runtime.operation();
            submit_layer_edit(
                &runtime,
                &mut pending,
                &mut feedback,
                LayerEditCommit {
                    editor_instance_id,
                    scope,
                    snapshot,
                    scope_generation: requested_generation,
                    operation_id,
                    target_layer_id,
                    before,
                    expected,
                    select_after,
                    change,
                },
            );
        }
    });

    LayerActions {
        enabled,
        feedback: visible_feedback,
        on_operation,
    }
}

fn current_saved_source(
    runtime: &Runtime,
    expected_source: Option<&LayerSource>,
    expected_generation: u64,
    generation: Signal<u64>,
    workspace: &'static str,
    admission_current: &dyn Fn() -> bool,
) -> Option<AcceptedSnapshot> {
    if workspace != "Keymap" || !admission_current() {
        return None;
    }
    let expected_source = expected_source?;
    let scope = &expected_source.scope;
    let token = expected_source.token;
    let revision = expected_source.revision;
    let model = runtime.model();
    let snapshot = model.accepted?;
    (runtime.scope().as_ref() == Some(scope)
        && scope.session_epoch == snapshot.session_epoch
        && scope.document_id == snapshot.document.id
        && scope.board_id == model.active_board_id
        && scope.instance_id == model.active_instance_id
        && snapshot.token == token
        && snapshot.document.revision == revision
        && snapshot
            .document
            .keymap
            .as_ref()
            .is_none_or(|keymap| !keymap.layers.is_empty())
        && model.lifecycle == Lifecycle::Ready
        && model.durability == (Durability::Saved { revision })
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && generation() == expected_generation)
        .then_some(snapshot)
}

fn current_feedback_snapshot(
    runtime: &Runtime,
    expected_source: Option<&LayerSource>,
) -> Option<AcceptedSnapshot> {
    let expected_source = expected_source?;
    let model = runtime.model();
    let snapshot = model.accepted?;
    (runtime.scope().as_ref() == Some(&expected_source.scope)
        && expected_source.scope.session_epoch == snapshot.session_epoch
        && expected_source.scope.document_id == snapshot.document.id
        && expected_source.scope.board_id == model.active_board_id
        && expected_source.scope.instance_id == model.active_instance_id)
        .then_some(snapshot)
}

fn submit_layer_edit(
    runtime: &Rc<Runtime>,
    pending: &mut Signal<Option<PendingLayerEdit>>,
    feedback: &mut Signal<Option<LayerFeedbackState>>,
    commit: LayerEditCommit,
) {
    let LayerEditCommit {
        editor_instance_id,
        scope,
        snapshot,
        scope_generation,
        operation_id,
        target_layer_id,
        before,
        expected,
        select_after,
        change,
    } = commit;
    let outcome = runtime.observe_operation(operation_id);
    let waiting = PendingLayerEdit {
        editor_instance_id,
        scope: scope.clone(),
        snapshot_token: snapshot.token,
        revision: snapshot.document.revision,
        scope_generation,
        operation_id,
        target_layer_id: target_layer_id.clone(),
        outcome,
        before: before.clone(),
        expected: expected.clone(),
        select_after,
    };
    let transaction_id = format!("keymap-layer-{editor_instance_id}-{}", operation_id.0);
    pending.set(Some(waiting));
    feedback.set(Some(LayerFeedbackState {
        editor_instance_id,
        scope: scope.clone(),
        snapshot_token: snapshot.token,
        scope_generation,
        operation_id,
        target_layer_id,
        before,
        expected,
        feedback: KeymapLayerFeedback::Pending,
    }));
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id,
            phase: EditPhase::Commit,
            target_ids: vec![scope.board_id],
            operation: EditOperation::EditKeymap { change },
        },
    });
}

fn failed_feedback(mut identity: LayerFeedbackState, message: String) -> LayerFeedbackState {
    identity.feedback = KeymapLayerFeedback::Failed(message);
    identity
}

fn failed_layer_edit_feedback(
    mut identity: LayerFeedbackState,
    waiting: &PendingLayerEdit,
    active_layer: &mut Signal<String>,
    message: String,
) -> LayerFeedbackState {
    let is_failed_add = !waiting
        .before
        .layers
        .iter()
        .any(|layer| layer.id == waiting.target_layer_id)
        && waiting
            .expected
            .layers
            .iter()
            .any(|layer| layer.id == waiting.target_layer_id);
    if is_failed_add && active_layer() == waiting.target_layer_id {
        if let Some(fallback) = waiting.before.layers.first() {
            active_layer.set(fallback.id.clone());
            identity.target_layer_id = fallback.id.clone();
        }
    }
    failed_feedback(identity, message)
}
