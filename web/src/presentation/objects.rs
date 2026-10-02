use crate::runtime::Runtime;
use boardstudio_application::{Event, SelectionMode};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[component]
pub(super) fn Objects() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = &snapshot.document;
    let items: Rc<Vec<_>> = Rc::new(
        document
            .parts
            .iter()
            .filter(|part| {
                document
                    .boards
                    .iter()
                    .find(|board| board.id == model.active_board_id)
                    .is_some_and(|board| board.part_ids.contains(&part.id))
            })
            .enumerate()
            .map(|(index, part)| {
                let kind = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .map(|definition| format!("{:?}", definition.kind))
                    .unwrap_or_else(|| "component".into());
                (
                    index,
                    part.id.clone(),
                    part.reference.clone(),
                    kind,
                    model.selected_part_ids.contains(&part.id),
                )
            })
            .collect(),
    );
    let selected = items.iter().any(|item| item.4);
    let board_runtime = runtime.clone();
    let instance_runtime = runtime.clone();
    let board_id = model.active_board_id.clone();
    let active_instance = model.active_instance_id.clone().unwrap_or_default();
    let instances: Vec<_> = document
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .filter(|instance| instance.board_id == model.active_board_id)
                .collect()
        })
        .unwrap_or_default();
    let select: Rc<dyn Fn(String)> = Rc::new({
        let runtime = runtime.clone();
        move |id| {
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: vec![id],
                range_part_ids: vec![],
                mode: SelectionMode::Replace,
            })
        }
    });
    rsx! {
        aside { class: "m1-objects", "aria-label": "Objects",
            header { h2 { "Objects" } }
            div { class: "m1-object-navigation",
                label { "Board"
                    select { "aria-label": "Board", value: "{model.active_board_id}", onchange: move |event: FormEvent| board_runtime.submit(Event::Navigate { operation_id: board_runtime.operation(), board_id: event.value(), instance_id: None }),
                        for board in &document.boards { option { value: "{board.id}", "{board.name}" } }
                    }
                }
                if !instances.is_empty() {
                    label { "Physical instance"
                        select { "aria-label": "Physical instance", value: "{active_instance}", onchange: move |event: FormEvent| {
                            let value = event.value();
                            instance_runtime.submit(Event::Navigate { operation_id: instance_runtime.operation(), board_id: board_id.clone(), instance_id: (!value.is_empty()).then_some(value) });
                        },
                            option { value: "", "Canonical board" }
                            for instance in &instances { option { key: "{instance.id}", value: "{instance.id}", "{instance.name}" } }
                        }
                    }
                }
            }
            div { class: "m1-object-tree",
                div { class: "m1-object-tree-heading", "{document.name}", span { "{items.len()} parts" } }
                div { role: "listbox", "aria-label": "Objects on current board", class: "m1-component-list",
                    for (index, id, reference, kind, is_selected) in items.iter().cloned() {
                        {
                            let click = select.clone();
                            let key_select = select.clone();
                            let items_for_key = items.clone();
                            rsx! { button { key: "{id}", id: "m1-object-{index}", class: if is_selected { "m1-component selected" } else { "m1-component" }, role: "option", "aria-label": "{reference}, {kind}", "aria-selected": "{is_selected}", tabindex: if is_selected || (!selected && index == 0) { "0" } else { "-1" }, onclick: move |_| click(id.clone()), onkeydown: move |event: KeyboardEvent| {
                                let key = event.data().key().to_string();
                                let next = match key.as_str() { "ArrowDown" => Some(index + 1), "ArrowUp" => Some(index.saturating_sub(1)), "Home" => Some(0), "End" => Some(items_for_key.len().saturating_sub(1)), _ => None };
                                let Some(next) = next.filter(|next| *next < items_for_key.len()) else { return; };
                                event.prevent_default();
                                if let Some((_, next_id, _, _, _)) = items_for_key.get(next) { key_select(next_id.clone()); }
                                if let Some(element) = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id(&format!("m1-object-{next}"))).and_then(|element| element.dyn_into::<HtmlElement>().ok()) { let _ = element.focus(); }
                            }, span { class: "m1-object-reference", "{reference}" } span { class: "m1-object-kind", "{kind}" } } }
                        }
                    }
                }
            }
        }
    }
}
