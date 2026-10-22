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
//! - the `role` of each toast and Escape are ticket 008 (accessibility floor).

use std::time::Duration;

use dioxus::prelude::*;
use dioxus_primitives::toast::{self, ToastProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

pub use dioxus_primitives::toast::{ToastOptions, use_toast};

#[component]
fn StyledToast(props: ToastProps) -> Element {
    let base = attributes!(div {
        class: "pointer-events-auto relative flex w-full items-start gap-3 rounded-xl bg-card text-card-foreground shadow-lg p-4 pr-8 text-sm data-[type=error]:text-destructive",
        "data-slot": "toast",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        toast::Toast {
            id: props.id,
            index: props.index,
            title: props.title,
            description: props.description,
            toast_type: props.toast_type,
            on_close: props.on_close,
            permanent: props.permanent,
            duration: props.duration,
            attributes: merged,
            toast::ToastContent { class: "flex flex-1 flex-col gap-1", "data-slot": "toast-content",
                toast::ToastTitle { class: "font-medium leading-none", "data-slot": "toast-title" }
                toast::ToastDescription { class: "text-muted-foreground", "data-slot": "toast-description" }
            }
            toast::ToastCloseButton {
                class: "absolute top-3 right-3 rounded-sm p-1 opacity-70 hover:opacity-100 outline-none focus-visible:ring-2 focus-visible:ring-ring",
                "data-slot": "toast-close",
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
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
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
