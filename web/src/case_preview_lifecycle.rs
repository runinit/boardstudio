//! Mounted lifetime of the native Case preview producer.
use crate::runtime::Runtime;
use boardstudio_application::{Scope, SnapshotToken};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

pub(crate) fn use_native_case_preview(
    runtime: Rc<Runtime>,
    source: Option<(Scope, SnapshotToken, u64)>,
) {
    let request_lifetime = use_hook(|| Rc::new(RefCell::new(None::<Rc<Cell<bool>>>)));
    let dropping = request_lifetime.clone();
    let cancelling = runtime.clone();
    use_drop(move || {
        if let Some(active) = dropping.borrow_mut().take() {
            active.set(false);
        }
        cancelling.cancel_native_case_preview();
    });
    use_effect(use_reactive((&source,), move |(source,)| {
        if let Some(active) = request_lifetime.borrow_mut().take() {
            active.set(false);
        }
        runtime.cancel_native_case_preview();
        if let Some((scope, token, revision)) = source {
            let active = Rc::new(Cell::new(true));
            *request_lifetime.borrow_mut() = Some(active.clone());
            let runtime = runtime.clone();
            spawn_local(async move {
                // Detached browser futures may first poll after the owner unmounted
                // or its source was replaced. Such work must not acquire a new lease.
                if !active.get() {
                    return;
                }
                let _ = runtime
                    .prepare_native_case_preview(scope, token, revision)
                    .await;
            });
        }
    }));
}
