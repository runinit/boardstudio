//! Ticket 03: the rail mount registry. Plain logic: compiled natively for tests.
use super::tool::LayoutTransformTool;
use std::{cell::RefCell, collections::HashMap};

/// Which rail a token joins. Ticket 03's matrix rail and Ticket 07's part rail keep separate
/// mount sets per svg and list their peers in their own canonical order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Rail {
    Matrix,
    Part,
}

/// Ticket 03: the rail peers mounted in one SVG canvas. Each overlay registers
/// its tool on mount and unregisters on unmount, keyed by the shared svg
/// handle, so every overlay of one selection computes the same peer list
/// without new props on the shared mount site. The list holds the mounted
/// tools; `Rail::peers` filters it into canonical order minus Origin.
#[derive(Default)]
struct RailMount {
    tools: Vec<LayoutTransformTool>,
    active: Option<LayoutTransformTool>,
}

thread_local! {
    static RAIL_MOUNTS: RefCell<HashMap<(Rail, usize), RailMount>> = RefCell::new(HashMap::new());
}

/// Canonical matrix rail order: peers list in this order however the mounts
/// interleave, matching the `transform_tools` listing order.
const MATRIX_RAIL_ORDER: [LayoutTransformTool; 9] = [
    LayoutTransformTool::Spread,
    LayoutTransformTool::Stagger,
    LayoutTransformTool::Offset,
    LayoutTransformTool::Shift,
    LayoutTransformTool::Bend,
    LayoutTransformTool::Splay,
    LayoutTransformTool::Move,
    LayoutTransformTool::Rotate,
    LayoutTransformTool::Anchor,
];

/// Canonical part rail order (Move, Rotate, Spread, Stagger), matching the `part_rail`
/// listing order so the drawn tokens match its tool list.
const PART_RAIL_ORDER: [LayoutTransformTool; 5] = [
    LayoutTransformTool::Move,
    LayoutTransformTool::Rotate,
    LayoutTransformTool::Spread,
    LayoutTransformTool::Stagger,
    LayoutTransformTool::Anchor,
];

impl Rail {
    fn order(self) -> &'static [LayoutTransformTool] {
        match self {
            Self::Matrix => &MATRIX_RAIL_ORDER,
            Self::Part => &PART_RAIL_ORDER,
        }
    }

    pub(super) fn mount(self, svg_key: usize, tool: LayoutTransformTool) {
        RAIL_MOUNTS.with(|mounts| {
            let mut mounts = mounts.borrow_mut();
            let mount = mounts.entry((self, svg_key)).or_default();
            if !mount.tools.contains(&tool) {
                mount.tools.push(tool);
            }
        });
    }

    pub(super) fn unmount(self, svg_key: usize, tool: LayoutTransformTool) {
        RAIL_MOUNTS.with(|mounts| {
            if let Some(mount) = mounts.borrow_mut().get_mut(&(self, svg_key)) {
                mount.tools.retain(|peer| *peer != tool);
                if mount.active == Some(tool) {
                    mount.active = None;
                }
            }
        });
    }

    pub(super) fn set_active(self, svg_key: usize, tool: Option<LayoutTransformTool>) {
        RAIL_MOUNTS.with(|mounts| {
            if let Some(mount) = mounts.borrow_mut().get_mut(&(self, svg_key)) {
                mount.active = tool;
            }
        });
    }

    /// The rail peers for one overlay: the mounted tools in canonical order.
    /// Origin stays a point on the canvas, so it never joins the rail. Members,
    /// companions and locked parts never mount, so a part rail only ever holds one
    /// independent part's tools.
    pub(super) fn peers(self, svg_key: usize) -> Vec<LayoutTransformTool> {
        RAIL_MOUNTS.with(|mounts| {
            mounts
                .borrow()
                .get(&(self, svg_key))
                .map_or_else(Vec::new, |mount| {
                    self.order()
                        .iter()
                        .filter(|peer| mount.tools.contains(peer))
                        .copied()
                        .collect()
                })
        })
    }

    pub(super) fn active(self, svg_key: usize) -> Option<LayoutTransformTool> {
        RAIL_MOUNTS.with(|mounts| {
            mounts
                .borrow()
                .get(&(self, svg_key))
                .and_then(|mount| mount.active)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LayoutTransformTool::*;

    #[test]
    fn peers_list_in_canonical_order_however_mounts_interleave() {
        let svg = 0x5101;
        for tool in [Anchor, Rotate, Origin, Spread, Splay, Move, Stagger] {
            Rail::Matrix.mount(svg, tool);
        }
        Rail::Matrix.mount(svg, Spread);
        assert_eq!(
            Rail::Matrix.peers(svg),
            [Spread, Stagger, Splay, Move, Rotate, Anchor],
            "Origin never joins the rail and a repeated mount lists once"
        );
        for tool in [Anchor, Stagger, Spread, Rotate, Move] {
            Rail::Part.mount(svg, tool);
        }
        assert_eq!(
            Rail::Part.peers(svg),
            [Move, Rotate, Spread, Stagger, Anchor]
        );
        assert_eq!(Rail::Matrix.peers(svg).len(), 6, "rails keep separate sets");
        assert!(
            Rail::Part.peers(svg + 1).is_empty(),
            "each svg keeps its own set"
        );
    }

    #[test]
    fn unmounting_the_active_tool_clears_the_active_peer() {
        for rail in [Rail::Matrix, Rail::Part] {
            let svg = 0x5102;
            rail.mount(svg, Move);
            rail.mount(svg, Rotate);
            assert_eq!(rail.active(svg), None);
            rail.set_active(svg, Some(Move));
            assert_eq!(rail.active(svg), Some(Move));
            rail.unmount(svg, Rotate);
            assert_eq!(
                rail.active(svg),
                Some(Move),
                "{rail:?}: a peer leaving keeps it"
            );
            rail.unmount(svg, Move);
            assert_eq!(rail.active(svg), None, "{rail:?}");
            assert!(rail.peers(svg).is_empty(), "{rail:?}");
            rail.set_active(svg + 1, Some(Move));
            assert_eq!(
                rail.active(svg + 1),
                None,
                "an unmounted svg has no active peer"
            );
        }
    }
}
