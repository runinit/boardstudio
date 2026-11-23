//! Shared runtime fixtures and detached task support for web tests.
pub mod layout_fixtures;

#[cfg(not(target_arch = "wasm32"))]
pub fn new_native_runtime() -> std::rc::Rc<boardstudio_web_runtime::runtime::Runtime> {
    boardstudio_web_runtime::runtime::Runtime::new()
}

thread_local! {
    static TASKS: std::cell::RefCell<Vec<std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>>> =
        std::cell::RefCell::default();
}

/// Schedule a detached future, mirroring `wasm_bindgen_futures::spawn_local` in the browser.
/// Native `VirtualDom` lifecycle tests share this queue to drive hooks that detach async work.
pub fn spawn_local(task: impl std::future::Future<Output = ()> + 'static) {
    TASKS.with_borrow_mut(|tasks| tasks.push(Box::pin(task)));
}

/// Drive every detached future until it pends, mirroring the browser microtask queue.
pub fn poll_detached() {
    use std::task::{Context, Waker};
    TASKS.with_borrow_mut(|tasks| {
        tasks.retain_mut(|task| {
            task.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        });
    });
}

/// Number of detached futures still pending on the shared queue.
pub fn pending_detached() -> usize {
    TASKS.with_borrow(|tasks| tasks.len())
}

/// Drop every queued detached future without polling it.
pub fn clear_detached() {
    TASKS.with_borrow_mut(Vec::clear);
}

#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_runtime::{gate_driver, runtime};
#[cfg(target_arch = "wasm32")]
mod fixtures;
#[cfg(target_arch = "wasm32")]
pub use fixtures::{firmware_export_test_support, in_process_support, project_name_test_support};
