//! Look copied from rust-ui: `app_crates/registry/src/ui/select.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::select` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - the trigger is filled with `bg-field` and has no border, cards style (theme contract section 8); the floating
//! -   list keeps its border; focus rings kept;
//! - upstream's scroll arrows (`use_can_scroll_vertical`) are dropped: the list scrolls natively;
//! - icons come from `crate::ui::icons` (ticket 004);
//! - the list's accessible name and focus return are ticket 006 (accessibility floor).

use dioxus::prelude::*;
use dioxus_primitives::select::{self, SelectGroupLabelProps, SelectOptionProps, SelectProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

pub use dioxus_primitives::select::SelectGroup;

use crate::ui::icons::{Check, ChevronDown};

#[component]
pub fn Select<T: Clone + PartialEq + 'static>(props: SelectProps<T>) -> Element {
    let base = attributes!(div { class: "relative w-fit", "data-slot": "select" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        select::Select {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            name: props.name,
            roving_loop: props.roving_loop,
            typeahead_timeout: props.typeahead_timeout,
            attributes: merged,
            select::SelectTrigger {
                class: "w-full p-2 h-9 inline-flex items-center justify-between gap-2 text-sm font-medium whitespace-nowrap rounded-md transition-colors outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 [&_svg:not([class*='size-'])]:size-4 bg-field hover:bg-accent hover:text-accent-foreground [&_[data-placeholder=true]]:text-muted-foreground",
                "data-slot": "select-trigger",
                select::SelectValue {}
                ChevronDown { class: "text-muted-foreground" }
            }
            select::SelectList {
                class: "absolute top-[calc(100%+4px)] left-0 min-w-full z-50 p-1 rounded-md border bg-card shadow-md max-h-[300px] overflow-auto transition-all duration-200 origin-top data-[state=closed]:opacity-0 data-[state=closed]:scale-95 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 data-[state=open]:scale-100",
                "data-slot": "select-content",
                {props.children}
            }
        }
    }
}

#[component]
pub fn SelectGroupLabel(props: SelectGroupLabelProps) -> Element {
    let base = attributes!(div { class: "px-2 py-1.5 text-sm font-medium mb-1", "data-slot": "select-label" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        select::SelectGroupLabel { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn SelectOption<T: Clone + PartialEq + 'static>(props: SelectOptionProps<T>) -> Element {
    let base = attributes!(div {
        class: "group flex gap-2 items-center w-full rounded-sm px-2 py-1.5 text-sm cursor-pointer outline-none transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:text-accent-foreground data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 [&_svg:not([class*='size-'])]:size-4",
        "data-slot": "select-option",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        select::SelectOption::<T> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            aria_label: props.aria_label,
            aria_roledescription: props.aria_roledescription,
            attributes: merged,
            {props.children}
            select::SelectItemIndicator {
                Check { class: "ml-auto size-4 text-muted-foreground" }
            }
        }
    }
}
