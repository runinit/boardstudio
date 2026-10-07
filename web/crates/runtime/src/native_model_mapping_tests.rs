use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn owner(token: u64) -> crate::case_preview::CasePreviewOwnerIdentity {
    crate::case_preview::CasePreviewOwnerIdentity {
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "project".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        snapshot_token: SnapshotToken(token),
        accepted_revision: token,
        accepted_scene_identity: 1,
        viewer_instance: 1,
        projection_generation: token,
        batch_generation: token,
        core_executor_epoch: 1,
        core_worker_identity: 1,
        request_token: format!("preview-{token}"),
    }
}

#[wasm_bindgen_test]
async fn failed_mapping_notifier_does_not_readmit_the_same_preview() {
    for malformed in [false, true] {
        let state = RefCell::new(NativeModelDeliveryState::default());
        let version = Cell::new(1);
        let starts = Cell::new(0);
        let mut observed = 0;
        let mut reports = 0;
        // Drive the actual production mapping callback on every Runtime notification.
        // Bound the driver so the original loop fails deterministically rather than hanging.
        for _ in 0..8 {
            if observed == version.get() {
                break;
            }
            observed = version.get();
            let result = resolve_native_model_paths(
                &state,
                &owner(1),
                vec!["model.step".into()],
                || true,
                || version.set(version.get() + 1),
                |_| {
                    starts.set(starts.get() + 1);
                    async move {
                        if malformed {
                            Ok(vec![])
                        } else {
                            Err("module import rejected".into())
                        }
                    }
                },
            )
            .await;
            if result.is_err() {
                reports += 1;
                // Runtime::report also notifies the version subscriber.
                version.set(version.get() + 1);
            }
        }
        assert_eq!(
            starts.get(),
            1,
            "a failed preview is terminal until its owner changes"
        );
        assert_eq!(reports, 1, "one current failure is reported once");
    }
}

#[wasm_bindgen_test]
async fn delayed_old_owner_mapping_failure_cannot_report_into_replacement() {
    let state = RefCell::new(NativeModelDeliveryState::default());
    let current = Cell::new(true);
    let notifications = Cell::new(0);
    let result = resolve_native_model_paths(
        &state,
        &owner(1),
        vec!["model.step".into()],
        || current.get(),
        || notifications.set(notifications.get() + 1),
        |_| async {
            gloo_timers::future::TimeoutFuture::new(0).await;
            current.set(false);
            Err("old module import rejected".into())
        },
    )
    .await;
    assert_eq!(
        result,
        Ok(None),
        "stale failure must not reach the caller's global report callback"
    );
    assert_eq!(
        notifications.get(),
        1,
        "stale completion does not notify replacement state"
    );
}

#[wasm_bindgen_test]
async fn new_preview_retries_after_failure_and_success_keeps_batch_pending() {
    let state = RefCell::new(NativeModelDeliveryState::default());
    assert!(
        resolve_native_model_paths(
            &state,
            &owner(1),
            vec!["model.step".into()],
            || true,
            || {},
            |_| async { Err("offline".into()) },
        )
        .await
        .is_err()
    );
    let next = owner(2);
    let mapped = resolve_native_model_paths(
        &state,
        &next,
        vec!["model.step".into()],
        || true,
        || {},
        |_| async { Ok(vec![Some("bundled-model:known.step".into())]) },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        mapped.get("model.step"),
        Some(&Some("bundled-model:known.step".into()))
    );
    assert_eq!(state.borrow().pending.as_ref(), Some(&next));
    assert!(state.borrow().failed.is_none());
    assert_eq!(
        resolve_native_model_paths(
            &state,
            &next,
            vec!["model.step".into()],
            || true,
            || {},
            |_| async { panic!("the decoding batch already owns this preview") },
        )
        .await,
        Ok(None)
    );
}

#[wasm_bindgen_test]
async fn obsolete_failure_keeps_new_owner_pending_and_emits_no_report() {
    let state = RefCell::new(NativeModelDeliveryState::default());
    let current = Cell::new(1);
    let next = owner(2);
    let result = resolve_native_model_paths(
        &state,
        &owner(1),
        vec!["old.step".into()],
        || current.get() == 1,
        || {},
        |_| async {
            gloo_timers::future::TimeoutFuture::new(0).await;
            current.set(2);
            assert!(
                resolve_native_model_paths(
                    &state,
                    &next,
                    vec!["new.step".into()],
                    || current.get() == 2,
                    || {},
                    |_| async { Ok(vec![None]) },
                )
                .await
                .unwrap()
                .is_some()
            );
            Err("obsolete import failed".into())
        },
    )
    .await;
    assert_eq!(result, Ok(None));
    assert_eq!(state.borrow().pending.as_ref(), Some(&next));
    assert!(state.borrow().failed.is_none());
}

#[wasm_bindgen_test]
async fn stale_mapping_success_cannot_enter_delivery_or_replace_pending() {
    let state = RefCell::new(NativeModelDeliveryState::default());
    let current = Cell::new(true);
    let result = resolve_native_model_paths(
        &state,
        &owner(1),
        vec!["old.step".into()],
        || current.get(),
        || {},
        |_| async {
            gloo_timers::future::TimeoutFuture::new(0).await;
            current.set(false);
            Ok(vec![Some("bundled-model:known.step".into())])
        },
    )
    .await;
    assert_eq!(result, Ok(None));
    assert!(state.borrow().pending.is_none());
}
