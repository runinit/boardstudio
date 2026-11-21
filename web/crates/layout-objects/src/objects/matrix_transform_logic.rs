//! Host-compatible workspace-route predicate for the matrix transform inspector.
//!
//! The production controller (wasm) delegates to this module so the same
//! predicate runs under `cargo test` without a browser.

pub(super) fn workspace_route_matches(current: &str, owner: &str) -> bool {
    matches!(owner, "Layout" | "PCB") && current == owner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_transform_owner_cannot_cross_workspace_routes() {
        assert!(workspace_route_matches("Layout", "Layout"));
        assert!(workspace_route_matches("PCB", "PCB"));
        assert!(!workspace_route_matches("PCB", "Layout"));
        assert!(!workspace_route_matches("Layout", "PCB"));
        assert!(!workspace_route_matches("Case", "Case"));
    }
}
