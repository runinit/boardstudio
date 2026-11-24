//! Outline planning vocabulary and pure edit settlement shared with the Layout crate.

pub mod edit;
pub mod planner;

pub use edit::{
    Admission, OutlineDrawTool, OutlineFeedback, OutlineVersionChoice, OutlineVersionOwnership,
    active_outline, admission, blocked_export_labels, editable_perimeter, gap_focus_camera,
    outline_action_matches_saved_owner, outline_edit, outline_state, outline_version,
    retire_reason, settle, target_ids, version_geometry,
};
pub use planner::{
    EditablePerimeter, OutlineAction, OutlineActionEnvelope, OutlineActionKind, OutlinePointTarget,
    PerimeterAnchor, attach_connection_point, connection_point_world, move_connection_point,
};
