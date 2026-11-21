//! Shared screen-size rail chrome: token glyphs, the rail backdrop and the rail mount hook
//! every transform overlay uses.
use super::*;

/// Registers `tool` on `rail` for this canvas while the overlay stays mounted.
///
/// W3-C: the mount set lives in the canvas-owned [`LayoutTransformSvg`]
/// handle, not in a thread-local registry. Every overlay of one canvas shares
/// the same `Rc`, so simultaneous mounts on one canvas list together while a
/// second canvas (or a remount with a fresh handle) owns a separate set.
/// Dropping the overlay unmounts its tool; dropping the handle retires both rails.
pub(super) fn use_rail_mount(rail: Rail, svg: LayoutTransformSvg, tool: LayoutTransformTool) {
    let svg_for_mount = svg.clone();
    use_hook(move || {
        let handle = svg_for_mount.clone();
        match rail {
            Rail::Matrix => handle.0.matrix_rail.borrow_mut().mount(tool),
            Rail::Part => handle.0.part_rail.borrow_mut().mount(tool),
        }
    });
    // Peers mount in one commit, so the first overlay paints before the rest
    // register. This second pass re-reads the full mount set once every peer
    // of the commit has registered, keeping every rail centred.
    let mut mounted = use_signal(|| false);
    use_effect(move || mounted.set(true));
    let _ = mounted();
    use_drop(move || match rail {
        Rail::Matrix => svg.0.matrix_rail.borrow_mut().unmount(&tool),
        Rail::Part => svg.0.part_rail.borrow_mut().unmount(&tool),
    });
}

/// The live peer on `rail` for this canvas, if a drag or pick holds one.
pub(super) fn rail_active(rail: Rail, svg: &LayoutTransformSvg) -> Option<LayoutTransformTool> {
    match rail {
        Rail::Matrix => svg.0.matrix_rail.borrow().active(),
        Rail::Part => svg.0.part_rail.borrow().active(),
    }
}

/// Mark the live peer on `rail` for this canvas (`None` clears).
pub(super) fn set_rail_active(
    rail: Rail,
    svg: &LayoutTransformSvg,
    tool: Option<LayoutTransformTool>,
) {
    match rail {
        Rail::Matrix => svg.0.matrix_rail.borrow_mut().set_active(tool),
        Rail::Part => svg.0.part_rail.borrow_mut().set_active(tool),
    }
}

/// The mounted peers on `rail` for this canvas, in canonical order.
pub(super) fn rail_peers(rail: Rail, svg: &LayoutTransformSvg) -> Vec<LayoutTransformTool> {
    match rail {
        Rail::Matrix => rail.peers_in(&svg.0.matrix_rail.borrow()),
        Rail::Part => rail.peers_in(&svg.0.part_rail.borrow()),
    }
}

/// World units per screen px times three: the overlays draw screen-size chrome from px art
/// scaled by this factor, so tokens stay 18 px at every zoom.
pub(super) fn handle_scale(svg: &LayoutTransformSvg, width: f64, height: f64) -> f64 {
    svg.surface().borrow().as_ref().map_or(3.0, |surface| {
        let rect = surface.get_bounding_client_rect();
        if rect.width() > 0.0 {
            (width / rect.width()).max(height / rect.height().max(1.0)) * 3.0
        } else {
            3.0
        }
    })
}

/// The matrix rail's peers for a selection. Column 0 follows the pitch, so it has no Spread
/// step to drag: a Spread token never joins its rail. Every overlay of the selection filters
/// the same mount set, so the rail stays centred on the tokens that draw.
pub(super) fn matrix_rail_tools(
    svg: &LayoutTransformSvg,
    context: &TreeContext,
) -> Vec<LayoutTransformTool> {
    let selected_column = context_column(context).unwrap_or(0);
    let selected_row = context_row(context);
    rail_peers(Rail::Matrix, svg)
        .into_iter()
        .filter(|peer| {
            !matches!(peer, LayoutTransformTool::Spread)
                || selected_column != 0
                || selected_row.is_some()
        })
        .collect()
}

/// The rail centred over `points` (world units): horizontally on their extent, just above
/// the topmost one. `None` without points or tools.
pub(super) fn rail_over_points(
    points: &[Vec2],
    tool_count: usize,
    world_per_screen: f64,
) -> Option<handle_rail::WorldRailLayout> {
    if points.is_empty() {
        return None;
    }
    let min_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let top = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    handle_rail::rail_world((min_x + max_x) / 2.0, top, tool_count, world_per_screen)
}

/// The top edge of a part's courtyard in its local frame, or 0 without a courtyard.
pub(super) fn part_courtyard_top(document: &ProjectDoc, definition_id: &str) -> f64 {
    document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id)
        .map(|definition| {
            definition
                .courtyard
                .iter()
                .map(|point| point.y)
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .filter(|top| top.is_finite())
        .unwrap_or(0.0)
}

/// A token's (fill, glyph stroke): primary while its drag or pick is live, the selection
/// colours while hovered or focused, otherwise transparent with a muted glyph.
pub(super) fn token_paint(active: bool, lit: bool) -> (&'static str, &'static str) {
    if active {
        ("var(--primary)", "var(--primary-foreground)")
    } else if lit {
        ("var(--selected)", "var(--selected-foreground)")
    } else {
        ("transparent", "var(--muted-foreground)")
    }
}

/// The rail's card backdrop. The first token of the rail draws it, so it paints once.
pub(super) fn rail_backdrop(
    layout: &handle_rail::WorldRailLayout,
    tool_count: usize,
    px: f64,
) -> Element {
    let rail_w = handle_rail::rail_width(tool_count);
    let rail_cx = layout.rail.x + layout.rail.w / 2.0;
    let rail_cy = layout.rail.y + layout.rail.h / 2.0;
    rsx! {
        g { "aria-hidden": "true",
            transform: "translate({rail_cx} {rail_cy}) scale({px} {-px})",
            style: "pointer-events:none",
            rect {
                x: "{-rail_w / 2.0}",
                y: "-11",
                width: "{rail_w}",
                height: "22",
                rx: "6",
                style: "fill:var(--card);stroke:var(--border-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))",
            }
        }
    }
}

/// A token's face inside its px group: the 18 px tile, the taller hit area, the glyph and
/// the native title.
pub(super) fn token_face(fill: &str, glyph: Element, title: Option<&str>) -> Element {
    rsx! {
        rect { x: "-9", y: "-9", width: "18", height: "18", rx: "4", style: "fill:{fill};stroke:none" }
        rect { x: "-10", y: "-12", width: "20", height: "24", style: "fill:transparent;stroke:none;pointer-events:all" }
        {glyph}
        if let Some(title) = title {
            title { "{title}" }
        }
    }
}

/// The dashed guide drawn through a lit one-dimensional token along its drag axis
/// (`axis`, any length), 17 screen px either side.
pub(super) fn axis_guide(
    (tok_x, tok_y): (f64, f64),
    (axis_x, axis_y): (f64, f64),
    world_per_screen: f64,
    opacity: &str,
) -> Element {
    let axis_len = axis_x.hypot(axis_y).max(1e-9);
    let (guide_dx, guide_dy) = (
        axis_x / axis_len * 17.0 * world_per_screen,
        axis_y / axis_len * 17.0 * world_per_screen,
    );
    rsx! {
        line {
            x1: "{tok_x - guide_dx}",
            y1: "{tok_y - guide_dy}",
            x2: "{tok_x + guide_dx}",
            y2: "{tok_y + guide_dy}",
            class: "pointer-events-none",
            style: "fill:none;stroke:var(--primary);stroke-width:1px;stroke-dasharray:3 3;opacity:{opacity};pointer-events:none;vector-effect:non-scaling-stroke",
        }
    }
}

/// Prototype variant D token glyphs, in screen px. The rail draws them in a
/// y-down px group, so these match the spec sheet verbatim.
pub(super) const GLYPH_SPREAD: &str = "M-5 0h10M-3 -2l-2 2 2 2M3 -2l2 2-2 2";
pub(super) const GLYPH_STAGGER: &str = "M-5 0h10M-3 -2l-2 2 2 2M3 -2l2 2-2 2";
pub(super) const GLYPH_FOUR: &str = "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4";
pub(super) const GLYPH_SPLAY: &str = "M-5 4A9 9 0 0 1 4 -5M1.5 -6.5 4 -5l-1.3 2.4M-5 4h2";
/// Prototype variant D bend token: a curved arrow, shared by row and key.
pub(super) const GLYPH_BEND: &str = "M-4 5C-4 0-1-4 4-4M1.5-6 4-4 1.5-2";
pub(super) const GLYPH_ROTATE: &str = "M4.5 -2A5 5 0 1 1 -2 -4.6M4.6 -5v3h-3";
/// Prototype variant D anchor token: the anchor shank with its ring.
pub(super) const GLYPH_ANCHOR: &str =
    "M0-2.5V5M-3-.5h6M-4.5 2.2a4.5 4.5 0 0 0 9 0M0-5.6a1.5 1.5 0 1 0 .01 0";

/// The token glyph for one rail tool: the axis arrow along `glyph_degrees`
/// for the one-dimensional tools, the spec glyphs for Splay and Rotate, and
/// the four-way arrow for Move and the Ctrl quick offset.
pub(super) fn rail_token_glyph(
    tool: LayoutTransformTool,
    quick_offset: bool,
    glyph_degrees: f64,
    stroke: &str,
) -> Element {
    let style = format!(
        "fill:none;stroke:{stroke};stroke-width:1.5px;stroke-linecap:round;stroke-linejoin:round;vector-effect:non-scaling-stroke"
    );
    match (tool, quick_offset) {
        (LayoutTransformTool::Spread, _) => rsx! {
            path { d: "{GLYPH_SPREAD}", style: "{style}" }
        },
        // A Shift token is always the four-way arrow: the drag is already a
        // free 2-D move, so no Ctrl quick-offset state gates it.
        (LayoutTransformTool::Stagger | LayoutTransformTool::Offset, true)
        | (LayoutTransformTool::Shift | LayoutTransformTool::Move, _) => rsx! {
            g { transform: "scale(1.5)",
                path { d: "{GLYPH_FOUR}", style: "{style}" }
            }
        },
        (LayoutTransformTool::Stagger | LayoutTransformTool::Offset, false) => rsx! {
            g { transform: "rotate({glyph_degrees})",
                path { d: "{GLYPH_STAGGER}", style: "{style}" }
            }
        },
        (LayoutTransformTool::Bend, _) => rsx! {
            path { d: "{GLYPH_BEND}", style: "{style}" }
        },
        (LayoutTransformTool::Splay, _) => rsx! {
            path { d: "{GLYPH_SPLAY}", style: "{style}" }
        },
        (LayoutTransformTool::Rotate, _) => rsx! {
            path { d: "{GLYPH_ROTATE}", style: "{style}" }
        },
        (LayoutTransformTool::Anchor, _) => rsx! {
            path { d: "{GLYPH_ANCHOR}", style: "{style}" }
        },
        (LayoutTransformTool::Origin, _) => rsx! {},
    }
}
