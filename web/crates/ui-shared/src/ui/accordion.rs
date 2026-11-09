//! Look copied from rust-ui: `app_crates/registry/src/ui/accordion.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::accordion` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream's hidden radio inputs are replaced by the primitive's trigger and content;
//! - the item divider (`divide-y`) is kept: it separates rows, it is not an outline;
//! - icons come from `crate::ui::icons` (ticket 004).

use dioxus::prelude::*;
use dioxus_primitives::accordion::{
    self, AccordionContentProps, AccordionItemProps, AccordionProps, AccordionTriggerProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

use crate::ui::icons::ChevronDown;

#[component]
pub fn Accordion(props: AccordionProps) -> Element {
    let base = attributes!(div { class: "divide-y divide-input w-full", "data-slot": "accordion" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        accordion::Accordion {
            id: props.id,
            allow_multiple_open: props.allow_multiple_open,
            disabled: props.disabled,
            collapsible: props.collapsible,
            horizontal: props.horizontal,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn AccordionItem(props: AccordionItemProps) -> Element {
    let base =
        attributes!(div { class: "group/accordion-item w-full", "data-slot": "accordion-item" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        accordion::AccordionItem {
            disabled: props.disabled,
            default_open: props.default_open,
            on_change: props.on_change,
            on_trigger_click: props.on_trigger_click,
            index: props.index,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn AccordionTrigger(props: AccordionTriggerProps) -> Element {
    let base = attributes!(button {
        class: "flex w-full justify-between items-center p-3 text-left cursor-pointer outline-none rounded-md [&_svg:not([class*='size-'])]:size-4 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        "data-slot": "accordion-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        accordion::AccordionTrigger { id: props.id, attributes: merged,
            {props.children}
            ChevronDown { class: "shrink-0 transition-transform duration-200 group-data-[open=true]/accordion-item:rotate-180" }
        }
    }
}

#[component]
pub fn AccordionContent(props: AccordionContentProps) -> Element {
    let base = attributes!(div { class: "p-3 pt-0", "data-slot": "accordion-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        accordion::AccordionContent { id: props.id, attributes: merged, {props.children} }
    }
}

#[component]
pub fn AccordionTitle(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge::tw_merge!("text-sm font-medium", class.as_deref().unwrap_or(""));
    rsx! { span { "data-slot": "accordion-title", class: "{merged}", {children} } }
}

#[component]
pub fn AccordionDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge::tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );
    rsx! { p { "data-slot": "accordion-description", class: "{merged}", {children} } }
}
