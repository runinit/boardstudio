//! Look copied from rust-ui: `app_crates/registry/src/ui/dropdown_menu.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour uses the public `dioxus_primitives::popover` and controlled-state hooks (rev
//! `9a758255`), with menu keyboard/focus handling here: DropdownMenu's focus collection is private.
//! Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - built-in trigger and floating-content borders removed, cards style (theme contract section 8); focus rings kept;
//! - control height reads the shared theme-independent metric;
//! - menu roles, initial/return focus and typeahead meet ticket 007's accessibility floor;
//! - checkbox/radio items use `role` and `aria_checked` on Item; selection stays caller-owned;
//! - submenus and links are outside the action-menu mapping.

use dioxus::prelude::*;
use dioxus_primitives::dropdown_menu::{
    DropdownMenuContentProps, DropdownMenuItemProps, DropdownMenuProps, DropdownMenuTriggerProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes, popover, use_controlled};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[derive(Clone, Copy)]
struct MenuContext {
    open: Memo<bool>,
    set_open: Callback<bool>,
    disabled: ReadSignal<bool>,
    roving_loop: ReadSignal<bool>,
    trigger_id: Signal<String>,
    trigger: Signal<Option<HtmlElement>>,
    content: Signal<Option<HtmlElement>>,
    search: Signal<(String, f64)>,
}

fn menu_items(content: &HtmlElement) -> Vec<HtmlElement> {
    let Ok(nodes) = content.query_selector_all("[role^=menuitem]:not([aria-disabled=true])") else {
        return Vec::new();
    };
    let mut items: Vec<HtmlElement> = (0..nodes.length())
        .filter_map(|index| nodes.item(index)?.dyn_into().ok())
        .collect();
    items.sort_by_key(|item| {
        item.get_attribute("data-menu-index")
            .and_then(|index| index.parse::<usize>().ok())
    });
    items
}

fn focus_item(items: &[HtmlElement], index: usize) {
    for (position, item) in items.iter().enumerate() {
        item.set_tab_index(if position == index { 0 } else { -1 });
    }
    if let Some(item) = items.get(index) {
        let _ = item.focus();
    }
}

fn item_name(item: &HtmlElement) -> String {
    let name = item.get_attribute("aria-label").unwrap_or_else(|| {
        if let Some(ids) = item.get_attribute("aria-labelledby") {
            let document = item.owner_document().unwrap();
            ids.split_whitespace()
                .filter_map(|id| document.get_element_by_id(id))
                .filter_map(|label| label.text_content())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            item.text_content().unwrap_or_default()
        }
    });
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn navigate(mut ctx: MenuContext, event: KeyboardEvent) {
    if event.key() == Key::Escape {
        event.prevent_default();
        event.stop_propagation();
        ctx.set_open.call(false);
        return;
    }
    let Some(content) = (ctx.content)() else {
        return;
    };
    let items = menu_items(&content);
    if items.is_empty() {
        return;
    }
    let active = content
        .owner_document()
        .and_then(|document| document.active_element());
    let current = items
        .iter()
        .position(|item| Some(item.as_ref()) == active.as_ref());
    let last = items.len() - 1;
    let target = match event.key() {
        Key::ArrowDown => Some(current.map_or(0, |index| {
            if index < last {
                index + 1
            } else if (ctx.roving_loop)() {
                0
            } else {
                last
            }
        })),
        Key::ArrowUp => Some(current.map_or(last, |index| {
            if index > 0 {
                index - 1
            } else if (ctx.roving_loop)() {
                last
            } else {
                0
            }
        })),
        Key::Home => Some(0),
        Key::End => Some(last),
        Key::Character(character)
            if character.chars().count() == 1
                && character != " "
                && !event
                    .modifiers()
                    .intersects(Modifiers::CONTROL | Modifiers::META | Modifiers::ALT) =>
        {
            let now = js_sys::Date::now();
            let (mut prefix, previous) = ctx.search.peek().clone();
            if now - previous > 700.0 {
                prefix.clear();
            }
            prefix.push_str(&character.to_lowercase());
            // Repeating one letter cycles matches; different letters build a prefix.
            let repeated = prefix.chars().all(|ch| Some(ch) == prefix.chars().next());
            let query = if repeated {
                character.to_lowercase()
            } else {
                prefix.clone()
            };
            ctx.search.set((query.clone(), now));
            let start = if query.chars().count() > 1 {
                current.unwrap_or(0)
            } else {
                current.map_or(0, |index| (index + 1) % items.len())
            };
            (0..items.len())
                .map(|offset| (start + offset) % items.len())
                .find(|&index| item_name(&items[index]).starts_with(&query))
        }
        _ => return,
    };
    event.prevent_default();
    event.stop_propagation();
    if let Some(index) = target {
        focus_item(&items, index);
    }
}

#[component]
pub fn DropdownMenu(props: DropdownMenuProps) -> Element {
    let (open, set_open) = use_controlled(props.open, props.default_open, props.on_open_change);
    let trigger_id =
        use_signal(|| format!("action-menu-trigger-{}", dioxus::core::current_scope_id().0));
    let ctx = use_context_provider(|| MenuContext {
        open,
        set_open,
        disabled: props.disabled,
        roving_loop: props.roving_loop,
        trigger_id,
        trigger: Signal::new(None),
        content: Signal::new(None),
        search: Signal::new((String::new(), 0.0)),
    });
    let previous_open = use_hook(|| Rc::new(Cell::new(false)));
    use_effect(move || {
        let was_open = previous_open.replace(open());
        if open() {
            if let Some(content) = (ctx.content)() {
                focus_item(&menu_items(&content), 0);
            }
        } else if was_open {
            let mut search = ctx.search;
            search.set((String::new(), 0.0));
            let document = web_sys::window().unwrap().document().unwrap();
            let active = document.active_element();
            let inside = (ctx.content)().is_some_and(|content| {
                content.contains(active.as_ref().map(|element| element.as_ref()))
            });
            // Outside dismissal keeps the newly focused control, as BoardPopover does.
            if inside || active == document.body().map(Into::into) || active.is_none() {
                if let Some(trigger) = (ctx.trigger)() {
                    let _ = trigger.focus();
                }
            }
        }
    });
    let base = attributes!(div {
        class: "relative inline-block", "data-slot": "dropdown-menu",
        "data-disabled": (props.disabled)(),
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    let id = merged
        .iter()
        .find(|attribute| attribute.name == "id")
        .and_then(|attribute| {
            if let dioxus::core::AttributeValue::Text(value) = &attribute.value {
                Some(value.clone())
            } else {
                None
            }
        });
    rsx! {
        popover::PopoverRoot {
            id,
            open: Some(open()),
            on_open_change: set_open,
            is_modal: false,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn DropdownMenuTrigger(props: DropdownMenuTriggerProps) -> Element {
    let mut ctx: MenuContext = use_context();
    let base = attributes!(button {
        class: "px-4 py-2 h-control inline-flex justify-center items-center text-sm font-medium whitespace-nowrap rounded-md transition-colors w-fit focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&_svg:not([class*='size-'])]:size-4 bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground",
        "data-slot": "dropdown-menu-trigger",
        "data-state": if (ctx.open)() { "open" } else { "closed" },
        "data-disabled": (ctx.disabled)(),
        id: "{ctx.trigger_id}",
        r#type: "button",
        disabled: (ctx.disabled)(),
        aria_haspopup: "menu",
        aria_expanded: (ctx.open)(),
        aria_controls: if (ctx.open)() { (ctx.content)().map(|content| content.id()) } else { None },
        onclick: move |_| {
            if !(ctx.disabled)() { ctx.set_open.call(!(ctx.open)()); }
        },
        onkeydown: move |event: KeyboardEvent| {
            if !(ctx.disabled)() && (matches!(event.key(), Key::Enter | Key::ArrowDown | Key::ArrowUp)
                || event.key() == Key::Character(" ".to_string())) {
                event.prevent_default();
                event.stop_propagation();
                ctx.set_open.call(true);
            }
        },
        onmounted: move |event: MountedEvent| {
            ctx.trigger.set(event.data().downcast::<web_sys::Element>().and_then(|element| element.dyn_ref::<HtmlElement>()).cloned());
        },
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    if let Some(render) = props.r#as {
        render.call(merged)
    } else {
        rsx! { popover::PopoverTrigger { attributes: merged, {props.children} } }
    }
}

#[component]
pub fn DropdownMenuContent(props: DropdownMenuContentProps) -> Element {
    let mut ctx: MenuContext = use_context();
    let base = attributes!(div {
        class: "absolute top-full left-0 mt-1 z-50 p-1 min-w-[180px] rounded-md bg-card shadow-md h-fit transition-all duration-200 data-[state=closed]:opacity-0 data-[state=closed]:scale-95 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 data-[state=open]:scale-100",
        "data-slot": "dropdown-menu-content",
        role: "menu",
        aria_labelledby: (ctx.trigger)().map(|trigger| trigger.id()),
        onkeydown: move |event: KeyboardEvent| navigate(ctx, event),
        onmounted: move |event: MountedEvent| {
            ctx.content.set(event.data().downcast::<web_sys::Element>().and_then(|element| element.dyn_ref::<HtmlElement>()).cloned());
        },
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    rsx! {
        if (ctx.open)() {
            popover::PopoverContent { id: props.id, attributes: merged, {props.children} }
        }
    }
}

#[component]
pub fn DropdownMenuItem<T: Clone + PartialEq + 'static>(
    props: DropdownMenuItemProps<T>,
) -> Element {
    let ctx: MenuContext = use_context();
    let disabled = move || (ctx.disabled)() || (props.disabled)();
    let activate = move || {
        if !disabled() {
            props.on_select.call((props.value)());
            ctx.set_open.call(false);
        }
    };
    let base = attributes!(div {
        class: "flex gap-2 items-center w-full rounded-sm px-2 py-1.5 text-sm cursor-pointer outline-none transition-colors duration-200 text-popover-foreground hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:text-accent-foreground data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 [&_svg:not([class*='size-'])]:size-4",
        "data-slot": "dropdown-menu-item",
        role: "menuitem",
        tabindex: "-1",
        "data-menu-index": (props.index)(),
        "data-disabled": disabled(),
        aria_disabled: disabled(),
        onclick: move |event: MouseEvent| {
            event.stop_propagation();
            activate();
        },
        onkeydown: move |event: KeyboardEvent| {
            if event.key() == Key::Enter || event.key() == Key::Character(" ".to_string()) {
                event.prevent_default();
                event.stop_propagation();
                activate();
            }
        },
    });
    let merged = merge_attributes(vec![base, props.attributes.clone()]);
    rsx! {
        div { ..merged, {props.children} }
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
