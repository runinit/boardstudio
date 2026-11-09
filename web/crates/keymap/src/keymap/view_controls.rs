use super::view::KeymapView;
use boardstudio_core::model::Vec2;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FitCamera {
    pub center: Vec2,
    pub zoom: f64,
}

pub fn selected_bounds(
    view: &KeymapView,
    selected_ids: &BTreeSet<String>,
) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    let mut include = |x: f64, y: f64| {
        bounds = Some(bounds.map_or((x, x, y, y), |(min_x, max_x, min_y, max_y)| {
            (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
        }));
    };
    for key in view
        .keys
        .iter()
        .filter(|key| selected_ids.contains(key.id.as_ref()))
    {
        let angle = key.pose.rotation.to_radians();
        let (sin, cos) = angle.sin_cos();
        for (local_x, local_y) in [
            (-key.size.x / 2.0, -key.size.y / 2.0),
            (-key.size.x / 2.0, key.size.y / 2.0),
            (key.size.x / 2.0, -key.size.y / 2.0),
            (key.size.x / 2.0, key.size.y / 2.0),
        ] {
            include(
                key.pose.at.x + local_x * cos - local_y * sin,
                key.pose.at.y + local_x * sin + local_y * cos,
            );
        }
    }
    bounds
}

pub fn fit_camera(
    canvas_bounds: (f64, f64, f64, f64),
    target_bounds: (f64, f64, f64, f64),
    surface: (f64, f64),
    toolbar_height: f64,
    left_inset: f64,
) -> Option<FitCamera> {
    let values = [
        canvas_bounds.0,
        canvas_bounds.1,
        canvas_bounds.2,
        canvas_bounds.3,
        target_bounds.0,
        target_bounds.1,
        target_bounds.2,
        target_bounds.3,
        surface.0,
        surface.1,
        toolbar_height,
        left_inset,
    ];
    if values.iter().any(|value| !value.is_finite()) || surface.0 <= 0.0 || surface.1 <= 0.0 {
        return None;
    }

    // The mounted Keymap SVG starts from the raw board bounds with 20 mm of
    // padding on each side. Model those actual viewBox dimensions here. React's
    // getBounds adds 12 mm to the target geometry before fitCamera is called.
    let canvas_width = (canvas_bounds.1 - canvas_bounds.0 + 40.0).max(50.0);
    let canvas_height = (canvas_bounds.3 - canvas_bounds.2 + 40.0).max(50.0);
    let target_width = (target_bounds.1 - target_bounds.0 + 24.0).max(1.0);
    let target_height = (target_bounds.3 - target_bounds.2 + 24.0).max(1.0);
    let aspect = surface.0.max(1.0) / surface.1.max(1.0);
    // preserveAspectRatio="xMidYMid meet" expands the visible base extent on
    // the constrained axis, just as React's aspectBounds does.
    let visible_width = canvas_width.max(canvas_height * aspect);
    let visible_height = canvas_height.max(canvas_width / aspect);
    let top = toolbar_height + 24.0;
    let bottom = 24.0;
    let usable_width = (surface.0 - left_inset - 32.0).max(1.0);
    let usable_height = (surface.1 - top - bottom).max(1.0);
    let zoom = (visible_width / target_width * usable_width / surface.0)
        .min(visible_height / target_height * usable_height / surface.1)
        .max(0.01);

    Some(FitCamera {
        center: Vec2 {
            // Increasing viewBox X moves content left; the left overlay needs the opposite.
            x: (target_bounds.0 + target_bounds.1 - canvas_bounds.0 - canvas_bounds.1) * 0.5
                - left_inset / 2.0 * visible_width / zoom / surface.0,
            y: (target_bounds.2 + target_bounds.3 - canvas_bounds.2 - canvas_bounds.3) * 0.5
                + (top - bottom) / 2.0 * visible_height / zoom / surface.1,
        },
        zoom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_camera_preserves_unobstructed_camera() {
        // Pin the pre-guide camera contract, including the toolbar's screen-space shift.
        let camera = fit_camera(
            (0.0, 60.0, 0.0, 20.0),
            (10.0, 86.0, 10.0, 46.0),
            (1000.0, 600.0),
            60.0,
            0.0,
        )
        .unwrap();
        assert_eq!(camera.zoom, 0.82);
        assert_eq!(camera.center.x, 18.0);
        assert_eq!(camera.center.y, 21.658536585365855);
    }

    #[test]
    fn fit_camera_reserves_left_guide_and_moves_content_right() {
        let camera = fit_camera(
            (0.0, 60.0, 0.0, 20.0),
            (10.0, 86.0, 10.0, 46.0),
            (1000.0, 600.0),
            60.0,
            300.0,
        )
        .unwrap();
        // Base extent 100x60, padded target 100x60. Width now limits zoom:
        // (1000 - 300 - 32)/1000 = .668. The free center is at screen x=650,
        // so subtract 150px * (100/.668)/1000 from the world center (18).
        assert_eq!(camera.zoom, 0.668);
        assert!((camera.center.x - -4.45508982035928).abs() < 1e-12);
        assert!((camera.center.y - 22.491017964071856).abs() < 1e-12);
        // The padded target starts at 316px; geometry has another 12mm of margin.
        let left_px = 500.0 + (10.0 - 30.0 - camera.center.x) * camera.zoom * 10.0;
        assert!((left_px - 396.16).abs() < 1e-10);
    }
}
