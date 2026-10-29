//! Ticket 02: screen-size canvas chrome.
//!
//! A pure projection from screen pixels to canvas (world millimetre) units
//! for the current view: the view-box width the `Scene` draws over the
//! surface's CSS pixel width. Every fixed-size label and ring reads this
//! helper instead of a hard-coded world size, so canvas chrome stays 11 px /
//! 7 px on screen at every zoom.

/// Maps a screen-pixel length to canvas units for the current view scale.
///
/// `view_width_world` is the drawn view-box width in canvas units and
/// `surface_width_px` the surface's CSS pixel width. Degenerate inputs fall
/// back to a finite, non-negative size instead of NaN or an infinity.
pub fn px_to_world(px: f64, view_width_world: f64, surface_width_px: f64) -> f64 {
    let px = if px.is_finite() && px > 0.0 { px } else { 0.0 };
    let view = if view_width_world.is_finite() && view_width_world > 0.0 {
        view_width_world
    } else {
        50.0
    };
    let surface = if surface_width_px.is_finite() && surface_width_px > 0.0 {
        surface_width_px
    } else {
        1.0
    };
    px * view / surface
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eleven_px_maps_to_world_for_the_current_view() {
        // A 200 mm view over 800 px: 11 px of chrome is 2.75 mm of canvas.
        assert!((px_to_world(11.0, 200.0, 800.0) - 2.75).abs() < 1e-9);
    }

    #[test]
    fn doubling_zoom_halves_world_size_so_on_screen_stays_fixed() {
        let wide = px_to_world(11.0, 200.0, 800.0);
        let close = px_to_world(11.0, 100.0, 800.0);
        assert!((close * 2.0 - wide).abs() < 1e-9);
        // On-screen size is world divided by the view scale: 11 px at both.
        assert!((wide / (200.0 / 800.0) - 11.0).abs() < 1e-9);
        assert!((close / (100.0 / 800.0) - 11.0).abs() < 1e-9);
    }

    #[test]
    fn degenerate_inputs_stay_finite() {
        for surface in [0.0, -4.0, f64::NAN, f64::INFINITY] {
            let world = px_to_world(11.0, 200.0, surface);
            assert!(
                world.is_finite() && world >= 0.0,
                "surface {surface:?} must stay finite, got {world:?}"
            );
        }
        for view in [0.0, -10.0, f64::NAN, f64::INFINITY] {
            let world = px_to_world(11.0, view, 800.0);
            assert!(
                world.is_finite() && world >= 0.0,
                "view {view:?} must stay finite, got {world:?}"
            );
        }
    }
}
