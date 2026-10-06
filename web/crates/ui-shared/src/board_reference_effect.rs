//! Reset routed-board discovery state when the accepted reference is absent.
use dioxus::prelude::*;

/// Keep stale routed-board failures actionable without carrying a specific
/// error from an old project/board/editor scope into the current one.
pub fn retryable_error(message: String, owner_lineage_is_current: bool) -> String {
    if owner_lineage_is_current {
        message
    } else {
        "The project, board, or editor changed while the routed-board operation was in progress. Its result was discarded; retry from the current board.".into()
    }
}

pub fn clear_missing_reference(
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

#[cfg(test)]
mod tests {
    use super::retryable_error;

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn current_failure_keeps_specific_reason_and_stale_failure_is_retryable() {
        assert_eq!(
            retryable_error("Saved routed-board file is missing".into(), true),
            "Saved routed-board file is missing"
        );
        let stale = retryable_error("old-board parse failure".into(), false);
        assert!(stale.contains("result was discarded"));
        assert!(stale.contains("retry from the current board"));
        assert!(!stale.contains("old-board"));
    }
}
