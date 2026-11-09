//! Look copied from rust-ui: `app_crates/registry/src/ui/dialog.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::dialog` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - the primitive's root is the backdrop and its content is the panel; upstream's built-in close button
//! -   is dropped (the primitive has no close part; callers close through `open` / `on_open_change`);
//! - the panel's border is removed, cards style (theme contract section 8).

use dioxus::prelude::*;
use dioxus_primitives::dialog::{self, DialogDescriptionProps, DialogRootProps, DialogTitleProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Dialog(props: DialogRootProps) -> Element {
    let base = attributes!(div {
        class: "relative bg-background rounded-2xl shadow-lg p-6 w-full max-w-[calc(100%-2rem)] sm:max-w-lg max-h-[85vh] fixed top-[50%] left-[50%] translate-x-[-50%] translate-y-[-50%] z-100 flex flex-col gap-4",
        "data-slot": "dialog-content",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        dialog::DialogRoot {
            class: "fixed inset-0 z-100 bg-black/50 transition-opacity duration-200 data-[state=closed]:opacity-0 data-[state=open]:opacity-100",
            "data-slot": "dialog",
            id: props.id,
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            dialog::DialogContent { class: None, attributes: merged, {props.children} }
        }
    }
}

#[component]
pub fn DialogTitle(props: DialogTitleProps) -> Element {
    let base = attributes!(h2 { class: "text-lg leading-none font-semibold", "data-slot": "dialog-title" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        dialog::DialogTitle { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DialogDescription(props: DialogDescriptionProps) -> Element {
    let base = attributes!(p { class: "text-muted-foreground text-sm", "data-slot": "dialog-description" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        dialog::DialogDescription { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DialogHeader(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge::tw_merge!(
        "flex flex-col gap-2 text-center sm:text-left",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "dialog-header", class: "{merged}", {children} } }
}

#[component]
pub fn DialogBody(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge::tw_merge!("flex flex-col gap-4", class.as_deref().unwrap_or(""));
    rsx! { div { "data-slot": "dialog-body", class: "{merged}", {children} } }
}

#[component]
pub fn DialogFooter(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge::tw_merge!(
        "flex flex-col-reverse gap-2 sm:flex-row sm:justify-end",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "dialog-footer", class: "{merged}", {children} } }
}
