#[path = "../../src/presentation/keymap/layer_edit.rs"]
mod layer_edit;
#[path = "../../src/presentation/keymap/panel.rs"]
mod panel;
#[path = "../../src/presentation/keymap/view.rs"]
mod view;
use boardstudio_application::{AcceptedSnapshot, Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use dioxus::core::{AttributeValue, ElementId, Mutation, Mutations};
use dioxus::html::{
    FileData, FormValue, HasFileData, HasFocusData, HasFormData, HasMouseData, HtmlEventConverter,
    PlatformEventData,
};
use dioxus::prelude::*;
use std::{
    any::Any,
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    task::{Context, Waker},
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

#[derive(Clone)]
struct Probe {
    scope: Rc<RefCell<Scope>>,
    selected: Rc<RefCell<Option<String>>>,
    calls: Rc<RefCell<Vec<String>>>,
    layer: Rc<RefCell<String>>,
    feedback: Rc<RefCell<Option<layer_edit::KeymapLayerFeedback>>>,
}
fn keymap_view(scope: &Scope) -> Rc<view::KeymapView> {
    let mut document = ProjectDoc::empty(&scope.document_id, "Fixture");
    document.boards.push(serde_json::from_value(serde_json::json!({"id":scope.board_id,"name":"Board","outlineIds":[],"partIds":["SW17","SW18"],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
    document.definitions.push(serde_json::from_value(serde_json::json!({"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
    for key in ["SW17", "SW18"] {
        document.parts.push(serde_json::from_value(serde_json::json!({"id":key,"definitionId":"switch","reference":key,"pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"})).unwrap());
    }
    document.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Main","bindings":{},"sensors":{}},{"id":"fn","name":"Fn","bindings":{},"sensors":{}}],"macros":[]})).unwrap());
    let scene: SceneDelta = serde_json::from_value(serde_json::json!({"revision":0,"transactionId":"fixture","changedIds":[],"transforms":[],"matrixScenes":[],"contours":[],"boardContours":[],"boardReadiness":[],"findings":[],"readiness":{"layout":true,"outline":true,"pcb":true,"case":false}})).unwrap();
    let snapshot = AcceptedSnapshot {
        session_epoch: scope.session_epoch,
        token: SnapshotToken(1),
        document: std::sync::Arc::new(document),
        scene: std::sync::Arc::new(scene),
    };
    view::project(&snapshot, Some(scope), &scope.board_id, "base").unwrap()
}
fn host() -> Element {
    let probe = use_context::<Probe>();
    let scope = probe.scope.borrow().clone();
    let selected_key_id = probe.selected.borrow().clone();
    let active_layer_id = probe.layer.borrow().clone();
    let layer_feedback = probe.feedback.borrow().clone();
    let selected = probe.selected.clone();
    let calls = probe.calls.clone();
    rsx! { panel::KeymapPanel {
        view:keymap_view(&scope), scope, active_layer_id, selected_key_id,
        layer_operations_enabled:true, layer_feedback,
        on_layer: move |id| *probe.layer.borrow_mut()=id,
        on_layer_operation: |_| {},
        on_select_key: move |id: String| { calls.borrow_mut().push(id.clone()); *selected.borrow_mut()=Some(id); },
        keys_editor: rsx!{ div { "Key editor" } },
        macros_editor: rsx!{ div { "Macro editor" } },
        encoders_editor: rsx!{ div { "Encoder editor" } },
    } }
}
#[derive(Default)]
struct DomState {
    input: Option<ElementId>,
    inputs: Vec<ElementId>,
    blur_inputs: std::collections::BTreeSet<ElementId>,
    select: Option<ElementId>,
    clicks: Vec<ElementId>,
    tabs: BTreeMap<String, ElementId>,
    values: BTreeMap<ElementId, String>,
}
impl DomState {
    fn apply(&mut self, mutations: Mutations) {
        for mutation in mutations.edits {
            match mutation {
                Mutation::NewEventListener { name, id } if name == "input" => self.inputs.push(id),
                Mutation::NewEventListener { name, id } if name == "blur" => {
                    self.blur_inputs.insert(id);
                }
                Mutation::NewEventListener { name, id } if name == "change" => {
                    self.select = Some(id)
                }
                Mutation::NewEventListener { name, id } if name == "click" => self.clicks.push(id),
                Mutation::CreateTextNode { value, .. }
                    if ["Keys", "Macros", "Encoders"].contains(&value.as_str()) =>
                {
                    self.tabs.insert(value, *self.clicks.last().unwrap());
                }
                Mutation::SetAttribute {
                    name: "value",
                    value: AttributeValue::Text(value),
                    id,
                    ..
                } => {
                    self.values.insert(id, value);
                }
                _ => {}
            }
        }
        self.input = self
            .inputs
            .iter()
            .rev()
            .copied()
            .find(|id| !self.blur_inputs.contains(id));
    }
    fn query(&self) -> &str {
        self.values.get(&self.input.unwrap()).unwrap()
    }
    fn selected(&self) -> &str {
        self.values.get(&self.select.unwrap()).unwrap()
    }
}
fn flush(dom: &mut VirtualDom, state: &mut DomState) {
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..6 {
        state.apply(dom.render_immediate_to_vec());
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = std::future::Future::poll(work.as_mut(), &mut Context::from_waker(Waker::noop()));
    }
}
fn mounted() -> (Probe, VirtualDom, DomState) {
    dioxus::html::set_event_converter(Box::new(TestEvents));
    let probe = Probe {
        scope: Rc::new(RefCell::new(Scope {
            session_epoch: SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        })),
        selected: Rc::default(),
        calls: Rc::default(),
        layer: Rc::new(RefCell::new("base".into())),
        feedback: Rc::default(),
    };
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    let mut state = DomState::default();
    state.apply(dom.rebuild_to_vec());
    flush(&mut dom, &mut state);
    (probe, dom, state)
}
fn event(dom: &VirtualDom, id: ElementId, name: &str, value: &str) {
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(value.to_owned())));
    dom.runtime()
        .handle_event(name, Event::new(data, false), id);
}
#[test]
fn selected_key_change_clears_the_search_from_the_previous_selection() {
    let (probe, mut dom, mut state) = mounted();
    event(&dom, state.input.unwrap(), "input", "SW17");
    flush(&mut dom, &mut state);
    assert_eq!(state.query(), "SW17");
    event(&dom, state.select.unwrap(), "change", "SW17");
    flush(&mut dom, &mut state);
    assert_eq!(
        state.query(),
        "",
        "accepted chooser selection must reset the Inspector search like React"
    );
    assert_eq!(state.selected(), "SW17");
    assert_eq!(*probe.calls.borrow(), ["SW17"]);
    event(&dom, state.input.unwrap(), "input", "SW17");
    flush(&mut dom, &mut state);
    // Canvas selection reaches this same accepted selected_key_id prop.
    *probe.selected.borrow_mut() = Some("SW18".into());
    flush(&mut dom, &mut state);
    assert_eq!(
        state.query(),
        "",
        "canvas-selected SW18 cannot remain hidden by the SW17 filter"
    );
    assert_eq!(state.selected(), "SW18");
    assert_eq!(
        probe.calls.borrow().len(),
        1,
        "reconciliation must not submit selection or edits"
    );
}
#[test]
fn same_owner_rerender_and_editor_tabs_preserve_search() {
    let (_probe, mut dom, mut state) = mounted();
    event(&dom, state.input.unwrap(), "input", "SW17");
    flush(&mut dom, &mut state);
    let keys = state.tabs["Keys"];
    let macros = state.tabs["Macros"];
    event(&dom, macros, "click", "");
    flush(&mut dom, &mut state);
    event(&dom, keys, "click", "");
    flush(&mut dom, &mut state);
    assert_eq!(
        state.query(),
        "SW17",
        "tabs alone do not reset the reference query"
    );
    flush(&mut dom, &mut state);
    assert_eq!(state.query(), "SW17");
}
#[test]
fn new_scope_with_identical_key_ids_resets_search() {
    let (probe, mut dom, mut state) = mounted();
    *probe.selected.borrow_mut() = Some("SW17".into());
    flush(&mut dom, &mut state);
    event(&dom, state.input.unwrap(), "input", "SW17");
    flush(&mut dom, &mut state);
    probe.scope.borrow_mut().session_epoch = SessionEpoch(2);
    flush(&mut dom, &mut state);
    assert_eq!(state.query(), "");
    assert_eq!(state.selected(), "SW17");
    assert!(probe.calls.borrow().is_empty());
}
#[test]
fn layer_and_operation_feedback_updates_keep_the_same_selection_search() {
    let (probe, mut dom, mut state) = mounted();
    event(&dom, state.input.unwrap(), "input", "no match");
    flush(&mut dom, &mut state);
    *probe.layer.borrow_mut() = "fn".into();
    for feedback in [
        layer_edit::KeymapLayerFeedback::Pending,
        layer_edit::KeymapLayerFeedback::Saved,
        layer_edit::KeymapLayerFeedback::Failed("retry".into()),
    ] {
        *probe.feedback.borrow_mut() = Some(feedback);
        flush(&mut dom, &mut state);
        assert_eq!(state.query(), "no match");
    }
    assert!(probe.calls.borrow().is_empty());
}
#[test]
fn canvas_selection_clears_a_filter_that_would_hide_the_current_key() {
    let (probe, mut dom, mut state) = mounted();
    *probe.selected.borrow_mut() = Some("SW17".into());
    flush(&mut dom, &mut state);
    event(&dom, state.input.unwrap(), "input", "SW17");
    flush(&mut dom, &mut state);
    *probe.selected.borrow_mut() = Some("SW18".into());
    flush(&mut dom, &mut state);
    assert_eq!(
        state.query(),
        "",
        "canvas-selected SW18 must not be excluded by the old SW17 filter"
    );
    assert_eq!(state.selected(), "SW18");
    assert!(probe.calls.borrow().is_empty());
}
#[test]
fn selection_change_returns_from_macros_to_keys_like_the_reference_owner() {
    let (probe, mut dom, mut state) = mounted();
    let macros = state.tabs["Macros"];
    event(&dom, macros, "click", "");
    flush(&mut dom, &mut state);
    let inputs_before = state.inputs.len();
    *probe.selected.borrow_mut() = Some("SW18".into());
    flush(&mut dom, &mut state);
    assert!(
        state.inputs.len() > inputs_before,
        "new selection must show Keys again, mounting its search field"
    );
    assert_eq!(state.query(), "");
    assert_eq!(state.selected(), "SW18");
}
