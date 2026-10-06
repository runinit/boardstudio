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

    fn target_exists(&self, map: Option<&boardstudio_core::model::KeymapConfiguration>) -> bool {
        map.is_some_and(|map| map.layers.iter().any(|layer| layer.id == self.layer_id()))
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

    fn is_applied_to(&self, map: Option<&boardstudio_core::model::KeymapConfiguration>) -> bool {
        let Some(map) = map else {
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

    fn failure_is_still_relevant(
        &self,
        map: Option<&boardstudio_core::model::KeymapConfiguration>,
        admitted_name: Option<&str>,
    ) -> bool {
        let target =
            map.and_then(|map| map.layers.iter().find(|layer| layer.id == self.layer_id()));
        match self {
            Self::Add { .. } => target.is_none(),
            Self::Rename { layer_id, name } => {
                target.is_some_and(|layer| Some(layer.name.as_str()) == admitted_name)
                    || (map.is_none()
                        && layer_id == "base"
                        && admitted_name == Some("Base")
                        && name != "Base")
            }
            Self::Remove { .. } => target.is_some(),
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
    admitted_name: Option<String>,
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
    admitted_name: Option<String>,
    failure_snapshot_token: Option<SnapshotToken>,
    feedback: KeymapLayerFeedback,
}

#[derive(Clone)]
pub struct LayerSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
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
pub struct LayerActions {
    pub enabled: bool,
    pub feedback: Option<KeymapLayerFeedback>,
    pub on_operation: EventHandler<KeymapLayerOperation>,
}

/// Owns pending work at the Editor lifetime so hiding the Keymap workspace does
/// not abandon its single-flight guard or the Session terminal observer.
pub fn use_layer_operations(
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
    let pending = use_signal(|| None::<PendingLayerEdit>);
    let feedback = use_signal(|| None::<LayerFeedbackState>);

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
                    if waiting
                        .request
                        .is_applied_to(snapshot.document.keymap.as_ref())
                    {
                        feedback.set(Some(feedback_state(&waiting, KeymapLayerFeedback::Saved)));
                    } else {
                        feedback.set(Some(failed_feedback(
                            feedback_state(&waiting, KeymapLayerFeedback::Pending),
                            "The saved Keymap no longer contains this layer change. Review the current layer and retry.".into(),
                            Some(snapshot.token),
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
                        None,
                    )));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    feedback.set(Some(failed_feedback(
                        feedback_state(&waiting, KeymapLayerFeedback::Pending),
                        "The Keymap edit did not complete in the active session.".into(),
                        None,
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
    let feedback_guard = feedback.read();
    let visible_feedback = feedback_guard.as_ref().and_then(|state| {
        let snapshot = feedback_snapshot.as_ref()?;
        (state.editor_instance_id == editor_instance_id
            && source.as_ref().map(|source| &source.scope) == Some(&state.scope)
            && state.scope_generation == captured_generation
            && scope_generation() == state.scope_generation
            && workspace() == "Keymap"
            && feedback_targets_current_layer(
                &state.request,
                snapshot.document.keymap.as_ref(),
                &active_layer(),
            ))
        .then_some((state, snapshot))
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
            && state
                .request
                .is_applied_to(snapshot.document.keymap.as_ref()))
        .then_some(KeymapLayerFeedback::Saved),
        KeymapLayerFeedback::Failed(message) => failure_is_current(
            &state.request,
            state.admitted_name.as_deref(),
            snapshot.document.keymap.as_ref(),
            snapshot.token,
            state.failure_snapshot_token,
        )
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
            let active_display_layer = resolve_display_layer_id(existing_map, &active_layer());
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
                    if active_display_layer != Some(layer_id.as_str()) {
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
                    if active_display_layer != Some(layer_id.as_str()) {
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
        admitted_name: admitted_name_for_request(&request, snapshot.document.keymap.as_ref()),
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
        admitted_name: waiting.admitted_name.clone(),
        failure_snapshot_token: None,
        feedback,
    }
}

fn resolve_display_layer_id<'a>(
    map: Option<&'a boardstudio_core::model::KeymapConfiguration>,
    requested_layer_id: &str,
) -> Option<&'a str> {
    let Some(map) = map.filter(|map| !map.layers.is_empty()) else {
        return Some("base");
    };
    map.layers
        .iter()
        .find(|layer| layer.id == requested_layer_id)
        .or_else(|| map.layers.first())
        .map(|layer| layer.id.as_str())
}

fn feedback_targets_current_layer(
    request: &LayerEditIntent,
    map: Option<&boardstudio_core::model::KeymapConfiguration>,
    active_layer_id: &str,
) -> bool {
    if resolve_display_layer_id(map, active_layer_id) == Some(request.layer_id()) {
        return true;
    }
    active_layer_id == request.layer_id()
        && !request.target_exists(map)
        && matches!(
            request,
            LayerEditIntent::Add { .. } | LayerEditIntent::Remove { .. }
        )
}

fn failed_feedback(
    mut identity: LayerFeedbackState,
    message: String,
    failure_snapshot_token: Option<SnapshotToken>,
) -> LayerFeedbackState {
    identity.failure_snapshot_token = failure_snapshot_token;
    identity.feedback = KeymapLayerFeedback::Failed(message);
    identity
}

fn admitted_name_for_request(
    request: &LayerEditIntent,
    map: Option<&boardstudio_core::model::KeymapConfiguration>,
) -> Option<String> {
    let LayerEditIntent::Rename { layer_id, .. } = request else {
        return None;
    };
    map.and_then(|map| {
        map.layers
            .iter()
            .find(|layer| layer.id == *layer_id)
            .map(|layer| layer.name.clone())
    })
    .or_else(|| (map.is_none() && layer_id == "base").then(|| "Base".into()))
}

fn failure_is_current(
    request: &LayerEditIntent,
    admitted_name: Option<&str>,
    map: Option<&boardstudio_core::model::KeymapConfiguration>,
    current_token: SnapshotToken,
    failure_snapshot_token: Option<SnapshotToken>,
) -> bool {
    match failure_snapshot_token {
        Some(acknowledgement_token) => {
            acknowledgement_token == current_token && !request.is_applied_to(map)
        }
        None => request.failure_is_still_relevant(map, admitted_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    use boardstudio_core::model::{KeymapConfiguration, KeymapLayer};
    use std::collections::BTreeMap;

    fn keymap(ids: &[&str]) -> KeymapConfiguration {
        KeymapConfiguration {
            layers: ids
                .iter()
                .map(|id| KeymapLayer {
                    id: (*id).into(),
                    name: (*id).into(),
                    bindings: BTreeMap::new(),
                    sensors: BTreeMap::new(),
                })
                .collect(),
            macros: vec![],
        }
    }

    #[wasm_bindgen_test]
    fn retained_removed_id_resolves_then_restores_by_stable_identity() {
        let with_child = keymap(&["custom-primary", "layer-a"]);
        let after_remove = keymap(&["custom-primary"]);

        assert_eq!(
            resolve_display_layer_id(Some(&with_child), "layer-a"),
            Some("layer-a")
        );
        assert_eq!(
            resolve_display_layer_id(Some(&after_remove), "layer-a"),
            Some("custom-primary")
        );
        assert_eq!(
            resolve_display_layer_id(Some(&with_child), "layer-a"),
            Some("layer-a")
        );
    }

    #[wasm_bindgen_test]
    fn fallback_rename_and_terminal_feedback_follow_the_resolved_layer() {
        let map = keymap(&["custom-primary"]);
        let rename = LayerEditIntent::Rename {
            layer_id: "custom-primary".into(),
            name: "Primary renamed".into(),
        };
        let removed = LayerEditIntent::Remove {
            layer_id: "layer-a".into(),
        };
        let failed_add = LayerEditIntent::Add {
            layer_id: "layer-new".into(),
            name: "Layer 2".into(),
        };

        assert_eq!(
            resolve_display_layer_id(Some(&map), "layer-a"),
            Some("custom-primary")
        );
        assert!(feedback_targets_current_layer(
            &rename,
            Some(&map),
            "layer-a"
        ));
        assert!(feedback_targets_current_layer(
            &removed,
            Some(&map),
            "layer-a"
        ));
        assert!(feedback_targets_current_layer(
            &failed_add,
            Some(&map),
            "layer-new"
        ));
    }

    #[wasm_bindgen_test]
    fn rejected_rename_feedback_tracks_the_admitted_name_key() {
        let mut accepted = keymap(&["layer-a"]);
        accepted.layers[0].name = "Original".into();
        let request = LayerEditIntent::Rename {
            layer_id: "layer-a".into(),
            name: "bad\"name".into(),
        };
        let admitted_name = admitted_name_for_request(&request, Some(&accepted));

        assert!(failure_is_current(
            &request,
            admitted_name.as_deref(),
            Some(&accepted),
            SnapshotToken(2),
            None,
        ));

        let mut renamed = accepted.clone();
        renamed.layers[0].name = "Primary".into();
        assert!(!failure_is_current(
            &request,
            admitted_name.as_deref(),
            Some(&renamed),
            SnapshotToken(3),
            None,
        ));
    }

    #[wasm_bindgen_test]
    fn completed_add_mismatch_survives_a_later_rename_at_acknowledgement_token() {
        let request = LayerEditIntent::Add {
            layer_id: "layer-new".into(),
            name: "Layer 1".into(),
        };
        let mut saved = keymap(&["layer-new"]);
        saved.layers[0].name = "Primary".into();
        let acknowledgement_token = SnapshotToken(7);

        assert!(!request.is_applied_to(Some(&saved)));
        assert!(!request.failure_is_still_relevant(Some(&saved), None));
        assert!(failure_is_current(
            &request,
            None,
            Some(&saved),
            acknowledgement_token,
            Some(acknowledgement_token),
        ));
        assert!(!failure_is_current(
            &request,
            None,
            Some(&saved),
            SnapshotToken(8),
            Some(acknowledgement_token),
        ));
    }
}
