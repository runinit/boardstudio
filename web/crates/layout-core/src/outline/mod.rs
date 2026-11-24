//! Outline planning vocabulary and pure edit evaluation shared with the Layout crate.

pub mod edit;
pub mod planner;

pub use edit::{
    OutlineDrawTool, OutlineFeedback, OutlineVersionChoice, OutlineVersionOwnership,
    active_outline, blocked_export_labels, editable_perimeter, evaluate_outline_edit,
    gap_focus_camera, outline_action_matches_saved_owner, outline_edit, outline_state,
    outline_version, target_ids, version_geometry,
};
pub use planner::{
    EditablePerimeter, OutlineAction, OutlineActionEnvelope, OutlineActionKind, OutlinePointTarget,
    PerimeterAnchor, attach_connection_point, connection_point_world, move_connection_point,
};
