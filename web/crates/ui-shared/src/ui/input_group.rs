//! Copied from rust-ui: `app_crates/registry/src/ui/input_group.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - built-in borders removed, cards style (theme contract section 8); focus rings kept; filled with `bg-field`, no shadow
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

use crate::ui::input::InputType;

#[component]
pub fn InputGroup(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "group/input-group bg-field relative flex w-full items-center rounded-md transition-[color,box-shadow] outline-none h-control min-w-0 has-[>textarea]:h-auto has-[>[data-align=inline-start]]:[&>input]:pl-2 has-[>[data-align=inline-end]]:[&>input]:pr-2 has-[>[data-align=block-start]]:h-auto has-[>[data-align=block-start]]:flex-col has-[>[data-align=block-start]]:[&>input]:pb-3 has-[>[data-align=block-end]]:h-auto has-[>[data-align=block-end]]:flex-col has-[>[data-align=block-end]]:[&>input]:pt-3 has-[[data-slot=input-group-control]:focus-visible]:border-ring has-[[data-slot=input-group-control]:focus-visible]:ring-ring/50 has-[[data-slot=input-group-control]:focus-visible]:ring-[3px] has-[[data-slot][aria-invalid=true]]:ring-destructive/20 has-[[data-slot][aria-invalid=true]]:border-destructive dark:has-[[data-slot][aria-invalid=true]]:ring-destructive/40",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { "data-name": "InputGroup", "data-slot": "input-group", role: "group", class: "{merged}",
            {children}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, strum::IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum InputGroupAddonAlign {
    #[default]
    InlineStart,
    InlineEnd,
    BlockStart,
    BlockEnd,
}

impl InputGroupAddonAlign {
    fn as_str(self) -> &'static str {
        self.into()
    }

    const fn class(self) -> &'static str {
        match self {
            Self::InlineStart => {
                "order-first pl-3 has-[>button]:ml-[-0.45rem] has-[>kbd]:ml-[-0.35rem]"
            }
            Self::InlineEnd => {
                "order-last pr-3 has-[>button]:mr-[-0.45rem] has-[>kbd]:mr-[-0.35rem]"
            }
            Self::BlockStart => {
                "order-first w-full justify-start px-3 pt-3 [.border-b]:pb-3 group-has-[>input]/input-group:pt-2.5"
            }
            Self::BlockEnd => {
                "order-last w-full justify-start px-3 pb-3 [.border-t]:pt-3 group-has-[>input]/input-group:pb-2.5"
            }
        }
    }
}

#[component]
pub fn InputGroupAddon(
    #[props(default = InputGroupAddonAlign::default())] align: InputGroupAddonAlign,
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "text-muted-foreground flex h-auto cursor-text items-center justify-center gap-2 py-1.5 text-sm font-medium select-none [&>svg:not([class*='size-'])]:size-4 [&>kbd]:rounded-[calc(var(--radius)-5px)] group-data-[disabled=true]/input-group:opacity-50",
        align.class(),
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            "data-name": "InputGroupAddon",
            "data-slot": "input-group-addon",
            "data-align": align.as_str(),
            role: "group",
            class: "{merged}",
            {children}
        }
    }
}

#[component]
pub fn InputGroupText(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "text-muted-foreground flex items-center gap-2 text-sm [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4",
        class.as_deref().unwrap_or("")
    );
    rsx! { span { "data-slot": "input-group-text", class: "{merged}", {children} } }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum InputGroupButtonVariant {
    #[default]
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum InputGroupButtonSize {
    Xs,
    #[default]
    Sm,
    IconXs,
    IconSm,
}

impl InputGroupButtonSize {
    const fn class(self) -> &'static str {
        match self {
            Self::Xs => {
                "h-6 gap-1 px-2 rounded-[calc(var(--radius)-5px)] [&>svg:not([class*='size-'])]:size-3.5 has-[>svg]:px-2"
            }
            Self::Sm => "h-control-compact px-2.5 gap-1.5 rounded-md has-[>svg]:px-2.5",
            Self::IconXs => "size-6 rounded-[calc(var(--radius)-5px)] p-0 has-[>svg]:p-0",
            Self::IconSm => "size-control-compact p-0 has-[>svg]:p-0",
        }
    }
}

#[component]
pub fn InputGroupButton(
    #[props(default = InputGroupButtonVariant::default())] variant: InputGroupButtonVariant,
    #[props(default = InputGroupButtonSize::default())] size: InputGroupButtonSize,
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let _ = variant;
    let merged = tw_merge!(
        "text-sm shadow-none flex gap-2 items-center",
        size.class(),
        class.as_deref().unwrap_or("")
    );
    rsx! {
        button { "data-slot": "input-group-button",
            r#type: "button",
            class: "{merged}",
            aria_label: aria_label,
            onclick: move |e| {
                if let Some(handler) = &onclick {
                    handler.call(e);
                }
            },
            {children}
        }
    }
}

/// Native input with extra global attributes/events. Explicit props own same-name attributes,
/// even when optional props are omitted; use those props for value, constraints and the label.
/// Blur/key/mount listeners and validation ARIA stay on this native input.
#[component]
pub fn InputGroupInput(
    #[props(into, optional)] class: Option<String>,
    #[props(default = InputType::default())] r#type: InputType,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] name: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] value: Option<String>,
    #[props(into, optional)] autocomplete: Option<String>,
    #[props(optional)] disabled: bool,
    #[props(optional)] readonly: bool,
    #[props(optional)] required: bool,
    #[props(into, optional)] step: Option<String>,
    #[props(into, optional)] min: Option<String>,
    #[props(into, optional)] max: Option<String>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(optional)] oninput: Option<EventHandler<FormEvent>>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let merged = tw_merge!(
        "flex-1 rounded-none border-0 bg-transparent shadow-none focus-visible:ring-0 dark:bg-transparent",
        "placeholder:text-muted-foreground h-control w-full min-w-0 px-3 py-1 text-base outline-none transition-[color,box-shadow]",
        "disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    // Explicit props, including omitted options, own their native attributes and input listener.
    // RSX requires the spread last; namespaced style properties and other events/ARIA pass through.
    let attributes = attributes
        .into_iter()
        .filter(|attribute| {
            attribute.namespace.is_some()
                || !matches!(
                    attribute.name,
                    "data-slot"
                        | "type"
                        | "class"
                        | "placeholder"
                        | "name"
                        | "id"
                        | "value"
                        | "autocomplete"
                        | "step"
                        | "min"
                        | "max"
                        | "aria-label"
                        | "disabled"
                        | "readonly"
                        | "required"
                        | "oninput"
                )
        })
        .collect::<Vec<_>>();
    rsx! {
        input {
            "data-slot": "input-group-control",
            r#type: r#type.as_str(),
            class: "{merged}",
            placeholder: placeholder,
            name: name,
            id: id,
            value: value,
            autocomplete: autocomplete,
            step: step,
            min: min,
            max: max,
            aria_label: aria_label,
            disabled: disabled,
            readonly: readonly,
            required: required,
            oninput: move |e| {
                if let Some(handler) = &oninput {
                    handler.call(e);
                }
            },
            ..attributes,
        }
    }
}

#[component]
pub fn InputGroupTextarea(#[props(into, optional)] class: Option<String>) -> Element {
    let merged = tw_merge!(
        "flex-1 resize-none rounded-none border-0 bg-transparent py-3 shadow-none focus-visible:ring-0 dark:bg-transparent",
        "placeholder:text-muted-foreground w-full min-w-0 px-3 text-base outline-none transition-[color,box-shadow]",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        textarea { "data-slot": "input-group-control", class: "{merged}" }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::{test_role::by_role_name, test_wait::wait_until};

    fn fixture() -> Element {
        rsx! {
            InputGroupInput { r#type: InputType::Number, min: "0", step: "any", aria_label: "Snap gap" }
            InputGroupInput { r#type: InputType::Number }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn numeric_input_forwards_constraints_and_omits_optional_defaults() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("numeric controls mount", || {
            by_role_name(&root, "spinbutton", "Snap gap").is_some()
        })
        .await;
        let gap = by_role_name(&root, "spinbutton", "Snap gap").unwrap();
        assert_eq!(gap.get_attribute("min").as_deref(), Some("0"));
        assert_eq!(gap.get_attribute("step").as_deref(), Some("any"));
        let default = by_role_name(&root, "spinbutton", "").unwrap();
        for attribute in ["min", "max", "step", "aria-label"] {
            assert!(!default.has_attribute(attribute));
        }
        root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod forwarding_tests {
    use super::*;
    use crate::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn listener_update_fixture() -> Element {
        use crate::ui::button::Button;
        let mut removed = use_signal(|| false);
        let mut events = use_signal(Vec::<String>::new);
        let mut value = use_signal(|| "1.2345678901234567".to_owned());
        rsx! {
            InputGroupInput {
                r#type: InputType::Number, aria_label: "Updating input", value: value(), class: "font-mono",
                oninput: move |event: FormEvent| { value.set(event.value()); events.write().push("owner-input".into()); },
                attributes: {
                    let mut attributes = Vec::new();
                    if !removed() { attributes.push(dioxus::events::onfocus(move |_| events.write().push("focus".into()))); }
                    attributes.push(dioxus::events::onblur(move |_| events.write().push("blur".into())));
                    attributes.push(Attribute::new("data-retained", "kept", None, false));
                    attributes.push(Attribute::new("class", "collision", None, false));
                    attributes.push(Attribute::new("value", "999", None, true));
                    attributes.push(Attribute::new("color", "rgb(1, 2, 3)", Some("style"), false));
                    attributes.push(dioxus::events::oninput(move |_| events.write().push("collision-input".into())));
                    if !removed() { attributes.push(Attribute::new("data-removed", "remove", None, false)); }
                    attributes
                },
            }
            Button { onclick: move |_| removed.set(true), "Remove input focus" }
            Button { onclick: move |_| events.write().push("flush".into()), "Flush input updates" }
            output { role: "status", aria_label: "Input phase", "{removed}" }
            output { role: "status", aria_label: "Input update events", "{events:?}" }
        }
    }

    #[wasm_bindgen_test]
    async fn input_updating_attributes_remove_focus_but_keep_nonbubbling_blur_and_owner_value() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(listener_update_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("updating input mounts", || {
            by_role_name(&root, "spinbutton", "Updating input").is_some()
        })
        .await;
        let input: web_sys::HtmlInputElement = by_role_name(&root, "spinbutton", "Updating input")
            .unwrap()
            .dyn_into()
            .unwrap();
        assert!(input.has_attribute("data-removed"));
        for phase in 0..2 {
            if phase == 1 {
                by_role_name(&root, "button", "Remove input focus")
                    .unwrap()
                    .click();
                wait_until("input focus removal renders", || {
                    by_role_name(&root, "status", "Input phase")
                        .unwrap()
                        .text_content()
                        .as_deref()
                        == Some("true")
                })
                .await;
            }
            input.focus().unwrap();
            input.blur().unwrap();
            by_role_name(&root, "button", "Flush input updates")
                .unwrap()
                .click();
            wait_until("input event render barrier", || {
                by_role_name(&root, "status", "Input update events")
                    .unwrap()
                    .text_content()
                    .unwrap()
                    .matches("flush")
                    .count()
                    == phase + 1
            })
            .await;
        }
        let log = by_role_name(&root, "status", "Input update events")
            .unwrap()
            .text_content()
            .unwrap();
        assert_eq!(log.matches("focus").count(), 1, "{log}");
        assert_eq!(log.matches("blur").count(), 2, "{log}");
        assert_eq!(input.value(), "1.2345678901234567");
        assert_eq!(
            input.get_attribute("data-retained").as_deref(),
            Some("kept")
        );
        assert!(!input.has_attribute("data-removed"));
        assert!(!input.get_attribute("class").unwrap().contains("collision"));
        assert_eq!(
            input.style().get_property_value("color").unwrap(),
            "rgb(1, 2, 3)"
        );
        input.set_value("1.8765432109876543");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        wait_until("retained owner input listener runs", || {
            by_role_name(&root, "status", "Input update events")
                .unwrap()
                .text_content()
                .unwrap()
                .contains("owner-input")
        })
        .await;
        let log = by_role_name(&root, "status", "Input update events")
            .unwrap()
            .text_content()
            .unwrap();
        assert_eq!(log.matches("owner-input").count(), 1, "{log}");
        assert!(!log.contains("collision-input"));
        assert_eq!(input.value(), "1.8765432109876543");
        root.remove();
    }

    fn fixture() -> Element {
        let mut value = use_signal(|| "1.2345678901234567".to_owned());
        let mut inputs = use_signal(|| 0);
        let mut collisions = use_signal(|| 0);
        let mut blurs = use_signal(|| 0);
        let mut keys = use_signal(|| 0);
        let mut mounts = use_signal(|| 0);
        rsx! {
            InputGroupInput {
                r#type: InputType::Number, value: value(),
                class: "font-mono", min: "0.1", max: "2", step: "any",
                aria_label: "Precise forwarded input",
                aria_invalid: "true", aria_describedby: "input-forwarding-error",
                oninput: move |event: FormEvent| { inputs += 1; value.set(event.value()); },
                attributes: vec![
                    Attribute::new("value", "999", None, true),
                    Attribute::new("class", "collision-class", None, false),
                    Attribute::new("min", "99", None, false),
                    Attribute::new("color", "rgb(1, 2, 3)", Some("style"), false),
                    Attribute::new("data-forwarded", "kept", None, false),
                    dioxus::events::oninput(move |_| collisions += 1),
                    dioxus::events::onblur(move |_| blurs += 1),
                    dioxus::events::onkeydown(move |_| keys += 1),
                    dioxus::events::onmounted(move |_| mounts += 1),
                ],
            }
            InputGroupInput {
                aria_label: "Omitted native defaults",
                attributes: vec![Attribute::new("max", "5", None, false)],
            }
            p { id: "input-forwarding-error", "Enter a dimension within the supported range." }
            output { aria_label: "Forwarding counts", "{inputs}:{collisions}:{blurs}:{keys}:{mounts}" }
        }
    }

    #[wasm_bindgen_test]
    async fn native_input_forwards_constraints_events_and_error_aria_once() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("forwarded input mounts once", || {
            root.text_content().unwrap().contains("0:0:0:0:1")
        })
        .await;
        let input: web_sys::HtmlInputElement =
            by_role_name(&root, "spinbutton", "Precise forwarded input")
                .unwrap()
                .dyn_into()
                .unwrap();
        assert_eq!(input.value(), "1.2345678901234567");
        for (name, value) in [
            ("min", "0.1"),
            ("max", "2"),
            ("step", "any"),
            ("aria-invalid", "true"),
            ("aria-describedby", "input-forwarding-error"),
            ("data-slot", "input-group-control"),
        ] {
            assert_eq!(input.get_attribute(name).as_deref(), Some(value), "{name}");
        }
        let class = input.get_attribute("class").unwrap();
        assert!(class.contains("font-mono") && class.contains("h-control"));
        assert!(!class.contains("collision-class"));
        assert_eq!(
            input.get_attribute("data-forwarded").as_deref(),
            Some("kept")
        );
        assert_eq!(
            input.style().get_property_value("color").unwrap(),
            "rgb(1, 2, 3)"
        );
        assert!(
            document
                .get_element_by_id(&input.get_attribute("aria-describedby").unwrap())
                .is_some()
        );
        let default = by_role_name(&root, "textbox", "Omitted native defaults").unwrap();
        for name in [
            "min",
            "max",
            "step",
            "value",
            "aria-invalid",
            "aria-describedby",
        ] {
            assert!(!default.has_attribute(name), "{name} stays omitted");
        }
        input.focus().unwrap();
        input.set_value("1.8765432109876543");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        let key = web_sys::KeyboardEventInit::new();
        key.set_key("Escape");
        key.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key)
                    .unwrap(),
            )
            .unwrap();
        input.blur().unwrap();
        wait_until(
            "each forwarded listener runs once and explicit input owns collisions",
            || root.text_content().unwrap().contains("1:0:1:1:1"),
        )
        .await;
        assert_eq!(
            input.value(),
            "1.8765432109876543",
            "native precision remains intact"
        );
        assert!(input.check_validity());
        input.set_value("3");
        assert!(!input.check_validity(), "native max stays enforced");
        input.set_value("0");
        assert!(!input.check_validity(), "native min stays enforced");
        root.remove();
    }
}
