use crate::model::Vec2;
pub fn rotate(point: Vec2, degrees: f64, pivot: Vec2) -> Vec2 {
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let x = point.x - pivot.x;
    let y = point.y - pivot.y;
    Vec2 {
        x: pivot.x + x * cos - y * sin,
        y: pivot.y + x * sin + y * cos,
    }
}
