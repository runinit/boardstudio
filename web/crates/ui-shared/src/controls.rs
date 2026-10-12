//! BoardStudio's small, controlled wrappers around shared browser controls.
//!
//! These wrappers keep native button semantics and centralize the visual
//! vocabulary without owning the state of the consuming workspace.

use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;
use dioxus_primitives::{
    popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    toggle::Toggle,
};
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardButtonVariant {
    Primary,
    #[default]
    Secondary,
    Quiet,
    Danger,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardButtonDensity {
    #[default]
    Comfortable,
    Compact,
}

#[component]
pub fn BoardButton(
    #[props(default)] variant: BoardButtonVariant,
    #[props(default)] density: BoardButtonDensity,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
    #[props(default)] style: String,
    #[props(default)] pressed: Option<bool>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let variant_class = match variant {
        BoardButtonVariant::Primary => "board-button--primary",
        BoardButtonVariant::Secondary => "board-button--secondary",
        BoardButtonVariant::Quiet => "board-button--quiet",
        BoardButtonVariant::Danger => "board-button--danger",
    };
    let density_class = match density {
        BoardButtonDensity::Comfortable => "board-button--comfortable",
        BoardButtonDensity::Compact => "board-button--compact",
    };
    rsx! {
        button {
            r#type: "button",
            class: "board-button {variant_class} {density_class} {class}",
            style,
            disabled,
            aria_pressed: pressed,
            aria_label,
            title,
            onclick,
            {children}
        }
    }
}

/// A controlled toggle; the owner remains the only source of its pressed state.
#[component]
pub fn BoardToggle(
    pressed: bool,
    #[props(default)] disabled: bool,
    aria_label: String,
    #[props(default)] title: Option<String>,
    #[props(default)] class: String,
    #[props(default)] style: String,
    #[props(default)] on_pressed_change: EventHandler<bool>,
    children: Element,
) -> Element {
    rsx! {
        Toggle {
            pressed,
            disabled,
            aria_label,
            title,
            class: "board-toggle {class}",
            style,
            on_pressed_change: move |next| on_pressed_change.call(next),
            {children}
        }
    }
}

/// A controlled non-modal popover. Trigger and content remain owned by the caller.
#[component]
pub fn BoardPopover(
    open: bool,
    on_open_change: EventHandler<bool>,
    trigger_id: String,
    content_id: String,
    trigger_label: String,
    #[props(default)] root_class: String,
    #[props(default)] menu_kind: Option<String>,
    #[props(default)] trigger_class: String,
    #[props(default)] content_class: String,
    #[props(default)] content_role: String,
    #[props(default)] content_label: Option<String>,
    trigger: Element,
    children: Element,
) -> Element {
    let trigger_for_focus = trigger_id.clone();
    let content_for_focus = content_id.clone();
    let content_label_id = content_label
        .as_ref()
        .map(|_| format!("{content_id}-label"));
    let content_label_text = content_label.clone();
    let mut previous_open = use_signal(|| open);
    let mut restore_focus_on_close = use_signal(|| true);
    let mut escape_pressed = use_signal(|| false);
    let restore_version = use_hook(|| Rc::new(Cell::new(0_u32)));
    let restore_version_for_effect = restore_version.clone();
    let restore_version_for_callback = restore_version.clone();
    let escape_keydown = onkeydown(move |event: KeyboardEvent| {
        if event.key().to_string() == "Escape" {
            escape_pressed.set(true);
        }
    });
    use_effect(use_reactive!(|open| {
        let was_open = previous_open();
        let restore = restore_focus_on_close();
        let version = restore_version_for_effect.get();
        if was_open != open {
            if was_open && !open && restore {
                restore_popover_focus(
                    &trigger_for_focus,
                    &content_for_focus,
                    restore_focus_on_close,
                    restore_version_for_effect.clone(),
                    restore,
                    version,
                );
            }
            previous_open.set(open);
        }
    }));
    rsx! {
        PopoverRoot {
            open,
            is_modal: false,
            class: "board-popover {root_class}",
            "data-layout-menu": menu_kind,
            attributes: vec![escape_keydown],
            on_open_change: move |next| {
                let escape_was_pressed = escape_pressed();
                restore_version_for_callback
                    .set(restore_version_for_callback.get() + 1);
                if next {
                    restore_focus_on_close.set(true);
                    escape_pressed.set(false);
                } else {
                    restore_focus_on_close.set(escape_was_pressed);
                    escape_pressed.set(false);
                }
                on_open_change.call(next);
            },
            PopoverTrigger {
                id: trigger_id,
                class: "board-popover-trigger {trigger_class}",
                aria_label: trigger_label,
                aria_expanded: "{open}",
                aria_controls: content_id.clone(),
                {trigger}
            }
            PopoverContent {
                id: content_id,
                class: "board-popover-content {content_class}",
                role: if content_role.is_empty() { None } else { Some(content_role) },
                aria_label: content_label,
                aria_labelledby: content_label_id.clone(),
                if let (Some(label_id), Some(label_text)) = (content_label_id, content_label_text) {
                    span { id: label_id, class: "sr-only", "{label_text}" }
                }
                {children}
            }
        }
    }
}

fn restore_popover_focus(
    trigger_id: &str,
    content_id: &str,
    restore_focus_on_close: Signal<bool>,
    restore_version: Rc<Cell<u32>>,
    queued_restore: bool,
    queued_version: u32,
) {
    let trigger_id = trigger_id.to_owned();
    let content_id = content_id.to_owned();
    spawn(async move {
        gloo_timers::future::TimeoutFuture::new(0).await;
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let active = document.active_element();
        let focus_was_inside = active
            .as_ref()
            .and_then(|active| {
                document
                    .get_element_by_id(&content_id)
                    .map(|content| (active, content))
            })
            .is_some_and(|(active, content)| {
                let Ok(active) = active.clone().dyn_into::<web_sys::Node>() else {
                    return false;
                };
                let Ok(content) = content.dyn_into::<web_sys::Node>() else {
                    return false;
                };
                content.contains(Some(&active))
            });
        let active_is_body = active
            .as_ref()
            .and_then(|active| active.dyn_ref::<web_sys::HtmlElement>())
            .is_some_and(|active| active.tag_name().eq_ignore_ascii_case("body"));
        let restore_now = restore_focus_on_close();
        let version_now = restore_version.get();
        let restore_is_current = queued_restore && restore_now && queued_version == version_now;
        let should_focus =
            restore_is_current && (focus_was_inside || active_is_body || active.is_none());
        let trigger = should_focus
            .then(|| {
                document
                    .get_element_by_id(&trigger_id)
                    .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            })
            .flatten();
        if let Some(trigger) = trigger {
            let _ = trigger.focus();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{Event as WebEvent, EventInit, KeyboardEvent, KeyboardEventInit};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn controls_fixture() -> Element {
        let mut open = use_signal(|| false);
        let mut selected = use_signal(|| 0_u32);
        let mut pressed = use_signal(|| false);
        let mut button_activations = use_signal(|| 0_u32);
        let mut disabled_changes = use_signal(|| 0_u32);
        rsx! {
            BoardPopover {
                open: open(),
                on_open_change: move |next| open.set(next),
                trigger_id: "board-controls-trigger",
                content_id: "board-controls-content",
                trigger_label: "Choose layer".to_owned(),
                root_class: "board-controls-popover",
                content_role: "group",
                content_label: Some("Layer choices".to_owned()),
                trigger: rsx! { span { "Choose layer" } },
                div {
                    button {
                        id: "board-controls-select",
                        r#type: "button",
                        onclick: move |_| {
                            selected.set(selected() + 1);
                            open.set(false);
                        },
                        "Select"
                    }
                }
            }
            button { id: "board-controls-outside", r#type: "button", "Outside target" }
            div { id: "board-controls-nonfocusable-outside", "Non-focusable outside target" }
            BoardToggle {
                pressed: pressed(),
                aria_label: "Visibility".to_owned(),
                class: "board-controls-toggle",
                on_pressed_change: move |next| pressed.set(next),
                "Visible"
            }
            BoardToggle {
                pressed: false,
                disabled: true,
                aria_label: "Unavailable visibility".to_owned(),
                class: "board-controls-disabled-toggle",
                on_pressed_change: move |_| disabled_changes.set(disabled_changes() + 1),
                "Unavailable"
            }
            BoardButton {
                class: "board-controls-selection",
                pressed: Some(button_activations() > 0),
                aria_label: Some("Select option".to_owned()),
                onclick: move |_| button_activations.set(button_activations() + 1),
                "Option"
            }
            output { id: "board-controls-selected", "{selected()}" }
            output { id: "board-controls-pressed", "{pressed()}" }
            output { id: "board-controls-button-activations", "{button_activations()}" }
            output { id: "board-controls-disabled-changes", "{disabled_changes()}" }
        }
    }

    fn root() -> web_sys::Element {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("board-controls-test-root")
            .expect("controls fixture root")
    }

    fn element(selector: &str) -> web_sys::Element {
        root()
            .query_selector(selector)
            .unwrap()
            .expect("fixture element")
    }

    fn button(selector: &str) -> web_sys::HtmlElement {
        element(selector).dyn_into().unwrap()
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(60).await;
    }

    async fn mount() {
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(previous) = document.get_element_by_id("board-controls-test-root") {
            previous.remove();
        }
        let mount = document.create_element("div").unwrap();
        mount.set_id("board-controls-test-root");
        document.body().unwrap().append_child(&mount).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(controls_fixture),
            dioxus_web::Config::new().rootnode(mount.into()),
        );
        rendered().await;
        rendered().await;
    }

    #[wasm_bindgen_test]
    async fn controlled_toggle_activates_once_and_keeps_disabled_state_truthful() {
        mount().await;
        let toggle = element(".board-controls-toggle");
        assert_eq!(
            toggle.get_attribute("aria-pressed").as_deref(),
            Some("false")
        );
        button(".board-controls-toggle").click();
        rendered().await;
        assert_eq!(
            element("#board-controls-pressed").text_content().as_deref(),
            Some("true")
        );
        assert_eq!(
            element(".board-controls-toggle")
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );

        let selection = element(".board-controls-selection");
        assert_eq!(selection.tag_name(), "BUTTON");
        assert_eq!(selection.get_attribute("type").as_deref(), Some("button"));
        button(".board-controls-selection").click();
        rendered().await;
        assert_eq!(
            element("#board-controls-button-activations")
                .text_content()
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            element(".board-controls-selection")
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );

        let disabled = element(".board-controls-disabled-toggle");
        assert!(disabled.has_attribute("disabled"));
        button(".board-controls-disabled-toggle").click();
        rendered().await;
        assert_eq!(
            element("#board-controls-disabled-changes")
                .text_content()
                .as_deref(),
            Some("0")
        );
        root().remove();
    }

    #[wasm_bindgen_test]
    async fn popover_preserves_group_semantics_selection_focus_and_outside_focus() {
        mount().await;
        let trigger = button("#board-controls-trigger");
        assert_eq!(
            trigger.get_attribute("aria-controls").as_deref(),
            Some("board-controls-content")
        );
        trigger.click();
        rendered().await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        let content = element("#board-controls-content");
        assert_eq!(content.get_attribute("role").as_deref(), Some("group"));
        assert_eq!(
            content.get_attribute("aria-label").as_deref(),
            Some("Layer choices")
        );
        let labelledby = content.get_attribute("aria-labelledby").unwrap();
        assert_eq!(labelledby, "board-controls-content-label");
        assert_eq!(
            element("#board-controls-content-label")
                .text_content()
                .as_deref(),
            Some("Layer choices"),
            "the explicit group name must resolve without inheriting the trigger label"
        );

        button("#board-controls-select").click();
        rendered().await;
        assert_eq!(
            element("#board-controls-selected")
                .text_content()
                .as_deref(),
            Some("1")
        );
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .id(),
            "board-controls-trigger"
        );

        trigger.click();
        rendered().await;
        let escape = KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap();
        element("#board-controls-select")
            .dispatch_event(&event)
            .unwrap();
        rendered().await;
        rendered().await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .id(),
            "board-controls-trigger"
        );

        trigger.click();
        rendered().await;
        let outside = button("#board-controls-outside");
        outside.focus().unwrap();
        let pointer = EventInit::new();
        pointer.set_bubbles(true);
        element("#board-controls-outside")
            .dispatch_event(&WebEvent::new_with_event_init_dict("pointerdown", &pointer).unwrap())
            .unwrap();
        rendered().await;
        rendered().await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .id(),
            "board-controls-outside"
        );

        trigger.click();
        rendered().await;
        let document = web_sys::window().unwrap().document().unwrap();
        document
            .active_element()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .blur()
            .unwrap();
        let pointer = EventInit::new();
        pointer.set_bubbles(true);
        element("#board-controls-nonfocusable-outside")
            .dispatch_event(&WebEvent::new_with_event_init_dict("pointerdown", &pointer).unwrap())
            .unwrap();
        rendered().await;
        rendered().await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_ne!(
            document.active_element().unwrap().id(),
            "board-controls-trigger",
            "outside dismissal must not move body focus back to the trigger"
        );
        root().remove();
    }
}
