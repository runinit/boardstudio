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
    use wasm_bindgen_test::wasm_bindgen_test;

    fn unique_database(tag: &str) -> String {
        format!("boardstudio-diagnostic-gate-{tag}-{}", js_sys::Date::now())
    }

    fn open_store(name: &str) -> BrowserStore {
        BrowserStore::new(name)
            .map_err(|error| error.to_string())
            .unwrap()
    }

    async fn delete_database(name: &str) {
        web_sys::window()
            .unwrap()
            .indexed_db()
            .unwrap()
            .unwrap()
            .delete_database(name)
            .unwrap();
    }

    async fn pump(milliseconds: u32) {
        gloo_timers::future::TimeoutFuture::new(milliseconds).await;
    }

    fn probe_document(name: &str, revision: u64) -> ProjectDoc {
        let mut document = ProjectDoc::empty("diagnostic-009", name);
        document.revision = revision;
        document
    }

    #[wasm_bindgen_test]
    async fn held_save_reports_entered_without_ack_or_durable_write_until_released() {
        let database = unique_database("hold");
        let store = open_store(&database);
        let gate = DiagnosticPersistGate::through_store(store.clone());
        let document = probe_document("Diagnostic hold probe", 7);

        gate.arm_hold();
        let completed: Rc<RefCell<Option<Result<StorageToken, SaveError>>>> =
            Rc::new(RefCell::new(None));
        let done = completed.clone();
        let held = gate.clone();
        let held_document = document.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let outcome = held
                .save_document(&held_document, &BTreeMap::new(), None)
                .await;
            *done.borrow_mut() = Some(outcome);
        });
        pump(150).await;

        assert!(
            completed.borrow().is_none(),
            "the held save receives no acknowledgement while armed"
        );
        let snapshot = gate.snapshot();
        assert!(snapshot.entered, "the save is entered at the gate");
        assert_eq!(snapshot.document_id.as_deref(), Some("diagnostic-009"));
        assert_eq!(
            snapshot.document_name.as_deref(),
            Some("Diagnostic hold probe")
        );
        assert_eq!(snapshot.revision, Some(7));
        assert!(!snapshot.released, "nothing released the held save yet");
        assert_eq!(snapshot.acked, None, "the store acknowledged nothing yet");
        assert_eq!(
            store
                .load_document("diagnostic-009".to_owned())
                .await
                .map_err(|error| error.to_string())
                .unwrap(),
            None,
            "the held save made no durable change"
        );

        gate.release();
        for _ in 0..100 {
            if completed.borrow().is_some() {
                break;
            }
            pump(50).await;
        }
        let outcome = completed
            .borrow_mut()
            .take()
            .expect("release acks the save");
        outcome.expect("the released save completes against the real store");
        let snapshot = gate.snapshot();
        assert!(snapshot.released, "the release is attested");
        assert_eq!(snapshot.acked.as_deref(), Some("completed"));
        let reopened = store
            .load_document("diagnostic-009".to_owned())
            .await
            .map_err(|error| error.to_string())
            .unwrap()
            .expect("the released save is durable");
        assert_eq!(reopened.name, "Diagnostic hold probe");
        assert_eq!(reopened.revision, 7);
        delete_database(&database).await;
    }

    #[wasm_bindgen_test]
    async fn armed_fail_aborts_without_durable_write_and_explicit_retry_succeeds() {
        let database = unique_database("fail");
        let store = open_store(&database);
        let gate = DiagnosticPersistGate::through_store(store.clone());
        let document = probe_document("Diagnostic fail probe", 3);

        gate.arm_fail("diagnostic quota");
        let outcome = gate.save_document(&document, &BTreeMap::new(), None).await;
        assert!(
            matches!(outcome, Err(SaveError::Storage(_))),
            "the armed fail aborts the save, got {outcome:?}"
        );
        assert_eq!(
            store
                .load_document("diagnostic-009".to_owned())
                .await
                .map_err(|error| error.to_string())
                .unwrap(),
            None,
            "the failed save wrote nothing durable"
        );

        gate.save_document(&document, &BTreeMap::new(), None)
            .await
            .expect("an explicit retry succeeds once the single-armed fail is spent");
        let reopened = store
            .load_document("diagnostic-009".to_owned())
            .await
            .map_err(|error| error.to_string())
            .unwrap()
            .expect("the retry is durable");
        assert_eq!(reopened.name, "Diagnostic fail probe");
        delete_database(&database).await;
    }

    #[wasm_bindgen_test]
    async fn hold_then_fail_releases_into_failure_atomically_and_disarms() {
        let database = unique_database("hold-fail");
        let store = open_store(&database);
        let gate = DiagnosticPersistGate::through_store(store.clone());

        let baseline = probe_document("Diagnostic baseline", 4);
        gate.save_document(&baseline, &BTreeMap::new(), None)
            .await
            .expect("the baseline save lands first");
        let token = StorageToken::for_document(&baseline)
            .map_err(|error| error.to_string())
            .unwrap();

        let attempted = probe_document("Diagnostic overwritten", 5);
        gate.arm_hold_then_fail("diagnostic quota");
        let completed: Rc<RefCell<Option<Result<StorageToken, SaveError>>>> =
            Rc::new(RefCell::new(None));
        let done = completed.clone();
        let held = gate.clone();
        let attempted_held = attempted.clone();
        let token_held = token.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let outcome = held
                .save_document(&attempted_held, &BTreeMap::new(), Some(&token_held))
                .await;
            *done.borrow_mut() = Some(outcome);
        });
        pump(150).await;

        assert!(
            completed.borrow().is_none(),
            "the held save receives no acknowledgement before release"
        );
        let snapshot = gate.snapshot();
        assert!(snapshot.entered, "the save is entered at the gate");
        assert!(
            snapshot.expected_token_present,
            "entry records the expected token"
        );

        gate.release();
        for _ in 0..100 {
            if completed.borrow().is_some() {
                break;
            }
            pump(50).await;
        }
        let outcome = completed
            .borrow_mut()
            .take()
            .expect("release enters failure");
        assert!(
            matches!(&outcome, Err(SaveError::Storage(reason)) if reason.as_str() == "diagnostic quota"),
            "releasing the hold fails atomically without re-arming, got {outcome:?}"
        );
        let kept = store
            .load_document("diagnostic-009".to_owned())
            .await
            .map_err(|error| error.to_string())
            .unwrap()
            .expect("the baseline is still the durable value");
        assert_eq!(kept.name, "Diagnostic baseline");
        assert_eq!(kept.revision, 4);

        gate.save_document(&attempted, &BTreeMap::new(), Some(&token))
            .await
            .expect("the gate is reusable after the single-armed hold-then-fail");
        delete_database(&database).await;
    }

    #[wasm_bindgen_test]
    async fn disarmed_gate_passes_saves_through_transparently() {
        let database = unique_database("passthrough");
        let store = open_store(&database);
        let gate = DiagnosticPersistGate::through_store(store.clone());
        let document = probe_document("Diagnostic passthrough", 1);

        gate.save_document(&document, &BTreeMap::new(), None)
            .await
            .expect("a disarmed gate delegates to the real store");
        let reopened = store
            .load_document("diagnostic-009".to_owned())
            .await
            .map_err(|error| error.to_string())
            .unwrap()
            .expect("the delegated save is durable");
        assert_eq!(reopened.name, "Diagnostic passthrough");
        delete_database(&database).await;
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
        let store = open_store(&database);
        let _gate = wrap_store(store);
        let window = web_sys::window().expect("a window hosts the gate");
        let property = wasm_bindgen::JsValue::from_str(WINDOW_PROPERTY);
        let surface =
            js_sys::Reflect::get(&window.into(), &property).expect("the gate is registered");
        assert!(surface.is_object(), "the gate surface is an object");
        for method in [
            "arm_hold",
            "arm_fail",
            "arm_hold_then_fail",
            "snapshot",
            "release",
            "disarm",
        ] {
            let member = js_sys::Reflect::get(&surface, &wasm_bindgen::JsValue::from_str(method))
                .unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
            assert!(member.is_function(), "the gate surface exposes {method}()");
        }

        DiagnosticPersistGateHandle.arm_hold();
        let armed = current_gate()
            .expect("wrap_store publishes the gate")
            .snapshot();
        assert_eq!(armed.armed.as_deref(), Some("hold"));
        delete_database(&database).await;
    }
}
