#[path = "../../src/presentation/objects/matrix_inspector.rs"]
mod matrix_inspector;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::DiodeDirection;
use dioxus::core::{AttributeValue, ElementId, Mutation, Mutations};
use dioxus::html::{
    FileData, FormValue, HasFileData, HasFocusData, HasFormData, HtmlEventConverter,
    PlatformEventData,
};
use dioxus::prelude::*;
use matrix_inspector::*;
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
struct TestEvents;
macro_rules! convert_event {
    (convert_form_data, $data:ident) => {
        fn convert_form_data(&self, event: &PlatformEventData) -> $data {
            FormData::new(TestForm(event.downcast::<String>().unwrap().clone()))
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
    projection: Rc<RefCell<MatrixInspectorProjection>>,
    requests: Rc<RefCell<Vec<MatrixEditRequest>>>,
    feedback: Rc<RefCell<Vec<MatrixEditFeedback>>>,
}
fn projection(matrix: &str) -> MatrixInspectorProjection {
    MatrixInspectorProjection {
        owner: MatrixInspectorOwner {
            editor_instance_id: 1,
            context_generation: 1,
            scope_generation: 1,
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: "project".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            matrix_id: matrix.into(),
            name_target: MatrixNameTarget::Matrix,
        },
        snapshot_token: SnapshotToken(1),
        revision: 1,
        matrix_label: matrix.into(),
        name_label: "Matrix name",
        name_value: "Same".into(),
        name_baseline: MatrixEditValue::Name(Some("Same".into())),
        rows: 3,
        columns: 4,
        pitch_x: 19.05,
        pitch_y: 19.05,
        definition_id: "switch-definition".into(),
        switch_choices: Vec::new(),
        diode_direction: DiodeDirection::Row2col,
        edge_gap_x: 1.0,
        edge_gap_y: 1.0,
        preset: None,
        orientation: None,
        baseline_variant: None,
        layout_relation: None,
    }
}
fn host() -> Element {
    let probe = use_context::<Probe>();
    let request_sequence = use_signal(|| 0);
    let projection = probe.projection.borrow().clone();
    let feedback = probe.feedback.borrow().clone();
    rsx! { MatrixInspector {
        projection,
        request_sequence,
        editable: true,
        busy: false,
        feedback,
        on_edit: move |request| probe.requests.borrow_mut().push(request),
        on_apply_preset: move |_| {},
        on_delete: move |_| {},
        on_unlink: move |_| {},
        on_duplicate: move |_| {},
    } }
}
#[derive(Default)]
struct DomState {
    inputs: Vec<ElementId>,
    values: BTreeMap<ElementId, String>,
    labels: BTreeMap<ElementId, String>,
}
impl DomState {
    fn input(&self, label: &str) -> ElementId {
        *self
            .inputs
            .iter()
            .rev()
            .find(|id| self.labels.get(id).is_some_and(|value| value == label))
            .unwrap()
    }
    fn apply(&mut self, mutations: Mutations) {
        for mutation in mutations.edits {
            match mutation {
                Mutation::NewEventListener { name, id } if name == "input" => self.inputs.push(id),
                Mutation::SetAttribute {
                    name: "aria-label",
                    value: AttributeValue::Text(value),
                    id,
                    ..
                } => {
                    self.labels.insert(id, value);
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
    }
}
fn flush(dom: &mut VirtualDom, state: &mut DomState) {
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..4 {
        state.apply(dom.render_immediate_to_vec());
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = std::future::Future::poll(work.as_mut(), &mut Context::from_waker(Waker::noop()));
    }
}
fn mounted() -> (Probe, VirtualDom, DomState) {
    dioxus::html::set_event_converter(Box::new(TestEvents));
    let probe = Probe {
        projection: Rc::new(RefCell::new(projection("A"))),
        requests: Rc::default(),
        feedback: Rc::default(),
    };
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    let mut state = DomState::default();
    state.apply(dom.rebuild_to_vec());
    flush(&mut dom, &mut state);
    assert_eq!(state.inputs.len(), 7);
    (probe, dom, state)
}
fn event(dom: &VirtualDom, id: ElementId, name: &str, value: &str) {
    let data: Rc<dyn Any> = Rc::new(PlatformEventData::new(Box::new(value.to_owned())));
    dom.runtime()
        .handle_event(name, Event::new(data, false), id);
}

#[test]
fn dirty_fields_do_not_follow_an_equal_baseline_to_another_matrix() {
    for (label, draft) in [
        ("Matrix name", "Draft A"),
        ("Rows", "8"),
        ("Columns", "9"),
        ("Pitch X", "21.5"),
        ("Pitch Y", "22.5"),
    ] {
        let (probe, mut dom, mut state) = mounted();
        let old = state.input(label);
        event(&dom, old, "input", draft);
        flush(&mut dom, &mut state);
        assert_eq!(state.values.get(&old).map(String::as_str), Some(draft));
        *probe.projection.borrow_mut() = projection("B");
        flush(&mut dom, &mut state);
        let current = state.input(label);
        event(&dom, current, "blur", "");
        flush(&mut dom, &mut state);
        assert!(
            probe.requests.borrow().is_empty(),
            "A's dirty {label} must not submit to B: {:?}",
            probe.requests.borrow()
        );
    }
}
#[test]
fn unrelated_revision_keeps_the_same_owners_uncommitted_draft() {
    let (probe, mut dom, mut state) = mounted();
    let input = state.input("Matrix name");
    event(&dom, input, "input", "Draft A");
    flush(&mut dom, &mut state);
    {
        let mut next = probe.projection.borrow_mut();
        next.snapshot_token = SnapshotToken(2);
        next.revision = 2;
    }
    flush(&mut dom, &mut state);
    assert_eq!(state.inputs.len(), 7, "same owner must not remount");
    assert_eq!(
        state.values.get(&input).map(String::as_str),
        Some("Draft A")
    );
    event(&dom, input, "blur", "");
    flush(&mut dom, &mut state);
    let requests = probe.requests.borrow();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].owner.matrix_id, "A");
    assert_eq!(
        requests[0].value,
        MatrixEditValue::Name(Some("Draft A".into()))
    );
    assert_eq!(requests[0].snapshot_token, SnapshotToken(2));
}
#[test]
fn layout_name_target_switch_resets_the_name_draft() {
    let (probe, mut dom, mut state) = mounted();
    let input = state.input("Matrix name");
    event(&dom, input, "input", "Old layout draft");
    flush(&mut dom, &mut state);
    probe.projection.borrow_mut().owner.name_target = MatrixNameTarget::Layout {
        id: "layout".into(),
    };
    flush(&mut dom, &mut state);
    event(&dom, state.input("Matrix name"), "blur", "");
    flush(&mut dom, &mut state);
    assert!(probe.requests.borrow().is_empty());
}
#[test]
fn same_owner_pending_and_saved_feedback_settle_the_actual_field() {
    let (probe, mut dom, mut state) = mounted();
    let input = state.input("Rows");
    event(&dom, input, "input", "8");
    flush(&mut dom, &mut state);
    event(&dom, input, "blur", "");
    flush(&mut dom, &mut state);
    let request = probe.requests.borrow()[0].clone();
    *probe.feedback.borrow_mut() = vec![MatrixEditFeedback {
        owner: request.owner.clone(),
        request_id: request.request_id,
        field: request.field,
        state: MatrixEditState::Pending,
        message: None,
    }];
    flush(&mut dom, &mut state);
    probe.projection.borrow_mut().rows = 8;
    probe.feedback.borrow_mut()[0].state = MatrixEditState::Saved;
    flush(&mut dom, &mut state);
    assert_eq!(
        state.values.get(&state.input("Rows")).map(String::as_str),
        Some("8")
    );
    event(&dom, state.input("Rows"), "blur", "");
    flush(&mut dom, &mut state);
    assert_eq!(probe.requests.borrow().len(), 1);
}
