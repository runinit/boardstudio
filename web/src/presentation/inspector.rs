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
    let mut x = use_signal(String::new);
    let mut y = use_signal(String::new);
    let numeric_edit = use_hook(|| Rc::new(RefCell::new(None::<NumericEdit>)));
    let pending = use_signal(PendingEditSignals::<()>::new);
    let apply_disabled = use_signal(|| false);
    pending.peek().bind_one_shot((), apply_disabled);
    let mut pending_owner = use_signal(|| None::<NumericOwner>);
    let mut submitted_draft = use_signal(|| None::<(String, String)>);
    let mut failure = use_signal(|| None::<String>);
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
            pending.peek().settle(false, |_| String::new());
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
                let results = pending.peek().settle(owner_is_live, |_| String::new());
                let settled = !results.is_empty();
                for result in results {
                    match result {
                        boardstudio_web_runtime::pending_edits::PendingEditResult::Failed {
                            message,
                            ..
                        } => failure.set(Some(message)),
                        boardstudio_web_runtime::pending_edits::PendingEditResult::Landed {
                            ..
                        }
                        | boardstudio_web_runtime::pending_edits::PendingEditResult::Retired {
                            ..
                        } => failure.set(None),
                    }
                    if let Some((submitted_x, submitted_y)) = submitted_draft.peek().clone()
                        && let Some((_, accepted_x, accepted_y)) = key.as_ref()
                    {
                        if x.peek().as_str() == submitted_x {
                            x.set(accepted_x.to_string());
                        }
                        if y.peek().as_str() == submitted_y {
                            y.set(accepted_y.to_string());
                        }
                    }
                    submitted_draft.set(None);
                }
                if !pending.peek().is_pending(&()) {
                    if pending_owner.peek().is_some() {
                        pending_owner.set(None);
                    }
                }
                if let Some((_, px, py)) = key.as_ref() {
                    let draft = numeric_edit.borrow();
                    if !settled
                        && !draft.as_ref().is_some_and(|edit| edit.x_changed)
                        && !pending.peek().is_pending(&())
                    {
                        x.set(px.to_string());
                    }
                    if !settled
                        && !draft.as_ref().is_some_and(|edit| edit.y_changed)
                        && !pending.peek().is_pending(&())
                    {
                        y.set(py.to_string());
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
        let mut submitted_draft = submitted_draft;
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |_| {
            failure.set(None);
            commit_numeric(
                &runtime,
                &numeric_edit,
                &pending,
                &mut pending_owner,
                &mut submitted_draft,
                &x(),
                &y(),
            );
        }
    };
    let cancel_numeric: KeyboardHandler = {
        let mut pending_owner = pending_owner;
        let mut submitted_draft = submitted_draft;
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        let mut x = x;
        let mut y = y;
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
                    x.set(restored_x);
                    y.set(restored_y);
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
                    &mut submitted_draft,
                    &x(),
                    &y(),
                );
            }
        })))
    };
    rsx! { aside { class: "m1-inspector", "aria-label": "Inspect",
        header { h2 { "Inspect" } }
        h2 { "Position" }
        if let Some(part) = selected {
            p { "{part.reference}" }
            label { "X (mm)" input { id: "m1-position-x", r#type: "number", step: "any", value: "{x}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                x.set(event.value());
                let (Ok(px), Ok(py)) = (event.value().parse::<f64>(), y().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.first() == Some(&p.id)) else { return; };
                let mut edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at, part_ids: runtime.model().selected_part_ids, board_id: runtime.model().active_board_id, x_changed: false, y_changed: false });
                edit.x_changed = true;
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py });
            }} } }
            label { "Y (mm)" input { id: "m1-position-y", r#type: "number", step: "any", value: "{y}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                y.set(event.value());
                let (Ok(px), Ok(py)) = (x().parse::<f64>(), event.value().parse::<f64>()) else { return; };
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
    pending: &Signal<PendingEditSignals<()>>,
    pending_owner: &mut Signal<Option<NumericOwner>>,
    submitted_draft: &mut Signal<Option<(String, String)>>,
    x: &str,
    y: &str,
) {
    let submitted_x = x.to_owned();
    let submitted_y = y.to_owned();
    let (Ok(x), Ok(y)) = (x.parse::<f64>(), y.parse::<f64>()) else {
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
    submitted_draft.set(Some((submitted_x, submitted_y)));
    pending.peek().begin_one_shot(
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
    );
}
