//! Look in the style of rust-ui (`app_crates/registry/src/ui/toast.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318`) (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::toast` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream's `show_toast` builds toasts with `document::eval` and a script outside the registry; here the
//! -   primitive's provider and `use_toast()` drive them, and the item takes Card's look (filled card, soft shadow,
//! -   no border) with upstream's container placement (`fixed`, bottom right);
//! - ticket 008 uses one atomic status announcement; Escape stays with the surrounding workspace.

use std::time::Duration;

use super::button::{Button, keep_space_for_activation};
use dioxus::prelude::*;
use dioxus_primitives::toast::{self, ToastProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

pub use dioxus_primitives::toast::{ToastOptions, use_toast};

/// A generic action slot for one persistent notice, carried by our
/// [`ToastProvider`] (which forwards it down to the rendered toasts). The upstream
/// toast carries no caller identity, so the slot matches on what a notice is: an
/// info toast that is `permanent` and has [`ToastActionSlot::toast_title`]. A
/// transient result toast that merely shares the words never matches. A matched
/// toast renders a working action control with [`ToastActionSlot::action_label`]
/// and reports dismissal through [`ToastActionSlot::on_dismiss`]; every other
/// toast keeps its stock shape. Ticket 019 uses this for the update notice's
/// "Update Now" without changing transient result toasts.
#[derive(Clone, PartialEq)]
pub struct ToastActionSlot {
    pub toast_title: String,
    pub action_label: String,
    pub on_action: Callback<()>,
    pub on_dismiss: Callback<()>,
}

impl ToastActionSlot {
    fn matches(&self, props: &ToastProps) -> bool {
        props.permanent
            && props.toast_type == toast::ToastType::Info
            && props.title == self.toast_title
    }
}

#[component]
fn StyledToast(props: ToastProps) -> Element {
    let slot = try_consume_context::<Option<ToastActionSlot>>()
        .flatten()
        .filter(|slot| slot.matches(&props));
    // The action control sits inside the card, so the matched toast names its
    // announcement explicitly; otherwise the button text would join the status
    // name and screen readers would hear it twice (once in the status, once as
    // the focused control). Stock toasts keep content-derived names.
    let slot_title: Option<String> = slot.as_ref().map(|slot| slot.toast_title.clone());
    let base = attributes!(div {
        class: "pointer-events-auto relative flex w-full items-start gap-3 rounded-xl bg-card text-card-foreground shadow-lg p-4 pr-8 text-sm data-[type=error]:text-destructive",
        "data-slot": "toast",
        role: "status",
        aria_atomic: "true",
        aria_label: slot_title,
        aria_modal: Option::<String>::None,
        tabindex: "-1",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    let action = slot
        .clone()
        .map(|slot| (slot.action_label.clone(), slot.on_action));
    let on_close = props.on_close;
    rsx! {
        toast::Toast {
            id: props.id,
            index: props.index,
            title: props.title,
            description: props.description,
            toast_type: props.toast_type,
            on_close,
            permanent: props.permanent,
            duration: props.duration,
            attributes: merged,
            toast::ToastContent { role: "none", class: "flex flex-1 flex-col gap-1", "data-slot": "toast-content",
                toast::ToastTitle { class: "font-medium leading-none", "data-slot": "toast-title" }
                toast::ToastDescription { class: "text-muted-foreground", "data-slot": "toast-description" }
                if let Some((label, on_action)) = action {
                    Button { aria_label: label.clone(), onclick: move |_| on_action.call(()), onkeydown: move |event: KeyboardEvent| keep_space_for_activation(&event), "{label}" }
                }
            }
            if let Some(slot) = slot {
                button {
                    class: "absolute top-3 right-3 rounded-sm p-1 opacity-70 hover:opacity-100 outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    "data-slot": "toast-close",
                    aria_label: "close",
                    type: "button",
                    onclick: move |event| {
                        slot.on_dismiss.call(());
                        on_close.call(event);
                    },
                    "\u{d7}"
                }
            } else {
                toast::ToastCloseButton {
                    class: "absolute top-3 right-3 rounded-sm p-1 opacity-70 hover:opacity-100 outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    "data-slot": "toast-close",
                }
            }
        }
    }
}

#[component]
pub fn ToastProvider(
    #[props(default = ReadSignal::new(Signal::new(Some(Duration::from_secs(5)))))] default_duration: ReadSignal<
        Option<Duration>,
    >,
    #[props(default = ReadSignal::new(Signal::new(10)))] max_toasts: ReadSignal<usize>,
    #[props(into, optional)] action_slot: Option<ToastActionSlot>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    // Hooks run unconditionally, so the slot travels as an Option: `None` is
    // the stock provider with no action on any toast.
    use_context_provider(|| action_slot.clone());
    let base = attributes!(div {
        class: "fixed right-6 bottom-6 z-50 flex w-[400px] max-w-[calc(100vw-3rem)] flex-col gap-[15px] pointer-events-none [&_ol]:flex [&_ol]:flex-col [&_ol]:gap-[15px]",
        "data-slot": "toast-provider",
    });
    let merged = merge_attributes(vec![base, attributes]);
    rsx! {
        toast::ToastProvider {
            default_duration,
            max_toasts,
            render_toast: Callback::new(|p: toast::ToastPropsWithOwner| rsx! { StyledToast { ..p } }),
            attributes: merged,
            {children}
        }
    }
}
