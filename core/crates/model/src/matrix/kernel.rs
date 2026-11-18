use crate::model::Vec2;
/// An evaluated world frame, never a raw input anchor. Axes retain mirror handedness.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposedFrame {
    pub origin: Vec2,
    pub rotation: f64,
    pub axis_x: Vec2,
    pub axis_y: Vec2,
}
impl ComposedFrame {
    /// Invert a displacement through the evaluated, mirror-aware orthonormal axes.
    pub fn inverse_vector(&self, delta: Vec2) -> Vec2 {
        Vec2 {
            x: delta.x * self.axis_x.x + delta.y * self.axis_x.y,
            y: delta.x * self.axis_y.x + delta.y * self.axis_y.y,
        }
    }
}
