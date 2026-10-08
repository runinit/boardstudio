use crate::runtime::Runtime;
use boardstudio_application::{Event, Scope};
use boardstudio_core::model::{EditOperation, Position, Vec2};
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

mod layout_component_inspector;
pub(super) use layout_component_inspector::{
    BoardPartChoice, ComponentPositionAxis, LayoutChoice, LayoutComponentInspector,
    LayoutComponentInspectorAction, LayoutComponentInspectorLifetime,
    LayoutComponentInspectorOwner, LayoutComponentInspectorOwnerKey,
    LayoutComponentInspectorProjection, LayoutConstraintValues,
};

#[derive(Clone)]
struct NumericEdit {
    id: String,
    transaction_id: String,
    start: Vec2,
    part_ids: Vec<String>,
    board_id: String,
    x_changed: bool,
    y_changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NumericOwner {
    scope: Option<Scope>,
    part_ids: Vec<String>,
    board_id: String,
}
type KeyboardHandler = Rc<RefCell<Box<dyn FnMut(KeyboardEvent)>>>;
#[component]
pub(super) fn Inspector() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let selected = model
        .accepted
        .as_ref()
        .and_then(|s| {
            s.document
                .parts
                .iter()
                .find(|p| model.selected_part_ids.first() == Some(&p.id))
        })
        .cloned();
    let mut draft = use_signal(|| (String::new(), String::new()));
    let mut dirty_axes = use_signal(|| (None::<String>, false, false));
    let numeric_edit = use_hook(|| Rc::new(RefCell::new(None::<NumericEdit>)));
    let pending = use_signal(PendingEditSignals::<(), (String, String)>::new);
    let apply_disabled = use_signal(|| false);
    pending.peek().bind_one_shot((), apply_disabled);
    let mut pending_owner = use_signal(|| None::<NumericOwner>);
    let mut failure = use_signal(|| None::<String>);
    pending.peek().bind_field((), draft, failure);
    use_drop({
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move || {
            let pending = numeric_edit.borrow_mut().take();
            if let Some(edit) = pending {
                let start = edit.start;
                submit_position(&runtime, &numeric_edit, edit, start);
            }
        }
    });
    use_drop({
        let pending = pending;
        move || {
            pending.peek().unbind_field(&());
            pending.peek().unbind_one_shot(&());
            pending
                .peek()
                .settle(false, |_| (String::new(), String::new()));
        }
    });
    let key = selected
        .as_ref()
        .map(|p| (p.id.clone(), p.pose.at.x, p.pose.at.y));
    let current_owner = selected.as_ref().map(|_| NumericOwner {
        scope: runtime.scope(),
        part_ids: model.selected_part_ids.clone(),
        board_id: model.active_board_id.clone(),
    });
    use_effect(use_reactive(
        (&key, &version, &current_owner, &pending_owner),
        {
            let runtime = runtime.clone();
            let numeric_edit = numeric_edit.clone();
            move |(key, _, current_owner, captured_owner)| {
                let owner_is_live = current_owner.is_some() && current_owner == captured_owner();
                pending.peek().settle(owner_is_live, |_| {
                    key.as_ref()
                        .map(|(_, px, py)| (px.to_string(), py.to_string()))
                        .unwrap_or_default()
                });
                if !pending.peek().is_pending(&()) {
                    if pending_owner.peek().is_some() {
                        pending_owner.set(None);
                    }
                }
                let (draft_id, x_dirty, y_dirty) = dirty_axes.peek().clone();
                let selected_id = key.as_ref().map(|(id, _, _)| id);
                let same_target = draft_id.as_ref() == selected_id;
                if !same_target {
                    dirty_axes.set((selected_id.cloned(), false, false));
                }
                if let Some((_, px, py)) = key.as_ref() {
                    if !pending.peek().is_pending(&()) {
                        let mut projected = draft.peek().clone();
                        if !same_target || !x_dirty {
                            projected.0 = px.to_string();
                        }
                        if !same_target || !y_dirty {
                            projected.1 = py.to_string();
                        }
                        if *draft.peek() != projected {
                            draft.set(projected);
                        }
                    }
                }
                let changed_target = numeric_edit
                    .borrow()
                    .as_ref()
                    .is_some_and(|edit| key.as_ref().is_none_or(|(id, _, _)| id != &edit.id));
                if changed_target && let Some(edit) = numeric_edit.borrow_mut().take() {
                    let start = edit.start;
                    submit_position(&runtime, &Rc::new(RefCell::new(None)), edit, start);
                }
            }
        },
    ));
    let submit = {
        let pending = pending;
        let mut pending_owner = pending_owner;
        let mut dirty_axes = dirty_axes;
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |_| {
            failure.set(None);
            commit_numeric(
                &runtime,
                &numeric_edit,
                &pending,
                &mut pending_owner,
                &mut dirty_axes,
                &draft(),
            );
        }
    };
    let cancel_numeric: KeyboardHandler = {
        let mut pending_owner = pending_owner;
        let mut dirty_axes = dirty_axes;
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        let mut draft = draft;
        Rc::new(RefCell::new(Box::new(move |event: KeyboardEvent| {
            let key = event.data().key().to_string();
            if key == "Escape" {
                event.prevent_default();
                let edit = numeric_edit.borrow_mut().take();
                if let Some(edit) = edit {
                    let start = edit.start;
                    submit_position(&runtime, &numeric_edit, edit, start);
                }
                if let Some(part) = runtime.model().accepted.and_then(|s| {
                    s.document
                        .parts
                        .iter()
                        .find(|p| runtime.model().selected_part_ids.first() == Some(&p.id))
                        .cloned()
                }) {
                    let restored_x = part.pose.at.x.to_string();
                    let restored_y = part.pose.at.y.to_string();
                    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                        if let Some(input) = document
                            .get_element_by_id("m1-position-x")
                            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                        {
                            input.set_value(&restored_x);
                        }
                        if let Some(input) = document
                            .get_element_by_id("m1-position-y")
                            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                        {
                            input.set_value(&restored_y);
                        }
                    }
                    draft.set((restored_x, restored_y));
                    dirty_axes.set((Some(part.id), false, false));
                }
                runtime.report("Position preview canceled.");
            } else if key == "Enter" {
                event.prevent_default();
                failure.set(None);
                commit_numeric(
                    &runtime,
                    &numeric_edit,
                    &pending,
                    &mut pending_owner,
                    &mut dirty_axes,
                    &draft(),
                );
            }
        })))
    };
    let (x, y) = draft();
    rsx! { aside { class: "m1-inspector", "aria-label": "Inspect",
        header { h2 { "Inspect" } }
        h2 { "Position" }
        if let Some(part) = selected {
            p { "{part.reference}" }
            label { "X (mm)" input { id: "m1-position-x", r#type: "number", step: "any", value: "{x}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                draft.write().0 = event.value();
                let id = runtime.model().selected_part_ids.first().cloned();
                let (draft_id, _, y_dirty) = dirty_axes.peek().clone();
                let same_target = draft_id == id;
                dirty_axes.set((id, true, same_target && y_dirty));
                let (Ok(px), Ok(py)) = (event.value().parse::<f64>(), draft().1.parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.first() == Some(&p.id)) else { return; };
                let mut edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at, part_ids: runtime.model().selected_part_ids, board_id: runtime.model().active_board_id, x_changed: false, y_changed: false });
                edit.x_changed = true;
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py });
            }} } }
            label { "Y (mm)" input { id: "m1-position-y", r#type: "number", step: "any", value: "{y}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                draft.write().1 = event.value();
                let id = runtime.model().selected_part_ids.first().cloned();
                let (draft_id, x_dirty, _) = dirty_axes.peek().clone();
                let same_target = draft_id == id;
                dirty_axes.set((id, same_target && x_dirty, true));
                let (Ok(px), Ok(py)) = (draft().0.parse::<f64>(), event.value().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.first() == Some(&p.id)) else { return; };
                let mut edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at, part_ids: runtime.model().selected_part_ids, board_id: runtime.model().active_board_id, x_changed: false, y_changed: false });
                edit.y_changed = true;
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py });
            }} } }
            button { disabled: apply_disabled(), onclick: submit, "Apply position" }
            if let Some(message) = failure() { p { role: "alert", "{message}" } }
            if numeric_edit.borrow().is_some() { p { role: "status", "Preview only. Press Enter or Apply position to save, or Escape to cancel." } }
        } else { p { "Select a component to edit its position." } }
    }}
}

fn submit_position(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    edit: NumericEdit,
    at: Vec2,
) {
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: edit.transaction_id.clone(),
        target_ids: vec![edit.id.clone()],
        operation: EditOperation::MoveParts {
            positions: vec![Position {
                id: edit.id.clone(),
                at,
            }],
        },
    });
    if at != edit.start {
        *current.borrow_mut() = Some(edit);
    } else {
        current.borrow_mut().take();
    }
}

fn commit_numeric(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    pending: &Signal<PendingEditSignals<(), (String, String)>>,
    pending_owner: &mut Signal<Option<NumericOwner>>,
    dirty_axes: &mut Signal<(Option<String>, bool, bool)>,
    draft: &(String, String),
) {
    let (Ok(x), Ok(y)) = (draft.0.parse::<f64>(), draft.1.parse::<f64>()) else {
        runtime.report("Enter finite X and Y coordinates.");
        return;
    };
    if !x.is_finite() || !y.is_finite() {
        runtime.report("Enter finite X and Y coordinates.");
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted else {
        return;
    };
    let Some(part) = snapshot
        .document
        .parts
        .iter()
        .find(|part| model.selected_part_ids.first() == Some(&part.id))
    else {
        return;
    };
    let start = part.pose.at;
    let edit = current.borrow_mut().take().unwrap_or_else(|| NumericEdit {
        id: part.id.clone(),
        transaction_id: format!("position-{}", runtime.operation().0),
        start,
        part_ids: model.selected_part_ids.clone(),
        board_id: model.active_board_id.clone(),
        x_changed: true,
        y_changed: true,
    });
    if edit.part_ids != model.selected_part_ids || edit.board_id != model.active_board_id {
        return;
    }
    let owner = NumericOwner {
        scope: runtime.scope(),
        part_ids: edit.part_ids.clone(),
        board_id: edit.board_id.clone(),
    };
    pending_owner.set(Some(owner));
    dirty_axes.set((Some(edit.id.clone()), false, false));
    pending.peek().begin_value(
        runtime,
        (),
        "layout-old-position",
        Some("position".into()),
        super::layout_component_edits::commit_position_resolver(
            edit.part_ids,
            edit.board_id,
            edit.x_changed.then_some(x),
            edit.y_changed.then_some(y),
            edit.transaction_id,
        ),
        draft,
    );
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::{Board, Part, Pose2, ProjectDoc, Side};
    use wasm_bindgen_test::*;

    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let subscription_runtime = runtime.clone();
        use_hook(move || {
            subscription_runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        });
        rsx! {
            Inspector {}
            button {
                id: "position-test-deselect",
                onclick: {
                    let runtime = runtime.clone();
                    move |_| {
                        runtime.submit(Event::SelectParts {
                            operation_id: runtime.operation(),
                            part_ids: vec![],
                            range_part_ids: vec![],
                            mode: boardstudio_application::SelectionMode::Replace,
                        });
                    }
                },
                "Deselect"
            }
            button {
                id: "position-test-select-a",
                onclick: move |_| {
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: vec!["a".into()],
                        range_part_ids: vec![],
                        mode: boardstudio_application::SelectionMode::Replace,
                    });
                },
                "Select A"
            }
        }
    }

    async fn mounted_position_inspector() -> (Rc<Runtime>, web_sys::Element) {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("position-draft-doc", "Position draft");
        document.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "switch:base",
                "name": "MX switch",
                "kind": "switch",
                "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
                "pads": []
            }))
            .unwrap(),
        );
        document.parts.push(Part {
            id: "a".into(),
            definition_id: "switch:base".into(),
            reference: "A".into(),
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
            part_ids: vec!["a".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        support::open_document(&runtime, document).await;
        runtime.submit(Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: vec!["a".into()],
            range_part_ids: vec![],
            mode: boardstudio_application::SelectionMode::Replace,
        });
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        (runtime, root)
    }

    #[wasm_bindgen_test]
    async fn mounted_position_landing_preserves_a_newer_draft_at_the_original_coordinate() {
        let (runtime, root) = mounted_position_inspector().await;
        let x = root
            .query_selector("#m1-position-x")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let y = root
            .query_selector("#m1-position-y")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let apply = root
            .query_selector("button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let bubbling = web_sys::EventInit::new();
        bubbling.set_bubbles(true);
        x.set_value("5");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
        support::run_pending(&runtime).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        apply.click();
        support::drive_pending(&runtime);
        entered.await.expect("the position commit reached Core");
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert!(
            apply.has_attribute("disabled"),
            "Apply remains disabled while its edit is pending"
        );

        x.set_value("0");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
        release.send(()).expect("release the held position result");
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.parts[0].pose.at, Vec2 { x: 5.0, y: 0.0 });
        assert_eq!(
            x.value(),
            "0",
            "landing preserves newer text even at the original coordinate"
        );
        assert_eq!(
            y.value(),
            "0",
            "the clean axis projects its accepted coordinate"
        );
        assert!(
            !apply.has_attribute("disabled"),
            "Apply becomes available after settlement"
        );
        assert!(root.query_selector("[role='alert']").unwrap().is_none());
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_failed_position_restores_the_clean_axis_and_preserves_a_newer_draft() {
        let (runtime, root) = mounted_position_inspector().await;
        let x = root
            .query_selector("#m1-position-x")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let y = root
            .query_selector("#m1-position-y")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let apply = root
            .query_selector("button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let bubbling = web_sys::EventInit::new();
        bubbling.set_bubbles(true);
        for (field, value) in [(&x, "5"), (&y, "7")] {
            field.set_value(value);
            field
                .dispatch_event(
                    &web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap(),
                )
                .unwrap();
        }
        support::run_pending(&runtime).await;
        support::fail_next_persist(&runtime, "legacy position durable write failure");
        let (entered, release) = support::gate_next_core_reply(&runtime);
        apply.click();
        support::drive_pending(&runtime);
        entered.await.expect("the position commit reached Core");
        x.set_value("9");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert_eq!(
            y.value(),
            "7",
            "the submitted Y stays visible while pending"
        );
        assert!(apply.has_attribute("disabled"));

        release.send(()).expect("release the held position result");
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let model = runtime.model();
        assert_eq!(
            model.accepted.unwrap().document.parts[0].pose.at,
            Vec2 { x: 0.0, y: 0.0 },
            "a failed durable save leaves the accepted pair unchanged"
        );
        assert_eq!(
            model.lifecycle,
            boardstudio_application::Lifecycle::RecoveryRequired
        );
        assert_eq!(x.value(), "9", "failure preserves the newer X draft");
        assert_eq!(
            y.value(),
            "0",
            "the unchanged submitted Y restores its accepted value"
        );
        assert!(!apply.has_attribute("disabled"));
        let alert = root
            .query_selector("[role='alert']")
            .unwrap()
            .expect("the failed save is explained inline")
            .text_content()
            .unwrap();
        assert!(
            alert.contains("legacy position durable write failure"),
            "{alert}"
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_position_reselection_discards_the_departed_draft() {
        let (runtime, root) = mounted_position_inspector().await;
        let x = root
            .query_selector("#m1-position-x")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let bubbling = web_sys::EventInit::new();
        bubbling.set_bubbles(true);
        x.set_value("5");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
        root.query_selector(".m1-inspector button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.parts[0]
                .pose
                .at
                .x,
            5.0
        );
        x.set_value("9");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
            .unwrap();
        support::run_pending(&runtime).await;
        root.query_selector("#position-test-deselect")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        for _ in 0..5 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(root.query_selector("#m1-position-x").unwrap().is_none());
        root.query_selector("#position-test-select-a")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        for _ in 0..5 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let reselected_x = root
            .query_selector("#m1-position-x")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        assert_eq!(
            reselected_x.value(),
            "5",
            "reselection projects the accepted coordinate after the previous owner leaves"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.parts[0]
                .pose
                .at
                .x,
            5.0
        );
        runtime.unsubscribe();
        root.remove();
    }
}
