//! Look adapted from rust-ui `app_crates/registry/src/ui/sheet.rs` at
//! `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see `web/assets/licenses/rust-ui-MIT.txt`).
//! Rust owns the non-modal behaviour: no backdrop, scroll lock or focus trap. The owned Escape
//! listener respects a nested control's handled event and is removed on close/unmount.

use dioxus::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};
use wasm_bindgen::{JsCast, closure::Closure};

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

struct EscapeListener {
    window: web_sys::Window,
    callback: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

impl Drop for EscapeListener {
    fn drop(&mut self) {
        let _ = self
            .window
            .remove_event_listener_with_callback("keydown", self.callback.as_ref().unchecked_ref());
    }
}

/// Controlled non-modal Sheet. Keep domain drafts in the caller, as with other overlay primitives.
#[component]
pub fn Sheet(
    mut open: Signal<bool>,
    #[props(into, optional)] id: Option<String>,
    children: Element,
) -> Element {
    let id = use_hook(|| {
        id.unwrap_or_else(|| format!("sheet-{}", SHEET_COUNTER.fetch_add(1, Ordering::Relaxed)))
    });
    let trigger = use_signal(|| None::<Rc<MountedData>>);
    use_context_provider(|| SheetContext { id, open, trigger });
    let listener = use_hook(|| Rc::new(RefCell::new(None::<EscapeListener>)));
    let was_open = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    use_effect({
        let listener = listener.clone();
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
            listener.borrow_mut().take();
            if is_open && let Some(window) = web_sys::window() {
                let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                    if event.key() == "Escape" && !event.default_prevented() {
                        event.prevent_default();
                        open.set(false);
                    }
                }) as Box<dyn FnMut(_)>);
                // Bubble after primitives' document listeners have consumed their own Escape.
                if window
                    .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())
                    .is_ok()
                {
                    *listener.borrow_mut() = Some(EscapeListener { window, callback });
                }
            }
        }
    });
    use_drop(move || {
        listener.borrow_mut().take();
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
    children: Element,
) -> Element {
    let SheetContext { id, open, .. } = use_context::<SheetContext>();
    let class = tw_merge::tw_merge!(
        "fixed box-border z-100 bg-card text-card-foreground shadow-lg rounded-xl p-6 overflow-y-auto overscroll-y-contain outline-none focus-visible:ring-2 focus-visible:ring-ring",
        direction.class(),
        class.as_deref().unwrap_or("")
    );
    rsx! {
        if open() {
            section {
                id, "data-slot": "sheet-content", "data-state": "open", "data-side": direction.side(),
                class, role: "dialog", aria_label, aria_describedby, aria_modal: "false", tabindex: -1,
                onmounted: move |event| async move { let _ = event.data().set_focus(true).await; },
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
