//! Look adapted from rust-ui `app_crates/registry/src/ui/sheet.rs` at
//! `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see `web/assets/licenses/rust-ui-MIT.txt`).
//! Rust owns the non-modal behaviour: no backdrop, scroll lock or focus trap. The owned Escape
//! listener respects a nested control's handled event and is removed on close/unmount.

use dioxus::prelude::*;
use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::button::{Button, ButtonVariant};

static SHEET_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetDirection {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

impl SheetDirection {
    fn side(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::Top => "top-0 left-0 w-full h-[400px] max-h-[65dvh]",
            Self::Bottom => "bottom-0 left-0 w-full h-[400px] max-h-[65dvh]",
            Self::Left => "top-0 left-0 h-full w-[400px] max-w-[85vw]",
            Self::Right => "top-0 right-0 h-full w-[400px] max-w-[85vw]",
        }
    }
}

#[derive(Clone)]
struct SheetContext {
    id: String,
    open: Signal<bool>,
    trigger: Signal<Option<Rc<MountedData>>>,
}

/// Controlled non-modal Sheet. Content can retain its owner when used as a persistent dock.
#[component]
pub fn Sheet(
    open: Signal<bool>,
    #[props(into, optional)] id: Option<String>,
    children: Element,
) -> Element {
    let id = use_hook(|| {
        id.unwrap_or_else(|| format!("sheet-{}", SHEET_COUNTER.fetch_add(1, Ordering::Relaxed)))
    });
    let trigger = use_signal(|| None::<Rc<MountedData>>);
    use_context_provider(|| SheetContext { id, open, trigger });
    #[cfg(target_arch = "wasm32")]
    let escape_guard = use_signal(|| None::<crate::DomEventListener>);
    let was_open = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    use_effect({
        #[cfg(target_arch = "wasm32")]
        let mut escape_guard = escape_guard;
        move || {
            let is_open = open();
            if was_open.replace(is_open)
                && !is_open
                && let Some(trigger) = trigger.peek().clone()
            {
                spawn(async move {
                    let _ = trigger.set_focus(true).await;
                });
            }
            // Bubble after primitives' document listeners have consumed their own Escape.
            #[cfg(target_arch = "wasm32")]
            escape_guard.set(is_open.then(web_sys::window).flatten().and_then(|window| {
                crate::DomEventListener::listen(
                    &window,
                    "keydown",
                    false,
                    move |event: web_sys::Event| {
                        let event: web_sys::KeyboardEvent =
                            wasm_bindgen::JsCast::unchecked_into(event);
                        if event.key() == "Escape" && !event.default_prevented() {
                            event.prevent_default();
                            open.set(false);
                        }
                    },
                )
            }));
        }
    });
    rsx! { div { "data-slot": "sheet", class: "w-fit", {children} } }
}

#[component]
pub fn SheetTrigger(children: Element) -> Element {
    let SheetContext {
        id,
        mut open,
        mut trigger,
    } = use_context::<SheetContext>();
    rsx! {
        button {
            "data-slot": "sheet-trigger", r#type: "button",
            class: "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium bg-secondary text-secondary-foreground shadow-xs hover:bg-accent h-control px-4 py-2 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring [@media(pointer:coarse)]:min-h-control-touch",
            aria_haspopup: "dialog", aria_expanded: open(), aria_controls: id,
            onmounted: move |event| trigger.set(Some(event.data())),
            onclick: move |_| open.toggle(),
            {children}
        }
    }
}

#[component]
pub fn SheetContent(
    aria_label: String,
    aria_describedby: String,
    #[props(default)] direction: SheetDirection,
    #[props(into, optional)] class: Option<String>,
    #[props(default)] keep_mounted: bool,
    #[props(default)] inline: bool,
    children: Element,
) -> Element {
    let SheetContext { id, open, .. } = use_context::<SheetContext>();
    let mut mounted = use_signal(|| None::<Rc<MountedData>>);
    use_effect(use_reactive(
        (&inline, &keep_mounted),
        move |(inline, keep_mounted)| {
            if keep_mounted
                && open()
                && !inline
                && let Some(element) = mounted()
            {
                spawn(async move {
                    let _ = element.set_focus(true).await;
                });
            }
        },
    ));
    let class = tw_merge::tw_merge!(
        "fixed box-border z-100 bg-card text-card-foreground shadow-lg rounded-xl p-6 overflow-y-auto overscroll-y-contain outline-none focus-visible:ring-2 focus-visible:ring-ring",
        direction.class(),
        class.as_deref().unwrap_or("")
    );
    rsx! {
        if open() || keep_mounted || inline {
            section {
                id, "data-slot": "sheet-content", "data-state": if open() { "open" } else { "closed" }, "data-side": direction.side(),
                "data-inline": inline,
                class: if inline { "contents".to_owned() } else { class },
                role: (!inline && open()).then_some("dialog"),
                aria_label: (!inline && open()).then_some(aria_label),
                aria_describedby: (!inline && open()).then_some(aria_describedby),
                aria_modal: (!inline && open()).then_some("false"),
                tabindex: (!inline && open()).then_some(-1),
                hidden: !inline && !open(), inert: (!inline && !open()).then_some(""),
                onmounted: move |event| async move {
                    if keep_mounted { mounted.set(Some(event.data())); }
                    else if !inline { let _ = event.data().set_focus(true).await; }
                },
                {children}
            }
        }
    }
}

#[component]
pub fn SheetClose(children: Element) -> Element {
    let mut open = use_context::<SheetContext>().open;
    rsx! { Button { variant: ButtonVariant::Secondary, onclick: move |_| open.set(false), {children} } }
}
