//! Fresh-source admission and exact-operation acknowledgement for Keymap bindings.
use super::binding_editor::{
    BindingEditFeedback, BindingEditRequest, BindingEditStatus, BindingField, BindingLayerChoice,
    BindingMacroChoice, BindingTarget, EncoderInputIdentity,
};
use super::layer_controller::LayerSource;
use super::view::KeymapView;
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, EncoderDirection, KeyBinding, KeymapChange,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
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
    pub current_encoder_projection: Rc<dyn Fn() -> Option<EncoderInputProjection>>,
}

#[derive(Clone)]
struct BindingTicket {
    request: BindingEditRequest,
    scope_generation: u64,
    ticket: EditTicket,
}

#[derive(Clone)]
struct BindingFeedbackState {
    request: BindingEditRequest,
    scope_generation: u64,
    status: BindingEditStatus,
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
    let pending = use_signal(Vec::<BindingTicket>::new);
    let feedback = use_signal(Vec::<BindingFeedbackState>::new);
    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let mut tickets = pending.peek().clone();
            let before = tickets.len();
            tickets.retain(|waiting| {
                let live = runtime.scope().as_ref() == Some(&waiting.request.scope)
                    && scope_generation() == waiting.scope_generation;
                let status = match waiting.ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(BindingEditStatus::Saved),
                    Settlement::Failed { message } => Some(BindingEditStatus::Failed(message)),
                    Settlement::Retired => None,
                };
                if let Some(status) = status {
                    if let Some(entry) = feedback
                        .write()
                        .iter_mut()
                        .find(|entry| entry.request.request_id == waiting.request.request_id)
                    {
                        entry.status = status;
                    }
                } else {
                    feedback
                        .write()
                        .retain(|entry| entry.request.request_id != waiting.request.request_id);
                }
                false
            });
            if tickets.len() != before {
                pending.set(tickets);
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
    let enabled = current_edit_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    )
    .is_some();
    let visible_feedback = feedback
        .read()
        .iter()
        .rev()
        .find(|state| {
            state.request.editor_instance_id == editor_instance_id
                && source.as_ref().map(|source| &source.scope) == Some(&state.request.scope)
                && state.scope_generation == scope_generation()
                && state.request.active_layer_id == active_layer_id
                && match &state.request.target {
                    BindingTarget::Key { key_id } => {
                        selected_key_id.as_deref() == Some(key_id.as_str())
                    }
                    _ => current_encoder_projection().is_some_and(|current| {
                        state
                            .request
                            .input_identity
                            .as_ref()
                            .is_some_and(|identity| same_input_lineage(identity, &current.identity))
                    }),
                }
        })
        .map(feedback_for_state);

    let on_change = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let source = source.clone();
        let view = view.clone();
        let current_encoder_inputs = current_encoder_projection.clone();
        let choices = choices.clone();
        move |request: BindingEditRequest| {
            if request.request_id <= last_admitted_request_id() {
                return;
            }
            let Some(snapshot) = current_edit_source(
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
                },
            ) else {
                return;
            };
            if apply_requested_field(&current.binding, &request.binding, request.field).is_none() {
                return;
            }
            let ticket = EditTicket::begin(
                &runtime,
                "keymap-binding",
                Some("binding".into()),
                binding_resolver(request.clone()),
            );
            pending.write().push(BindingTicket {
                request: request.clone(),
                scope_generation: captured_generation,
                ticket,
            });
            feedback.write().retain(|entry| {
                !(entry.request.target == request.target && entry.request.field == request.field)
            });
            feedback.write().push(BindingFeedbackState {
                request: request.clone(),
                scope_generation: captured_generation,
                status: BindingEditStatus::Pending,
            });
            last_admitted_request_id.set(request.request_id);
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

fn current_edit_source(
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
        && matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
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
            if !encoder_request_matches_projection(request, source?, encoder_inputs?, snapshot) {
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
            if !encoder_request_matches_projection(request, source?, encoder_inputs?, snapshot) {
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
) -> bool {
    let Some(captured) = request.input_identity.as_ref() else {
        return false;
    };
    if captured.scope != request.scope
        || !input_identity_matches_source(&inputs.identity, source, snapshot)
    {
        return false;
    }
    captured == &inputs.identity
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

fn binding_resolver(request: BindingEditRequest) -> EditResolver {
    EditResolver::new("keymap-binding", move |accepted: &AcceptedSnapshot| {
        if accepted.session_epoch != request.scope.session_epoch
            || accepted.document.id != request.scope.document_id
        {
            return Resolution::Retire("The project is no longer open.".into());
        }
        let map = accepted.document.keymap.as_ref();
        let layer = map.and_then(|map| {
            map.layers
                .iter()
                .find(|layer| layer.id == request.active_layer_id)
        });
        if layer.is_none() && !(map.is_none() && request.active_layer_id == "base") {
            return Resolution::Retire("This layer no longer exists.".into());
        }
        let layer_index = map
            .and_then(|map| {
                map.layers
                    .iter()
                    .position(|layer| layer.id == request.active_layer_id)
            })
            .unwrap_or(0);
        let key_binding = |key: &str| {
            binding_for_key_id(accepted, &request.scope.board_id, key, layer, layer_index)
        };
        let current = match &request.target {
            BindingTarget::Key { key_id } => {
                let view = super::view::project(
                    accepted,
                    Some(&request.scope),
                    &request.scope.board_id,
                    &request.active_layer_id,
                );
                if !view.is_some_and(|view| view.keys.iter().any(|key| key.id.as_ref() == key_id)) {
                    return Resolution::Retire("This key no longer exists.".into());
                }
                key_binding(key_id)
            }
            BindingTarget::EncoderRotation {
                encoder_id,
                direction,
            } => {
                if !boardstudio_core::electrical_peripherals::describe(
                    &accepted.document,
                    &request.scope.board_id,
                )
                .iter()
                .any(|encoder| encoder.kind == "encoder" && encoder.part_id == *encoder_id)
                {
                    return Resolution::Retire("This encoder no longer exists.".into());
                }
                layer
                    .and_then(|layer| layer.sensors.get(encoder_id))
                    .map_or(KeyBinding::None, |binding| match direction {
                        EncoderDirection::Clockwise => binding.clockwise.clone(),
                        EncoderDirection::Counterclockwise => binding.counterclockwise.clone(),
                    })
            }
            BindingTarget::EncoderPush { encoder_id, key_id } => {
                if !boardstudio_core::electrical_peripherals::describe(
                    &accepted.document,
                    &request.scope.board_id,
                )
                .iter()
                .any(|encoder| {
                    encoder.part_id == *encoder_id && encoder.press_key_id.as_ref() == Some(key_id)
                }) {
                    return Resolution::Retire("This encoder push input no longer exists.".into());
                }
                key_binding(key_id)
            }
        };
        let Some(binding) = apply_requested_field(&current, &request.binding, request.field) else {
            return Resolution::Retire("This binding field is no longer available.".into());
        };
        if binding == current {
            return Resolution::Unchanged;
        }
        let change = match &request.target {
            BindingTarget::Key { key_id } | BindingTarget::EncoderPush { key_id, .. } => {
                KeymapChange::Binding {
                    layer_id: request.active_layer_id.clone(),
                    key_id: key_id.clone(),
                    binding,
                }
            }
            BindingTarget::EncoderRotation {
                encoder_id,
                direction,
            } => KeymapChange::Encoder {
                layer_id: request.active_layer_id.clone(),
                encoder_id: encoder_id.clone(),
                direction: direction.clone(),
                binding,
            },
        };
        Resolution::Submit(EditCommand {
            base_revision: 0,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![request.scope.board_id.clone()],
            operation: EditOperation::EditKeymap { change },
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SnapshotToken;
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
