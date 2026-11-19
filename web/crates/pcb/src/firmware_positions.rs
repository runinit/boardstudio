//! Private presentation for the legacy PCB firmware-position map.
//!
//! The selected-board plan and bindings are projected by F5; this component only renders that
//! accepted value and turns a select change into the existing root-owned edit request.
use super::pcb_wiring::{
    FirmwarePositionEditRequest, FirmwarePositionFeedback, FirmwarePositionFeedbackState,
};
use crate::firmware_position_choices;
use crate::firmware_position_projection::{FirmwarePositionProjection, FirmwarePositionState};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::combobox::{Combobox, ComboboxOption};
use boardstudio_web_ui_shared::ui::field::{Field, FieldLabel};
use boardstudio_web_ui_shared::ui::spinner::Spinner;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct FirmwareKeymapPanelProps {
    pub projection: FirmwarePositionProjection,
    pub feedback: Option<FirmwarePositionFeedback>,
    pub editable: bool,
    pub on_change: EventHandler<FirmwarePositionEditRequest>,
}

#[component]
pub fn FirmwareKeymapPanel(props: FirmwareKeymapPanelProps) -> Element {
    let mut open = use_signal(|| false);
    let keys = props.projection.keys.as_ref();
    let assigned = keys
        .iter()
        .filter(|key| {
            props
                .projection
                .bindings
                .get(&key.id)
                .is_some_and(|binding| binding != "&none")
        })
        .count();
    let ready = matches!(&props.projection.state, FirmwarePositionState::Current)
        && props.projection.identity.is_some();
    let disabled = !ready || !props.editable;

    let content = match &props.projection.state {
        FirmwarePositionState::Idle => rsx! {
            p { class: "firmware-keymap-message", role: "status",
                "Resolve a current wiring plan to edit firmware positions."
            }
        },
        FirmwarePositionState::Pending => rsx! {
            p { class: "firmware-keymap-message", role: "status", Spinner {} "Loading firmware positions…" }
        },
        FirmwarePositionState::Failed(message) => rsx! {
            Alert { variant: AlertVariant::Destructive, "Could not load firmware positions: {message}" }
        },
        FirmwarePositionState::Current if keys.is_empty() => rsx! {
            p { class: "firmware-keymap-message", role: "status", "The current plan has no editable key positions." }
        },
        FirmwarePositionState::Current => {
            let on_change = props.on_change;
            rsx! {
                div { class: "firmware-keymap-body",
                    div { class: "firmware-keymap-grid",
                        for key in keys {
                            {
                                let key_id = key.id.clone();
                                let label = key.label.clone();
                                let id = format!("firmware-key-{}", key.id);
                                let identity = props.projection.identity.clone();
                                let accepted_value = props.projection.bindings
                                    .get(&key.id)
                                    .map_or("&none", String::as_str)
                                    .to_owned();
                                let value = identity.as_ref().and_then(|identity| super::pcb_wiring::pending_binding(identity, &key.id)).unwrap_or(accepted_value);
                                let choices = firmware_position_choices::choices();
                                rsx! {
                                    FirmwareChoice { key: "{key_id}", id, label: label.clone(), value, choices, disabled,
                                            on_change: move |binding: String| {
                                                if let Some(identity) = identity.clone() {
                                                    on_change.call(FirmwarePositionEditRequest {
                                                        identity,
                                                        key_id: key_id.clone(),
                                                        binding,
                                                    });
                                                }
                                            }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "firmware-keymap-preview", aria_label: "Firmware keymap preview",
                        for key in keys {
                            {
                                let key_id = key.id.clone();
                                let label = key.label.clone();
                                let binding = props.projection.bindings.get(&key.id).map_or("&none", String::as_str).to_owned();
                                rsx! {
                                    div { class: "firmware-keymap-preview-row", key: "{key_id}",
                                        span { "{label}" }
                                        code { "{binding}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let feedback = props
        .feedback
        .as_ref()
        .map(|feedback| match &feedback.state {
            FirmwarePositionFeedbackState::Pending => rsx! {
                p { class: "firmware-keymap-feedback", role: "status", Spinner {} "Saving firmware position…" }
            },
            FirmwarePositionFeedbackState::Failed(message) => rsx! {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            },
        });

    rsx! {
        Collapsible { class: "firmware-keymap-panel", open: open(), on_open_change: move |next| open.set(next), keep_mounted: true,
            CollapsibleTrigger { aria_label: "Firmware keymap {assigned}/{keys.len()} assigned", class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                span { "Firmware keymap" }
                span { class: "firmware-keymap-summary", "{assigned}/{keys.len()} assigned" }
            }
            CollapsibleContent { hidden: !open(), inert: (!open()).then_some(""), {content} {feedback} }
        }
    }
}

#[component]
fn FirmwareChoice(
    id: String,
    label: String,
    value: String,
    choices: Vec<(String, String)>,
    disabled: bool,
    on_change: EventHandler<String>,
) -> Element {
    let selected = use_memo(use_reactive((&value,), |(value,)| Some(value)));
    let name = format!("Binding for {label}");
    rsx! {
        Field { class: "firmware-keymap-row", FieldLabel { html_for: id.clone(), "{label}" }
            Combobox::<String> { id, aria_label: name.clone(), list_aria_label: name, value: Some(selected.into()), disabled,
                class: "max-[960px]:[&_[data-slot=combobox-input]]:min-h-control-touch [@media(pointer:coarse)]:[&_[data-slot=combobox-input]]:min-h-control-touch",
                on_value_change: move |value: Option<String>| if let Some(value) = value { on_change.call(value); },
                for (index, (value, label)) in choices.into_iter().enumerate() {
                    ComboboxOption::<String> { index, value, text_value: label.clone(), class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{label}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod mounted_tests {
    use super::*;
    use crate::firmware_position_projection::{
        FirmwarePlanIdentity, FirmwarePositionIdentity, FirmwarePositionKey,
    };
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    #[derive(Clone)]
    struct Capture(Rc<RefCell<Option<FirmwarePositionEditRequest>>>);

    fn fixture() -> Element {
        let capture = use_context::<Capture>();
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "document".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let identity = FirmwarePositionIdentity {
            plan: FirmwarePlanIdentity {
                scope: scope.clone(),
                token: SnapshotToken(1),
                revision: 1,
                executor_epoch: 1,
            },
            ui_scope: scope,
            scope_generation: 1,
        };
        let projection = FirmwarePositionProjection {
            identity: Some(identity),
            state: FirmwarePositionState::Current,
            keys: vec![FirmwarePositionKey {
                id: "key-1".into(),
                label: "SW1".into(),
            }]
            .into(),
            bindings: Rc::new(BTreeMap::new()),
        };
        rsx! { FirmwareKeymapPanel { projection, editable: true, on_change: move |edit| *capture.0.borrow_mut() = Some(edit) } }
    }

    #[wasm_bindgen_test]
    async fn firmware_binding_picker_filters_and_submits_by_keyboard() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let capture = Rc::new(RefCell::new(None));
        let dom = VirtualDom::new(fixture);
        dom.provide_root_context(Capture(capture.clone()));
        let _mounted = MountedFixture::new(root.clone(), dom);
        wait_until("firmware disclosure mounts", || {
            by_role_name(&root, "button", "Firmware keymap 0/1 assigned").is_some()
        })
        .await;
        by_role_name(&root, "button", "Firmware keymap 0/1 assigned")
            .unwrap()
            .click();
        wait_until("binding picker mounts", || {
            by_role_name(&root, "combobox", "Binding for SW1").is_some()
        })
        .await;
        let picker = by_role_name(&root, "combobox", "Binding for SW1").unwrap();
        picker.focus().unwrap();
        let key = |value: &str| {
            let event = web_sys::KeyboardEventInit::new();
            event.set_key(value);
            event.set_bubbles(true);
            event.set_cancelable(true);
            picker
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                        .unwrap(),
                )
                .unwrap();
        };
        key("ArrowDown");
        let input: web_sys::HtmlInputElement = picker.clone().dyn_into().unwrap();
        input.set_value("F12");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        wait_until("binding options filter", || {
            by_role_name(&root, "option", "F12").is_some()
                && by_role_name(&root, "option", "F1").is_none()
        })
        .await;
        key("ArrowDown");
        wait_until("matching binding is active", || {
            by_role_name(&root, "option", "F12").is_some_and(|option| {
                picker.get_attribute("aria-activedescendant").as_deref()
                    == Some(option.id().as_str())
            })
        })
        .await;
        key("Enter");
        wait_until("binding is submitted", || {
            capture
                .borrow()
                .as_ref()
                .is_some_and(|edit: &FirmwarePositionEditRequest| {
                    edit.key_id == "key-1" && edit.binding == "&kp F12"
                })
        })
        .await;
        root.remove();
    }
}
