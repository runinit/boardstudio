//! Transform-property navigation for the Layout command pill and the canvas rail overlays.
//!
//! `tool`, `rail` and `placement` hold the plain tool, rail mount state and placement logic (unit
//! tested natively); `chrome` and `feedback` hold the rail token, popup and toast chrome and
//! the hooks every overlay shares; `matrix_tools`, `part_tools` and `anchor_pick` are the
//! overlays, with their gesture logic in `matrix_gesture` and `part_drag`; `menu` holds the
//! contextual toolbar and the Transform menu.
use super::layout_toolbar::LayoutSelectionKind;
use super::layout_toolbar::{GestureSnapInputs, LayoutCommandMenu, close_layout_command_menu};
use super::{ScopedTreeContext, TreeContext};
use crate::anchor_pick::{
    AnchorMajorKind, AnchorPickCandidates, CANDIDATE_HIT_PX, CANDIDATE_RING_HOVER_PX,
    CANDIDATE_RING_PX, MAJOR_RING_PX, PickTarget, anchor_key_label, anchor_pick_candidates,
    matrix_display_name, pick_commit, preview_anchor_poses,
};
use crate::matrix_relations;
use crate::matrix_transform_operation::{
    ArrowKey, MatrixDragAnchor, PlacementParameter, PlacementValue, SceneBasis, column_angle,
    gesture_value, handle_rail, matrix_drag_anchor_pose, matrix_drag_transport, nudge_value,
    placement_operation, solve_matrix_origin, solve_matrix_rotation,
};
use crate::presentation::{
    LayoutOwnerIdentity,
    canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution,
};
use boardstudio_core_kernel::matrix::layout::evaluated_part_pose;
use boardstudio_core_kernel::matrix::layout::layout_half_offset;
use boardstudio_model::model::{
    EditOperation, Matrix, MatrixScene, MatrixSplayAffect, MatrixSplayChange, Mirror, PartAnchor,
    PartAnchorValueChange, Pose2, Position, ProjectDoc, Vec2,
};
use boardstudio_web_layout_core::rail_state::RailState;
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_model::transform_feedback::{
    FeedbackDelta, FeedbackEvent, FeedbackState, advance_feedback,
};
use boardstudio_web_ui_shared::ui::{
    callout::Callout,
    dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger},
    kbd::Kbd,
    toolbar::{Toolbar, ToolbarButton},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::SvgElement;

mod anchor_pick;
mod chrome;
mod commit;
mod feedback;
mod matrix_gesture;
mod matrix_tools;
mod menu;
mod part_drag;
mod part_tools;
mod placement;
mod rail;
mod tool;

pub use anchor_pick::{AnchorPickOwner, LayoutAnchorPickOverlay};
use chrome::*;
use commit::{
    PendingTransformCommit, PendingTransformToast, begin_transform_ticket, raise_commit_toast,
    transform_commit_is_live, transform_version, use_settled_transform_commits,
};
use feedback::*;
use matrix_gesture::*;
pub use matrix_tools::LayoutTransformToolOverlay;
pub use menu::{LayoutContextActionToolbar, LayoutTransformToolbar};
use part_drag::*;
pub use part_tools::LayoutPartToolOverlay;
use placement::*;
use rail::*;
use tool::*;
pub use tool::{LayoutTransformTool, ToolActivation, activate_transform_tool};

#[cfg(all(test, target_arch = "wasm32"))]
mod tests;

#[derive(Clone, PartialEq)]
pub struct LayoutTransformMenuMount {
    pub properties_available: bool,
    pub column_available: bool,
    pub row_available: bool,
    pub pointer_tools_visible: bool,
    pub pointer_tools_available: bool,
    pub active_tool: Option<LayoutTransformTool>,
    /// `None` when the selection is not a Matrix; otherwise whether the
    /// matrix follows rotation, which decides whether the Rotate entry is
    /// offered enabled or disabled with an explanation.
    pub matrix_rotate_followed: Option<bool>,
    pub on_selection_kind: EventHandler<LayoutSelectionKind>,
    pub on_show_properties: EventHandler<()>,
    pub on_transform_tool: EventHandler<LayoutTransformTool>,
}

#[derive(Clone)]
pub struct LayoutTransformRuntime(pub Rc<Runtime>);

impl PartialEq for LayoutTransformRuntime {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The shared svg handle plus the component-owned rail mount sets for one canvas.
///
/// W3-C: the matrix rail and the part rail each own one [`RailState`] inside
/// the svg handle the canvas component owns. Sibling overlays of one canvas
/// share the same `Rc`, so every overlay of one selection reads the same
/// mount set; a second canvas (or a remount) owns a second handle with its
/// own sets. Dropping the handle retires both rails; dropping one overlay
/// unmounts only its tool.
pub struct LayoutSvgInner {
    pub surface: Rc<RefCell<Option<SvgElement>>>,
    pub matrix_rail: RefCell<RailState<LayoutTransformTool>>,
    pub part_rail: RefCell<RailState<LayoutTransformTool>>,
}

#[derive(Clone)]
pub struct LayoutTransformSvg(pub Rc<LayoutSvgInner>);

impl LayoutTransformSvg {
    /// Wrap the canvas-owned surface handle with fresh, empty rail sets.
    pub fn new(surface: Rc<RefCell<Option<SvgElement>>>) -> Self {
        Self(Rc::new(LayoutSvgInner {
            surface,
            matrix_rail: RefCell::new(RailState::new()),
            part_rail: RefCell::new(RailState::new()),
        }))
    }

    /// The canvas surface handle the overlays draw through.
    pub fn surface(&self) -> &Rc<RefCell<Option<SvgElement>>> {
        &self.0.surface
    }
}

impl PartialEq for LayoutTransformSvg {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
