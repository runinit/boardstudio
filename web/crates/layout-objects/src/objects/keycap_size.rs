//! Layout Inspector controls for accepted keycap size.
use super::keycap_resize::ResizeAxis;
use super::keycap_size_controller::{KeySizeMount, KeySizeOwner, KeySizeRequest};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::inspector_fields::{SelectField, ToggleField};
use boardstudio_web_ui_shared::ui::field::FieldError;

use dioxus::prelude::*;
use std::cell::{Cell, RefCell};

#[derive(Clone)]
struct ResizeIntent {
    owner: KeySizeOwner,
    snapshot_token: boardstudio_application::SnapshotToken,
    revision: u64,
    units: boardstudio_model::model::Vec2,
    axis: Option<ResizeAxis>,
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
    let mut draft = use_signal(|| projection.units);
    let mut draft_dirty = use_signal(|| false);
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
            draft_dirty.set(false);
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
    let selected_size = use_memo(use_reactive((&mixed,), move |(mixed,)| {
        let units = draft();
        if mixed || units.x.min(units.y) != 1.0 {
            "custom".to_owned()
        } else {
            units.x.max(units.y).to_string()
        }
    }));
    let event_owner = projection.owner.clone();
    let timer_generation = keyboard_generation.clone();

    rsx! {
        section { class: "grid min-w-0 gap-2.5 border-b border-border p-0 text-foreground", aria_label: "Key size",
        onkeydown: super::stop_space_pan,
            {
                let units = draft();
                let scale = 80.0 / units.y.max(units.x / 2.0).max(1.0);
                let width = units.x * scale;
                let height = units.y * scale;
                rsx! {
                    figure { class: "m-0 mb-3 text-center",
                        svg {
                            class: "block h-[100px] w-full",
                            view_box: "0 0 200 120",
                            role: "img",
                            "aria-label": "Keycap size {value_label}",
                            rect {
                                class: "fill-secondary stroke-primary stroke-[1]",
                                x: "{100.0 - width / 2.0}",
                                y: "{60.0 - height / 2.0}",
                                width: "{width}",
                                height: "{height}",
                                rx: "5",
                            }
                            rect {
                                class: "fill-field stroke-border-strong stroke-[1]",
                                x: "{104.0 - width / 2.0}",
                                y: "{64.0 - height / 2.0}",
                                width: "{(width - 8.0).max(1.0)}",
                                height: "{(height - 8.0).max(1.0)}",
                                rx: "3",
                            }
                        }
                        figcaption {
                            output { "{value_label}" }
                        }
                    }
                }
            }
            SelectField {
                label: "Size".to_owned(),
                value: selected_size(),
                options: {
                    let mut options = Vec::new();
                    if mixed || draft().x.min(draft().y) != 1.0 {
                        options.push(("custom".to_owned(), value_label.to_string()));
                    }
                    for step in 4..=28 {
                        let size = f64::from(step) / 4.0;
                        options.push((size.to_string(), format!("{size}u")));
                    }
                    options
                },
                aria_label: "Key size",
                disabled: !props.mount.editable,
                on_commit: {
                    let owner = event_owner.clone();
                    let generation = timer_generation.clone();
                    move |choice: String| {
                        let Ok(size) = choice.parse::<f64>() else {
                            return;
                        };
                        generation.set(generation.get().wrapping_add(1));
                        let rotated = draft().y > draft().x;
                        let next = if rotated {
                            boardstudio_model::model::Vec2 { x: 1.0, y: size }
                        } else {
                            boardstudio_model::model::Vec2 { x: size, y: 1.0 }
                        };
                        draft_dirty.set(true);
                        draft.set(next);
                        submit.call(ResizeIntent {
                            owner: owner.clone(),
                            snapshot_token,
                            revision,
                            units: next,
                            axis: None,
                        });
                    }
                },
            }
            ToggleField {
                label: "Rotate 90°".to_owned(),
                checked: !mixed && draft().y > draft().x,
                aria_label: "Rotate keycap",
                disabled_reason: (!props.mount.editable).then(|| "Placement is unavailable".to_owned()),
                on_commit: {
                    let owner = event_owner.clone();
                    let generation = timer_generation.clone();
                    move |checked: bool| {
                        generation.set(generation.get().wrapping_add(1));
                        let current = draft();
                        let long = current.x.max(current.y);
                        let short = current.x.min(current.y);
                        let next = if checked {
                            boardstudio_model::model::Vec2 { x: short, y: long }
                        } else {
                            boardstudio_model::model::Vec2 { x: long, y: short }
                        };
                        draft_dirty.set(true);
                        draft.set(next);
                        submit.call(ResizeIntent {
                            owner: owner.clone(),
                            snapshot_token,
                            revision,
                            units: next,
                            axis: None,
                        });
                    }
                },
            }
            if let Some(error) = error {
                FieldError { "{error}"  }
            }
        }
        if let Some(warning) = overlap_warning {
            div { role: "status", class: "-mt-1 mx-4 mb-3 rounded-[4px] border border-warning-dark bg-warning-light px-2.5 py-2 text-[14px] leading-[1.55] text-warning-dark", Callout { variant: CalloutVariant::Warning, "{warning}" } }
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
