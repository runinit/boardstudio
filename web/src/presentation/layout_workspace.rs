//! Layout-owned Objects, toolbar and Inspector composition.
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use dioxus::prelude::*;

pub(super) struct ToolbarInput {
    pub(super) selection_indicator: Option<String>,
    pub(super) document_name: String,
    pub(super) save_failure: Option<String>,
    pub(super) recovery_required: bool,
    pub(super) footprints_pressed: bool,
    pub(super) on_toggle_footprints: EventHandler<()>,
    pub(super) on_retry_save: EventHandler<()>,
    pub(super) on_recover_saved: EventHandler<()>,
}

pub(super) struct InspectorInput {
    pub(super) context_title: Option<String>,
    pub(super) context_detail: Option<String>,
    pub(super) show_position_inspector: bool,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
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

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-canvas-toolbar",
            if let Some(indicator) = input.selection_indicator.as_ref() {
                span { class: "m1-selection-indicator", "{indicator}" }
            }
            span { "{input.document_name}" }
            button {
                class: "m1-footprints-toggle",
                aria_pressed: "{input.footprints_pressed}",
                onclick: move |_| input.on_toggle_footprints.call(()),
                "Footprints"
            }
            if let Some(reason) = input.save_failure.as_ref() {
                p { role: "alert", class: "m1-save-error", "Save failed: {reason}" }
                button { onclick: move |_| input.on_retry_save.call(()), "Retry save" }
            }
            if input.recovery_required {
                button { onclick: move |_| input.on_recover_saved.call(()), "Reopen last saved version (discard pending changes)" }
            }
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    rsx! {
        if let Some(title) = input.context_title.as_ref() {
            section { class: "m1-selected-context", "aria-label": "Selected context",
                h2 { "{title}" }
                if let Some(detail) = input.context_detail.as_ref() { p { "{detail}" } }
            }
        }
        if input.show_position_inspector { super::inspector::Inspector {} }
        if let Some(projection) = input.matrix_inspector.projection.clone() {
            objects::MatrixInspector {
                projection,
                request_sequence: input.matrix_inspector.request_sequence,
                editable: input.matrix_inspector.editable,
                busy: input.matrix_inspector.busy,
                feedback: input.matrix_inspector.feedback.clone(),
                on_edit: input.matrix_inspector.on_edit,
            }
        }
    }
}
