//! BoardStudio's small, controlled wrappers around shared browser controls.
//!
//! These wrappers keep native button semantics and centralize the visual
//! vocabulary without owning the state of the consuming workspace.

use std::{cell::Cell, rc::Rc};

use dioxus::prelude::*;
use dioxus_primitives::{
    popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    toggle::Toggle,
    toolbar::{Toolbar, ToolbarButton},
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

/// A labeled group of direct actions with the pinned primitive's arrow-key navigation.
#[component]
pub fn BoardToolbar(label: String, #[props(default)] class: String, children: Element) -> Element {
    rsx! { Toolbar { aria_label: label, class: "board-toolbar {class}", {children} } }
}

/// A toolbar action. The consuming workspace retains tool state and edit callbacks.
#[component]
pub fn BoardToolbarButton(
    index: usize,
    #[props(default)] disabled: bool,
    #[props(default)] pressed: Option<bool>,
    #[props(default)] class: String,
    aria_label: String,
    #[props(default)] title: Option<String>,
    on_activate: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        ToolbarButton {
            index, disabled,
            class: "board-button board-button--compact {class}",
            aria_label, aria_pressed: pressed, title,
            on_click: move |_| on_activate.call(()),
            {children}
        }
    }
}

#[component]
pub fn BoardButton(
    #[props(default = "button".to_owned())] r#type: String,
    #[props(default)] variant: BoardButtonVariant,
    #[props(default)] density: BoardButtonDensity,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
    #[props(default)] style: String,
    #[props(default)] pressed: Option<bool>,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] title: Option<String>,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(extends = button)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let variant_class = match variant {
        BoardButtonVariant::Primary => "board-button--primary",
        BoardButtonVariant::Secondary => "",
        BoardButtonVariant::Quiet => "board-button--quiet",
        BoardButtonVariant::Danger => "board-button--danger",
    };
    let density_class = match density {
        BoardButtonDensity::Comfortable => "",
        BoardButtonDensity::Compact => "board-button--compact",
    };
    rsx! {
        button {
            r#type,
            class: "board-button {variant_class} {density_class} {class}",
            style,
            disabled,
            aria_pressed: pressed,
            aria_label,
            title,
            onclick,
            ..attributes,
            {children}
        }
    }
}

/// A native controlled checkbox. Form events and label activation stay native.
#[component]
pub fn BoardCheckbox(
    checked: bool,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] role: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
    #[props(default)] onchange: EventHandler<FormEvent>,
    #[props(extends = input)] attributes: Vec<Attribute>,
) -> Element {
    rsx! {
        input {
            r#type: "checkbox", class: "board-checkbox {class}",
            checked, disabled, onchange, aria_label, role,
            ..attributes,
        }
    }
}

/// Inline sidebar section using the same keyboard-accessible accordion primitive as setup.
#[component]
pub fn BoardAccordionSection(
    class: String,
    label: String,
    id: String,
    #[props(default)] initially_open: bool,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
    heading: Element,
    children: Element,
) -> Element {
    use dioxus_primitives::accordion::{
        Accordion, AccordionContent, AccordionItem, AccordionTrigger,
    };
    rsx! {
        Accordion {
            attributes: vec![dioxus::events::onkeydown(move |event| onkeydown.call(event))],
            class: "board-accordion {class}",
            AccordionItem { index: 0usize, default_open: initially_open,
                AccordionTrigger { id, class: "board-accordion-trigger", aria_label: label,
                    {heading}
                    svg { class: "board-accordion-chevron", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m5 8 5 5 5-5" } }
                }
                AccordionContent { class: "board-accordion-content", {children} }
            }
        }
    }
}

/// A shared disclosure with native keyboard handling and persistent child drafts.
#[component]
pub fn BoardDisclosure(
    #[props(default)] open: bool,
    #[props(default)] class: String,
    heading: Element,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        details { class: "board-disclosure {class}", open,
            ..attributes,
            {heading}
            {children}
        }
    }
}

/// Shared modal confirmation or non-modal inline task surface.
/// Native modal dialogs provide focus trapping, Escape and focus restoration.
#[component]
pub fn BoardDialog(
    open: bool,
    #[props(default = true)] modal: bool,
    #[props(default = true)] dismissible: bool,
    #[props(default)] class: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] aria_labelledby: Option<String>,
    #[props(default)] aria_describedby: Option<String>,
    #[props(default)] on_close: EventHandler<()>,
    children: Element,
) -> Element {
    let id = format!("board-dialog-{}", dioxus::core::current_scope_id().0);
    let effect_id = id.clone();
    use_effect(use_reactive((&open, &modal), move |(open, modal)| {
        if !modal {
            return;
        }
        let Some(dialog) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(&effect_id))
            .and_then(|element| element.dyn_into::<web_sys::HtmlDialogElement>().ok())
        else {
            return;
        };
        if open && !dialog.open() {
            let _ = dialog.show_modal();
        } else if !open && dialog.open() {
            dialog.close();
        }
    }));
    rsx! {
        if modal {
            dialog {
                id, class: "board-dialog {class}",
                aria_label, aria_labelledby, aria_describedby,
                onkeydown: move |event: KeyboardEvent| event.stop_propagation(),
                oncancel: move |event| {
                    event.prevent_default();
                    if dismissible { on_close.call(()); }
                },
                {children}
            }
        } else if open {
            div {
                id, role: "dialog", class: "board-dialog-inline {class}",
                aria_label, aria_labelledby, aria_describedby,
                onkeydown: move |event: KeyboardEvent| {
                    if event.key() == Key::Escape && dismissible {
                        event.prevent_default(); event.stop_propagation(); on_close.call(());
                    }
                },
                {children}
            }
        }
    }
}

/// A controlled numeric draft. Parsing and committing remain with the owner.
#[component]
pub fn BoardNumberField(
    label: String,
    value: String,
    #[props(default)] unit: String,
    #[props(default)] readonly: bool,
    #[props(default)] disabled: bool,
    #[props(default)] min: Option<String>,
    #[props(default)] max: Option<String>,
    #[props(default = "any".to_owned())] step: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] class: String,
    #[props(default)] oninput: EventHandler<FormEvent>,
    #[props(default)] onfocus: EventHandler<FocusEvent>,
    #[props(default)] onblur: EventHandler<FocusEvent>,
    #[props(default)] onpointerenter: EventHandler<PointerEvent>,
    #[props(default)] onpointerleave: EventHandler<PointerEvent>,
    #[props(default)] onkeydown: EventHandler<KeyboardEvent>,
) -> Element {
    let scope = dioxus::core::current_scope_id().0;
    let unit_id = format!("board-number-{scope}-unit");
    let error_id = format!("board-number-{scope}-error");
    let described_by = match (unit.is_empty(), error.is_some()) {
        (false, true) => Some(format!("{unit_id} {error_id}")),
        (false, false) => Some(unit_id.clone()),
        (true, true) => Some(error_id.clone()),
        (true, false) => None,
    };
    let error_class = if error.is_some() { "has-error" } else { "" };
    rsx! {
        label { class: "board-field grid gap-1 {error_class} {class}",
            span { "{label}" }
            span { class: "board-field-input flex items-center gap-1.5 min-w-0",
                input {
                    class: "board-field-control min-w-0 w-full tabular-nums",
                    r#type: "number", step, min, max, value, readonly, disabled,
                    aria_label: aria_label.unwrap_or(label),
                    aria_invalid: error.is_some(),
                    aria_describedby: described_by,
                    oninput, onfocus, onblur, onpointerenter, onpointerleave, onkeydown,
                }
                if !unit.is_empty() { small { id: unit_id, "{unit}" } }
            }
            if let Some(message) = error.as_deref() { small { id: error_id, role: "alert", "{message}" } }
        }
    }
}

/// A labeled native select with shared styling and associated validation feedback.
#[component]
pub fn BoardSelectField(
    label: String,
    value: String,
    #[props(default)] aria_label: Option<String>,
    #[props(default)] disabled: bool,
    #[props(default)] error: Option<String>,
    #[props(default)] class: String,
    #[props(default)] onchange: EventHandler<FormEvent>,
    children: Element,
) -> Element {
    let error_id = format!("board-select-{}-error", dioxus::core::current_scope_id().0);
    let described_by = error.as_ref().map(|_| error_id.clone());
    let error_class = if error.is_some() { "has-error" } else { "" };
    rsx! {
        label { class: "board-field grid gap-1 {error_class} {class}",
            span { "{label}" }
            select {
                class: "board-field-control min-w-0 w-full",
                value, disabled,
                aria_label: aria_label.unwrap_or(label),
                aria_invalid: error.is_some(),
                aria_describedby: described_by,
                onchange,
                {children}
            }
            if let Some(message) = error.as_deref() { small { id: error_id, role: "alert", "{message}" } }
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
    #[props(default)] toolbar_index: Option<usize>,
    #[props(default)] pressed: Option<bool>,
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
            if let Some(index) = toolbar_index {
                ToolbarButton {
                    index,
                    id: trigger_id,
                    class: "board-popover-trigger {trigger_class}",
                    title: trigger_label.clone(),
                    aria_label: trigger_label,
                    aria_pressed: pressed,
                    aria_expanded: "{open}",
                    aria_controls: content_id.clone(),
                    on_click: move |_| {
                        // Switching toolbar popovers must not restore the previous trigger's focus.
                        restore_focus_on_close.set(false);
                        escape_pressed.set(false);
                        on_open_change.call(!open);
                    },
                    {trigger}
                }
            } else {
            PopoverTrigger {
                id: trigger_id,
                class: "board-popover-trigger {trigger_class}",
                title: trigger_label.clone(),
                aria_label: trigger_label,
                aria_expanded: "{open}",
                aria_controls: content_id.clone(),
                {trigger}
            }
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
            BoardNumberField {
                class: "board-controls-number",
                label: "Width",
                value: "3",
                min: Some("0".to_owned()),
                max: Some("10".to_owned()),
                step: "0.5",
                disabled: true,
                error: Some("Enter 0 or greater.".to_owned()),
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
    async fn number_field_exposes_bounds_step_disabled_and_associated_error() {
        mount().await;
        let input = element(".board-controls-number input");
        for (name, expected) in [
            ("min", "0"),
            ("max", "10"),
            ("step", "0.5"),
            ("aria-invalid", "true"),
        ] {
            assert_eq!(
                input.get_attribute(name).as_deref(),
                Some(expected),
                "{name}"
            );
        }
        assert!(input.has_attribute("disabled"));
        let error = element(".board-controls-number small[role='alert']");
        assert_eq!(
            input.get_attribute("aria-describedby"),
            error.get_attribute("id")
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

#[cfg(test)]
mod native_control_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[component]
    fn fixture() -> Element {
        let mut checked = use_signal(|| false);
        let mut changes = use_signal(|| 0_u32);
        let mut submissions = use_signal(|| 0_u32);
        let mut open = use_signal(|| false);
        let mut busy = use_signal(|| false);
        rsx! {
            label { id: "shared-checkbox-label",
                BoardCheckbox { checked: checked(), aria_label: "Shared enabled",
                    onchange: move |event: FormEvent| { checked.set(event.checked()); changes.set(changes() + 1); },
                }
                "Enabled"
            }
            label {
                BoardCheckbox { checked: false, disabled: true, aria_label: "Shared disabled",
                    onchange: move |_| changes.set(changes() + 100),
                }
                "Disabled"
            }
            output { id: "shared-checkbox-changes", "{changes()}" }
            BoardDisclosure {
                class: "shared-disclosure",
                heading: rsx! { summary { "Draft settings" } },
                input { aria_label: "Persistent draft", value: "original" }
            }
            form { onsubmit: move |event: FormEvent| { event.prevent_default(); submissions.set(submissions() + 1); },
                BoardButton { r#type: "submit", class: "shared-submit", "Save" }
                BoardButton { r#type: "submit", class: "shared-disabled-submit", disabled: true, "Unavailable save" }
            }
            output { id: "shared-submissions", "{submissions()}" }
            button { id: "shared-dialog-trigger", onclick: move |_| open.set(true), "Open confirmation" }
            BoardDialog { open: open(), dismissible: !busy(), class: "shared-dialog",
                aria_labelledby: "shared-dialog-title", on_close: move |_| open.set(false),
                h2 { id: "shared-dialog-title", "Confirm action" }
                label {
                    BoardCheckbox { checked: busy(), aria_label: "Busy confirmation",
                        onchange: move |event: FormEvent| busy.set(event.checked()),
                    }
                    "Busy"
                }
                BoardButton { class: "shared-dialog-cancel", autofocus: true, disabled: busy(),
                    onclick: move |_| open.set(false), "Cancel"
                }
            }
        }
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(60).await;
    }

    async fn mount() -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        root
    }

    fn element(root: &web_sys::Element, selector: &str) -> web_sys::HtmlElement {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn checkbox_label_activates_once_and_disclosure_keeps_its_draft() {
        let root = mount().await;
        let disclosure = element(&root, ".shared-disclosure");
        element(&root, ".shared-disclosure > summary").click();
        assert!(disclosure.has_attribute("open"));
        let input: web_sys::HtmlInputElement =
            element(&root, "input[aria-label='Persistent draft']")
                .dyn_into()
                .unwrap();
        input.set_value("unfinished draft");
        element(&root, "#shared-checkbox-label").click();
        settle().await;
        assert_eq!(
            element(&root, "#shared-checkbox-changes")
                .text_content()
                .as_deref(),
            Some("1")
        );
        assert!(
            disclosure.has_attribute("open"),
            "an unrelated render must not collapse the section"
        );
        element(&root, "input[aria-label='Shared disabled']").click();
        element(&root, ".shared-disclosure > summary").click();
        assert!(!disclosure.has_attribute("open"));
        element(&root, ".shared-disclosure > summary").click();
        assert_eq!(input.value(), "unfinished draft");
        assert_eq!(
            element(&root, "#shared-checkbox-changes")
                .text_content()
                .as_deref(),
            Some("1")
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn action_buttons_preserve_submit_and_disabled_semantics() {
        let root = mount().await;
        element(&root, ".shared-disabled-submit").click();
        element(&root, ".shared-submit").click();
        settle().await;
        assert_eq!(
            element(&root, "#shared-submissions")
                .text_content()
                .as_deref(),
            Some("1")
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn confirmation_focus_and_busy_cancellation_stay_native() {
        let root = mount().await;
        let trigger = element(&root, "#shared-dialog-trigger");
        trigger.focus().unwrap();
        trigger.click();
        settle().await;
        let dialog: web_sys::HtmlDialogElement =
            element(&root, ".shared-dialog").dyn_into().unwrap();
        assert!(dialog.open());
        let document = web_sys::window().unwrap().document().unwrap();
        assert!(
            document
                .active_element()
                .unwrap()
                .class_name()
                .contains("shared-dialog-cancel")
        );
        element(&root, "input[aria-label='Busy confirmation']").click();
        settle().await;
        let init = web_sys::EventInit::new();
        init.set_cancelable(true);
        dialog
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("cancel", &init).unwrap())
            .unwrap();
        settle().await;
        assert!(dialog.open(), "a pending action cannot be dismissed");
        element(&root, "input[aria-label='Busy confirmation']").click();
        settle().await;
        dialog
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("cancel", &init).unwrap())
            .unwrap();
        settle().await;
        assert!(!dialog.open());
        assert_eq!(
            document.active_element().unwrap().id(),
            "shared-dialog-trigger"
        );
        root.remove();
    }
}
