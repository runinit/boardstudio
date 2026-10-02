//! Layout-owned Objects, toolbar and Inspector composition.
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use dioxus::prelude::*;

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
}

pub(super) struct ToolbarInput {
    pub(super) selection_indicator: Option<String>,
    pub(super) document_name: String,
    pub(super) save_failure: Option<String>,
    pub(super) recovery_required: bool,
    pub(super) footprints_pressed: bool,
    pub(super) on_toggle_footprints: EventHandler<()>,
    pub(super) on_retry_save: EventHandler<()>,
    pub(super) on_recover_saved: EventHandler<()>,
    pub(super) selection_kind: objects::LayoutSelectionKind,
    pub(super) snap_settings: objects::LayoutSnapSettings,
    pub(super) align: objects::LayoutAlignMount,
    pub(super) on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub(super) on_snap_intent: EventHandler<objects::LayoutSnapIntent>,
}

pub(super) struct InspectorInput {
    pub(super) context_title: Option<String>,
    pub(super) context_detail: Option<String>,
    pub(super) show_position_inspector: bool,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) matrix_transform_inspector: objects::MatrixTransformInspectorMount,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        div { class: "m1-layout-objects-content",
            objects::Objects {
                selected_context: input.shared.selected_context,
                on_select: input.shared.on_select,
                on_navigate: input.shared.on_navigate,
                on_nudge: input.shared.on_nudge,
            }
            LayoutAddObjectEntry { mount: input.matrix_setup }
        }
    }
}

#[component]
fn LayoutAddObjectEntry(mount: objects::MatrixSetupMount) -> Element {
    let mut menu_open = use_signal(|| false);
    let open_menu = menu_open();
    rsx! {
        div { class: "m1-layout-add-object",
            button {
                r#type: "button",
                aria_expanded: open_menu,
                onclick: move |_| menu_open.set(!menu_open()),
                "Add object"
            }
            if open_menu {
                div { role: "dialog", "aria-label": "Add", class: "m1-layout-add-menu",
                    section { "aria-label": "Layouts",
                        h3 { "Layouts" }
                        button {
                            r#type: "button",
                            disabled: !mount.can_open,
                            onclick: move |_| {
                                menu_open.set(false);
                                mount.on_open.call(());
                            },
                            "Matrix…"
                            small { "Rows, columns & key assemblies" }
                        }
                    }
                }
            }
            if let Some(projection) = mount.projection.clone() {
                objects::MatrixSetup {
                    projection,
                    on_cancel: mount.on_cancel,
                    on_create: mount.on_create,
                }
            }
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-canvas-toolbar",
            objects::LayoutSelectionSnapToolbar {
                selection_kind: input.selection_kind,
                snap_settings: input.snap_settings,
                on_selection_kind: input.on_selection_kind,
                on_snap_intent: input.on_snap_intent,
            }
            objects::LayoutAlignToolbar { mount: input.align.clone() }
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
        if input.matrix_transform_inspector.projection.is_some() {
            objects::MatrixTransformInspector {
                mount: input.matrix_transform_inspector,
            }
        }
    }
}
