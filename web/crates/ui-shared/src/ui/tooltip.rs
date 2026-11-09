//! Look copied from rust-ui: `app_crates/registry/src/ui/tooltip.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::tooltip` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream's CSS-hover reveal and arrow are replaced by the primitive's open state and placement classes
//! -   keyed to `data-side`.

use dioxus::prelude::*;
use dioxus_primitives::tooltip::{self, TooltipContentProps, TooltipProps, TooltipTriggerProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Tooltip(props: TooltipProps) -> Element {
    let base = attributes!(div { class: "relative inline-block", "data-slot": "tooltip" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tooltip::Tooltip {
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TooltipTrigger(props: TooltipTriggerProps) -> Element {
    let base = attributes!(button { class: "inline-flex", "data-slot": "tooltip-trigger" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tooltip::TooltipTrigger { id: props.id, as: props.r#as, attributes: merged, {props.children} }
    }
}

#[component]
pub fn TooltipContent(props: TooltipContentProps) -> Element {
    let base = attributes!(div {
        class: "absolute z-50 rounded-md py-2 px-2.5 text-xs whitespace-nowrap shadow-lg text-background bg-foreground/90 data-[side=bottom]:top-full data-[side=bottom]:mt-2 data-[side=top]:bottom-full data-[side=top]:mb-2 data-[side=bottom]:left-1/2 data-[side=bottom]:-translate-x-1/2 data-[side=top]:left-1/2 data-[side=top]:-translate-x-1/2 data-[side=right]:left-full data-[side=right]:ml-2 data-[side=left]:right-full data-[side=left]:mr-2 data-[side=right]:top-1/2 data-[side=right]:-translate-y-1/2 data-[side=left]:top-1/2 data-[side=left]:-translate-y-1/2",
        "data-slot": "tooltip-content",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tooltip::TooltipContent { id: props.id, side: props.side, align: props.align, attributes: merged, {props.children} }
    }
}
