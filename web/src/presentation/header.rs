//! Header presentation only. Session, history and workspace ownership stay with App.
use boardstudio_web_ui_model::Workspace;
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
use std::rc::Rc;
use wasm_bindgen::{JsCast, closure::Closure};

#[component]
fn HeaderSaveStatus(on_retry_save: EventHandler<()>) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let _ = try_use_context::<Signal<u64>>().map(|v| v());
    let _ = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>().map(|d| d.0());
    let (save_state, save_retryable) = runtime.with_model(|model| {
        let save_state = if model.accepted.is_none() {
            "ready"
        } else {
            super::durability_state(&model.durability)
        };
        let save_retryable = !matches!(
            &model.durability,
            boardstudio_application::Durability::Failed { reason, .. }
                if reason == boardstudio_web_host::host::CONFLICT
        );
        (save_state, save_retryable)
    });
    let mut save_open = use_signal(|| false);
    rsx! {
        Popover { is_modal: false, open: save_open(), on_open_change: move |o| save_open.set(o),
            PopoverTrigger { class: "header-save", "data-state": save_state, aria_haspopup: "dialog", aria_expanded: save_open(),
                aria_label: match save_state { "saved" => "Saved locally", "saving" => "Saving locally", "failed" => "Local save failed", _ => "Local save status unavailable" },
                span { class: "header-save-dot", "aria-hidden": "true" }
                span { class: "header-save-label", match save_state { "saved" => "Saved", "saving" => "Saving…", "failed" => "Save failed", "ready" => "Ready", _ => "Pending" } }
            }
            PopoverContent { id: "header-save-details", class: "header-save-content", aria_label: "Local save details",
                div { role: "status",
                    match save_state {
                        "saved" => "Changes are saved in this browser.",
                        "saving" => "Saving changes in this browser…",
                        "failed" if !save_retryable => boardstudio_web_host::host::CONFLICT,
                        "failed" => "Changes could not be saved locally.",
                        _ => "Local save status is unavailable.",
                    }
                }
                if save_state == "failed" && save_retryable {
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| on_retry_save.call(()), "Retry save" }
                }
            }
        }
    }
}

#[component]
pub(super) fn Header(
    project_menu: Element,
    workspace: Signal<Workspace>,
    show_workspaces: bool,
    has_project: bool,
    on_retry_save: EventHandler<()>,
    mut settings_open: Signal<bool>,
    palette_open: ReadSignal<bool>,
    on_search: EventHandler<()>,
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
        query.map(|q| {
            let obs = q.clone();
            let cb = Closure::wrap(
                Box::new(move |_: web_sys::Event| compact.set(obs.matches())) as Box<dyn FnMut(_)>,
            );
            let _ = q.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref());
            Rc::new((q, cb))
        })
    });
    use_drop(move || {
        if let Some(l) = listener {
            let _ =
                l.0.remove_event_listener_with_callback("change", l.1.as_ref().unchecked_ref());
        }
    });
    let mut mobile_open = use_signal(|| false);
    use_effect(move || {
        let _ = (workspace(), compact());
        mobile_open.set(false);
    });
    use_effect(move || {
        if settings_open() && compact() {
            mobile_open.set(true);
        }
    });

    let export = Workspace::Export.as_str();
    let commands = rsx! {
        div { class: "header-commands",
            if show_workspaces { super::workspace_navigation::WorkspaceObjectNavigation { workspace } }
            Button { variant: ButtonVariant::Ghost, aria_label: "Search commands", aria_haspopup: "dialog", aria_expanded: palette_open(),
                onclick: move |_| { mobile_open.set(false); on_search.call(()); }, "Search" Kbd { "Ctrl/Cmd K" } }
            if !compact() { HeaderSaveStatus { on_retry_save } }
            if has_project {
                div { role: "group", aria_label: "Edit history", class: "header-history",
                    Button { variant: ButtonVariant::Ghost, aria_label: "Undo", onclick: move |_| { mobile_open.set(false); on_undo.call(()); }, "Undo" Kbd { "Ctrl/Cmd Z" } }
                    Button { variant: ButtonVariant::Ghost, aria_label: "Redo", onclick: move |_| { mobile_open.set(false); on_redo.call(()); }, "Redo" Kbd { "Ctrl/Cmd Shift Z / Ctrl Y" } }
                }
            }
            {panel_controls}
            ThemePicker { open: settings_open }
            Button { id: "m1-tab-{export}", aria_label: export, aria_pressed: workspace() == Workspace::Export, onclick: move |_| on_export.call(()), "{export}" }
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
            if compact() {
                HeaderSaveStatus { on_retry_save }
                Popover { is_modal: false, open: mobile_open(), on_open_change: move |o| { mobile_open.set(o); if !o { settings_open.set(false); } },
                    PopoverTrigger { id: "header-mobile-trigger", aria_label: "Workbench menu", aria_haspopup: "dialog", aria_expanded: mobile_open(), "Menu" ChevronDown {} }
                    PopoverContent { id: "header-mobile-commands", class: "header-mobile-menu", aria_label: "Workbench menu", {commands} }
                }
            }
        }
    }
}
