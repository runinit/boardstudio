#![cfg(all(feature = "page", not(target_arch = "wasm32")))]

#[path = "../src/operation_outcomes.rs"]
mod operation_outcomes;
#[path = "../src/presentation/parts/mechanical_profile.rs"]
mod parts_mechanical_profile;
#[path = "../src/presentation/parts/standard_profile_lifetime.rs"]
mod production_lifetime;
pub(crate) use production_lifetime::PartsStandardProfileLifetime;
mod editor_component {
    include!("../src/presentation/parts/mechanical_profile_editor.rs");
}

use boardstudio_application::{AcceptedSnapshot, OperationId, Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{
    MechanicalPartProfile, MechanicalSwitchFamily, PartDefinition, ProjectDoc, SceneDelta,
};
use dioxus::core::{AttributeValue, ElementId, Event, Mutation, Mutations};
use dioxus::html::{
    FileData, FocusData, FormData, FormValue, HasFileData, HasFocusData, HasFormData, HasMouseData,
    HtmlEventConverter, InteractionElementOffset, InteractionLocation, ModifiersInteraction,
    MouseData, PlatformEventData, PointerInteraction,
};
use dioxus::prelude::*;
use editor_component::{ManualProfileEditor, ManualProfileEditorPorts};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

struct TestForm(String);
impl HasFileData for TestForm {
    fn files(&self) -> Vec<FileData> {
        Vec::new()
    }
}
impl HasFormData for TestForm {
    fn value(&self) -> String {
        self.0.clone()
    }
    fn valid(&self) -> bool {
        true
    }
    fn values(&self) -> Vec<(String, FormValue)> {
        Vec::new()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
struct TestFocus;
impl HasFocusData for TestFocus {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
struct TestMouse;
impl HasMouseData for TestMouse {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl InteractionLocation for TestMouse {
    fn client_coordinates(&self) -> dioxus::html::geometry::ClientPoint {
        Default::default()
    }
    fn page_coordinates(&self) -> dioxus::html::geometry::PagePoint {
        Default::default()
    }
    fn screen_coordinates(&self) -> dioxus::html::geometry::ScreenPoint {
        Default::default()
    }
}
impl InteractionElementOffset for TestMouse {
    fn element_coordinates(&self) -> dioxus::html::geometry::ElementPoint {
        Default::default()
    }
}
impl ModifiersInteraction for TestMouse {
    fn modifiers(&self) -> dioxus::html::Modifiers {
        Default::default()
    }
}
impl PointerInteraction for TestMouse {
    fn held_buttons(&self) -> dioxus::html::input_data::MouseButtonSet {
        Default::default()
    }
    fn trigger_button(&self) -> Option<dioxus::html::input_data::MouseButton> {
        None
    }
}
struct TestEvents;
macro_rules! convert_event {
    (convert_form_data, $data:ident) => {
        fn convert_form_data(&self, event: &PlatformEventData) -> $data {
            FormData::new(TestForm(event.downcast::<String>().unwrap().clone()))
        }
    };
    (convert_mouse_data, $data:ident) => {
        fn convert_mouse_data(&self, _: &PlatformEventData) -> $data {
            MouseData::new(TestMouse)
        }
    };
    (convert_focus_data, $data:ident) => {
        fn convert_focus_data(&self, _: &PlatformEventData) -> $data {
            FocusData::new(TestFocus)
        }
    };
    ($method:ident, $data:ident) => {
        fn $method(&self, _: &PlatformEventData) -> dioxus::html::$data {
            panic!("unexpected event")
        }
    };
}
macro_rules! implement_events {
    (enum Event { $( #[convert = $method:ident] #[events = [ $( $(#[$attr:meta])* $name:ident => $raw:ident, )* ]] $(#[raw = [$($raw_only:ident),* $(,)?]])? $group:ident($data:ident), )* }) => {
        impl HtmlEventConverter for TestEvents { $(convert_event!($method, $data);)* }
    };
}
dioxus::html::with_html_event_groups!(implement_events);

type Detached = Pin<Box<dyn Future<Output = ()>>>;
thread_local! { static DETACHED: RefCell<Vec<Detached>> = RefCell::default(); }
fn poll_detached() {
    DETACHED.with_borrow_mut(|tasks| {
        tasks.retain_mut(|task| {
            task.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        })
    });
}

#[derive(Default)]
struct Gate {
    result: RefCell<Option<Result<MechanicalPartProfile, String>>>,
    waiter: RefCell<Option<Waker>>,
}
impl Gate {
    async fn wait(self: Rc<Self>) -> Result<MechanicalPartProfile, String> {
        std::future::poll_fn(|cx| {
            if let Some(result) = self.result.borrow_mut().take() {
                Poll::Ready(result)
            } else {
                *self.waiter.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }
    fn resolve(&self, result: Result<MechanicalPartProfile, String>) {
        *self.result.borrow_mut() = Some(result);
        if let Some(waker) = self.waiter.borrow_mut().take() {
            waker.wake();
        }
    }
}
#[derive(Clone)]
struct Request {
    definition_id: String,
    family: MechanicalSwitchFamily,
    gap: f64,
    gate: Rc<Gate>,
}
#[derive(Default)]
struct Probe {
    requests: RefCell<Vec<Request>>,
    saved: RefCell<Vec<MechanicalPartProfile>>,
    next_operation: Cell<u64>,
    selection: Cell<Option<Signal<Option<(Option<Scope>, String)>>>>,
    selection_generation: Cell<Option<Signal<u64>>>,
    current_scope: RefCell<Option<Scope>>,
    accepted_owner: Cell<bool>,
}
struct Fixture {
    probe: Rc<Probe>,
    definition: PartDefinition,
    profile: MechanicalPartProfile,
    snapshot: AcceptedSnapshot,
    scope: Scope,
}

#[component]
fn host() -> Element {
    let fixture = use_context::<Rc<Fixture>>();
    let probe = fixture.probe.clone();
    let selection =
        use_signal(|| Some((Some(fixture.scope.clone()), fixture.definition.id.clone())));
    let selection_generation = use_signal(|| 4u64);
    let scope_generation = use_signal(|| 8u64);
    let workspace = use_signal(|| "Parts");
    probe.selection.set(Some(selection));
    probe.selection_generation.set(Some(selection_generation));
    let owner = parts_mechanical_profile::ProfileEditOwner::new(
        OperationId(40),
        &fixture.snapshot,
        Some(fixture.scope.clone()),
        parts_mechanical_profile::ProfileDefinitionSource::Project,
        &fixture.definition,
    );
    let request_probe = probe.clone();
    let request_standard_profile: editor_component::StandardProfileRequester =
        Rc::new(move |definition_id, family, gap| {
            let gate = Rc::new(Gate::default());
            request_probe.requests.borrow_mut().push(Request {
                definition_id,
                family,
                gap,
                gate: gate.clone(),
            });
            let operation = request_probe.next_operation.get() + 1;
            request_probe.next_operation.set(operation);
            let future: editor_component::StandardProfileFuture = Box::pin(gate.wait());
            (OperationId(operation), future)
        });
    let spawn_detached: editor_component::DetachedProfileSpawner =
        Rc::new(|future| DETACHED.with_borrow_mut(|tasks| tasks.push(future)));
    let current_scope_probe = probe.clone();
    let current_scope: editor_component::CurrentProfileScope =
        Rc::new(move || current_scope_probe.current_scope.borrow().clone());
    let accepted_probe = probe.clone();
    let accepted_owner_is_current: editor_component::AcceptedProfileOwner =
        Rc::new(move |owner: &parts_mechanical_profile::ProfileEditOwner| {
            accepted_probe.accepted_owner.get()
                && accepted_probe.current_scope.borrow().as_ref() == owner.scope.as_ref()
        });
    let ports = ManualProfileEditorPorts {
        request_standard_profile,
        spawn_detached,
        current_scope,
        accepted_owner_is_current,
    };
    rsx! { ManualProfileEditor {
        definition: fixture.definition.clone(),
        initial: Some(fixture.profile.clone()),
        owner,
        snapshot: fixture.snapshot.clone(),
        selection,
        selection_generation,
        scope_generation,
        workspace,
        on_save: move |profile| probe.saved.borrow_mut().push(profile),
        on_close: |_| {},
        ports,
    } }
}

#[derive(Default)]
struct DomState {
    labels: HashMap<ElementId, String>,
    listeners: HashMap<ElementId, String>,
    disabled: HashMap<ElementId, bool>,
    texts: Vec<String>,
}
impl DomState {
    fn element(&self, label: &str, event: &str) -> Option<ElementId> {
        self.labels.iter().find_map(|(id, value)| {
            (value == label && self.listeners.get(id).is_some_and(|name| name == event))
                .then_some(*id)
        })
    }
    fn is_disabled(&self, label: &str) -> bool {
        self.element(label, "click")
            .and_then(|id| self.disabled.get(&id).copied())
            .unwrap_or(false)
    }
    fn apply(&mut self, mutations: Mutations) {
        for mutation in mutations.edits {
            match mutation {
                Mutation::NewEventListener { name, id } => {
                    self.listeners.insert(id, name);
                }
                Mutation::SetAttribute {
                    name: "aria-label",
                    value: AttributeValue::Text(value),
                    id,
                    ..
                } => {
                    self.labels.insert(id, value);
                }
                Mutation::SetAttribute {
                    name: "disabled",
                    value,
                    id,
                    ..
                } => {
                    self.disabled
                        .insert(id, matches!(value, AttributeValue::Bool(true)));
                }
                Mutation::CreateTextNode { value, .. } | Mutation::SetText { value, .. } => {
                    self.texts.push(value)
                }
                _ => {}
            }
        }
    }
}
fn fixture(kind: &str, initial_json: serde_json::Value) -> Fixture {
    DETACHED.with_borrow_mut(Vec::clear);
    dioxus::html::set_event_converter(Box::new(TestEvents));
    let definition: PartDefinition = serde_json::from_value(serde_json::json!({
        "id":"custom-switch", "name":"Saved part", "kind":kind,
        "courtyard":[], "pads":[]
    }))
    .unwrap();
    let profile = serde_json::from_value(initial_json).unwrap();
    let mut document = ProjectDoc::empty("profile-project", "Fixture");
    document.definitions.push(definition.clone());
    let scene: SceneDelta = serde_json::from_value(serde_json::json!({
        "revision":0,"transactionId":"fixture","changedIds":[],"transforms":[],
        "matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[],
        "readiness":{"layout":true,"outline":true,"pcb":true,"case":false}
    }))
    .unwrap();
    let snapshot = AcceptedSnapshot {
        session_epoch: SessionEpoch(1),
        token: SnapshotToken(7),
        document: std::sync::Arc::new(document),
        scene: std::sync::Arc::new(scene),
    };
    let scope = Scope {
        session_epoch: SessionEpoch(1),
        document_id: "profile-project".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let probe = Rc::new(Probe {
        current_scope: RefCell::new(Some(scope.clone())),
        accepted_owner: Cell::new(true),
        ..Default::default()
    });
    Fixture {
        probe,
        definition,
        profile,
        snapshot,
        scope,
    }
}
fn mount(fixture: Fixture) -> (Rc<Probe>, VirtualDom, DomState) {
    let probe = fixture.probe.clone();
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(Rc::new(fixture));
    let mut state = DomState::default();
    state.apply(dom.rebuild_to_vec());
    flush(&mut dom, &mut state);
    (probe, dom, state)
}
fn flush(dom: &mut VirtualDom, state: &mut DomState) {
    for _ in 0..4 {
        state.apply(dom.render_immediate_to_vec());
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
    }
}
fn event(dom: &VirtualDom, state: &DomState, label: &str, name: &str, value: &str) {
    let id = state.element(label, name).unwrap_or_else(|| {
        panic!(
            "missing {name} listener for {label}; labels={:?}",
            state.labels
        )
    });
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(value.to_owned())));
    dom.runtime()
        .handle_event(name, Event::new(data, false), id);
}
fn fit(family: &str, gap: f64, source: &str) -> MechanicalPartProfile {
    serde_json::from_value(serde_json::json!({
        "definitionId":"custom-switch", "source":source, "cutouts":[[
            {"x":-7.0,"y":-7.0},{"x":7.0,"y":-7.0},{"x":7.0,"y":7.0},{"x":-7.0,"y":7.0}
        ]], "plateToPcb":gap, "switchFamily":family
    }))
    .unwrap()
}
fn base_profile(family: Option<&str>) -> serde_json::Value {
    let mut profile = serde_json::json!({
        "definitionId":"custom-switch", "source":"user draft", "cutouts":[], "plateToPcb":0.9,
        "clearances":[[{"x":-1.0,"y":-1.0},{"x":1.0,"y":-1.0},{"x":1.0,"y":1.0}]]
    });
    if let Some(family) = family {
        profile["switchFamily"] = family.into();
    }
    profile
}

#[test]
fn mounted_switch_selector_dispatches_load_preserves_latest_draft_and_settles_save() {
    let fixture = fixture("switch", base_profile(None));
    let (probe, mut dom, mut state) = mount(fixture);
    assert!(state.element("Switch fit family", "change").is_some());
    event(&dom, &state, "Switch fit family", "change", "choc-v1");
    poll_detached();
    flush(&mut dom, &mut state);
    assert_eq!(probe.requests.borrow().len(), 1);
    assert_eq!(probe.requests.borrow()[0].definition_id, "custom-switch");
    assert_eq!(
        probe.requests.borrow()[0].family,
        MechanicalSwitchFamily::ChocV1
    );
    assert_eq!(probe.requests.borrow()[0].gap, 2.2);
    assert!(
        state.is_disabled("Save fit profile"),
        "Save is disabled while lookup is pending"
    );

    event(&dom, &state, "Add clearance", "click", "");
    flush(&mut dom, &mut state);
    let request = probe.requests.borrow()[0].clone();
    request.gate.resolve(Ok(fit("choc-v1", 2.2, "bundled")));
    poll_detached();
    flush(&mut dom, &mut state);
    assert!(
        !state.is_disabled("Save fit profile"),
        "Save is enabled after successful settlement"
    );
    event(&dom, &state, "Save fit profile", "click", "");
    flush(&mut dom, &mut state);
    let saved = probe.saved.borrow();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].switch_family, Some(MechanicalSwitchFamily::ChocV1));
    assert_eq!(
        saved[0].cutouts.len(),
        1,
        "loaded standard cutout is merged"
    );
    assert_eq!(
        saved[0].clearances.as_ref().map(Vec::len),
        Some(2),
        "latest draft clearance edit survives sparse result"
    );
}

#[test]
fn mounted_non_switch_with_saved_family_gets_action_only_and_failure_reenables_save() {
    let fixture = fixture("controller", base_profile(Some("choc-v2")));
    let (probe, mut dom, mut state) = mount(fixture);
    assert!(
        state.element("Switch fit family", "change").is_none(),
        "non-switch definition has no selector"
    );
    assert!(
        state.element("Use standard cutout", "click").is_some(),
        "saved family retains source-independent action"
    );
    event(&dom, &state, "Use standard cutout", "click", "");
    poll_detached();
    flush(&mut dom, &mut state);
    assert_eq!(probe.requests.borrow().len(), 1);
    assert_eq!(
        probe.requests.borrow()[0].family,
        MechanicalSwitchFamily::ChocV2
    );
    assert!(state.is_disabled("Save fit profile"));
    probe.requests.borrow()[0]
        .gate
        .resolve(Err("controlled lookup failure".into()));
    poll_detached();
    flush(&mut dom, &mut state);
    assert!(
        !state.is_disabled("Save fit profile"),
        "Save re-enables after an error settlement"
    );
    assert!(
        state
            .texts
            .iter()
            .any(|text| text.contains("controlled lookup failure"))
    );
}

#[test]
fn mounted_stale_selection_discards_success_without_replacing_draft() {
    let fixture = fixture("switch", base_profile(None));
    let probe_for_test = fixture.probe.clone();
    let (probe, mut dom, mut state) = mount(fixture);
    event(&dom, &state, "Switch fit family", "change", "mx");
    poll_detached();
    flush(&mut dom, &mut state);
    let mut selection = probe.selection.get().unwrap();
    selection.set(Some((
        Some(probe.current_scope.borrow().clone().unwrap()),
        "other-definition".into(),
    )));
    probe.selection_generation.get().unwrap().set(5);
    flush(&mut dom, &mut state);
    probe.requests.borrow()[0]
        .gate
        .resolve(Ok(fit("mx", 3.5, "stale bundled")));
    poll_detached();
    flush(&mut dom, &mut state);
    assert!(
        !state.is_disabled("Save fit profile"),
        "stale owner completion retires pending state"
    );
    event(&dom, &state, "Save fit profile", "click", "");
    flush(&mut dom, &mut state);
    let saved = probe.saved.borrow();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].switch_family, None);
    assert!(saved[0].cutouts.is_empty());
    assert!(Rc::ptr_eq(&probe, &probe_for_test));
}

#[test]
fn mounted_stale_selection_discards_error_without_publishing_or_replacing_draft() {
    let fixture = fixture("switch", base_profile(None));
    let (probe, mut dom, mut state) = mount(fixture);
    event(&dom, &state, "Switch fit family", "change", "mx");
    poll_detached();
    flush(&mut dom, &mut state);

    let mut selection = probe.selection.get().unwrap();
    selection.set(Some((
        Some(probe.current_scope.borrow().clone().unwrap()),
        "other-definition".into(),
    )));
    probe.selection_generation.get().unwrap().set(5);
    flush(&mut dom, &mut state);

    probe.requests.borrow()[0]
        .gate
        .resolve(Err("stale lookup failure".into()));
    poll_detached();
    flush(&mut dom, &mut state);
    assert!(
        !state.is_disabled("Save fit profile"),
        "stale owner completion retires pending state"
    );
    assert!(
        !state
            .texts
            .iter()
            .any(|text| text.contains("stale lookup failure")),
        "stale failure is not published into the replacement selection"
    );
    event(&dom, &state, "Save fit profile", "click", "");
    flush(&mut dom, &mut state);
    let saved = probe.saved.borrow();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].switch_family, None);
    assert!(saved[0].cutouts.is_empty());
}
