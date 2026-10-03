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
    pub(super) has_selection_context: bool,
    pub(super) show_relationships: bool,
    pub(super) on_show_relationships: EventHandler<()>,
}

pub(super) struct InspectorInput {
    pub(super) geometry_scripts_open: bool,
    pub(super) on_close_geometry_scripts: EventHandler<()>,
    pub(super) context_title: Option<String>,
    pub(super) context_detail: Option<String>,
    pub(super) show_position_inspector: bool,
    pub(super) component_inspector: Option<super::inspector::LayoutComponentInspectorProjection>,
    pub(super) on_component_inspector_action:
        EventHandler<super::inspector::LayoutComponentInspectorAction>,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) key_size: objects::KeySizeMount,
    pub(super) matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub(super) inspector_tab: Signal<LayoutInspectorTab>,
    pub(super) matrix_relationship_summary: Option<String>,
    pub(super) matrix_relationship_target: Option<objects::TreeSelectRequest>,
    pub(super) on_select_context: EventHandler<objects::TreeSelectRequest>,
    pub(super) outline_inspector: Option<Box<super::outline_lifecycle::OutlineInspectorProjection>>,
    pub(super) board_inspector: Option<super::board_inspector::BoardInspectorProjection>,
    pub(super) on_board_rename: EventHandler<super::board_inspector::BoardRenameAction>,
    pub(super) findings_page: Option<super::layout_findings::InspectorMount>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutInspectorTab {
    #[default]
    Properties,
    Relations,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        div { class: "m1-layout-objects-content",
            objects::Objects {
                selected_context: input.shared.selected_context,
                on_select: input.shared.on_select,
                on_navigate: input.shared.on_navigate,
                on_nudge: input.shared.on_nudge,
                on_open_geometry_scripts: input.shared.on_open_geometry_scripts,
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
                    has_selection_context: input.has_selection_context,
                    show_relationships: input.show_relationships,
                    on_show_relationships: input.on_show_relationships,
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

pub(super) fn inspector(mut input: InspectorInput) -> Element {
    if let Some(page) = input.findings_page.filter(|page| page.open) {
        return rsx! {
            super::layout_findings::LayoutFindingsInspector {
                open: page.open,
                document: page.document,
                findings: page.findings,
                source: page.source,
                on_close: page.on_close,
                on_navigate: page.on_navigate,
            }
        };
    }
    if input.geometry_scripts_open {
        return rsx! {
            super::geometry_scripts::GeometryScriptsEditor {
                on_back: input.on_close_geometry_scripts,
            }
        };
    }
    let board_context_header = input.board_inspector.is_some();
    let matrix_context_tabs = input.component_inspector.is_none()
        && input.outline_inspector.is_none()
        && input.board_inspector.is_none()
        && input.matrix_transform_inspector.projection.is_some();
    let show_matrix_relations =
        matrix_context_tabs && (input.inspector_tab)() == LayoutInspectorTab::Relations;
    rsx! {
        if input.outline_inspector.is_none() && input.component_inspector.is_none() {
        if let Some(title) = input.context_title.as_ref() {
            section { class: if board_context_header { "m1-selected-context m1-board-context" } else { "m1-selected-context" }, "aria-label": "Selected context",
                h2 { "{title}" }
                if let Some(detail) = input.context_detail.as_ref() { p { "{detail}" } }
            }
        }
        }
        if matrix_context_tabs {
            div { class: "m1-layout-component-tabs", role: "tablist", aria_label: "Inspector details",
                button {
                    role: "tab",
                    aria_selected: "{(input.inspector_tab)() == LayoutInspectorTab::Properties}",
                    onclick: move |_| input.inspector_tab.set(LayoutInspectorTab::Properties),
                    "Properties"
                }
                button {
                    role: "tab",
                    aria_selected: "{(input.inspector_tab)() == LayoutInspectorTab::Relations}",
                    onclick: move |_| input.inspector_tab.set(LayoutInspectorTab::Relations),
                    "Relations"
                }
            }
        }
        if show_matrix_relations {
            section { class: "m1-layout-component-inspector", aria_label: "Relationships",
                h2 { "Relationships" }
                p { class: "m1-layout-component-relation-summary", "{input.matrix_relationship_summary.as_deref().unwrap_or(\"No saved placement relationship on this selection.\")}" }
                if let Some(target) = input.matrix_relationship_target.clone() {
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            input.inspector_tab.set(LayoutInspectorTab::Properties);
                            input.on_select_context.call(target.clone());
                        },
                        "Edit placement relationship"
                    }
                }
                p { class: "m1-layout-component-matrix-note", "Matrix rows and columns share pitch, stagger and splay. Edit those in Properties." }
            }
        } else {
            if input.outline_inspector.is_none() {
                if let Some(projection) = input.component_inspector {
                    super::inspector::LayoutComponentInspector {
                        projection,
                        inspector_tab: input.inspector_tab,
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
                    on_unlink: input.matrix_inspector.on_unlink,
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
            if let Some(projection) = input.board_inspector {
                super::board_inspector::BoardInspector {
                    projection,
                    on_rename: input.on_board_rename,
                }
            }
        }
    }
}
