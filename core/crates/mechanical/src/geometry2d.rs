//! Shared 2D polygon, bounds and clearance primitives for mechanical resolution.
//!
//! The gasket, occupancy and part-geometry passes previously carried their own
//! copies of these vector and contour conversions. They live here so every pass
//! shares one implementation.
//!
//! Pinned floating-point contract: every helper preserves the exact evaluation
//! order it was extracted from (`hypot` for distances, `p.x * cos - p.y * sin`
//! for rotations, the bounds fast-path before overlay). Do not reorder sums,
//! products or `hypot` calls; poses and clearances are hashed downstream.

use crate::geometry::{self, BooleanOverlayRule};
use crate::model::*;

pub(crate) type FloatPath = Vec<[f64; 2]>;

pub(crate) fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2 {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

pub(crate) fn scale(a: Vec2, s: f64) -> Vec2 {
    Vec2 {
        x: a.x * s,
        y: a.y * s,
    }
}

pub(crate) fn distance(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

pub(crate) fn paths(contours: &[Contour]) -> Vec<FloatPath> {
    contours
        .iter()
        .map(|c| c.points.iter().map(|p| [p.x, p.y]).collect())
        .collect()
}

pub(crate) fn overlaps(a: &[Vec2], b: &[Vec2]) -> bool {
    let bounds = |points: &[Vec2]| {
        points.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)],
        )
    };
    let x = bounds(a);
    let y = bounds(b);
    if x[0] >= y[2] || x[2] <= y[0] || x[1] >= y[3] || x[3] <= y[1] {
        return false;
    }
    let a = a.iter().map(|p| [p.x, p.y]).collect::<FloatPath>();
    let b = b.iter().map(|p| [p.x, p.y]).collect::<FloatPath>();
    !geometry::overlay(&a, &b, BooleanOverlayRule::Intersect).is_empty()
}

/// Rebuilds overlay output shapes as contours, marking every ring after the
/// first in each shape as a hole.
pub(crate) fn shapes_to_contours(shapes: Vec<Vec<FloatPath>>) -> Vec<Contour> {
    shapes
        .into_iter()
        .flat_map(|shape| {
            shape
                .into_iter()
                .enumerate()
                .map(|(index, points)| Contour {
                    hole: index > 0,
                    points: points
                        .into_iter()
                        .map(|point| Vec2 {
                            x: point[0],
                            y: point[1],
                        })
                        .collect(),
                })
        })
        .collect()
}

/// Rotates one point by a precomputed `(sin, cos)` pair.
pub(crate) fn rotate_point(p: Vec2, sin: f64, cos: f64) -> Vec2 {
    Vec2 {
        x: p.x * cos - p.y * sin,
        y: p.x * sin + p.y * cos,
    }
}

/// Rotates a polygon by a precomputed `(sin, cos)` pair.
pub(crate) fn rotate_polygon(points: &[Vec2], sin: f64, cos: f64) -> Vec<Vec2> {
    points.iter().map(|p| rotate_point(*p, sin, cos)).collect()
}

/// Whether a polygon has at least three finite vertices.
pub(crate) fn is_finite_polygon(points: &[Vec2]) -> bool {
    points.len() >= 3 && points.iter().all(|p| p.x.is_finite() && p.y.is_finite())
}
