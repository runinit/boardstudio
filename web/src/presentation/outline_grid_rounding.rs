//! Grid rounding follows the pinned React editor's `Math.round` tie policy.
use boardstudio_core::model::Vec2;

fn javascript_round(value: f64) -> f64 {
    let rounded = (value + 0.5).floor();
    if rounded == 0.0 && value.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

pub(super) fn rounded(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub(super) fn snap_to_grid(point: Vec2, grid: Vec2) -> Vec2 {
    Vec2 {
        x: if grid.x > 0.0 {
            rounded(javascript_round(point.x / grid.x) * grid.x)
        } else {
            point.x
        },
        y: if grid.y > 0.0 {
            rounded(javascript_round(point.y / grid.y) * grid.y)
        } else {
            point.y
        },
    }
}

#[cfg(test)]
mod tests {
    use super::snap_to_grid;
    use boardstudio_core::model::Vec2;

    #[test]
    fn negative_half_grid_matches_javascript_math_round() {
        let snapped = snap_to_grid(Vec2 { x: -0.5, y: -1.5 }, Vec2 { x: 1.0, y: 1.0 });

        assert_eq!(snapped.x, 0.0);
        assert!(snapped.x.is_sign_negative());
        assert_eq!(snapped.y, -1.0);
    }
}
