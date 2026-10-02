//! Fresh-source admission and exact-operation acknowledgement for Keymap bindings.
use super::binding_editor::{
    BindingEditFeedback, BindingEditRequest, BindingEditStatus, BindingField, BindingLayerChoice,
    BindingMacroChoice,
};
use super::layer_controller::LayerSource;
use super::view::{self, KeymapView};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, KeyBinding, KeymapChange};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct BindingEditorProjection {
    pub(in crate::presentation) effective_layer_id: String,
    pub(in crate::presentation) key_label: String,
    pub(in crate::presentation) binding: KeyBinding,
    pub(in crate::presentation) layers: Rc<[BindingLayerChoice]>,
    pub(in crate::presentation) macros: Rc<[BindingMacroChoice]>,
}

#[derive(Clone)]
struct PendingBindingEdit {
    request: BindingEditRequest,
    scope_generation: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
    admission_field_value: AcceptedFieldValue,
}

#[derive(Clone)]
struct BindingFeedbackState {
    request: BindingEditRequest,
    scope_generation: u64,
    admission_field_value: AcceptedFieldValue,
    failure_snapshot_token: Option<SnapshotToken>,
    status: BindingEditStatus,
}

/// Editor-lifetime state and the narrow current projection passed to the panel.
pub(in crate::presentation) struct BindingActions {
    pub(in crate::presentation) editor_instance_id: u64,
    /// Monotonic for the Editor lifetime; components may unmount and remount.
    pub(in crate::presentation) request_sequence: Signal<u64>,
    pub(in crate::presentation) enabled: bool,
    pub(in crate::presentation) projection: Option<Rc<BindingEditorProjection>>,
    pub(in crate::presentation) feedback: Option<BindingEditFeedback>,
    pub(in crate::presentation) on_change: EventHandler<BindingEditRequest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AcceptedFieldValue {
    Behavior(&'static str),
    Keycode(&'static str, String),
    Tap(String),
    HoldModifier(String),
    Layer(String),
    Macro(String),
}

fn field_value(binding: &KeyBinding, field: BindingField) -> Option<AcceptedFieldValue> {
    Some(match (field, binding) {
        (BindingField::Behavior, KeyBinding::KeyPress { .. }) => {
            AcceptedFieldValue::Behavior("key-press")
        }
        (BindingField::Behavior, KeyBinding::ModTap { .. }) => {
            AcceptedFieldValue::Behavior("mod-tap")
        }
        (BindingField::Behavior, KeyBinding::LayerTap { .. }) => {
            AcceptedFieldValue::Behavior("layer-tap")
        }
        (BindingField::Behavior, KeyBinding::MomentaryLayer { .. }) => {
            AcceptedFieldValue::Behavior("momentary-layer")
        }
        (BindingField::Behavior, KeyBinding::ToggleLayer { .. }) => {
            AcceptedFieldValue::Behavior("toggle-layer")
        }
        (BindingField::Behavior, KeyBinding::ToLayer { .. }) => {
            AcceptedFieldValue::Behavior("to-layer")
        }
        (BindingField::Behavior, KeyBinding::StickyLayer { .. }) => {
            AcceptedFieldValue::Behavior("sticky-layer")
        }
        (BindingField::Behavior, KeyBinding::StickyKey { .. }) => {
            AcceptedFieldValue::Behavior("sticky-key")
        }
        (BindingField::Behavior, KeyBinding::Macro { .. }) => AcceptedFieldValue::Behavior("macro"),
        (BindingField::Behavior, KeyBinding::Transparent) => {
            AcceptedFieldValue::Behavior("transparent")
        }
        (BindingField::Behavior, KeyBinding::None) => AcceptedFieldValue::Behavior("none"),
        (BindingField::Keycode, KeyBinding::KeyPress { keycode }) => {
            AcceptedFieldValue::Keycode("key-press", keycode.clone())
        }
        (BindingField::Keycode, KeyBinding::StickyKey { keycode }) => {
            AcceptedFieldValue::Keycode("sticky-key", keycode.clone())
        }
        (BindingField::Tap, KeyBinding::ModTap { tap, .. })
        | (BindingField::Tap, KeyBinding::LayerTap { tap, .. }) => {
            AcceptedFieldValue::Tap(tap.clone())
        }
        (BindingField::HoldModifier, KeyBinding::ModTap { hold, .. }) => {
            AcceptedFieldValue::HoldModifier(hold.clone())
        }
        (BindingField::Layer, KeyBinding::LayerTap { layer_id, .. })
        | (BindingField::Layer, KeyBinding::MomentaryLayer { layer_id })
        | (BindingField::Layer, KeyBinding::ToggleLayer { layer_id })
        | (BindingField::Layer, KeyBinding::ToLayer { layer_id })
        | (BindingField::Layer, KeyBinding::StickyLayer { layer_id }) => {
            AcceptedFieldValue::Layer(layer_id.clone())
        }
        (BindingField::Macro, KeyBinding::Macro { macro_id }) => {
            AcceptedFieldValue::Macro(macro_id.clone())
        }
        _ => return None,
    })
}

/// Apply only the requested semantic field to the freshly admitted binding.
fn apply_requested_field(
    current: &KeyBinding,
    requested: &KeyBinding,
    field: BindingField,
) -> Option<KeyBinding> {
    Some(match (field, current, requested) {
        (BindingField::Behavior, _, next) if field_value(next, field).is_some() => next.clone(),
        (BindingField::Keycode, KeyBinding::KeyPress { .. }, KeyBinding::KeyPress { keycode }) => {
            KeyBinding::KeyPress {
                keycode: keycode.clone(),
            }
        }
        (
            BindingField::Keycode,
            KeyBinding::StickyKey { .. },
            KeyBinding::StickyKey { keycode },
        ) => KeyBinding::StickyKey {
            keycode: keycode.clone(),
        },
        (BindingField::Tap, KeyBinding::ModTap { hold, .. }, KeyBinding::ModTap { tap, .. }) => {
            KeyBinding::ModTap {
                hold: hold.clone(),
                tap: tap.clone(),
            }
        }
        (
            BindingField::Tap,
            KeyBinding::LayerTap { layer_id, .. },
            KeyBinding::LayerTap { tap, .. },
        ) => KeyBinding::LayerTap {
            layer_id: layer_id.clone(),
            tap: tap.clone(),
        },
        (
            BindingField::HoldModifier,
            KeyBinding::ModTap { tap, .. },
            KeyBinding::ModTap { hold, .. },
        ) => KeyBinding::ModTap {
            hold: hold.clone(),
            tap: tap.clone(),
        },
        (
            BindingField::Layer,
            KeyBinding::LayerTap { tap, .. },
            KeyBinding::LayerTap { layer_id, .. },
        ) => KeyBinding::LayerTap {
            layer_id: layer_id.clone(),
            tap: tap.clone(),
        },
        (
            BindingField::Layer,
            KeyBinding::MomentaryLayer { .. },
            KeyBinding::MomentaryLayer { layer_id },
        ) => KeyBinding::MomentaryLayer {
            layer_id: layer_id.clone(),
        },
        (
            BindingField::Layer,
            KeyBinding::ToggleLayer { .. },
            KeyBinding::ToggleLayer { layer_id },
        ) => KeyBinding::ToggleLayer {
            layer_id: layer_id.clone(),
        },
        (BindingField::Layer, KeyBinding::ToLayer { .. }, KeyBinding::ToLayer { layer_id }) => {
            KeyBinding::ToLayer {
                layer_id: layer_id.clone(),
            }
        }
        (
            BindingField::Layer,
            KeyBinding::StickyLayer { .. },
            KeyBinding::StickyLayer { layer_id },
        ) => KeyBinding::StickyLayer {
            layer_id: layer_id.clone(),
        },
        (BindingField::Macro, KeyBinding::Macro { .. }, KeyBinding::Macro { macro_id }) => {
            KeyBinding::Macro {
                macro_id: macro_id.clone(),
            }
        }
        _ => return None,
    })
}

/// Projects one binding and only its reference-choice labels from an already
/// accepted Keymap view. This never clones a document, map, or unrelated key.
pub(in crate::presentation) fn project_binding_editor(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    board_id: &str,
    active_layer_id: &str,
    selected_key_id: &str,
    view: &KeymapView,
) -> Option<BindingEditorProjection> {
    if scope.session_epoch != snapshot.session_epoch
        || scope.document_id != snapshot.document.id
        || scope.board_id != board_id
    {
        return None;
    }
    let selected = view
        .keys
        .iter()
        .find(|key| key.id.as_ref() == selected_key_id)?;
    let saved_map = snapshot
        .document
        .keymap
        .as_ref()
        .filter(|map| !map.layers.is_empty());
    let saved_layers = saved_map.map_or(&[][..], |map| map.layers.as_slice());
    let layer_index = saved_layers
        .iter()
        .position(|layer| layer.id == active_layer_id)
        .unwrap_or(0);
    let effective_layer_id = saved_layers
        .get(layer_index)
        .map_or_else(|| "base".to_owned(), |layer| layer.id.clone());
    let binding = saved_layers
        .get(layer_index)
        .and_then(|layer| layer.bindings.get(selected_key_id))
        .cloned()
        .unwrap_or_else(|| {
            if layer_index > 0 {
                KeyBinding::Transparent
            } else {
                legacy_base_binding(snapshot, board_id, selected_key_id)
            }
        });
    let layers = if saved_layers.is_empty() {
        vec![BindingLayerChoice {
            id: "base".into(),
            name: "Base".into(),
        }]
    } else {
        saved_layers
            .iter()
            .map(|layer| BindingLayerChoice {
                id: layer.id.clone(),
                name: layer.name.clone(),
            })
            .collect()
    };
    let macros = saved_map.map_or_else(Vec::new, |map| {
        map.macros
            .iter()
            .map(|item| BindingMacroChoice {
                id: item.id.clone(),
                name: item.name.clone(),
            })
            .collect()
    });

    Some(BindingEditorProjection {
        effective_layer_id,
        key_label: selected.reference.to_string(),
        binding,
        layers: Rc::from(layers),
        macros: Rc::from(macros),
    })
}

fn legacy_base_binding(snapshot: &AcceptedSnapshot, board_id: &str, key_id: &str) -> KeyBinding {
    let legacy = snapshot
        .document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| board.board_id == board_id)
        })
        .and_then(|board| board.key_bindings.get(key_id));
    match legacy.map(String::as_str) {
        Some(value) if value.starts_with("&kp ") => KeyBinding::KeyPress {
            keycode: value[4..].to_owned(),
        },
        Some("&trans") => KeyBinding::Transparent,
        Some("&none") => KeyBinding::None,
        _ => KeyBinding::None,
    }
}

/// Owns binding operation state for the Editor lifetime, including while the
/// Keymap panel is hidden. Fresh event admission is repeated in the callback.
pub(in crate::presentation) fn use_binding_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    view: Option<Rc<KeymapView>>,
    active_layer: Signal<String>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> BindingActions {
    let version = use_context::<Signal<u64>>()();
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let request_sequence = use_signal(|| 0_u64);
    let mut last_admitted_request_id = use_signal(|| 0_u64);
    let captured_generation = scope_generation();
    let pending = use_signal(|| None::<PendingBindingEdit>);
    let feedback = use_signal(|| None::<BindingFeedbackState>);

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
            if waiting.request.editor_instance_id != editor_instance_id
                || live_scope.as_ref() != Some(&waiting.request.scope)
                || waiting.scope_generation != captured_generation
                || scope_generation() != waiting.scope_generation
            {
                pending.set(None);
                feedback.set(None);
                return;
            }

            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            let identity = feedback_for(&waiting, BindingEditStatus::Pending, None);
            match outcome {
                TerminalOutcome::Completed => {
                    let saved_current = model.lifecycle == Lifecycle::Ready
                        && model.durability
                            == (Durability::Saved {
                                revision: snapshot.document.revision,
                            });
                    if !saved_current
                        || snapshot.token == waiting.request.admission_token
                        || snapshot.document.revision <= waiting.request.admission_revision
                    {
                        return;
                    }
                    let acknowledged = current_binding_for_request(
                        snapshot,
                        &waiting.request,
                        &waiting.request.active_layer_id,
                    )
                    .is_some_and(|current| {
                        field_value(&current.binding, waiting.request.field)
                            == field_value(&waiting.request.binding, waiting.request.field)
                    });
                    pending.set(None);
                    if acknowledged {
                        feedback.set(Some(BindingFeedbackState {
                            status: BindingEditStatus::Saved,
                            failure_snapshot_token: None,
                            ..identity
                        }));
                    } else {
                        feedback.set(Some(BindingFeedbackState {
                            status: BindingEditStatus::Failed(
                                "The saved binding no longer matches this edit. Review the accepted value and retry.".into(),
                            ),
                            failure_snapshot_token: Some(snapshot.token),
                            ..identity
                        }));
                    }
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(BindingFeedbackState {
                        status: BindingEditStatus::Failed(message),
                        failure_snapshot_token: None,
                        ..identity
                    }));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    feedback.set(Some(BindingFeedbackState {
                        status: BindingEditStatus::Failed(
                            "The binding edit did not complete in the active session.".into(),
                        ),
                        failure_snapshot_token: None,
                        ..identity
                    }));
                }
            }
        }
    }));

    let model = runtime.model();
    let selected_key_id = model.selected_part_ids.first().cloned();
    let active_layer_id = active_layer();
    let display_snapshot = current_display_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
    );
    let source_key = source
        .as_ref()
        .map(|source| (source.scope.clone(), source.token, source.revision));
    let binding_projection = use_memo(use_reactive(
        (
            &version,
            &source_key,
            &view,
            &active_layer_id,
            &selected_key_id,
        ),
        {
            let runtime = runtime.clone();
            let source = source.clone();
            move |(_, _, view, layer_id, key_id)| {
                let snapshot = current_display_source(
                    &runtime,
                    source.as_ref(),
                    captured_generation,
                    scope_generation,
                )?;
                let source = source.as_ref()?;
                project_binding_editor(
                    &snapshot,
                    &source.scope,
                    &source.scope.board_id,
                    &layer_id,
                    key_id.as_deref()?,
                    view.as_deref()?,
                )
                .map(Rc::new)
            }
        },
    ));
    let projection = display_snapshot
        .as_ref()
        .and_then(|_| binding_projection.read().clone());
    let admission_snapshot = current_saved_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    );
    let enabled = pending.read().is_none() && admission_snapshot.is_some() && projection.is_some();
    let feedback_guard = feedback.read();
    let visible_feedback = feedback_guard.as_ref().and_then(|state| {
        let source = source.as_ref()?;
        let snapshot = display_snapshot.as_ref()?;
        let projection = projection.as_deref()?;
        let request = &state.request;
        if request.editor_instance_id != editor_instance_id
            || request.scope != source.scope
            || state.scope_generation != captured_generation
            || scope_generation() != state.scope_generation
            || request.active_layer_id != projection.effective_layer_id
            || selected_key_id.as_deref() != Some(request.key_id.as_str())
        {
            return None;
        }
        let pending_guard = pending.read();
        feedback_visible(state, snapshot, projection, pending_guard.as_ref()).then_some(state)
    });
    let visible_feedback = visible_feedback.map(feedback_for_state);

    let on_change = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let source = source.clone();
        move |mut request: BindingEditRequest| {
            if pending.read().is_some() {
                return;
            }
            if request.request_id <= last_admitted_request_id() {
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
            if request.editor_instance_id != editor_instance_id
                || request.scope != scope
                || request.admission_token != snapshot.token
                || request.admission_revision != snapshot.document.revision
                || request.scope.board_id != runtime.model().active_board_id
                || runtime.model().selected_part_ids.first() != Some(&request.key_id)
            {
                return;
            }
            let live_layer_id = active_layer();
            let Some(current) = current_binding_for_request(&snapshot, &request, &live_layer_id)
            else {
                return;
            };
            let Some(admission_field_value) = field_value(&current.binding, request.field) else {
                return;
            };
            let Some(next_binding) =
                apply_requested_field(&current.binding, &request.binding, request.field)
            else {
                return;
            };
            if current.binding == next_binding {
                return;
            }
            request.binding = next_binding;

            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let waiting = PendingBindingEdit {
                request: request.clone(),
                scope_generation: captured_generation,
                outcome,
                admission_field_value,
            };
            let identity = feedback_for(&waiting, BindingEditStatus::Pending, None);
            let transaction_id = format!(
                "keymap-binding-{}-{}-{}",
                request.editor_instance_id, request.request_id, operation_id.0
            );
            pending.set(Some(waiting));
            feedback.set(Some(identity));
            last_admitted_request_id.set(request.request_id);
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![scope.board_id],
                    operation: EditOperation::EditKeymap {
                        change: KeymapChange::Binding {
                            layer_id: request.active_layer_id,
                            key_id: request.key_id,
                            binding: request.binding,
                        },
                    },
                },
            });
        }
    });

    BindingActions {
        editor_instance_id,
        request_sequence,
        enabled,
        projection,
        feedback: visible_feedback,
        on_change,
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
    let model = runtime.model();
    let snapshot = model.accepted?;
    let revision = expected_source.revision;
    (runtime.scope().as_ref() == Some(&expected_source.scope)
        && expected_source.scope.session_epoch == snapshot.session_epoch
        && expected_source.scope.document_id == snapshot.document.id
        && expected_source.scope.board_id == model.active_board_id
        && expected_source.scope.instance_id == model.active_instance_id
        && snapshot.token == expected_source.token
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

/// Keep the accepted editor projection mounted while a valid operation is
/// Busy/Unsaved, but still require the exact current scope and snapshot.
fn current_display_source(
    runtime: &Runtime,
    expected_source: Option<&LayerSource>,
    expected_generation: u64,
    generation: Signal<u64>,
) -> Option<AcceptedSnapshot> {
    let expected_source = expected_source?;
    let model = runtime.model();
    let snapshot = model.accepted?;
    (runtime.scope().as_ref() == Some(&expected_source.scope)
        && expected_source.scope.session_epoch == snapshot.session_epoch
        && expected_source.scope.document_id == snapshot.document.id
        && expected_source.scope.board_id == model.active_board_id
        && expected_source.scope.instance_id == model.active_instance_id
        && snapshot.token == expected_source.token
        && snapshot.document.revision == expected_source.revision
        && generation() == expected_generation)
        .then_some(snapshot)
}

fn current_binding_for_request(
    snapshot: &AcceptedSnapshot,
    request: &BindingEditRequest,
    active_layer_id: &str,
) -> Option<BindingEditorProjection> {
    let view = view::project(
        snapshot,
        Some(&request.scope),
        &request.scope.board_id,
        active_layer_id,
    )?;
    let projection = project_binding_editor(
        snapshot,
        &request.scope,
        &request.scope.board_id,
        active_layer_id,
        &request.key_id,
        &view,
    )?;
    (projection.effective_layer_id == request.active_layer_id).then_some(projection)
}

fn feedback_for(
    waiting: &PendingBindingEdit,
    status: BindingEditStatus,
    failure_snapshot_token: Option<SnapshotToken>,
) -> BindingFeedbackState {
    BindingFeedbackState {
        request: waiting.request.clone(),
        scope_generation: waiting.scope_generation,
        admission_field_value: waiting.admission_field_value.clone(),
        failure_snapshot_token,
        status,
    }
}

fn feedback_for_state(state: &BindingFeedbackState) -> BindingEditFeedback {
    BindingEditFeedback {
        scope: state.request.scope.clone(),
        admission_token: state.request.admission_token,
        admission_revision: state.request.admission_revision,
        active_layer_id: state.request.active_layer_id.clone(),
        key_id: state.request.key_id.clone(),
        field: state.request.field,
        editor_instance_id: state.request.editor_instance_id,
        request_id: state.request.request_id,
        status: state.status.clone(),
    }
}

fn feedback_visible(
    state: &BindingFeedbackState,
    snapshot: &AcceptedSnapshot,
    projection: &BindingEditorProjection,
    pending: Option<&PendingBindingEdit>,
) -> bool {
    let Some(current_value) = field_value(&projection.binding, state.request.field) else {
        return false;
    };
    let requested_value = field_value(&state.request.binding, state.request.field);
    match &state.status {
        BindingEditStatus::Pending => pending.is_some_and(|waiting| {
            waiting.request == state.request && waiting.scope_generation == state.scope_generation
        }),
        BindingEditStatus::Saved => {
            snapshot.token != state.request.admission_token
                && requested_value.as_ref() == Some(&current_value)
        }
        BindingEditStatus::Failed(_) => match state.failure_snapshot_token {
            Some(acknowledgement_token) => {
                snapshot.token == acknowledgement_token
                    && requested_value.as_ref() != Some(&current_value)
            }
            None => current_value == state.admission_field_value,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_keys_cover_all_typed_variants_without_collapsing_empty_kinds() {
        let layer_id = "nav-stable-id".to_owned();
        let values = [
            KeyBinding::KeyPress {
                keycode: "A".into(),
            },
            KeyBinding::ModTap {
                hold: "LSHIFT".into(),
                tap: "A".into(),
            },
            KeyBinding::LayerTap {
                layer_id: layer_id.clone(),
                tap: "SPACE".into(),
            },
            KeyBinding::MomentaryLayer {
                layer_id: layer_id.clone(),
            },
            KeyBinding::ToggleLayer {
                layer_id: layer_id.clone(),
            },
            KeyBinding::ToLayer {
                layer_id: layer_id.clone(),
            },
            KeyBinding::StickyLayer { layer_id },
            KeyBinding::StickyKey {
                keycode: "LC(A)".into(),
            },
            KeyBinding::Macro {
                macro_id: "macro-stable-id".into(),
            },
            KeyBinding::Transparent,
            KeyBinding::None,
        ];

        assert!(
            values
                .iter()
                .all(|binding| field_value(binding, BindingField::Behavior).is_some())
        );
        assert!(field_value(&values[0], BindingField::Keycode).is_some());
        assert!(field_value(&values[7], BindingField::Keycode).is_some());
        assert!(field_value(&values[1], BindingField::Tap).is_some());
        assert!(field_value(&values[2], BindingField::Tap).is_some());
        assert!(field_value(&values[1], BindingField::HoldModifier).is_some());
        for binding in &values[2..7] {
            assert!(field_value(binding, BindingField::Layer).is_some());
        }
        assert!(field_value(&values[8], BindingField::Macro).is_some());
        assert_ne!(
            field_value(&KeyBinding::Transparent, BindingField::Behavior),
            field_value(&KeyBinding::None, BindingField::Behavior)
        );
    }

    #[test]
    fn field_updates_preserve_sibling_values_from_fresh_binding() {
        let current = KeyBinding::ModTap {
            hold: "LCTRL".into(),
            tap: "A".into(),
        };
        let stale_payload = KeyBinding::ModTap {
            hold: "LSHIFT".into(),
            tap: "B".into(),
        };
        assert_eq!(
            apply_requested_field(&current, &stale_payload, BindingField::Tap),
            Some(KeyBinding::ModTap {
                hold: "LCTRL".into(),
                tap: "B".into(),
            })
        );
        assert_eq!(
            apply_requested_field(&current, &stale_payload, BindingField::HoldModifier),
            Some(KeyBinding::ModTap {
                hold: "LSHIFT".into(),
                tap: "A".into(),
            })
        );
        assert_eq!(
            apply_requested_field(&current, &KeyBinding::None, BindingField::Tap),
            None
        );
    }
}
