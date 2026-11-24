//! Shared SVG `points` formatting for 2D board and footprint previews.
//!
//! Several presenters flatten `Vec2` outlines into the space-separated `x,y`
//! pairs an SVG `polygon` takes; they share this one helper so the formatting
//! cannot drift between workspaces.
use boardstudio_model::model::Vec2;

/// Format `points` as the space-separated `x,y` pairs an SVG `polygon` takes.
pub fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}
