//! The rail mount state: pure logic, compiled natively for tests.
//!
//! Each canvas owns one [`RailState`] per rail (matrix vs part) inside its svg
//! handle. Overlays mount their tool while they stay mounted and unmount on
//! drop; the active peer tracks the live drag or pick. This module holds the
//! rail identity and canonical orders plus the peer filtering; the Dioxus
//! hook that registers against the component-owned handle lives in `chrome`.
use super::tool::LayoutTransformTool;
use boardstudio_web_layout_core::rail_state::RailState;

/// Which rail a token joins. The matrix rail and the part rail keep separate
/// mount sets per svg handle and list their peers in their own canonical order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Rail {
    Matrix,
    Part,
}

/// Canonical matrix rail order: peers list in this order however the mounts
/// interleave, matching the `transform_tools` listing order.
pub(super) const MATRIX_RAIL_ORDER: [LayoutTransformTool; 9] = [
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
pub(super) const PART_RAIL_ORDER: [LayoutTransformTool; 5] = [
    LayoutTransformTool::Move,
    LayoutTransformTool::Rotate,
    LayoutTransformTool::Spread,
    LayoutTransformTool::Stagger,
    LayoutTransformTool::Anchor,
];

impl Rail {
    pub(super) fn order(self) -> &'static [LayoutTransformTool] {
        match self {
            Self::Matrix => &MATRIX_RAIL_ORDER,
            Self::Part => &PART_RAIL_ORDER,
        }
    }

    /// The rail peers for one overlay: the mounted tools in canonical order.
    /// Origin stays a point on the canvas, so it never joins the rail. Members,
    /// companions and locked parts never mount, so a part rail only ever holds one
    /// independent part's tools.
    pub(super) fn peers_in(
        self,
        state: &RailState<LayoutTransformTool>,
    ) -> Vec<LayoutTransformTool> {
        state.peers_in_order(self.order())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LayoutTransformTool::*;

    fn mounted<const N: usize>(
        order: &[LayoutTransformTool],
        tools: [LayoutTransformTool; N],
    ) -> Vec<LayoutTransformTool> {
        let mut state = RailState::new();
        for tool in tools {
            state.mount(tool);
        }
        state.peers_in_order(order)
    }

    #[test]
    fn peers_list_in_canonical_order_however_mounts_interleave() {
        assert_eq!(
            mounted(
                &MATRIX_RAIL_ORDER,
                [Anchor, Rotate, Origin, Spread, Splay, Move, Stagger]
            ),
            [Spread, Stagger, Splay, Move, Rotate, Anchor],
            "Origin never joins the rail and a repeated mount lists once"
        );
        // A repeated mount lists once.
        let mut state = RailState::new();
        state.mount(Spread);
        state.mount(Spread);
        assert_eq!(state.peers_in_order(&MATRIX_RAIL_ORDER), vec![Spread]);
        assert_eq!(
            mounted(&PART_RAIL_ORDER, [Anchor, Stagger, Spread, Rotate, Move]),
            [Move, Rotate, Spread, Stagger, Anchor]
        );
        // Rails keep separate sets: one state per rail, so a matrix tool never
        // leaks into the part peers.
        let mut matrix = RailState::new();
        let mut part = RailState::new();
        for tool in [Anchor, Stagger, Spread, Rotate, Move] {
            part.mount(tool);
        }
        matrix.mount(Move);
        assert_eq!(matrix.peers_in_order(&MATRIX_RAIL_ORDER).len(), 1);
        assert_eq!(part.peers_in_order(&PART_RAIL_ORDER).len(), 5);
        assert!(
            RailState::<LayoutTransformTool>::new()
                .peers_in_order(&PART_RAIL_ORDER)
                .is_empty(),
            "an unmounted svg has no peers"
        );
    }

    #[test]
    fn unmounting_the_active_tool_clears_the_active_peer() {
        for order in [&MATRIX_RAIL_ORDER[..], &PART_RAIL_ORDER[..]] {
            let mut state = RailState::new();
            state.mount(Move);
            state.mount(Rotate);
            assert_eq!(state.active(), None);
            state.set_active(Some(Move));
            assert_eq!(state.active(), Some(Move));
            state.unmount(&Rotate);
            assert_eq!(state.active(), Some(Move), "a peer leaving keeps it");
            state.unmount(&Move);
            assert_eq!(state.active(), None);
            assert!(state.peers_in_order(order).is_empty());
        }
        let mut fresh = RailState::<LayoutTransformTool>::new();
        fresh.set_active(Some(Move));
        assert_eq!(fresh.active(), None);
    }
}
