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
//! - built-in trigger and floating-list borders removed, cards style (theme contract section 8); focus rings kept;
//! - control height reads the shared theme-independent metric;
//! - upstream's scroll arrows (`use_can_scroll_vertical`) are dropped: the list scrolls natively;
//! - icons come from `crate::ui::icons` (ticket 004);
//! - Field identity and ARIA go to the trigger; its label also names the listbox;
//! - actual open-to-closed transitions restore trigger focus, including controlled closes;
//! - Enter and Space open on keydown and prevent a second native click (ticket 006).
//!
//! Set `text_value` on options when their visible label differs from their value; the primitive
//! uses it for the selected label and typeahead without changing the caller's value.

use dioxus::prelude::*;
use dioxus_primitives::select::{self, SelectGroupLabelProps, SelectOptionProps, SelectProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

pub use dioxus_primitives::select::SelectGroup;

use crate::ui::icons::{Check, ChevronDown};

#[component]
pub fn Select<T: Clone + PartialEq + 'static>(props: SelectProps<T>) -> Element {
    let (open, set_open) =
        dioxus_primitives::use_controlled(props.open, (props.default_open)(), props.on_open_change);
    let mut trigger: Signal<Option<std::rc::Rc<MountedData>>> = use_signal(|| None);
    let was_open = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(false)));
    use_effect(move || {
        let is_open = open();
        if was_open.replace(is_open)
            && !is_open
            && let Some(trigger) = trigger()
        {
            spawn(async move {
                let _ = trigger.set_focus(true).await;
            });
        }
    });
    let base = attributes!(div {
        class: "relative w-fit", "data-slot": "select",
        onkeydown: move |event: KeyboardEvent| {
            if !open() && !(props.disabled)()
                && (event.key() == Key::Enter || event.key() == Key::Character(" ".into()))
            {
                set_open.call(true);
                event.prevent_default();
                event.stop_propagation();
            }
        },
    });
    // Field identity and ARIA describe the interactive trigger, not the layout wrapper.
    let (trigger_attributes, root_attributes): (Vec<_>, Vec<_>) = props
        .attributes
        .into_iter()
        .partition(|attribute| attribute.name == "id" || attribute.name.starts_with("aria-"));
    let list_attributes = trigger_attributes
        .iter()
        .filter(|attribute| matches!(attribute.name, "aria-label" | "aria-labelledby"))
        .cloned()
        .collect::<Vec<_>>();
    let trigger_attributes = merge_attributes(vec![
        attributes!(button {
            onmounted: move |event| trigger.set(Some(event.data()))
        }),
        trigger_attributes,
    ]);
    let merged = merge_attributes(vec![base, root_attributes]);
    rsx! {
        select::Select {
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            open: Some(open()),
            // Restoring focus can blur an option retained by its exit animation.
            on_open_change: move |next| {
                if next != open() {
                    set_open.call(next);
                }
            },
            name: props.name,
            roving_loop: props.roving_loop,
            typeahead_timeout: props.typeahead_timeout,
            attributes: merged,
            select::SelectTrigger {
                class: "w-full p-2 h-control inline-flex items-center justify-between gap-2 text-sm font-medium whitespace-nowrap rounded-md transition-colors outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 [&_svg:not([class*='size-'])]:size-4 bg-field hover:bg-accent hover:text-accent-foreground [&_[data-placeholder=true]]:text-muted-foreground",
                "data-slot": "select-trigger",
                attributes: trigger_attributes,
                select::SelectValue {}
                ChevronDown { class: "text-muted-foreground" }
            }
            select::SelectList {
                class: "absolute top-[calc(100%+4px)] left-0 min-w-full z-50 p-1 rounded-md bg-card shadow-md max-h-[300px] overflow-auto transition-all duration-200 origin-top data-[state=closed]:opacity-0 data-[state=closed]:scale-95 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 data-[state=open]:scale-100",
                "data-slot": "select-content",
                attributes: list_attributes,
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
