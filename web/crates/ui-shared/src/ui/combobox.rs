//! Look in the style of rust-ui (`app_crates/registry/src/ui/combobox.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318`) (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::combobox` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - rust-ui has no combobox file at this commit; the input takes `Input`'s classes (filled with `bg-field`,
//! -   no border) and the list and options take `select.rs`'s classes;
//! - icons come from `crate::ui::icons` (ticket 004).

use dioxus::prelude::*;
use dioxus_primitives::combobox::{
    self, ComboboxEmptyProps, ComboboxOptionProps, default_combobox_filter,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

use crate::ui::icons::{Check, ChevronsUpDown};

#[derive(Props, Clone, PartialEq)]
pub struct ComboboxProps<T: Clone + PartialEq + 'static = String> {
    #[props(default)]
    pub value: Option<ReadSignal<Option<T>>>,
    #[props(default)]
    pub default_value: Option<T>,
    #[props(default)]
    pub on_value_change: Callback<Option<T>>,
    #[props(default)]
    pub disabled: ReadSignal<bool>,
    #[props(default)]
    pub open: ReadSignal<Option<bool>>,
    #[props(default)]
    pub default_open: ReadSignal<bool>,
    #[props(default)]
    pub on_open_change: Callback<bool>,
    #[props(default)]
    pub query: ReadSignal<Option<String>>,
    #[props(default)]
    pub default_query: ReadSignal<String>,
    #[props(default)]
    pub on_query_change: Callback<String>,
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,
    #[props(default = Callback::new(|(q, t): (String, String)| default_combobox_filter(&q, &t)))]
    pub filter: Callback<(String, String), bool>,
    #[props(default)]
    pub placeholder: ReadSignal<String>,
    #[props(default)]
    pub aria_label: Option<String>,
    #[props(default)]
    pub list_aria_label: Option<String>,
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    pub children: Element,
}

#[component]
pub fn Combobox<T: Clone + PartialEq + 'static>(props: ComboboxProps<T>) -> Element {
    let base = attributes!(div { class: "relative inline-block w-full", "data-slot": "combobox" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        combobox::Combobox {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            query: props.query,
            default_query: props.default_query,
            on_query_change: props.on_query_change,
            roving_loop: props.roving_loop,
            filter: props.filter,
            attributes: merged,
            div { class: "relative", "data-slot": "combobox-input-wrapper",
                combobox::ComboboxInput {
                    class: "text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground flex h-control w-full min-w-0 rounded-md bg-field pl-3 pr-8 py-1 text-base transition-[color,box-shadow] outline-none disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm focus-visible:ring-ring/50 focus-visible:ring-2",
                    "data-slot": "combobox-input",
                    placeholder: props.placeholder,
                    aria_label: props.aria_label.clone(),
                }
                ChevronsUpDown { class: "pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" }
            }
            combobox::ComboboxList {
                class: "absolute top-[calc(100%+4px)] left-0 min-w-full z-50 p-1 rounded-md bg-card shadow-md max-h-[300px] overflow-auto transition-all duration-200 origin-top data-[state=closed]:opacity-0 data-[state=closed]:scale-95 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 data-[state=open]:scale-100",
                "data-slot": "combobox-list",
                aria_label: props.list_aria_label.clone(),
                {props.children}
            }
        }
    }
}

#[component]
pub fn ComboboxEmpty(props: ComboboxEmptyProps) -> Element {
    let base = attributes!(div { class: "py-6 text-center text-sm text-muted-foreground", "data-slot": "combobox-empty" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        combobox::ComboboxEmpty { attributes: merged, {props.children} }
    }
}

#[component]
pub fn ComboboxOption<T: Clone + PartialEq + 'static>(props: ComboboxOptionProps<T>) -> Element {
    let base = attributes!(div {
        class: "flex gap-2 items-center w-full rounded-sm px-2 py-1.5 text-sm cursor-pointer outline-none transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground data-[highlighted=true]:bg-accent data-[highlighted=true]:text-accent-foreground data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50",
        "data-slot": "combobox-option",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        combobox::ComboboxOption::<T> {
            value: props.value,
            text_value: props.text_value,
            disabled: props.disabled,
            id: props.id,
            index: props.index,
            aria_label: props.aria_label,
            aria_roledescription: props.aria_roledescription,
            attributes: merged,
            {props.children}
            combobox::ComboboxItemIndicator {
                Check { class: "ml-auto size-4 text-muted-foreground" }
            }
        }
    }
}
