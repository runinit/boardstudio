//! Header presentation only. Session, history and workspace ownership stay with App.
use boardstudio_web_ui_shared::{
    project_menu::ThemePicker,
    ui::{
        button::{Button, ButtonVariant},
        icons::ChevronDown,
        kbd::Kbd,
        popover::{Popover, PopoverContent, PopoverTrigger},
    },
};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};

#[component]
pub(super) fn Header(
    project_menu: Element,
    workspace: Signal<&'static str>,
    show_workspaces: bool,
    has_project: bool,
    save_state: &'static str,
    mut settings_open: Signal<bool>,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_export: EventHandler<()>,
    panel_controls: Element,
) -> Element {
    let query = use_hook(|| {
        web_sys::window().and_then(|w| w.match_media("(max-width: 1250px)").ok().flatten())
    });
    let mut compact = use_signal(|| query.as_ref().is_some_and(|q| q.matches()));
    let listener = use_hook(move || {
        query.map(|query| {
            let observed = query.clone();
            let callback =
                Closure::wrap(
                    Box::new(move |_: web_sys::Event| compact.set(observed.matches()))
                        as Box<dyn FnMut(_)>,
                );
            let _ =
                query.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref());
            Rc::new((query, callback))
        })
    });
    use_drop(move || {
        if let Some(listener) = listener {
            let _ = listener
                .0
                .remove_event_listener_with_callback("change", listener.1.as_ref().unchecked_ref());
        }
    });
    let mut mobile_open = use_signal(|| false);
    let mut save_open = use_signal(|| false);
    let mut menu_content = use_signal(|| None::<web_sys::HtmlElement>);
    let previous_open = use_hook(|| Rc::new(Cell::new(false)));
    use_effect(move || {
        let was_open = previous_open.replace(mobile_open());
        let Some(content) = menu_content() else {
            return;
        };
        let active = content
            .owner_document()
            .and_then(|document| document.active_element());
        let inside = content.contains(active.as_ref().map(|element| element.as_ref()));
        if mobile_open() {
            if !inside
                && let Ok(Some(first)) = content.query_selector("button:not(:disabled)")
                && let Some(first) = first.dyn_ref::<web_sys::HtmlElement>()
            {
                let _ = first.focus();
            }
        } else if was_open
            && (inside
                || active.is_none()
                || active.is_some_and(|element| element.tag_name() == "BODY"))
        {
            // The primitive's animated close can retain focus inside its closing content.
            if let Some(trigger) = content
                .owner_document()
                .and_then(|document| document.get_element_by_id("header-mobile-trigger"))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = trigger.focus();
            }
        }
    });
    use_effect(move || {
        let _ = (workspace(), compact());
        mobile_open.set(false);
    });
    use_effect(move || {
        if settings_open() && compact() {
            mobile_open.set(true);
        }
    });
    let save_status = rsx! {
            Popover { is_modal: false, open: save_open(), on_open_change: move |open| save_open.set(open),
                PopoverTrigger { class: "header-save", "data-state": save_state,
                    aria_haspopup: "dialog", aria_expanded: save_open(),
                    aria_controls: save_open().then_some("header-save-details"),
                    aria_label: match save_state { "saved" => "Saved locally", "saving" => "Saving locally", "failed" => "Local save failed", _ => "Local save status unavailable" },
                    span { class: "header-save-dot", "aria-hidden": "true" }
                    span { class: "header-save-label", match save_state { "saved" => "Saved", "saving" => "Saving…", "failed" => "Save failed", "ready" => "Ready", _ => "Pending" } }
                }
                PopoverContent { id: "header-save-details", class: "header-save-content", aria_label: "Local save details",
                    div { role: "status",
                        match save_state {
                            "saved" => "Changes are saved in this browser. Use Save project copy for a portable backup.",
                            "saving" => "Saving changes in this browser…",
                            "failed" => "Changes could not be saved locally. Check available browser storage before editing further.",
                            _ => "Local save status is unavailable.",
                        }
                    }
                }
            }
    };
    let commands = rsx! {
        div { class: "header-commands",
            onmounted: move |event: MountedEvent| menu_content.set(event.data().downcast::<web_sys::Element>().and_then(|element| element.dyn_ref::<web_sys::HtmlElement>()).cloned()),
            if show_workspaces { super::workspace_navigation::WorkspaceObjectNavigation { workspace } }
            // Ticket 012 owns opening the Command palette and its shortcut.
            Button { variant: ButtonVariant::Ghost, aria_label: "Search commands", "Search" Kbd { "Ctrl/Cmd K" } }
            if !compact() { {save_status.clone()} }
            if has_project {
                div { role: "group", aria_label: "Edit history", class: "header-history",
                    Button { variant: ButtonVariant::Ghost, aria_label: "Undo", onclick: move |_| on_undo.call(()), "Undo" }
                    Button { variant: ButtonVariant::Ghost, aria_label: "Redo", onclick: move |_| on_redo.call(()), "Redo" }
                }
            }
            {panel_controls}
            ThemePicker { open: settings_open }
            Button { id: "m1-tab-Export", aria_label: "Export", aria_pressed: workspace() == "Export",
                onclick: move |_| on_export.call(()), "Export" }
        }
    };
    rsx! {
        header { class: "workbench-header", "data-slot": "workbench-header",
            div { class: "header-brand", role: "img", aria_label: "BoardStudio",
                svg { view_box: "0 0 24 20", fill: "none", stroke: "currentColor", stroke_width: "var(--icon-stroke)", "aria-hidden": "true",
                    rect { x: "1", y: "2", width: "22", height: "16", rx: "2" }
                    path { d: "M1 7h22M1 12h22M6 2v10M12 2v10M18 2v10M5 12v6M19 12v6" }
                }
                span { "BoardStudio" }
            }
            {project_menu}
            if !compact() { {commands.clone()} }
            if compact() { {save_status} }
            if compact() {
                Popover { is_modal: false, open: mobile_open(), on_open_change: move |open| {
                    mobile_open.set(open);
                    if !open { settings_open.set(false); }
                },
                    PopoverTrigger { id: "header-mobile-trigger", aria_label: "Workbench menu", aria_haspopup: "dialog", aria_expanded: mobile_open(),
                        aria_controls: mobile_open().then_some("header-mobile-commands"), "Menu" ChevronDown {} }
                    PopoverContent { id: "header-mobile-commands", class: "header-mobile-menu", aria_label: "Workbench menu",
                        {commands} }
                }
            }
        }
    }
}
