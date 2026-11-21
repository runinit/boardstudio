//! Pure key routes for the canvas-owned transform key listener.
//!
//! One canvas owns one listener set (document `keydown`/`keyup`, window `blur`)
//! for every transform overlay it draws, instead of one set per overlay. Each
//! overlay subscribes an Escape route or a Control-modifier target while it
//! stays mounted; the canvas keeps its listeners installed while any route
//! exists and removes them when the last one leaves. Escape routes run in
//! subscription order, the order the per-overlay listeners used to fire in.

/// One subscription, returned by `add_*` and handed back to [`CanvasKeyRoutes::remove`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRouteId(u64);

/// The Escape routes (`E`) and Control-modifier targets (`C`) of one canvas.
#[derive(Debug)]
pub struct CanvasKeyRoutes<E, C> {
    next: u64,
    escape: Vec<(KeyRouteId, E)>,
    control: Vec<(KeyRouteId, C)>,
}

impl<E, C> Default for CanvasKeyRoutes<E, C> {
    fn default() -> Self {
        Self {
            next: 0,
            escape: Vec::new(),
            control: Vec::new(),
        }
    }
}

impl<E: Clone, C: Clone> CanvasKeyRoutes<E, C> {
    fn id(&mut self) -> KeyRouteId {
        self.next += 1;
        KeyRouteId(self.next)
    }

    /// Subscribe an Escape route after every earlier one.
    pub fn add_escape(&mut self, route: E) -> KeyRouteId {
        let id = self.id();
        self.escape.push((id, route));
        id
    }

    /// Subscribe a target the Control key's held state reaches.
    pub fn add_control(&mut self, target: C) -> KeyRouteId {
        let id = self.id();
        self.control.push((id, target));
        id
    }

    /// Drop one subscription; the others keep their order.
    pub fn remove(&mut self, id: KeyRouteId) {
        self.escape.retain(|(route, _)| *route != id);
        self.control.retain(|(route, _)| *route != id);
    }

    /// Whether nothing listens, so the canvas can remove its listeners.
    pub fn is_empty(&self) -> bool {
        self.escape.is_empty() && self.control.is_empty()
    }

    /// The Escape routes in subscription order, detached so a route may run
    /// without holding the routes borrowed.
    pub fn escape_routes(&self) -> Vec<E> {
        self.escape.iter().map(|(_, route)| route.clone()).collect()
    }

    /// The Control-modifier targets, detached like [`Self::escape_routes`].
    pub fn control_targets(&self) -> Vec<C> {
        self.control
            .iter()
            .map(|(_, target)| target.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_routes_keep_subscription_order_across_removal() {
        let mut routes = CanvasKeyRoutes::<&str, u8>::default();
        let first = routes.add_escape("matrix");
        let control = routes.add_control(1);
        routes.add_escape("part");
        routes.add_escape("pick");
        assert_eq!(routes.escape_routes(), ["matrix", "part", "pick"]);
        routes.remove(first);
        assert_eq!(routes.escape_routes(), ["part", "pick"]);
        assert_eq!(routes.control_targets(), [1]);
        routes.remove(control);
        assert!(routes.control_targets().is_empty());
        let late = routes.add_escape("matrix");
        assert_eq!(routes.escape_routes(), ["part", "pick", "matrix"]);
        assert_ne!(late, first, "a remount never reuses a retired id");
    }

    #[test]
    fn the_listener_stays_while_any_route_exists() {
        let mut routes = CanvasKeyRoutes::<&str, u8>::default();
        assert!(routes.is_empty(), "a fresh canvas installs nothing");
        let escape = routes.add_escape("matrix");
        let control = routes.add_control(1);
        routes.remove(escape);
        assert!(!routes.is_empty(), "a Control target alone keeps it");
        routes.remove(escape);
        assert!(!routes.is_empty(), "a repeated removal is a no-op");
        routes.remove(control);
        assert!(routes.is_empty(), "the last route leaving removes it");
    }

    #[test]
    fn canvases_keep_separate_routes() {
        let mut first = CanvasKeyRoutes::<&str, u8>::default();
        let mut second = CanvasKeyRoutes::<&str, u8>::default();
        let id = first.add_escape("matrix");
        second.add_escape("part");
        first.remove(id);
        assert!(first.is_empty(), "the first canvas retires its listener");
        assert_eq!(second.escape_routes(), ["part"], "the second keeps its own");
    }
}
