use crate::runtime::Runtime;
use boardstudio_web_ui_shared::inspector_fields::TextField;
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    callout::{Callout, CalloutVariant},
    field::FieldError,
    input::InputType,
};
use dioxus::prelude::*;
use std::rc::Rc;

mod companion_placement;
mod layout_component_inspector;
pub(super) use companion_placement::{
    CompanionPlacementAction, CompanionPlacementInspector, CompanionPlacementOwner,
    CompanionPlacementProjection,
};
pub(super) use layout_component_inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorOwner,
    LayoutComponentInspectorOwnerKey, LayoutComponentInspectorProjection,
};

fn parse_axis(raw: &str, label: &str) -> Result<f64, String> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("Enter a valid number for {label}."))
}

#[component]
pub(super) fn Inspector() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let mut failure = use_signal(|| None::<String>);
    let model = runtime.model();
    let selected = model.accepted.as_ref().and_then(|s| {
        s.document
            .parts
            .iter()
            .find(|p| model.selected_part_ids.first() == Some(&p.id))
            .cloned()
    });
    let current_key = selected
        .as_ref()
        .map(|p| (p.id.clone(), p.pose.at.x.to_bits(), p.pose.at.y.to_bits()));
    let mut synced_key = use_signal(|| current_key.clone());
    let mut x_text = use_signal(|| {
        selected
            .as_ref()
            .map_or_else(String::new, |p| p.pose.at.x.to_string())
    });
    let mut y_text = use_signal(|| {
        selected
            .as_ref()
            .map_or_else(String::new, |p| p.pose.at.y.to_string())
    });
    use_effect(use_reactive((&current_key,), move |(current_key,)| {
        if *synced_key.peek() != current_key {
            let (x, y) = current_key.as_ref().map_or_else(
                || (String::new(), String::new()),
                |(_, x_bits, y_bits)| {
                    (
                        f64::from_bits(*x_bits).to_string(),
                        f64::from_bits(*y_bits).to_string(),
                    )
                },
            );
            synced_key.set(current_key);
            x_text.set(x);
            y_text.set(y);
            failure.set(None);
        }
    }));
    let apply_position = {
        let rt = runtime.clone();
        move || {
            let x = match parse_axis(&x_text(), "X (mm)") {
                Ok(v) => v,
                Err(msg) => {
                    failure.set(Some(msg));
                    return;
                }
            };
            let y = match parse_axis(&y_text(), "Y (mm)") {
                Ok(v) => v,
                Err(msg) => {
                    failure.set(Some(msg));
                    return;
                }
            };
            let m = rt.model();
            let res = rt.commit(
                "layout-old-position",
                super::layout_component_edits::commit_position_resolver(
                    m.selected_part_ids,
                    m.active_board_id,
                    Some(x),
                    Some(y),
                    format!("position-{}", rt.operation().0),
                ),
            );
            failure.set(res.err());
        }
    };
    let mut submit_click = apply_position.clone();
    let mut submit_key = apply_position;
    let on_axis_key = move |event: KeyboardEvent| {
        if event.key() == Key::Enter {
            event.prevent_default();
            submit_key();
        }
    };
    rsx! {
        aside { class: "flex flex-col min-w-0 min-h-0 overflow-auto bg-transparent max-[760px]:overflow-visible", "aria-label": "Inspect",
            header { class: "hidden", h2 { "Inspect" } }
            h2 { class: "hidden", "Position" }
            if let Some(part) = selected {
                p { class: "mx-4 text-muted-foreground", "{part.reference}" }
                div { class: "flex flex-col gap-2 px-4", onkeydown: on_axis_key,
                    TextField { id: "m1-position-x".to_owned(), label: "X (mm)".to_owned(), r#type: InputType::Number, value: x_text(), oninput: move |e: FormEvent| x_text.set(e.value()) }
                    TextField { id: "m1-position-y".to_owned(), label: "Y (mm)".to_owned(), r#type: InputType::Number, value: y_text(), oninput: move |e: FormEvent| y_text.set(e.value()) }
                    Button { variant: ButtonVariant::Secondary, button_type: "button", class: "mt-1", onclick: move |_| submit_click(), "Apply position" }
                }
                if let Some(msg) = failure() { FieldError { "{msg}" } }
            } else {
                Callout { variant: CalloutVariant::Info, "Select a component to edit its position." }
            }
        }
    }
}
