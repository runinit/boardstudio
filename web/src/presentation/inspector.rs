use crate::runtime::Runtime;
use boardstudio_application::Event;
use boardstudio_core::model::{EditOperation, Position, Vec2};
use boardstudio_web_ui_shared::ui::{
    button::Button,
    callout::{Callout, CalloutVariant},
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput, InputGroupText},
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

mod companion_placement;
mod layout_component_inspector;
pub(super) use companion_placement::{
    CompanionPlacementAction, CompanionPlacementInspector, CompanionPlacementOwner,
    CompanionPlacementProjection,
};
#[cfg(test)]
pub(super) use layout_component_inspector::MirroredHalfProjection;
pub(super) use layout_component_inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorOwner,
    LayoutComponentInspectorOwnerKey, LayoutComponentInspectorProjection, LayoutConstraintValues,
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
    let key = selected
        .as_ref()
        .map(|p| (p.id.clone(), p.pose.at.x, p.pose.at.y));
    use_effect(use_reactive((&key, &version), {
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |(key, _)| {
            let (draft_id, x_dirty, y_dirty) = dirty_axes.peek().clone();
            let selected_id = key.as_ref().map(|(id, _, _)| id);
            let same_target = draft_id.as_ref() == selected_id;
            if !same_target {
                dirty_axes.set((selected_id.cloned(), false, false));
                failure.set(None);
            }
            if let Some((_, px, py)) = key.as_ref() {
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
            let changed_target = numeric_edit
                .borrow()
                .as_ref()
                .is_some_and(|edit| key.as_ref().is_none_or(|(id, _, _)| id != &edit.id));
            if changed_target && let Some(edit) = numeric_edit.borrow_mut().take() {
                let start = edit.start;
                submit_position(&runtime, &Rc::new(RefCell::new(None)), edit, start);
            }
        }
    }));
    let submit = {
        let mut dirty_axes = dirty_axes;
        let runtime = runtime.clone();
        let numeric_edit = numeric_edit.clone();
        move |_| {
            failure.set(None);
            commit_numeric(&runtime, &numeric_edit, &mut dirty_axes, draft, failure);
        }
    };
    let cancel_numeric: KeyboardHandler = {
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
                commit_numeric(&runtime, &numeric_edit, &mut dirty_axes, draft, failure);
            }
        })))
    };
    let (x, y) = draft();
    rsx! { aside { class: "flex flex-col min-w-0 min-h-0 overflow-auto bg-transparent max-[760px]:overflow-visible", "aria-label": "Inspect",
        header { class: "hidden", h2 { "Inspect" } }
        h2 { class: "hidden", "Position" }
        if let Some(part) = selected {
            p { class: "mx-4 text-muted-foreground", "{part.reference}" }
            div { onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) },
            Field {
            FieldLabel { html_for: "m1-position-x", "X (mm)" }
            InputGroup {
            InputGroupInput { id: "m1-position-x", r#type: InputType::Number, step: "any", value: "{x}", oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
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
            }} } InputGroupText { "mm" } } } }
            div { onkeydown: { let cancel = cancel_numeric.clone(); move |event| (cancel.borrow_mut())(event) },
            Field {
            FieldLabel { html_for: "m1-position-y", "Y (mm)" }
            InputGroup {
            InputGroupInput { id: "m1-position-y", r#type: InputType::Number, step: "any", value: "{y}", oninput: { let runtime = runtime.clone(); let numeric_edit = numeric_edit.clone(); move |event: FormEvent| {
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
            }} } InputGroupText { "mm" } } } }
            Button { class: "mx-4 mt-1", onclick: submit, "Apply position" }
            if let Some(message) = failure() { FieldError { "{message}" } }
            if numeric_edit.borrow().is_some() { div { role: "status", Callout { variant: CalloutVariant::Info, "Preview only. Press Enter or Apply position to save, or Escape to cancel." } } }
        } else { Callout { variant: CalloutVariant::Info, "Select a component to edit its position." } }
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
    dirty_axes: &mut Signal<(Option<String>, bool, bool)>,
    mut draft: Signal<(String, String)>,
    mut failure: Signal<Option<String>>,
) {
    let text = draft.peek().clone();
    let (Ok(x), Ok(y)) = (text.0.parse::<f64>(), text.1.parse::<f64>()) else {
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
    let id = edit.id;
    dirty_axes.set((Some(id.clone()), false, false));
    let result = runtime.commit(
        "layout-old-position",
        super::layout_component_edits::commit_position_resolver(
            edit.part_ids,
            edit.board_id,
            edit.x_changed.then_some(x),
            edit.y_changed.then_some(y),
            edit.transaction_id,
        ),
    );
    if let Some(part) = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|snapshot| snapshot.document.parts.iter().find(|part| part.id == id))
    {
        draft.set((part.pose.at.x.to_string(), part.pose.at.y.to_string()));
    }
    failure.set(result.err());
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use boardstudio_core::model::{Board, Part, Pose2, ProjectDoc, Side};
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
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
            Button {
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
            Button {
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
            anchor: None,
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
        wait_until("the mounted position Inspector renders", || {
            by_role_name(&root, "spinbutton", "X (mm)").is_some()
        })
        .await;
        (runtime, root)
    }

    #[wasm_bindgen_test]
    async fn mounted_position_keeps_native_precision_escape_enter_and_focus_owner() {
        let (runtime, root) = mounted_position_inspector().await;
        let x = by_role_name(&root, "spinbutton", "X (mm)")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        assert_eq!(x.step(), "any");
        assert_eq!(
            x.get_attribute("data-slot").as_deref(),
            Some("input-group-control")
        );
        x.focus().unwrap();
        let bubbling = web_sys::EventInit::new();
        bubbling.set_bubbles(true);
        let keyboard = |key: &str| {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(key);
            init.set_bubbles(true);
            x.dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
        };
        let input = |value: &str| {
            x.set_value(value);
            x.dispatch_event(
                &web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap(),
            )
            .unwrap();
        };
        let accepted_x = || {
            runtime.model().accepted.unwrap().document.parts[0]
                .pose
                .at
                .x
        };
        input("0.1234567890123456");
        support::run_pending(&runtime).await;
        keyboard("Escape");
        support::run_pending(&runtime).await;
        assert_eq!(accepted_x(), 0.0);
        assert_eq!(x.value(), "0");
        input("0.1234567890123456");
        support::run_pending(&runtime).await;
        keyboard("Enter");
        support::run_pending(&runtime).await;
        assert_eq!(accepted_x(), 0.1234567890123456);
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element(),
            Some(x.clone().into())
        );
        assert_eq!(
            by_role_name(&root, "spinbutton", "X (mm)").unwrap(),
            x.clone().into()
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(accepted_x(), 0.0);
        runtime.unsubscribe();
        root.remove();
    }
}
