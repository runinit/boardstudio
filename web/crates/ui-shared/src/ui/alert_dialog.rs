//! Look copied from rust-ui: `app_crates/registry/src/ui/alert_dialog.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::alert_dialog` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream builds Alert Dialog on its own Dialog; here it is the primitive's alert dialog with the same
//! -   panel classes as `dialog.rs` (border removed, cards style);
//! - Cancel is a secondary button and Action a primary one, with upstream's `DialogClose` / `DialogAction` classes
//! -   (Cancel filled instead of outlined).

use dioxus::prelude::*;
use dioxus_primitives::alert_dialog::{
    self, AlertDialogActionProps, AlertDialogActionsProps, AlertDialogCancelProps,
    AlertDialogDescriptionProps, AlertDialogRootProps, AlertDialogTitleProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn AlertDialog(props: AlertDialogRootProps) -> Element {
    let base = attributes!(div {
        class: "fixed inset-0 z-100 bg-black/50 transition-opacity duration-200 data-[state=closed]:opacity-0 data-[state=open]:opacity-100",
        "data-slot": "alert-dialog",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        alert_dialog::AlertDialogRoot {
            id: props.id,
            default_open: props.default_open,
            open: props.open,
            on_open_change: props.on_open_change,
            attributes: merged,
            alert_dialog::AlertDialogContent {
                class: "relative bg-background rounded-2xl shadow-lg p-6 w-full max-w-[calc(100%-2rem)] sm:max-w-lg max-h-[85vh] fixed top-[50%] left-[50%] translate-x-[-50%] translate-y-[-50%] z-100 flex flex-col gap-4",
                "data-slot": "alert-dialog-content",
                {props.children}
            }
        }
    }
}

#[component]
pub fn AlertDialogTitle(props: AlertDialogTitleProps) -> Element {
    let base = attributes!(h2 { class: "text-lg leading-none font-semibold", "data-slot": "alert-dialog-title" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        alert_dialog::AlertDialogTitle { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogDescription(props: AlertDialogDescriptionProps) -> Element {
    let base = attributes!(p { class: "text-muted-foreground text-sm", "data-slot": "alert-dialog-description" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        alert_dialog::AlertDialogDescription { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogActions(props: AlertDialogActionsProps) -> Element {
    let base = attributes!(div {
        class: "flex flex-col-reverse gap-2 sm:flex-row sm:justify-end",
        "data-slot": "alert-dialog-actions",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        alert_dialog::AlertDialogActions { attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogCancel(props: AlertDialogCancelProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium bg-secondary text-secondary-foreground shadow-xs hover:bg-accent hover:text-accent-foreground h-control px-4 py-2 cursor-pointer outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "data-slot": "alert-dialog-cancel",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        alert_dialog::AlertDialogCancel { on_click: props.on_click, attributes: merged, {props.children} }
    }
}

#[component]
pub fn AlertDialogAction(props: AlertDialogActionProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium bg-primary text-primary-foreground shadow-xs hover:bg-primary/90 h-control px-4 py-2 cursor-pointer outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "data-slot": "alert-dialog-action",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        alert_dialog::AlertDialogAction { on_click: props.on_click, attributes: merged, {props.children} }
    }
}
