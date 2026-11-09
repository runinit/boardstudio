//! Look copied from rust-ui: `app_crates/registry/src/ui/tabs.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::tabs` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - orientation classes key on the primitive's lower-case `data-orientation` and `data-state=active`;
//! -   the primitive's `horizontal` defaults to false, so a row of tabs passes `horizontal: true`;
//! - the active trigger's `dark:border-input` outline is removed, cards style (theme contract section 8);
//! - upstream's `Line` variant is dropped: the mapping uses the default pill tabs.

use dioxus::prelude::*;
use dioxus_primitives::tabs::{self, TabContentProps, TabListProps, TabTriggerProps, TabsProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

#[component]
pub fn Tabs(props: TabsProps) -> Element {
    let base = attributes!(div {
        class: "group/tabs flex gap-2 data-[orientation=horizontal]:flex-col data-[orientation=vertical]:flex-row",
        "data-slot": "tabs",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tabs::Tabs {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TabsList(props: TabListProps) -> Element {
    let base = attributes!(div {
        class: "group/tabs-list inline-flex w-fit items-center justify-center rounded-lg p-[3px] text-muted-foreground bg-muted group-data-[orientation=horizontal]/tabs:h-control-compact group-data-[orientation=vertical]/tabs:h-fit group-data-[orientation=vertical]/tabs:flex-col",
        "data-slot": "tabs-list",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tabs::TabList { attributes: merged, {props.children} }
    }
}

#[component]
pub fn TabsTrigger(props: TabTriggerProps) -> Element {
    let base = attributes!(button {
        class: "relative inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md px-1.5 py-0.5 text-sm font-medium whitespace-nowrap transition-all cursor-pointer select-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50 group-data-[orientation=vertical]/tabs:w-full group-data-[orientation=vertical]/tabs:justify-start text-foreground/60 hover:text-foreground data-[state=active]:text-foreground data-[state=active]:bg-background data-[state=active]:shadow-sm dark:data-[state=active]:bg-input/30",
        "data-slot": "tabs-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tabs::TabTrigger {
            class: props.class,
            id: props.id,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn TabsContent(props: TabContentProps) -> Element {
    let base =
        attributes!(div { class: "flex-1 text-sm outline-none", "data-slot": "tabs-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        tabs::TabContent {
            class: props.class,
            value: props.value,
            id: props.id,
            index: props.index,
            attributes: merged,
            {props.children}
        }
    }
}
