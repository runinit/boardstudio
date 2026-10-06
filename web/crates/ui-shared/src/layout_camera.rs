use boardstudio_core::model::Vec2;

/// 2D canvas zoom limits shared by wheel and Zoom buttons. These are the pinned
/// TypeScript Workbench limits (`Math.min(4, Math.max(0.25, ...))`).
pub const MIN_ZOOM: f64 = 0.25;
pub const MAX_ZOOM: f64 = 4.0;

/// Wheel zoom: `basis_ratio` converts the stored camera zoom into the displayed
/// (reference) scale, which is the scale the limits apply to.
pub fn wheel_zoom(current: f64, basis_ratio: f64, delta_y: f64) -> f64 {
    (current * basis_ratio * (-delta_y * 0.001).exp()).clamp(MIN_ZOOM, MAX_ZOOM) / basis_ratio
}

/// One Zoom button step on the displayed scale; `direction < 0` zooms out.
pub fn step_zoom(effective: f64, direction: f64) -> f64 {
    if direction < 0.0 {
        (effective / 1.2).max(MIN_ZOOM)
    } else {
        (effective * 1.2).min(MAX_ZOOM)
    }
}

pub fn bridge_camera_offset(viewport_bounds: (f64, f64, f64, f64), target: Vec2) -> Vec2 {
    // The Layout view box is already centered on the visible-part bounds; camera.center is
    // a pan delta from that origin, not an absolute world-space target.
    let viewport_center = Vec2 {
        x: (viewport_bounds.0 + viewport_bounds.1) * 0.5,
        y: (viewport_bounds.2 + viewport_bounds.3) * 0.5,
    };
    Vec2 {
        x: target.x - viewport_center.x,
        y: target.y - viewport_center.y,
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::{MAX_ZOOM, MIN_ZOOM, bridge_camera_offset, step_zoom, wheel_zoom};
    use boardstudio_core::model::Vec2;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn two_d_zoom_limits_match_the_pinned_typescript_workbench() {
        // Keep wheel zoom within the supported scale range,
        // Zoom out `Math.max(.25, zoom / 1.2)`, Zoom in `Math.min(4, zoom * 1.2)`.
        assert_eq!((MIN_ZOOM, MAX_ZOOM), (0.25, 4.0));
    }

    #[wasm_bindgen_test]
    fn wheel_zoom_is_clamped_and_does_not_move_past_a_limit() {
        // In range: exp(-deltaY / 1000) scaling.
        let zoomed = wheel_zoom(1.0, 1.0, -400.0);
        assert!((zoomed - (0.4f64).exp()).abs() < 1e-12);
        // Far beyond either limit clamps to it.
        assert_eq!(wheel_zoom(1.0, 1.0, -100_000.0), 4.0);
        assert_eq!(wheel_zoom(1.0, 1.0, 100_000.0), 0.25);
        // Already at a limit: further wheel in that direction leaves zoom unchanged.
        assert_eq!(wheel_zoom(MAX_ZOOM, 1.0, -400.0), MAX_ZOOM);
        assert_eq!(wheel_zoom(MIN_ZOOM, 1.0, 400.0), MIN_ZOOM);
        // The opposite direction still moves off the limit.
        assert!(wheel_zoom(MAX_ZOOM, 1.0, 400.0) < MAX_ZOOM);
        assert!(wheel_zoom(MIN_ZOOM, 1.0, -400.0) > MIN_ZOOM);
        // A non-unit basis ratio limits the displayed scale, not the stored zoom.
        let ratio = 2.0;
        assert_eq!(wheel_zoom(1.5, ratio, -100_000.0) * ratio, MAX_ZOOM);
        assert_eq!(wheel_zoom(0.1, ratio, 100_000.0) * ratio, MIN_ZOOM);
    }

    #[wasm_bindgen_test]
    fn zoom_buttons_stop_at_the_same_limits() {
        assert_eq!(step_zoom(MAX_ZOOM, 1.0), MAX_ZOOM);
        assert_eq!(step_zoom(MIN_ZOOM, -1.0), MIN_ZOOM);
        assert_eq!(step_zoom(3.9, 1.0), MAX_ZOOM);
        assert_eq!(step_zoom(0.26, -1.0), MIN_ZOOM);
        assert!((step_zoom(1.0, 1.0) - 1.2).abs() < 1e-12);
        assert!((step_zoom(1.0, -1.0) - 1.0 / 1.2).abs() < 1e-12);
    }

    #[wasm_bindgen_test]
    fn selected_bridge_camera_uses_a_viewport_relative_pan_delta() {
        let viewport_bounds = (0.0, 280.0, -80.0, 0.0);
        let bridge_center = Vec2 { x: 270.0, y: -30.0 };

        let camera_offset = bridge_camera_offset(viewport_bounds, bridge_center);

        assert!((camera_offset.x - 130.0).abs() < 1e-9);
        assert!((camera_offset.y - 10.0).abs() < 1e-9);
    }
}
