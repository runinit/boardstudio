use boardstudio_core::model::Vec2;

pub(super) fn bridge_camera_offset(viewport_bounds: (f64, f64, f64, f64), target: Vec2) -> Vec2 {
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

#[cfg(test)]
mod tests {
    use super::bridge_camera_offset;
    use boardstudio_core::model::Vec2;

    #[test]
    fn selected_bridge_camera_uses_a_viewport_relative_pan_delta() {
        let viewport_bounds = (0.0, 280.0, -80.0, 0.0);
        let bridge_center = Vec2 { x: 270.0, y: -30.0 };

        let camera_offset = bridge_camera_offset(viewport_bounds, bridge_center);

        assert!((camera_offset.x - 130.0).abs() < 1e-9);
        assert!((camera_offset.y - 10.0).abs() < 1e-9);
    }
}
