use crate::ui::{
    button::{Button, ButtonVariant},
    popover::{Popover, PopoverContent, PopoverTrigger},
    sheet::{Sheet, SheetClose, SheetContent, SheetDirection},
    sidenav::Sidenav,
    tabs::{Tabs, TabsContent, TabsList, TabsTrigger},
};
use dioxus::prelude::*;
use policy::decode_settings;
pub use policy::{PanelMode, PanelSettings, PanelSide};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{HtmlElement, Storage};

mod policy;

/// Open the Parts workspace with its browser panel visible.
pub fn browse_parts_workspace(
    mut workspace: Signal<&'static str>,
    mut compact_open: Signal<bool>,
    mut settings: Signal<PanelSettings>,
    compact: bool,
) {
    workspace.set("Parts");
    if compact {
        compact_open.set(true);
        return;
    }
    let mut current = settings();
    if current.mode != PanelMode::Pinned {
        current.mode = PanelMode::Pinned;
        settings.set(current);
    }
}

/// Apply ordinary workspace panel defaults without overriding an explicit reveal.
pub fn use_workspace_panel_defaults(
    workspace: &'static str,
    mut objects_open: Signal<bool>,
    mut inspector_open: Signal<bool>,
) -> Signal<Option<&'static str>> {
    let mut requested_workspace = use_signal(|| None::<&'static str>);
    use_effect(use_reactive((&workspace,), move |_| {
        let requested = *requested_workspace.peek();
        if requested.is_some() {
            requested_workspace.set(None);
        }
        if requested == Some(workspace) {
            return;
        }
        if workspace == "Keymap" || workspace == "Case" {
            objects_open.set(false);
            inspector_open.set(true);
        }
    }));
    requested_workspace
}

/// Header controls use the App-owned compact panel signals.
#[component]
pub fn CompactPanelControls() -> Element {
    let state = use_context::<super::CompactPanelState>();
    let compact = use_compact_viewport();
    rsx! {
        if compact() {
            nav { class: "dock-controls", aria_label: "Panel visibility",
                for (side, label) in [(PanelSide::Objects, "Objects"), (PanelSide::Inspector, "Inspector")] {
                    Button {
                        id: toggle_id(side), variant: ButtonVariant::Ghost, aria_label: label,
                        aria_expanded: match side { PanelSide::Objects => (state.objects_open)(), PanelSide::Inspector => (state.inspector_open)() },
                        aria_haspopup: "dialog", aria_controls: "workspace-panel-sheet",
                        onclick: move |_| toggle_compact_panel(state, side),
                        "{label}"
                    }
                }
            }
        }
    }
}

// A mobile menu can close/unmount its opener while the Sheet stays open.
fn focus_return_candidates() -> Vec<HtmlElement> {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return Vec::new();
    };
    let Some(active) = document
        .active_element()
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return Vec::new();
    };
    let mut candidates = vec![active.clone()];
    if let Some(content) = active.closest("[role=dialog]").ok().flatten()
        && !content.id().is_empty()
        && let Ok(controls) = document.query_selector_all("[aria-controls]")
    {
        for index in 0..controls.length() {
            if let Some(control) = controls
                .item(index)
                .and_then(|e| e.dyn_into::<HtmlElement>().ok())
                && control
                    .get_attribute("aria-controls")
                    .is_some_and(|ids| ids.split_whitespace().any(|id| id == content.id()))
            {
                candidates.push(control);
            }
        }
    }
    candidates
}

fn focus_target_available(target: &HtmlElement) -> bool {
    let rect = target.get_bounding_client_rect();
    target.is_connected()
        && rect.width() > 0.0
        && rect.height() > 0.0
        && target
            .closest("[hidden], [inert], [aria-hidden=true]")
            .ok()
            .flatten()
            .is_none()
        && web_sys::window()
            .and_then(|w| w.get_computed_style(target).ok().flatten())
            .and_then(|s| s.get_property_value("visibility").ok())
            .is_some_and(|v| v == "visible")
}

fn toggle_compact_panel(state: super::CompactPanelState, side: PanelSide) {
    let was_open = match side {
        PanelSide::Objects => (state.objects_open)(),
        PanelSide::Inspector => (state.inspector_open)(),
    };
    set_bool(state.objects_open, side == PanelSide::Objects && !was_open);
    set_bool(
        state.inspector_open,
        side == PanelSide::Inspector && !was_open,
    );
}

/// Canvas stays in the same child position while only the dock presentation changes.
#[component]
pub fn DockLayout(
    objects: Option<Element>,
    inspector: Option<Element>,
    children: Element,
) -> Element {
    let state = use_context::<super::CompactPanelState>();
    let compact = use_compact_viewport();
    let mut sheet_open = use_signal(|| false);
    let requested_before = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    let opener = use_hook(|| Rc::new(RefCell::new(Vec::<HtmlElement>::new())));
    use_effect(move || {
        let open = sheet_open();
        let requested = compact() && ((state.objects_open)() || (state.inspector_open)());
        let was_requested = requested_before.replace(requested);
        if requested != was_requested {
            if requested {
                *opener.borrow_mut() = focus_return_candidates();
            } else {
                let active = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.active_element());
                let inside = active.as_ref().is_none_or(|active| {
                    active.tag_name() == "BODY"
                        || active
                            .closest("[role=dialog]")
                            .ok()
                            .flatten()
                            .is_some_and(|dialog| dialog.id() == "workspace-panel-sheet")
                });
                if (!open || inside)
                    && let Some(target) = opener
                        .borrow()
                        .iter()
                        .find(|target| focus_target_available(target))
                {
                    let _ = target.focus();
                }
                opener.borrow_mut().clear();
            }
            sheet_open.set(requested);
        } else if requested && !open {
            set_bool(state.objects_open, false);
            set_bool(state.inspector_open, false);
        }
    });
    let active_tab = use_memo(move || {
        if (state.inspector_open)() {
            Some("inspector".to_owned())
        } else {
            Some("objects".to_owned())
        }
    });
    rsx! {
        {children}
        if compact() {
            if objects.is_some() || inspector.is_some() {
                Sheet { open: sheet_open, id: "workspace-panel-sheet",
                    SheetContent { direction: SheetDirection::Bottom, aria_label: "Workspace panels", aria_describedby: "workspace-panel-description", class: "dock-sheet",
                        p { id: "workspace-panel-description", class: "sr-only", "Objects and Inspector settings. The canvas remains interactive." }
                        div { class: "dock-sheet-heading",
                            Tabs { value: active_tab, horizontal: true,
                                on_value_change: move |value: String| {
                                    set_bool(state.objects_open, value == "objects");
                                    set_bool(state.inspector_open, value == "inspector");
                                },
                                TabsList { aria_label: "Workspace panels",
                                    TabsTrigger { id: "workspace-objects-tab", value: "objects", index: 0usize, disabled: objects.is_none(), "Objects" }
                                    TabsTrigger { id: "workspace-inspector-tab", value: "inspector", index: 1usize, disabled: inspector.is_none(), "Inspector" }
                                }
                                TabsContent { aria_labelledby: "workspace-objects-tab", value: "objects", index: 0usize, {objects} }
                                TabsContent { aria_labelledby: "workspace-inspector-tab", value: "inspector", index: 1usize, {inspector} }
                            }
                            SheetClose { "Close panels" }
                        }
                    }
                }
            }
        } else {
            {objects}
            {inspector}
        }
    }
}

pub fn use_panel_settings(side: PanelSide, storage_warning: Signal<bool>) -> Signal<PanelSettings> {
    let settings = use_signal(|| read_settings(side, storage_warning));
    use_effect(use_reactive((&settings(),), {
        move |(value,)| write_settings(side, value, storage_warning)
    }));
    settings
}

fn read_settings(side: PanelSide, mut storage_warning: Signal<bool>) -> PanelSettings {
    let Some(storage) = preference_storage(&mut storage_warning) else {
        return PanelSettings::default_pinned();
    };
    let stored = match storage.get_item(&format!("boardstudio:v2:panel:{}", side.storage_side())) {
        Ok(stored) => stored,
        Err(_) => {
            storage_warning.set(true);
            None
        }
    };
    let Some(value) =
        stored.and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok())
    else {
        return PanelSettings::default_pinned();
    };
    let mode = value.get("mode").and_then(serde_json::Value::as_str);
    let width = value.get("width").and_then(serde_json::Value::as_f64);
    decode_settings(side, mode, width)
}

fn write_settings(side: PanelSide, value: PanelSettings, mut storage_warning: Signal<bool>) {
    let Ok(json) = serde_json::to_string(&serde_json::json!({
        "mode": value.mode.as_str(),
        "width": value.width,
    })) else {
        return;
    };
    if let Some(storage) = preference_storage(&mut storage_warning)
        && storage
            .set_item(
                &format!("boardstudio:v2:panel:{}", side.storage_side()),
                &json,
            )
            .is_err()
    {
        storage_warning.set(true);
    }
}

fn preference_storage(storage_warning: &mut Signal<bool>) -> Option<Storage> {
    let Some(window) = web_sys::window() else {
        storage_warning.set(true);
        return None;
    };
    match window.local_storage() {
        Ok(Some(storage)) => Some(storage),
        Ok(None) | Err(_) => {
            storage_warning.set(true);
            None
        }
    }
}

fn read_preference(mut storage_warning: Signal<bool>, key: &str) -> Option<String> {
    preference_storage(&mut storage_warning).and_then(|storage| match storage.get_item(key) {
        Ok(value) => value,
        Err(_) => {
            storage_warning.set(true);
            None
        }
    })
}

fn write_preference(mut storage_warning: Signal<bool>, key: &str, value: &str) {
    if let Some(storage) = preference_storage(&mut storage_warning)
        && storage.set_item(key, value).is_err()
    {
        storage_warning.set(true);
    }
}

/// Appearance is saved under `boardstudio:v2:appearance`; until it is first saved, the legacy
/// `boardstudio:v2:theme` value (the same three words) still applies.
pub fn read_appearance_preference(storage_warning: Signal<bool>) -> &'static str {
    let stored = read_preference(storage_warning, "boardstudio:v2:appearance")
        .or_else(|| read_preference(storage_warning, "boardstudio:v2:theme"));
    match stored.as_deref() {
        Some("light") => "light",
        Some("dark") => "dark",
        _ => "system",
    }
}

pub fn write_appearance_preference(storage_warning: Signal<bool>, preference: &'static str) {
    write_preference(storage_warning, "boardstudio:v2:appearance", preference);
}

pub fn read_workbench_theme_preference(storage_warning: Signal<bool>) -> &'static str {
    match read_preference(storage_warning, "boardstudio:v2:workbench-theme").as_deref() {
        Some("shadcn") => "shadcn",
        _ => "original",
    }
}

pub fn write_workbench_theme_preference(storage_warning: Signal<bool>, preference: &'static str) {
    write_preference(
        storage_warning,
        "boardstudio:v2:workbench-theme",
        preference,
    );
}

#[component]
pub fn ObjectsPanel(
    compact_open: Signal<bool>,
    settings: Signal<PanelSettings>,
    header_action: Option<Element>,
    children: Element,
) -> Element {
    let options = crate::object_options::use_object_options();
    panel_frame(
        PanelSide::Objects,
        compact_open,
        settings,
        children,
        Some(options),
        header_action,
    )
}

#[component]
pub fn InspectorPanel(
    compact_open: Signal<bool>,
    settings: Signal<PanelSettings>,
    children: Element,
) -> Element {
    let body = rsx! { div { class: "m1-inspector-body", {children} } };
    panel_frame(
        PanelSide::Inspector,
        compact_open,
        settings,
        body,
        None,
        None,
    )
}

fn panel_frame(
    side: PanelSide,
    _compact_open: Signal<bool>,
    settings: Signal<PanelSettings>,
    children: Element,
    options: Option<Element>,
    header_action: Option<Element>,
) -> Element {
    let viewport = use_viewport_width();
    let compact = viewport() < 980.0;
    let available = use_memo(move || {
        (viewport() - 312.0)
            * if side == PanelSide::Objects {
                0.45
            } else {
                0.55
            }
    });
    let label = if side == PanelSide::Objects {
        "Objects"
    } else {
        "Inspector"
    };
    let action = if side == PanelSide::Objects {
        "objects"
    } else {
        "inspector"
    };
    let mut focus_rail = use_signal(|| false);
    use_effect(move || {
        if focus_rail() {
            focus_rail.set(false);
            focus_element(if settings().mode == PanelMode::Collapsed {
                rail_id(side)
            } else {
                collapse_id(side)
            });
        }
    });
    let content = rsx! {
        div { class: "dock-pane", "data-side": side.storage_side(),
            header { class: "dock-heading",
                h2 { "{label}" }
                {header_action}
                if let Some(options) = options {
                    Popover { is_modal: false,
                        PopoverTrigger { aria_label: "Objects options", "Options" }
                        PopoverContent { aria_label: "Objects panel options", {options} }
                    }
                }
                if !compact {
                    Button { id: collapse_id(side), variant: ButtonVariant::Ghost, aria_label: "Collapse {action}",
                        onclick: move |_| { set_panel_mode(settings, PanelMode::Collapsed); focus_rail.set(true); },
                        "Collapse"
                    }
                }
            }
            div { class: "dock-body", id: match side { PanelSide::Objects => "m1-objects-panel-content", PanelSide::Inspector => "m1-inspector-panel-content" }, {children} }
        }
    };
    let collapsed = !compact && settings().mode == PanelMode::Collapsed;
    rsx! {
        div { class: "dock-slot", id: match side { PanelSide::Objects => "m1-objects-panel", PanelSide::Inspector => "m1-inspector-panel" },
            "data-side": side.storage_side(), "data-mode": settings().mode.as_str(), aria_hidden: "false",
            if collapsed {
                div { class: "dock-rail",
                    Button { id: rail_id(side), variant: ButtonVariant::Ghost, aria_label: "Show {action}",
                        onclick: move |_| { set_panel_mode(settings, PanelMode::Pinned); focus_rail.set(true); }, "{label}"
                    }
                }
            }
            div { class: "dock-mount", hidden: collapsed, inert: collapsed.then_some(""),
                Sidenav { side, settings, aria_label: label, available_width: available, class: "workbench-dock", {content} }
            }
        }
    }
}

fn set_panel_mode(mut settings: Signal<PanelSettings>, mode: PanelMode) {
    let mut next = settings();
    next.mode = mode;
    settings.set(next);
}

fn collapse_id(side: PanelSide) -> &'static str {
    match side {
        PanelSide::Objects => "objects-dock-collapse",
        PanelSide::Inspector => "inspector-dock-collapse",
    }
}

fn rail_id(side: PanelSide) -> &'static str {
    match side {
        PanelSide::Objects => "objects-dock-rail",
        PanelSide::Inspector => "inspector-dock-rail",
    }
}

fn toggle_id(side: PanelSide) -> &'static str {
    match side {
        PanelSide::Objects => "objects-sheet-toggle",
        PanelSide::Inspector => "inspector-sheet-toggle",
    }
}

pub fn focus_panel_toggle(side: PanelSide) {
    focus_element(toggle_id(side));
}

fn focus_element(id: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

type ResizeListener = Rc<(web_sys::Window, Closure<dyn FnMut(web_sys::Event)>)>;

fn use_viewport_width() -> Signal<f64> {
    let read_width = || {
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(1280.0)
    };
    let mut width = use_signal(read_width);
    let listener = use_hook(|| {
        web_sys::window().map(|window| {
            let callback = Closure::wrap(
                Box::new(move |_: web_sys::Event| width.set(read_width())) as Box<dyn FnMut(_)>,
            );
            let _ = window
                .add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref());
            Rc::new((window, callback)) as ResizeListener
        })
    });
    use_drop(move || {
        if let Some(listener) = listener {
            let _ = listener
                .0
                .remove_event_listener_with_callback("resize", listener.1.as_ref().unchecked_ref());
        }
    });
    width
}

fn use_compact_viewport() -> Memo<bool> {
    let width = use_viewport_width();
    use_memo(move || width() < 980.0)
}

fn set_bool(mut signal: Signal<bool>, value: bool) {
    if *signal.peek() != value {
        signal.set(value);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "panels_scroll_tests.rs"]
mod scroll_tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "panels_resize_tests.rs"]
mod resize_tests;
