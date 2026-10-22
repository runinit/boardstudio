//! Copied from rust-ui: `app_crates/registry/src/ui/button.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - the router `Link` branch is a plain `a` with `href`;
//! - `data-slot="button"` on both elements: the legacy unlayered `button`, `input` and `select`
//!   rules in `m1.css` skip elements with a `data-slot`, and `tailwind.css` gives them a minimal
//!   reset instead (the app has no Tailwind preflight). Every copied component's root element
//!   carries a `data-slot` for the same reason, until the legacy rules are deleted.
//! - heights read the shared control sizes (`h-9` → `h-control`, `h-8` → `h-control-compact`,
//!   `h-10` → `h-control-touch`, `size-9`/`size-8` likewise; theme contract section 5).
//! - the `Outline` variant's border, including `dark:border-input`, is removed: cards style (theme
//!   contract section 8); focus rings kept.
//! - optional `aria-current` and `aria-pressed` forward caller-owned header/navigation state.

use dioxus::{core::AttributeValue, prelude::*};
use tw_merge::tw_merge;

#[derive(Default, Clone, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Destructive,
    Warning,
    Success,
    Outline,
    Secondary,
    Ghost,
    Link,
}

#[cfg(all(test, target_arch = "wasm32"))]
mod forwarding_tests {
    use super::*;
    use crate::{test_role::by_role_name, test_wait::wait_until};

    fn fixture() -> Element {
        let mut events = use_signal(Vec::<String>::new);
        rsx! {
            div { onkeydown: move |event: KeyboardEvent| events.write().push(format!("owner:{}", event.key())),
                for (label, href) in [("Native contract", None), ("Link contract", Some("#contract"))] {
                    Button {
                        href: href.map(str::to_owned), aria_label: label, aria_describedby: "button-contract-description",
                        onkeydown: move |event: KeyboardEvent| events.write().push(format!("{label}:{}", event.key())),
                        onfocus: move |_| events.write().push(format!("{label}:focus")),
                        onblur: move |_| events.write().push(format!("{label}:blur")),
                        onpointerenter: move |_| events.write().push(format!("{label}:enter")),
                        onpointerdown: move |_| events.write().push(format!("{label}:pointerdown")),
                        onpointerup: move |_| events.write().push(format!("{label}:pointerup")),
                        onpointerleave: move |_| events.write().push(format!("{label}:pointerleave")),
                        onpointercancel: move |_| events.write().push(format!("{label}:pointercancel")),
                        onclick: move |_| events.write().push(format!("{label}:click")),
                        "{label}"
                    }
                }
                Button { aria_label: "Default native", "Default native" }
                Button { href: "#default", aria_label: "Default link", "Default link" }
            }
            span { id: "button-contract-description", "The owner describes this action." }
            output { role: "status", aria_label: "Contract events", "{events:?}" }
        }
    }

    fn tooltip_fixture() -> Element {
        use crate::ui::tooltip::{Tooltip, TooltipContent, TooltipTrigger};
        let mut events = use_signal(Vec::<String>::new);
        rsx! {
            for (label, href) in [("Composed native", None), ("Composed link", Some("#composed"))] {
                Tooltip { on_open_change: move |open| events.write().push(format!("{label}:tip:{open}")),
                    TooltipTrigger { id: format!("fallback-{label}"), r#as: move |mut attributes: Vec<Attribute>| {
                        attributes.extend([
                            Attribute::new("data-extra", "kept", None, false),
                            Attribute::new("aria-describedby", "owner-description owner-description", None, false),
                            dioxus::events::onpointerenter(move |_| events.write().push(format!("{label}:raw:enter"))),
                            dioxus::events::onclick(move |_| events.write().push(format!("{label}:raw:click"))),
                            dioxus::events::onpointerdown(move |_| events.write().push(format!("{label}:raw:pointerdown"))),
                            dioxus::events::onpointerup(move |_| events.write().push(format!("{label}:raw:pointerup"))),
                            dioxus::events::onpointerleave(move |_| events.write().push(format!("{label}:raw:pointerleave"))),
                            dioxus::events::onpointercancel(move |_| events.write().push(format!("{label}:raw:pointercancel"))),
                        ]);
                        rsx! { Button {
                            attributes, href: href.map(str::to_owned), id: format!("owner-{label}"),
                            aria_label: label, aria_describedby: "owner-description", class: "font-mono",
                            onfocus: move |_| events.write().push(format!("{label}:owner:focus")),
                            onblur: move |_| events.write().push(format!("{label}:owner:blur")),
                            onkeydown: move |_| events.write().push(format!("{label}:owner:key")),
                            onpointerenter: move |_| events.write().push(format!("{label}:owner:enter")),
                            onclick: move |_| events.write().push(format!("{label}:owner:click")),
                            onpointerdown: move |_| events.write().push(format!("{label}:owner:pointerdown")),
                            onpointerup: move |_| events.write().push(format!("{label}:owner:pointerup")),
                            onpointerleave: move |_| events.write().push(format!("{label}:owner:pointerleave")),
                            onpointercancel: move |_| events.write().push(format!("{label}:owner:pointercancel")),
                            "{label}"
                        } }
                    } }
                    TooltipContent { "{label} tooltip" }
                }
            }
            Button { class: "font-mono", aria_label: "Reference style", "Reference" }
            Button { disabled: true, aria_label: "Disabled native", onclick: move |_| events.write().push("disabled-native".into()), "Disabled" }
            Button { disabled: true, href: "#disabled-link", aria_label: "Disabled link", onclick: move |_| events.write().push("disabled-link".into()), "Link" }
            span { id: "owner-description", "Owner description" }
            output { role: "status", aria_label: "Composition events", "{events:?}" }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn tooltip_and_button_owners_compose_once_in_order_without_a_style_fork() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(tooltip_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("composed Button description settles", || {
            by_role_name(&root, "button", "Composed native").is_some_and(|action| {
                action
                    .get_attribute("aria-describedby")
                    .unwrap_or_default()
                    .split_ascii_whitespace()
                    .count()
                    == 2
            })
        })
        .await;
        let observed = || {
            by_role_name(&root, "status", "Composition events")
                .unwrap()
                .text_content()
                .unwrap()
        };
        let reference_class = by_role_name(&root, "button", "Reference style")
            .unwrap()
            .get_attribute("class");
        for (role, label) in [("button", "Composed native"), ("link", "Composed link")] {
            let action = by_role_name(&root, role, label).unwrap();
            assert_eq!(action.get_attribute("class"), reference_class);
            assert_eq!(action.get_attribute("data-slot").as_deref(), Some("button"));
            assert_eq!(action.get_attribute("id"), Some(format!("owner-{label}")));
            assert_eq!(action.get_attribute("data-extra").as_deref(), Some("kept"));
            assert!(!action.has_attribute("style"));
            let descriptions = action.get_attribute("aria-describedby").unwrap();
            assert_eq!(
                descriptions
                    .split_ascii_whitespace()
                    .filter(|id| *id == "owner-description")
                    .count(),
                1
            );
            action.focus().unwrap();
            wait_until("Tooltip opens on the same Button focus", || {
                by_role_name(&root, "tooltip", &format!("{label} tooltip")).is_some()
            })
            .await;
            for id in descriptions.split_ascii_whitespace() {
                assert!(document.get_element_by_id(id).is_some());
            }
            let key = web_sys::KeyboardEventInit::new();
            key.set_key("Escape");
            key.set_bubbles(true);
            key.set_cancelable(true);
            let escape =
                web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key).unwrap();
            assert!(!action.dispatch_event(&escape).unwrap());
            assert!(escape.default_prevented());
            action.blur().unwrap();
            for name in ["mouseenter", "mouseleave"] {
                let event = web_sys::MouseEvent::new(name).unwrap();
                event.init_event_with_bubbles(name, true);
                action.dispatch_event(&event).unwrap();
                let open = name == "mouseenter";
                wait_until("Tooltip mouse state changes", || {
                    by_role_name(&root, "tooltip", &format!("{label} tooltip")).is_some() == open
                })
                .await;
            }
            for name in [
                "pointerenter",
                "pointerdown",
                "pointerup",
                "pointerleave",
                "pointercancel",
            ] {
                let event = web_sys::PointerEvent::new(name).unwrap();
                event.init_event_with_bubbles(name, true);
                action.dispatch_event(&event).unwrap();
            }
            let click = web_sys::MouseEvent::new("click").unwrap();
            click.init_event_with_bubbles_and_cancelable("click", true, true);
            assert!(
                action.dispatch_event(&click).unwrap(),
                "composition keeps click's default action"
            );
            wait_until("composed events render", || {
                observed().contains(&format!("{label}:owner:click"))
            })
            .await;
            let log = observed();
            for event in [
                "focus",
                "blur",
                "key",
                "enter",
                "click",
                "pointerdown",
                "pointerup",
                "pointerleave",
                "pointercancel",
            ] {
                assert_eq!(
                    log.matches(&format!("{label}:owner:{event}\"")).count(),
                    1,
                    "{log}"
                );
            }
            for event in [
                "enter",
                "click",
                "pointerdown",
                "pointerup",
                "pointerleave",
                "pointercancel",
            ] {
                assert_eq!(
                    log.matches(&format!("{label}:raw:{event}\"")).count(),
                    1,
                    "{log}"
                );
                assert!(
                    log.contains(&format!("{label}:raw:{event}\", \"{label}:owner:{event}")),
                    "{log}"
                );
            }
            for (first, second) in [
                ("tip:true", "owner:focus"),
                ("tip:false", "owner:key"),
                ("tip:false", "owner:blur"),
            ] {
                assert!(
                    log.contains(&format!("{label}:{first}\", \"{label}:{second}")),
                    "{log}"
                );
            }
            assert_eq!(
                log.matches(&format!("{label}:tip:true")).count(),
                2,
                "pointer events do not reopen Tooltip: {log}"
            );
            assert_eq!(
                log.matches(&format!("{label}:tip:false")).count(),
                3,
                "{log}"
            );
        }
        let disabled = by_role_name(&root, "button", "Disabled native").unwrap();
        assert!(disabled.has_attribute("disabled"));
        disabled.click();
        let link = by_role_name(&root, "link", "Disabled link").unwrap();
        assert!(!link.has_attribute("disabled"));
        assert_eq!(
            link.get_attribute("href").as_deref(),
            Some("#disabled-link")
        );
        link.click();
        wait_until("link retains its existing disabled semantics", || {
            observed().contains("disabled-link")
        })
        .await;
        assert!(!observed().contains("disabled-native"));
        root.remove();
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn owner_click_and_pointer_callbacks_forward_once_on_native_and_link() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("owner callback controls mount", || {
            by_role_name(&root, "link", "Link contract").is_some()
        })
        .await;
        for (role, label) in [("button", "Native contract"), ("link", "Link contract")] {
            let action = by_role_name(&root, role, label).unwrap();
            let click = web_sys::MouseEvent::new("click").unwrap();
            click.init_event_with_bubbles_and_cancelable("click", true, true);
            assert!(action.dispatch_event(&click).unwrap());
            for name in ["pointerdown", "pointerup", "pointerleave", "pointercancel"] {
                let event = web_sys::PointerEvent::new(name).unwrap();
                event.init_event_with_bubbles(name, true);
                action.dispatch_event(&event).unwrap();
            }
        }
        wait_until("owner callback render flushes", || {
            by_role_name(&root, "status", "Contract events")
                .unwrap()
                .text_content()
                .unwrap()
                .contains("Native contract:pointercancel")
        })
        .await;
        let observed = by_role_name(&root, "status", "Contract events")
            .unwrap()
            .text_content()
            .unwrap();
        for label in ["Native contract", "Link contract"] {
            for event in [
                "click",
                "pointerdown",
                "pointerup",
                "pointerleave",
                "pointercancel",
            ] {
                assert_eq!(
                    observed.matches(&format!("{label}:{event}\"")).count(),
                    1,
                    "{observed}"
                );
            }
        }
        root.remove();
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn button_additive_events_preserve_owner_handlers_and_optional_defaults_in_both_branches()
    {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("Button contracts mount", || {
            by_role_name(&root, "link", "Default link").is_some()
        })
        .await;
        for (role, label) in [("button", "Native contract"), ("link", "Link contract")] {
            let action = by_role_name(&root, role, label).unwrap();
            let description = action.get_attribute("aria-describedby").unwrap();
            assert_eq!(
                document
                    .get_element_by_id(&description)
                    .unwrap()
                    .text_content()
                    .as_deref(),
                Some("The owner describes this action.")
            );
            action.focus().unwrap();
            action.blur().unwrap();
            action
                .dispatch_event(&web_sys::PointerEvent::new("pointerenter").unwrap())
                .unwrap();
            for key in ["Enter", " "] {
                let init = web_sys::KeyboardEventInit::new();
                init.set_key(key);
                init.set_bubbles(true);
                action
                    .dispatch_event(
                        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict(
                            "keydown", &init,
                        )
                        .unwrap(),
                    )
                    .unwrap();
            }
            wait_until("all caller events forward", || {
                by_role_name(&root, "status", "Contract events")
                    .unwrap()
                    .text_content()
                    .unwrap()
                    .contains(&format!("{label}:enter"))
            })
            .await;
            let observed = by_role_name(&root, "status", "Contract events")
                .unwrap()
                .text_content()
                .unwrap();
            for event in ["focus", "blur", "enter", "Enter", " "] {
                assert_eq!(
                    observed.matches(&format!("{label}:{event}\"")).count(),
                    1,
                    "{observed}"
                );
            }
            assert!(
                !observed.contains(&format!("{label}:click")),
                "synthetic key events do not synthesize native clicks"
            );
        }
        let observed = by_role_name(&root, "status", "Contract events")
            .unwrap()
            .text_content()
            .unwrap();
        assert_eq!(
            observed.matches("owner:Enter").count(),
            2,
            "owner listener is never overwritten"
        );
        for (role, label) in [("button", "Default native"), ("link", "Default link")] {
            let action = by_role_name(&root, role, label).unwrap();
            for attr in [
                "aria-describedby",
                "onkeydown",
                "onfocus",
                "onblur",
                "onpointerenter",
            ] {
                assert!(!action.has_attribute(attr));
            }
            action.focus().unwrap();
            action.blur().unwrap();
            action.click();
        }
        assert_eq!(
            by_role_name(&root, "button", "Default native")
                .unwrap()
                .get_attribute("type")
                .as_deref(),
            Some("button")
        );
        root.remove();
    }
}

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Eq)]
pub enum ButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
    Icon,
    IconSm,
    IconXs,
}

impl ButtonVariant {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
            Self::Destructive => {
                "bg-destructive text-white shadow-xs hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:focus-visible:ring-destructive/40 dark:bg-destructive/60"
            }
            Self::Warning => "bg-warning text-warning-foreground hover:bg-warning/90",
            Self::Success => "bg-success text-success-foreground hover:bg-success/90",
            Self::Outline => {
                "bg-background shadow-xs hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:hover:bg-input/5"
            }
            Self::Secondary => {
                "bg-secondary text-secondary-foreground shadow-xs hover:bg-secondary/80"
            }
            Self::Ghost => "hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50",
            Self::Link => "text-primary underline-offset-4 hover:underline",
        }
    }
}

impl ButtonSize {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "h-control px-4 py-2 has-[>svg]:px-3",
            Self::Sm => "h-control-compact rounded-md gap-1.5 px-3 has-[>svg]:px-2.5",
            Self::Lg => "h-control-touch rounded-md px-6 has-[>svg]:px-4",
            Self::Icon => "size-control",
            Self::IconSm => "size-control-compact rounded-md",
            Self::IconXs => "size-6 rounded-md",
        }
    }
}

// Compose platform listeners before their typed adapters, matching the public Dioxus event API.
fn button_attributes(
    mut attributes: Vec<Attribute>,
    owners: Vec<Attribute>,
    id: Option<&str>,
    description: Option<&str>,
) -> Vec<Attribute> {
    let mut descriptions = Vec::new();
    for value in description
        .into_iter()
        .chain(attributes.iter().filter_map(|attribute| {
            if attribute.namespace.is_none()
                && attribute.name == "aria-describedby"
                && let AttributeValue::Text(value) = &attribute.value
            {
                Some(value.as_str())
            } else {
                None
            }
        }))
    {
        for token in value.split_ascii_whitespace() {
            if !descriptions.iter().any(|existing| existing == token) {
                descriptions.push(token.to_owned());
            }
        }
    }
    attributes.retain(|attribute| {
        if attribute.namespace == Some("style") {
            return false;
        }
        attribute.namespace.is_some()
            || !(matches!(
                attribute.name,
                "class"
                    | "style"
                    | "data-slot"
                    | "type"
                    | "href"
                    | "disabled"
                    | "aria-label"
                    | "aria-describedby"
                    | "aria-current"
                    | "aria-pressed"
                    | "aria-expanded"
                    | "aria-controls"
                    | "aria-haspopup"
                    | "title"
            ) || (attribute.name == "id" && id.is_some()))
    });
    if !descriptions.is_empty() {
        attributes.push(Attribute::new(
            "aria-describedby",
            descriptions.join(" "),
            None,
            false,
        ));
    }
    for mut owner in owners {
        if let Some(attribute) = attributes.iter_mut().find(|attribute| {
            attribute.name == owner.name && attribute.namespace == owner.namespace
        }) {
            if let (AttributeValue::Listener(trigger), AttributeValue::Listener(handler)) =
                (&attribute.value, &owner.value)
            {
                let trigger = trigger.clone();
                let handler = handler.clone();
                owner.value = AttributeValue::listener(
                    move |event: Event<dioxus::events::PlatformEventData>| {
                        trigger.call(event.clone().into_any());
                        handler.call(event.into_any());
                    },
                );
            }
            *attribute = owner;
        } else {
            attributes.push(owner);
        }
    }
    attributes
}

#[cfg(test)]
mod attribute_tests {
    use super::*;

    #[test]
    fn button_owns_scalars_and_styles_but_keeps_id_fallback_and_unknown_attributes() {
        let canonical = [
            "class",
            "style",
            "data-slot",
            "type",
            "href",
            "disabled",
            "aria-label",
            "aria-current",
            "aria-pressed",
            "aria-expanded",
            "aria-controls",
            "aria-haspopup",
            "title",
        ];
        let mut raw: Vec<_> = canonical
            .iter()
            .map(|name| Attribute::new(name, "collision", None, false))
            .collect();
        raw.extend([
            Attribute::new("color", "red", Some("style"), false),
            Attribute::new("id", "trigger-id", None, false),
            Attribute::new("aria-describedby", "tip owner tip", None, false),
            Attribute::new("data-extra", "kept", None, false),
            Attribute::new("tabindex", "0", None, false),
            Attribute::new("custom", "kept", Some("example"), false),
        ]);
        let merged = button_attributes(raw.clone(), vec![], None, Some("owner shared owner"));
        assert!(
            merged
                .iter()
                .all(|attribute| !canonical.contains(&attribute.name)
                    && attribute.namespace != Some("style"))
        );
        for (name, value) in [
            ("id", "trigger-id"),
            ("aria-describedby", "owner shared tip"),
            ("data-extra", "kept"),
            ("tabindex", "0"),
            ("custom", "kept"),
        ] {
            assert_eq!(
                merged
                    .iter()
                    .find(|attribute| attribute.name == name)
                    .unwrap()
                    .value,
                AttributeValue::Text(value.into())
            );
        }
        assert!(
            !button_attributes(raw, vec![], Some("owner-id"), None)
                .iter()
                .any(|attribute| attribute.name == "id")
        );
        assert!(button_attributes(vec![], vec![], None, None).is_empty());
    }
}

/// Explicit attributes support public trigger composition. Button owns its structural/style and
/// typed scalar attributes; id falls back to the vector, descriptions combine tokens, and raw
/// listeners run before typed owner listeners. Other attributes pass through on both branches.
#[component]
pub fn Button(
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(default = ButtonVariant::default())] variant: ButtonVariant,
    #[props(default = ButtonSize::default())] size: ButtonSize,
    #[props(optional)] disabled: bool,
    #[props(into, optional)] href: Option<String>,
    // TODO: replace button_type with a proper ButtonType enum (e.g. ButtonType::Submit/Reset/Button)
    #[props(into, optional)] button_type: Option<String>,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(optional)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(optional)] onfocus: Option<EventHandler<FocusEvent>>,
    #[props(optional)] onblur: Option<EventHandler<FocusEvent>>,
    #[props(optional)] onpointerenter: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointerdown: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointerup: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointerleave: Option<EventHandler<PointerEvent>>,
    #[props(optional)] onpointercancel: Option<EventHandler<PointerEvent>>,
    #[props(into, optional)] aria_label: Option<String>,
    #[props(into, optional)] aria_describedby: Option<String>,
    #[props(into, optional)] aria_current: Option<String>,
    #[props(optional)] aria_pressed: Option<bool>,
    #[props(optional)] aria_expanded: Option<bool>,
    #[props(into, optional)] aria_controls: Option<String>,
    #[props(into, optional)] aria_haspopup: Option<String>,
    #[props(into, optional)] title: Option<String>,
    #[props(default)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let merged_class = tw_merge!(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive w-fit hover:cursor-pointer active:scale-[0.98] active:opacity-100 touch-manipulation [-webkit-tap-highlight-color:transparent] select-none [-webkit-touch-callout:none]",
        variant.as_str(),
        size.as_str(),
        class.as_deref().unwrap_or("")
    );

    let owners = [
        onclick.map(|handler| dioxus::events::onclick(move |event| handler.call(event))),
        onkeydown.map(|handler| dioxus::events::onkeydown(move |event| handler.call(event))),
        onfocus.map(|handler| dioxus::events::onfocus(move |event| handler.call(event))),
        onblur.map(|handler| dioxus::events::onblur(move |event| handler.call(event))),
        onpointerenter
            .map(|handler| dioxus::events::onpointerenter(move |event| handler.call(event))),
        onpointerdown
            .map(|handler| dioxus::events::onpointerdown(move |event| handler.call(event))),
        onpointerup.map(|handler| dioxus::events::onpointerup(move |event| handler.call(event))),
        onpointerleave
            .map(|handler| dioxus::events::onpointerleave(move |event| handler.call(event))),
        onpointercancel
            .map(|handler| dioxus::events::onpointercancel(move |event| handler.call(event))),
    ]
    .into_iter()
    .flatten()
    .collect();
    let attributes = button_attributes(
        attributes,
        owners,
        id.as_deref(),
        aria_describedby.as_deref(),
    );

    if let Some(url) = href {
        rsx! {
            a {
                id: id,
                "data-slot": "button",
                class: "{merged_class}",
                href: url,
                "aria-label": aria_label.as_deref(),
                "aria-current": aria_current.as_deref(),
                "aria-pressed": aria_pressed,
                "aria-expanded": aria_expanded,
                "aria-controls": aria_controls.as_deref(),
                "aria-haspopup": aria_haspopup.as_deref(),
                title: title.as_deref(),
                ..attributes,
                {children}
            }
        }
    } else {
        rsx! {
            button {
                id: id.as_deref(),
                "data-slot": "button",
                class: "{merged_class}",
                r#type: button_type.as_deref().unwrap_or("button"),
                disabled,
                "aria-label": aria_label.as_deref(),
                "aria-current": aria_current.as_deref(),
                "aria-pressed": aria_pressed,
                "aria-expanded": aria_expanded,
                "aria-controls": aria_controls.as_deref(),
                "aria-haspopup": aria_haspopup.as_deref(),
                title: title.as_deref(),
                ..attributes,
                {children}
            }
        }
    }
}
