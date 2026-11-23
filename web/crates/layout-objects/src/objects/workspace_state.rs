//! Layout selection, command and view state shared by the workspace's surfaces.
use super::{layout_rail_tool::LayoutTransformTool, layout_selection_kind::LayoutSelectionKind};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::MatrixSplayAffect;
use boardstudio_web_ui_model::{
    Workspace,
    split_focus::SplitHalf,
    state::LayoutSnapSettings,
    tree::{ScopedTreeContext, TreeContext},
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub type TransformToolOwner = (Option<Scope>, u64, Option<String>, Workspace, bool);
type InspectorFieldFailures = (
    Signal<Option<String>>,
    Signal<Option<String>>,
    Signal<Option<String>>,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutCommandMenu {
    Select,
    Transform,
    Align,
    Snap,
}

/// The selection a transform edit or armed canvas pick belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixTransformInspectorOwner {
    pub scope: Scope,
    pub matrix_id: String,
    pub context: TreeContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairOwner {
    pub editor_instance_id: u64,
    pub open_id: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairCreated {
    pub owner: MirroredPairOwner,
    pub result_token: SnapshotToken,
    pub result_revision: u64,
    pub scope: Scope,
    pub left_layout_id: String,
    pub right_layout_id: String,
    pub left_matrix_id: String,
    pub right_matrix_id: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutInspectorTab {
    #[default]
    Properties,
    Relations,
}

/// Layout's selection, command, transform, and view state. Editor retains the
/// cross-workspace owner and supplies the current selected tree context.
#[derive(Clone, Copy)]
pub struct LayoutWorkspaceState {
    selection_kind: Signal<LayoutSelectionKind>,
    snap_settings: Signal<LayoutSnapSettings>,
    inspector_tab: Signal<LayoutInspectorTab>,
    command_menu: Signal<Option<LayoutCommandMenu>>,
    transform_tool: Signal<Option<LayoutTransformTool>>,
    transform_tool_owner: Signal<Option<TransformToolOwner>>,
    transform_handles: Signal<bool>,
    /// Split editing focus starts on the source half and survives workspace
    /// switches and spacing changes without editing the document.
    split_focus: Signal<SplitHalf>,
    assembly_3d: Signal<bool>,
    splay_affect: Signal<MatrixSplayAffect>,
    pending_splay_origin_pick: Signal<Option<MatrixTransformInspectorOwner>>,
    pending_anchor_pick: Signal<Option<MatrixTransformInspectorOwner>>,
    pair_created_selection: Signal<Option<MirroredPairCreated>>,
    component_inspector_x: Signal<String>,
    component_inspector_y: Signal<String>,
    component_inspector_margin: Signal<String>,
    component_inspector_error: Signal<Option<String>>,
    component_inspector_x_failure: Signal<Option<String>>,
    component_inspector_y_failure: Signal<Option<String>>,
    component_inspector_margin_failure: Signal<Option<String>>,
    companion_placement_error: Signal<Option<String>>,
    add_object_menu_open: Signal<bool>,
    /// The Align toolbar's reference part. Owned here at session lifetime so
    /// `use_canvas_align` can run inside `LayoutWorkspace` (which unmounts on
    /// workspace switches) without losing the choice.
    selected_reference: Signal<Option<String>>,
}

pub fn use_layout_workspace_state(
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<Workspace>,
) -> LayoutWorkspaceState {
    let mut state = LayoutWorkspaceState {
        selection_kind: use_signal(LayoutSelectionKind::default),
        snap_settings: use_signal(LayoutSnapSettings::default),
        inspector_tab: use_signal(LayoutInspectorTab::default),
        command_menu: use_signal(|| None),
        transform_tool: use_signal(|| None),
        transform_tool_owner: use_signal(|| None),
        transform_handles: use_signal(|| true),
        split_focus: use_signal(SplitHalf::default),
        assembly_3d: use_signal(|| false),
        splay_affect: use_signal(|| MatrixSplayAffect::Following),
        pending_splay_origin_pick: use_signal(|| None),
        pending_anchor_pick: use_signal(|| None),
        pair_created_selection: use_signal(|| None),
        component_inspector_x: use_signal(String::new),
        component_inspector_y: use_signal(String::new),
        component_inspector_margin: use_signal(String::new),
        component_inspector_error: use_signal(|| None),
        component_inspector_x_failure: use_signal(|| None),
        component_inspector_y_failure: use_signal(|| None),
        component_inspector_margin_failure: use_signal(|| None),
        companion_placement_error: use_signal(|| None),
        add_object_menu_open: use_signal(|| false),
        selected_reference: use_signal(|| None),
    };
    // Reset the tab only when the selection changes, not on unrelated revisions.
    let observed_context = selected_context();
    let previous_context = use_hook(|| Rc::new(RefCell::new(observed_context.clone())));
    use_effect(use_reactive!(|observed_context| {
        if previous_context.borrow().as_ref() != observed_context.as_ref() {
            state.inspector_tab.set(LayoutInspectorTab::Properties);
        }
        *previous_context.borrow_mut() = observed_context.clone();
    }));
    let observed_workspace = workspace();
    use_effect(use_reactive!(|observed_workspace| {
        if observed_workspace != Workspace::Layout {
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
    pub fn add_object_menu_open(self) -> Signal<bool> {
        self.add_object_menu_open
    }
    pub fn selected_reference(self) -> Signal<Option<String>> {
        self.selected_reference
    }
    pub fn selection_kind(self) -> Signal<LayoutSelectionKind> {
        self.selection_kind
    }
    pub fn snap_settings(self) -> Signal<LayoutSnapSettings> {
        self.snap_settings
    }
    pub fn inspector_tab(self) -> Signal<LayoutInspectorTab> {
        self.inspector_tab
    }
    pub fn command_menu(self) -> Signal<Option<LayoutCommandMenu>> {
        self.command_menu
    }
    pub fn transform_tool(self) -> Signal<Option<LayoutTransformTool>> {
        self.transform_tool
    }
    pub fn transform_handles(self) -> Signal<bool> {
        self.transform_handles
    }
    pub fn split_focus(self) -> Signal<SplitHalf> {
        self.split_focus
    }
    pub fn assembly_3d(self) -> Signal<bool> {
        self.assembly_3d
    }
    pub fn splay_affect(self) -> Signal<MatrixSplayAffect> {
        self.splay_affect
    }
    pub fn pending_splay_origin_pick(self) -> Signal<Option<MatrixTransformInspectorOwner>> {
        self.pending_splay_origin_pick
    }
    pub fn pending_anchor_pick(self) -> Signal<Option<MatrixTransformInspectorOwner>> {
        self.pending_anchor_pick
    }
    pub fn pair_created_selection(self) -> Signal<Option<MirroredPairCreated>> {
        self.pair_created_selection
    }

    pub fn component_inspector_drafts(
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
    pub fn companion_placement_error(self) -> Signal<Option<String>> {
        self.companion_placement_error
    }

    pub fn component_inspector_field_failures(self) -> InspectorFieldFailures {
        (
            self.component_inspector_x_failure,
            self.component_inspector_y_failure,
            self.component_inspector_margin_failure,
        )
    }

    pub fn set_selection_kind_from_context(mut self, context: &TreeContext) {
        if let Some(kind) = selection_kind_for_tree_context(context) {
            self.selection_kind.set(kind);
        }
    }

    pub fn set_assembly_3d(mut self, assembly_3d: bool) {
        self.assembly_3d.set(assembly_3d);
        if assembly_3d {
            self.pending_splay_origin_pick.set(None);
            self.pending_anchor_pick.set(None);
        }
    }

    pub fn observe_transform_tool_owner(self, owner: TransformToolOwner) {
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

fn selection_kind_for_tree_context(context: &TreeContext) -> Option<LayoutSelectionKind> {
    match context {
        TreeContext::Matrix { .. } => Some(LayoutSelectionKind::Matrix),
        TreeContext::Row { .. } => Some(LayoutSelectionKind::Row),
        TreeContext::Column { .. } => Some(LayoutSelectionKind::Column),
        TreeContext::Key { .. } => Some(LayoutSelectionKind::Key),
        TreeContext::Component { .. } => Some(LayoutSelectionKind::Part),
        TreeContext::Outline { .. }
        | TreeContext::OutlineVersion { .. }
        | TreeContext::Bridge { .. }
        | TreeContext::MountedModule { .. }
        | TreeContext::Board { .. }
        | TreeContext::LayoutGroup { .. } => None,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
