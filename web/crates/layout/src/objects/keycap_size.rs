//! Layout Inspector controls for accepted keycap size.
use super::keycap_resize::ResizeAxis;
use super::keycap_size_controller::{KeySizeMount, KeySizeOwner, KeySizeRequest};
use dioxus::prelude::*;
use std::cell::{Cell, RefCell};

#[derive(Clone)]
struct ResizeIntent {
    owner: KeySizeOwner,
    snapshot_token: boardstudio_application::SnapshotToken,
    revision: u64,
    units: boardstudio_core::model::Vec2,
    axis: Option<ResizeAxis>,
}

impl ResizeIntent {
    fn for_owner(
        owner: &KeySizeOwner,
        snapshot_token: boardstudio_application::SnapshotToken,
        revision: u64,
        units: boardstudio_core::model::Vec2,
        axis: ResizeAxis,
    ) -> Self {
        Self {
            owner: owner.clone(),
            snapshot_token,
            revision,
            units,
            axis: Some(axis),
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeySizeControlsProps {
    pub mount: KeySizeMount,
}

#[component]
pub fn KeySizeControls(props: KeySizeControlsProps) -> Element {
    let Some(projection) = props.mount.projection.clone() else {
        return rsx! {};
    };
    let mut inspector_mounted = props.mount.inspector_mounted;
    use_hook(|| inspector_mounted.set(true));
    let mut draft = use_signal(|| projection.units);
    let mut draft_dirty = use_signal(|| false);
    let failure = use_signal(|| None::<String>);
    let on_bind_draft = props.mount.on_bind_draft;
    on_bind_draft.call((
        projection.owner.clone(),
        Some((draft, failure, draft_dirty)),
    ));
    use_drop({
        let owner = projection.owner.clone();
        move || {
            on_bind_draft.call((owner, None));
            inspector_mounted.set(false);
        }
    });
    let sent = use_signal(|| None::<String>);
    let keyboard_generation = use_hook(|| std::rc::Rc::new(Cell::new(0u64)));
    let sequence = props.mount.request_sequence;
    let base_units = projection.units;
    let mixed = projection.mixed;
    let mixed_x = projection.mixed_x;
    let mixed_y = projection.mixed_y;
    let items = projection.items.clone();
    let overlap_warning = (!projection.overlap_references.is_empty()).then(|| {
        let references = projection.overlap_references[..projection.overlap_references.len().min(8)]
            .join(", ");
        let remaining = projection.overlap_references.len().saturating_sub(8);
        if remaining > 0 {
            format!("Keycaps overlap: {references} and {remaining} more. Adjust their rows or columns to clear the overlap.")
        } else {
            format!("Keycaps overlap: {references}. Adjust their rows or columns to clear the overlap.")
        }
    });
    let snapshot_token = projection.snapshot_token;
    let revision = projection.revision;
    let owner_for_effect = projection.owner.clone();
    let owner_for_projection =
        use_hook(|| std::rc::Rc::new(RefCell::new(projection.owner.clone())));
    let feedback = props.mount.feedback.clone();
    let mut draft_for_effect = draft;
    let mut sent_for_effect = sent;
    let mut failure_for_effect = failure;
    let mut dirty_for_effect = draft_dirty;
    use_effect(use_reactive(
        (&owner_for_effect, &base_units, &mixed, &feedback),
        {
            let keyboard_generation = keyboard_generation.clone();
            move |(owner, units, _, _)| {
                keyboard_generation.set(keyboard_generation.get().wrapping_add(1));
                let mut previous_owner = owner_for_projection.borrow_mut();
                if *previous_owner != owner || !*dirty_for_effect.peek() {
                    if *draft_for_effect.peek() != units {
                        draft_for_effect.set(units);
                    }
                    if *dirty_for_effect.peek() {
                        dirty_for_effect.set(false);
                    }
                }
                if *previous_owner != owner && failure_for_effect.peek().is_some() {
                    failure_for_effect.set(None);
                }
                *previous_owner = owner;
                sent_for_effect.set(None);
            }
        },
    ));

    let submit = use_callback({
        let callback = props.mount.on_resize;
        let owner = projection.owner.clone();
        let mut sent = sent;
        let mut sequence = sequence;
        move |intent: ResizeIntent| {
            if intent.owner != owner
                || intent.snapshot_token != snapshot_token
                || intent.revision != revision
            {
                return;
            }
            let units = intent.units;
            let axis = intent.axis;
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
                identity_key(&intent.owner),
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
                owner: intent.owner,
                request_id,
                snapshot_token: intent.snapshot_token,
                revision: intent.revision,
                units,
                axis,
            });
        }
    });

    let width_value = draft().x;
    let height_value = draft().y;
    let keyboard_commit = use_callback({
        let generation = keyboard_generation.clone();
        move |intent: ResizeIntent| {
            let next = generation.get().wrapping_add(1);
            generation.set(next);
            let captured = next;
            let generation = generation.clone();
            let submit = submit;
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(150).await;
                if generation.get() == captured {
                    submit.call(intent);
                }
            });
        }
    });
    let current_request_id = (props.mount.request_sequence)();
    let error = feedback
        .as_ref()
        .filter(|value| value.request_id == current_request_id && value.owner == projection.owner)
        .and_then(|value| value.message.clone());
    let value_label = if mixed {
        "Mixed".to_owned()
    } else {
        format!("{}u × {}u", draft().x, draft().y)
    };
    let event_owner = projection.owner.clone();
    let timer_generation = keyboard_generation.clone();

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
                    value: "{width_value}", aria_valuetext: "{width_value}u", disabled: !props.mount.editable,
                    oninput: move |event| if let Ok(value) = event.value().parse::<f64>() { draft_dirty.set(true); draft.write().x = value; },
                    onpointerup: {
                        let owner = event_owner.clone();
                        let generation = timer_generation.clone();
                        move |_| {
                            generation.set(generation.get().wrapping_add(1));
                            submit.call(ResizeIntent::for_owner(&owner, snapshot_token, revision, draft(), ResizeAxis::X));
                        }
                    },
                    onkeyup: {
                        let owner = event_owner.clone();
                        move |_| keyboard_commit.call(ResizeIntent::for_owner(&owner, snapshot_token, revision, draft(), ResizeAxis::X))
                    },
                    onblur: {
                        let owner = event_owner.clone();
                        let generation = timer_generation.clone();
                        move |_| {
                            generation.set(generation.get().wrapping_add(1));
                            submit.call(ResizeIntent::for_owner(&owner, snapshot_token, revision, draft(), ResizeAxis::X));
                        }
                    },
                }
                span { "{width_value}u" }
            }
            label { class: "m1-key-size-axis",
                "Height"
                input {
                    r#type: "range", aria_label: "Key height", min: "1", max: "7", step: "0.25",
                    value: "{height_value}", aria_valuetext: "{height_value}u", disabled: !props.mount.editable,
                    oninput: move |event| if let Ok(value) = event.value().parse::<f64>() { draft_dirty.set(true); draft.write().y = value; },
                    onpointerup: {
                        let owner = event_owner.clone();
                        let generation = timer_generation.clone();
                        move |_| {
                            generation.set(generation.get().wrapping_add(1));
                            submit.call(ResizeIntent::for_owner(&owner, snapshot_token, revision, draft(), ResizeAxis::Y));
                        }
                    },
                    onkeyup: {
                        let owner = event_owner.clone();
                        move |_| keyboard_commit.call(ResizeIntent::for_owner(&owner, snapshot_token, revision, draft(), ResizeAxis::Y))
                    },
                    onblur: {
                        let owner = event_owner.clone();
                        let generation = timer_generation.clone();
                        move |_| {
                            generation.set(generation.get().wrapping_add(1));
                            submit.call(ResizeIntent::for_owner(&owner, snapshot_token, revision, draft(), ResizeAxis::Y));
                        }
                    },
                }
                span { "{height_value}u" }
            }
            div { class: "m1-key-size-orientation", role: "group", aria_label: "Key orientation",
                button { disabled: !props.mount.editable, aria_pressed: !mixed && draft().x >= draft().y,
                onclick: {
                    let owner = event_owner.clone();
                    let generation = timer_generation.clone();
                    move |_| {
                        generation.set(generation.get().wrapping_add(1));
                        let current = draft();
                        let long = current.x.max(current.y);
                        let short = current.x.min(current.y);
                        let next = boardstudio_core::model::Vec2 { x: long, y: short };
                        draft_dirty.set(true);
                        draft.set(next);
                        submit.call(ResizeIntent { owner: owner.clone(), snapshot_token, revision, units: next, axis: None });
                    }
                }, "Wide" }
                button { disabled: !props.mount.editable, aria_pressed: !mixed && draft().y > draft().x,
                onclick: {
                    let owner = event_owner.clone();
                    let generation = timer_generation.clone();
                    move |_| {
                        generation.set(generation.get().wrapping_add(1));
                        let current = draft();
                        let long = current.x.max(current.y);
                        let short = current.x.min(current.y);
                        let next = boardstudio_core::model::Vec2 { x: short, y: long };
                        draft_dirty.set(true);
                        draft.set(next);
                        submit.call(ResizeIntent { owner: owner.clone(), snapshot_token, revision, units: next, axis: None });
                    }
                }, "Tall" }
            }
            if let Some(error) = error { p { role: "alert", class: "m1-key-size-error", "{error}" } }
        }
        if let Some(warning) = overlap_warning {
            p { class: "m1-key-size-warning", role: "status", "{warning}" }
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
