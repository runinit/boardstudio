//! Case-owned Objects, viewer mount point and Inspector composition.
use super::case_controller::CaseBodyInspector;
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use crate::cad_presentation::CasePanel;
use dioxus::prelude::*;

pub(super) struct CanvasInput {
    pub(super) mechanical_settings: super::MechanicalSettingsMount,
    pub(super) instance_scope_pending: bool,
}

pub(super) struct InspectorInput {
    pub(super) on_show_configured_board: EventHandler<String>,
    pub(super) instance_scope_pending: bool,
}

pub(super) fn objects(input: SharedObjectsInput) -> Element {
    rsx! {
        objects::Objects {
            selected_context: input.selected_context,
            on_select: input.on_select,
            on_navigate: input.on_navigate,
            on_nudge: input.on_nudge,
        }
    }
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    if input.instance_scope_pending {
        rsx! { p { role: "status", "Selecting physical assembly…" } }
    } else {
        rsx! { CasePanel { mechanical_settings: input.mechanical_settings } }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    if input.instance_scope_pending {
        rsx! { p { role: "status", "Selecting physical assembly…" } }
    } else {
        rsx! {
            CaseBodyInspector { on_show_configured_board: input.on_show_configured_board }
        }
    }
}
