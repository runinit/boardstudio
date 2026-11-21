//! Mounted tests drive the helpers through real Signal lifecycle: a launched
//! VirtualDom with real inputs and buttons, the real WASM Runtime over Session and
//! Core, and its in-process gates. Native `cargo test` executes only the pure
//! policy tests above; every behavior below runs in the browser.

use super::*;
use boardstudio_application::{Event, Resolution};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event as WebEvent, EventInit, HtmlInputElement};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

type TestRuntime = Rc<boardstudio_web_runtime::runtime::Runtime>;
use boardstudio_web_test_runtime::project_name_test_support as support;

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

struct MountedProbe<T> {
    probe: T,
    _mounted: crate::test_mount::MountedFixture,
}

impl<T> std::ops::Deref for MountedProbe<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.probe
    }
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
    ProjectDoc::empty(id, name)
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
    let helpers = use_hook(PendingEditSignals::<Field>::new);
    let mut draft = use_signal(|| "Zephyr".to_string());
    let failure = use_signal(|| None::<String>);
    let disabled = use_signal(|| false);
    let name_disabled = use_signal(|| false);
    use_hook({
        let helpers = helpers.clone();
        move || {
            helpers.bind_field(Field::Name, draft, failure);
            helpers.bind_one_shot(Field::Name, name_disabled);
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
        button { id: "composite-control", disabled: name_disabled(),
            "composite control" }
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
        RenderProbe {}
    }
}

#[component]
fn RenderProbe() -> Element {
    let mut generation = use_signal(|| 0u64);
    rsx! {
        button { id: "render-probe", onclick: move |_| generation += 1, "render" }
        output { id: "render-generation", "{generation()}" }
    }
}

fn root_id() -> &'static str {
    "pending-edit-helper-root"
}

async fn mount_fixture(runtime: TestRuntime) -> MountedProbe<Probe> {
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
    let mounted = crate::test_mount::MountedFixture::new(root, dom);
    // MountedFixture renders in a background task; wait for the fixture DOM.
    rendered().await;
    rendered().await;
    MountedProbe {
        probe,
        _mounted: mounted,
    }
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

async fn rendered() {
    let document = web_sys::window().unwrap().document().unwrap();
    crate::test_wait::wait_until("pending-edit helper host renders", || {
        document
            .query_selector(&format!("#{} #render-generation", root_id()))
            .unwrap()
            .is_some()
    })
    .await;
    let label = element("#render-generation");
    let expected = label
        .text_content()
        .unwrap()
        .parse::<u64>()
        .expect("render generation is numeric")
        + 1;
    element("#render-probe")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    crate::test_wait::wait_until("pending-edit helper render is committed", || {
        label.text_content().as_deref() == Some(expected.to_string().as_str())
    })
    .await;
}

#[wasm_bindgen_test]
async fn draft_is_retained_while_pending_and_restored_at_the_accepted_value() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-field");
    rendered().await;
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
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the rename edit reaches a terminal outcome",
        || outcome.borrow().is_some(),
    )
    .await;
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
async fn field_action_disables_and_releases_its_bound_composite_control() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-field");
    rendered().await;
    assert!(
        element("#composite-control").has_attribute("disabled"),
        "begin_field disables the one-shot bound to the same logical key"
    );
    support::drive_pending(&runtime);
    entered.await.expect("the field action reached Core");
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the bound field action lands",
        || outcome.borrow().is_some(),
    )
    .await;
    click("#settle");
    rendered().await;
    assert!(
        !element("#composite-control").has_attribute("disabled"),
        "terminal settlement re-enables the composite control"
    );
    assert_eq!(draft_value(), "PROJECTED");
    assert!(probe.results.borrow().iter().any(|result| matches!(
        result,
        PendingEditResult::Landed {
            key: Field::Name,
            ..
        }
    )));
}

#[wasm_bindgen_test]
async fn a_failed_edit_restores_the_accepted_value_and_reports_inline() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_fixture(runtime.clone()).await;
    support::fail_next_core_reply(&runtime, "engine died");
    let outcome = support::observe_next(&runtime);
    click("#begin-failing-field");
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the failing field action reports its outcome",
        || outcome.borrow().is_some(),
    )
    .await;
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
    let _probe = mount_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-field");
    support::drive_pending(&runtime);
    entered.await.unwrap();
    type_draft("Zephyr v2");
    rendered().await;
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the older rename reaches its terminal outcome",
        || outcome.borrow().is_some(),
    )
    .await;
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
    let outcome = support::observe_next(&failing);
    click("#begin-failing-field");
    crate::test_wait::settle_until(
        &failing,
        support::run_pending,
        "the failing rename reaches its terminal outcome",
        || outcome.borrow().is_some(),
    )
    .await;
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
    let outcome = support::observe_next(&runtime);
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
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the retired scope edit reaches Core",
        || outcome.borrow().is_some(),
    )
    .await;
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
    let outcome = support::observe_next(&runtime);
    click("#begin-one-shot");
    support::drive_pending(&runtime);
    entered.await.unwrap();
    assert!(probe.pending.get());
    assert!(
        one_shot_disabled(),
        "the control disables while its edit is pending"
    );
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the one-shot edit reaches a terminal outcome",
        || outcome.borrow().is_some(),
    )
    .await;
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
    let outcome = support::observe_next(&runtime);
    click("#begin-unchanged-field");
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the unchanged edit is reported as landed",
        || outcome.borrow().is_some(),
    )
    .await;
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
    let first_outcome = support::observe_next(&runtime);
    click("#begin-field");
    support::drive_pending(&runtime);
    entered.await.unwrap();
    type_draft("Second");
    rendered().await;
    let second_outcome = support::observe_next(&runtime);
    click("#begin-field");
    support::drive_pending(&runtime);
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "both rename operations reach terminal outcomes",
        || first_outcome.borrow().is_some() && second_outcome.borrow().is_some(),
    )
    .await;
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

#[derive(Clone, PartialEq)]
struct RetainedDraft {
    text: String,
    marker: Option<Rc<()>>,
}

#[derive(Clone)]
struct RetentionProbe {
    runtime: TestRuntime,
    initial: Rc<RefCell<Option<RetainedDraft>>>,
    results: Rc<RefCell<Vec<PendingEditResult<Field>>>>,
}

fn retention_host() -> Element {
    let probe = use_context::<RetentionProbe>();
    let helpers = use_hook(PendingEditSignals::<Field, RetainedDraft>::new);
    let draft = use_signal(|| probe.initial.borrow_mut().take().expect("initial draft"));
    let failure = use_signal(|| None::<String>);
    let disabled = use_signal(|| false);
    use_hook({
        let helpers = helpers.clone();
        move || {
            helpers.bind_field(Field::Name, draft, failure);
            helpers.bind_one_shot(Field::Name, disabled);
        }
    });
    use_drop({
        let helpers = helpers.clone();
        move || {
            helpers.unbind_field(&Field::Name);
            helpers.unbind_one_shot(&Field::Name);
        }
    });
    rsx! {
        input { id: "retention-draft", value: "{draft().text}" }
        if let Some(message) = failure() {
            p { id: "retention-failure", "{message}" }
        }
        button { id: "begin-retention", disabled: disabled(), onclick: {
                let helpers = helpers.clone();
                let runtime = probe.runtime.clone();
                move |_| helpers.begin_value(&runtime, Field::Name, "retained-draft",
                    Some("name".into()), rename_resolver("Retained"), &draft.peek().clone())
            }, "begin retained draft" }
        button { id: "settle-retention", onclick: {
                let helpers = helpers.clone();
                let results = probe.results.clone();
                move |_| {
                    *results.borrow_mut() = helpers.settle(true, |_| RetainedDraft {
                        text: "PROJECTED".into(),
                        marker: None,
                    });
                }
            }, "settle retained draft" }
        RenderProbe {}
    }
}

async fn mount_retention_fixture(
    runtime: TestRuntime,
    initial: RetainedDraft,
) -> MountedProbe<RetentionProbe> {
    let probe = RetentionProbe {
        runtime,
        initial: Rc::new(RefCell::new(Some(initial))),
        results: Default::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    if let Some(previous) = document.get_element_by_id(root_id()) {
        previous.remove();
    }
    let root = document.create_element("div").unwrap();
    root.set_id(root_id());
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(retention_host);
    dom.provide_root_context(probe.clone());
    let mounted = crate::test_mount::MountedFixture::new(root, dom);
    rendered().await;
    rendered().await;
    MountedProbe {
        probe,
        _mounted: mounted,
    }
}

#[wasm_bindgen_test]
async fn terminal_settlement_releases_submitted_typed_draft() {
    let runtime = opened_runtime("Original").await;
    let marker = Rc::new(());
    let retained = Rc::downgrade(&marker);
    let probe = mount_retention_fixture(
        runtime.clone(),
        RetainedDraft {
            text: "Submitted".into(),
            marker: Some(marker),
        },
    )
    .await;
    assert!(
        probe.initial.borrow().is_none(),
        "the caller drops its initial draft"
    );
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-retention");
    rendered().await;
    support::drive_pending(&runtime);
    entered.await.expect("the real name edit reached Core");
    assert!(element("#begin-retention").has_attribute("disabled"));
    assert!(
        retained.upgrade().is_some(),
        "the pending draft remains live"
    );
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the retained draft edit reaches a terminal",
        || outcome.borrow().is_some(),
    )
    .await;
    click("#settle-retention");
    rendered().await;
    assert!(matches!(
        probe.results.borrow().as_slice(),
        [PendingEditResult::Landed {
            key: Field::Name,
            revision: 1
        }]
    ));
    assert_eq!(
        element("#retention-draft")
            .dyn_into::<HtmlInputElement>()
            .unwrap()
            .value(),
        "PROJECTED"
    );
    assert!(!element("#begin-retention").has_attribute("disabled"));
    assert!(
        retained.upgrade().is_none(),
        "a drained terminal releases the submitted draft while the helper stays mounted"
    );
}

// ---- Unbinding: a component that owns bound Signals leaves while its edit runs.

#[derive(Clone, Default)]
struct UnbindProbe {
    runtime: Rc<RefCell<Option<TestRuntime>>>,
    helpers: Rc<RefCell<Option<PendingEditSignals<Field>>>>,
    results: Rc<RefCell<Vec<PendingEditResult<Field>>>>,
}

#[component]
fn FieldChild(initial: String, start_disabled: bool) -> Element {
    let probe = use_context::<UnbindProbe>();
    let helpers = probe
        .helpers
        .borrow()
        .clone()
        .expect("helpers are prepared");
    let draft = use_signal(|| initial.clone());
    let failure = use_signal(|| None::<String>);
    let composite = use_signal(|| false);
    let other = use_signal(|| start_disabled);
    use_hook({
        let helpers = helpers.clone();
        move || {
            helpers.bind_field(Field::Name, draft, failure);
            helpers.bind_one_shot(Field::Name, composite);
            helpers.bind_one_shot(Field::Other, other);
        }
    });
    use_drop({
        let helpers = helpers.clone();
        move || {
            // A composite field/one-shot component releases both bindings.
            helpers.unbind_field(&Field::Name);
            helpers.unbind_one_shot(&Field::Name);
            helpers.unbind_one_shot(&Field::Other);
            // Unbinding is idempotent.
            helpers.unbind_field(&Field::Name);
            helpers.unbind_one_shot(&Field::Name);
            helpers.unbind_one_shot(&Field::Other);
        }
    });
    rsx! {
        input { id: "child-draft", value: "{draft}" }
        if let Some(message) = failure() {
            p { id: "child-failure", "{message}" }
        }
        button { id: "child-composite", disabled: composite(), "composite" }
        button { id: "child-other", disabled: other(), "other" }
    }
}

fn retire_resolver(message: &'static str) -> EditResolver {
    EditResolver::new("helper-test", move |_accepted| {
        Resolution::Retire(message.into())
    })
}

fn unbind_host() -> Element {
    let probe = use_context::<UnbindProbe>();
    let runtime = use_hook(|| {
        probe
            .runtime
            .borrow()
            .clone()
            .expect("the runtime is prepared before mounting")
    });
    let helpers = probe
        .helpers
        .borrow()
        .clone()
        .expect("helpers are prepared");
    let mut shown = use_signal(|| true);
    let mut mounts = use_signal(|| 0u32);
    rsx! {
        if shown() {
            FieldChild { key: "{mounts}", initial: if mounts() == 0 { "Zephyr" } else { "Fresh" }, start_disabled: mounts() > 0 }
        }
        button { id: "unmount", onclick: move |_| shown.set(false), "unmount" }
        button { id: "remount", onclick: move |_| { mounts += 1; shown.set(true); }, "remount" }
        button { id: "begin-rename", onclick: {
                let helpers = helpers.clone();
                let runtime = runtime.clone();
                move |_| helpers.begin_field(&runtime, Field::Name, "name-field",
                    Some("project".into()), rename_resolver("Renamed"), "Zephyr")
            }, "begin rename" }
        button { id: "begin-rejected", onclick: {
                let helpers = helpers.clone();
                let runtime = runtime.clone();
                move |_| helpers.begin_field(&runtime, Field::Name, "name-field",
                    Some("project".into()), retire_resolver("old failure"), "Zephyr")
            }, "begin rejected" }
        button { id: "begin-other", onclick: {
                let helpers = helpers.clone();
                let runtime = runtime.clone();
                move |_| helpers.begin_one_shot(&runtime, Field::Other, "one-shot",
                    Some("project".into()), rename_resolver("Applied"))
            }, "begin other" }
        button { id: "settle-unbind", onclick: {
                let helpers = helpers.clone();
                let probe = probe.clone();
                move |_| {
                    let results = helpers.settle(true, accepted_name);
                    probe.results.borrow_mut().extend(results);
                }
            }, "settle" }
        RenderProbe {}
    }
}

async fn mount_unbind_fixture(runtime: TestRuntime) -> MountedProbe<UnbindProbe> {
    let probe = UnbindProbe {
        runtime: Rc::new(RefCell::new(Some(runtime))),
        helpers: Rc::new(RefCell::new(Some(PendingEditSignals::new()))),
        results: Default::default(),
    };
    let window = web_sys::window().unwrap();
    let dom_document = window.document().unwrap();
    if let Some(previous) = dom_document.get_element_by_id(root_id()) {
        previous.remove();
    }
    let root = dom_document.create_element("div").unwrap();
    root.set_id(root_id());
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(unbind_host);
    dom.provide_root_context(probe.clone());
    let mounted = crate::test_mount::MountedFixture::new(root, dom);
    rendered().await;
    rendered().await;
    MountedProbe {
        probe,
        _mounted: mounted,
    }
}

fn exists(selector: &str) -> bool {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} {}", root_id(), selector))
        .unwrap()
        .is_some()
}

fn child_draft() -> String {
    element("#child-draft")
        .dyn_into::<HtmlInputElement>()
        .unwrap()
        .value()
}

#[wasm_bindgen_test]
async fn an_unmounted_field_never_has_its_dropped_signals_written_and_its_edit_still_runs() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_unbind_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-rename");
    rendered().await;
    support::drive_pending(&runtime);
    entered.await.expect("the field action reached Core");
    click("#unmount");
    rendered().await;
    assert!(!exists("#child-draft"), "the field component is gone");
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the unmounted field edit reaches Core",
        || outcome.borrow().is_some(),
    )
    .await;
    click("#settle-unbind");
    rendered().await;
    assert!(
        probe.results.borrow().iter().any(|result| matches!(
            result,
            PendingEditResult::Landed {
                key: Field::Name,
                ..
            }
        )),
        "the outcome still reaches the caller"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Renamed",
        "the queued Session edit kept executing"
    );
}

#[wasm_bindgen_test]
async fn a_remounted_field_with_the_same_key_never_receives_the_old_outcome() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_unbind_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let first_outcome = support::observe_next(&runtime);
    click("#begin-rename");
    rendered().await;
    support::drive_pending(&runtime);
    entered.await.expect("the first edit reached Core");
    // The newer edit for the same key is the observed one; it is rejected once it runs.
    let rejected_outcome = support::observe_next(&runtime);
    click("#begin-rejected");
    rendered().await;
    click("#unmount");
    rendered().await;
    click("#remount");
    rendered().await;
    assert_eq!(child_draft(), "Fresh");
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "both field outcomes reach the caller",
        || first_outcome.borrow().is_some() && rejected_outcome.borrow().is_some(),
    )
    .await;
    click("#settle-unbind");
    rendered().await;
    assert_eq!(
        child_draft(),
        "Fresh",
        "the old submitted text never restores into the new component"
    );
    assert!(
        !exists("#child-failure"),
        "the old failure is not shown by the new component"
    );
    assert!(
        probe.results.borrow().iter().any(|result| matches!(
            result,
            PendingEditResult::Failed { message, .. } if message.contains("old failure")
        )),
        "the caller still receives the old outcome for its own follow-up"
    );
}

#[wasm_bindgen_test]
async fn an_unmounted_one_shot_is_not_released_through_a_dropped_signal() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_unbind_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-other");
    rendered().await;
    assert!(element("#child-other").has_attribute("disabled"));
    support::drive_pending(&runtime);
    entered.await.expect("the one-shot reached Core");
    click("#unmount");
    rendered().await;
    assert!(
        !exists("#child-other"),
        "the control is gone and not remounted"
    );
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the unmounted one-shot reaches the caller",
        || outcome.borrow().is_some(),
    )
    .await;
    // Releasing a control through its dropped Signal would panic here.
    click("#settle-unbind");
    rendered().await;
    assert!(
        probe.results.borrow().iter().any(|result| matches!(
            result,
            PendingEditResult::Landed {
                key: Field::Other,
                ..
            }
        )),
        "the outcome still reaches the caller"
    );
    assert_eq!(
        runtime.model().accepted.unwrap().document.name,
        "Applied",
        "the queued Session edit kept executing"
    );
}

#[wasm_bindgen_test]
async fn an_old_one_shot_outcome_never_re_enables_a_replacement_that_is_disabled() {
    let runtime = opened_runtime("Original").await;
    let probe = mount_unbind_fixture(runtime.clone()).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let outcome = support::observe_next(&runtime);
    click("#begin-other");
    rendered().await;
    support::drive_pending(&runtime);
    entered.await.expect("the old one-shot reached Core");
    click("#unmount");
    rendered().await;
    click("#remount");
    rendered().await;
    assert!(
        element("#child-other").has_attribute("disabled"),
        "the replacement is disabled for reasons of its own"
    );
    release.send(()).unwrap();
    crate::test_wait::settle_until(
        &runtime,
        support::run_pending,
        "the old one-shot reaches the caller",
        || outcome.borrow().is_some(),
    )
    .await;
    click("#settle-unbind");
    rendered().await;
    assert!(
        probe.results.borrow().iter().any(|result| matches!(
            result,
            PendingEditResult::Landed {
                key: Field::Other,
                ..
            }
        )),
        "the old outcome still reaches the caller for its follow-up"
    );
    assert!(
        element("#child-other").has_attribute("disabled"),
        "the old outcome never re-enables the replacement"
    );
}
