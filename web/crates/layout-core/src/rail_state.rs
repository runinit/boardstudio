//! Pure per-canvas rail mount state for the Layout transform rails.
//!
//! One canvas owns one [`RailState`] per rail (matrix vs part). Overlays
//! mount their tool while they stay mounted and unmount on drop; the active
//! peer tracks the live drag or pick. No Dioxus, no thread-local, no global:
//! the canvas component owns the instance (via its svg handle) so simultaneous
//! mounts stay independent and retirement is deterministic (drop clears).

/// Which tools share one [`RailState`]: the matrix rail and the part rail keep
/// separate mount sets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RailState<T> {
    tools: Vec<T>,
    active: Option<T>,
}

impl<T: Clone + PartialEq> RailState<T> {
    /// An empty rail: no mounted tools, no active peer.
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            active: None,
        }
    }

    /// Register `tool` on this rail. Repeated mounts list once.
    pub fn mount(&mut self, tool: T) {
        if !self.tools.contains(&tool) {
            self.tools.push(tool);
        }
    }

    /// Unregister `tool`. Clearing the active peer when it leaves keeps the
    /// dimming honest; other peers stay.
    pub fn unmount(&mut self, tool: &T) {
        self.tools.retain(|peer| peer != tool);
        if self.active.as_ref() == Some(tool) {
            self.active = None;
        }
    }

    /// Mark the live peer (`None` clears). The canvas sets this on drag/pick
    /// start and clears it on drop, cancel, Escape or owner change.
    pub fn set_active(&mut self, tool: Option<T>) {
        self.active = tool;
    }

    /// The live peer, if any.
    pub fn active(&self) -> Option<T> {
        self.active.clone()
    }

    /// The mounted tools, in mount order.
    pub fn tools(&self) -> &[T] {
        &self.tools
    }

    /// The mounted tools in the rail's canonical `order` (Origin never joins:
    /// it is simply absent from the order).
    pub fn peers_in_order(&self, order: &[T]) -> Vec<T> {
        order
            .iter()
            .filter(|peer| self.tools.contains(peer))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_list_once_in_canonical_order() {
        let order = [1, 2, 3, 4];
        let mut rail = RailState::new();
        for tool in [3, 1, 4, 2] {
            rail.mount(tool);
        }
        rail.mount(1);
        assert_eq!(rail.peers_in_order(&order), vec![1, 2, 3, 4]);
        assert_eq!(rail.active(), None);
    }

    #[test]
    fn unmounting_the_active_tool_clears_it() {
        let mut rail = RailState::new();
        rail.mount(1);
        rail.mount(2);
        rail.set_active(Some(1));
        assert_eq!(rail.active(), Some(1));
        rail.unmount(&2);
        assert_eq!(rail.active(), Some(1), "a peer leaving keeps it");
        rail.unmount(&1);
        assert_eq!(rail.active(), None);
        assert!(rail.peers_in_order(&[1, 2]).is_empty());
    }

    #[test]
    fn rails_keep_separate_sets() {
        let mut matrix = RailState::new();
        let mut part = RailState::new();
        matrix.mount("move");
        part.mount("move");
        part.mount("rotate");
        assert_eq!(matrix.peers_in_order(&["move", "rotate"]), vec!["move"]);
        assert_eq!(
            part.peers_in_order(&["move", "rotate"]),
            vec!["move", "rotate"]
        );
        matrix.set_active(Some("move"));
        assert_eq!(part.active(), None, "active does not leak across rails");
    }
}
