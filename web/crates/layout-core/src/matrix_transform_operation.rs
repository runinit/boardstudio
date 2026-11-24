//! Private, pure mapping from accepted matrix transform fields to existing edit operations.

//! Ticket 03: the handle-rail layout lives here (instead of beside its
//! overlay) so the pure geometry stays natively tested without touching the
//! shared crate root, which ticket 02 owns this round.
#[path = "handle_rail.rs"]
pub mod handle_rail;

mod build;
mod catalogue;
mod drag;
mod fields;
mod key_omit;
mod placement;
#[cfg(test)]
mod tests;

pub use build::{build_operation, set_cells_enabled};
pub use catalogue::{
    attachment_component_choices, catalogue_retry_due, component_edit_admitted_by_catalogue,
};
pub use drag::{
    MatrixDragAnchor, MatrixDragTransport, matrix_drag_anchor_pose, matrix_drag_transport,
    solve_matrix_origin, solve_matrix_rotation,
};
pub use fields::{
    KeyHalfException, MatrixTransformField, MatrixTransformFields, MatrixTransformValue,
    column_angle,
};
pub use key_omit::{
    key_half_exception, key_half_exception_at_resolver, key_half_exception_omitted,
    key_half_exception_resolver,
};
pub use placement::{
    ArrowKey, BEND_DRAG_DEGREES_PER_MM, BEND_DRAG_SNAP_DEGREES, PlacementParameter, PlacementSet,
    PlacementValue, SceneBasis, convert_offset_operation, gesture_value, nudge_value,
    placement_operation, placement_value,
};
