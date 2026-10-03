//! Reset routed-board discovery state when the accepted reference is absent.
use dioxus::prelude::*;

pub(super) fn clear_missing_reference(
    mut model_paths: Signal<Vec<String>>,
    mut paths_asset_id: Signal<Option<String>>,
    mut attempted_discovery: Signal<Option<String>>,
    mut request_generation: Signal<u64>,
    mut error: Signal<Option<String>>,
) {
    model_paths.set(Vec::new());
    paths_asset_id.set(None);
    attempted_discovery.set(None);
    // This epoch invalidates requests; reading it reactively would subscribe the
    // reset effect to its own write and prevent a missing reference from settling.
    let next_generation = request_generation.peek().wrapping_add(1);
    request_generation.set(next_generation);
    error.set(None);
}
