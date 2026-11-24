use super::view::KeymapView;
use boardstudio_model::model::Vec2;
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
    for key in view
        .keys
        .iter()
        .filter(|k| selected_ids.contains(k.id.as_ref()))
    {
        let (sin, cos) = key.pose.rotation.to_radians().sin_cos();
        let (hx, hy) = (key.size.x / 2.0, key.size.y / 2.0);
        for (lx, ly) in [(-hx, -hy), (-hx, hy), (hx, -hy), (hx, hy)] {
            let (x, y) = (
                key.pose.at.x + lx * cos - ly * sin,
                key.pose.at.y + lx * sin + ly * cos,
            );
            bounds = Some(bounds.map_or((x, x, y, y), |(x0, x1, y0, y1)| {
                (x0.min(x), x1.max(x), y0.min(y), y1.max(y))
            }));
        }
    }
    bounds
}

pub fn fit_camera(
    canvas_bounds: (f64, f64, f64, f64),
    target_bounds: (f64, f64, f64, f64),
    surface: (f64, f64),
    toolbar_height: f64,
) -> Option<FitCamera> {
    let vals = [
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
    ];
    if vals.iter().any(|v| !v.is_finite()) || surface.0 <= 0.0 || surface.1 <= 0.0 {
        return None;
    }
    let (cw, ch) = (
        (canvas_bounds.1 - canvas_bounds.0 + 40.0).max(50.0),
        (canvas_bounds.3 - canvas_bounds.2 + 40.0).max(50.0),
    );
    let (tw, th) = (
        (target_bounds.1 - target_bounds.0 + 24.0).max(1.0),
        (target_bounds.3 - target_bounds.2 + 24.0).max(1.0),
    );
    let aspect = surface.0.max(1.0) / surface.1.max(1.0);
    let (vw, vh) = (cw.max(ch * aspect), ch.max(cw / aspect));
    let (top, bottom) = (toolbar_height + 24.0, 24.0);
    let zoom = (vw / tw * (surface.0 - 32.0).max(1.0) / surface.0)
        .min(vh / th * (surface.1 - top - bottom).max(1.0) / surface.1)
        .max(0.01);
    Some(FitCamera {
        center: Vec2 {
            x: (target_bounds.0 + target_bounds.1 - canvas_bounds.0 - canvas_bounds.1) * 0.5,
            y: (target_bounds.2 + target_bounds.3 - canvas_bounds.2 - canvas_bounds.3) * 0.5
                + (top - bottom) / 2.0 * vh / zoom / surface.1,
        },
        zoom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_camera_frames_valid_bounds_and_rejects_degenerate() {
        let fit = fit_camera(
            (-50.0, 50.0, -50.0, 50.0),
            (-9.0, 9.0, -9.0, 9.0),
            (800.0, 600.0),
            48.0,
        )
        .expect("fit");
        assert!(fit.zoom.is_finite() && fit.zoom > 0.0);
        assert!(
            fit_camera(
                (-50.0, 50.0, -50.0, 50.0),
                (-9.0, 9.0, -9.0, 9.0),
                (0.0, 600.0),
                48.0
            )
            .is_none()
        );
    }
}
