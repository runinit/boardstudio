//! Shared 2D coordinates, grid status and camera actions.
use super::objects::{LayoutCommandMenu, LayoutSnapSettings};
use boardstudio_web_ui_shared::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonVariant},
    toolbar::{Toolbar, ToolbarButton},
};
use dioxus::prelude::*;

#[component]
pub(in crate::presentation) fn CanvasStatusFooter(
    snap_settings: LayoutSnapSettings,
    active_part_position: Option<boardstudio_core::model::Vec2>,
    snap_menu_available: bool,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    board_available: bool,
    selection_available: bool,
    zoom_percent: f64,
    findings_count: Option<usize>,
    on_toggle_findings: EventHandler<()>,
    on_fit_board: EventHandler<()>,
    on_fit_selection: EventHandler<()>,
    on_zoom_out: EventHandler<()>,
    on_zoom_in: EventHandler<()>,
) -> Element {
    let _ = (snap_settings, snap_menu_available, open_menu);
    let (x, y) = active_part_position.map_or((0.0, 0.0), |p| (p.x, p.y));
    rsx! {
        div { class: "shell-coordinates", aria_label: "Selected part coordinates",
            span { "mm" } span { "X " b { "{x:.2}" } } span { "Y " b { "{y:.2}" } }
        }
        Toolbar { class: "shell-camera-toolbar", aria_label: "Canvas view controls",
            ToolbarButton { index: 0usize, disabled: !board_available, aria_label: "Fit board", on_click: move |_| on_fit_board.call(()), "Fit board" }
            ToolbarButton { index: 1usize, disabled: !selection_available, on_click: move |_| on_fit_selection.call(()), "Fit selection" }
            ToolbarButton { index: 2usize, aria_label: "Zoom out", on_click: move |_| on_zoom_out.call(()), "−" }
            span { class: "shell-zoom-label", "{zoom_percent:.0}%" }
            ToolbarButton { index: 3usize, aria_label: "Zoom in", on_click: move |_| on_zoom_in.call(()), "+" }
        }
        if let Some(count) = findings_count { FindingsButton { count, on_toggle: on_toggle_findings } }
    }
}

#[component]
pub(super) fn FindingsButton(count: usize, on_toggle: EventHandler<()>) -> Element {
    rsx! {
        Button { id: "m1-layout-findings-trigger", class: "ml-auto", variant: ButtonVariant::Ghost, aria_label: "Layout findings: {count}",
            onclick: move |_| on_toggle.call(()), "Layout findings: " Badge { variant: BadgeVariant::Warning, "{count}" } }
    }
}
