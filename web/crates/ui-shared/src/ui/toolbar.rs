//! Look copied from rust-ui: `app_crates/registry/src/ui/toolbar.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::toolbar` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - built-in border removed, cards style (theme contract section 8); focus rings kept;
//! - upstream's list, link and toggle-group parts are dropped: Toolbar holds `ToolbarButton`s and a
//! -   `ToggleGroup` (mapping section 1), and its separator is the primitive's.

use dioxus::prelude::*;
use dioxus_primitives::toolbar::{self, ToolbarButtonProps, ToolbarProps, ToolbarSeparatorProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Toolbar(props: ToolbarProps) -> Element {
    let base = attributes!(div {
        class: "flex items-center gap-1 p-1 rounded-md bg-background shadow-sm w-fit data-[orientation=vertical]:flex-col",
        "data-slot": "toolbar",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        toolbar::Toolbar {
            aria_label: props.aria_label,
            disabled: props.disabled,
            horizontal: props.horizontal,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ToolbarButton(props: ToolbarButtonProps) -> Element {
    let base = attributes!(button {
        class: "inline-flex items-center justify-center gap-2 rounded-sm px-2 h-control-compact text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50 cursor-pointer outline-none focus-visible:ring-2 focus-visible:ring-ring bg-transparent hover:bg-accent hover:text-accent-foreground [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 [&_svg]:shrink-0",
        "data-slot": "toolbar-button",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        toolbar::ToolbarButton {
            index: props.index,
            disabled: props.disabled,
            on_click: props.on_click,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn ToolbarSeparator(props: ToolbarSeparatorProps) -> Element {
    let base = attributes!(div {
        class: "shrink-0 bg-border data-[orientation=vertical]:mx-1 data-[orientation=vertical]:self-stretch data-[orientation=vertical]:w-[1px] data-[orientation=horizontal]:my-1 data-[orientation=horizontal]:h-[1px] data-[orientation=horizontal]:self-stretch",
        "data-slot": "toolbar-separator",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        toolbar::ToolbarSeparator { decorative: props.decorative, horizontal: props.horizontal, attributes: merged }
    }
}
