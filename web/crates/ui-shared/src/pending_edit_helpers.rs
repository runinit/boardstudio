//! Signal-bound text-field and one-shot helpers over the Runtime `PendingEdits`
//! collection. The helpers own the settlement coordination panels repeated per field —
//! submitted-draft memory, draft restoration, inline failures, one-shot disabling — so a
//! panel binds its Signals once and reads drained results for its own follow-ups.
//!
//! A panel uses the helpers like this:
//!
//! ```ignore
//! let helpers = use_hook(|| Rc::new(PendingEditSignals::<MatrixField>::new()));
//! let rows = use_signal(String::new);
//! let rows_failure = use_signal(|| None::<String>);
//! let preset_disabled = use_signal(|| false);
//! helpers.bind_field(MatrixField::Rows, rows, rows_failure);
//! helpers.bind_one_shot(MatrixField::Preset, preset_disabled);
//!
//! // When the user commits a draft, submit it as intent; the draft stays visible:
//! helpers.begin_field(&runtime, MatrixField::Rows, "matrix-rows", Some("matrix".into()),
//!     resolver, &rows.peek().clone());
//! // One-shot controls disable while their edit is pending:
//! helpers.begin_one_shot(&runtime, MatrixField::Preset, "matrix-preset",
//!     Some("matrix".into()), resolver);
//!
//! // Settle on each render pass (the panel owns the loop, as today):
//! for result in helpers.settle(owner_is_live(), |key| accepted_text(key)) {
//!     // Landed: run caller follow-ups (exact selection, refresh). Failed: place the
//!     // message wherever else the panel needs it. Retired: nothing to do.
//! }
//! ```
//!
//! Invariants (decision [01](../../../../docs/plans/module-deepening/issues/01-decide-pending-edit-settlement.md),
//! [ADR-0005 amendment](../../../../docs/adr/0005-resolve-queued-edits-at-execution.md)):
//! the latest ticket per key drives it; a settlement may only write a draft that still
//! is the text its edit was submitted with, so an older outcome never overwrites a newer
//! draft; failures surface inline at the field by default while the results keep them
//! available for other placement; retirement is silent; there is no Saved state and no
//! automatic retry. Domain projection and value conversion stay with the caller through
//! the `accepted` projection.

use boardstudio_application::EditResolver;
use boardstudio_web_runtime::edit_ticket::EditTicketPort;
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Whether a settlement may write a field's draft: only while the live draft still is
/// the text the edit was submitted with. An unknown submitted draft never qualifies.
fn draft_may_restore(submitted: Option<&str>, current: &str) -> bool {
    submitted.is_some_and(|submitted| submitted == current)
}

/// The Signals one text field binds: its live draft and its inline failure message.
#[derive(Clone, Copy)]
pub struct FieldView {
    pub draft: Signal<String>,
    pub failure: Signal<Option<String>>,
}

/// The Signal one one-shot control binds: unavailable while its edit is pending.
#[derive(Clone, Copy)]
pub struct OneShotView {
    pub disabled: Signal<bool>,
}

struct Binding<K> {
    key: K,
    view: FieldView,
}

struct OneShotBinding<K> {
    key: K,
    view: OneShotView,
}

/// The collection, the bound views and the submitted-draft memory, shared by every
/// clone of [`PendingEditSignals`].
struct Shared<K> {
    edits: RefCell<PendingEdits<K>>,
    fields: RefCell<Vec<Binding<K>>>,
    one_shots: RefCell<Vec<OneShotBinding<K>>>,
    /// The draft text each key's latest edit was submitted with.
    submitted: RefCell<Vec<(K, String)>>,
}

/// Signal-bound coordination over one [`PendingEdits`] collection. Clone to share
/// between a panel's hooks and handlers; every clone drives the same collection and
/// bindings.
#[derive(Clone)]
pub struct PendingEditSignals<K> {
    shared: Rc<Shared<K>>,
}

impl<K> Default for PendingEditSignals<K>
where
    K: PartialEq + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K: PartialEq + 'static> PendingEditSignals<K> {
    pub fn new() -> Self {
        Self {
            shared: Rc::new(Shared {
                edits: RefCell::new(PendingEdits::default()),
                fields: RefCell::new(Vec::new()),
                one_shots: RefCell::new(Vec::new()),
                submitted: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Bind one text field's draft and inline failure Signals. Rebinding a key replaces
    /// its view; only bound fields receive settlement writes.
    pub fn bind_field(&self, key: K, draft: Signal<String>, failure: Signal<Option<String>>) {
        let view = FieldView { draft, failure };
        let mut fields = self.shared.fields.borrow_mut();
        match fields.iter_mut().find(|binding| binding.key == key) {
            Some(binding) => binding.view = view,
            None => fields.push(Binding { key, view }),
        }
    }

    /// Bind one one-shot control's disabled Signal. Rebinding a key replaces its view.
    pub fn bind_one_shot(&self, key: K, disabled: Signal<bool>) {
        let view = OneShotView { disabled };
        let mut one_shots = self.shared.one_shots.borrow_mut();
        match one_shots.iter_mut().find(|binding| binding.key == key) {
            Some(binding) => binding.view = view,
            None => one_shots.push(OneShotBinding { key, view }),
        }
    }

    /// Submit a field edit as intent under `key`, remembering the draft it was
    /// submitted with so a later settlement cannot clobber a newer draft.
    pub fn begin_field(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
        current_draft: &str,
    ) where
        K: Clone,
    {
        self.shared
            .edits
            .borrow_mut()
            .begin(port, key.clone(), label, feature, resolver);
        let mut submitted = self.shared.submitted.borrow_mut();
        match submitted.iter_mut().find(|(existing, _)| *existing == key) {
            Some(entry) => entry.1 = current_draft.to_owned(),
            None => submitted.push((key, current_draft.to_owned())),
        }
    }

    /// Submit a one-shot edit as intent under `key` and disable its bound control until
    /// the edit settles or retires.
    pub fn begin_one_shot(
        &self,
        port: &dyn EditTicketPort,
        key: K,
        label: &str,
        feature: Option<String>,
        resolver: EditResolver,
    ) {
        let disabled = self
            .shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| binding.key == key)
            .map(|binding| binding.view.disabled);
        if let Some(mut disabled) = disabled {
            disabled.set(true);
        }
        self.shared
            .edits
            .borrow_mut()
            .begin(port, key, label, feature, resolver);
    }

    /// Whether the key's latest edit is still pending in its captured document scope.
    pub fn is_pending(&self, key: &K) -> bool {
        self.shared.edits.borrow().is_pending(key)
    }

    /// Drain terminal results once and apply the field policy:
    ///
    /// - *Landed* and *Retired* project the accepted value (via `accepted`) into the
    ///   field's draft while that draft still is the submitted text; retirement is
    ///   silent. *Failed* additionally reports the message inline. A newer draft is
    ///   never overwritten; its field still receives the failure message.
    /// - Any terminal re-enables a bound one-shot control.
    ///
    /// Every drained result is returned in drain order for caller follow-ups — Landed
    /// revisions, failure placement beyond the field, and domain projection the
    /// `accepted` closure does not cover.
    pub fn settle(
        &self,
        owner_is_live: bool,
        accepted: impl Fn(&K) -> String,
    ) -> Vec<PendingEditResult<K>> {
        let results = self.shared.edits.borrow_mut().settle(owner_is_live);

        for result in &results {
            match result {
                PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key } => {
                    self.apply_to_field(key, result, &accepted);
                    self.release_one_shot(key);
                }
            }
        }
        results
    }

    fn apply_to_field(
        &self,
        key: &K,
        result: &PendingEditResult<K>,
        accepted: &impl Fn(&K) -> String,
    ) {
        let mut view = match self
            .shared
            .fields
            .borrow()
            .iter()
            .find(|binding| &binding.key == key)
        {
            Some(binding) => binding.view,
            None => return,
        };
        let message = match result {
            PendingEditResult::Failed { message, .. } => Some(message.as_str()),
            _ => None,
        };
        if let Some(message) = message {
            view.failure.set(Some(message.to_owned()));
        } else if view.failure.read().is_some() {
            view.failure.set(None);
        }
        let submitted = self
            .shared
            .submitted
            .borrow()
            .iter()
            .find(|(existing, _)| existing == key)
            .map(|(_, draft)| draft.as_str())
            .map(|draft| draft.to_owned());
        let current = view.draft.peek().as_str().to_owned();
        let projected = accepted(key);
        if draft_may_restore(submitted.as_deref(), &current) && current != projected {
            view.draft.set(projected);
        }
    }

    fn release_one_shot(&self, key: &K) {
        let disabled = self
            .shared
            .one_shots
            .borrow()
            .iter()
            .find(|binding| &binding.key == key)
            .map(|binding| binding.view.disabled);
        if let Some(mut disabled) = disabled {
            disabled.set(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_requires_the_submitted_draft() {
        assert!(draft_may_restore(Some("1.5"), "1.5"));
        assert!(!draft_may_restore(Some("1.5"), "2.0"), "a newer draft wins");
        assert!(
            !draft_may_restore(None, "1.5"),
            "an unknown draft is never replaced"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    //! Mounted tests drive the helpers through real Signal lifecycle: a launched
    //! VirtualDom with real inputs and buttons, the real WASM Runtime over Session and
    //! Core, and its in-process gates. Native `cargo test` executes only the pure
    //! policy tests above; every behavior below runs in the browser.

    use super::*;
    use boardstudio_application::{Event, Resolution};
    use boardstudio_core::model::{
        Board, EditOperation, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
    };
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::{Event as WebEvent, EventInit, HtmlInputElement};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    type TestRuntime = Rc<boardstudio_web_runtime::runtime::Runtime>;
    use boardstudio_web_runtime::runtime::project_name_test_support as support;

    #[derive(Clone, Debug, PartialEq)]
    enum Field {
        Name,
        Other,
    }

    /// The caller's accepted projection: it converts to a value no real document ever
    /// carried, so restored drafts prove the caller's code ran.
    fn accepted_name(key: &Field) -> String {
        match key {
            Field::Name => "PROJECTED".into(),
            Field::Other => "OTHER".into(),
        }
    }

    fn rename_resolver(name: &'static str) -> EditResolver {
        EditResolver::new("helper-test", move |accepted| {
            let mut document = (*accepted.document).clone();
            document.name = name.into();
            Resolution::submit(
                vec![document.id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        })
    }

    fn unchanged_resolver() -> EditResolver {
        EditResolver::new("helper-test", |_accepted| Resolution::Unchanged)
    }

    #[derive(Clone, Default)]
    struct Probe {
        runtime: Rc<RefCell<Option<TestRuntime>>>,
        /// The drained results of the last settle, for caller follow-up assertions.
        results: Rc<RefCell<Vec<PendingEditResult<Field>>>>,
        /// The pending answer the last begin/settle recorded.
        pending: Rc<Cell<bool>>,
    }

    fn document(id: &str, name: &str) -> ProjectDoc {
        let mut document = ProjectDoc::empty(id, name);
        document.definitions.push(PartDefinition {
            mechanical_profile: None,
            id: "key".into(),
            name: "Key".into(),
            kind: PartKind::Switch,
            courtyard: vec![],
            pads: vec![],
            models: None,
            hardware_profile: None,
            input_profile: None,
            keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            generator: None,
        });
        document.parts.push(Part {
            id: "key".into(),
            definition_id: "key".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    async fn opened_runtime(name: &str) -> TestRuntime {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document("helper-test", name)).await;
        runtime
    }

    fn helper_host() -> Element {
        let probe = use_context::<Probe>();
        let runtime = use_hook(|| {
            probe
                .runtime
                .borrow()
                .clone()
                .expect("the runtime is prepared before mounting")
        });
        let helpers = use_hook(|| PendingEditSignals::<Field>::new());
        let mut draft = use_signal(|| "Zephyr".to_string());
        let failure = use_signal(|| None::<String>);
        let disabled = use_signal(|| false);
        use_hook({
            let helpers = helpers.clone();
            move || {
                helpers.bind_field(Field::Name, draft, failure);
                helpers.bind_one_shot(Field::Other, disabled);
            }
        });

        rsx! {
            input { id: "field-draft", value: "{draft}",
                oninput: move |event: FormEvent| draft.set(event.value()) }
            if let Some(message) = failure() {
                p { id: "field-failure", "{message}" }
            }
            button { id: "one-shot-control", disabled: disabled(),
                "one-shot control" }
            button { id: "begin-field", onclick: {
                    let helpers = helpers.clone();
                    let runtime = runtime.clone();
                    let probe = probe.clone();
                    move |_| {
                        helpers.begin_field(&runtime, Field::Name, "name-field",
                            Some("project".into()), rename_resolver("Renamed"), &draft.peek().clone());
                        probe.pending.set(helpers.is_pending(&Field::Name));
                    }
                }, "begin field" }
            button { id: "begin-failing-field", onclick: {
                    let helpers = helpers.clone();
                    let runtime = runtime.clone();
                    let probe = probe.clone();
                    move |_| {
                        helpers.begin_field(&runtime, Field::Name, "name-field",
                            Some("project".into()), rename_resolver("Rejected"), &draft.peek().clone());
                        probe.pending.set(helpers.is_pending(&Field::Name));
                    }
                }, "begin failing field" }
            button { id: "begin-unchanged-field", onclick: {
                    let helpers = helpers.clone();
                    let runtime = runtime.clone();
                    move |_| {
                        helpers.begin_field(&runtime, Field::Name, "name-field",
                            Some("project".into()), unchanged_resolver(), &draft.peek().clone());
                    }
                }, "begin unchanged field" }
            button { id: "begin-one-shot", onclick: {
                    let helpers = helpers.clone();
                    let runtime = runtime.clone();
                    let probe = probe.clone();
                    move |_| {
                        helpers.begin_one_shot(&runtime, Field::Other, "one-shot",
                            Some("project".into()), rename_resolver("Applied"));
                        probe.pending.set(helpers.is_pending(&Field::Other));
                    }
                }, "begin one-shot" }
            button { id: "settle", onclick: {
                    let helpers = helpers.clone();
                    let probe = probe.clone();
                    move |_| {
                        let results = helpers.settle(true, accepted_name);
                        *probe.results.borrow_mut() = results;
                        probe.pending.set(helpers.is_pending(&Field::Name));
                    }
                }, "settle" }
            button { id: "settle-departed", onclick: {
                    let helpers = helpers.clone();
                    let probe = probe.clone();
                    move |_| {
                        let results = helpers.settle(false, accepted_name);
                        *probe.results.borrow_mut() = results;
                    }
                }, "settle for a departed owner" }
        }
    }

    fn root_id() -> &'static str {
        "pending-edit-helper-root"
    }

    async fn mount_fixture(runtime: TestRuntime) -> Probe {
        let probe = Probe {
            runtime: Rc::new(RefCell::new(Some(runtime))),
            results: Default::default(),
            pending: Default::default(),
        };
        let window = web_sys::window().unwrap();
        let dom_document = window.document().unwrap();
        if let Some(previous) = dom_document.get_element_by_id(root_id()) {
            previous.remove();
        }
        let root = dom_document.create_element("div").unwrap();
        root.set_id(root_id());
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(helper_host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        // launch_virtual_dom renders in a background task; wait for the fixture DOM.
        rendered().await;
        rendered().await;
        probe
    }

    fn element(selector: &str) -> web_sys::Element {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{} {}", root_id(), selector))
            .unwrap()
            .expect("fixture element")
    }

    fn click(selector: &str) {
        element(selector)
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    fn draft_value() -> String {
        element("#field-draft")
            .dyn_into::<HtmlInputElement>()
            .unwrap()
            .value()
    }

    fn type_draft(value: &str) {
        let input = element("#field-draft")
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        input.set_value(value);
        let bubbling = EventInit::new();
        bubbling.set_bubbles(true);
        input
            .dispatch_event(&WebEvent::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
    }

    fn failure_message() -> Option<String> {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{} #field-failure", root_id()))
            .unwrap()
            .map(|node| node.text_content().unwrap())
    }

    fn one_shot_disabled() -> bool {
        element("#one-shot-control").has_attribute("disabled")
    }

    fn taken_results(probe: &Probe) -> Vec<PendingEditResult<Field>> {
        std::mem::take(&mut *probe.results.borrow_mut())
    }

    async fn settle(runtime: &TestRuntime) {
        for _ in 0..12 {
            support::run_pending(runtime).await;
            rendered().await;
        }
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(30).await;
    }

    #[wasm_bindgen_test]
    async fn draft_is_retained_while_pending_and_restored_at_the_accepted_value() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        click("#begin-field");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        assert!(
            probe.pending.get(),
            "the edit is pending while Core holds it"
        );
        assert_eq!(
            draft_value(),
            "Zephyr",
            "the draft stays visible while pending"
        );
        assert!(failure_message().is_none());

        release.send(()).unwrap();
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        assert_eq!(
            taken_results(&probe),
            vec![PendingEditResult::Landed {
                key: Field::Name,
                revision: 1,
            }],
            "Landed revision stays available for caller follow-ups"
        );
        assert!(!probe.pending.get());
        assert_eq!(
            draft_value(),
            "PROJECTED",
            "an untouched draft is restored to the caller-projected accepted value"
        );
        assert!(failure_message().is_none());
    }

    #[wasm_bindgen_test]
    async fn a_failed_edit_restores_the_accepted_value_and_reports_inline() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        support::fail_next_core_reply(&runtime, "engine died");
        click("#begin-failing-field");
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        assert_eq!(
            taken_results(&probe),
            vec![PendingEditResult::Failed {
                key: Field::Name,
                message: "The project change could not be applied: engine died".into(),
            }],
            "the failure stays available for caller placement"
        );
        assert_eq!(
            draft_value(),
            "PROJECTED",
            "failure restores the accepted value"
        );
        assert_eq!(
            failure_message().as_deref(),
            Some("The project change could not be applied: engine died"),
            "failure is inline by default"
        );
    }

    #[wasm_bindgen_test]
    async fn a_newer_draft_is_never_replaced_by_an_older_outcome() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        click("#begin-field");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        type_draft("Zephyr v2");
        rendered().await;
        release.send(()).unwrap();
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        assert_eq!(
            draft_value(),
            "Zephyr v2",
            "a landed outcome must not overwrite a newer draft"
        );
        assert!(failure_message().is_none());

        let failing = opened_runtime("Original").await;
        let _probe = mount_fixture(failing.clone()).await;
        support::fail_next_core_reply(&failing, "engine died");
        click("#begin-failing-field");
        settle(&failing).await;
        type_draft("Zephyr v2");
        rendered().await;
        click("#settle");
        rendered().await;
        assert_eq!(
            draft_value(),
            "Zephyr v2",
            "the newer draft survives the failure"
        );
        assert_eq!(
            failure_message().as_deref(),
            Some("The project change could not be applied: engine died"),
            "the newer draft still receives the inline failure"
        );
    }

    #[wasm_bindgen_test]
    async fn retirement_is_silent_and_restores_the_accepted_value() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        click("#begin-field");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        // A new project changes the captured Scope while the panel is still open: the
        // ticket retires itself with the owner live. The Open queues behind the held
        // reply, so release it before settling.
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: document("second-scope", "Second scope"),
        });
        release.send(()).unwrap();
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        assert_eq!(
            taken_results(&probe),
            vec![PendingEditResult::Retired { key: Field::Name }],
            "the scope change retires the edit with the owner live"
        );
        assert_eq!(draft_value(), "PROJECTED");
        assert!(
            failure_message().is_none(),
            "retirement is silent, per the ADR-0005 amendment"
        );

        // A departed owner retires the same way: no message, nothing applied.
        let (entered, _release) = support::gate_next_core_reply(&runtime);
        click("#begin-field");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        assert!(probe.pending.get());
        click("#settle-departed");
        rendered().await;
        assert_eq!(
            taken_results(&probe),
            vec![PendingEditResult::Retired { key: Field::Name }]
        );
        assert_eq!(draft_value(), "PROJECTED");
        assert!(failure_message().is_none());
    }

    #[wasm_bindgen_test]
    async fn one_shot_disables_while_pending_and_lands_for_caller_followups() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        click("#begin-one-shot");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        assert!(probe.pending.get());
        assert!(
            one_shot_disabled(),
            "the control disables while its edit is pending"
        );
        release.send(()).unwrap();
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        assert_eq!(
            taken_results(&probe),
            vec![PendingEditResult::Landed {
                key: Field::Other,
                revision: 1,
            }]
        );
        assert!(
            !one_shot_disabled(),
            "the control re-enables at the terminal"
        );
        assert!(failure_message().is_none());
    }

    #[wasm_bindgen_test]
    async fn an_unchanged_landing_still_restores_the_accepted_value() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        click("#begin-unchanged-field");
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        assert_eq!(
            taken_results(&probe),
            vec![PendingEditResult::Landed {
                key: Field::Name,
                revision: 0,
            }],
            "an unchanged edit lands at the current revision"
        );
        assert_eq!(draft_value(), "PROJECTED");
        assert!(failure_message().is_none());
    }

    #[wasm_bindgen_test]
    async fn replacing_a_key_settles_once_with_the_latest_submitted_draft() {
        let runtime = opened_runtime("Original").await;
        let probe = mount_fixture(runtime.clone()).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        click("#begin-field");
        support::drive_pending(&runtime);
        entered.await.unwrap();
        type_draft("Second");
        rendered().await;
        click("#begin-field");
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        settle(&runtime).await;
        click("#settle");
        rendered().await;
        let results = taken_results(&probe);
        assert_eq!(
            results.len(),
            1,
            "the replaced ticket no longer reports; only the latest observation drains"
        );
        assert!(matches!(
            results[0],
            PendingEditResult::Landed { revision: 2, .. }
        ));
        assert_eq!(draft_value(), "PROJECTED");
        assert!(failure_message().is_none());
    }
}
