//! Diagnostic-only gated persistence adapter for packaged verification (ticket 009).
//!
//! Behind the `diagnostic-persist-gate` Cargo feature this module wraps the real
//! [`BrowserStore`] at the [`DocumentPersistence`] port with a single-armed gate:
//! the next save can be held genuinely unanswered (pre-transaction, before any
//! durable write), failed outright, or held-then-failed atomically on release.
//! The target is always the real browser store; the worker lifecycle, Session and
//! save policy are untouched. Normal builds (feature off) compile this module out
//! entirely and the production composition installs the bare store.
//!
//! Packaged drivers arm and release the gate through `window.__boardstudioPersistGate`
//! (`arm_hold`, `arm_fail`, `arm_hold_then_fail`, `snapshot`, `release`, `disarm`);
//! `snapshot` is polled for state. Persistence is judged by the store's
//! completion/abort acknowledgements (`Ok`/`Err` of `save_document`, which resolve
//! only on IndexedDB transaction complete/abort), never by request-success
//! callbacks, save labels or worker labels.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use boardstudio_core::model::ProjectDoc;
use boardstudio_web_host::host::{BrowserStore, SaveError, StorageToken};
use futures_channel::oneshot;

use crate::runtime::DocumentPersistence;

/// Window property hosting the diagnostic control surface (diagnostic builds only).
pub(crate) const WINDOW_PROPERTY: &str = "__boardstudioPersistGate";

/// Single-armed behavior waiting for the next save at the persistence port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ArmedMode {
    Hold,
    Fail(String),
    HoldThenFail(String),
}

impl ArmedMode {
    fn as_str(&self) -> &'static str {
        match self {
            ArmedMode::Hold => "hold",
            ArmedMode::Fail(_) => "fail",
            ArmedMode::HoldThenFail(_) => "hold_then_fail",
        }
    }
}

/// Identity captured when a save reaches the gate, before any durable write.
#[derive(Clone, Debug, PartialEq, Eq)]
struct EnteredSave {
    document_id: String,
    document_name: String,
    revision: u64,
    expected_token_present: bool,
}

/// How the entered save was acknowledged by the real store.
#[derive(Clone, Debug, PartialEq, Eq)]
enum AckOutcome {
    Completed,
    Aborted(String),
}

impl AckOutcome {
    fn as_string(&self) -> String {
        match self {
            AckOutcome::Completed => "completed".to_owned(),
            AckOutcome::Aborted(reason) => format!("aborted:{reason}"),
        }
    }
}

#[derive(Debug, Default)]
struct GateState {
    armed: Option<ArmedMode>,
    entered: Option<EnteredSave>,
    release: Option<oneshot::Sender<()>>,
    /// Hold-then-fail reason parked with the held save; consumed atomically on release.
    fail_after_hold: Option<String>,
    released: bool,
    acked: Option<AckOutcome>,
}

/// Polled diagnostic snapshot of the gate (see the `snapshot()` control).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct GateSnapshot {
    pub(crate) armed: Option<String>,
    pub(crate) entered: bool,
    pub(crate) document_id: Option<String>,
    pub(crate) document_name: Option<String>,
    pub(crate) revision: Option<u64>,
    pub(crate) expected_token_present: bool,
    pub(crate) released: bool,
    pub(crate) acked: Option<String>,
}

/// Diagnostic gate around the real browser store. The production composition
/// installs it through [`wrap_store`] in diagnostic builds only.
pub struct DiagnosticPersistGate {
    store: BrowserStore,
    state: Rc<RefCell<GateState>>,
}

impl DiagnosticPersistGate {
    pub(crate) fn through_store(store: BrowserStore) -> Rc<Self> {
        Rc::new(Self {
            store,
            state: Rc::new(RefCell::new(GateState::default())),
        })
    }

    /// Hold the next save unanswered until [`release`](Self::release).
    /// Single-armed: arming again replaces the pending mode; entry state is reset.
    pub(crate) fn arm_hold(&self) {
        let mut state = self.state.borrow_mut();
        state.armed = Some(ArmedMode::Hold);
        state.entered = None;
        state.released = false;
        state.acked = None;
    }

    /// Fail the next save outright with `reason`; no durable write happens.
    pub(crate) fn arm_fail(&self, reason: impl Into<String>) {
        let mut state = self.state.borrow_mut();
        state.armed = Some(ArmedMode::Fail(reason.into()));
        state.entered = None;
        state.released = false;
        state.acked = None;
    }

    /// Hold the next save, then fail it atomically when released (one armed
    /// mode, not a hold re-armed into a fail).
    pub(crate) fn arm_hold_then_fail(&self, reason: impl Into<String>) {
        let mut state = self.state.borrow_mut();
        state.armed = Some(ArmedMode::HoldThenFail(reason.into()));
        state.entered = None;
        state.released = false;
        state.acked = None;
    }

    /// Release a held save: a plain hold completes against the real store, a
    /// hold-then-fail aborts. No-op when nothing is held.
    pub(crate) fn release(&self) {
        let waiter = self.state.borrow_mut().release.take();
        if let Some(waiter) = waiter {
            self.state.borrow_mut().released = true;
            let _ = waiter.send(());
        }
    }

    /// Clear a pending arm (and a parked hold-then-fail reason). A save already
    /// held stays held until released; the release then completes it against
    /// the real store.
    pub(crate) fn disarm(&self) {
        let mut state = self.state.borrow_mut();
        state.armed = None;
        state.fail_after_hold = None;
    }

    /// Current polled state of the gate.
    pub(crate) fn snapshot(&self) -> GateSnapshot {
        let state = self.state.borrow();
        GateSnapshot {
            armed: state.armed.as_ref().map(|mode| mode.as_str().to_owned()),
            entered: state.entered.is_some(),
            document_id: state
                .entered
                .as_ref()
                .map(|entry| entry.document_id.clone()),
            document_name: state
                .entered
                .as_ref()
                .map(|entry| entry.document_name.clone()),
            revision: state.entered.as_ref().map(|entry| entry.revision),
            expected_token_present: state
                .entered
                .as_ref()
                .is_some_and(|entry| entry.expected_token_present),
            released: state.released,
            acked: state.acked.as_ref().map(AckOutcome::as_string),
        }
    }
}

impl DocumentPersistence for DiagnosticPersistGate {
    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
        expected: Option<&'a StorageToken>,
    ) -> Pin<Box<dyn Future<Output = Result<StorageToken, SaveError>> + 'a>> {
        Box::pin(async move {
            // Single-shot: the armed mode is taken at persist entry; the gate is
            // disarmed again afterwards and must be re-armed explicitly. The
            // borrow ends here so arms can borrow the state again.
            let behavior = self.state.borrow_mut().armed.take();
            match behavior {
                None => self.delegate(document, assets, expected).await,
                Some(ArmedMode::Fail(reason)) => {
                    self.state.borrow_mut().acked = Some(AckOutcome::Aborted(reason.clone()));
                    Err(SaveError::Storage(reason))
                }
                Some(ArmedMode::Hold) => self.hold_save(document, assets, expected, None).await,
                Some(ArmedMode::HoldThenFail(reason)) => {
                    self.hold_save(document, assets, expected, Some(reason))
                        .await
                }
            }
        })
    }
}

impl DiagnosticPersistGate {
    /// Delegate to the real store, attesting its completion/abort acknowledgement.
    async fn delegate(
        &self,
        document: &ProjectDoc,
        assets: &BTreeMap<String, Vec<u8>>,
        expected: Option<&StorageToken>,
    ) -> Result<StorageToken, SaveError> {
        let outcome = self
            .store
            .save_document_expected(document, assets, expected)
            .await;
        self.state.borrow_mut().acked = Some(match &outcome {
            Ok(_) => AckOutcome::Completed,
            Err(error) => AckOutcome::Aborted(error.to_string()),
        });
        outcome
    }

    /// Park the save before any transaction starts: record its identity, wait for
    /// [`release`](Self::release), then complete against the real store or fail
    /// atomically when a hold-then-fail reason was parked with it.
    async fn hold_save(
        &self,
        document: &ProjectDoc,
        assets: &BTreeMap<String, Vec<u8>>,
        expected: Option<&StorageToken>,
        fail_after_hold: Option<String>,
    ) -> Result<StorageToken, SaveError> {
        let waiter = {
            let mut state = self.state.borrow_mut();
            state.entered = Some(EnteredSave {
                document_id: document.id.clone(),
                document_name: document.name.clone(),
                revision: document.revision,
                expected_token_present: expected.is_some(),
            });
            let (sender, waiter) = oneshot::channel();
            state.release = Some(sender);
            state.fail_after_hold = fail_after_hold;
            waiter
        };
        let _ = waiter.await;
        let fail_after_hold = self.state.borrow_mut().fail_after_hold.take();
        if let Some(reason) = fail_after_hold {
            self.state.borrow_mut().acked = Some(AckOutcome::Aborted(reason.clone()));
            return Err(SaveError::Storage(reason));
        }
        self.delegate(document, assets, expected).await
    }
}

/// True in diagnostic builds; the module (and this function) is compiled out
/// otherwise, so a normal packaged build contains no armed gate. Test-only
/// scaffolding: the packaged identity is the window surface itself.
#[cfg(test)]
pub(crate) fn build_has_diagnostic_gate() -> bool {
    true
}

std::thread_local! {
    static CURRENT_GATE: RefCell<Option<Rc<DiagnosticPersistGate>>> =
        const { RefCell::new(None) };
}

fn current_gate() -> Option<Rc<DiagnosticPersistGate>> {
    CURRENT_GATE.with(|slot| slot.borrow().clone())
}

/// Install `store` behind the diagnostic gate and publish the window control
/// surface. Called once from the production persistence composition in
/// diagnostic builds; the worker, Session and save policy are untouched.
pub(crate) fn wrap_store(store: BrowserStore) -> Rc<dyn DocumentPersistence> {
    let gate = DiagnosticPersistGate::through_store(store);
    CURRENT_GATE.with(|slot| *slot.borrow_mut() = Some(gate.clone()));
    register_window_gate();
    gate
}

fn register_window_gate() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let handle = DiagnosticPersistGateHandle;
    let _ = js_sys::Reflect::set(
        &window.into(),
        &wasm_bindgen::JsValue::from_str(WINDOW_PROPERTY),
        &wasm_bindgen::JsValue::from(handle),
    );
}

/// Window handle driving the latest constructed gate (`window.__boardstudioPersistGate`).
#[wasm_bindgen::prelude::wasm_bindgen]
pub struct DiagnosticPersistGateHandle;

#[wasm_bindgen::prelude::wasm_bindgen]
impl DiagnosticPersistGateHandle {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = arm_hold)]
    pub fn arm_hold(&self) {
        if let Some(gate) = current_gate() {
            gate.arm_hold();
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = arm_fail)]
    pub fn arm_fail(&self, reason: String) {
        if let Some(gate) = current_gate() {
            gate.arm_fail(reason);
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = arm_hold_then_fail)]
    pub fn arm_hold_then_fail(&self, reason: String) {
        if let Some(gate) = current_gate() {
            gate.arm_hold_then_fail(reason);
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = snapshot)]
    pub fn snapshot(&self) -> wasm_bindgen::JsValue {
        let snapshot = current_gate().map(|gate| gate.snapshot());
        serde_wasm_bindgen::to_value(&snapshot)
            .unwrap_or_else(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = release)]
    pub fn release(&self) {
        if let Some(gate) = current_gate() {
            gate.release();
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen(js_name = disarm)]
    pub fn disarm(&self) {
        if let Some(gate) = current_gate() {
            gate.disarm();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_test::wasm_bindgen_test;

    const WAIT_BOUND_MS: f64 = 5_000.0;
    const POLL_MS: u32 = 5;

    // A returned check failure lets the fixture await deletion before the harness
    // reports it. Panic-abort/traps cannot run asynchronous Drop on wasm.
    macro_rules! check {
        ($condition:expr, $($message:tt)+) => {
            let passed = $condition;
            if !passed {
                return Err(format!($($message)+));
            }
        };
    }

    macro_rules! check_eq {
        ($actual:expr, $expected:expr $(, $message:literal)? $(,)?) => {{
            let (actual, expected) = ($actual, $expected);
            check!(
                actual == expected,
                "{}: got {actual:?}, expected {expected:?}",
                concat!(stringify!($actual), $(": ", $message,)?)
            );
        }};
    }

    struct DatabaseCallbacks {
        request: web_sys::IdbOpenDbRequest,
        _success: Closure<dyn FnMut(web_sys::Event)>,
        _error: Closure<dyn FnMut(web_sys::Event)>,
        _blocked: Closure<dyn FnMut(web_sys::Event)>,
    }

    impl Drop for DatabaseCallbacks {
        fn drop(&mut self) {
            self.request.set_onsuccess(None);
            self.request.set_onerror(None);
            self.request.set_onblocked(None);
        }
    }

    fn database_factory() -> Result<web_sys::IdbFactory, String> {
        web_sys::window()
            .ok_or("no test window")?
            .indexed_db()
            .map_err(|error| format!("IndexedDB unavailable: {error:?}"))?
            .ok_or_else(|| "IndexedDB unavailable".to_owned())
    }

    async fn database_request(request: web_sys::IdbOpenDbRequest) -> Result<JsValue, String> {
        let outcome = Rc::new(RefCell::new(None));
        let success = outcome.clone();
        let success_request = request.clone();
        let on_success = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            success.borrow_mut().get_or_insert_with(|| {
                success_request
                    .result()
                    .map_err(|error| format!("IndexedDB result: {error:?}"))
            });
        });
        let error = outcome.clone();
        let error_request = request.clone();
        let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            error.borrow_mut().get_or_insert_with(|| {
                Err(format!(
                    "IndexedDB request error: {:?}",
                    error_request.error()
                ))
            });
        });
        let blocked = outcome.clone();
        let on_blocked = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            blocked.borrow_mut().get_or_insert_with(|| {
                Err("IndexedDB request blocked by an open connection".to_owned())
            });
        });
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        request.set_onblocked(Some(on_blocked.as_ref().unchecked_ref()));
        let _callbacks = DatabaseCallbacks {
            request,
            _success: on_success,
            _error: on_error,
            _blocked: on_blocked,
        };
        let deadline = js_sys::Date::now() + WAIT_BOUND_MS;
        loop {
            if let Some(result) = outcome.borrow_mut().take() {
                return result;
            }
            if js_sys::Date::now() >= deadline {
                return Err("IndexedDB request timed out without a terminal event".to_owned());
            }
            pump(POLL_MS).await;
        }
    }

    fn unique_database(tag: &str) -> String {
        format!("boardstudio-diagnostic-gate-{tag}-{}", js_sys::Date::now())
    }

    fn open_store(name: &str) -> BrowserStore {
        BrowserStore::new(name)
            .map_err(|error| error.to_string())
            .unwrap()
    }

    async fn delete_database(name: &str) -> Result<(), String> {
        let request = database_factory()?
            .delete_database(name)
            .map_err(|error| format!("delete {name}: {error:?}"))?;
        database_request(request)
            .await
            .map(|_| ())
            .map_err(|error| format!("delete {name}: {error}"))
    }

    #[wasm_bindgen_test]
    async fn cleanup_reports_blocked_instead_of_attesting_deletion() {
        let name = unique_database("blocked");
        let request = database_factory().unwrap().open(&name).unwrap();
        let connection: web_sys::IdbDatabase =
            database_request(request).await.unwrap().dyn_into().unwrap();
        let outcome = delete_database(&name).await;
        connection.close();
        delete_database(&name).await.unwrap();
        assert!(
            outcome.is_err_and(|error| error.contains("blocked")),
            "an open connection must make cleanup fail honestly"
        );
    }

    async fn pump(milliseconds: u32) {
        gloo_timers::future::TimeoutFuture::new(milliseconds).await;
    }

    async fn in_database(
        name: &str,
        test: impl Future<Output = Result<(), String>>,
    ) -> Result<(), String> {
        let outcome = test.await;
        let cleanup = delete_database(name).await;
        match (outcome, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (outcome, Err(error)) => Err(format!(
                "test outcome: {outcome:?}; cleanup failed: {error}"
            )),
        }
    }

    async fn entered_snapshot(
        gate: &DiagnosticPersistGate,
        mut save: Pin<&mut impl Future<Output = Result<StorageToken, SaveError>>>,
    ) -> Result<GateSnapshot, String> {
        let deadline = js_sys::Date::now() + WAIT_BOUND_MS;
        loop {
            // Own the save rather than detach a task: a failed check drops it
            // before deletion, so an unreleased hold cannot write later.
            let ack =
                std::future::poll_fn(|cx| std::task::Poll::Ready(save.as_mut().poll(cx))).await;
            check!(
                ack.is_pending(),
                "held save acknowledged before release: {ack:?}"
            );
            let snapshot = gate.snapshot();
            if snapshot.entered {
                return Ok(snapshot);
            }
            check!(
                js_sys::Date::now() < deadline,
                "save never entered the gate: {snapshot:?}"
            );
            pump(POLL_MS).await;
        }
    }

    async fn released_ack(
        mut save: Pin<&mut impl Future<Output = Result<StorageToken, SaveError>>>,
    ) -> Result<Result<StorageToken, SaveError>, String> {
        let deadline = js_sys::Date::now() + WAIT_BOUND_MS;
        loop {
            if let std::task::Poll::Ready(outcome) =
                std::future::poll_fn(|cx| std::task::Poll::Ready(save.as_mut().poll(cx))).await
            {
                return Ok(outcome);
            }
            check!(
                js_sys::Date::now() < deadline,
                "released save never acknowledged"
            );
            pump(POLL_MS).await;
        }
    }

    #[wasm_bindgen_test]
    async fn cleanup_awaits_deletion_even_after_a_failed_check() {
        let name = unique_database("failed-check");
        let outcome = in_database(&name, async {
            let store = open_store(&name);
            let document = probe_document("Cleanup failure-path fixture", 1);
            store
                .save_document(&document, &BTreeMap::new())
                .await
                .map_err(|error| error.to_string())?;
            check!(false, "deliberate failed check");
            Ok(())
        })
        .await;

        // Opening an absent database raises upgradeneeded; opening a retained
        // database would not. Close and delete this verification open as well.
        let request = database_factory().unwrap().open(&name).unwrap();
        let recreated = Rc::new(std::cell::Cell::new(false));
        let observed = recreated.clone();
        let on_upgrade = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| observed.set(true));
        request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
        let opened = database_request(request.clone()).await;
        request.set_onupgradeneeded(None);
        let connection: web_sys::IdbDatabase = opened.unwrap().dyn_into().unwrap();
        connection.close();
        delete_database(&name).await.unwrap();
        assert_eq!(outcome, Err("deliberate failed check".to_owned()));
        assert!(
            recreated.get(),
            "failed checks must still await actual deletion"
        );
    }

    #[wasm_bindgen_test]
    async fn cleanup_error_event_is_reported_and_callbacks_detach() {
        let name = unique_database("error-event");
        let request = database_factory().unwrap().delete_database(&name).unwrap();
        let mut waiting = std::pin::pin!(database_request(request.clone()));
        let initial =
            std::future::poll_fn(|cx| std::task::Poll::Ready(waiting.as_mut().poll(cx))).await;
        // Synthetic error exercises the handler deterministically, not a claim
        // that the browser's storage backend suffered a real deletion error.
        request
            .dispatch_event(&web_sys::Event::new("error").unwrap())
            .unwrap();
        let outcome = waiting.await;
        let detached = request.onsuccess().is_none()
            && request.onerror().is_none()
            && request.onblocked().is_none();
        delete_database(&name).await.unwrap();
        assert!(initial.is_pending());
        assert!(outcome.is_err_and(|error| error.contains("request error")));
        assert!(
            detached,
            "error callbacks must not outlive their Rust closures"
        );
    }

    #[wasm_bindgen_test]
    async fn queued_cleanup_times_out_and_detaches_without_claiming_success() {
        let name = unique_database("timeout");
        let open = database_factory().unwrap().open(&name).unwrap();
        let connection: web_sys::IdbDatabase =
            database_request(open).await.unwrap().dyn_into().unwrap();
        let blocked = delete_database(&name).await;
        // This deletion is queued behind the first, still-blocked deletion: no
        // event can complete it until the real connection is closed below.
        let request = database_factory().unwrap().delete_database(&name).unwrap();
        let outcome = database_request(request.clone()).await;
        let detached = request.onsuccess().is_none()
            && request.onerror().is_none()
            && request.onblocked().is_none();
        connection.close();
        delete_database(&name).await.unwrap();
        assert!(blocked.is_err_and(|error| error.contains("blocked")));
        assert!(outcome.is_err_and(|error| error.contains("timed out")));
        assert!(
            detached,
            "timeout must detach callbacks before dropping closures"
        );
    }

    fn probe_document(name: &str, revision: u64) -> ProjectDoc {
        let mut document = ProjectDoc::empty("diagnostic-009", name);
        document.revision = revision;
        document
    }

    #[wasm_bindgen_test]
    async fn held_save_reports_entered_without_ack_or_durable_write_until_released() {
        let database = unique_database("hold");
        let result = in_database(&database, async {
            let store = open_store(&database);
            let gate = DiagnosticPersistGate::through_store(store.clone());
            let document = probe_document("Diagnostic hold probe", 7);
            gate.arm_hold();
            let mut save = std::pin::pin!(async {
                // Deliberately enter later than the former 150 ms guess. This is
                // latency stimulus, not a sleep used to decide readiness.
                pump(250).await;
                gate.save_document(&document, &BTreeMap::new(), None).await
            });
            let snapshot = entered_snapshot(&gate, save.as_mut()).await?;
            check_eq!(snapshot.document_id.as_deref(), Some("diagnostic-009"));
            check_eq!(
                snapshot.document_name.as_deref(),
                Some("Diagnostic hold probe")
            );
            check_eq!(snapshot.revision, Some(7));
            check!(!snapshot.released, "nothing released the held save yet");
            check_eq!(snapshot.acked, None, "the store acknowledged nothing yet");
            check_eq!(
                store
                    .load_document("diagnostic-009".to_owned())
                    .await
                    .map_err(|error| error.to_string())?,
                None,
                "the entered, unanswered save made no durable change"
            );

            gate.release();
            released_ack(save.as_mut())
                .await?
                .map_err(|error| error.to_string())?;
            let snapshot = gate.snapshot();
            check!(snapshot.released, "the release is attested");
            check_eq!(snapshot.acked.as_deref(), Some("completed"));
            let reopened = store
                .load_document("diagnostic-009".to_owned())
                .await
                .map_err(|error| error.to_string())?
                .ok_or("the released save is not durable")?;
            check_eq!(reopened.name, "Diagnostic hold probe");
            check_eq!(reopened.revision, 7);
            Ok(())
        })
        .await;
        assert_eq!(result, Ok(()));
    }

    #[wasm_bindgen_test]
    async fn armed_fail_aborts_without_durable_write_and_explicit_retry_succeeds() {
        let database = unique_database("fail");
        let result = in_database(&database, async {
            let store = open_store(&database);
            let gate = DiagnosticPersistGate::through_store(store.clone());
            let document = probe_document("Diagnostic fail probe", 3);
            gate.arm_fail("diagnostic quota");
            let outcome = gate.save_document(&document, &BTreeMap::new(), None).await;
            check!(
                matches!(outcome, Err(SaveError::Storage(_))),
                "the armed fail must abort, got {outcome:?}"
            );
            check_eq!(
                store
                    .load_document("diagnostic-009".to_owned())
                    .await
                    .map_err(|error| error.to_string())?,
                None,
                "the failed save wrote nothing durable"
            );
            gate.save_document(&document, &BTreeMap::new(), None)
                .await
                .map_err(|error| error.to_string())?;
            let reopened = store
                .load_document("diagnostic-009".to_owned())
                .await
                .map_err(|error| error.to_string())?
                .ok_or("the retry is not durable")?;
            check_eq!(reopened.name, "Diagnostic fail probe");
            Ok(())
        })
        .await;
        assert_eq!(result, Ok(()));
    }

    #[wasm_bindgen_test]
    async fn hold_then_fail_releases_into_failure_atomically_and_disarms() {
        let database = unique_database("hold-fail");
        let result = in_database(&database, async {
            let store = open_store(&database);
            let gate = DiagnosticPersistGate::through_store(store.clone());
            let baseline = probe_document("Diagnostic baseline", 4);
            gate.save_document(&baseline, &BTreeMap::new(), None)
                .await
                .map_err(|error| error.to_string())?;
            let token = StorageToken::for_document(&baseline).map_err(|error| error.to_string())?;
            let attempted = probe_document("Diagnostic overwritten", 5);
            gate.arm_hold_then_fail("diagnostic quota");
            let assets = BTreeMap::new();
            let mut save = std::pin::pin!(gate.save_document(&attempted, &assets, Some(&token)));
            let snapshot = entered_snapshot(&gate, save.as_mut()).await?;
            check!(
                snapshot.expected_token_present,
                "entry records the expected token"
            );
            check_eq!(
                snapshot.acked,
                None,
                "the store acknowledged nothing before release"
            );
            check_eq!(
                store
                    .load_document("diagnostic-009".to_owned())
                    .await
                    .map_err(|error| error.to_string())?,
                Some(baseline.clone()),
                "the entered hold made no durable change"
            );

            gate.release();
            let outcome = released_ack(save.as_mut()).await?;
            check!(
                matches!(&outcome, Err(SaveError::Storage(reason)) if reason == "diagnostic quota"),
                "release must fail atomically without re-arming, got {outcome:?}"
            );
            let kept = store
                .load_document("diagnostic-009".to_owned())
                .await
                .map_err(|error| error.to_string())?
                .ok_or("the baseline disappeared")?;
            check_eq!(kept.name, "Diagnostic baseline");
            check_eq!(kept.revision, 4);
            gate.save_document(&attempted, &BTreeMap::new(), Some(&token))
                .await
                .map_err(|error| error.to_string())?;
            Ok(())
        })
        .await;
        assert_eq!(result, Ok(()));
    }

    #[wasm_bindgen_test]
    async fn disarmed_gate_passes_saves_through_transparently() {
        let database = unique_database("passthrough");
        let result = in_database(&database, async {
            let store = open_store(&database);
            let gate = DiagnosticPersistGate::through_store(store.clone());
            let document = probe_document("Diagnostic passthrough", 1);
            gate.save_document(&document, &BTreeMap::new(), None)
                .await
                .map_err(|error| error.to_string())?;
            let reopened = store
                .load_document("diagnostic-009".to_owned())
                .await
                .map_err(|error| error.to_string())?
                .ok_or("the delegated save is not durable")?;
            check_eq!(reopened.name, "Diagnostic passthrough");
            Ok(())
        })
        .await;
        assert_eq!(result, Ok(()));
    }

    #[wasm_bindgen_test]
    async fn diagnostic_build_reports_its_gate() {
        assert!(
            build_has_diagnostic_gate(),
            "this test runs in the diagnostic build"
        );
    }

    #[wasm_bindgen_test]
    async fn window_surface_exposes_the_named_gate() {
        let database = unique_database("window");
        let result = in_database(&database, async {
            let store = open_store(&database);
            let _gate = wrap_store(store);
            let window = web_sys::window().ok_or("a window must host the gate")?;
            let surface = js_sys::Reflect::get(&window, &JsValue::from_str(WINDOW_PROPERTY))
                .map_err(|error| format!("gate property: {error:?}"))?;
            check!(surface.is_object(), "the gate surface is not an object");
            for method in [
                "arm_hold",
                "arm_fail",
                "arm_hold_then_fail",
                "snapshot",
                "release",
                "disarm",
            ] {
                let member = js_sys::Reflect::get(&surface, &JsValue::from_str(method))
                    .map_err(|error| format!("gate method {method}: {error:?}"))?;
                check!(
                    member.is_function(),
                    "the gate surface must expose {method}()"
                );
            }
            DiagnosticPersistGateHandle.arm_hold();
            let armed = current_gate()
                .ok_or("wrap_store must publish the gate")?
                .snapshot();
            check_eq!(armed.armed.as_deref(), Some("hold"));
            Ok(())
        })
        .await;
        CURRENT_GATE.with(|slot| slot.borrow_mut().take());
        let window = web_sys::window().unwrap();
        let removed = js_sys::Reflect::delete_property(
            window.unchecked_ref::<js_sys::Object>(),
            &JsValue::from_str(WINDOW_PROPERTY),
        );
        assert_eq!(removed, Ok(true));
        assert_eq!(result, Ok(()));
    }
}
