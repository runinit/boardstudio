//! Shared 2D coordinates, grid status and camera actions.
use super::objects::{LayoutCommandMenu, LayoutSnapSettings};
use boardstudio_web_ui_shared::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonVariant},
    toolbar::{Toolbar, ToolbarButton},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
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
    let (x, y) = active_part_position
        .map(|position| (position.x, position.y))
        .unwrap_or((0.0, 0.0));
    rsx! {
        div { class: "shell-coordinates", aria_label: "Selected part coordinates",
            span { "mm" }
            span { "X " b { "{x:.2}" } }
            span { "Y " b { "{y:.2}" } }
        }
        Toolbar { class: "shell-camera-toolbar", aria_label: "Canvas view controls",
            ToolbarButton {
                index: 0usize,
                disabled: !board_available,
                aria_label: "Fit board",
                on_click: move |_| on_fit_board.call(()),
                "Fit board"
            }
            ToolbarButton {
                index: 1usize,
                disabled: !selection_available,
                on_click: move |_| on_fit_selection.call(()),
                "Fit selection"
            }
            Tooltip {
                TooltipTrigger { r#as: move |mut attributes: Vec<Attribute>| {
                    // Keep Toolbar tab order and item identity; Tooltip supplies description/events.
                    attributes.retain(|attribute| !matches!(attribute.name, "tabindex" | "data-slot"));
                    rsx! { ToolbarButton { index: 2usize, attributes, aria_label: "Zoom out", on_click: move |_| on_zoom_out.call(()), "−" } }
                } }
                TooltipContent { "Zoom out" }
            }
            span { class: "shell-zoom-label", "{zoom_percent:.0}%" }
            Tooltip {
                TooltipTrigger { r#as: move |mut attributes: Vec<Attribute>| {
                    attributes.retain(|attribute| !matches!(attribute.name, "tabindex" | "data-slot"));
                    rsx! { ToolbarButton { index: 3usize, attributes, aria_label: "Zoom in", on_click: move |_| on_zoom_in.call(()), "+" } }
                } }
                TooltipContent { "Zoom in" }
            }
        }
        if let Some(count) = findings_count {
            FindingsButton {
                count,
                on_toggle: on_toggle_findings,
            }
        }
    }
}

#[component]
pub(super) fn FindingsButton(count: usize, on_toggle: EventHandler<()>) -> Element {
    rsx! {
        Button {
            id: "m1-layout-findings-trigger", class: "ml-auto",
            variant: ButtonVariant::Ghost, aria_label: "Layout findings: {count}",
            onclick: move |_| on_toggle.call(()),
            "Layout findings: " Badge { variant: BadgeVariant::Warning, "{count}" }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen_test::*;

    fn fixture() -> Element {
        let menu = use_signal(|| None);
        let mut calls = use_signal(|| [0; 5]);
        rsx! {
            CanvasStatusFooter {
                snap_settings: LayoutSnapSettings::default(),
                active_part_position: None,
                snap_menu_available: false,
                open_menu: menu,
                board_available: true,
                selection_available: false,
                zoom_percent: 123.0,
                findings_count: Some(3),
                on_toggle_findings: move |_| calls.write()[4] += 1,
                on_fit_board: move |_| calls.write()[0] += 1,
                on_fit_selection: move |_| calls.write()[1] += 1,
                on_zoom_out: move |_| calls.write()[2] += 1,
                on_zoom_in: move |_| calls.write()[3] += 1,
            }
            output { role: "status", aria_label: "Camera calls", "{calls():?}" }
        }
    }

    #[wasm_bindgen_test]
    async fn footer_toolbar_preserves_camera_callbacks_and_disabled_selection() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the camera actions render", || {
            by_role_name(&root, "button", "Fit board").is_some()
        })
        .await;
        let toolbar = by_role_name(&root, "toolbar", "Canvas view controls")
            .expect("camera controls adopt the accessible Toolbar");
        let fit = by_role_name(&toolbar, "button", "Fit board").unwrap();
        let selection = by_role_name(&toolbar, "button", "Fit selection").unwrap();
        assert!(selection.has_attribute("disabled"));
        fit.click();
        selection.click();
        by_role_name(&toolbar, "button", "Zoom out")
            .unwrap()
            .click();
        by_role_name(&toolbar, "button", "Zoom in").unwrap().click();
        by_role_name(&root, "button", "Layout findings: 3")
            .unwrap()
            .click();
        wait_until("only enabled camera callbacks reach their owners", || {
            by_role_name(&root, "status", "Camera calls")
                .is_some_and(|output| output.text_content().as_deref() == Some("[1, 0, 1, 1, 1]"))
        })
        .await;
        root.remove();
    }
}
