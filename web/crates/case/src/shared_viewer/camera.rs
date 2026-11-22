pub(super) fn wheel_zoom(delta: f64) -> f64 {
    (delta.signum() * (delta.abs() * 0.001).min(1.0)).exp()
}

pub(super) fn crossed_drag_threshold(start: (f64, f64), point: (f64, f64)) -> bool {
    (point.0 - start.0).abs() + (point.1 - start.1).abs() > 4.0
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn wheel_zoom_is_reversible_and_caps_large_deltas() {
        assert_eq!(wheel_zoom(0.0), 1.0);
        assert_eq!(wheel_zoom(1000.0), wheel_zoom(5000.0));
        assert!((wheel_zoom(250.0) * wheel_zoom(-250.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn click_tolerance_is_measured_from_pointer_down_in_css_pixels() {
        assert!(!crossed_drag_threshold((12.0, 20.0), (14.0, 18.0)));
        assert!(crossed_drag_threshold((12.0, 20.0), (14.0, 17.9)));
    }
}
