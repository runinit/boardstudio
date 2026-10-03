use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_timers::callback::Timeout;
use policy::decode_settings;
pub(super) use policy::{PanelMode, PanelSettings, PanelSide};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Document, HtmlElement, MediaQueryList, Node, PointerEvent, Storage};

mod policy;

/// Apply ordinary workspace panel defaults without overriding an explicit reveal.
pub(super) fn use_workspace_panel_defaults(
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

#[component]
pub(super) fn CompactPanelControls() -> Element {
    let state = use_context::<super::CompactPanelState>();
    let workspace = use_context::<super::WorkspaceState>().0;
    let objects_compact = use_compact_viewport(PanelSide::Objects);
    let inspector_compact = use_compact_viewport(PanelSide::Inspector);
    let objects_open = state.objects_open;
    let inspector_open = state.inspector_open;
    rsx! {
        nav { class: "m1-compact-panel-controls", "aria-label": "Panel visibility",
            if objects_compact() && workspace() != "Export" {
                button {
                    class: "m1-panel-toggle",
                    aria_label: "Objects",
                    title: "Objects",
                    aria_controls: "m1-objects-panel",
                    aria_expanded: if objects_open() { "true" } else { "false" },
                    onclick: move |_| {
                        if objects_open() {
                            set_bool(objects_open, false);
                        } else {
                            set_bool(objects_open, true);
                            set_bool(inspector_open, false);
                        }
                    },
                    svg { "aria-hidden": "true", view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linejoin: "round",
                        rect { x: "2.5", y: "3", width: "15", height: "14", rx: "1.5" }
                        path { d: "M7.5 3v14" }
                    }
                }
            }
            if inspector_compact() && workspace() != "Export" {
                button {
                    class: "m1-panel-toggle",
                    aria_label: "Inspect",
                    title: "Inspect",
                    aria_controls: "m1-inspector-panel",
                    aria_expanded: if inspector_open() { "true" } else { "false" },
                    onclick: move |_| {
                        if inspector_open() {
                            set_bool(inspector_open, false);
                        } else {
                            set_bool(inspector_open, true);
                            set_bool(objects_open, false);
                        }
                    },
                    svg { "aria-hidden": "true", view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linejoin: "round",
                        rect { x: "2.5", y: "3", width: "15", height: "14", rx: "1.5" }
                        path { d: "M12.5 3v14" }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn CompactPanelScrim() -> Element {
    let state = use_context::<super::CompactPanelState>();
    let workspace = use_context::<super::WorkspaceState>().0;
    let objects_compact = use_compact_viewport(PanelSide::Objects);
    let inspector_compact = use_compact_viewport(PanelSide::Inspector);
    let objects_visible = objects_compact() && (state.objects_open)();
    let inspector_visible = inspector_compact() && (state.inspector_open)();
    rsx! {
        if workspace() != "Export" && (objects_visible || inspector_visible) {
            button {
                class: "m1-drawer-scrim",
                aria_label: "Close panels",
                onclick: move |_| {
                    focus_panel_toggle(if objects_visible { PanelSide::Objects } else { PanelSide::Inspector });
                    set_bool(state.objects_open, false);
                    set_bool(state.inspector_open, false);
                },
            }
        }
    }
}

type OutsideListener = Rc<RefCell<Option<(Document, Closure<dyn FnMut(PointerEvent)>)>>>;
type MediaChangeListener =
    Rc<RefCell<Option<(MediaQueryList, Closure<dyn FnMut(web_sys::Event)>)>>>;

pub(super) fn use_panel_settings(
    side: PanelSide,
    storage_warning: Signal<bool>,
) -> Signal<PanelSettings> {
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
    if let Some(storage) = preference_storage(&mut storage_warning) {
        if storage
            .set_item(
                &format!("boardstudio:v2:panel:{}", side.storage_side()),
                &json,
            )
            .is_err()
        {
            storage_warning.set(true);
        }
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

pub(super) fn read_theme_preference(mut storage_warning: Signal<bool>) -> &'static str {
    let stored = preference_storage(&mut storage_warning).and_then(|storage| {
        match storage.get_item("boardstudio:v2:theme") {
            Ok(value) => value,
            Err(_) => {
                storage_warning.set(true);
                None
            }
        }
    });
    match stored.as_deref() {
        Some("light") => "light",
        Some("dark") => "dark",
        _ => "system",
    }
}

pub(super) fn write_theme_preference(mut storage_warning: Signal<bool>, preference: &'static str) {
    if let Some(storage) = preference_storage(&mut storage_warning)
        && storage
            .set_item("boardstudio:v2:theme", preference)
            .is_err()
    {
        storage_warning.set(true);
    }
}

#[component]
pub(super) fn ObjectsPanel(
    compact_open: Signal<bool>,
    settings: Signal<PanelSettings>,
    children: Element,
) -> Element {
    let options = super::objects::use_object_options();
    panel_frame(
        PanelSide::Objects,
        compact_open,
        settings,
        children,
        Some(options),
    )
}

#[component]
pub(super) fn InspectorPanel(
    compact_open: Signal<bool>,
    settings: Signal<PanelSettings>,
    children: Element,
) -> Element {
    let body = rsx! { div { class: "m1-inspector-body", {children} } };
    panel_frame(PanelSide::Inspector, compact_open, settings, body, None)
}

fn panel_frame(
    side: PanelSide,
    compact_open: Signal<bool>,
    settings: Signal<PanelSettings>,
    children: Element,
    options: Option<Element>,
) -> Element {
    let compact = use_compact_viewport(side);
    let menu_open = use_signal(|| false);
    let revealed = use_signal(|| false);
    let hovered = use_signal(|| false);
    let focus_rail_after_render = use_signal(|| false);
    let hide_timer = use_hook(|| Rc::new(RefCell::new(None::<Timeout>)));
    let outside_listener = use_hook(OutsideListener::default);

    let ids = PanelIds::for_side(side);
    let current = settings();
    let primary_action = if current.mode == PanelMode::Autohide {
        ids.pin_action
    } else {
        ids.autohide_action
    };
    let compact_closed = compact() && !compact_open();
    let content_hidden = if compact() {
        !compact_open()
    } else {
        current.mode != PanelMode::Pinned && !revealed()
    };
    let timer_focusin = hide_timer.clone();
    let timer_focusout = hide_timer.clone();
    let timer_rail_enter = hide_timer.clone();
    let timer_rail_leave = hide_timer.clone();
    let timer_rail_click = hide_timer.clone();
    let timer_content_enter = hide_timer.clone();
    let timer_content_leave = hide_timer.clone();

    use_effect(use_reactive((&compact(),), {
        let hide_timer = hide_timer.clone();
        move |(is_compact,)| {
            if is_compact {
                set_bool(menu_open, false);
                cancel_hide_timer(&hide_timer);
                if case_workspace_has_focus() {
                    set_bool(compact_open, false);
                }
            }
        }
    }));

    use_effect(use_reactive((&menu_open(),), {
        let outside_listener = outside_listener.clone();
        move |(open,)| {
            remove_outside_listener(&outside_listener);
            if !open {
                return;
            }
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let menu_open = menu_open;
            let trigger_id = ids.options_trigger.to_owned();
            let options_id = ids.options_group.to_owned();
            let listener_document = document.clone();
            let listener = Closure::wrap(Box::new(move |event: PointerEvent| {
                let target = event
                    .target()
                    .and_then(|target| target.dyn_into::<Node>().ok());
                let inside = target.as_ref().is_some_and(|target| {
                    element_contains(&listener_document, &options_id, target)
                        || element_contains(&listener_document, &trigger_id, target)
                });
                if !inside {
                    set_bool(menu_open, false);
                }
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("pointerdown", listener.as_ref().unchecked_ref());
            *outside_listener.borrow_mut() = Some((document, listener));
        }
    }));
    use_drop({
        let outside_listener = outside_listener.clone();
        let hide_timer = hide_timer.clone();
        move || {
            hide_timer.borrow_mut().take();
            remove_outside_listener(&outside_listener);
        }
    });

    use_effect(use_reactive((&focus_rail_after_render(),), {
        move |(should_focus,)| {
            if should_focus {
                set_bool(focus_rail_after_render, false);
                focus_element(ids.rail);
            }
        }
    }));

    rsx! {
        div {
            id: ids.shell,
            class: match (side, compact_open()) {
                (PanelSide::Objects, true) => "m1-object-slot m1-panel-slot compact-open",
                (PanelSide::Objects, false) => "m1-object-slot m1-panel-slot compact-closed",
                (PanelSide::Inspector, true) => "m1-inspector-slot m1-panel-slot compact-open",
                (PanelSide::Inspector, false) => "m1-inspector-slot m1-panel-slot compact-closed",
            },
            "data-side": side.as_str(),
            "data-mode": current.mode.as_str(),
            "data-revealed": if revealed() { "true" } else { "false" },
            inert: compact_closed.then_some(""),
            aria_hidden: if compact_closed { "true" } else { "false" },
            onfocusin: move |_| cancel_hide_timer(&timer_focusin),
            onfocusout: move |_| schedule_hide_timer(side, settings, revealed, hovered, compact, timer_focusout.clone()),
            if !compact() && current.mode != PanelMode::Pinned {
                button {
                    id: ids.rail,
                    class: "m1-panel-rail",
                    aria_label: ids.show_label,
                    aria_expanded: if revealed() { "true" } else { "false" },
                    aria_controls: ids.content,
                    title: ids.show_label,
                    onpointerenter: move |event: dioxus::prelude::PointerEvent| {
                        if event.data().pointer_type() != "touch" {
                            set_bool(hovered, true);
                            cancel_hide_timer(&timer_rail_enter);
                            set_bool(revealed, true);
                        }
                    },
                    onpointerleave: move |_| {
                        set_bool(hovered, false);
                        schedule_hide_timer(side, settings, revealed, hovered, compact, timer_rail_leave.clone());
                    },
                    onclick: move |_| {
                        cancel_hide_timer(&timer_rail_click);
                        if current.mode == PanelMode::Collapsed {
                            apply_panel_mode(settings, menu_open, revealed, focus_rail_after_render, PanelMode::Pinned);
                        } else {
                            set_bool(revealed, true);
                        }
                    },
                    "{ids.rail_text}"
                }
            }
            div {
                id: ids.content,
                class: "m1-panel-content",
                inert: content_hidden.then_some(""),
                aria_hidden: if content_hidden { "true" } else { "false" },
                "data-mode": current.mode.as_str(),
                "data-revealed": if revealed() { "true" } else { "false" },
                onpointerenter: move |_| {
                    set_bool(hovered, true);
                    cancel_hide_timer(&timer_content_enter);
                },
                onpointerleave: move |_| {
                    set_bool(hovered, false);
                    schedule_hide_timer(side, settings, revealed, hovered, compact, timer_content_leave.clone());
                },
                onkeydown: move |event: KeyboardEvent| {
                    if event.data().key().to_string() != "Escape" { return; }
                    let Some(raw) = event.data().try_as_web_event() else { return; };
                    if raw.default_prevented() { return; }
                    if menu_open() {
                        event.prevent_default();
                        event.stop_propagation();
                        set_bool(menu_open, false);
                        focus_element(ids.options_trigger);
                        return;
                    }
                    if compact() {
                        event.prevent_default();
                        event.stop_propagation();
                        set_bool(compact_open, false);
                        focus_panel_toggle(side);
                        return;
                    }
                    if current.mode == PanelMode::Pinned { return; }
                    event.prevent_default();
                    set_bool(revealed, false);
                    set_bool(focus_rail_after_render, true);
                },
                    header { class: "m1-panel-heading",
                        h2 { "{ids.title}" }
                        if compact() {
                            button {
                                class: "m1-panel-close",
                                aria_label: match side { PanelSide::Objects => "Close objects", PanelSide::Inspector => "Close inspector" },
                                onclick: move |_| {
                                    set_bool(menu_open, false);
                                    set_bool(compact_open, false);
                                    focus_panel_toggle(side);
                                },
                                svg { "aria-hidden": "true", view_box: "0 0 16 16", path { d: "m4 4 8 8M12 4l-8 8" } }
                            }
                        }
                        button {
                            id: ids.options_trigger,
                            class: "m1-panel-options-trigger",
                            aria_label: ids.options_label,
                            aria_expanded: if menu_open() { "true" } else { "false" },
                            aria_controls: ids.options_group,
                            title: ids.options_label,
                            onclick: move |_| set_bool(menu_open, !menu_open()),
                            svg { "aria-hidden": "true", view_box: "0 0 16 16",
                                circle { cx: "8", cy: "3", r: "1.2" }
                                circle { cx: "8", cy: "8", r: "1.2" }
                                circle { cx: "8", cy: "13", r: "1.2" }
                            }
                        }
                    }
                    if menu_open() {
                        div {
                            id: ids.options_group,
                            class: "m1-panel-options",
                            role: "group",
                            aria_label: ids.options_group_label,
                            onkeydown: move |event: KeyboardEvent| {
                                if event.data().key().to_string() == "Escape" {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    set_bool(menu_open, false);
                                    focus_element(ids.options_trigger);
                                }
                            },
                            {options}
                            if !compact() {
                                button { onclick: move |_| apply_panel_mode(settings, menu_open, revealed, focus_rail_after_render, if current.mode == PanelMode::Autohide { PanelMode::Pinned } else { PanelMode::Autohide }), "{primary_action}" }
                                button { onclick: move |_| apply_panel_mode(settings, menu_open, revealed, focus_rail_after_render, PanelMode::Collapsed), "{ids.collapse_action}" }
                            }
                        }
                    }
                {children}
            }
        }
    }
}

#[derive(Clone, Copy)]
struct PanelIds {
    shell: &'static str,
    content: &'static str,
    rail: &'static str,
    options_trigger: &'static str,
    options_group: &'static str,
    title: &'static str,
    rail_text: &'static str,
    show_label: &'static str,
    options_label: &'static str,
    options_group_label: &'static str,
    autohide_action: &'static str,
    pin_action: &'static str,
    collapse_action: &'static str,
}

impl PanelIds {
    const fn for_side(side: PanelSide) -> Self {
        match side {
            PanelSide::Objects => Self {
                shell: "m1-objects-panel",
                content: "m1-objects-panel-content",
                rail: "m1-objects-panel-rail",
                options_trigger: "m1-objects-panel-options",
                options_group: "m1-objects-panel-options-group",
                title: "Objects",
                rail_text: "Objects",
                show_label: "Show objects",
                options_label: "Objects options",
                options_group_label: "Objects panel options",
                autohide_action: "Auto-hide objects",
                pin_action: "Pin objects",
                collapse_action: "Collapse objects",
            },
            PanelSide::Inspector => Self {
                shell: "m1-inspector-panel",
                content: "m1-inspector-panel-content",
                rail: "m1-inspector-panel-rail",
                options_trigger: "m1-inspector-panel-options",
                options_group: "m1-inspector-panel-options-group",
                title: "Inspect",
                rail_text: "Inspect",
                show_label: "Show inspector",
                options_label: "Inspector options",
                options_group_label: "Inspector panel options",
                autohide_action: "Auto-hide inspector",
                pin_action: "Pin inspector",
                collapse_action: "Collapse inspector",
            },
        }
    }
}

impl PanelSide {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Objects => "objects",
            Self::Inspector => "inspector",
        }
    }
}

pub(super) fn focus_panel_toggle(side: PanelSide) {
    let selector = format!(
        ".m1-compact-panel-controls button[aria-controls='{}']",
        PanelIds::for_side(side).shell
    );
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(&selector).ok().flatten())
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

fn use_compact_viewport(side: PanelSide) -> Signal<bool> {
    let compact = use_signal(|| media_query(side).is_some_and(|query| query.matches()));
    let listener = use_hook(MediaChangeListener::default);
    use_effect({
        let listener = listener.clone();
        move || {
            if let Some(query) = media_query(side) {
                let observed_query = query.clone();
                let callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                    set_bool(compact, observed_query.matches());
                }) as Box<dyn FnMut(_)>);
                let _ = query
                    .add_event_listener_with_callback("change", callback.as_ref().unchecked_ref());
                *listener.borrow_mut() = Some((query, callback));
            }
        }
    });
    use_drop({
        let listener = listener.clone();
        move || {
            if let Some((query, callback)) = listener.borrow_mut().take() {
                let _ = query.remove_event_listener_with_callback(
                    "change",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    });
    compact
}

fn media_query(side: PanelSide) -> Option<MediaQueryList> {
    let width = match side {
        PanelSide::Objects => 980,
        PanelSide::Inspector => 820,
    };
    web_sys::window().and_then(|window| {
        window
            .match_media(&format!("(max-width: {width}px)"))
            .ok()
            .flatten()
    })
}

fn case_workspace_has_focus() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
        .and_then(|active| active.closest(".m1-workspace-content").ok().flatten())
        .and_then(|workspace| {
            workspace
                .query_selector(":scope > .m1-case-panel")
                .ok()
                .flatten()
        })
        .is_some()
}

fn set_bool(mut signal: Signal<bool>, value: bool) {
    signal.set(value);
}

fn apply_panel_mode(
    mut settings: Signal<PanelSettings>,
    menu_open: Signal<bool>,
    revealed: Signal<bool>,
    focus_rail_after_render: Signal<bool>,
    mode: PanelMode,
) {
    let mut next = settings();
    next.mode = mode;
    settings.set(next);
    set_bool(menu_open, false);
    match mode {
        PanelMode::Pinned => set_bool(revealed, false),
        PanelMode::Autohide => set_bool(revealed, true),
        PanelMode::Collapsed => {
            set_bool(revealed, false);
            set_bool(focus_rail_after_render, true);
        }
    }
}

fn cancel_hide_timer(timer: &Rc<RefCell<Option<Timeout>>>) {
    timer.borrow_mut().take();
}

fn schedule_hide_timer(
    side: PanelSide,
    settings: Signal<PanelSettings>,
    revealed: Signal<bool>,
    hovered: Signal<bool>,
    compact: Signal<bool>,
    timer: Rc<RefCell<Option<Timeout>>>,
) {
    cancel_hide_timer(&timer);
    if compact() || settings().mode == PanelMode::Pinned {
        return;
    }
    let timeout = Timeout::new(280, move || {
        if !compact()
            && settings().mode != PanelMode::Pinned
            && !hovered()
            && !focus_is_inside(PanelIds::for_side(side))
        {
            set_bool(revealed, false);
        }
    });
    *timer.borrow_mut() = Some(timeout);
}

fn focus_is_inside(ids: PanelIds) -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    let Some(active) = document.active_element() else {
        return false;
    };
    let active: &Node = active.as_ref();
    element_contains(&document, ids.content, active)
        || element_contains(&document, ids.rail, active)
}

fn element_contains(document: &Document, id: &str, node: &Node) -> bool {
    document
        .get_element_by_id(id)
        .is_some_and(|element| element.contains(Some(node)))
}

fn focus_element(id: &str) {
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(id))
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

fn remove_outside_listener(listener: &OutsideListener) {
    if let Some((document, callback)) = listener.borrow_mut().take() {
        let _ = document
            .remove_event_listener_with_callback("pointerdown", callback.as_ref().unchecked_ref());
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "panels_scroll_tests.rs"]
mod scroll_tests;
