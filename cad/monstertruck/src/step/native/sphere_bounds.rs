//! Bound the projected source trim independently of the tessellation.
use super::{super::bounds, *};

pub(super) struct ArcExtent {
    pub(super) offset: Vector3<f64>,
    pub(super) x: Vector3<f64>,
    pub(super) y: Vector3<f64>,
    pub(super) start: f64,
    pub(super) end: f64,
}

impl ArcExtent {
    fn minimum_dot(&self, axis: Vector3<f64>) -> f64 {
        let (a, b) = (axis.dot(self.x), axis.dot(self.y));
        [self.start, self.end]
            .into_iter()
            .chain(bounds::ellipse_extrema(a, b, self.start, self.end))
            .map(|t| axis.dot(self.offset) + a * t.cos() + b * t.sin())
            .fold(f64::INFINITY, f64::min)
    }
}

pub(super) fn enclosing_planes(
    hemisphere: Vector3<f64>,
    inward: [Vector3<f64>; 3],
    arcs: &[ArcExtent],
    corners: [Vector3<f64>; 3],
    epsilon: f64,
) -> Result<[Vector3<f64>; 3], String> {
    let minimum = |axis| {
        arcs.iter()
            .map(|arc| arc.minimum_dot(axis))
            .chain(corners.iter().map(|p| axis.dot(*p)))
            .fold(f64::INFINITY, f64::min)
    };
    let chart_depth = minimum(hemisphere) - epsilon;
    if !chart_depth.is_finite() || chart_depth <= 0.0 {
        return Err("native spherical boundary leaves its hemisphere".into());
    }
    // Tilt each trim plane outward just enough to enclose all boundary rays.
    // In the hemisphere's gnomonic chart this is an affine half-plane.
    // A half-plane containing the boundary also contains its bounded interior.
    Ok(inward.map(|side| side + hemisphere * ((epsilon - minimum(side)).max(0.0) / chart_depth)))
}
