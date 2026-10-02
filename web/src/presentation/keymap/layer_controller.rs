//! Editor-lifetime admission and acknowledgement for private Keymap layer edits.
use super::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, KeymapChange};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
enum LayerEditIntent {
    Add { layer_id: String, name: String },
    Rename { layer_id: String, name: String },
    Remove { layer_id: String },
}

impl LayerEditIntent {
    fn layer_id(&self) -> &str {
        match self {
            Self::Add { layer_id, .. }
            | Self::Rename { layer_id, .. }
            | Self::Remove { layer_id } => layer_id,
        }
    }

    fn change(&self) -> KeymapChange {
        match self {
            Self::Add { layer_id, name } => KeymapChange::AddLayer {
                id: layer_id.clone(),
                name: name.clone(),
            },
            Self::Rename { layer_id, name } => KeymapChange::RenameLayer {
                id: layer_id.clone(),
                name: name.clone(),
            },
            Self::Remove { layer_id } => KeymapChange::RemoveLayer {
                id: layer_id.clone(),
            },
        }
    }

    fn is_applied_to(&self, document: &boardstudio_core::model::ProjectDoc) -> bool {
        let Some(map) = document.keymap.as_ref() else {
            return false;
        };
        if map.layers.is_empty() {
            return false;
        }
        match self {
            Self::Add { layer_id, name } | Self::Rename { layer_id, name } => map
                .layers
                .iter()
                .any(|layer| layer.id == *layer_id && layer.name == *name),
            Self::Remove { layer_id } => map.layers.iter().all(|layer| layer.id != *layer_id),
        }
    }
}

#[derive(Clone)]
struct PendingLayerEdit {
    editor_instance_id: u64,
    scope: Scope,
    snapshot: AcceptedSnapshot,
    scope_generation: u64,
    operation_id: OperationId,
    request: LayerEditIntent,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

#[derive(Clone)]
struct LayerFeedbackState {
    editor_instance_id: u64,
    scope: Scope,
    snapshot_token: SnapshotToken,
    scope_generation: u64,
    operation_id: OperationId,
    request: LayerEditIntent,
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
    request: LayerEditIntent,
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
    let captured_generation = scope_generation();
    let mut pending = use_signal(|| None::<PendingLayerEdit>);
    let mut feedback = use_signal(|| None::<LayerFeedbackState>);

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

            let live_scope = runtime.scope();
            if waiting.editor_instance_id != editor_instance_id
                || live_scope.as_ref() != Some(&waiting.scope)
                || waiting.scope_generation != captured_generation
                || scope_generation() != waiting.scope_generation
            {
                pending.set(None);
                return;
            }

            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            match outcome {
                TerminalOutcome::Completed => {
                    // A later queued edit can make the model busy after this
                    // operation settles. Keep the slot until a saved snapshot is
                    // available, then verify the requested target against it.
                    let saved_current = model.lifecycle == Lifecycle::Ready
                        && model.durability
                            == (Durability::Saved {
                                revision: snapshot.document.revision,
                            });
                    if !saved_current
                        || snapshot.token == waiting.snapshot.token
                        || snapshot.document.revision <= waiting.snapshot.document.revision
                    {
                        return;
                    }

                    pending.set(None);
                    if waiting.request.is_applied_to(&snapshot.document) {
                        feedback.set(Some(feedback_state(&waiting, KeymapLayerFeedback::Saved)));
                    } else {
                        feedback.set(Some(failed_feedback(
                            feedback_state(&waiting, KeymapLayerFeedback::Pending),
                            "The saved Keymap no longer contains this layer change. Review the current layer and retry.".into(),
                        )));
                    }
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(failed_feedback(
                        feedback_state(&waiting, KeymapLayerFeedback::Pending),
                        message,
                    )));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    feedback.set(Some(failed_feedback(
                        feedback_state(&waiting, KeymapLayerFeedback::Pending),
                        "The Keymap edit did not complete in the active session.".into(),
                    )));
                }
            }
        }
    }));

    let current_source = current_saved_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    );
    let enabled = pending.read().is_none() && current_source.is_some();
    let feedback_snapshot = current_feedback_snapshot(&runtime, source.as_ref());
    let visible_feedback = feedback.read().as_ref().and_then(|state| {
        let snapshot = feedback_snapshot.as_ref()?;
        (state.editor_instance_id == editor_instance_id
            && source.as_ref().map(|source| &source.scope) == Some(&state.scope)
            && state.scope_generation == captured_generation
            && scope_generation() == state.scope_generation
            && workspace() == "Keymap"
            && active_layer() == state.request.layer_id())
        .then(|| (state, snapshot))
    });
    let visible_feedback = visible_feedback.and_then(|(state, snapshot)| match &state.feedback {
        KeymapLayerFeedback::Pending => pending
            .read()
            .as_ref()
            .is_some_and(|waiting| {
                waiting.editor_instance_id == state.editor_instance_id
                    && waiting.scope == state.scope
                    && waiting.operation_id == state.operation_id
                    && snapshot.token == state.snapshot_token
            })
            .then_some(KeymapLayerFeedback::Pending),
        KeymapLayerFeedback::Saved => (snapshot.token != state.snapshot_token
            && state.request.is_applied_to(&snapshot.document))
        .then_some(KeymapLayerFeedback::Saved),
        KeymapLayerFeedback::Failed(message) => (snapshot.token == state.snapshot_token)
            .then(|| KeymapLayerFeedback::Failed(message.clone())),
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
            let Some(snapshot) = current_saved_source(
                &runtime,
                source.as_ref(),
                captured_generation,
                scope_generation,
                workspace(),
                admission_current.as_ref(),
            ) else {
                return;
            };
            let Some(scope) = source.as_ref().map(|source| source.scope.clone()) else {
                return;
            };
            let existing_map = snapshot.document.keymap.as_ref();
            if existing_map.is_some_and(|map| map.layers.is_empty()) {
                return;
            }
            let layer_count = existing_map.map_or(1, |map| map.layers.len());
            match request {
                KeymapLayerOperation::Add => {
                    if layer_count >= 32 {
                        return;
                    }
                    // Entity and Session operation identities use separate
                    // counter allocations; neither ID doubles as the other.
                    let entity_nonce = runtime.operation().0;
                    let mut layer_id = format!("keymap-layer-{editor_instance_id}-{entity_nonce}");
                    if let Some(map) = existing_map {
                        let mut suffix = 0u32;
                        while map.layers.iter().any(|layer| layer.id == layer_id) {
                            suffix = suffix.saturating_add(1);
                            layer_id = format!(
                                "keymap-layer-{editor_instance_id}-{entity_nonce}-{suffix}"
                            );
                        }
                    }
                    let name = format!("Layer {layer_count}");
                    let operation_id = runtime.operation();
                    active_layer.set(layer_id.clone());
                    submit_layer_edit(
                        &runtime,
                        &mut pending,
                        &mut feedback,
                        LayerEditCommit {
                            editor_instance_id,
                            scope,
                            snapshot,
                            scope_generation: captured_generation,
                            operation_id,
                            request: LayerEditIntent::Add { layer_id, name },
                        },
                    );
                }
                KeymapLayerOperation::Rename { layer_id, name } => {
                    if active_layer() != layer_id {
                        return;
                    }
                    let exists = match existing_map {
                        Some(map) => map.layers.iter().any(|layer| layer.id == layer_id),
                        None => layer_id == "base",
                    };
                    if !exists {
                        return;
                    }
                    submit_layer_edit(
                        &runtime,
                        &mut pending,
                        &mut feedback,
                        LayerEditCommit {
                            editor_instance_id,
                            scope,
                            snapshot,
                            scope_generation: captured_generation,
                            operation_id: runtime.operation(),
                            request: LayerEditIntent::Rename { layer_id, name },
                        },
                    );
                }
                KeymapLayerOperation::Remove { layer_id } => {
                    if active_layer() != layer_id {
                        return;
                    }
                    let Some(map) = existing_map else { return };
                    let Some(index) = map.layers.iter().position(|layer| layer.id == layer_id)
                    else {
                        return;
                    };
                    if index == 0 || map.layers.len() <= 1 {
                        return;
                    }
                    submit_layer_edit(
                        &runtime,
                        &mut pending,
                        &mut feedback,
                        LayerEditCommit {
                            editor_instance_id,
                            scope,
                            snapshot,
                            scope_generation: captured_generation,
                            operation_id: runtime.operation(),
                            request: LayerEditIntent::Remove { layer_id },
                        },
                    );
                }
            }
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
        request,
    } = commit;
    let outcome = runtime.observe_operation(operation_id);
    let waiting = PendingLayerEdit {
        editor_instance_id,
        scope: scope.clone(),
        snapshot: snapshot.clone(),
        scope_generation,
        operation_id,
        request: request.clone(),
        outcome,
    };
    let change = request.change();
    let transaction_id = format!("keymap-layer-{editor_instance_id}-{}", operation_id.0);
    pending.set(Some(waiting.clone()));
    feedback.set(Some(feedback_state(&waiting, KeymapLayerFeedback::Pending)));
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

fn feedback_state(waiting: &PendingLayerEdit, feedback: KeymapLayerFeedback) -> LayerFeedbackState {
    LayerFeedbackState {
        editor_instance_id: waiting.editor_instance_id,
        scope: waiting.scope.clone(),
        snapshot_token: waiting.snapshot.token,
        scope_generation: waiting.scope_generation,
        operation_id: waiting.operation_id,
        request: waiting.request.clone(),
        feedback,
    }
}

fn failed_feedback(mut identity: LayerFeedbackState, message: String) -> LayerFeedbackState {
    identity.feedback = KeymapLayerFeedback::Failed(message);
    identity
}
