//! Editor-lifetime admission and exact-operation acknowledgement for macro edits.
use super::layer_controller::LayerSource;
use super::macro_editor::{
    MacroEditChange, MacroEditFeedback, MacroEditRequest, MacroEditStatus, MacroEditTarget,
    MacroReadSource, MacroStepSequence,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, KeyBinding, KeymapChange, KeymapMacro, MacroChange,
    MacroStep,
};
use dioxus::prelude::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone)]
struct MacroIntent {
    macro_id: Option<String>,
    target: MacroEditTarget,
    change: MacroEditChange,
    original_field: OriginalField,
    expected_steps: Option<Rc<[MacroStep]>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OriginalField {
    Absent,
    Present,
    Name(String),
    TapMs(u32),
    WaitMs(u32),
    Steps(Rc<[MacroStep]>),
}

type PreparedMacroChange = (OriginalField, MacroChange, Option<Rc<[MacroStep]>>);

#[derive(Clone)]
struct PendingMacroEdit {
    request: MacroEditRequest,
    operation_id: OperationId,
    outcome: crate::operation_outcomes::OutcomeSlot,
    intent: MacroIntent,
}

#[derive(Clone)]
struct MacroFeedbackState {
    request: MacroEditRequest,
    operation_id: OperationId,
    intent: MacroIntent,
    status: MacroEditStatus,
    failure_snapshot_token: Option<SnapshotToken>,
}

pub struct MacroActions {
    pub editor_instance_id: u64,
    pub request_sequence: Signal<u64>,
    pub source: Option<MacroReadSource>,
    pub sequences: Rc<[MacroStepSequence]>,
    pub enabled: bool,
    pub feedback: Option<MacroEditFeedback>,
    pub on_change: EventHandler<MacroEditRequest>,
}

struct SequenceCache {
    by_macro: RefCell<HashMap<String, Rc<[MacroStep]>>>,
}

/// Owns all macro submission state for the Editor lifetime, including while
/// another workspace hides the Keymap panel.
pub fn use_macro_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> MacroActions {
    let version = use_context::<Signal<u64>>()();
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let request_sequence = use_signal(|| 0_u64);
    let mut last_admitted_request = use_signal(|| 0_u64);
    let captured_generation = scope_generation();
    let pending = use_signal(|| None::<PendingMacroEdit>);
    let feedback = use_signal(|| None::<MacroFeedbackState>);
    let cache = use_hook(|| {
        Rc::new(SequenceCache {
            by_macro: RefCell::new(HashMap::new()),
        })
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
            if waiting.request.editor_instance_id != editor_instance_id
                || runtime.scope().as_ref() != Some(&waiting.request.scope)
                || waiting.request.scope_generation != captured_generation
                || scope_generation() != waiting.request.scope_generation
            {
                pending.set(None);
                feedback.set(None);
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
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
                    let acknowledged = intent_applied(&waiting.intent, snapshot);
                    pending.set(None);
                    feedback.set(Some(MacroFeedbackState {
                        request: waiting.request,
                        operation_id: waiting.operation_id,
                        intent: waiting.intent,
                        status: if acknowledged { MacroEditStatus::Saved } else {
                            MacroEditStatus::Failed("The saved macro no longer matches this edit. Review the accepted macro and retry.".into())
                        },
                        failure_snapshot_token: (!acknowledged).then_some(snapshot.token),
                    }));
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(MacroFeedbackState {
                        request: waiting.request,
                        operation_id: waiting.operation_id,
                        intent: waiting.intent,
                        status: MacroEditStatus::Failed(message),
                        failure_snapshot_token: None,
                    }));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    feedback.set(Some(MacroFeedbackState {
                        request: waiting.request,
                        operation_id: waiting.operation_id,
                        intent: waiting.intent,
                        status: MacroEditStatus::Failed(
                            "The macro edit did not complete in the active session.".into(),
                        ),
                        failure_snapshot_token: None,
                    }));
                }
            }
        }
    }));

    let display_snapshot = current_display_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
    );
    let source_key = source.as_ref().map(|source| {
        (
            source.scope.clone(),
            source.token,
            source.revision,
            captured_generation,
        )
    });
    let sequence_cache = cache.clone();
    let projected = use_memo(use_reactive((&source_key,), {
        let runtime = runtime.clone();
        let source = source.clone();
        move |_| {
            let snapshot = current_display_source(
                &runtime,
                source.as_ref(),
                captured_generation,
                scope_generation,
            )?;
            let macros = snapshot
                .document
                .keymap
                .as_ref()
                .map_or(&[][..], |map| map.macros.as_slice());
            let mut next = HashMap::with_capacity(macros.len());
            let mut sequences = Vec::with_capacity(macros.len());
            {
                let previous = sequence_cache.by_macro.borrow();
                for item in macros {
                    let stamp = previous
                        .get(&item.id)
                        .filter(|steps| steps.as_ref() == item.steps.as_slice())
                        .cloned()
                        .unwrap_or_else(|| Rc::from(item.steps.clone()));
                    next.insert(item.id.clone(), stamp.clone());
                    sequences.push(MacroStepSequence {
                        macro_id: Rc::from(item.id.as_str()),
                        steps: stamp,
                    });
                }
            }
            *sequence_cache.by_macro.borrow_mut() = next;
            Some((
                MacroReadSource::new(
                    snapshot.document.clone(),
                    snapshot.token,
                    snapshot.document.revision,
                ),
                Rc::from(sequences),
            ))
        }
    }));
    let projected = display_snapshot
        .as_ref()
        .and_then(|_| projected.read().clone());
    let admission_snapshot = current_saved_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    );
    let enabled = pending.read().is_none() && admission_snapshot.is_some() && projected.is_some();

    let visible_feedback = feedback.read().as_ref().and_then(|state| {
        let display = display_snapshot.as_ref()?;
        if state.request.editor_instance_id != editor_instance_id
            || source.as_ref().map(|item| &item.scope) != Some(&state.request.scope)
            || state.request.scope_generation != captured_generation
            || scope_generation() != state.request.scope_generation
        {
            return None;
        }
        let pending_match = pending.read().as_ref().is_some_and(|waiting| {
            waiting.operation_id == state.operation_id
                && waiting.request.request_id == state.request.request_id
        });
        match &state.status {
            MacroEditStatus::Pending => {
                pending_match.then(|| public_feedback(state, MacroEditStatus::Pending))
            }
            MacroEditStatus::Saved => intent_applied(&state.intent, display)
                .then(|| public_feedback(state, MacroEditStatus::Saved)),
            MacroEditStatus::Failed(message) => {
                let relevant = if state.failure_snapshot_token.is_some() {
                    state.failure_snapshot_token == Some(display.token)
                        && !intent_applied(&state.intent, display)
                } else {
                    failure_still_relevant(&state.intent, display, &cache)
                };
                relevant.then(|| public_feedback(state, MacroEditStatus::Failed(message.clone())))
            }
        }
    });

    let on_change = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let source = source.clone();
        let sequence_cache = cache.clone();
        move |request: MacroEditRequest| {
            if pending.read().is_some() || request.request_id <= last_admitted_request() {
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
                || request.scope_generation != captured_generation
                || request.admission_token != snapshot.token
                || request.admission_revision != snapshot.document.revision
                || request.request_id > request_sequence()
            {
                return;
            }
            let map = snapshot.document.keymap.as_ref();
            if map.is_some_and(|map| map.layers.is_empty()) {
                return;
            }
            let Some(intent) =
                admit_request(&request, &snapshot, editor_instance_id, &sequence_cache)
            else {
                return;
            };
            if is_noop(&intent, &snapshot) {
                return;
            }

            let operation_id = runtime.operation();
            let outcome = runtime.observe_operation(operation_id);
            let waiting = PendingMacroEdit {
                request: request.clone(),
                operation_id,
                outcome,
                intent: intent.clone(),
            };
            let change = core_change(&intent);
            let transaction_id = format!(
                "keymap-macro-{editor_instance_id}-{}-{}",
                request.request_id, operation_id.0
            );
            pending.set(Some(waiting.clone()));
            feedback.set(Some(MacroFeedbackState {
                request: request.clone(),
                operation_id,
                intent,
                status: MacroEditStatus::Pending,
                failure_snapshot_token: None,
            }));
            last_admitted_request.set(request.request_id);
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
    });

    let (projected_source, sequences) = projected
        .map_or((None, Rc::from([])), |(source, sequences)| {
            (Some(source), sequences)
        });
    MacroActions {
        editor_instance_id,
        request_sequence,
        source: projected_source,
        sequences,
        enabled,
        feedback: visible_feedback,
        on_change,
    }
}

fn public_feedback(state: &MacroFeedbackState, status: MacroEditStatus) -> MacroEditFeedback {
    MacroEditFeedback {
        scope: state.request.scope.clone(),
        scope_generation: state.request.scope_generation,
        admission_token: state.request.admission_token,
        admission_revision: state.request.admission_revision,
        editor_instance_id: state.request.editor_instance_id,
        request_id: state.request.request_id,
        macro_id: state.intent.macro_id.clone(),
        target: state.intent.target,
        status,
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
    let expected = expected_source?;
    let model = runtime.model();
    let snapshot = model.accepted?;
    let revision = expected.revision;
    (runtime.scope().as_ref() == Some(&expected.scope)
        && expected.scope.session_epoch == snapshot.session_epoch
        && expected.scope.document_id == snapshot.document.id
        && expected.scope.board_id == model.active_board_id
        && expected.scope.instance_id == model.active_instance_id
        && snapshot.token == expected.token
        && snapshot.document.revision == revision
        && snapshot
            .document
            .keymap
            .as_ref()
            .is_none_or(|map| !map.layers.is_empty())
        && model.lifecycle == Lifecycle::Ready
        && model.durability == (Durability::Saved { revision })
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && generation() == expected_generation)
        .then_some(snapshot)
}

fn current_display_source(
    runtime: &Runtime,
    expected_source: Option<&LayerSource>,
    expected_generation: u64,
    generation: Signal<u64>,
) -> Option<AcceptedSnapshot> {
    let expected = expected_source?;
    let model = runtime.model();
    let snapshot = model.accepted?;
    (runtime.scope().as_ref() == Some(&expected.scope)
        && expected.scope.session_epoch == snapshot.session_epoch
        && expected.scope.document_id == snapshot.document.id
        && expected.scope.board_id == model.active_board_id
        && expected.scope.instance_id == model.active_instance_id
        && snapshot.token == expected.token
        && snapshot.document.revision == expected.revision
        && generation() == expected_generation)
        .then_some(snapshot)
}

fn admit_request(
    request: &MacroEditRequest,
    snapshot: &AcceptedSnapshot,
    editor_id: u64,
    cache: &SequenceCache,
) -> Option<MacroIntent> {
    if request.editor_instance_id != editor_id {
        return None;
    }
    let macros = snapshot
        .document
        .keymap
        .as_ref()
        .map_or(&[][..], |map| map.macros.as_slice());
    let requested = request.macro_id.as_deref();
    match (&request.target, &request.change) {
        (
            MacroEditTarget::AddMacro,
            MacroEditChange::Add {
                name,
                tap_ms,
                wait_ms,
                steps,
            },
        ) if requested.is_none()
            && request.step_sequence.is_none()
            && macros.len() < 128
            && *tap_ms == 30
            && *wait_ms == 0
            && name == &format!("Macro {}", macros.len() + 1)
            && steps.as_slice() == [tap_a()] =>
        {
            let nonce = editor_id ^ request.request_id.rotate_left(17) ^ snapshot.token.0;
            let mut id = format!("keymap-macro-{editor_id}-{nonce}");
            let mut suffix = 0u32;
            while macros.iter().any(|item| item.id == id) {
                suffix = suffix.checked_add(1)?;
                id = format!("keymap-macro-{editor_id}-{nonce}-{suffix}");
            }
            Some(MacroIntent {
                macro_id: Some(id.clone()),
                target: request.target,
                change: MacroEditChange::Add {
                    name: format!("Macro {}", macros.len() + 1),
                    tap_ms: *tap_ms,
                    wait_ms: *wait_ms,
                    steps: steps.clone(),
                },
                original_field: OriginalField::Absent,
                expected_steps: Some(Rc::from(steps.clone())),
            })
        }
        (MacroEditTarget::RemoveMacro, MacroEditChange::Remove)
            if request.step_sequence.is_none() =>
        {
            let id = requested?;
            (macros.iter().filter(|item| item.id == id).count() == 1).then(|| MacroIntent {
                macro_id: Some(id.to_owned()),
                target: request.target,
                change: request.change.clone(),
                original_field: OriginalField::Present,
                expected_steps: None,
            })
        }
        (target, MacroEditChange::Change(change)) => {
            let id = requested?;
            let item = macros.iter().find(|item| item.id == id)?;
            if macros.iter().filter(|item| item.id == id).count() != 1 {
                return None;
            }
            let (original_field, normalized_change, expected_steps) =
                normalize_change(target, change, item, request, cache)?;
            Some(MacroIntent {
                macro_id: Some(id.to_owned()),
                target: *target,
                change: MacroEditChange::Change(normalized_change),
                original_field,
                expected_steps,
            })
        }
        _ => None,
    }
}

fn normalize_change(
    target: &MacroEditTarget,
    change: &MacroChange,
    item: &KeymapMacro,
    request: &MacroEditRequest,
    cache: &SequenceCache,
) -> Option<PreparedMacroChange> {
    match (target, change) {
        (MacroEditTarget::Name, MacroChange::Name { value }) if request.step_sequence.is_none() => {
            Some((
                OriginalField::Name(item.name.clone()),
                MacroChange::Name {
                    value: value.clone(),
                },
                None,
            ))
        }
        (MacroEditTarget::TapMs, MacroChange::TapMs { value })
            if request.step_sequence.is_none() =>
        {
            Some((
                OriginalField::TapMs(item.tap_ms),
                MacroChange::TapMs { value: *value },
                None,
            ))
        }
        (MacroEditTarget::WaitMs, MacroChange::WaitMs { value })
            if request.step_sequence.is_none() =>
        {
            Some((
                OriginalField::WaitMs(item.wait_ms),
                MacroChange::WaitMs { value: *value },
                None,
            ))
        }
        (MacroEditTarget::AddStep, MacroChange::AddStep { value }) => {
            let current = verified_steps(item, request, cache)?;
            if current.len() >= 128 || value != &tap_a() {
                return None;
            }
            let mut next = current.to_vec();
            next.push(value.clone());
            Some((
                OriginalField::Steps(current),
                MacroChange::AddStep {
                    value: value.clone(),
                },
                Some(Rc::from(next)),
            ))
        }
        (
            MacroEditTarget::RemoveStep { index },
            MacroChange::RemoveStep {
                index: change_index,
            },
        ) if index == change_index => {
            let current = verified_steps(item, request, cache)?;
            if current.len() <= 1 || *index >= current.len() {
                return None;
            }
            let mut next = current.to_vec();
            next.remove(*index);
            Some((
                OriginalField::Steps(current),
                MacroChange::RemoveStep { index: *index },
                Some(Rc::from(next)),
            ))
        }
        (
            MacroEditTarget::StepKind { index },
            MacroChange::Step {
                index: change_index,
                value,
            },
        ) if index == change_index => {
            let current = verified_steps(item, request, cache)?;
            if *index >= current.len() || !valid_kind_default(value) {
                return None;
            }
            let mut next = current.to_vec();
            next[*index] = value.clone();
            Some((
                OriginalField::Steps(current),
                MacroChange::Step {
                    index: *index,
                    value: value.clone(),
                },
                Some(Rc::from(next)),
            ))
        }
        (
            MacroEditTarget::StepDelay { index },
            MacroChange::Step {
                index: change_index,
                value: MacroStep::Wait { ms },
            },
        ) if index == change_index => {
            let current = verified_steps(item, request, cache)?;
            if *index >= current.len() || !matches!(current[*index], MacroStep::Wait { .. }) {
                return None;
            }
            let mut next = current.to_vec();
            next[*index] = MacroStep::Wait { ms: *ms };
            Some((
                OriginalField::Steps(current),
                MacroChange::Step {
                    index: *index,
                    value: MacroStep::Wait { ms: *ms },
                },
                Some(Rc::from(next)),
            ))
        }
        (
            MacroEditTarget::StepKeycode { index },
            MacroChange::Step {
                index: change_index,
                value,
            },
        ) if index == change_index => {
            let current = verified_steps(item, request, cache)?;
            let old = current.get(*index)?;
            let code = match value {
                MacroStep::Tap {
                    binding: KeyBinding::KeyPress { keycode },
                } if matches!(old, MacroStep::Tap { .. }) => keycode,
                MacroStep::Press {
                    binding: KeyBinding::KeyPress { keycode },
                } if matches!(old, MacroStep::Press { .. }) => keycode,
                MacroStep::Release {
                    binding: KeyBinding::KeyPress { keycode },
                } if matches!(old, MacroStep::Release { .. }) => keycode,
                _ => return None,
            };
            let merged = match old {
                MacroStep::Tap { .. } => MacroStep::Tap {
                    binding: KeyBinding::KeyPress {
                        keycode: code.clone(),
                    },
                },
                MacroStep::Press { .. } => MacroStep::Press {
                    binding: KeyBinding::KeyPress {
                        keycode: code.clone(),
                    },
                },
                MacroStep::Release { .. } => MacroStep::Release {
                    binding: KeyBinding::KeyPress {
                        keycode: code.clone(),
                    },
                },
                MacroStep::Wait { .. } => return None,
            };
            let mut next = current.to_vec();
            next[*index] = merged.clone();
            Some((
                OriginalField::Steps(current),
                MacroChange::Step {
                    index: *index,
                    value: merged,
                },
                Some(Rc::from(next)),
            ))
        }
        _ => None,
    }
}

fn verified_steps(
    item: &KeymapMacro,
    request: &MacroEditRequest,
    cache: &SequenceCache,
) -> Option<Rc<[MacroStep]>> {
    let requested = request.step_sequence.as_ref()?;
    let cached = cache.by_macro.borrow().get(&item.id)?.clone();
    (Rc::ptr_eq(requested, &cached) && requested.as_ref() == item.steps.as_slice())
        .then_some(cached)
}

fn valid_kind_default(step: &MacroStep) -> bool {
    match step {
        MacroStep::Wait { ms } => *ms == 100,
        MacroStep::Tap { binding }
        | MacroStep::Press { binding }
        | MacroStep::Release { binding } => {
            matches!(binding, KeyBinding::KeyPress { keycode } if keycode == "A")
        }
    }
}

fn tap_a() -> MacroStep {
    MacroStep::Tap {
        binding: KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}

fn is_noop(intent: &MacroIntent, snapshot: &AcceptedSnapshot) -> bool {
    let Some(id) = intent.macro_id.as_deref() else {
        return false;
    };
    let item = snapshot
        .document
        .keymap
        .as_ref()
        .and_then(|map| map.macros.iter().find(|item| item.id == id));
    match &intent.change {
        MacroEditChange::Change(MacroChange::Name { value }) => {
            item.is_some_and(|item| item.name == *value)
        }
        MacroEditChange::Change(MacroChange::TapMs { value }) => {
            item.is_some_and(|item| item.tap_ms == *value)
        }
        MacroEditChange::Change(MacroChange::WaitMs { value }) => {
            item.is_some_and(|item| item.wait_ms == *value)
        }
        MacroEditChange::Change(MacroChange::Step { index, value }) => item
            .and_then(|item| item.steps.get(*index))
            .is_some_and(|step| step == value),
        _ => false,
    }
}

fn core_change(intent: &MacroIntent) -> KeymapChange {
    match (&intent.change, &intent.macro_id) {
        (
            MacroEditChange::Add {
                name,
                tap_ms,
                wait_ms,
                steps,
            },
            Some(id),
        ) => KeymapChange::SaveMacro {
            value: KeymapMacro {
                id: id.clone(),
                name: name.clone(),
                tap_ms: *tap_ms,
                wait_ms: *wait_ms,
                steps: steps.clone(),
            },
        },
        (MacroEditChange::Remove, Some(id)) => KeymapChange::RemoveMacro { id: id.clone() },
        (MacroEditChange::Change(change), Some(id)) => KeymapChange::EditMacro {
            macro_id: id.clone(),
            change: change.clone(),
        },
        _ => unreachable!("admitted macro intent has a complete entity identity"),
    }
}

fn intent_applied(intent: &MacroIntent, snapshot: &AcceptedSnapshot) -> bool {
    let macros = snapshot
        .document
        .keymap
        .as_ref()
        .map_or(&[][..], |map| map.macros.as_slice());
    match (&intent.change, intent.macro_id.as_deref()) {
        (
            MacroEditChange::Add {
                name,
                tap_ms,
                wait_ms,
                steps,
            },
            Some(id),
        ) => macros.iter().any(|item| {
            item.id == id
                && item.name == *name
                && item.tap_ms == *tap_ms
                && item.wait_ms == *wait_ms
                && item.steps == *steps
        }),
        (MacroEditChange::Remove, Some(id)) => macros.iter().all(|item| item.id != id),
        (MacroEditChange::Change(MacroChange::Name { value }), Some(id)) => macros
            .iter()
            .any(|item| item.id == id && item.name == *value),
        (MacroEditChange::Change(MacroChange::TapMs { value }), Some(id)) => macros
            .iter()
            .any(|item| item.id == id && item.tap_ms == *value),
        (MacroEditChange::Change(MacroChange::WaitMs { value }), Some(id)) => macros
            .iter()
            .any(|item| item.id == id && item.wait_ms == *value),
        (_, Some(id)) => intent.expected_steps.as_ref().is_some_and(|expected| {
            macros
                .iter()
                .any(|item| item.id == id && item.steps.as_slice() == expected.as_ref())
        }),
        _ => false,
    }
}

fn failure_still_relevant(
    intent: &MacroIntent,
    snapshot: &AcceptedSnapshot,
    cache: &SequenceCache,
) -> bool {
    let macros = snapshot
        .document
        .keymap
        .as_ref()
        .map_or(&[][..], |map| map.macros.as_slice());
    let item = intent
        .macro_id
        .as_deref()
        .and_then(|id| macros.iter().find(|item| item.id == id));
    match (&intent.original_field, &intent.change) {
        (OriginalField::Absent, MacroEditChange::Add { .. }) => item.is_none(),
        (OriginalField::Present, MacroEditChange::Remove) => item.is_some(),
        (OriginalField::Name(value), _) => item.is_some_and(|item| item.name == *value),
        (OriginalField::TapMs(value), _) => item.is_some_and(|item| item.tap_ms == *value),
        (OriginalField::WaitMs(value), _) => item.is_some_and(|item| item.wait_ms == *value),
        (OriginalField::Steps(value), _) => item.is_some_and(|item| {
            item.steps.as_slice() == value.as_ref()
                && cache
                    .by_macro
                    .borrow()
                    .get(&item.id)
                    .is_some_and(|current| Rc::ptr_eq(current, value))
        }),
        _ => false,
    }
}
