//! Fresh-source admission and exact-operation acknowledgement for Keymap bindings.
use super::binding_editor::{
    BindingEditFeedback, BindingEditRequest, BindingEditStatus, BindingField, BindingLayerChoice,
    BindingMacroChoice, BindingTarget, EncoderInputIdentity,
};
use super::layer_controller::LayerSource;
use super::view::KeymapView;
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, Scope, SnapshotToken,
    TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, EncoderDirection, KeyBinding, KeymapChange,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct BindingEditorProjection {
    pub effective_layer_id: String,
    pub key_label: String,
    pub binding: KeyBinding,
    pub layers: Rc<[BindingLayerChoice]>,
    pub macros: Rc<[BindingMacroChoice]>,
}

#[derive(Clone, Debug, PartialEq)]
struct BindingReferenceChoices {
    layers: Rc<[BindingLayerChoice]>,
    macros: Rc<[BindingMacroChoice]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderInputChoice {
    pub id: Rc<str>,
    pub label: Rc<str>,
    pub push_key_id: Option<Rc<str>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderInputProjection {
    pub identity: EncoderInputIdentity,
    pub encoders: Rc<[EncoderInputChoice]>,
    /// True once the F5 board plan for this accepted snapshot has settled.
    /// A completed binding edit waits for this edge before acknowledging a
    /// physical encoder whose input identity came from that plan.
    pub electrical_plan_settled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderBindingRow {
    pub id: Rc<str>,
    pub label: Rc<str>,
    pub clockwise: KeyBinding,
    pub counterclockwise: KeyBinding,
    pub push_key_id: Option<Rc<str>>,
    pub push_binding: Option<KeyBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderEditorProjection {
    pub input_identity: EncoderInputIdentity,
    pub effective_layer_id: String,
    pub rows: Rc<[EncoderBindingRow]>,
    pub layers: Rc<[BindingLayerChoice]>,
    pub macros: Rc<[BindingMacroChoice]>,
}

/// The accepted canvas and encoder projections that define one Keymap read
/// surface. Encoder display data is memoized; the live getter is reserved for
/// request admission and current feedback checks.
pub struct BindingProjectionSources {
    pub source: Option<LayerSource>,
    pub view: Option<Rc<KeymapView>>,
    pub encoder_projection: Memo<Option<EncoderInputProjection>>,
    pub current_encoder_projection:
        Rc<dyn Fn() -> Option<EncoderInputProjection>>,
}

#[derive(Clone)]
struct PendingBindingEdit {
    request: BindingEditRequest,
    scope_generation: u64,
    operation_id: OperationId,
    outcome: crate::operation_outcomes::OutcomeSlot,
    admission_field_value: AcceptedFieldValue,
}

#[derive(Clone)]
struct BindingFeedbackState {
    request: BindingEditRequest,
    scope_generation: u64,
    operation_id: OperationId,
    admission_field_value: AcceptedFieldValue,
    failure_snapshot_token: Option<SnapshotToken>,
    status: BindingEditStatus,
}

#[derive(Clone, Copy)]
enum EncoderIdentityCheck {
    ExactAdmission,
    StableLineage,
}

/// Inputs for reading one current binding. Grouping these keeps the accepted
/// source and the target's identity policy together at each read site.
struct BindingReadContext<'a> {
    snapshot: &'a AcceptedSnapshot,
    active_layer_id: &'a str,
    source: Option<&'a LayerSource>,
    view: Option<&'a KeymapView>,
    encoder_inputs: Option<&'a EncoderInputProjection>,
    choices: &'a BindingReferenceChoices,
    encoder_identity_check: EncoderIdentityCheck,
}

/// Editor-lifetime state and the narrow current projection passed to the panel.
pub struct BindingActions {
    pub editor_instance_id: u64,
    /// Monotonic for the Editor lifetime; components may unmount and remount.
    pub request_sequence: Signal<u64>,
    pub enabled: bool,
    pub projection: Option<Rc<BindingEditorProjection>>,
    pub encoder_projection: Option<Rc<EncoderEditorProjection>>,
    pub feedback: Option<BindingEditFeedback>,
    pub on_change: EventHandler<BindingEditRequest>,
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
fn project_binding_editor(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    board_id: &str,
    active_layer_id: &str,
    selected_key_id: &str,
    view: &KeymapView,
    choices: &BindingReferenceChoices,
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
    Some(BindingEditorProjection {
        effective_layer_id,
        key_label: selected.reference.to_string(),
        binding,
        layers: choices.layers.clone(),
        macros: choices.macros.clone(),
    })
}

fn project_binding_choices(snapshot: &AcceptedSnapshot) -> BindingReferenceChoices {
    let saved_map = snapshot
        .document
        .keymap
        .as_ref()
        .filter(|map| !map.layers.is_empty());
    let saved_layers = saved_map.map_or(&[][..], |map| map.layers.as_slice());
    let layers: Vec<_> = if saved_layers.is_empty() {
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
    let macros: Vec<_> = saved_map.map_or_else(Vec::new, |map| {
        map.macros
            .iter()
            .map(|item| BindingMacroChoice {
                id: item.id.clone(),
                name: item.name.clone(),
            })
            .collect()
    });
    BindingReferenceChoices {
        layers: Rc::from(layers),
        macros: Rc::from(macros),
    }
}

fn project_encoder_editor(
    snapshot: &AcceptedSnapshot,
    source: &LayerSource,
    input: &EncoderInputProjection,
    active_layer_id: &str,
    choices: &BindingReferenceChoices,
) -> Option<EncoderEditorProjection> {
    if !input_identity_matches_source(&input.identity, source, snapshot) {
        return None;
    }
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
    let layer = saved_layers.get(layer_index);
    let effective_layer_id = layer.map_or_else(|| "base".to_owned(), |layer| layer.id.clone());
    let rows: Vec<_> = input
        .encoders
        .iter()
        .map(|encoder| {
            let sensor = layer.and_then(|layer| layer.sensors.get(encoder.id.as_ref()));
            let clockwise = sensor.map_or(KeyBinding::None, |value| value.clockwise.clone());
            let counterclockwise =
                sensor.map_or(KeyBinding::None, |value| value.counterclockwise.clone());
            let push_binding = encoder.push_key_id.as_deref().map(|key_id| {
                binding_for_key_id(snapshot, &source.scope.board_id, key_id, layer, layer_index)
            });
            EncoderBindingRow {
                id: encoder.id.clone(),
                label: encoder.label.clone(),
                clockwise,
                counterclockwise,
                push_key_id: encoder.push_key_id.clone(),
                push_binding,
            }
        })
        .collect();
    Some(EncoderEditorProjection {
        input_identity: input.identity.clone(),
        effective_layer_id,
        rows: Rc::from(rows),
        layers: choices.layers.clone(),
        macros: choices.macros.clone(),
    })
}

fn input_identity_matches_source(
    identity: &EncoderInputIdentity,
    source: &LayerSource,
    snapshot: &AcceptedSnapshot,
) -> bool {
    identity.scope == source.scope
        && identity.token == source.token
        && identity.revision == source.revision
        && identity.scope.session_epoch == snapshot.session_epoch
        && identity.scope.document_id == snapshot.document.id
        && identity.scope.board_id == source.scope.board_id
}

fn same_input_lineage(captured: &EncoderInputIdentity, current: &EncoderInputIdentity) -> bool {
    captured.scope == current.scope
        && captured.projection_generation == current.projection_generation
        && captured.electrical_fingerprint == current.electrical_fingerprint
}

fn waiting_for_electrical_plan(
    captured: Option<&EncoderInputIdentity>,
    current: Option<&EncoderInputProjection>,
) -> bool {
    captured.is_some_and(|identity| identity.electrical_fingerprint.is_some())
        && current.is_some_and(|inputs| !inputs.electrical_plan_settled)
}

fn binding_for_key_id(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    key_id: &str,
    layer: Option<&boardstudio_core::model::KeymapLayer>,
    layer_index: usize,
) -> KeyBinding {
    layer
        .and_then(|layer| layer.bindings.get(key_id))
        .cloned()
        .unwrap_or_else(|| {
            if layer_index > 0 {
                KeyBinding::Transparent
            } else {
                legacy_base_binding(snapshot, board_id, key_id)
            }
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
pub fn use_binding_operations(
    runtime: Rc<Runtime>,
    projections: BindingProjectionSources,
    active_layer: Signal<String>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> BindingActions {
    let BindingProjectionSources {
        source,
        view,
        encoder_projection,
        current_encoder_projection,
    } = projections;
    let version = use_context::<Signal<u64>>()();
    let encoder_inputs_value = encoder_projection();
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let request_sequence = use_signal(|| 0_u64);
    let mut last_admitted_request_id = use_signal(|| 0_u64);
    let captured_generation = scope_generation();
    let pending = use_signal(|| None::<PendingBindingEdit>);
    let feedback = use_signal(|| None::<BindingFeedbackState>);
    let outcome_source = source.clone();
    let outcome_view = view.clone();

    use_effect(use_reactive((&version, &encoder_inputs_value), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let source = outcome_source;
        let view = outcome_view;
        let current_encoder_inputs = current_encoder_projection.clone();
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
            let current_inputs = current_encoder_inputs();
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
                    let choices = project_binding_choices(snapshot);
                    // F5 invalidates its plan while the accepted edit advances
                    // the snapshot, then publishes the replacement plan
                    // asynchronously. Keep a successful physical encoder edit
                    // pending until that plan settles so its own snapshot
                    // transition is not mistaken for an input replacement.
                    if waiting_for_electrical_plan(
                        waiting.request.input_identity.as_ref(),
                        current_inputs.as_ref(),
                    ) {
                        return;
                    }
                    let current = current_binding_for_request(
                        &waiting.request,
                        BindingReadContext {
                            snapshot,
                            active_layer_id: &waiting.request.active_layer_id,
                            source: source.as_ref(),
                            view: view.as_deref(),
                            encoder_inputs: current_inputs.as_ref(),
                            choices: &choices,
                            encoder_identity_check: EncoderIdentityCheck::StableLineage,
                        },
                    );
                    pending.set(None);
                    let Some(current) = current else {
                        feedback.set(None);
                        return;
                    };
                    let acknowledged = field_value(&current.binding, waiting.request.field)
                        == field_value(&waiting.request.binding, waiting.request.field);
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
                    let choices = project_binding_choices(snapshot);
                    if current_binding_for_request(
                        &waiting.request,
                        BindingReadContext {
                            snapshot,
                            active_layer_id: &waiting.request.active_layer_id,
                            source: source.as_ref(),
                            view: view.as_deref(),
                            encoder_inputs: current_inputs.as_ref(),
                            choices: &choices,
                            encoder_identity_check: EncoderIdentityCheck::StableLineage,
                        },
                    )
                    .is_some()
                    {
                        feedback.set(Some(BindingFeedbackState {
                            status: BindingEditStatus::Failed(message),
                            failure_snapshot_token: None,
                            ..identity
                        }));
                    } else {
                        feedback.set(None);
                    }
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    let choices = project_binding_choices(snapshot);
                    if current_binding_for_request(
                        &waiting.request,
                        BindingReadContext {
                            snapshot,
                            active_layer_id: &waiting.request.active_layer_id,
                            source: source.as_ref(),
                            view: view.as_deref(),
                            encoder_inputs: current_inputs.as_ref(),
                            choices: &choices,
                            encoder_identity_check: EncoderIdentityCheck::StableLineage,
                        },
                    )
                    .is_some()
                    {
                        feedback.set(Some(BindingFeedbackState {
                            status: BindingEditStatus::Failed(
                                "The binding edit did not complete in the active session.".into(),
                            ),
                            failure_snapshot_token: None,
                            ..identity
                        }));
                    } else {
                        feedback.set(None);
                    }
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
    let choices_projection = use_memo(use_reactive((&source_key,), {
        let runtime = runtime.clone();
        let source = source.clone();
        move |_| {
            let snapshot = current_display_source(
                &runtime,
                source.as_ref(),
                captured_generation,
                scope_generation,
            )?;
            Some(Rc::new(project_binding_choices(&snapshot)))
        }
    }));
    let choices = choices_projection.read().clone();
    let binding_projection = use_memo(use_reactive(
        (
            &source_key,
            &view,
            &active_layer_id,
            &selected_key_id,
            &choices,
        ),
        {
            let runtime = runtime.clone();
            let source = source.clone();
            move |(_, view, layer_id, key_id, choices)| {
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
                    choices.as_deref()?,
                )
                .map(Rc::new)
            }
        },
    ));
    let projection = display_snapshot
        .as_ref()
        .and_then(|_| binding_projection.read().clone());
    let encoder_projection_memo = use_memo(use_reactive(
        (
            &source_key,
            &active_layer_id,
            &encoder_inputs_value,
            &choices,
        ),
        {
            let runtime = runtime.clone();
            let source = source.clone();
            move |(_, layer_id, inputs, choices)| {
                let snapshot = current_display_source(
                    &runtime,
                    source.as_ref(),
                    captured_generation,
                    scope_generation,
                )?;
                project_encoder_editor(
                    &snapshot,
                    source.as_ref()?,
                    inputs.as_ref()?,
                    &layer_id,
                    choices.as_deref()?,
                )
                .map(Rc::new)
            }
        },
    ));
    let encoder_projection = display_snapshot
        .as_ref()
        .and_then(|_| encoder_projection_memo.read().clone())
        .filter(|projection| {
            current_encoder_projection()
                .is_some_and(|current| current.identity == projection.input_identity)
        });
    // Focus follows the exact accepted display owner through Busy/Unsaved settlement.
    // Dispatch below separately requires Saved and rejects additional pending requests.
    let enabled = workspace() == "Keymap" && admission_current() && display_snapshot.is_some();
    let feedback_guard = feedback.read();
    let visible_feedback = feedback_guard.as_ref().and_then(|state| {
        let source = source.as_ref()?;
        let snapshot = display_snapshot.as_ref()?;
        let choices = choices.as_deref()?;
        let request = &state.request;
        if request.editor_instance_id != editor_instance_id
            || request.scope != source.scope
            || state.scope_generation != captured_generation
            || scope_generation() != state.scope_generation
        {
            return None;
        }
        let current_inputs = current_encoder_projection();
        let projection = current_binding_for_request(
            request,
            BindingReadContext {
                snapshot,
                active_layer_id: &active_layer_id,
                source: Some(source),
                view: view.as_deref(),
                encoder_inputs: current_inputs.as_ref(),
                choices,
                encoder_identity_check: EncoderIdentityCheck::StableLineage,
            },
        )?;
        if request.active_layer_id != projection.effective_layer_id
            || matches!(
                &request.target,
                BindingTarget::Key { key_id }
                    if selected_key_id.as_deref() != Some(key_id.as_str())
            )
        {
            return None;
        }
        let pending_guard = pending.read();
        feedback_visible(state, snapshot, &projection, pending_guard.as_ref()).then_some(state)
    });
    let visible_feedback = visible_feedback.map(feedback_for_state);

    let on_change = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let source = source.clone();
        let view = view.clone();
        let current_encoder_inputs = current_encoder_projection.clone();
        let choices = choices.clone();
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
            let Some(expected_source) = source.as_ref() else {
                return;
            };
            let Some(choices) = choices.as_deref() else {
                return;
            };
            let current_inputs = current_encoder_inputs();
            if request.editor_instance_id != editor_instance_id
                || request.scope != expected_source.scope
                || request.admission_token != snapshot.token
                || request.admission_revision != snapshot.document.revision
                || request.scope.board_id != runtime.model().active_board_id
                || matches!(
                    &request.target,
                    BindingTarget::Key { key_id }
                        if runtime.model().selected_part_ids.first() != Some(key_id)
                )
            {
                return;
            }
            let live_layer_id = active_layer();
            let Some(current) = current_binding_for_request(
                &request,
                BindingReadContext {
                    snapshot: &snapshot,
                    active_layer_id: &live_layer_id,
                    source: Some(expected_source),
                    view: view.as_deref(),
                    encoder_inputs: current_inputs.as_ref(),
                    choices,
                    encoder_identity_check: EncoderIdentityCheck::ExactAdmission,
                },
            ) else {
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
                operation_id,
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
            let change = match &request.target {
                BindingTarget::Key { key_id } | BindingTarget::EncoderPush { key_id, .. } => {
                    KeymapChange::Binding {
                        layer_id: request.active_layer_id.clone(),
                        key_id: key_id.clone(),
                        binding: request.binding,
                    }
                }
                BindingTarget::EncoderRotation {
                    encoder_id,
                    direction,
                } => KeymapChange::Encoder {
                    layer_id: request.active_layer_id,
                    encoder_id: encoder_id.clone(),
                    direction: direction.clone(),
                    binding: request.binding,
                },
            };
            runtime.submit(Event::Edit {
                operation_id,
                command: EditCommand {
                    base_revision: snapshot.document.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![expected_source.scope.board_id.clone()],
                    operation: EditOperation::EditKeymap { change },
                },
            });
        }
    });

    BindingActions {
        editor_instance_id,
        request_sequence,
        enabled,
        projection,
        encoder_projection,
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
    request: &BindingEditRequest,
    context: BindingReadContext<'_>,
) -> Option<BindingEditorProjection> {
    let BindingReadContext {
        snapshot,
        active_layer_id,
        source,
        view,
        encoder_inputs,
        choices,
        encoder_identity_check,
    } = context;
    if request.scope.session_epoch != snapshot.session_epoch
        || request.scope.document_id != snapshot.document.id
    {
        return None;
    }
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
    let layer = saved_layers.get(layer_index);
    let effective_layer_id = layer.map_or_else(|| "base".to_owned(), |layer| layer.id.clone());
    if effective_layer_id != request.active_layer_id {
        return None;
    }
    let (key_label, binding) = match &request.target {
        BindingTarget::Key { key_id } => {
            if request.input_identity.is_some() {
                return None;
            }
            let view = view?;
            let projection = project_binding_editor(
                snapshot,
                &request.scope,
                &request.scope.board_id,
                active_layer_id,
                key_id,
                view,
                choices,
            )?;
            (projection.key_label, projection.binding)
        }
        BindingTarget::EncoderRotation {
            encoder_id,
            direction,
        } => {
            if !encoder_request_matches_projection(
                request,
                source?,
                encoder_inputs?,
                snapshot,
                encoder_identity_check,
            ) {
                return None;
            }
            let encoder = encoder_inputs?
                .encoders
                .iter()
                .find(|encoder| encoder.id.as_ref() == encoder_id)?;
            let sensor = layer.and_then(|layer| layer.sensors.get(encoder_id));
            let binding = match (sensor, direction) {
                (Some(value), EncoderDirection::Clockwise) => value.clockwise.clone(),
                (Some(value), EncoderDirection::Counterclockwise) => value.counterclockwise.clone(),
                (None, _) => KeyBinding::None,
            };
            let direction_label = match direction {
                EncoderDirection::Clockwise => "clockwise",
                EncoderDirection::Counterclockwise => "counterclockwise",
            };
            (format!("{} {direction_label}", encoder.label), binding)
        }
        BindingTarget::EncoderPush { encoder_id, key_id } => {
            if !encoder_request_matches_projection(
                request,
                source?,
                encoder_inputs?,
                snapshot,
                encoder_identity_check,
            ) {
                return None;
            }
            let encoder = encoder_inputs?.encoders.iter().find(|encoder| {
                encoder.id.as_ref() == encoder_id
                    && encoder.push_key_id.as_deref() == Some(key_id.as_str())
            })?;
            (
                format!("{} push", encoder.label),
                binding_for_key_id(
                    snapshot,
                    &request.scope.board_id,
                    key_id,
                    layer,
                    layer_index,
                ),
            )
        }
    };
    Some(BindingEditorProjection {
        effective_layer_id,
        key_label,
        binding,
        layers: choices.layers.clone(),
        macros: choices.macros.clone(),
    })
}

fn encoder_request_matches_projection(
    request: &BindingEditRequest,
    source: &LayerSource,
    inputs: &EncoderInputProjection,
    snapshot: &AcceptedSnapshot,
    check: EncoderIdentityCheck,
) -> bool {
    let Some(captured) = request.input_identity.as_ref() else {
        return false;
    };
    if captured.scope != request.scope
        || !input_identity_matches_source(&inputs.identity, source, snapshot)
    {
        return false;
    }
    match check {
        EncoderIdentityCheck::ExactAdmission => captured == &inputs.identity,
        EncoderIdentityCheck::StableLineage => same_input_lineage(captured, &inputs.identity),
    }
}

fn feedback_for(
    waiting: &PendingBindingEdit,
    status: BindingEditStatus,
    failure_snapshot_token: Option<SnapshotToken>,
) -> BindingFeedbackState {
    BindingFeedbackState {
        request: waiting.request.clone(),
        scope_generation: waiting.scope_generation,
        operation_id: waiting.operation_id,
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
        target: state.request.target.clone(),
        input_identity: state.request.input_identity.clone(),
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
            waiting.request == state.request
                && waiting.scope_generation == state.scope_generation
                && waiting.operation_id == state.operation_id
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
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn encoder_lineage_survives_own_snapshot_advance_but_not_input_replacement() {
        let scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(4),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: Some("instance".into()),
        };
        let captured = EncoderInputIdentity {
            scope: scope.clone(),
            token: SnapshotToken(8),
            revision: 12,
            projection_generation: 21,
            electrical_fingerprint: Some("f5-layout".into()),
        };
        let after_own_edit = EncoderInputIdentity {
            token: SnapshotToken(9),
            revision: 13,
            ..captured.clone()
        };
        let replaced_inputs = EncoderInputIdentity {
            projection_generation: 22,
            ..after_own_edit.clone()
        };

        assert!(same_input_lineage(&captured, &after_own_edit));
        assert!(!same_input_lineage(&captured, &replaced_inputs));
    }

    #[wasm_bindgen_test]
    fn completed_physical_encoder_edit_waits_for_pending_plan_then_resumes() {
        let scope = Scope {
            session_epoch: boardstudio_application::SessionEpoch(4),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let captured = EncoderInputIdentity {
            scope: scope.clone(),
            token: SnapshotToken(8),
            revision: 12,
            projection_generation: 21,
            electrical_fingerprint: Some("plan".into()),
        };
        let mut current = EncoderInputProjection {
            identity: EncoderInputIdentity {
                token: SnapshotToken(9),
                revision: 13,
                ..captured.clone()
            },
            encoders: Rc::from([]),
            electrical_plan_settled: false,
        };
        assert!(waiting_for_electrical_plan(Some(&captured), Some(&current)));
        current.electrical_plan_settled = true;
        assert!(!waiting_for_electrical_plan(
            Some(&captured),
            Some(&current)
        ));
    }

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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
