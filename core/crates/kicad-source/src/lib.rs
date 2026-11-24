#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! KiCad source parsing for BoardStudio Core: s-expression access, board and footprint
//! preview projection, and exact mechanical geometry extraction.

pub use boardstudio_model::model;

#[allow(unused_imports)]
pub(crate) use boardstudio_core_kernel::{hash, matrix};
use model::Vec2;

pub mod footprint_geometry;
pub mod mechanical_extract;
pub mod parsed_board;
pub mod preview;
pub mod sexpr;

// Legacy KiCad uses start=center and end=radius point, with a Y-down sweep.
pub(crate) fn legacy_arc_parameters(
    start: Vec2,
    end: Vec2,
    degrees: f64,
) -> Option<(Vec2, f64, f64, f64)> {
    let radius = (end.x - start.x).hypot(end.y - start.y);
    if radius <= 0.0 {
        return None;
    }
    let first = (end.y - start.y).atan2(end.x - start.x);
    Some((start, first, -degrees.to_radians(), radius))
}
