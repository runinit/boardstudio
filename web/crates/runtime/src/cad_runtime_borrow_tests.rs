use super::*;
use crate::runtime::project_name_test_support as support;
use futures_channel::oneshot;
use std::task::{Context, Poll, Waker};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn pending_keycap_preview_keeps_its_executor_without_borrowing_configuration() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, in_process_support::board_document()).await;
    let accepted = runtime.model().accepted.unwrap();
    let input = KeycapsPreviewInput {
        scope: runtime.scope().unwrap(),
        token: accepted.token,
        revision: accepted.document.revision,
        specs: Vec::new(),
    };
    let (release, held) = oneshot::channel::<()>();
    let held = Rc::new(RefCell::new(Some(held)));
    runtime.set_keycaps_preview_executor_test(Rc::new(move |_| {
        let held = held.borrow_mut().take().unwrap();
        Box::pin(async move {
            held.await
                .map_err(|_| "controlled reply dropped".to_owned())?;
            Err("captured executor".into())
        })
    }));
    let mut pending = Box::pin(runtime.request_keycaps_cad_preview(input.clone()));
    assert!(matches!(
        pending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    // Provider errors identify the selected executor; this is not a geometry check.
    runtime.set_keycaps_preview_executor_test(Rc::new(|_| {
        Box::pin(async { Err("replacement executor".into()) })
    }));
    release.send(()).unwrap();
    assert_eq!(pending.await.unwrap_err(), "captured executor");
    assert_eq!(
        runtime
            .request_keycaps_cad_preview(input)
            .await
            .unwrap_err(),
        "replacement executor"
    );
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
}

#[wasm_bindgen_test]
async fn pending_store_save_keeps_its_target_while_future_saves_switch_to_memory() {
    let runtime = support::new_runtime();
    support::install(
        &runtime,
        Session::new(),
        boardstudio_core::CoreEngine::new(),
    );
    let stored_id = new_project_id().unwrap();
    let mut target_captured = Box::pin(support::track_next_persist_target(&runtime));
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: ProjectDoc::empty(&stored_id, "Captured store save"),
    });
    let mut pending = Box::pin(support::run_pending(&runtime));
    // Switch only after the save captures its store, regardless of earlier Core waits.
    std::future::poll_fn(|cx| {
        assert!(pending.as_mut().poll(cx).is_pending());
        match target_captured.as_mut().poll(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(()),
            Poll::Ready(Err(_)) => panic!("persistence target notification was dropped"),
            Poll::Pending => Poll::Pending,
        }
    })
    .await;
    support::install_memory_persistence(&runtime);
    pending.await;
    assert_eq!(runtime.model().accepted.unwrap().document.id, stored_id);
    assert!(matches!(
        runtime.model().durability,
        Durability::Saved { .. }
    ));
    assert_eq!(
        runtime
            .store
            .load_document(stored_id.clone())
            .await
            .unwrap()
            .unwrap()
            .name,
        "Captured store save"
    );
    assert!(support::saved_document(&runtime, &stored_id).is_none());

    let memory_id = new_project_id().unwrap();
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: ProjectDoc::empty(&memory_id, "Subsequent memory save"),
    });
    support::run_pending(&runtime).await;
    assert_eq!(runtime.model().accepted.unwrap().document.id, memory_id);
    assert_eq!(
        support::saved_document(&runtime, &memory_id).unwrap().name,
        "Subsequent memory save"
    );
    assert!(
        runtime
            .store
            .load_document(memory_id)
            .await
            .unwrap()
            .is_none()
    );
    runtime.store.delete_project(stored_id).await.unwrap();
}
