//! Private Layout selection and snap controls plus their shared value adapters.
pub(super) use super::layout_toolbar_logic::SNAP_STEPS;
pub use super::layout_toolbar_logic::{
    GestureSnapInputs, LayoutCanvasGridStyle, LayoutSnapIntent, context_for_selection_kind,
    gesture_snap_inputs, layout_canvas_grid_style, snap_label,
};
use boardstudio_web_ui_model::Workspace;
pub use boardstudio_web_ui_model::state::{LayoutSnapSettings, TreeCellAnchor};
use boardstudio_web_ui_shared::inspector_fields::{SelectField, ToggleField};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    callout::Callout,
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    command::{Command, use_commands},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    popover::{Popover, PopoverContent, PopoverTrigger},
    toggle_group::{ToggleGroup, ToggleItem},
    toolbar::Toolbar,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

pub use super::{layout_selection_kind::LayoutSelectionKind, workspace_state::LayoutCommandMenu};

impl LayoutCommandMenu {
    const fn meta(self) -> (&'static str, &'static str) {
        match self {
            Self::Select => ("select", "m1-layout-select-trigger"),
            Self::Transform => ("transform", "m1-layout-transform-trigger"),
            Self::Align => ("align", "m1-layout-align-trigger"),
            Self::Snap => ("snap", "m1-layout-snap-trigger"),
        }
    }
}

pub fn close_layout_command_menu(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
) {
    let menu = open_menu().unwrap_or(menu);
    open_menu.set(None);
    restore_layout_command_focus(menu);
}

fn popup_header(
    label: String,
    close_label: String,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
) -> Element {
    let aria = close_label.clone();
    rsx! {
        div { class: "layout-popup-heading",
            strong { "{label}" }
            Tooltip {
                TooltipTrigger { r#as: move |attributes: Vec<Attribute>| rsx! {
                    Button { attributes, variant: ButtonVariant::Ghost, class: "layout-popup-close", aria_label: aria.clone(), onclick: move |_| close_layout_command_menu(open_menu, menu), span { aria_hidden: true, "×" } }
                } }
                TooltipContent { "{close_label}" }
            }
        }
    }
}

fn doc() -> Option<web_sys::Document> {
    web_sys::window().and_then(|w| w.document())
}

fn restore_layout_command_focus(menu: LayoutCommandMenu) {
    let Some(doc) = doc() else { return };
    let target = (menu == LayoutCommandMenu::Select)
        .then(|| {
            doc.query_selector(".layout-scope-trigger[aria-pressed='true']")
                .ok()
                .flatten()
        })
        .flatten()
        .or_else(|| doc.get_element_by_id(menu.meta().1));
    if let Some(el) = target.and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        let _ = el.focus();
    }
}

fn focus_layout_popup(id: &str) {
    let Some(el) = doc().and_then(|d| d.get_element_by_id(id)) else {
        return;
    };
    if el.get_attribute("data-state").as_deref() != Some("open") || el.has_attribute("inert") {
        return;
    }
    let sel = "input:not(:disabled), [data-slot=select-trigger]:not(:disabled), [data-slot=toolbar-button]:not(:disabled)";
    let ctrl = el
        .query_selector(sel)
        .ok()
        .flatten()
        .or_else(|| el.query_selector("button:not(:disabled)").ok().flatten());
    if let Some(el) = ctrl.as_ref().and_then(|c| c.dyn_ref::<HtmlElement>()) {
        let _ = el.focus();
    }
}

fn dismiss_layout_popup(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
    id: &str,
) {
    let restore = doc().is_some_and(|d| {
        let active = d.active_element();
        active.is_none()
            || active == d.body().map(Into::into)
            || d.get_element_by_id(id)
                .is_some_and(|c| c.contains(active.as_ref().map(|e| e.as_ref())))
    });
    if restore {
        close_layout_command_menu(open_menu, menu);
    } else {
        open_menu.set(None);
    }
}

fn use_layout_palette_commands(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    on_kind: EventHandler<LayoutSelectionKind>,
    on_snap: EventHandler<LayoutSnapIntent>,
    snap: ReadSignal<LayoutSnapSettings>,
    t: &super::layout_rail_menu::LayoutTransformMenuMount,
) {
    use LayoutCommandMenu as M;
    use LayoutSelectionKind as K;
    let mut raw: Vec<(String, Option<String>, EventHandler<()>)> = K::ALL
        .into_iter()
        .map(|k| {
            let cb = use_callback(move |()| {
                on_kind.call(k);
                open_menu.set(Some(M::Select));
            });
            (format!("Select {}", k.label()), None, cb)
        })
        .collect();
    for (label, target) in [
        ("Snap settings", M::Snap),
        ("Align", M::Align),
        ("Transform", M::Transform),
    ] {
        let cb = use_callback(move |()| {
            open_menu.set(Some(target));
            if target == M::Snap {
                restore_layout_command_focus(target);
            }
        });
        raw.push((label.into(), None, cb));
    }
    let c_geo =
        use_callback(move |()| on_snap.call(LayoutSnapIntent::GeometrySnap(!snap().geometry_snap)));
    let c_co = use_callback(move |()| on_kind.call(K::Column));
    let c_ro = use_callback(move |()| on_kind.call(K::Row));
    let ws = try_use_context::<boardstudio_web_ui_model::state::WorkspaceState>();
    let commands = if ws.is_some_and(|w| w.0() == Workspace::Layout) {
        let m =
            "Select a matrix, row, column, key or part to open its current transform properties.";
        let r = |ok: bool| (!ok).then(|| m.to_owned());
        raw.push(("Geometry snap".into(), None, c_geo));
        raw.push((
            "Position & rotation".into(),
            r(t.properties_available),
            t.on_show_properties,
        ));
        raw.push((
            "Stagger or offset column".into(),
            r(t.column_available),
            c_co,
        ));
        raw.push(("Offset row".into(), r(t.row_available), c_ro));
        raw.into_iter()
            .map(|(l, r, on)| Command {
                group: "Current Workspace",
                label: l,
                reason: r,
                hint: None,
                on_select: on,
            })
            .collect()
    } else {
        Vec::new()
    };
    use_commands(commands);
}

#[component]
pub fn LayoutCommandPill(
    command_label: String,
    menu_owner_key: String,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
    snap_settings: ReadSignal<LayoutSnapSettings>,
    transform: super::layout_rail_menu::LayoutTransformMenuMount,
    align: super::layout_align::LayoutAlignMount,
    show_relationships: bool,
    on_selection_kind: EventHandler<LayoutSelectionKind>,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    has_selection_context: bool,
    on_show_relationships: EventHandler<()>,
    #[props(default)] snap_in_dock: bool,
) -> Element {
    let settings = snap_settings();
    let geometry_snap = settings.geometry_snap;
    let t = &transform;
    use_layout_palette_commands(
        open_menu,
        on_selection_kind,
        on_snap_intent,
        snap_settings,
        t,
    );
    use_effect(use_reactive((&menu_owner_key,), move |_| {
        open_menu.set(None)
    }));
    use_effect(use_reactive((&selection_kind,), move |(k,)| {
        if open_menu() == Some(LayoutCommandMenu::Select) {
            focus_layout_popup(&format!("m1-layout-scope-panel-{}", k.label()));
        }
    }));
    let has_actions = t.properties_available || t.column_available || t.row_available;
    let idx = LayoutSelectionKind::ALL
        .iter()
        .position(|k| *k == selection_kind)
        .unwrap_or(0);

    rsx! {
        div { id: "m1-layout-command-pill", class: "layout-toolbar-strips", role: "group", aria_label: "{command_label}",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " "
                    && let Some(raw) = event.data().try_as_web_event()
                    && raw.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).is_some_and(|t| {
                        t.id() == "m1-layout-snap-trigger" || (t.tag_name() == "BUTTON"
                            && t.closest("#m1-layout-command-pill").ok().flatten().is_some()
                            && t.matches(".layout-scope-trigger, .layout-action-control, .layout-geometry-control, .layout-popup-close, .layout-menu-actions [data-slot=popover-trigger], .layout-menu-actions [data-slot=dropdown-menu-trigger], .layout-scope-popup [data-slot=button], .layout-settings-popup [data-slot=button], .layout-settings-popup [data-slot=checkbox][aria-label='Geometry snap'], .layout-settings-popup [data-slot=checkbox][aria-label='Envelope gap']").unwrap_or(false))
                    })
                {
                    event.stop_propagation();
                    raw.stop_propagation();
                }
            },
            div { class: "layout-scope-strip",
                Toolbar { class: "layout-scope-toolbar", aria_label: "Selection scope",
                    ToggleGroup { aria_label: "Selection scope choices", horizontal: true, pressed: Some([idx].into()),
                        for (index, kind) in LayoutSelectionKind::ALL.into_iter().enumerate() {
                            Popover {
                                open: open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind, is_modal: false,
                                on_open_change: move |next| {
                                    if next { on_selection_kind.call(kind); open_menu.set(Some(LayoutCommandMenu::Select)); }
                                    else if open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind { dismiss_layout_popup(open_menu, LayoutCommandMenu::Select, &format!("m1-layout-scope-panel-{}", kind.label())); }
                                },
                                ToggleItem {
                                    index, class: "layout-scope-trigger", id: format!("m1-layout-scope-{}", kind.label()),
                                    aria_label: format!("Select {}", kind.label()), aria_haspopup: "dialog",
                                    aria_expanded: open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind,
                                    aria_controls: (open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind).then(|| format!("m1-layout-scope-panel-{}", kind.label())),
                                    attributes: vec![onclick(move |_| { let was = open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind; on_selection_kind.call(kind); open_menu.set((!was).then_some(LayoutCommandMenu::Select)); })],
                                    {selection_icon(kind)}
                                    span { "{kind.label()}" }
                                }
                                PopoverContent {
                                    id: format!("m1-layout-scope-panel-{}", kind.label()), class: "layout-scope-popup", role: "dialog",
                                    aria_label: format!("{} actions", kind.label()), aria_labelledby: Option::<String>::None,
                                    inert: (!(open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind)).then_some(""),
                                    attributes: vec![onmounted(move |_| if open_menu() == Some(LayoutCommandMenu::Select) && kind == selection_kind { focus_layout_popup(&format!("m1-layout-scope-panel-{}", kind.label())); })],
                                    {popup_header(format!("{} actions", kind.label()), format!("Close {} actions", kind.label()), open_menu, LayoutCommandMenu::Select)}
                                    if has_actions {
                                        super::layout_rail_menu::LayoutContextActionToolbar { mount: transform.clone(), open_menu, selection_kind: kind }
                                    } else {
                                        Callout { "Select a {kind.label().to_lowercase()} on the canvas to edit it." }
                                        Button { variant: ButtonVariant::Secondary, onclick: { let t = transform.clone(); move |_| { t.on_show_properties.call(()); close_layout_command_menu(open_menu, LayoutCommandMenu::Select); } }, "Open Inspector" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "layout-context-strip",
                div { class: "layout-action-slot",
                    if has_selection_context {
                        super::layout_rail_menu::LayoutContextActionToolbar { mount: transform.clone(), open_menu, selection_kind }
                    }
                }
                div { class: "layout-menu-actions", role: "group", aria_label: "Selection actions",
                    super::layout_align::LayoutAlignToolbar {
                        mount: align, open_menu, show_relationships, has_selection_context, on_show_relationships,
                    }
                    super::layout_rail_menu::LayoutTransformToolbar { open_menu, mount: transform, selection_kind }
                    if !snap_in_dock {
                        LayoutCommandPopup {
                            open_menu, menu: LayoutCommandMenu::Snap, label: "Snap settings",
                            trigger: rsx! { span { "Snap" } small { "{snap_label(settings.snap_fraction)}" } },
                            {snap_options(settings, on_snap_intent)}
                        }
                        Tooltip {
                            TooltipTrigger { r#as: move |attributes: Vec<Attribute>| rsx! {
                                Button { attributes, variant: ButtonVariant::Ghost, class: "layout-geometry-control",
                                    aria_label: "Geometry snap", aria_pressed: geometry_snap,
                                    onclick: move |_| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(!geometry_snap)),
                                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
                                }
                            } }
                            TooltipContent { "Geometry snap" }
                        }
                    }
                }
            }
        }
    }
}

fn selection_icon(kind: LayoutSelectionKind) -> Element {
    use LayoutSelectionKind as K;
    let p_path = "M7 2v4M13 2v4M7 14v4M13 14v4M2 7h4M2 13h4M14 7h4M14 13h4";
    let (x, y, w, h, rx, d) = match kind {
        K::Matrix => ("3", "3", "14", "14", "1", "M3 8h14M3 12h14M8 3v14M12 3v14"),
        K::Row => ("2", "7", "16", "6", "1", "M7 7v6M13 7v6"),
        K::Column => ("7", "2", "6", "16", "1", "M7 7h6M7 13h6"),
        K::Key => ("3", "3", "14", "14", "3", "M6 13h8"),
        K::Part => ("6", "6", "8", "8", "1", p_path),
    };
    rsx! {
        svg { class: "block size-[22px]! shrink-0 fill-none stroke-current stroke-[1.5] [stroke-linecap:round] [stroke-linejoin:round]", view_box: "0 0 20 20", "aria-hidden": "true",
            rect { x, y, width: w, height: h, rx } path { d }
        }
    }
}

#[component]
pub fn LayoutSnapDock(
    settings: LayoutSnapSettings,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
    #[props(default = true)] handles_visible: bool,
    #[props(default)] on_toggle_handles: EventHandler<bool>,
) -> Element {
    let mut snap_open = use_signal(|| false);
    rsx! {
        Collapsible { class: "relative left-0 w-full min-w-0 flex-none border-t border-border bg-card p-0",
            open: Some(snap_open()), on_open_change: move |open| snap_open.set(open),
            CollapsibleTrigger { id: "m1-layout-snap-trigger", aria_label: "Snap",
                class: "flex w-full min-h-control-touch items-center justify-start gap-2 rounded-none border-0 bg-transparent px-3 py-2.5 text-left text-foreground hover:bg-accent",
                attributes: vec![dioxus::events::onkeydown(super::stop_space_pan)],
                svg { class: "size-[18px] fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M5 3v8a5 5 0 0 0 10 0V3h-4v7a1 1 0 0 1-2 0V3ZM5 6h4M11 6h4" } }
                span { "Snap" } small { class: "ml-auto text-muted-foreground", "{snap_label(settings.snap_fraction)}" }
            }
            CollapsibleContent {
                {snap_options(settings, on_snap_intent)}
                div { class: "flex items-center gap-2 px-3 py-2 text-sm",
                    Checkbox { aria_label: "Canvas handles", checked: Some(if handles_visible { CheckboxState::Checked } else { CheckboxState::Unchecked }), on_checked_change: move |v: CheckboxState| on_toggle_handles.call(v.into()) }
                    span { "Canvas handles" }
                }
            }
        }
    }
}

#[component]
pub(super) fn LayoutCommandPopup(
    mut open_menu: Signal<Option<LayoutCommandMenu>>,
    menu: LayoutCommandMenu,
    label: String,
    trigger: Element,
    children: Element,
) -> Element {
    let (key, trigger_id) = menu.meta();
    let menu_id = move || format!("m1-layout-{key}-menu");
    let is_open = open_menu() == Some(menu);
    let close = if menu == LayoutCommandMenu::Snap {
        "Close Snap".into()
    } else {
        format!("Close {label}")
    };
    use_effect(move || {
        if open_menu() == Some(menu) {
            focus_layout_popup(&menu_id());
        }
    });
    rsx! {
        Popover { open: is_open, is_modal: false,
            on_open_change: move |next| if next { open_menu.set(Some(menu)); } else if is_open { dismiss_layout_popup(open_menu, menu, &menu_id()); },
            PopoverTrigger { id: trigger_id, aria_label: label.clone(), aria_haspopup: "dialog", aria_expanded: is_open, aria_controls: is_open.then(menu_id), {trigger} }
            PopoverContent { id: menu_id(), class: "layout-settings-popup", role: "dialog", aria_label: label.clone(), inert: (!is_open).then_some(""),
                attributes: vec![onmounted(move |_| if open_menu() == Some(menu) { focus_layout_popup(&menu_id()); })],
                {popup_header(label, close, open_menu, menu)}
                {children}
            }
        }
    }
}

fn snap_options(
    settings: LayoutSnapSettings,
    on_snap_intent: EventHandler<LayoutSnapIntent>,
) -> Element {
    let scope_id = dioxus::core::current_scope_id().0;
    rsx! {
        div { class: "layout-snap-fields",
            SelectField::<String> {
                label: "Snap increment", aria_label: "Snap increment", value: settings.snap_fraction.to_string(),
                options: SNAP_STEPS.iter().map(|(_, v, l)| (v.to_string(), (*l).into())).collect(),
                on_commit: move |next: String| if let Ok(v) = next.parse::<f64>() && v.is_finite() && SNAP_STEPS.iter().any(|(s, _, _)| *s == v) { on_snap_intent.call(LayoutSnapIntent::Fraction(v)); },
            }
            ToggleField { id: format!("m1-layout-snap-geometry-{scope_id}"), aria_label: "Geometry snap", label: "Geometry snap", checked: settings.geometry_snap, on_commit: move |v| on_snap_intent.call(LayoutSnapIntent::GeometrySnap(v)) }
            ToggleField { id: format!("m1-layout-snap-envelope-gap-{scope_id}"), aria_label: "Envelope gap", label: "Envelope gap", checked: settings.gap_snap, disabled_reason: (!settings.geometry_snap).then(|| "Enable geometry snap first".into()), on_commit: move |v| on_snap_intent.call(LayoutSnapIntent::GapSnap(v)) }
            Label { html_for: "layout-snap-gap", "Gap (mm)" }
            InputGroup { InputGroupInput { id: "layout-snap-gap", r#type: InputType::Number, min: "0", step: "any", aria_label: "Snap gap", value: settings.gap_override, oninput: move |e: FormEvent| on_snap_intent.call(LayoutSnapIntent::GapOverride(e.value())) } }
            Callout { "Shared by Layout, drawing and perimeter editing. Unit steps use matrix pitch; mm steps use world coordinates. Hold Alt to bypass snapping." }
        }
    }
}
