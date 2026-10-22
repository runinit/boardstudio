//! Look copied from rust-ui: `app_crates/registry/src/ui/toggle_group.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::toggle_group` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream's variants and spacing are kept as classes; the `Outline` variant's borders are removed and
//! -   the group is filled (`bg-muted`), cards style (theme contract section 8); focus rings kept;
//! - `ToggleGroupAction` (a link styled as an item) is dropped: nothing in the mapping uses it.

use dioxus::prelude::*;
use dioxus_primitives::toggle_group::{self, ToggleGroupProps, ToggleItemProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn ToggleGroup(props: ToggleGroupProps) -> Element {
    let base = attributes!(div {
        class: "group/toggle-group flex w-fit items-center gap-px rounded-md bg-muted p-0.5 data-[orientation=vertical]:flex-col",
        "data-slot": "toggle-group",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        toggle_group::ToggleGroup {
            default_pressed: props.default_pressed,
            pressed: props.pressed,
            on_pressed_change: props.on_pressed_change,
            disabled: props.disabled,
            allow_multiple_pressed: props.allow_multiple_pressed,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ToggleItem(props: ToggleItemProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex flex-1 gap-2 justify-center items-center px-2 min-w-0 h-control-compact rounded-sm text-sm font-medium whitespace-nowrap bg-transparent shadow-none outline-none focus:z-10 focus-visible:z-10 disabled:opacity-50 disabled:pointer-events-none data-[state=on]:bg-background data-[state=on]:text-foreground data-[state=on]:shadow-xs [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 [&_svg]:shrink-0 transition-[color,box-shadow] hover:bg-accent hover:text-accent-foreground focus-visible:ring-ring/50 focus-visible:ring-[3px]",
        "data-slot": "toggle-group-item",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        toggle_group::ToggleItem {
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}
