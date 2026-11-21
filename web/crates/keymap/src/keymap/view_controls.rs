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
    let usable_width = (surface.0 - 32.0).max(1.0);
    let usable_height = (surface.1 - top - bottom).max(1.0);
    let zoom = (visible_width / target_width * usable_width / surface.0)
        .min(visible_height / target_height * usable_height / surface.1)
        .max(0.01);

    Some(FitCamera {
        center: Vec2 {
            x: (target_bounds.0 + target_bounds.1 - canvas_bounds.0 - canvas_bounds.1) * 0.5,
            y: (target_bounds.2 + target_bounds.3 - canvas_bounds.2 - canvas_bounds.3) * 0.5
                + (top - bottom) / 2.0 * visible_height / zoom / surface.1,
        },
        zoom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn key(id: &str, reference: &str, x: f64, y: f64) -> crate::keymap::view::KeymapKey {
        crate::keymap::view::KeymapKey {
            id: Rc::from(id),
            reference: Rc::from(reference),
            pose: boardstudio_model::model::Pose2 {
                at: Vec2 { x, y },
                rotation: 0.0,
            },
            binding_title: Rc::from("A"),
            color: Rc::from("#fff"),
            size: Vec2 { x: 18.0, y: 18.0 },
            search_index: format!("{reference} a"),
        }
    }

    fn view() -> crate::keymap::view::KeymapView {
        crate::keymap::view::KeymapView {
            layers: vec![],
            keys: vec![key("a", "SW1", 0.0, 0.0), key("b", "SW2", 19.05, 0.0)],
        }
    }

    #[test]
    fn selected_bounds_cover_the_chosen_keys_only() {
        let view = view();
        let one: BTreeSet<String> = ["a".into()].iter().cloned().collect();
        let bounds = selected_bounds(&view, &one).expect("bounds");
        assert!((bounds.0 - -9.0).abs() < 1e-6);
        assert!((bounds.1 - 9.0).abs() < 1e-6);
        let both: BTreeSet<String> = ["a".into(), "b".into()].iter().cloned().collect();
        let wide = selected_bounds(&view, &both).expect("bounds");
        assert!(wide.1 - wide.0 > bounds.1 - bounds.0);
        let missing: BTreeSet<String> = ["ghost".into()].iter().cloned().collect();
        assert!(selected_bounds(&view, &missing).is_none());
    }

    #[test]
    fn fit_camera_rejects_degenerate_inputs_and_frames_the_target() {
        let canvas = (-50.0, 50.0, -50.0, 50.0);
        let target = (-9.0, 9.0, -9.0, 9.0);
        let fitted = fit_camera(canvas, target, (800.0, 600.0), 48.0).expect("camera");
        assert!(fitted.zoom.is_finite() && fitted.zoom > 0.0);
        assert!(fitted.center.x.is_finite() && fitted.center.y.is_finite());
        assert!(fit_camera(canvas, target, (0.0, 600.0), 48.0).is_none());
        assert!(fit_camera((f64::NAN, 50.0, -50.0, 50.0), target, (800.0, 600.0), 48.0).is_none());
    }
}
