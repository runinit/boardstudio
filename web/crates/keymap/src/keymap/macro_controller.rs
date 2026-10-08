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
    /// A key without a request, for binding a field's Signals.
    fn bound(macro_id: &str, target: MacroEditTarget) -> Self {
        Self {
            macro_id: Some(macro_id.to_owned()),
            target,
            request: None,
        }
    }

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

/// Bind a macro text field's draft and failure Signals to the controller's helper for as
/// long as the calling component is mounted. The component's Signals are released when it
/// leaves, so settlement never writes them afterwards.
pub(super) fn use_bound_macro_field(
    macro_id: &str,
    target: MacroEditTarget,
    draft: Signal<String>,
    failure: Signal<Option<String>>,
) {
    let edits = try_consume_context::<MacroTickets>();
    let bound = use_hook(|| Rc::new(RefCell::new(None::<MacroKey>)));
    if let Some(edits) = edits {
        let key = MacroKey::bound(macro_id, target);
        let mut previous = bound.borrow_mut();
        if let Some(old) = previous.as_ref()
            && *old != key
        {
            edits.0.peek().unbind_field(old);
        }
        edits.0.peek().bind_field(key.clone(), draft, failure);
        *previous = Some(key);
    }
    use_drop({
        let bound = bound.clone();
        move || {
            if let (Some(edits), Some(key)) = (edits, bound.borrow_mut().take()) {
                edits.0.peek().unbind_field(&key);
            }
        }
    });
}

/// The text a macro text field shows for the accepted document.
fn accepted_macro_text(
    document: Option<&boardstudio_core::model::ProjectDoc>,
    key: &MacroKey,
) -> String {
    let item = document
        .and_then(|document| document.keymap.as_ref())
        .zip(key.macro_id.as_deref())
        .and_then(|(map, id)| map.macros.iter().find(|item| item.id == id));
    match (item, key.target) {
        (Some(item), MacroEditTarget::Name) => item.name.clone(),
        (Some(item), MacroEditTarget::TapMs) => item.tap_ms.to_string(),
        (Some(item), MacroEditTarget::WaitMs) => item.wait_ms.to_string(),
        (
            Some(item),
            MacroEditTarget::StepKeycode { index } | MacroEditTarget::StepDelay { index },
        ) => item
            .steps
            .get(index)
            .map_or_else(String::new, |step| macro_step_text(step, key.target)),
        _ => String::new(),
    }
}

/// The draft text a request submits for its field.
fn submitted_macro_text(request: &MacroEditRequest) -> String {
    match &request.change {
        MacroEditChange::Change(MacroChange::Name { value }) => value.clone(),
        MacroEditChange::Change(MacroChange::TapMs { value })
        | MacroEditChange::Change(MacroChange::WaitMs { value }) => value.to_string(),
        MacroEditChange::Change(MacroChange::Step { index, value })
            if matches!(request.target,
                MacroEditTarget::StepKeycode { index: target } | MacroEditTarget::StepDelay { index: target }
                if target == *index) =>
        {
            macro_step_text(value, request.target)
        }
        _ => String::new(),
    }
}

fn macro_step_text(step: &MacroStep, target: MacroEditTarget) -> String {
    match (step, target) {
        (MacroStep::Wait { ms }, MacroEditTarget::StepDelay { .. }) => ms.to_string(),
        (
            MacroStep::Tap { binding }
            | MacroStep::Press { binding }
            | MacroStep::Release { binding },
            MacroEditTarget::StepKeycode { .. },
        ) => match binding {
            boardstudio_core::model::KeyBinding::KeyPress { keycode } => keycode.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

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

/// Keeps controller Signals for the Editor lifetime; UI observations belong to
/// the visible Keymap panel and retire when another workspace hides it.
pub fn use_macro_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> MacroActions {
    let version = use_context::<Signal<u64>>()();
    let observed_workspace = workspace();
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

    use_effect(use_reactive((&version, &observed_workspace), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |(_, observed_workspace)| {
            // Scope changes and leaving the panel retire its UI observations before
            // bound Signals receive outcomes. Authoritative Session edits continue.
            let owner_scope = if observed_workspace == "Keymap" {
                runtime.scope()
            } else {
                None
            };
            let owner_generation = scope_generation();
            if pending
                .peek()
                .owner_changed(owner_scope.as_ref(), owner_generation)
            {
                pending
                    .write()
                    .follow_owner(owner_scope.as_ref(), owner_generation);
                feedback.write().retain(|entry| {
                    owner_scope.as_ref() == Some(&entry.scope)
                        && entry.scope_generation == owner_generation
                });
            }
            if !pending.peek().has_terminal() {
                return;
            }
            let document = runtime.model().accepted.map(|snapshot| snapshot.document);
            let results = pending
                .peek()
                .helper
                .settle(true, |key| accepted_macro_text(document.as_deref(), key));
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
                    // A bound field reports its failure through the helper's Signal at the
                    // field; only actions without a bound field report in the panel status.
                    PendingEditResult::Failed { message, .. } if live && is_action(key.target) => {
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
            let submitted_text = pending
                .peek()
                .bound_draft(&key)
                .unwrap_or_else(|| submitted_macro_text(&request));
            pending.peek().helper.begin_field(
                &runtime,
                key.clone(),
                "keymap-macro",
                Some("macro".into()),
                macro_resolver(request.clone(), seed, preceding_structure),
                &submitted_text,
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

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_step_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::Event;
    use boardstudio_core::model::{KeyBinding, ProjectDoc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[derive(Clone)]
    struct Probe {
        runtime: Rc<Runtime>,
        generation: Rc<RefCell<Option<Signal<u64>>>>,
        workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
    }

    fn host() -> Element {
        let probe = use_context::<Probe>();
        let runtime = probe.runtime.clone();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let _ = version();
        let workspace = use_signal(|| "Keymap");
        *probe.workspace.borrow_mut() = Some(workspace);
        let generation = use_signal(|| 0_u64);
        *probe.generation.borrow_mut() = Some(generation);
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let actions = use_macro_operations(
            runtime,
            Some(LayerSource {
                scope: scope.clone(),
                token: accepted.token,
                revision: accepted.document.revision,
            }),
            workspace,
            generation,
            Rc::new(|| true),
        );
        let source = actions.source.filter(|_| workspace() == "Keymap");
        rsx! {
            if let Some(source) = source {
                super::super::macro_editor::MacroEditor {
                    scope, scope_generation: generation(),
                    editor_instance_id: actions.editor_instance_id,
                    request_sequence: actions.request_sequence,
                    source, sequences: actions.sequences,
                    enabled: actions.enabled, feedback: actions.feedback,
                    on_change: actions.on_change,
                }
            }
        }
    }

    async fn mounted() -> (Probe, web_sys::Element) {
        let runtime = support::new_runtime();
        let mut doc = ProjectDoc::empty("macro-steps", "Macro steps");
        doc.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        doc.keymap = Some(
            serde_json::from_value(serde_json::json!({
                "layers": [{"id": "base", "name": "Base", "bindings": {}, "sensors": {}}],
                "macros": [{"id": "macro", "name": "Original", "tapMs": 30, "waitMs": 0,
                    "steps": [{"kind": "tap", "binding": {"kind": "key-press", "keycode": "A"}},
                              {"kind": "wait", "ms": 100}]}]
            }))
            .unwrap(),
        );
        support::open_document(&runtime, doc).await;
        let probe = Probe {
            runtime,
            generation: Rc::new(RefCell::new(None)),
            workspace: Rc::new(RefCell::new(None)),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;
        (probe, root)
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }

    async fn settle(runtime: &Rc<Runtime>) {
        for _ in 0..15 {
            support::run_pending(runtime).await;
            rendered().await;
        }
    }

    fn input(root: &web_sys::Element, label: &str) -> web_sys::HtmlInputElement {
        root.query_selector(&format!("input[aria-label='{label}']"))
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn type_value(input: &web_sys::HtmlInputElement, value: &str) {
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    }

    fn commit(input: &web_sys::HtmlInputElement, value: &str) {
        input.focus().unwrap();
        type_value(input, value);
        input.blur().unwrap();
    }

    fn assert_inline_failure(root: &web_sys::Element, label: &str) {
        let field = input(root, label).parent_element().unwrap();
        let alert = field
            .query_selector("small[role='alert']")
            .unwrap()
            .expect("the failure stays beside its own step field");
        assert!(alert.text_content().unwrap().contains("macro step failed"));
        assert!(
            root.query_selector("p.m1-keymap-macro-status")
                .unwrap()
                .is_none()
        );
    }

    async fn failed_step_restores_unchanged_submitted_text(
        label: &str,
        submitted: &str,
        accepted: &str,
    ) {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let (entered, release) = support::gate_next_core_reply(runtime);
        commit(&input(&root, "Original tapMs"), "31");
        support::drive_pending(runtime);
        entered.await.unwrap();
        // The held reply consumed its gate. The next queued step now receives the
        // injected failure, while this independent duration edit lands normally.
        support::fail_next_core_reply(runtime, "macro step failed");
        commit(&input(&root, label), submitted);
        support::drive_pending(runtime);
        rendered().await;
        // An input event with the same text does not create a newer draft value. The
        // shared helper restores by submitted-value equality, independent of local dirty.
        type_value(&input(&root, label), submitted);
        release.send(()).unwrap();
        settle(runtime).await;
        assert_eq!(input(&root, label).value(), accepted);
        assert_inline_failure(&root, label);
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_failed_keycode_restores_unchanged_submitted_text() {
        failed_step_restores_unchanged_submitted_text("Keycode", "B", "A").await;
    }

    #[wasm_bindgen_test]
    async fn a_failed_delay_restores_unchanged_submitted_text() {
        failed_step_restores_unchanged_submitted_text("Delay (ms)", "250", "100").await;
    }

    async fn newer_step_draft_survives_failure(label: &str, submitted: &str, newer: &str) {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let (entered, release) = support::gate_next_core_reply(runtime);
        commit(&input(&root, "Original tapMs"), "31");
        support::drive_pending(runtime);
        entered.await.unwrap();
        // The held reply consumed its gate. The next queued step now receives the
        // injected failure, while this independent duration edit lands normally.
        support::fail_next_core_reply(runtime, "macro step failed");
        commit(&input(&root, label), submitted);
        support::drive_pending(runtime);
        rendered().await;
        type_value(&input(&root, label), newer);
        release.send(()).unwrap();
        settle(runtime).await;
        assert_eq!(input(&root, label).value(), newer);
        assert_inline_failure(&root, label);
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_newer_keycode_draft_keeps_the_older_failure_inline() {
        newer_step_draft_survives_failure("Keycode", "B", "C").await;
    }

    #[wasm_bindgen_test]
    async fn a_newer_delay_draft_keeps_the_older_failure_inline() {
        newer_step_draft_survives_failure("Delay (ms)", "250", "375").await;
    }

    #[wasm_bindgen_test]
    async fn the_latest_step_keycode_observation_preserves_both_undo_steps() {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let (entered, release) = support::gate_next_core_reply(runtime);
        commit(&input(&root, "Keycode"), "B");
        support::drive_pending(runtime);
        entered.await.unwrap();
        rendered().await;
        commit(&input(&root, "Keycode"), "C");
        support::drive_pending(runtime);
        release.send(()).unwrap();
        settle(runtime).await;
        assert_eq!(input(&root, "Keycode").value(), "C");
        assert!(
            root.query_selector("small[role='alert']")
                .unwrap()
                .is_none()
        );
        for expected in ["C", "B", "A"] {
            let accepted = runtime.model().accepted.unwrap();
            assert_eq!(
                accepted.document.keymap.as_ref().unwrap().macros[0].steps[0],
                MacroStep::Tap {
                    binding: KeyBinding::KeyPress {
                        keycode: expected.into()
                    }
                }
            );
            if expected != "A" {
                runtime.submit(Event::Undo {
                    operation_id: runtime.operation(),
                });
                settle(runtime).await;
            }
        }
        assert_eq!(input(&root, "Keycode").value(), "A");
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_departed_macro_generation_cannot_write_its_failure_into_the_same_step_field() {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let (entered, release) = support::gate_next_core_reply(runtime);
        commit(&input(&root, "Original tapMs"), "31");
        support::drive_pending(runtime);
        entered.await.unwrap();
        support::fail_next_core_reply(runtime, "macro step failed");
        commit(&input(&root, "Keycode"), "B");
        support::drive_pending(runtime);
        rendered().await;
        let mut generation = probe.generation.borrow().unwrap();
        generation += 1;
        rendered().await;
        assert!(
            root.query_selector("input[aria-label='Keycode']")
                .unwrap()
                .is_some()
        );
        release.send(()).unwrap();
        settle(runtime).await;
        assert!(
            root.query_selector("small[role='alert']")
                .unwrap()
                .is_none(),
            "a departed owner's failure is silent while the same logical field remains mounted"
        );
        assert_eq!(input(&root, "Keycode").value(), "A");
        assert!(
            root.query_selector("p.m1-keymap-macro-status")
                .unwrap()
                .is_none()
        );
        root.remove();
    }

    async fn held_raw_step_text_lands_as_accepted_text(label: &str, raw: &str, accepted: &str) {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let (entered, release) = support::gate_next_core_reply(runtime);
        commit(&input(&root, label), raw);
        support::drive_pending(runtime);
        entered.await.unwrap();
        rendered().await;
        assert_eq!(
            input(&root, label).value(),
            raw,
            "the exact typed text stays visible while its edit is pending"
        );
        release.send(()).unwrap();
        settle(runtime).await;
        assert_eq!(
            input(&root, label).value(),
            accepted,
            "settlement projects the canonical accepted value"
        );
        assert_accepted_step_value(runtime, label, accepted);
        root.remove();
    }

    fn assert_accepted_step_value(runtime: &Runtime, label: &str, expected: &str) {
        let accepted = runtime.model().accepted.unwrap();
        let steps = &accepted.document.keymap.as_ref().unwrap().macros[0].steps;
        if label == "Keycode" {
            assert_eq!(
                steps[0],
                MacroStep::Tap {
                    binding: KeyBinding::KeyPress {
                        keycode: expected.into()
                    },
                }
            );
        } else {
            assert_eq!(
                steps[1],
                MacroStep::Wait {
                    ms: expected.parse().unwrap()
                }
            );
        }
    }

    #[wasm_bindgen_test]
    async fn raw_step_keycode_whitespace_stays_pending_and_lands_canonical() {
        held_raw_step_text_lands_as_accepted_text("Keycode", " B ", "B").await;
    }

    #[wasm_bindgen_test]
    async fn raw_step_delay_leading_zeros_stay_pending_and_land_canonical() {
        held_raw_step_text_lands_as_accepted_text("Delay (ms)", "003", "3").await;
    }

    async fn newer_raw_step_text_survives_older_landing(
        label: &str,
        raw: &str,
        newer: &str,
        accepted: &str,
    ) {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let (entered, release) = support::gate_next_core_reply(runtime);
        commit(&input(&root, label), raw);
        support::drive_pending(runtime);
        entered.await.unwrap();
        rendered().await;
        type_value(&input(&root, label), newer);
        release.send(()).unwrap();
        settle(runtime).await;
        assert_eq!(
            input(&root, label).value(),
            newer,
            "the older landing does not replace a newer exact draft"
        );
        assert_accepted_step_value(runtime, label, accepted);
        assert!(
            root.query_selector("small[role='alert']")
                .unwrap()
                .is_none()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn raw_step_newer_keycode_whitespace_survives_older_landing() {
        newer_raw_step_text_survives_older_landing("Keycode", " B ", " C ", "B").await;
    }

    #[wasm_bindgen_test]
    async fn raw_step_newer_delay_leading_zeros_survive_older_landing() {
        newer_raw_step_text_survives_older_landing("Delay (ms)", "003", "004", "3").await;
    }

    #[wasm_bindgen_test]
    async fn keymap_workspace_departure_retires_a_held_step_projection_without_cancelling_session()
    {
        let (probe, root) = mounted().await;
        let runtime = &probe.runtime;
        let kind = || {
            root.query_selector("select[aria-label='Original step 1']")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlSelectElement>()
                .unwrap()
        };
        let (entered, release) = support::gate_next_core_reply(runtime);
        kind().set_value("wait");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        kind()
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
            .unwrap();
        support::drive_pending(runtime);
        entered.await.unwrap();
        rendered().await;
        assert_eq!(
            kind().value(),
            "wait",
            "the original owner displays its pending kind"
        );
        let mut workspace = probe.workspace.borrow().unwrap();
        workspace.set("Parts");
        rendered().await;
        assert!(
            root.query_selector(".m1-keymap-macros").unwrap().is_none(),
            "the panel actually unmounts while the Editor controller remains mounted"
        );
        workspace.set("Keymap");
        rendered().await;
        assert_eq!(
            kind().value(),
            "tap",
            "returning before the held reply shows the accepted kind, without reviving old observation"
        );
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.keymap.as_ref().unwrap().macros[0].steps[0],
            MacroStep::Tap {
                binding: KeyBinding::KeyPress {
                    keycode: "A".into()
                }
            }
        );
        release.send(()).unwrap();
        settle(runtime).await;
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.keymap.as_ref().unwrap().macros[0].steps[0],
            MacroStep::Wait { ms: 100 },
            "leaving the panel never cancels authoritative Session work"
        );
        assert_eq!(kind().value(), "wait");
        assert!(
            root.query_selector("small[role='alert'], p.m1-keymap-macro-status")
                .unwrap()
                .is_none()
        );
        root.remove();
    }
}
