use crate::model::{Pose2, Vec2};

/// The pose whose forward map is [`super::pair::inverse`] for `mapping`.
///
/// The map's linear part is its own inverse, so the reverse direction is the same
/// reflection/rotation anchored at the negated image of the mapping translation.
/// It lets callers reuse forward-only helpers such as
/// [`super::pair::reflect_opening`] when deriving the second member back into the
/// first member's frame.
pub fn inverse_pose(mapping: Pose2) -> Pose2 {
    let fixed = super::pair::forward_vector(mapping, mapping.at);
    Pose2 {
        at: Vec2 {
            x: -fixed.x,
            y: -fixed.y,
        },
        rotation: mapping.rotation,
    }
}
