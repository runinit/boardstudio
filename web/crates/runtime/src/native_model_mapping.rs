//! Admission and mapping completion for the version-triggered Case model delivery
//! callback. A pure state machine over caller-supplied ports, so it runs natively.
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Default)]
pub(crate) struct NativeModelDeliveryState {
    pub(crate) pending: Option<crate::case_preview::CasePreviewOwnerIdentity>,
    pub(crate) failed: Option<crate::case_preview::CasePreviewOwnerIdentity>,
    pub(crate) published: Option<(
        crate::case_preview::CasePreviewOwnerIdentity,
        crate::model_delivery::ModelDeliveryRows,
    )>,
}

/// Admission and mapping completion for the version-triggered Case delivery callback.
pub(crate) async fn resolve_native_model_paths<
    F: std::future::Future<Output = Result<Vec<Option<String>>, String>>,
>(
    state: &RefCell<NativeModelDeliveryState>,
    owner: &crate::case_preview::CasePreviewOwnerIdentity,
    paths: Vec<String>,
    is_current: impl Fn() -> bool,
    changed: impl Fn(),
    resolve: impl FnOnce(Vec<String>) -> F,
) -> Result<Option<BTreeMap<String, Option<String>>>, String> {
    if !is_current() {
        return Ok(None);
    }
    {
        let mut state = state.borrow_mut();
        if state
            .published
            .as_ref()
            .is_some_and(|(published, _)| published == owner)
            || state.pending.as_ref() == Some(owner)
            || state.failed.as_ref() == Some(owner)
        {
            return Ok(None);
        }
        state.published = None;
        state.failed = None;
        state.pending = Some(owner.clone());
    }
    changed();
    let result = resolve(paths.clone()).await.and_then(|ids| {
        if ids.len() == paths.len() {
            Ok(ids)
        } else {
            Err("Core returned an incomplete model path mapping".into())
        }
    });
    // Mapping imports may outlive the preview lease or a replacement request.
    // Stale completion must neither retire the replacement nor escape to global status.
    if !is_current() || state.borrow().pending.as_ref() != Some(owner) {
        let mut state = state.borrow_mut();
        if state.pending.as_ref() == Some(owner) {
            state.pending = None;
        }
        return Ok(None);
    }
    match result {
        Ok(ids) => Ok(Some(paths.into_iter().zip(ids).collect())),
        Err(error) => {
            {
                let mut state = state.borrow_mut();
                state.pending = None;
                // Runtime::report also notifies the version subscriber. Retain this exact
                // failure before notifying so it cannot automatically admit itself again.
                state.failed = Some(owner.clone());
            }
            changed();
            Err(error)
        }
    }
}
