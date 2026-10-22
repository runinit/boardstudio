//! Look copied from rust-ui: `app_crates/registry/src/ui/popover.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::popover` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream's CSS anchor positioning is replaced by placement classes keyed to the primitive's `data-side`;
//! - the trigger is filled (`bg-secondary`) instead of outlined, cards style (theme contract section 8); the floating
//! -   content keeps its border, which separates it from a card of the same fill in dark mode.

use dioxus::prelude::*;
use dioxus_primitives::popover::{
    self, PopoverContentProps, PopoverRootProps, PopoverTriggerProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Popover(props: PopoverRootProps) -> Element {
    let base = attributes!(div { class: "relative inline-block", "data-slot": "popover" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        popover::PopoverRoot {
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn PopoverTrigger(props: PopoverTriggerProps) -> Element {
    let base = attributes!(button {
        class: "px-4 py-2 h-9 inline-flex justify-center items-center text-sm font-medium whitespace-nowrap rounded-md transition-colors w-fit focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&_svg:not(:last-child)]:mr-2 [&_svg:not(:first-child)]:ml-2 bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground",
        "data-slot": "popover-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        popover::PopoverTrigger { attributes: merged, {props.children} }
    }
}

#[component]
pub fn PopoverContent(props: PopoverContentProps) -> Element {
    let base = attributes!(div { "data-slot": "popover-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let class = format!(
        "absolute z-50 p-4 rounded-md border bg-card text-card-foreground shadow-md w-[250px] data-[side=bottom]:top-full data-[side=bottom]:mt-2 data-[side=top]:bottom-full data-[side=top]:mb-2 data-[side=bottom]:left-1/2 data-[side=bottom]:-translate-x-1/2 data-[side=top]:left-1/2 data-[side=top]:-translate-x-1/2 data-[side=right]:left-full data-[side=right]:ml-2 data-[side=left]:right-full data-[side=left]:mr-2 data-[side=right]:top-1/2 data-[side=right]:-translate-y-1/2 data-[side=left]:top-1/2 data-[side=left]:-translate-y-1/2 transition-opacity duration-150 data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 {}",
        props.class.unwrap_or_default()
    );
    rsx! {
        popover::PopoverContent {
            class,
            id: props.id,
            side: props.side,
            align: props.align,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn PopoverTitle(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge::tw_merge!(
        "leading-none font-medium mb-3",
        class.as_deref().unwrap_or("")
    );
    rsx! { h3 { "data-slot": "popover-title", class: "{merged}", {children} } }
}

#[component]
pub fn PopoverDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge::tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );
    rsx! { p { "data-slot": "popover-description", class: "{merged}", {children} } }
}
