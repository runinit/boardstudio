//! Header presentation only. Session, history and workspace ownership stay with App.
use boardstudio_web_ui_shared::{
    project_menu::ThemePicker,
    ui::{
        button::{Button, ButtonVariant, keep_space_for_activation},
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
    palette_open: ReadSignal<bool>,
    on_search: EventHandler<()>,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_export: EventHandler<()>,
    panel_controls: Element,
    /// Ticket 019: a waiting build the Toast already announced. The caller shows
    /// this notice once the Toast is dismissed and hides it when no update waits.
    update_available: bool,
    on_update: EventHandler<()>,
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
            Button { variant: ButtonVariant::Ghost, aria_label: "Search commands",
                aria_haspopup: "dialog", aria_expanded: palette_open(),
                aria_controls: palette_open().then(|| "command-palette".to_owned()),
                onclick: move |_| { mobile_open.set(false); on_search.call(()); }, "Search" Kbd { "Ctrl/Cmd K" } }
            if !compact() { {save_status.clone()} }
            if has_project {
                div { role: "group", aria_label: "Edit history", class: "header-history",
                    Button { variant: ButtonVariant::Ghost, aria_label: "Undo", onclick: move |_| { mobile_open.set(false); on_undo.call(()); }, "Undo" Kbd { "Ctrl/Cmd Z" } }
                    Button { variant: ButtonVariant::Ghost, aria_label: "Redo", onclick: move |_| { mobile_open.set(false); on_redo.call(()); }, "Redo" Kbd { "Ctrl/Cmd Shift Z / Ctrl Y" } }
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
            if update_available {
                div { class: "header-update",
                    span { class: "header-update-dot", "aria-hidden": "true" }
                    span { class: "header-update-label", role: "status", "Update Available" }
                    Button { variant: ButtonVariant::Ghost, aria_label: "Update Now",
                        onclick: move |_| on_update.call(()),
                        onkeydown: move |event: KeyboardEvent| keep_space_for_activation(&event),
                        "Update Now" }
                }
            }
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
                        inert: (!mobile_open()).then_some(""),
                        {commands} }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_ui_model::state::{AppearanceState, PreferenceStorageWarning, ThemeState};
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use std::cell::Cell;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// Ticket 019: the persistent header-bar notice with a working Update Now.
    /// It renders in the header root (both compact and full layouts) whenever
    /// the caller reports a waiting update, and stays out of the mobile menu.
    #[derive(Clone)]
    struct UpdateProbe {
        available: bool,
        fired: Rc<Cell<u32>>,
    }

    fn update_fixture() -> Element {
        let probe = use_context::<UpdateProbe>();
        // ThemePicker renders in the full layout, so provide its contexts: the
        // shared shard may run this fixture at desktop width when an earlier
        // test leaves the viewport wide.
        let preference_warning = use_signal(|| false);
        use_context_provider(|| PreferenceStorageWarning(preference_warning));
        let appearance = use_signal(|| "light");
        use_context_provider(|| AppearanceState(appearance));
        let theme = use_signal(|| "original");
        use_context_provider(|| ThemeState(theme));
        let workspace = use_signal(|| "Layout");
        let settings_open = use_signal(|| false);
        let palette_open = use_signal(|| false);
        let fired = probe.fired.clone();
        rsx! {
            Header {
                project_menu: rsx! {},
                workspace,
                show_workspaces: true,
                has_project: true,
                save_state: "saved",
                settings_open,
                palette_open,
                on_search: EventHandler::new(move |_| {}),
                on_undo: EventHandler::new(move |_| {}),
                on_redo: EventHandler::new(move |_| {}),
                on_export: EventHandler::new(move |_| {}),
                panel_controls: rsx! {},
                update_available: probe.available,
                on_update: EventHandler::new(move |_| fired.set(fired.get() + 1)),
            }
        }
    }

    fn update_root(available: bool) -> (web_sys::Element, Rc<Cell<u32>>) {
        let fired = Rc::new(Cell::new(0));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(update_fixture);
        dom.provide_root_context(UpdateProbe {
            available,
            fired: fired.clone(),
        });
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        (root, fired)
    }

    #[wasm_bindgen_test]
    async fn header_update_notice_offers_a_working_update_now() {
        let (root, fired) = update_root(true);
        wait_until("the header notice mounts", || {
            by_role_name(&root, "button", "Update Now").is_some()
        })
        .await;
        let notice = by_role_name(&root, "status", "Update Available").unwrap();
        assert!(
            notice.closest(".workbench-header").unwrap().is_some(),
            "the notice lives in the header bar"
        );
        assert!(
            notice.closest(".header-mobile-menu").unwrap().is_none(),
            "the notice is not buried in the mobile menu"
        );
        let update = by_role_name(&root, "button", "Update Now").unwrap();
        assert_eq!(update.tab_index(), 0, "Update Now is a keyboard tab stop");
        update.focus().unwrap();
        assert!(update.matches(":focus").unwrap());
        update.click();
        wait_until("Update Now fires", || fired.get() == 1).await;
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn header_update_notice_is_absent_without_a_waiting_update() {
        let (root, fired) = update_root(false);
        wait_until("the header mounts", || {
            by_role_name(&root, "img", "BoardStudio").is_some()
        })
        .await;
        assert!(by_role_name(&root, "button", "Update Now").is_none());
        assert!(by_role_name(&root, "status", "Update Available").is_none());
        assert_eq!(fired.get(), 0);
        root.remove();
    }
}
