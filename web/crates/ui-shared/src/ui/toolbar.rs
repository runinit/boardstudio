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
//! - ticket 008 repairs the private primitive collection's tab stops and End through owned DOM events.

use dioxus::prelude::*;
use dioxus_primitives::toolbar::{self, ToolbarButtonProps, ToolbarProps, ToolbarSeparatorProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};
use dioxus_web::WebEventExt;
use wasm_bindgen::{JsCast, closure::Closure};

struct ToolbarObserver {
    observer: web_sys::MutationObserver,
    _callback: Closure<dyn FnMut()>,
}

impl Drop for ToolbarObserver {
    fn drop(&mut self) {
        self.observer.disconnect();
    }
}

// The pinned primitive's collection is private; repair focus within this owned toolbar DOM only.
fn buttons(root: &web_sys::Element) -> Vec<web_sys::HtmlElement> {
    let nodes = root
        .query_selector_all("[data-slot=toolbar-button], [data-slot=toggle-group-item]")
        .expect("toolbar item selector");
    (0..nodes.length())
        .filter_map(|index| nodes.item(index)?.dyn_into::<web_sys::HtmlElement>().ok())
        .filter(|item| item.closest("[role=toolbar]").ok().flatten().as_ref() == Some(root))
        .collect()
}

fn sync_tab_stop(root: &web_sys::Element) {
    let items = buttons(root);
    let enabled: Vec<_> = items
        .iter()
        .filter(|item| !item.has_attribute("disabled"))
        .collect();
    let active = root
        .owner_document()
        .and_then(|document| document.active_element());
    let selected = enabled
        .iter()
        .copied()
        .find(|item| active.as_ref() == Some(item.as_ref()))
        .or_else(|| enabled.iter().copied().find(|item| item.tab_index() == 0))
        .or_else(|| enabled.first().copied());
    for item in &items {
        let tabindex = if selected == Some(item) { 0 } else { -1 };
        if item.tab_index() != tabindex {
            item.set_tab_index(tabindex);
        }
    }
}

#[component]
pub fn Toolbar(props: ToolbarProps) -> Element {
    let mut root = use_signal(|| None::<web_sys::Element>);
    let mut observer = use_hook(|| CopyValue::new(None::<ToolbarObserver>));
    use_effect(move || {
        let Some(element) = root() else {
            return;
        };
        let observed = element.clone();
        let callback = Closure::<dyn FnMut()>::new(move || sync_tab_stop(&observed));
        let native = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref())
            .expect("toolbar mutation observer");
        let options = web_sys::MutationObserverInit::new();
        options.set_attributes(true);
        options.set_attribute_filter(&js_sys::Array::of2(&"tabindex".into(), &"disabled".into()));
        options.set_child_list(true);
        options.set_subtree(true);
        native
            .observe_with_options(&element, &options)
            .expect("observe toolbar");
        observer.set(Some(ToolbarObserver {
            observer: native,
            _callback: callback,
        }));
        sync_tab_stop(&element);
    });
    let base = attributes!(div {
        class: "flex items-center gap-1 p-1 rounded-md bg-background shadow-sm w-fit data-[orientation=vertical]:flex-col",
        "data-slot": "toolbar",
        onmounted: move |event: MountedEvent| root.set(event.data().try_as_web_event()),
        onfocusin: move |_| {
            if let Some(element) = root() { sync_tab_stop(&element); }
        },
        onkeydown: move |event: KeyboardEvent| {
            if (props.disabled)() { return; }
            let Some(element) = root() else { return; };
            let items: Vec<_> = buttons(&element).into_iter().filter(|item| !item.has_attribute("disabled")).collect();
            let active = element.owner_document().and_then(|document| document.active_element());
            let Some(index) = items.iter().position(|item| active.as_ref() == Some(item.as_ref())) else { return; };
            let horizontal = (props.horizontal)();
            let next = match event.key() {
                Key::Home => 0,
                Key::End => items.len() - 1,
                Key::ArrowRight if horizontal => (index + 1).min(items.len() - 1),
                Key::ArrowDown if !horizontal => (index + 1).min(items.len() - 1),
                Key::ArrowLeft if horizontal => index.saturating_sub(1),
                Key::ArrowUp if !horizontal => index.saturating_sub(1),
                _ => return,
            };
            event.prevent_default();
            let _ = items[next].focus();
            sync_tab_stop(&element);
        },
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
