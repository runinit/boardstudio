//! Layout-owned Objects, toolbar and Inspector composition.
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use boardstudio_web_ui_model::split_focus::SplitHalf;
use boardstudio_web_ui_shared::controls::{BoardButton, BoardDisclosure};
use dioxus::prelude::*;
use std::rc::Rc;

type TransformToolOwner = (
    Option<boardstudio_application::Scope>,
    u64,
    Option<String>,
    &'static str,
    bool,
);
type InspectorFieldFailures = (
    Signal<Option<String>>,
    Signal<Option<String>>,
    Signal<Option<String>>,
);

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) pair_created: Signal<Option<objects::MirroredPairCreated>>,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
    pub(super) add_object_menu_open: Signal<bool>,
}

pub(super) struct ToolbarInput {
    pub(super) save_failure: Option<String>,
    pub(super) recovery_required: bool,
    pub(super) footprints_pressed: bool,
    pub(super) on_toggle_footprints: EventHandler<()>,
    pub(super) assembly_3d: bool,
    pub(super) on_view_mode: EventHandler<bool>,
    pub(super) on_retry_save: EventHandler<()>,
    pub(super) save_conflict: bool,
    pub(super) on_download_pending: EventHandler<()>,
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
    pub(super) component_inspector_pending_edits:
        dioxus::prelude::Signal<super::layout_component_edits::LayoutComponentInspectorEdits>,
    pub(super) component_inspector_x: Signal<String>,
    pub(super) component_inspector_y: Signal<String>,
    pub(super) component_inspector_margin: Signal<String>,
    pub(super) component_inspector_error: Signal<Option<String>>,
    pub(super) component_inspector_x_failure: Signal<Option<String>>,
    pub(super) component_inspector_y_failure: Signal<Option<String>>,
    pub(super) component_inspector_margin_failure: Signal<Option<String>>,
    pub(super) component_inspector_layout_pending: Signal<bool>,
    pub(super) component_inspector_constraint_pending: Signal<bool>,
    pub(super) component_inspector_remove_pending: Signal<bool>,
    pub(super) component_inspector_mirrored_half_pending: Signal<bool>,
    pub(super) companion_placement: Option<super::inspector::CompanionPlacementProjection>,
    pub(super) companion_placement_pending: Signal<bool>,
    pub(super) companion_placement_error: Signal<Option<String>>,
    pub(super) companion_placement_edits:
        Signal<super::layout_component_edits::LayoutComponentInspectorEdits>,
    pub(super) on_companion_placement_action:
        EventHandler<super::inspector::CompanionPlacementAction>,
    pub(super) on_component_inspector_action:
        EventHandler<super::inspector::LayoutComponentInspectorAction>,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) key_size: objects::KeySizeMount,
    pub(super) matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub(super) on_pick_splay_origin: EventHandler<()>,
    pub(super) splay_origin_pick_pending: bool,
    pub(super) on_cancel_splay_origin_pick: EventHandler<()>,
    pub(super) on_pick_anchor: EventHandler<()>,
    pub(super) anchor_pick_pending: bool,
    pub(super) on_cancel_anchor_pick: EventHandler<()>,
    /// Select an aggregate anchor part's key without writing.
    pub(super) on_navigate_anchor_part: EventHandler<objects::TreeContext>,
    pub(super) matrix_relations: Option<boardstudio_web_layout::matrix_relations::MatrixRelations>,
    /// Resolves a Relations reference against the live accepted document at click time:
    /// the request to submit (a component opens its Inspector context), or `None` when the
    /// target has gone.
    pub(super) relation_target:
        Rc<dyn Fn(objects::TreeSelectRequest) -> Option<objects::TreeSelectRequest>>,
    /// The scope of the selected tree context, which Relations navigation requests carry.
    pub(super) relation_scope: Option<boardstudio_application::Scope>,
    pub(super) on_select_context: EventHandler<objects::TreeSelectRequest>,
    pub(super) outline_inspector: Option<Box<super::outline_lifecycle::OutlineInspectorProjection>>,
    pub(super) findings_return_available: bool,
    pub(super) on_findings_return: EventHandler<()>,
    pub(super) board_inspector_mount: super::board_inspector::BoardInspectorMount,
    pub(super) findings_page: Option<super::layout_findings::InspectorMount>,
}

/// Layout's selection, command, transform, and view state. Editor retains the
/// cross-workspace owner and supplies the current selected tree context.
#[derive(Clone, Copy)]
pub(super) struct LayoutWorkspaceState {
    selection_kind: Signal<objects::LayoutSelectionKind>,
    snap_settings: Signal<objects::LayoutSnapSettings>,
    command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    transform_tool_owner: Signal<Option<TransformToolOwner>>,
    transform_handles: Signal<bool>,
    /// Ticket 14: view-only editing focus for split boards. Like Handles, this
    /// is workspace state, never a document edit: it starts on the source half
    /// and survives workspace switches and spacing changes.
    split_focus: Signal<SplitHalf>,
    assembly_3d: Signal<bool>,
    splay_affect: Signal<boardstudio_core::model::MatrixSplayAffect>,
    pending_splay_origin_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pair_created_selection: Signal<Option<objects::MirroredPairCreated>>,
    component_inspector_edits: Signal<super::layout_component_edits::LayoutComponentInspectorEdits>,
    component_inspector_x: Signal<String>,
    component_inspector_y: Signal<String>,
    component_inspector_margin: Signal<String>,
    component_inspector_error: Signal<Option<String>>,
    component_inspector_x_failure: Signal<Option<String>>,
    component_inspector_y_failure: Signal<Option<String>>,
    component_inspector_margin_failure: Signal<Option<String>>,
    component_inspector_layout_pending: Signal<bool>,
    component_inspector_constraint_pending: Signal<bool>,
    component_inspector_remove_pending: Signal<bool>,
    component_inspector_mirrored_half_pending: Signal<bool>,
    companion_placement_pending: Signal<bool>,
    companion_placement_error: Signal<Option<String>>,
    companion_placement_edits: Signal<super::layout_component_edits::LayoutComponentInspectorEdits>,
    add_object_menu_open: Signal<bool>,
}

pub(super) fn use_layout_workspace_state(
    _selected_context: Signal<Option<objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
) -> LayoutWorkspaceState {
    let mut state = LayoutWorkspaceState {
        selection_kind: use_signal(objects::LayoutSelectionKind::default),
        snap_settings: use_signal(objects::LayoutSnapSettings::default),
        command_menu: use_signal(|| None),
        transform_tool: use_signal(|| None),
        transform_tool_owner: use_signal(|| None),
        transform_handles: use_signal(|| true),
        split_focus: use_signal(SplitHalf::default),
        assembly_3d: use_signal(|| false),
        splay_affect: use_signal(|| boardstudio_core::model::MatrixSplayAffect::Following),
        pending_splay_origin_pick: use_signal(|| None),
        pending_anchor_pick: use_signal(|| None),
        pair_created_selection: use_signal(|| None),
        component_inspector_edits: use_signal(
            super::layout_component_edits::LayoutComponentInspectorEdits::new,
        ),
        component_inspector_x: use_signal(String::new),
        component_inspector_y: use_signal(String::new),
        component_inspector_margin: use_signal(String::new),
        component_inspector_error: use_signal(|| None),
        component_inspector_x_failure: use_signal(|| None),
        component_inspector_y_failure: use_signal(|| None),
        component_inspector_margin_failure: use_signal(|| None),
        component_inspector_layout_pending: use_signal(|| false),
        component_inspector_constraint_pending: use_signal(|| false),
        component_inspector_remove_pending: use_signal(|| false),
        component_inspector_mirrored_half_pending: use_signal(|| false),
        companion_placement_pending: use_signal(|| false),
        companion_placement_error: use_signal(|| None),
        companion_placement_edits: use_signal(
            super::layout_component_edits::LayoutComponentInspectorEdits::default,
        ),
        add_object_menu_open: use_signal(|| false),
    };
    let observed_workspace = workspace();
    use_effect(use_reactive!(|observed_workspace| {
        if observed_workspace != "Layout" {
            state.transform_tool.set(None);
        }
    }));
    let observed_assembly_3d = state.assembly_3d()();
    use_effect(use_reactive!(|observed_assembly_3d| {
        if observed_assembly_3d {
            state.pending_splay_origin_pick.set(None);
            state.pending_anchor_pick.set(None);
        }
    }));
    state
}

impl LayoutWorkspaceState {
    pub(super) fn add_object_menu_open(self) -> Signal<bool> {
        self.add_object_menu_open
    }
    pub(super) fn selection_kind(self) -> Signal<objects::LayoutSelectionKind> {
        self.selection_kind
    }
    pub(super) fn snap_settings(self) -> Signal<objects::LayoutSnapSettings> {
        self.snap_settings
    }
    pub(super) fn command_menu(self) -> Signal<Option<objects::LayoutCommandMenu>> {
        self.command_menu
    }
    pub(super) fn transform_tool(self) -> Signal<Option<objects::LayoutTransformTool>> {
        self.transform_tool
    }
    pub(super) fn transform_handles(self) -> Signal<bool> {
        self.transform_handles
    }
    pub(super) fn split_focus(self) -> Signal<SplitHalf> {
        self.split_focus
    }
    pub(super) fn assembly_3d(self) -> Signal<bool> {
        self.assembly_3d
    }
    pub(super) fn splay_affect(self) -> Signal<boardstudio_core::model::MatrixSplayAffect> {
        self.splay_affect
    }
    pub(super) fn pending_splay_origin_pick(
        self,
    ) -> Signal<Option<objects::MatrixTransformInspectorOwner>> {
        self.pending_splay_origin_pick
    }
    pub(super) fn pending_anchor_pick(
        self,
    ) -> Signal<Option<objects::MatrixTransformInspectorOwner>> {
        self.pending_anchor_pick
    }
    pub(super) fn pair_created_selection(self) -> Signal<Option<objects::MirroredPairCreated>> {
        self.pair_created_selection
    }

    pub(super) fn component_inspector_edits(
        self,
    ) -> Signal<super::layout_component_edits::LayoutComponentInspectorEdits> {
        self.component_inspector_edits
    }
    pub(super) fn component_inspector_drafts(
        self,
    ) -> (
        Signal<String>,
        Signal<String>,
        Signal<String>,
        Signal<Option<String>>,
    ) {
        (
            self.component_inspector_x,
            self.component_inspector_y,
            self.component_inspector_margin,
            self.component_inspector_error,
        )
    }
    pub(super) fn component_inspector_pending_controls(
        self,
    ) -> (Signal<bool>, Signal<bool>, Signal<bool>, Signal<bool>) {
        (
            self.component_inspector_layout_pending,
            self.component_inspector_constraint_pending,
            self.component_inspector_remove_pending,
            self.component_inspector_mirrored_half_pending,
        )
    }

    pub(super) fn companion_placement_controls(
        self,
    ) -> (
        Signal<bool>,
        Signal<Option<String>>,
        Signal<super::layout_component_edits::LayoutComponentInspectorEdits>,
    ) {
        (
            self.companion_placement_pending,
            self.companion_placement_error,
            self.companion_placement_edits,
        )
    }

    pub(super) fn component_inspector_field_failures(self) -> InspectorFieldFailures {
        (
            self.component_inspector_x_failure,
            self.component_inspector_y_failure,
            self.component_inspector_margin_failure,
        )
    }

    pub(super) fn set_selection_kind_from_context(mut self, context: &objects::TreeContext) {
        if let Some(kind) = selection_kind_for_tree_context(context) {
            self.selection_kind.set(kind);
        }
    }

    pub(super) fn set_assembly_3d(mut self, assembly_3d: bool) {
        self.assembly_3d.set(assembly_3d);
        if assembly_3d {
            self.pending_splay_origin_pick.set(None);
            self.pending_anchor_pick.set(None);
        }
    }

    pub(super) fn observe_transform_tool_owner(self, owner: TransformToolOwner) {
        let mut state = self;
        let owner = owner.clone();
        use_effect(use_reactive!(|owner| {
            if state
                .transform_tool_owner
                .peek()
                .as_ref()
                .is_some_and(|previous| previous != &owner)
            {
                state.transform_tool.set(None);
            }
            state.transform_tool_owner.set(Some(owner));
        }));
    }
}

pub(super) fn selection_kind_for_tree_context(
    context: &objects::TreeContext,
) -> Option<objects::LayoutSelectionKind> {
    match context {
        objects::TreeContext::Matrix { .. } => Some(objects::LayoutSelectionKind::Matrix),
        objects::TreeContext::Row { .. } => Some(objects::LayoutSelectionKind::Row),
        objects::TreeContext::Column { .. } => Some(objects::LayoutSelectionKind::Column),
        objects::TreeContext::Key { .. } => Some(objects::LayoutSelectionKind::Key),
        objects::TreeContext::Component { .. } => Some(objects::LayoutSelectionKind::Part),
        objects::TreeContext::Outline { .. }
        | objects::TreeContext::OutlineVersion { .. }
        | objects::TreeContext::Bridge { .. }
        | objects::TreeContext::MountedModule { .. }
        | objects::TreeContext::Board { .. }
        | objects::TreeContext::LayoutGroup { .. } => None,
    }
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
                pair_created: Some(input.pair_created),
                on_place_component: Some(input.on_place_component),
                layout_target: Some(input.layout_target),
                parts_query: Some(input.parts_query),
                on_browse_parts: Some(input.on_browse_parts),
                placement_error: input.placement_error,
                add_object_menu_open: Some(input.add_object_menu_open),
            }
        }
    }
}

pub(super) fn add_object_header_action(mut menu_open: Signal<bool>) -> Element {
    let open = menu_open();
    rsx! {
        div { class: "m1-layout-add-object",
            button {
                r#type: "button",
                id: "m1-add-object-trigger",
                aria_label: "Add object",
                title: "Add object",
                class: "m1-layout-add-trigger",
                aria_expanded: if open { "true" } else { "false" },
                onclick: move |_| menu_open.set(!menu_open()),
                svg { class: "m1-add-icon", "aria-hidden": "true", view_box: "0 0 16 16",
                    path { d: "M8 3v10M3 8h10" }
                }
                "Add object"
            }
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-layout-toolbar-region",
            div { class: "m1-canvas-toolbar",
                if !input.assembly_3d {
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
                        snap_in_dock: true,
                    }
                }
            }
            if let Some(reason) = input.save_failure.as_ref() {
                p { role: "alert", class: "m1-save-error", "Save failed: {reason}" }
                if !input.save_conflict {
                    BoardButton { onclick: move |_| input.on_retry_save.call(()), "Retry save" }
                }
                BoardButton { onclick: move |_| input.on_download_pending.call(()), "Download my pending edits" }
            }
            if input.recovery_required {
                BoardButton { onclick: move |_| input.on_recover_saved.call(()), "Reload saved version" }
            }
            div { class: "m1-layout-view-group board-tool-pill", role: "group", aria_label: "Design view",
                button {
                    r#type: "button",
                    aria_label: "2D", title: "2D layout",
                    aria_pressed: "{!input.assembly_3d}",
                    onclick: move |_| input.on_view_mode.call(false),
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", rect { x: "3", y: "3", width: "14", height: "14", rx: "2" } path { d: "M3 8h14M8 3v14" } }
                    span { class: "m1-visually-hidden", "2D" }
                }
                button {
                    r#type: "button",
                    aria_label: "3D assembly", title: "3D assembly",
                    aria_pressed: "{input.assembly_3d}",
                    onclick: move |_| input.on_view_mode.call(true),
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8" } }
                    span { class: "m1-visually-hidden", "3D assembly" }
                }
                if !input.assembly_3d {
                    button {
                        r#type: "button",
                        aria_label: "Footprints", title: "Footprints",
                        aria_pressed: "{input.footprints_pressed}",
                        onclick: move |_| input.on_toggle_footprints.call(()),
                        svg { view_box: "0 0 20 20", "aria-hidden": "true", rect { x: "5", y: "4", width: "10", height: "12", rx: "1" } path { d: "M2 7h3M2 13h3M15 7h3M15 13h3" } }
                        span { class: "m1-visually-hidden", "Footprints" }
                    }
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod workspace_state_tests {
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch};
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn state_host() -> Element {
        let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
        let mut workspace = use_signal(|| "Layout");
        let state = use_layout_workspace_state(selected_context, workspace);
        let selection_kind = state.selection_kind();
        let mut pending_pick = state.pending_splay_origin_pick();
        let mut pending_anchor_pick = state.pending_anchor_pick();
        let mut active_tool = state.transform_tool();
        let owner = objects::MatrixTransformInspectorOwner {
            editor_instance_id: 1,
            workspace: "Layout",
            context_generation: 1,
            scope_generation: 1,
            scope: Scope {
                session_epoch: SessionEpoch(1),
                document_id: "layout-state".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            matrix_id: "matrix".into(),
            context: objects::TreeContext::Matrix {
                matrix_id: "matrix".into(),
            },
        };
        let selection_owner = owner.clone();
        let pick_owner = owner.clone();
        let anchor_pick_owner = owner.clone();
        let selection_kind_label = match selection_kind() {
            objects::LayoutSelectionKind::Matrix => "Matrix",
            objects::LayoutSelectionKind::Row => "Row",
            objects::LayoutSelectionKind::Column => "Column",
            objects::LayoutSelectionKind::Key => "Key",
            objects::LayoutSelectionKind::Part => "Part",
        };
        let selection_present = selected_context().is_some();
        rsx! {
            button { id: "layout-state-select", onclick: move |_| {
                let context = objects::TreeContext::Matrix { matrix_id: "matrix".into() };
                selected_context.set(Some(objects::ScopedTreeContext {
                    scope: selection_owner.scope.clone(),
                    context: context.clone(),
                }));
                state.set_selection_kind_from_context(&context);
            }, "Select matrix" }
            button { id: "layout-state-arm-pick", onclick: move |_| pending_pick.set(Some(pick_owner.clone())), "Arm origin pick" }
            button { id: "layout-state-arm-anchor-pick", onclick: move |_| pending_anchor_pick.set(Some(anchor_pick_owner.clone())), "Arm anchor pick" }
            button { id: "layout-state-enter-3d", onclick: move |_| state.set_assembly_3d(true), "Enter 3D" }
            button { id: "layout-state-tool", onclick: move |_| active_tool.set(Some(objects::LayoutTransformTool::Stagger)), "Activate tool" }
            button { id: "layout-state-switch", onclick: move |_| workspace.set("Keycaps"), "Switch workspace" }
            output { id: "layout-state-selection-kind", "{selection_kind_label}" }
            output { id: "layout-state-selection-present", "{selection_present}" }
            output { id: "layout-state-pending", "{pending_pick().is_some()}" }
            output { id: "layout-state-anchor-pending", "{pending_anchor_pick().is_some()}" }
            output { id: "layout-state-tool-active", "{active_tool().is_some()}" }
        }
    }

    fn click(document: &web_sys::Document, id: &str) {
        document
            .get_element_by_id(id)
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    #[wasm_bindgen_test]
    async fn handle_resets_layout_interactions_when_selection_or_owner_changes() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(state_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("Layout workspace state controls mount", || {
            root.query_selector("#layout-state-select")
                .is_ok_and(|button| button.is_some())
        })
        .await;

        click(&document, "layout-state-select");
        wait_until("Layout selection state becomes present", || {
            root.query_selector("#layout-state-selection-present")
                .ok()
                .flatten()
                .is_some_and(|selected| selected.text_content().as_deref() == Some("true"))
        })
        .await;
        assert_eq!(
            document
                .get_element_by_id("layout-state-selection-kind")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Matrix")
        );
        click(&document, "layout-state-arm-pick");
        wait_until("the origin pick becomes armed", || {
            root.query_selector("#layout-state-pending")
                .ok()
                .flatten()
                .is_some_and(|pending| pending.text_content().as_deref() == Some("true"))
        })
        .await;
        click(&document, "layout-state-arm-anchor-pick");
        wait_until("the anchor pick becomes armed", || {
            root.query_selector("#layout-state-anchor-pending")
                .ok()
                .flatten()
                .is_some_and(|pending| pending.text_content().as_deref() == Some("true"))
        })
        .await;
        click(&document, "layout-state-enter-3d");
        wait_until("entering 3D clears the origin pick", || {
            root.query_selector("#layout-state-pending")
                .ok()
                .flatten()
                .is_some_and(|pending| pending.text_content().as_deref() == Some("false"))
        })
        .await;
        wait_until("entering 3D clears the anchor pick", || {
            root.query_selector("#layout-state-anchor-pending")
                .ok()
                .flatten()
                .is_some_and(|pending| pending.text_content().as_deref() == Some("false"))
        })
        .await;
        assert_eq!(
            document
                .get_element_by_id("layout-state-pending")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("false")
        );
        assert_eq!(
            document
                .get_element_by_id("layout-state-anchor-pending")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("false")
        );
        click(&document, "layout-state-tool");
        wait_until("the transform tool becomes active", || {
            root.query_selector("#layout-state-tool-active")
                .ok()
                .flatten()
                .is_some_and(|active| active.text_content().as_deref() == Some("true"))
        })
        .await;
        click(&document, "layout-state-switch");
        wait_until("leaving Layout clears the transform tool", || {
            root.query_selector("#layout-state-tool-active")
                .ok()
                .flatten()
                .is_some_and(|active| active.text_content().as_deref() == Some("false"))
        })
        .await;
        assert_eq!(
            document
                .get_element_by_id("layout-state-tool-active")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("false")
        );
        root.remove();
    }
}

pub(super) fn inspector(mut input: InspectorInput) -> Element {
    let mut matrix_relationship_escape_target = use_signal(|| None::<objects::TreeSelectRequest>);
    let component_owner = input
        .component_inspector
        .as_ref()
        .map(|projection| projection.owner.clone());
    let board_inspector_visible = input.board_inspector_mount.projection.is_some();
    let observed_relationship_target = matrix_relationship_escape_target();
    let observed_component_owner = component_owner.clone();
    let observed_board_visible = board_inspector_visible;
    use_effect(use_reactive!(
        |observed_relationship_target, observed_component_owner, observed_board_visible| {
            let Some(target_value) = observed_relationship_target.as_ref() else {
                return;
            };
            if observed_board_visible
                || observed_component_owner.as_ref().is_some_and(|owner| {
                    !matrix_relationship_escape_target_is_current(target_value, owner)
                })
            {
                matrix_relationship_escape_target.set(None);
            }
        }
    ));
    let mut return_target = matrix_relationship_escape_target;
    let on_matrix_relationship_escape =
        EventHandler::new(move |request: objects::TreeSelectRequest| {
            return_target.set(None);
            input.on_select_context.call(request);
        });
    let relation_scope = input.relation_scope.clone();
    let relation_target = input.relation_target.clone();
    let relation_select = input.on_select_context;
    let mut relation_escape_target = matrix_relationship_escape_target;
    let on_navigate_relation = EventHandler::new(move |context: objects::TreeContext| {
        let Some(scope) = relation_scope.clone() else {
            return;
        };
        let request = objects::TreeSelectRequest {
            scope,
            context,
            mode: boardstudio_application::SelectionMode::Replace,
            outline_action: None,
        };
        // A target removed since the row was projected: do nothing; the next accepted
        // revision re-projects without it.
        let Some(request) = relation_target(request) else {
            return;
        };
        if matches!(request.context, objects::TreeContext::Component { .. }) {
            relation_escape_target.set(Some(request.clone()));
        }
        relation_select.call(request);
    });
    // Parameter entries select their owner and open the matching tool/editor
    // beside the selection. The same currency guard applies: a target removed
    // since the row was projected does nothing.
    let open_scope = input.relation_scope.clone();
    let open_target = input.relation_target.clone();
    let open_select = input.on_select_context;
    let on_open_owner_relation = EventHandler::new(move |context: objects::TreeContext| {
        let Some(scope) = open_scope.clone() else {
            return;
        };
        let request = objects::TreeSelectRequest {
            scope,
            context,
            mode: boardstudio_application::SelectionMode::Replace,
            outline_action: None,
        };
        let Some(request) = open_target(request) else {
            return;
        };
        open_select.call(request);
    });
    if let Some(page) = input.findings_page.filter(|page| page.open) {
        return rsx! {
            PendingSplayOriginPickEscape {
                pending: input.splay_origin_pick_pending,
                on_cancel: input.on_cancel_splay_origin_pick,
            }
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
            PendingSplayOriginPickEscape {
                pending: input.splay_origin_pick_pending,
                on_cancel: input.on_cancel_splay_origin_pick,
            }
            super::geometry_scripts::GeometryScriptsEditor {
                on_back: input.on_close_geometry_scripts,
            }
        };
    }
    let board_context_header = input.board_inspector_mount.projection.is_some();
    let matrix_context = input.component_inspector.is_none()
        && input.outline_inspector.is_none()
        && input.board_inspector_mount.projection.is_none()
        && input.matrix_transform_inspector.projection.is_some();
    rsx! {
        PendingSplayOriginPickEscape {
            pending: input.splay_origin_pick_pending,
            on_cancel: input.on_cancel_splay_origin_pick,
        }
        super::layout_key_selection_actions::SelectedKeysActions {}
        MatrixRelationshipEscape {
            target: matrix_relationship_escape_target(),
            component_owner,
            on_escape: on_matrix_relationship_escape,
        }
        if input.findings_return_available {
            div { class: "m1-layout-findings-return",
                button {
                    class: "m1-layout-findings-back",
                    id: "m1-layout-findings-back-to-selection",
                    r#type: "button",
                    onclick: move |_| input.on_findings_return.call(()),
                    "Back to selection"
                }
            }
        }
        if input.outline_inspector.is_none() && input.component_inspector.is_none() {
        if let Some(title) = input.context_title.as_ref() {
            section { class: if board_context_header { "m1-selected-context m1-board-context" } else { "m1-selected-context" }, "aria-label": "Selected context",
                h2 { "{title}" }
                if let Some(detail) = input.context_detail.as_ref() { p { "{detail}" } }
            }
        }
        }
        if matrix_context {
            if let Some(relations) = input.matrix_relations.clone() {
                super::layout_matrix_relations::PlacementBreadcrumb {
                    relations,
                    on_navigate: on_navigate_relation,
                    on_open_owner: on_open_owner_relation,
                }
            }
        }
        {
            if input.outline_inspector.is_none() {
                if let Some(projection) = input.component_inspector {
                    super::inspector::LayoutComponentInspector {
                        projection,
                        pending_edits: input.component_inspector_pending_edits,
                        x: input.component_inspector_x,
                        y: input.component_inspector_y,
                        margin: input.component_inspector_margin,
                        error: input.component_inspector_error,
                        x_failure: input.component_inspector_x_failure,
                        y_failure: input.component_inspector_y_failure,
                        margin_failure: input.component_inspector_margin_failure,
                        layout_pending: input.component_inspector_layout_pending,
                        constraint_pending: input.component_inspector_constraint_pending,
                        remove_pending: input.component_inspector_remove_pending,
                        mirrored_half_pending: input.component_inspector_mirrored_half_pending,
                        on_action: input.on_component_inspector_action,
                    }
                } else if input.show_position_inspector { super::inspector::Inspector {} }
            }
            if let Some(projection) = input.companion_placement {
                super::inspector::CompanionPlacementInspector {
                    projection,
                    pending: input.companion_placement_pending,
                    error: input.companion_placement_error,
                    pending_edits: input.companion_placement_edits,
                    on_apply: input.on_companion_placement_action,
                }
            }
            if let Some(projection) = input.matrix_inspector.projection.clone() {
                objects::MatrixInspector {
                    projection,
                    request_sequence: input.matrix_inspector.request_sequence,
                    editable: input.matrix_inspector.editable,
                    busy: input.matrix_inspector.busy,
                    inspector_mounted: input.matrix_inspector.inspector_mounted,
                    fields: input.matrix_inspector.fields.clone(),
                    pending_fields: input.matrix_inspector.pending_fields.clone(),
                    feedback: input.matrix_inspector.feedback.clone(),
                    on_edit: input.matrix_inspector.on_edit,
                    on_apply_preset: input.matrix_inspector.on_apply_preset,
                    on_delete: input.matrix_inspector.on_delete,
                    on_unlink: input.matrix_inspector.on_unlink,
                    on_duplicate: input.matrix_inspector.on_duplicate,
                }
            }
            if input.key_size.projection.is_some() {
                BoardDisclosure {
                    class: "m1-layout-key-size-section m1-matrix-transform-section", open: true,
                    heading: rsx! { summary { "Key size" } },
                    objects::KeySizeControls { mount: input.key_size }
                }
            }
            if input.matrix_transform_inspector.projection.is_some() {
                objects::MatrixTransformInspector {
                    mount: input.matrix_transform_inspector.clone(),
                    on_pick_splay_origin: input.on_pick_splay_origin,
                    on_pick_anchor: input.on_pick_anchor,
                    anchor_pick_pending: input.anchor_pick_pending,
                    on_cancel_anchor_pick: input.on_cancel_anchor_pick,
                    // Part links select the part key without writing; the
                    // presentation-built handler guards currency the same way
                    // Relations references do.
                    on_navigate_anchor_part: input.on_navigate_anchor_part,
                }
            }
            if let Some(projection) = input.outline_inspector {
                super::outline_lifecycle::OutlineVersionInspector { projection: *projection }
            }
            if input.board_inspector_mount.projection.is_some() {
                super::board_inspector::BoardInspector {
                    mount: input.board_inspector_mount,
                }
            }
            if matrix_context {
                super::layout_matrix_relations::MatrixRelationsPanel {
                    relations: input.matrix_relations.clone(),
                    mount: input.matrix_transform_inspector.clone(),
                    on_navigate: on_navigate_relation,
                    on_open_owner: on_open_owner_relation,
                }
            }
        }
    }
}

#[component]
fn MatrixRelationshipEscape(
    target: Option<objects::TreeSelectRequest>,
    component_owner: Option<super::inspector::LayoutComponentInspectorOwner>,
    on_escape: EventHandler<objects::TreeSelectRequest>,
) -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        use std::{cell::RefCell, rc::Rc};
        use wasm_bindgen::{JsCast, closure::Closure};

        type Listener = Rc<
            RefCell<
                Option<(
                    web_sys::Document,
                    Closure<dyn FnMut(web_sys::KeyboardEvent)>,
                )>,
            >,
        >;

        let listener = use_hook(Listener::default);
        let effect_listener = listener.clone();
        use_effect(use_reactive!(|target, component_owner| {
            let listener = effect_listener.clone();
            if let Some((document, callback)) = listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            }
            let (Some(target), Some(component_owner)) = (target.as_ref(), component_owner.as_ref())
            else {
                return;
            };
            if !matrix_relationship_escape_target_is_current(target, component_owner) {
                return;
            }
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let target = target.clone();
            let component_owner = component_owner.clone();
            let on_escape = on_escape;
            let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|target| {
                        target
                            .closest("input, textarea, select, [contenteditable='true']")
                            .ok()
                            .flatten()
                            .is_some()
                    })
                {
                    return;
                }
                let Some(request) = matrix_relationship_escape_return_request(
                    &target,
                    &component_owner,
                    &event.key(),
                    event.default_prevented(),
                ) else {
                    return;
                };
                event.prevent_default();
                event.stop_propagation();
                on_escape.call(request);
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
            *listener.borrow_mut() = Some((document, callback));
        }));
        use_drop({
            let listener = listener.clone();
            move || {
                if let Some((document, callback)) = listener.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (target, component_owner, on_escape);
    rsx! {}
}

fn matrix_relationship_escape_target_is_current(
    target: &objects::TreeSelectRequest,
    component_owner: &super::inspector::LayoutComponentInspectorOwner,
) -> bool {
    target.scope == component_owner.scope
        && matches!(
            &target.context,
            objects::TreeContext::Component {
                part_id: Some(part_id),
                ..
            } if part_id == &component_owner.part_id
        )
}

fn matrix_relationship_escape_return_request(
    target: &objects::TreeSelectRequest,
    component_owner: &super::inspector::LayoutComponentInspectorOwner,
    key: &str,
    default_prevented: bool,
) -> Option<objects::TreeSelectRequest> {
    if key != "Escape"
        || default_prevented
        || !matrix_relationship_escape_target_is_current(target, component_owner)
    {
        return None;
    }
    Some(objects::TreeSelectRequest {
        scope: target.scope.clone(),
        context: objects::TreeContext::Board {
            board_id: target.scope.board_id.clone(),
        },
        mode: target.mode,
        outline_action: None,
    })
}

#[component]
fn PendingSplayOriginPickEscape(pending: bool, on_cancel: EventHandler<()>) -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        use std::{cell::RefCell, rc::Rc};
        use wasm_bindgen::{JsCast, closure::Closure};

        type Listener = Rc<
            RefCell<
                Option<(
                    web_sys::Document,
                    Closure<dyn FnMut(web_sys::KeyboardEvent)>,
                )>,
            >,
        >;

        let listener = use_hook(Listener::default);
        use_effect(use_reactive((&pending,), {
            let listener = listener.clone();
            move |(pending,)| {
                if let Some((document, callback)) = listener.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        callback.as_ref().unchecked_ref(),
                    );
                }
                if !pending {
                    return;
                }
                let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                    return;
                };
                let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                    if event.key() != "Escape" || event.default_prevented() {
                        return;
                    }
                    if event
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|target| {
                            target
                                .closest("input, textarea, select, [contenteditable='true']")
                                .ok()
                                .flatten()
                                .is_some()
                        })
                    {
                        return;
                    }
                    event.prevent_default();
                    event.stop_propagation();
                    on_cancel.call(());
                }) as Box<dyn FnMut(_)>);
                let _ = document
                    .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
                *listener.borrow_mut() = Some((document, callback));
            }
        }));
        use_drop({
            let listener = listener.clone();
            move || {
                if let Some((document, callback)) = listener.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (pending, on_cancel);

    rsx! {}
}

#[cfg(all(test, target_arch = "wasm32"))]
mod origin_pick_escape_regression_tests {
    use super::*;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn pending_pick_host() -> Element {
        let mut pending = use_signal(|| false);
        rsx! {
            PendingSplayOriginPickEscape {
                pending: pending(),
                on_cancel: EventHandler::new(move |()| pending.set(false)),
            }
            button {
                id: "arm-origin-pick",
                onclick: move |_| pending.set(true),
                "Pick origin"
            }
            output { id: "pending-origin-pick", "{pending()}" }
        }
    }

    #[wasm_bindgen_test]
    async fn escape_from_inspector_cancels_pending_origin_pick() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(pending_pick_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the origin-pick control renders", || {
            root.query_selector("#arm-origin-pick")
                .is_ok_and(|button| button.is_some())
        })
        .await;

        root.query_selector("#arm-origin-pick")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        wait_until("the origin pick becomes pending", || {
            root.query_selector("#pending-origin-pick")
                .ok()
                .flatten()
                .is_some_and(|state| state.text_content().as_deref() == Some("true"))
        })
        .await;
        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        document
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)
                    .unwrap(),
            )
            .unwrap();
        wait_until("Escape cancels the pending origin pick", || {
            root.query_selector("#pending-origin-pick")
                .ok()
                .flatten()
                .is_some_and(|state| state.text_content().as_deref() == Some("false"))
        })
        .await;

        assert_eq!(
            root.query_selector("#pending-origin-pick")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("false"),
            "Escape from the Inspector must clear the pending canvas pick"
        );
        root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod relation_escape_regression_tests {
    use super::*;
    use boardstudio_application::{Scope, SelectionMode, SessionEpoch};
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn route() -> (
        objects::TreeSelectRequest,
        super::super::inspector::LayoutComponentInspectorOwner,
    ) {
        let scope = Scope {
            session_epoch: SessionEpoch(5),
            document_id: "layout-escape".into(),
            board_id: "left".into(),
            instance_id: None,
        };
        let part_id = "left-keys-SW1".to_owned();
        (
            objects::TreeSelectRequest {
                scope: scope.clone(),
                context: objects::TreeContext::Component {
                    part_id: Some(part_id.clone()),
                    matrix_id: Some("left-keys".into()),
                    row: Some(0),
                    column: Some(0),
                    assembly_id: None,
                },
                mode: SelectionMode::Replace,
                outline_action: None,
            },
            super::super::inspector::LayoutComponentInspectorOwner {
                scope,
                snapshot_token: boardstudio_application::SnapshotToken(13),
                revision: 9,
                context_generation: 2,
                scope_generation: 3,
                part_id,
                selected_part_ids: vec!["left-keys-SW1".to_owned()],
            },
        )
    }

    #[component]
    fn relation_escape_host() -> Element {
        let (target, owner) = route();
        let mut context = use_signal(|| "component");
        rsx! {
            MatrixRelationshipEscape {
                target: Some(target),
                component_owner: Some(owner),
                on_escape: EventHandler::new(move |request: objects::TreeSelectRequest| {
                    context.set(if matches!(request.context, objects::TreeContext::Board { .. }) {
                        "board"
                    } else {
                        "unexpected"
                    });
                }),
            }
            output { id: "selection-context", "{context()}" }
        }
    }

    #[wasm_bindgen_test]
    async fn escape_from_matrix_relationship_component_returns_to_board_context() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(relation_escape_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the matrix relationship renders", || {
            root.query_selector("#selection-context")
                .ok()
                .flatten()
                .is_some_and(|context| context.text_content().as_deref() == Some("component"))
        })
        .await;

        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        options.set_bubbles(true);
        document
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)
                    .unwrap(),
            )
            .unwrap();
        wait_until("Escape returns to the board Inspector context", || {
            root.query_selector("#selection-context")
                .ok()
                .flatten()
                .is_some_and(|context| context.text_content().as_deref() == Some("board"))
        })
        .await;

        assert_eq!(
            root.query_selector("#selection-context")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("board"),
            "Escape after Edit placement relationship returns to the Board Inspector context"
        );
        root.remove();
    }
}
