//! Layout-owned Objects, toolbar and Inspector composition.
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use dioxus::prelude::*;

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) mirrored_pair: objects::MirroredPairMount,
    pub(super) pair_created: Signal<Option<objects::MirroredPairCreated>>,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
}

pub(super) struct ToolbarInput {
    pub(super) save_failure: Option<String>,
    pub(super) recovery_required: bool,
    pub(super) footprints_pressed: bool,
    pub(super) on_toggle_footprints: EventHandler<()>,
    pub(super) assembly_3d: bool,
    pub(super) on_view_mode: EventHandler<bool>,
    pub(super) on_retry_save: EventHandler<()>,
    pub(super) on_recover_saved: EventHandler<()>,
    pub(super) selection_kind: objects::LayoutSelectionKind,
    pub(super) snap_settings: objects::LayoutSnapSettings,
    pub(super) command_label: String,
    pub(super) align: objects::LayoutAlignMount,
    pub(super) transform: objects::LayoutTransformMenuMount,
    pub(super) menu_owner_key: String,
    pub(super) open_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub(super) on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub(super) on_snap_intent: EventHandler<objects::LayoutSnapIntent>,
}

pub(super) struct InspectorInput {
    pub(super) context_title: Option<String>,
    pub(super) context_detail: Option<String>,
    pub(super) show_position_inspector: bool,
    pub(super) component_inspector: Option<super::inspector::LayoutComponentInspectorProjection>,
    pub(super) on_component_inspector_action:
        EventHandler<super::inspector::LayoutComponentInspectorAction>,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) key_size: objects::KeySizeMount,
    pub(super) matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub(super) outline_inspector: Option<Box<super::outline_lifecycle::OutlineInspectorProjection>>,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        div { class: "m1-layout-objects-content",
            objects::Objects {
                selected_context: input.shared.selected_context,
                on_select: input.shared.on_select,
                on_navigate: input.shared.on_navigate,
                on_nudge: input.shared.on_nudge,
            board_setup: Some(input.shared.board_setup),
                matrix_setup: Some(input.matrix_setup),
                matrix_inspector: Some(input.matrix_inspector),
                mirrored_pair: Some(input.mirrored_pair),
                pair_created: Some(input.pair_created),
                on_place_component: Some(input.on_place_component),
                layout_target: Some(input.layout_target),
                parts_query: Some(input.parts_query),
                on_browse_parts: Some(input.on_browse_parts),
                placement_error: input.placement_error,
            }
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-canvas-toolbar",
            if input.assembly_3d {
                div { class: "m1-canvas-context",
                    strong { "Layout" }
                    span { "PCB assembly" }
                }
            } else {
                objects::LayoutCommandPill {
                    command_label: input.command_label,
                    menu_owner_key: input.menu_owner_key,
                    open_menu: input.open_menu,
                    transform: input.transform,
                    align: input.align,
                    selection_kind: input.selection_kind,
                    snap_settings: input.snap_settings,
                    on_selection_kind: input.on_selection_kind,
                    on_snap_intent: input.on_snap_intent,
                }
            }
            if let Some(reason) = input.save_failure.as_ref() {
                p { role: "alert", class: "m1-save-error", "Save failed: {reason}" }
                button { onclick: move |_| input.on_retry_save.call(()), "Retry save" }
            }
            if input.recovery_required {
                button { onclick: move |_| input.on_recover_saved.call(()), "Reopen last saved version (discard pending changes)" }
            }
        }
        div { class: "m1-layout-view-group", role: "group", aria_label: "Design view",
            button {
                r#type: "button",
                aria_pressed: "{!input.assembly_3d}",
                onclick: move |_| input.on_view_mode.call(false),
                "2D"
            }
            button {
                r#type: "button",
                aria_pressed: "{input.assembly_3d}",
                onclick: move |_| input.on_view_mode.call(true),
                "3D assembly"
            }
            if !input.assembly_3d {
                button {
                    r#type: "button",
                    aria_pressed: "{input.footprints_pressed}",
                    onclick: move |_| input.on_toggle_footprints.call(()),
                    "Footprints"
                }
            }
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    rsx! {
        if input.outline_inspector.is_none() && input.component_inspector.is_none() {
        if let Some(title) = input.context_title.as_ref() {
            section { class: "m1-selected-context", "aria-label": "Selected context",
                h2 { "{title}" }
                if let Some(detail) = input.context_detail.as_ref() { p { "{detail}" } }
            }
        }
        }
        if input.outline_inspector.is_none() {
            if let Some(projection) = input.component_inspector {
                super::inspector::LayoutComponentInspector {
                    projection,
                    on_action: input.on_component_inspector_action,
                }
            } else if input.show_position_inspector { super::inspector::Inspector {} }
        }
        if let Some(projection) = input.matrix_inspector.projection.clone() {
            objects::MatrixInspector {
                projection,
                request_sequence: input.matrix_inspector.request_sequence,
                editable: input.matrix_inspector.editable,
                busy: input.matrix_inspector.busy,
                feedback: input.matrix_inspector.feedback.clone(),
                on_edit: input.matrix_inspector.on_edit,
                on_apply_preset: input.matrix_inspector.on_apply_preset,
                on_delete: input.matrix_inspector.on_delete,
                on_duplicate: input.matrix_inspector.on_duplicate,
            }
        }
        if input.key_size.projection.is_some() {
            objects::KeySizeControls { mount: input.key_size }
        }
        if input.matrix_transform_inspector.projection.is_some() {
            objects::MatrixTransformInspector {
                mount: input.matrix_transform_inspector,
            }
        }
        if let Some(projection) = input.outline_inspector {
            super::outline_lifecycle::OutlineVersionInspector { projection: *projection }
        }
    }
}
