//! In-process Core and persistence adapters for mounted browser tests. The adapter serves
//! Session Core requests through a real `CoreEngine`, serves archives and artifacts through
//! the same Core crate functions the worker uses, and saves documents either through the
//! production browser-store port or into memory. Each port accepts gates that hold or fail
//! the next reply or save, so tests control completion timing without feature-named branches
//! in Runtime itself.

use super::*;
use boardstudio_core::{archive as core_archive, artifact_request};
use boardstudio_web_host::host::{ArchiveResult, CoreExecutorFuture, HostError};
use futures_channel::oneshot;

enum CoreReplyBehavior {
    Fail(String),
    Gate {
        entered: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    },
}

enum PersistTestBehavior {
    Fail(String),
    Gate {
        entered: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    },
}

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
    behavior: RefCell<Option<CoreReplyBehavior>>,
}

impl InProcessCore {
    pub fn new(engine: boardstudio_core::CoreEngine) -> Self {
        Self {
            engine: RefCell::new(engine),
            closed: Cell::new(false),
            behavior: RefCell::new(None),
        }
    }

    pub fn fail_next_reply(&self, reason: impl Into<String>) {
        *self.behavior.borrow_mut() = Some(CoreReplyBehavior::Fail(reason.into()));
    }

    pub fn is_closed(&self) -> bool {
        self.closed.get()
    }

    pub fn gate_next_reply(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, entered_rx) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        *self.behavior.borrow_mut() = Some(CoreReplyBehavior::Gate { entered, release: release_rx });
        (entered_rx, release)
    }

    fn take_behavior(&self) -> Option<CoreReplyBehavior> {
        self.behavior.borrow_mut().take()
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
            if let Some(behavior) = self.take_behavior() {
                match behavior {
                    CoreReplyBehavior::Fail(reason) => return Err(HostError(reason)),
                    CoreReplyBehavior::Gate { entered, release } => {
                        let _ = entered.send(());
                        let _ = release.await;
                    }
                }
            }
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            let reply = self.engine.borrow_mut().handle(request.clone());
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
        request_id: &'a str,
        _executor_epoch: &'a str,
        metadata: &'a str,
        buffers: Vec<Uint8Array>,
    ) -> CoreExecutorFuture<'a, ArchiveResult> {
        Box::pin(async move {
            if self.closed.get() {
                return Err(HostError("core worker is closed".into()));
            }
            let inputs = buffers.iter().map(|buffer| buffer.to_vec()).collect::<Vec<_>>();
            let (reply, output_bytes) = core_archive::request(metadata, &inputs);
            let _ = request_id;
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
            let frame = serde_json::to_string(request)
                .map_err(|error| HostError(format!("could not encode artifact request: {error}")))?;
            let frame_id = serde_json::from_str::<serde_json::Value>(&frame)
                .ok()
                .and_then(|value| value.get("id").and_then(serde_json::Value::as_str).map(str::to_owned));
            if frame_id.as_deref() != Some(request_id) {
                return Err(HostError(
                    "artifact request id does not match worker request id".into(),
                ));
            }
            let reply_json = artifact_request(&frame);
            let reply: ArtifactReply = serde_json::from_str(&reply_json)
                .map_err(|error| HostError(format!("invalid artifact reply: {error}")))?;
            let value: serde_json::Value = serde_json::from_str(&reply_json)
                .map_err(|error| HostError(format!("invalid artifact reply identity: {error}")))?;
            if value.get("id").and_then(serde_json::Value::as_str) != Some(request_id) {
                return Err(HostError("artifact reply id does not match request".into()));
            }
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
    }
}

/// The test persistence port. Saves run through one target — the production browser store by
/// default, or in-process memory once a test opts in — behind an optional one-shot behavior
/// that fails or holds the next save.
pub struct TestPersistence {
    target: RefCell<PersistTarget>,
    behavior: RefCell<Option<PersistTestBehavior>>,
}

impl TestPersistence {
    fn through_store(store: BrowserStore) -> Self {
        Self {
            target: RefCell::new(PersistTarget::Store(store)),
            behavior: RefCell::new(None),
        }
    }

    pub fn use_memory_saves(&self) {
        *self.target.borrow_mut() = PersistTarget::Memory(Rc::new(RefCell::new(BTreeMap::new())));
    }

    pub fn fail_next_save(&self, reason: impl Into<String>) {
        *self.behavior.borrow_mut() = Some(PersistTestBehavior::Fail(reason.into()));
    }

    pub fn gate_next_save(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered, entered_rx) = oneshot::channel();
        let (release, release_rx) = oneshot::channel();
        *self.behavior.borrow_mut() = Some(PersistTestBehavior::Gate { entered, release: release_rx });
        (entered_rx, release)
    }

    fn take_behavior(&self) -> Option<PersistTestBehavior> {
        self.behavior.borrow_mut().take()
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
                    PersistTestBehavior::Fail(reason) => return Err(reason),
                    PersistTestBehavior::Gate { entered, release } => {
                        let _ = entered.send(());
                        let _ = release.await;
                    }
                }
            }
            match &*self.target.borrow() {
                PersistTarget::Store(store) => {
                    store
                        .save_document(document, assets)
                        .await
                        .map_err(|error| error.to_string())
                }
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

/// The installed in-process adapters: the current in-process Core (replaced whenever the
/// runtime restarts its executor) and the gated persistence port. Tests reach the gates and
/// the saved copies through this handle.
pub struct InProcessAdapters {
    core: RefCell<Rc<InProcessCore>>,
    persistence: Rc<TestPersistence>,
}

impl InProcessAdapters {
    pub fn new(engine: boardstudio_core::CoreEngine, store: BrowserStore) -> Self {
        Self {
            core: RefCell::new(Rc::new(InProcessCore::new(engine))),
            persistence: Rc::new(TestPersistence::through_store(store)),
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

    pub fn current_core(&self) -> Rc<InProcessCore> {
        self.core.borrow().clone()
    }

    fn restart_core(&self) {
        *self.core.borrow_mut() =
            Rc::new(InProcessCore::new(boardstudio_core::CoreEngine::new()));
    }

    pub fn use_memory_saves(&self) {
        self.persistence.use_memory_saves();
    }

    pub fn persistence(&self) -> Rc<TestPersistence> {
        self.persistence.clone()
    }
}
