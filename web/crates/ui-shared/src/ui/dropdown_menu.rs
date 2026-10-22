//! Look copied from rust-ui: `app_crates/registry/src/ui/dropdown_menu.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::dropdown_menu` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - built-in trigger and floating-content borders removed, cards style (theme contract section 8); focus rings kept;
//! - control height reads the shared theme-independent metric;
//! - only the parts the primitive drives are kept (menu, trigger, content, item) plus the presentational label, group
//! -   and separator; sub-menus, links and radio items wait for ticket 007, which also fixes the roles and focus
//! -   ([accessibility floor](../../../../../docs/plans/rust-ui-migration/accessibility-floor.md)).

use dioxus::prelude::*;
use dioxus_primitives::dropdown_menu::{
    self, DropdownMenuContentProps, DropdownMenuItemProps, DropdownMenuProps,
    DropdownMenuTriggerProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn DropdownMenu(props: DropdownMenuProps) -> Element {
    let base = attributes!(div { class: "relative inline-block", "data-slot": "dropdown-menu" });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    rsx! {
        dropdown_menu::DropdownMenu {
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            disabled: props.disabled,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn DropdownMenuTrigger(props: DropdownMenuTriggerProps) -> Element {
    let base = attributes!(button {
        class: "px-4 py-2 h-control inline-flex justify-center items-center text-sm font-medium whitespace-nowrap rounded-md transition-colors w-fit focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&_svg:not([class*='size-'])]:size-4 bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground",
        "data-slot": "dropdown-menu-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        dropdown_menu::DropdownMenuTrigger { as: props.r#as, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DropdownMenuContent(props: DropdownMenuContentProps) -> Element {
    let base = attributes!(div {
        class: "absolute top-full left-0 mt-1 z-50 p-1 min-w-[180px] rounded-md bg-card shadow-md h-fit transition-all duration-200 data-[state=closed]:opacity-0 data-[state=closed]:scale-95 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 data-[state=open]:scale-100",
        "data-slot": "dropdown-menu-content",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    rsx! {
        dropdown_menu::DropdownMenuContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn DropdownMenuItem<T: Clone + PartialEq + 'static>(
    props: DropdownMenuItemProps<T>,
) -> Element {
    let base = attributes!(div {
        class: "flex gap-2 items-center w-full rounded-sm px-2 py-1.5 text-sm cursor-pointer outline-none transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:text-accent-foreground data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 [&_svg:not([class*='size-'])]:size-4",
        "data-slot": "dropdown-menu-item",
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    rsx! {
        dropdown_menu::DropdownMenuItem {
            disabled: props.disabled,
            value: props.value,
            index: props.index,
            on_select: props.on_select,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn DropdownMenuLabel(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge::tw_merge!(
        "px-2 py-1.5 text-sm font-medium data-inset:pl-8 mb-1",
        class.as_deref().unwrap_or("")
    );
    rsx! { span { "data-slot": "dropdown-menu-label", class: "{merged}", {children} } }
}

#[component]
pub fn DropdownMenuGroup(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge::tw_merge!("group", class.as_deref().unwrap_or(""));
    rsx! { div { "data-slot": "dropdown-menu-group", role: "group", class: "{merged}", {children} } }
}

#[component]
pub fn DropdownMenuSeparator(#[props(into, optional)] class: Option<String>) -> Element {
    let merged = tw_merge::tw_merge!("-mx-1 my-1 h-px bg-border", class.as_deref().unwrap_or(""));
    rsx! { div { "data-slot": "dropdown-menu-separator", role: "separator", class: "{merged}" } }
}

#[cfg(test)]
#[path = "../action_menu_tests.rs"]
mod action_menu_tests;
