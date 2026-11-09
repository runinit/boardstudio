//! Look copied from rust-ui: `app_crates/registry/src/ui/collapsible.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::collapsible` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - the content keeps upstream's grid-rows open/close transition, keyed to the primitive's `data-state`.

use dioxus::prelude::*;
use dioxus_primitives::collapsible::{
    self, CollapsibleContentProps, CollapsibleProps, CollapsibleTriggerProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Collapsible(props: CollapsibleProps) -> Element {
    let base = attributes!(div { "data-slot": "collapsible" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        collapsible::Collapsible {
            keep_mounted: props.keep_mounted,
            default_open: props.default_open,
            disabled: props.disabled,
            open: props.open,
            on_open_change: props.on_open_change,
            as: props.r#as,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn CollapsibleTrigger(props: CollapsibleTriggerProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex items-center gap-2 rounded-md outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "data-slot": "collapsible-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        collapsible::CollapsibleTrigger { as: props.r#as, attributes: merged, {props.children} }
    }
}

#[component]
pub fn CollapsibleContent(props: CollapsibleContentProps) -> Element {
    let base = attributes!(div {
        class: "overflow-hidden",
        "data-slot": "collapsible-content",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        collapsible::CollapsibleContent { id: props.id, attributes: merged, {props.children} }
    }
}
