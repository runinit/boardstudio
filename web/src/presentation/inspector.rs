use crate::runtime::Runtime;
use boardstudio_application::Event;
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, Position, Vec2};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
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
    revision: u64,
    transaction_id: String,
    start: Vec2,
    part_ids: Vec<String>,
    board_id: String,
    x_changed: bool,
    y_changed: bool,
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
    let tickets = use_hook(|| Rc::new(RefCell::new(Vec::<(NumericEdit, EditTicket)>::new())));
    let mut failure = use_signal(|| None::<String>);
    use_drop({
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move || {
            let pending = numeric_edit.borrow_mut().take();
            if let Some(edit) = pending {
                let start = edit.start;
                submit_position(&runtime, &numeric_edit, edit, start, EditPhase::Preview);
            }
        }
    });
    let key = selected
        .as_ref()
        .map(|p| (p.id.clone(), p.pose.at.x, p.pose.at.y));
    use_effect(use_reactive((&key, &version), {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        let tickets = tickets.clone();
        move |(key, _)| {
            let selected_ids = runtime.model().selected_part_ids;
            tickets.borrow_mut().retain(|(edit, ticket)| {
                match ticket.settlement(edit.part_ids == selected_ids) {
                    Settlement::Pending => true,
                    Settlement::Failed { message } => {
                        failure.set(Some(message));
                        false
                    }
                    Settlement::Landed { .. } | Settlement::Retired => false,
                }
            });
            if let Some((_, px, py)) = key.as_ref() {
                let draft = numeric_edit.borrow();
                if !draft.as_ref().is_some_and(|edit| edit.x_changed)
                    && !tickets.borrow().iter().any(|(edit, _)| edit.x_changed)
                {
                    x.set(px.to_string());
                }
                if !draft.as_ref().is_some_and(|edit| edit.y_changed)
                    && !tickets.borrow().iter().any(|(edit, _)| edit.y_changed)
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
                submit_position(
                    &runtime,
                    &Rc::new(RefCell::new(None)),
                    edit,
                    start,
                    EditPhase::Preview,
                );
            }
        }
    }));
    let submit = {
        let tickets = tickets.clone();
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |_| {
            failure.set(None);
            commit_numeric(&runtime, &numeric_edit, &tickets, &x(), &y());
        }
    };
    let cancel_numeric: KeyboardHandler = {
        let tickets = tickets.clone();
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
                    submit_position(&runtime, &numeric_edit, edit, start, EditPhase::Preview);
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
                commit_numeric(&runtime, &numeric_edit, &tickets, &x(), &y());
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
                let mut edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), revision: snapshot.document.revision, transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at, part_ids: runtime.model().selected_part_ids, board_id: runtime.model().active_board_id, x_changed: false, y_changed: false });
                edit.x_changed = true;
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py }, EditPhase::Preview);
            }} } }
            label { "Y (mm)" input { id: "m1-position-y", r#type: "number", step: "any", value: "{y}", onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) }, oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
                y.set(event.value());
                let (Ok(px), Ok(py)) = (x().parse::<f64>(), event.value().parse::<f64>()) else { return; };
                if !px.is_finite() || !py.is_finite() { return; }
                let Some(snapshot) = runtime.model().accepted else { return; };
                let Some(part) = snapshot.document.parts.iter().find(|p| runtime.model().selected_part_ids.first() == Some(&p.id)) else { return; };
                let mut edit = numeric_edit.borrow_mut().take().unwrap_or_else(|| NumericEdit { id: part.id.clone(), revision: snapshot.document.revision, transaction_id: format!("position-{}", runtime.operation().0), start: part.pose.at, part_ids: runtime.model().selected_part_ids, board_id: runtime.model().active_board_id, x_changed: false, y_changed: false });
                edit.y_changed = true;
                submit_position(&runtime, &numeric_edit, edit, Vec2 { x: px, y: py }, EditPhase::Preview);
            }} } }
            button { onclick: submit, "Apply position" }
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
    phase: EditPhase,
) {
    runtime.submit(Event::Edit {
        operation_id: runtime.operation(),
        command: EditCommand {
            base_revision: edit.revision,
            transaction_id: edit.transaction_id.clone(),
            phase,
            target_ids: vec![edit.id.clone()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: edit.id.clone(),
                    at,
                }],
            },
        },
    });
    if phase == EditPhase::Preview && at != edit.start {
        *current.borrow_mut() = Some(edit);
    } else {
        current.borrow_mut().take();
    }
}

fn commit_numeric(
    runtime: &Rc<Runtime>,
    current: &Rc<RefCell<Option<NumericEdit>>>,
    tickets: &Rc<RefCell<Vec<(NumericEdit, EditTicket)>>>,
    x: &str,
    y: &str,
) {
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
        revision: snapshot.document.revision,
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
    let ticket = EditTicket::begin(
        runtime,
        "layout-old-position",
        Some("position".into()),
        super::layout_component_edits::commit_position_resolver(
            edit.part_ids.clone(),
            edit.board_id.clone(),
            edit.x_changed.then_some(x),
            edit.y_changed.then_some(y),
            edit.transaction_id.clone(),
        ),
    );
    tickets.borrow_mut().push((edit, ticket));
}
