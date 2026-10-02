//! Layout Inspector controls for accepted keycap size.
use super::keycap_resize::ResizeAxis;
use super::keycap_size_controller::{KeySizeMount, KeySizeRequest, KeySizeState};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct KeySizeControlsProps {
    pub mount: KeySizeMount,
}

#[component]
pub(in crate::presentation) fn KeySizeControls(props: KeySizeControlsProps) -> Element {
    let Some(projection) = props.mount.projection.clone() else {
        return rsx! {};
    };
    let mut draft = use_signal(|| projection.units);
    let sent = use_signal(|| None::<String>);
    let keyboard_generation = use_signal(|| 0u64);
    let sequence = props.mount.request_sequence;
    let base_units = projection.units;
    let mixed = projection.mixed;
    let mixed_x = projection.mixed_x;
    let mixed_y = projection.mixed_y;
    let items = projection.items.clone();
    let snapshot_token = projection.snapshot_token;
    let revision = projection.revision;
    let owner_for_effect = projection.owner.clone();
    let mut draft_for_effect = draft;
    let mut sent_for_effect = sent;
    use_effect(use_reactive(
        (&owner_for_effect, &base_units, &mixed),
        move |(_, units, _)| {
            draft_for_effect.set(units);
            sent_for_effect.set(None);
        },
    ));

    let submit = use_callback({
        let callback = props.mount.on_resize;
        let owner = projection.owner.clone();
        let mut sent = sent;
        let mut sequence = sequence;
        move |(units, axis): (boardstudio_core::model::Vec2, Option<ResizeAxis>)| {
            let should_send = match axis {
                Some(ResizeAxis::X) => {
                    mixed_x
                        || items.iter().any(|item| {
                            quarter((item.size.x + item.gap.x) / item.pitch.x) != units.x
                        })
                }
                Some(ResizeAxis::Y) => {
                    mixed_y
                        || items.iter().any(|item| {
                            quarter((item.size.y + item.gap.y) / item.pitch.y) != units.y
                        })
                }
                None => mixed || units != base_units,
            };
            if !should_send {
                return;
            }
            let signature = format!(
                "{}:{}:{:.2}:{:.2}",
                identity_key(&owner),
                axis_key(axis),
                units.x,
                units.y
            );
            if sent().as_deref() == Some(signature.as_str()) {
                return;
            }
            sent.set(Some(signature));
            let request_id = sequence() + 1;
            sequence.set(request_id);
            callback.call(KeySizeRequest {
                owner: owner.clone(),
                request_id,
                snapshot_token,
                revision,
                units,
                axis,
            });
        }
    });

    let width_value = draft().x;
    let height_value = draft().y;
    let keyboard_commit = use_callback({
        let mut generation = keyboard_generation;
        let draft = draft;
        move |axis: ResizeAxis| {
            let next = generation().wrapping_add(1);
            generation.set(next);
            let captured = next;
            let generation = generation;
            let submit = submit;
            let draft = draft;
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(150).await;
                if generation() == captured {
                    submit.call((draft(), Some(axis)));
                }
            });
        }
    });
    let feedback = props.mount.feedback.clone();
    let current_request_id = (props.mount.request_sequence)();
    let status = feedback
        .as_ref()
        .filter(|value| value.request_id == current_request_id)
        .map(|value| match value.state {
            KeySizeState::Pending => "Saving key-size change…",
            KeySizeState::Saved => "Saved",
            KeySizeState::Failed => "Key-size change failed",
        });
    let error = feedback
        .as_ref()
        .filter(|value| value.request_id == current_request_id)
        .and_then(|value| value.message.clone());
    let value_label = if mixed {
        "Mixed".to_owned()
    } else {
        format!("{}u × {}u", draft().x, draft().y)
    };
    let mut timer_generation = keyboard_generation;

    rsx! {
        section { class: "m1-key-size", aria_label: "Key size",
            div { class: "m1-key-size-heading",
                strong { "Key size" }
                output { "{value_label}" }
            }
            label { class: "m1-key-size-axis",
                "Width"
                input {
                    r#type: "range", aria_label: "Key width", min: "1", max: "7", step: "0.25",
                    value: "{width_value}", aria_valuetext: "{width_value}u", disabled: !props.mount.editable || props.mount.busy,
                    oninput: move |event| if let Ok(value) = event.value().parse::<f64>() { draft.write().x = value; },
                    onpointerup: move |_| { timer_generation.set(timer_generation().wrapping_add(1)); submit.call((draft(), Some(ResizeAxis::X))); },
                    onkeyup: move |_| keyboard_commit.call(ResizeAxis::X),
                    onblur: move |_| { timer_generation.set(timer_generation().wrapping_add(1)); submit.call((draft(), Some(ResizeAxis::X))); },
                }
                span { "{width_value}u" }
            }
            label { class: "m1-key-size-axis",
                "Height"
                input {
                    r#type: "range", aria_label: "Key height", min: "1", max: "7", step: "0.25",
                    value: "{height_value}", aria_valuetext: "{height_value}u", disabled: !props.mount.editable || props.mount.busy,
                    oninput: move |event| if let Ok(value) = event.value().parse::<f64>() { draft.write().y = value; },
                    onpointerup: move |_| { timer_generation.set(timer_generation().wrapping_add(1)); submit.call((draft(), Some(ResizeAxis::Y))); },
                    onkeyup: move |_| keyboard_commit.call(ResizeAxis::Y),
                    onblur: move |_| { timer_generation.set(timer_generation().wrapping_add(1)); submit.call((draft(), Some(ResizeAxis::Y))); },
                }
                span { "{height_value}u" }
            }
            div { class: "m1-key-size-orientation", role: "group", aria_label: "Key orientation",
                button { disabled: !props.mount.editable || props.mount.busy, aria_pressed: !mixed && draft().x >= draft().y,
                    onclick: move |_| { let current = draft(); let long = current.x.max(current.y); let short = current.x.min(current.y); let next = boardstudio_core::model::Vec2 { x: long, y: short }; draft.set(next); submit.call((next, None)); }, "Wide" }
                button { disabled: !props.mount.editable || props.mount.busy, aria_pressed: !mixed && draft().y > draft().x,
                    onclick: move |_| { let current = draft(); let long = current.x.max(current.y); let short = current.x.min(current.y); let next = boardstudio_core::model::Vec2 { x: short, y: long }; draft.set(next); submit.call((next, None)); }, "Tall" }
            }
            if let Some(status) = status { p { role: "status", class: "m1-key-size-status", "{status}" } }
            if let Some(error) = error { p { role: "alert", class: "m1-key-size-error", "{error}" } }
        }
    }
}

fn quarter(value: f64) -> f64 {
    (value * 4.0).round() / 4.0
}
fn identity_key(owner: &super::keycap_size_controller::KeySizeOwner) -> String {
    format!("{:?}", owner)
}
fn axis_key(axis: Option<ResizeAxis>) -> &'static str {
    match axis {
        Some(ResizeAxis::X) => "x",
        Some(ResizeAxis::Y) => "y",
        None => "both",
    }
}
