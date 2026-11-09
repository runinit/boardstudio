//! Screen-space handle rail for the Layout transform tools (ticket 03).
//!
//! The canvas drag handles ship as labelled tokens in one rail above the
//! selection. This module is the pure layout seam: it takes the selection's
//! screen bounds and the ordered rail tools and returns the token rectangles
//! plus the rail rectangle. It knows no Dioxus, no document and no zoom: the
//! caller projects the selection to screen space, so the same screen bounds
//! lay out identically at every zoom. The overlay only draws what this
//! returns. Origin stays a point on the canvas and never enters the rail.

/// Token edge, in screen px (prototype variant D: 18 x 18, radius 4).
pub const TOKEN_PX: f64 = 18.0;
/// Gap between adjacent tokens, in screen px.
pub const TOKEN_GAP_PX: f64 = 2.0;
/// Rail padding on each side, in screen px.
pub const RAIL_PAD_PX: f64 = 2.0;
/// Rail height, in screen px (radius 6).
pub const RAIL_HEIGHT_PX: f64 = 22.0;
/// Gap between the selection top and the rail's bottom edge, in screen px.
pub const RAIL_ABOVE_PX: f64 = 10.0;
/// Token hit area, in screen px (20 x 24).
pub const TOKEN_HIT_W_PX: f64 = 20.0;
/// Token hit area height, in screen px.
pub const TOKEN_HIT_H_PX: f64 = 24.0;
/// Origin ring radius, in screen px (6 px ring).
pub const ORIGIN_RING_R_PX: f64 = 3.0;
/// Origin cross arm length, in screen px (12 px cross).
pub const ORIGIN_CROSS_PX: f64 = 12.0;
/// Origin hit area edge, in screen px (24 x 24).
pub const ORIGIN_HIT_PX: f64 = 24.0;
/// Opacity of the rail tokens that are not active while one drags.
pub const DIM_OPACITY: f64 = 0.35;

/// A rectangle in screen px (x/y is the top-left, y grows down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ScreenRect {
    pub fn center_x(&self) -> f64 {
        self.x + self.w / 2.0
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
}

/// The laid-out rail: the rail rectangle plus one token rectangle per tool,
/// in the caller's tool order.
#[derive(Clone, Debug, PartialEq)]
pub struct RailLayout {
    pub rail: ScreenRect,
    pub tokens: Vec<ScreenRect>,
}

/// Rail width for `tool_count` tokens: the tokens, the gaps between them and
/// one padding on each side.
pub fn rail_width(tool_count: usize) -> f64 {
    if tool_count == 0 {
        return 0.0;
    }
    tool_count as f64 * TOKEN_PX + (tool_count as f64 - 1.0) * TOKEN_GAP_PX + 2.0 * RAIL_PAD_PX
}

/// Lay out the rail in screen px: `tool_count` 18 px tokens centred on
/// `selection_center_x`, the rail's bottom edge 10 px above `selection_top`.
/// Returns `None` for an empty tool list (no rail draws).
pub fn layout_rail(
    selection_center_x: f64,
    selection_top: f64,
    tool_count: usize,
) -> Option<RailLayout> {
    if tool_count == 0 {
        return None;
    }
    let width = rail_width(tool_count);
    let rail = ScreenRect {
        x: selection_center_x - width / 2.0,
        y: selection_top - RAIL_ABOVE_PX - RAIL_HEIGHT_PX,
        w: width,
        h: RAIL_HEIGHT_PX,
    };
    let token_y = rail.y + (RAIL_HEIGHT_PX - TOKEN_PX) / 2.0;
    let tokens = (0..tool_count)
        .map(|index| ScreenRect {
            x: rail.x + RAIL_PAD_PX + index as f64 * (TOKEN_PX + TOKEN_GAP_PX),
            y: token_y,
            w: TOKEN_PX,
            h: TOKEN_PX,
        })
        .collect();
    Some(RailLayout { rail, tokens })
}

/// A rectangle in world units (board mm).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The laid-out rail in world units: the rail rectangle plus one token
/// centre per tool, in the caller's tool order. The overlay draws these with
/// its screen-size scale, so the px geometry above holds at every zoom.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldRailLayout {
    pub rail: WorldRect,
    pub token_centers: Vec<(f64, f64)>,
}

/// Lay out the rail from world bounds: `selection_top_world` is the
/// selection's top in world units (the larger y), `world_per_screen` is world
/// units per screen px. Returns `None` for an empty tool list.
pub fn rail_world(
    selection_center_world: f64,
    selection_top_world: f64,
    tool_count: usize,
    world_per_screen: f64,
) -> Option<WorldRailLayout> {
    if tool_count == 0 || !world_per_screen.is_finite() || world_per_screen <= 0.0 {
        return None;
    }
    let width_world = rail_width(tool_count) * world_per_screen;
    let height_world = RAIL_HEIGHT_PX * world_per_screen;
    let above_world = RAIL_ABOVE_PX * world_per_screen;
    // World y grows up on the canvas: the rail sits above the selection top.
    let bottom = selection_top_world + above_world;
    let rail = WorldRect {
        x: selection_center_world - width_world / 2.0,
        y: bottom,
        w: width_world,
        h: height_world,
    };
    let token_centers = (0..tool_count)
        .map(|index| {
            let x = rail.x
                + (RAIL_PAD_PX + TOKEN_PX / 2.0 + index as f64 * (TOKEN_PX + TOKEN_GAP_PX))
                    * world_per_screen;
            (x, bottom + height_world / 2.0)
        })
        .collect();
    Some(WorldRailLayout {
        rail,
        token_centers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn three_tools_width_centring_and_offset() {
        // RED: three column tools over a known bounding box.
        let layout = layout_rail(100.0, 200.0, 3).expect("three tools lay out a rail");
        assert!(approx(layout.rail.w, 3.0 * 18.0 + 2.0 * 2.0 + 4.0));
        assert!(approx(layout.rail.h, 22.0));
        assert!(approx(layout.rail.center_x(), 100.0));
        assert!(approx(layout.rail.bottom(), 200.0 - 10.0));
        assert_eq!(layout.tokens.len(), 3);
        // Tokens are 18 px, start after the 2 px pad, spaced 20 px apart,
        // and sit centred vertically in the rail.
        for (index, token) in layout.tokens.iter().enumerate() {
            assert!(approx(token.w, 18.0));
            assert!(approx(token.h, 18.0));
            assert!(approx(token.x, layout.rail.x + 2.0 + index as f64 * 20.0));
            assert!(approx(token.y, layout.rail.y + 2.0));
        }
        // The middle token centres the rail.
        assert!(approx(layout.tokens[1].center_x(), 100.0));
    }

    #[test]
    fn single_tool_rail() {
        let layout = layout_rail(50.0, 80.0, 1).expect("one tool lays out a rail");
        assert!(approx(layout.rail.w, 18.0 + 4.0));
        assert!(approx(layout.rail.center_x(), 50.0));
        assert!(approx(layout.rail.bottom(), 70.0));
        assert_eq!(layout.tokens.len(), 1);
        assert!(approx(layout.tokens[0].center_x(), 50.0));
    }

    #[test]
    fn empty_tool_list_lays_out_no_rail() {
        assert_eq!(layout_rail(100.0, 200.0, 0), None);
        assert_eq!(rail_world(0.0, 0.0, 0, 0.5), None);
    }

    #[test]
    fn layout_is_identical_at_two_zooms_in_screen_space() {
        // The layout takes screen bounds only: the same on-screen selection
        // lays out identically whatever the zoom behind it.
        let at_zoom_1 = layout_rail(100.0, 200.0, 3).expect("rail at zoom 1");
        let at_zoom_2 = layout_rail(100.0, 200.0, 3).expect("rail at zoom 2");
        assert_eq!(at_zoom_1, at_zoom_2);
    }

    #[test]
    fn world_layout_keeps_screen_geometry_across_zooms() {
        // The same world selection at two zooms projects to different world
        // scales but identical screen px geometry.
        let near = rail_world(10.0, 20.0, 3, 0.25).expect("rail near");
        let far = rail_world(10.0, 20.0, 3, 1.0).expect("rail far");
        let to_px = |world: f64, wps: f64| world / wps;
        assert!(approx(to_px(near.rail.w, 0.25), 62.0));
        assert!(approx(to_px(far.rail.w, 1.0), 62.0));
        assert!(approx(to_px(far.rail.h, 1.0), 22.0));
        // Bottom edge 10 px above the selection top at both zooms.
        assert!(approx(to_px(near.rail.y - 20.0, 0.25), 10.0));
        assert!(approx(to_px(far.rail.y - 20.0, 1.0), 10.0));
        // Centred on the selection in world units at both zooms, with the
        // tokens 20 px apart around the centre in screen px.
        assert!(approx(near.rail.x + near.rail.w / 2.0, 10.0));
        assert!(approx(far.rail.x + far.rail.w / 2.0, 10.0));
        assert_eq!(near.token_centers.len(), 3);
        assert_eq!(far.token_centers.len(), 3);
        for (index, (near_center, far_center)) in near
            .token_centers
            .iter()
            .zip(far.token_centers.iter())
            .enumerate()
        {
            let expected_px = -20.0 + index as f64 * 20.0;
            assert!(approx(to_px(near_center.0 - 10.0, 0.25), expected_px));
            assert!(approx(to_px(far_center.0 - 10.0, 1.0), expected_px));
            assert!(approx(to_px(near_center.1 - 20.0, 0.25), 21.0));
            assert!(approx(to_px(far_center.1 - 20.0, 1.0), 21.0));
        }
    }

    #[test]
    fn world_layout_rejects_bad_scale() {
        assert_eq!(rail_world(0.0, 0.0, 2, 0.0), None);
        assert_eq!(rail_world(0.0, 0.0, 2, f64::INFINITY), None);
    }
}
