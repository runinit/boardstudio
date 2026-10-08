//! Editor-lifetime admission and exact-operation acknowledgement for macro edits.
use super::layer_controller::LayerSource;
use super::macro_editor::{
    MacroEditChange, MacroEditFeedback, MacroEditRequest, MacroEditStatus, MacroEditTarget,
    MacroReadSource, MacroStepSequence,
};
use super::owned_edits::OwnedEdits;
use crate::macro_edit::{PrecedingStructure, macro_resolver};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Durability, Lifecycle};
use boardstudio_core::model::{MacroChange, MacroStep};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use dioxus::prelude::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// A macro edit's logical identity: the latest edit for it replaces the earlier one. The
/// key carries its request for follow-ups and draft projection; keys compare by macro and
/// target only.
#[derive(Clone)]
struct MacroKey {
    macro_id: Option<String>,
    target: MacroEditTarget,
    request: Option<Rc<MacroEditRequest>>,
}

impl PartialEq for MacroKey {
    fn eq(&self, other: &Self) -> bool {
        self.macro_id == other.macro_id && self.target == other.target
    }
}

impl MacroKey {
    fn of(request: &MacroEditRequest) -> Self {
        Self {
            macro_id: request.macro_id.clone(),
            target: request.target,
            request: Some(Rc::new(request.clone())),
        }
    }
}

type MacroPending = OwnedEdits<MacroKey>;

#[derive(Clone, Copy)]
struct MacroTickets(Signal<MacroPending>);

fn is_action(target: MacroEditTarget) -> bool {
    matches!(
        target,
        MacroEditTarget::AddMacro
            | MacroEditTarget::RemoveMacro
            | MacroEditTarget::AddStep
            | MacroEditTarget::RemoveStep { .. }
    )
}

fn structural_action(target: MacroEditTarget) -> bool {
    matches!(
        target,
        MacroEditTarget::AddStep | MacroEditTarget::RemoveStep { .. }
    )
}

fn same_action(left: MacroEditTarget, right: MacroEditTarget) -> bool {
    left == right || (structural_action(left) && structural_action(right))
}

pub(super) fn action_pending(macro_id: Option<&str>, target: MacroEditTarget) -> bool {
    try_consume_context::<MacroTickets>().is_some_and(|tickets| {
        tickets
            .0
            .read()
            .pending()
            .any(|entry| entry.macro_id.as_deref() == macro_id && same_action(entry.target, target))
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
        for request in tickets
            .0
            .read()
            .pending()
            .filter_map(|entry| entry.request.as_deref())
            .filter(|request| {
                request.scope == *scope && request.macro_id.as_deref() == Some(macro_id)
            })
        {
            if let MacroEditChange::Change(MacroChange::Step {
                index: changed,
                value: draft,
            }) = &request.change
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
    let pending = use_signal(MacroPending::default);
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
            if !pending.peek().has_terminal() {
                return;
            }
            let results = pending.peek().helper.settle(true, |_| String::new());
            for result in results {
                let (PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key }) = &result;
                let Some(request) = key.request.as_deref() else {
                    continue;
                };
                let live = runtime.scope().as_ref() == Some(&request.scope)
                    && scope_generation() == request.scope_generation;
                match &result {
                    PendingEditResult::Failed { message, .. } if live => {
                        if let Some(feedback) = feedback
                            .write()
                            .iter_mut()
                            .find(|feedback| feedback.request_id == request.request_id)
                        {
                            feedback.status = MacroEditStatus::Failed(message.clone());
                        }
                    }
                    _ => feedback
                        .write()
                        .retain(|feedback| feedback.request_id != request.request_id),
                }
            }
            pending.write().prune();
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
                && pending.peek().pending().any(|entry| {
                    entry.macro_id == request.macro_id && same_action(entry.target, request.target)
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
            // Structural one-shots share the macro sequence's control gate. At most
            // one can precede a field, so its positional effect can be captured purely.
            let preceding_structure = pending
                .peek()
                .pending()
                .find(|entry| entry.macro_id == request.macro_id && structural_action(entry.target))
                .and_then(|entry| {
                    let before = entry.request.as_ref()?.step_sequence.as_ref()?.len();
                    let now = request.step_sequence.as_ref()?.len();
                    match entry.target {
                        MacroEditTarget::AddStep if now == before => {
                            Some(PrecedingStructure::Append)
                        }
                        MacroEditTarget::AddStep if now == before + 1 => None,
                        MacroEditTarget::RemoveStep { index } if now == before => {
                            Some(PrecedingStructure::Remove(index))
                        }
                        MacroEditTarget::RemoveStep { .. } if now + 1 == before => None,
                        _ => Some(PrecedingStructure::Ambiguous),
                    }
                });
            let seed = runtime.operation().0;
            feedback.write().retain(|entry| {
                !(entry.macro_id == request.macro_id && entry.target == request.target)
            });
            feedback
                .write()
                .push(public_feedback(&request, MacroEditStatus::Pending));
            if pending
                .peek()
                .owner_changed(Some(&scope), captured_generation)
            {
                pending
                    .write()
                    .follow_owner(Some(&scope), captured_generation);
            }
            let key = MacroKey::of(&request);
            pending.peek().helper.begin_field(
                &runtime,
                key.clone(),
                "keymap-macro",
                Some("macro".into()),
                macro_resolver(request.clone(), seed, preceding_structure),
                "",
            );
            pending.write().remember(key);
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
