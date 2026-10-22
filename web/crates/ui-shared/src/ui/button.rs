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

use dioxus::prelude::*;
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
    children: Element,
) -> Element {
    let merged_class = tw_merge!(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-all disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive w-fit hover:cursor-pointer active:scale-[0.98] active:opacity-100 touch-manipulation [-webkit-tap-highlight-color:transparent] select-none [-webkit-touch-callout:none]",
        variant.as_str(),
        size.as_str(),
        class.as_deref().unwrap_or("")
    );

    if let Some(url) = href {
        rsx! {
            a {
                id: id,
                "data-slot": "button",
                class: "{merged_class}",
                href: url,
                "aria-label": aria_label.as_deref(),
                "aria-describedby": aria_describedby.as_deref(),
                "aria-current": aria_current.as_deref(),
                "aria-pressed": aria_pressed,
                "aria-expanded": aria_expanded,
                "aria-controls": aria_controls.as_deref(),
                "aria-haspopup": aria_haspopup.as_deref(),
                title: title.as_deref(),
                onkeydown: move |event| { if let Some(handler) = onkeydown { handler.call(event); } },
                onfocus: move |event| { if let Some(handler) = onfocus { handler.call(event); } },
                onblur: move |event| { if let Some(handler) = onblur { handler.call(event); } },
                onpointerenter: move |event| { if let Some(handler) = onpointerenter { handler.call(event); } },
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
                "aria-describedby": aria_describedby.as_deref(),
                "aria-current": aria_current.as_deref(),
                "aria-pressed": aria_pressed,
                "aria-expanded": aria_expanded,
                "aria-controls": aria_controls.as_deref(),
                "aria-haspopup": aria_haspopup.as_deref(),
                title: title.as_deref(),
                onkeydown: move |event| { if let Some(handler) = onkeydown { handler.call(event); } },
                onfocus: move |event| { if let Some(handler) = onfocus { handler.call(event); } },
                onblur: move |event| { if let Some(handler) = onblur { handler.call(event); } },
                onpointerenter: move |event| { if let Some(handler) = onpointerenter { handler.call(event); } },
                onclick: move |e| {
                    if let Some(handler) = &onclick {
                        handler.call(e);
                    }
                },
                onpointerdown: move |e| {
                    if let Some(handler) = &onpointerdown {
                        handler.call(e);
                    }
                },
                onpointerup: move |e| {
                    if let Some(handler) = &onpointerup {
                        handler.call(e);
                    }
                },
                onpointerleave: move |e| {
                    if let Some(handler) = &onpointerleave {
                        handler.call(e);
                    }
                },
                onpointercancel: move |e| {
                    if let Some(handler) = &onpointercancel {
                        handler.call(e);
                    }
                },
                {children}
            }
        }
    }
}
