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

/// Cursor anchor for a wheel-zoom tick: the pointer location the zoom keeps
/// fixed. Plain data (no web-sys) so burst coalescing stays unit-testable on
/// every target; the page shell converts to/from `PointerLocation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelAnchor {
    pub world: Vec2,
    pub x_fraction: f64,
    pub y_fraction: f64,
}

/// One coalesced wheel-zoom burst: the summed deltas plus the latest cursor
/// anchor. Applied once via `wheel_zoom` + `zoom_center_for_anchor`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelZoomBatch {
    pub total_delta_y: f64,
    pub anchor: WheelAnchor,
    pub samples: u32,
}

/// Accumulates the wheel ticks of one frame into a single zoom application.
/// The wheel handler pushes every tick and schedules one rAF flush while
/// `push` reports one is already pending; the flush takes the batch and
/// applies it once, so a burst of N ticks costs one `Runtime::submit`.
#[derive(Debug, Default)]
pub struct WheelZoomCoalescer {
    pending_delta_y: f64,
    samples: u32,
    anchor: Option<WheelAnchor>,
    flush_scheduled: bool,
}

impl WheelZoomCoalescer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one tick. Returns true when the caller must schedule a flush;
    /// false means a flush is already pending (or the delta carried nothing).
    /// Non-finite deltas never poison the sum; the anchor still tracks the
    /// latest cursor position.
    pub fn push(&mut self, delta_y: f64, anchor: WheelAnchor) -> bool {
        self.anchor = Some(anchor);
        if delta_y.is_finite() {
            self.pending_delta_y += delta_y;
            self.samples += 1;
        }
        if self.samples == 0 || self.flush_scheduled {
            return false;
        }
        self.flush_scheduled = true;
        true
    }

    /// Take the pending burst for a single zoom application. `None` means no
    /// flush was needed. Always clears the scheduled flag, even when empty.
    pub fn take_batch(&mut self) -> Option<WheelZoomBatch> {
        self.flush_scheduled = false;
        if self.samples == 0 {
            return None;
        }
        let batch = WheelZoomBatch {
            total_delta_y: self.pending_delta_y,
            anchor: self.anchor?,
            samples: self.samples,
        };
        self.pending_delta_y = 0.0;
        self.samples = 0;
        self.anchor = None;
        Some(batch)
    }

    pub fn pending_samples(&self) -> u32 {
        self.samples
    }

    pub fn is_flush_scheduled(&self) -> bool {
        self.flush_scheduled
    }
}

/// Cursor-anchored zoom center for `zoom`. Same math as the TypeScript
/// Workbench anchor; `CanvasNavigationState::zoom_center_at` delegates here
/// so there is exactly one copy of the formula.
pub fn zoom_center_for_anchor(
    (min_x, max_x, min_y, max_y): (f64, f64, f64, f64),
    anchor: WheelAnchor,
    zoom: f64,
) -> Vec2 {
    let base_width = (max_x - min_x).max(50.0);
    let base_height = (max_y - min_y).max(50.0);
    Vec2 {
        x: anchor.world.x
            - (min_x + max_x - base_width / zoom) * 0.5
            - anchor.x_fraction * base_width / zoom,
        y: anchor.world.y - (min_y + max_y + base_height / zoom) * 0.5
            + anchor.y_fraction * base_height / zoom,
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

/// Native regression tests for Issue 2 (laggy wheel zoom): a burst of wheel
/// ticks inside one frame must coalesce into a single zoom application whose
/// final zoom matches sequential application within float tolerance.
#[cfg(test)]
mod wheel_coalescing_tests {
    use super::{WheelAnchor, WheelZoomCoalescer, wheel_zoom, zoom_center_for_anchor};
    use boardstudio_core::model::Vec2;

    const BOUNDS: (f64, f64, f64, f64) = (-100.0, 100.0, -80.0, 80.0);
    const RATIO: f64 = 1.0;
    const OLD_ZOOM: f64 = 1.0;

    fn anchor_at(x: f64, y: f64) -> WheelAnchor {
        WheelAnchor {
            world: Vec2 { x, y },
            x_fraction: 0.5,
            y_fraction: 0.5,
        }
    }

    fn sequential_zoom(deltas: &[f64]) -> f64 {
        let mut zoom = OLD_ZOOM;
        for delta in deltas {
            zoom = wheel_zoom(zoom, RATIO, *delta);
        }
        zoom
    }

    #[test]
    fn burst_coalesces_into_a_single_zoom_application_with_the_sequential_zoom() {
        let deltas = [-120.0, -80.0, 45.0, -30.0, 15.0];
        let mut burst = WheelZoomCoalescer::new();
        for delta in deltas {
            burst.push(delta, anchor_at(10.0, -5.0));
        }
        assert_eq!(burst.pending_samples(), deltas.len() as u32);
        let batch = burst.take_batch().expect("burst holds one pending batch");
        assert_eq!(batch.samples as usize, deltas.len());
        // One flush consumes the burst: nothing left for a second application.
        assert!(burst.take_batch().is_none());
        let expected = sequential_zoom(&deltas);
        let coalesced = wheel_zoom(OLD_ZOOM, RATIO, batch.total_delta_y);
        assert!(
            (coalesced - expected).abs() < 1e-9,
            "coalesced {coalesced} != sequential {expected}"
        );
        let expected_center = zoom_center_for_anchor(BOUNDS, anchor_at(10.0, -5.0), expected);
        let coalesced_center = zoom_center_for_anchor(BOUNDS, batch.anchor, coalesced);
        assert!((coalesced_center.x - expected_center.x).abs() < 1e-9);
        assert!((coalesced_center.y - expected_center.y).abs() < 1e-9);
    }

    #[test]
    fn only_the_first_push_of_a_burst_schedules_a_flush() {
        let mut burst = WheelZoomCoalescer::new();
        assert!(burst.push(-120.0, anchor_at(0.0, 0.0)));
        assert!(!burst.push(-80.0, anchor_at(0.0, 0.0)));
        assert!(!burst.push(45.0, anchor_at(0.0, 0.0)));
        assert!(burst.is_flush_scheduled());
        burst.take_batch();
        assert!(!burst.is_flush_scheduled());
        assert!(burst.push(45.0, anchor_at(0.0, 0.0)));
    }

    #[test]
    fn the_latest_cursor_anchor_wins_the_burst() {
        let mut burst = WheelZoomCoalescer::new();
        burst.push(-120.0, anchor_at(1.0, 2.0));
        burst.push(-80.0, anchor_at(7.0, -3.0));
        let batch = burst.take_batch().expect("burst holds one pending batch");
        assert_eq!(batch.anchor, anchor_at(7.0, -3.0));
        assert_eq!(batch.total_delta_y, -200.0);
    }

    #[test]
    fn coalesced_burst_keeps_the_pinned_zoom_clamp() {
        let mut burst = WheelZoomCoalescer::new();
        for _ in 0..32 {
            burst.push(-400.0, anchor_at(0.0, 0.0));
        }
        let batch = burst.take_batch().expect("burst holds one pending batch");
        let coalesced = wheel_zoom(OLD_ZOOM, RATIO, batch.total_delta_y);
        assert_eq!(coalesced, super::MAX_ZOOM);
        assert_eq!(sequential_zoom(&[-400.0; 32]), super::MAX_ZOOM);
    }

    #[test]
    fn non_finite_deltas_do_not_poison_the_burst() {
        let mut burst = WheelZoomCoalescer::new();
        assert!(!burst.push(f64::NAN, anchor_at(0.0, 0.0)));
        assert_eq!(burst.pending_samples(), 0);
        assert!(burst.take_batch().is_none());
        assert!(burst.push(-120.0, anchor_at(0.0, 0.0)));
        assert!(!burst.push(f64::INFINITY, anchor_at(1.0, 1.0)));
        let batch = burst.take_batch().expect("burst holds one pending batch");
        assert_eq!(batch.total_delta_y, -120.0);
        assert_eq!(batch.samples, 1);
    }
}
