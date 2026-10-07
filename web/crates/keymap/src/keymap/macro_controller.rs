//! Editor-lifetime admission and exact-operation acknowledgement for macro edits.
use super::layer_controller::LayerSource;
use super::macro_editor::{
    MacroEditChange, MacroEditFeedback, MacroEditRequest, MacroEditStatus, MacroEditTarget,
    MacroReadSource, MacroStepSequence,
};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, KeymapChange, KeymapMacro, MacroChange, MacroStep,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone)]
struct MacroTicket {
    request: MacroEditRequest,
    ticket: EditTicket,
}

#[derive(Clone, Copy)]
struct MacroTickets(Signal<Vec<MacroTicket>>);

fn is_action(target: MacroEditTarget) -> bool {
    matches!(
        target,
        MacroEditTarget::AddMacro
            | MacroEditTarget::RemoveMacro
            | MacroEditTarget::AddStep
            | MacroEditTarget::RemoveStep { .. }
    )
}

pub(super) fn action_pending(macro_id: Option<&str>, target: MacroEditTarget) -> bool {
    try_consume_context::<MacroTickets>().is_some_and(|tickets| {
        tickets.0.read().iter().any(|entry| {
            entry.request.macro_id.as_deref() == macro_id
                && entry.request.target == target
                && entry.ticket.is_pending()
        })
    })
}

pub(super) fn draft_step(
    scope: &boardstudio_application::Scope,
    macro_id: &str,
    index: usize,
    accepted: &MacroStep,
) -> MacroStep {
    let mut value = accepted.clone();
    if let Some(tickets) = try_consume_context::<MacroTickets>() {
        for entry in tickets.0.read().iter().filter(|entry| {
            entry.request.scope == *scope
                && entry.request.macro_id.as_deref() == Some(macro_id)
                && entry.ticket.is_pending()
        }) {
            if let MacroEditChange::Change(MacroChange::Step {
                index: changed,
                value: draft,
            }) = &entry.request.change
                && *changed == index
            {
                value = draft.clone();
            }
        }
    }
    value
}

#[derive(Clone, Copy)]
struct MacroFeedbacks(Signal<Vec<MacroEditFeedback>>);

pub(super) fn field_feedback(
    scope: &boardstudio_application::Scope,
    macro_id: &str,
    target: MacroEditTarget,
) -> Option<MacroEditFeedback> {
    let feedback = try_consume_context::<MacroFeedbacks>()?;
    feedback
        .0
        .read()
        .iter()
        .rev()
        .find(|entry| {
            entry.scope == *scope
                && entry.macro_id.as_deref() == Some(macro_id)
                && entry.target == target
        })
        .cloned()
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
    let pending = use_signal(Vec::<MacroTicket>::new);
    use_context_provider(|| MacroTickets(pending));
    let feedback = use_signal(Vec::<MacroEditFeedback>::new);
    use_context_provider(|| MacroFeedbacks(feedback));
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
            let mut tickets = pending.peek().clone();
            let before = tickets.len();
            tickets.retain(|entry| {
                let live = runtime.scope().as_ref() == Some(&entry.request.scope)
                    && scope_generation() == entry.request.scope_generation;
                let status = match entry.ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(MacroEditStatus::Saved),
                    Settlement::Failed { message } => Some(MacroEditStatus::Failed(message)),
                    Settlement::Retired => None,
                };
                if let Some(status) = status {
                    if let Some(feedback) = feedback
                        .write()
                        .iter_mut()
                        .find(|feedback| feedback.request_id == entry.request.request_id)
                    {
                        feedback.status = status;
                    }
                } else {
                    feedback
                        .write()
                        .retain(|feedback| feedback.request_id != entry.request.request_id);
                }
                false
            });
            if before != tickets.len() {
                pending.set(tickets);
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
    let admission_snapshot = current_edit_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    );
    let enabled = admission_snapshot.is_some() && projected.is_some();

    let visible_feedback = feedback
        .read()
        .iter()
        .rev()
        .find(|entry| {
            runtime.scope().as_ref() == Some(&entry.scope)
                && scope_generation() == entry.scope_generation
        })
        .cloned();

    let on_change = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let source = source.clone();
        let sequence_cache = cache.clone();
        move |request: MacroEditRequest| {
            if request.request_id <= last_admitted_request() {
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
            if is_action(request.target)
                && pending.peek().iter().any(|entry| {
                    entry.request.macro_id == request.macro_id
                        && entry.request.target == request.target
                        && entry.ticket.is_pending()
                })
            {
                return;
            }
            if let Some(sequence) = request.step_sequence.as_ref() {
                if !request
                    .macro_id
                    .as_ref()
                    .and_then(|id| sequence_cache.by_macro.borrow().get(id).cloned())
                    .is_some_and(|current| Rc::ptr_eq(sequence, &current))
                {
                    return;
                }
            }
            let seed = runtime.operation().0;
            let ticket = EditTicket::begin(
                &runtime,
                "keymap-macro",
                Some("macro".into()),
                macro_resolver(request.clone(), seed),
            );
            feedback.write().retain(|entry| {
                !(entry.macro_id == request.macro_id && entry.target == request.target)
            });
            feedback
                .write()
                .push(public_feedback(&request, MacroEditStatus::Pending));
            pending.write().push(MacroTicket {
                request: request.clone(),
                ticket,
            });
            last_admitted_request.set(request.request_id);
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

fn public_feedback(request: &MacroEditRequest, status: MacroEditStatus) -> MacroEditFeedback {
    MacroEditFeedback {
        scope: request.scope.clone(),
        scope_generation: request.scope_generation,
        admission_token: request.admission_token,
        admission_revision: request.admission_revision,
        editor_instance_id: request.editor_instance_id,
        request_id: request.request_id,
        macro_id: request.macro_id.clone(),
        target: request.target,
        status,
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

fn macro_resolver(request: MacroEditRequest, seed: u64) -> EditResolver {
    EditResolver::new("keymap-macro", move |accepted: &AcceptedSnapshot| {
        if accepted.session_epoch != request.scope.session_epoch
            || accepted.document.id != request.scope.document_id
            || !accepted
                .document
                .boards
                .iter()
                .any(|board| board.id == request.scope.board_id)
        {
            return Resolution::Retire("This board no longer exists.".into());
        }
        let macros = accepted
            .document
            .keymap
            .as_ref()
            .map_or(&[][..], |map| map.macros.as_slice());
        let change = match (&request.target, &request.change) {
            (
                MacroEditTarget::AddMacro,
                MacroEditChange::Add {
                    tap_ms,
                    wait_ms,
                    steps,
                    ..
                },
            ) if request.macro_id.is_none() => {
                if macros.len() >= 128 {
                    return Resolution::Retire("The keymap already has 128 macros.".into());
                }
                let mut id = format!("keymap-macro-{seed}");
                let mut suffix = 0u64;
                while macros.iter().any(|item| item.id == id) {
                    suffix += 1;
                    id = format!("keymap-macro-{seed}-{suffix}");
                }
                KeymapChange::SaveMacro {
                    value: KeymapMacro {
                        id,
                        name: format!("Macro {}", macros.len() + 1),
                        tap_ms: *tap_ms,
                        wait_ms: *wait_ms,
                        steps: steps.clone(),
                    },
                }
            }
            _ => {
                let Some(item) = request
                    .macro_id
                    .as_ref()
                    .and_then(|id| macros.iter().find(|item| &item.id == id))
                else {
                    return Resolution::Retire("This macro no longer exists.".into());
                };
                // Steps have positional identities. A structural change invalidates a queued
                // positional request; ordinary field edits leave the sequence length intact.
                if matches!(
                    request.target,
                    MacroEditTarget::RemoveStep { .. }
                        | MacroEditTarget::StepKind { .. }
                        | MacroEditTarget::StepDelay { .. }
                        | MacroEditTarget::StepKeycode { .. }
                ) && request
                    .step_sequence
                    .as_ref()
                    .is_none_or(|steps| steps.len() != item.steps.len())
                {
                    return Resolution::Retire("This macro step is no longer available because the steps changed. Select the step again.".into());
                }
                match (&request.target, &request.change) {
                    (MacroEditTarget::RemoveMacro, MacroEditChange::Remove) => {
                        KeymapChange::RemoveMacro {
                            id: item.id.clone(),
                        }
                    }
                    (target, MacroEditChange::Change(change)) => {
                        let valid = match (target, change) {
                            (MacroEditTarget::Name, MacroChange::Name { value }) => {
                                if item.name == *value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            (MacroEditTarget::TapMs, MacroChange::TapMs { value }) => {
                                if item.tap_ms == *value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            (MacroEditTarget::WaitMs, MacroChange::WaitMs { value }) => {
                                if item.wait_ms == *value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            (MacroEditTarget::AddStep, MacroChange::AddStep { .. }) => {
                                item.steps.len() < 128
                            }
                            (
                                MacroEditTarget::RemoveStep { index },
                                MacroChange::RemoveStep { index: changed },
                            ) => {
                                index == changed
                                    && *index < item.steps.len()
                                    && item.steps.len() > 1
                            }
                            (
                                MacroEditTarget::StepKind { index }
                                | MacroEditTarget::StepDelay { index }
                                | MacroEditTarget::StepKeycode { index },
                                MacroChange::Step {
                                    index: changed,
                                    value,
                                },
                            ) => {
                                let Some(current) = item.steps.get(*index) else {
                                    return Resolution::Retire(
                                        "This macro step no longer exists.".into(),
                                    );
                                };
                                if index != changed {
                                    return Resolution::Retire(
                                        "This macro step is no longer available.".into(),
                                    );
                                }
                                if !matches!(target, MacroEditTarget::StepKind { .. })
                                    && std::mem::discriminant(current)
                                        != std::mem::discriminant(value)
                                {
                                    return Resolution::Retire(
                                        "This macro step field is no longer available.".into(),
                                    );
                                }
                                if current == value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            _ => false,
                        };
                        if !valid {
                            return Resolution::Retire(
                                "This macro field or step is no longer available.".into(),
                            );
                        }
                        KeymapChange::EditMacro {
                            macro_id: item.id.clone(),
                            change: change.clone(),
                        }
                    }
                    _ => {
                        return Resolution::Retire(
                            "This macro edit is no longer available.".into(),
                        );
                    }
                }
            }
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
