//! In-process Core and persistence adapters for mounted browser tests. The adapter serves
//! Session Core requests through a real `CoreEngine`, serves archives and artifacts through
//! the same Core crate functions the worker uses, and saves documents either through the
//! production browser-store port or into memory. Each port accepts gates that hold or fail
//! the next reply or save, so tests control completion timing without feature-named branches
//! in Runtime itself.

use super::*;
use crate::gate_driver::{GateDriver, OneShotBehavior};
use boardstudio_core::{archive as core_archive, artifact_request};
use boardstudio_web_host::host::{ArchiveResult, CoreExecutorFuture, HostError};
use futures_channel::oneshot;

type AsyncGate = (oneshot::Sender<()>, oneshot::Receiver<()>);

enum PersistTarget {
    Store(BrowserStore),
    Memory(Rc<RefCell<BTreeMap<String, MemorySave>>>),
}

#[derive(Clone)]
struct MemorySave {
    document: ProjectDoc,
    assets: BTreeMap<String, Vec<u8>>,
}

/// The in-process Core executor: one `CoreEngine`, always ready, closable, with an optional
/// one-shot behavior that fails or holds the next Core reply.
pub struct InProcessCore {
    engine: RefCell<boardstudio_core::CoreEngine>,
    closed: Cell<bool>,
    behavior: GateDriver,
    gate: RefCell<Option<AsyncGate>>,
    request_filter: Cell<Option<fn(&CoreRequest) -> bool>>,
    scripted_reply: Cell<Option<fn(&CoreRequest) -> Option<CoreReply>>>,
    archive_behavior: GateDriver,
    archive_gate: RefCell<Option<AsyncGate>>,
    archive_reply: RefCell<Option<ArchiveResult>>,
}

impl InProcessCore {
    pub fn new(engine: boardstudio_core::CoreEngine) -> Self {
        Self {
            engine: RefCell::new(engine),
            closed: Cell::new(false),
            behavior: GateDriver::default(),
            gate: RefCell::new(None),
            request_filter: Cell::new(None),
            scripted_reply: Cell::new(None),
            archive_behavior: GateDriver::default(),
            archive_gate: RefCell::new(None),
            archive_reply: RefCell::new(None),
        }
    }

    pub fn fail_next_reply(&self, reason: impl Into<String>) {
        self.request_filter.set(None);
        self.behavior.fail_next_core(reason);
    }

    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }

    pub fn gate_next_reply(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        self.request_filter.set(None);
        let (entered, entered_rx) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        self.behavior.hold_next_core();
        *self.gate.borrow_mut() = Some((entered, release_rx));
        (entered_rx, release)
    }

    /// Filter a one-shot behavior so unrelated Session requests still reach Core.
    pub fn fail_matching_reply(
        &self,
        matches: fn(&CoreRequest) -> bool,
        reason: impl Into<String>,
    ) {
        self.fail_next_reply(reason);
        self.request_filter.set(Some(matches));
    }

    pub fn gate_matching_reply(
        &self,
        matches: fn(&CoreRequest) -> bool,
    ) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let gate = self.gate_next_reply();
        self.request_filter.set(Some(matches));
        gate
    }

    /// Script only the external-provider reply a fixture needs; return None for real Core.
    pub fn script_replies(&self, reply: fn(&CoreRequest) -> Option<CoreReply>) {
        self.scripted_reply.set(Some(reply));
    }

    pub fn fail_next_archive(&self, reason: impl Into<String>) {
        self.archive_behavior.fail_next_save(reason);
    }

    pub fn gate_next_archive(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, entered_rx) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        self.archive_behavior.hold_next_save();
        *self.archive_gate.borrow_mut() = Some((entered, release_rx));
        (entered_rx, release)
    }

    pub fn reply_to_next_archive(&self, reply: ArchiveResult) {
        *self.archive_reply.borrow_mut() = Some(reply);
    }

    fn take_behavior(&self) -> Option<OneShotBehavior> {
        self.behavior.take_core()
    }
}

impl CoreExecutor for InProcessCore {
    fn request<'a>(
        &'a self,
        request_id: &'a str,
        _executor_epoch: &'a str,
        request: &'a CoreRequest,
    ) -> CoreExecutorFuture<'a, CoreReply> {
        Box::pin(async move {
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            let behavior = if self
                .request_filter
                .get()
                .is_none_or(|matches| matches(request))
            {
                self.take_behavior()
            } else {
                None
            };
            if let Some(behavior) = behavior {
                match behavior {
                    OneShotBehavior::Fail(reason) => return Err(HostError(reason)),
                    OneShotBehavior::Hold => {
                        let Some((entered, release)) = self.gate.borrow_mut().take() else {
                            return Err(HostError("Core gate was not configured".into()));
                        };
                        let _ = entered.send(());
                        let _ = release.await;
                    }
                }
            }
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            let reply = self
                .scripted_reply
                .get()
                .and_then(|reply| reply(request))
                .unwrap_or_else(|| self.engine.borrow_mut().handle(request.clone()));
            let value = serde_json::to_value(&reply)
                .map_err(|error| HostError(format!("invalid core reply: {error}")))?;
            if value.get("id").and_then(serde_json::Value::as_str) != Some(request_id) {
                return Err(HostError("core reply id does not match request".into()));
            }
            Ok(reply)
        })
    }

    fn archive<'a>(
        &'a self,
        _request_id: &'a str,
        _executor_epoch: &'a str,
        metadata: &'a str,
        buffers: Vec<Uint8Array>,
    ) -> CoreExecutorFuture<'a, ArchiveResult> {
        Box::pin(async move {
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            let behavior = self.archive_behavior.take_save();
            let scripted = self.archive_reply.borrow_mut().take();
            if let Some(behavior) = behavior {
                match behavior {
                    OneShotBehavior::Fail(reason) => return Err(HostError(reason)),
                    OneShotBehavior::Hold => {
                        let Some((entered, release)) = self.archive_gate.borrow_mut().take() else {
                            return Err(HostError("archive gate was not configured".into()));
                        };
                        let _ = entered.send(());
                        let _ = release.await;
                    }
                }
            }
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            if let Some(reply) = scripted {
                return Ok(reply);
            }
            let inputs = buffers
                .iter()
                .map(|buffer| buffer.to_vec())
                .collect::<Vec<_>>();
            let (reply, output_bytes) = core_archive::request(metadata, &inputs);
            Ok(ArchiveResult {
                metadata: reply,
                buffers: output_bytes
                    .iter()
                    .map(|bytes| Uint8Array::from(bytes.as_slice()))
                    .collect(),
            })
        })
    }

    fn artifact<'a>(
        &'a self,
        request_id: &'a str,
        _executor_epoch: &'a str,
        request: &'a ArtifactRequest,
    ) -> CoreExecutorFuture<'a, ArtifactReply> {
        Box::pin(async move {
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            let frame = serde_json::to_string(request).map_err(|error| {
                HostError(format!("could not encode artifact request: {error}"))
            })?;
            let frame_id = serde_json::from_str::<serde_json::Value>(&frame)
                .ok()
                .and_then(|value| {
                    value
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                });
            if frame_id.as_deref() != Some(request_id) {
                return Err(HostError(
                    "artifact request id does not match worker request id".into(),
                ));
            }
            let reply_json = artifact_request(&frame);
            let value: serde_json::Value = serde_json::from_str(&reply_json)
                .map_err(|error| HostError(format!("invalid artifact reply: {error}")))?;
            if value.get("id").and_then(serde_json::Value::as_str) != Some(request_id) {
                return Err(HostError("artifact reply id does not match request".into()));
            }
            let reply: ArtifactReply = serde_json::from_value(value)
                .map_err(|error| HostError(format!("invalid artifact reply: {error}")))?;
            Ok(reply)
        })
    }

    fn ready<'a>(&'a self) -> CoreExecutorFuture<'a, ()> {
        Box::pin(async move {
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            Ok(())
        })
    }

    fn close(&self) {
        self.closed.set(true);
        self.take_behavior();
        self.archive_behavior.take_save();
        self.gate.borrow_mut().take();
        self.archive_gate.borrow_mut().take();
        self.archive_reply.borrow_mut().take();
    }
}

/// The test persistence port. Saves run through one target — the production browser store by
/// default, or in-process memory once a test opts in — behind an optional one-shot behavior
/// that fails or holds the next save.
pub struct TestPersistence {
    target: RefCell<PersistTarget>,
    behavior: GateDriver,
    gate: RefCell<Option<AsyncGate>>,
}

impl TestPersistence {
    fn through_store(store: BrowserStore) -> Self {
        Self {
            target: RefCell::new(PersistTarget::Store(store)),
            behavior: GateDriver::default(),
            gate: RefCell::new(None),
        }
    }

    pub fn use_memory_saves(&self) {
        *self.target.borrow_mut() = PersistTarget::Memory(Rc::new(RefCell::new(BTreeMap::new())));
    }

    pub fn fail_next_save(&self, reason: impl Into<String>) {
        self.behavior.fail_next_save(reason);
    }

    pub fn gate_next_save(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, entered_rx) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        self.behavior.hold_next_save();
        *self.gate.borrow_mut() = Some((entered, release_rx));
        (entered_rx, release)
    }

    fn take_behavior(&self) -> Option<OneShotBehavior> {
        self.behavior.take_save()
    }

    fn memory_saves(&self) -> Option<Rc<RefCell<BTreeMap<String, MemorySave>>>> {
        match &*self.target.borrow() {
            PersistTarget::Memory(saves) => Some(saves.clone()),
            PersistTarget::Store(_) => None,
        }
    }

    pub fn saved_document(&self, project_id: &str) -> Option<ProjectDoc> {
        let saves = self.memory_saves().expect("memory saves are not installed");
        saves
            .borrow()
            .get(project_id)
            .map(|save| save.document.clone())
    }

    /// The asset bytes the completed save kept alongside the document.
    pub fn saved_assets(&self, project_id: &str) -> Option<BTreeMap<String, Vec<u8>>> {
        let saves = self.memory_saves().expect("memory saves are not installed");
        saves
            .borrow()
            .get(project_id)
            .map(|save| save.assets.clone())
    }
}

impl DocumentPersistence for TestPersistence {
    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + 'a>> {
        Box::pin(async move {
            if let Some(behavior) = self.take_behavior() {
                match behavior {
                    OneShotBehavior::Fail(reason) => return Err(reason),
                    OneShotBehavior::Hold => {
                        let Some((entered, release)) = self.gate.borrow_mut().take() else {
                            return Err("save gate was not configured".into());
                        };
                        let _ = entered.send(());
                        let _ = release.await;
                    }
                }
            }
            match &*self.target.borrow() {
                PersistTarget::Store(store) => store
                    .save_document(document, assets)
                    .await
                    .map_err(|error| error.to_string()),
                PersistTarget::Memory(saves) => {
                    saves.borrow_mut().insert(
                        document.id.clone(),
                        MemorySave {
                            document: document.clone(),
                            assets: assets.clone(),
                        },
                    );
                    Ok(())
                }
            }
        })
    }
}

/// A final artifact delivered through the installed test adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveredArtifact {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub media_type: Option<String>,
}

struct StoreReadGate {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

/// The installed in-process adapters: the current in-process Core (replaced whenever the
/// runtime restarts its executor) and the gated persistence port. Tests reach the gates and
/// the saved copies through this handle.
pub struct InProcessAdapters {
    core: RefCell<Rc<InProcessCore>>,
    persistence: Rc<TestPersistence>,
    deliveries: RefCell<Vec<DeliveredArtifact>>,
    store_read_gate: RefCell<Option<StoreReadGate>>,
    open_observer: RefCell<Option<oneshot::Sender<()>>>,
}

impl InProcessAdapters {
    pub fn new(engine: boardstudio_core::CoreEngine, store: BrowserStore) -> Self {
        Self {
            core: RefCell::new(Rc::new(InProcessCore::new(engine))),
            persistence: Rc::new(TestPersistence::through_store(store)),
            deliveries: RefCell::new(Vec::new()),
            store_read_gate: RefCell::new(None),
            open_observer: RefCell::new(None),
        }
    }

    /// Take over the runtime's Core executor, persistence port and executor factory. Saving
    /// stays on the production browser store until `use_memory_saves` is called.
    pub fn attach(self: &Rc<Self>, runtime: &Runtime) {
        *runtime.core.borrow_mut() = self.current_core();
        *runtime.persistence.borrow_mut() = self.persistence.clone();
        let adapters = self.clone();
        *runtime.core_executor_factory.borrow_mut() = Rc::new(move || {
            adapters.restart_core();
            Ok(adapters.current_core() as Rc<dyn CoreExecutor>)
        });
    }

    pub fn gate_next_store_read(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, entered_rx) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        *self.store_read_gate.borrow_mut() = Some(StoreReadGate {
            entered,
            release: release_rx,
        });
        (entered_rx, release)
    }

    /// Pause observation of a real store result without replacing it.
    pub(super) async fn store_read_completed(&self) {
        let gate = self.store_read_gate.borrow_mut().take();
        if let Some(gate) = gate {
            let _ = gate.entered.send(());
            let _ = gate.release.await;
        }
    }

    pub fn observe_next_open(&self) -> oneshot::Receiver<()> {
        let (done, observer) = oneshot::channel();
        *self.open_observer.borrow_mut() = Some(done);
        observer
    }

    pub(super) fn take_open_observer(&self) -> Option<oneshot::Sender<()>> {
        self.open_observer.borrow_mut().take()
    }

    pub(super) fn deliver(&self, artifact: &Artifact) {
        self.deliveries.borrow_mut().push(DeliveredArtifact {
            bytes: artifact.bytes.clone(),
            filename: artifact.filename.clone(),
            media_type: artifact.media_type.clone(),
        });
    }

    pub fn take_deliveries(&self) -> Vec<DeliveredArtifact> {
        std::mem::take(&mut *self.deliveries.borrow_mut())
    }

    pub fn current_core(&self) -> Rc<InProcessCore> {
        self.core.borrow().clone()
    }

    fn restart_core(&self) {
        *self.core.borrow_mut() = Rc::new(InProcessCore::new(boardstudio_core::CoreEngine::new()));
    }

    pub fn use_memory_saves(&self) {
        self.persistence.use_memory_saves();
    }

    pub fn persistence(&self) -> Rc<TestPersistence> {
        self.persistence.clone()
    }
}

/// The one-board document `opened_session` opens. Browser tests that need a real accepted
/// board project start from it, add what they exercise, and open it through the in-process
/// adapter (`project_name_test_support::open_document`).
pub fn board_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("zmk-export-test", "ZMK export test");
    document.boards.push(Board {
        id: "main-board".into(),
        name: "Main board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    document
}

pub fn opened_session() -> (Session, AcceptedSnapshot, Scope) {
    let mut session = Session::new();
    let mut core = boardstudio_core::CoreEngine::new();
    let mut effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: board_document(),
    });
    while let Some(effect) = effects.pop() {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => effects.extend(session.complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(core.handle(*request)),
            })),
            Effect::Persist {
                save_attempt_id, ..
            } => {
                effects.extend(session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                }));
            }
            _ => {}
        }
    }
    let accepted = session.read_model().accepted.clone().unwrap();
    let scope = session.scope().unwrap();
    (session, accepted, scope)
}
