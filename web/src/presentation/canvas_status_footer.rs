//! Shared 2D coordinates, grid status and camera actions.
use super::objects::{LayoutCommandMenu, LayoutSnapSettings};
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
    let (x, y) = active_part_position
        .map(|position| (position.x, position.y))
        .unwrap_or((0.0, 0.0));
    rsx! {
        div { class: "m1-canvas-footer-coordinates", aria_label: "Selected part coordinates",
            span { "mm" }
            span { "X " b { "{x:.2}" } }
            span { "Y " b { "{y:.2}" } }
        }
        div { class: "m1-canvas-footer-grid",
            button {
                r#type: "button",
                id: "m1-canvas-grid-trigger",
                disabled: !snap_menu_available,
                onclick: {
                    let mut open_menu = open_menu;
                    move |_| open_menu.set(Some(LayoutCommandMenu::Snap))
                },
                "Grid {super::objects::layout_snap_label(snap_settings.snap_fraction)}"
            }
            span { if snap_settings.geometry_snap { "Geometry snap on" } else { "Geometry snap off" } }
        }
        div { class: "m1-canvas-footer-view-controls", role: "group", aria_label: "Canvas view controls",
            button {
                class: "m1-canvas-footer-fit",
                r#type: "button",
                disabled: !board_available,
                aria_label: "Fit board",
                title: "Fit entire board",
                onclick: move |_| on_fit_board.call(()),
                "Fit board"
            }
            button {
                class: "m1-canvas-footer-fit",
                r#type: "button",
                disabled: !selection_available,
                onclick: move |_| on_fit_selection.call(()),
                "Fit selection"
            }
            button { class: "m1-canvas-footer-zoom", r#type: "button", aria_label: "Zoom out", title: "Zoom out", onclick: move |_| on_zoom_out.call(()), "−" }
            span { class: "m1-canvas-footer-zoom-label", "{zoom_percent:.0}%" }
            button { class: "m1-canvas-footer-zoom", r#type: "button", aria_label: "Zoom in", title: "Zoom in", onclick: move |_| on_zoom_in.call(()), "+" }
        }
        if let Some(count) = findings_count {
            super::layout_findings::LayoutFindingsFooterButton {
                count,
                on_toggle: on_toggle_findings,
            }
        }
    }
}
