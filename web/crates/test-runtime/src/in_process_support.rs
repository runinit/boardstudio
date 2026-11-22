//! In-process Core and persistence adapters for mounted browser tests. The adapter serves
//! Session Core requests through a real `CoreEngine`, serves archives and artifacts through
//! the same Core crate functions the worker uses, and saves documents either through the
//! production browser-store port or into memory. Each port accepts gates that hold or fail
//! the next reply or save, so tests control completion timing without feature-named branches
//! in Runtime itself.

use crate::gate_driver::{GateDriver, OneShotBehavior};
use crate::runtime::{DocumentPersistence, test_hooks::InstalledAdapters};
use boardstudio_application::{
    AcceptedSnapshot, Completion, Effect, Event, OperationId, SaveResult, Scope, Session,
};
use boardstudio_model::model::{Board, ProjectDoc};
use boardstudio_web_host::host::{BrowserStore, SaveError, StorageToken};
use futures_channel::oneshot;
use std::{cell::RefCell, collections::BTreeMap, future::Future, pin::Pin, rc::Rc};

type AsyncGate = (oneshot::Sender<()>, oneshot::Receiver<()>);

#[derive(Clone)]
enum PersistTarget {
    Store(BrowserStore),
    Memory(Rc<RefCell<BTreeMap<String, MemorySave>>>),
}

#[derive(Clone)]
struct MemorySave {
    document: ProjectDoc,
    token: StorageToken,
    assets: BTreeMap<String, Vec<u8>>,
}

/// The test persistence port. Saves run through one target — the production browser store by
/// default, or in-process memory once a test opts in — behind an optional one-shot behavior
/// that fails or holds the next save.
pub struct TestPersistence {
    target: RefCell<PersistTarget>,
    observed: RefCell<BTreeMap<String, StorageToken>>,
    target_observer: RefCell<Option<oneshot::Sender<()>>>,
    behavior: GateDriver,
    gate: RefCell<Option<AsyncGate>>,
}

impl TestPersistence {
    fn through_store(store: BrowserStore) -> Self {
        Self {
            target: RefCell::new(PersistTarget::Store(store)),
            observed: RefCell::default(),
            target_observer: RefCell::new(None),
            behavior: GateDriver::default(),
            gate: RefCell::new(None),
        }
    }

    pub fn use_memory_saves(&self) {
        self.observed.borrow_mut().clear();
        *self.target.borrow_mut() = PersistTarget::Memory(Rc::new(RefCell::new(BTreeMap::new())));
    }

    pub(super) fn observe_next_target(&self) -> oneshot::Receiver<()> {
        let (entered, receive) = oneshot::channel();
        *self.target_observer.borrow_mut() = Some(entered);
        receive
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
    fn observed_token(&self, id: &str) -> Option<StorageToken> {
        match &*self.target.borrow() {
            PersistTarget::Store(store) => store.observed_token(id),
            PersistTarget::Memory(_) => self.observed.borrow().get(id).cloned(),
        }
    }

    fn observe_token(&self, id: String, token: StorageToken) {
        match &*self.target.borrow() {
            PersistTarget::Store(store) => store.observe_token(id, token),
            PersistTarget::Memory(_) => { self.observed.borrow_mut().insert(id, token); }
        }
    }

    fn save_document<'a>(
        &'a self,
        document: &'a ProjectDoc,
        assets: &'a BTreeMap<String, Vec<u8>>,
        expected: Option<&'a StorageToken>,
    ) -> Pin<Box<dyn Future<Output = Result<StorageToken, SaveError>> + 'a>> {
        Box::pin(async move {
            if let Some(behavior) = self.take_behavior() {
                match behavior {
                    OneShotBehavior::Fail(reason) => return Err(SaveError::Storage(reason)),
                    OneShotBehavior::Hold => {
                        let Some((entered, release)) = self.gate.borrow_mut().take() else {
                            return Err(SaveError::Storage("save gate was not configured".into()));
                        };
                        let _ = entered.send(());
                        let _ = release.await;
                    }
                }
            }
            let target = self.target.borrow().clone();
            if let Some(entered) = self.target_observer.borrow_mut().take() {
                let _ = entered.send(());
            }
            match target {
                PersistTarget::Store(store) => {
                    store
                        .save_document_expected(document, assets, expected)
                        .await
                }
                PersistTarget::Memory(saves) => {
                    let actual = saves
                        .borrow()
                        .get(&document.id)
                        .map(|save| save.token.clone());
                    if actual.as_ref() != expected {
                        return Err(SaveError::Conflict);
                    }
                    let token = StorageToken::for_document(document)?;
                    saves.borrow_mut().insert(
                        document.id.clone(),
                        MemorySave {
                            document: document.clone(),
                            token: token.clone(),
                            assets: assets.clone(),
                        },
                    );
                    self.observe_token(document.id.clone(), token.clone());
                    Ok(token)
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
    persistence: Rc<TestPersistence>,
    deliveries: RefCell<Vec<DeliveredArtifact>>,
    store_read_gate: RefCell<Option<StoreReadGate>>,
    open_observer: RefCell<Option<oneshot::Sender<()>>>,
}

impl InProcessAdapters {
    pub fn new(store: BrowserStore) -> Self {
        Self {
            persistence: Rc::new(TestPersistence::through_store(store)),
            deliveries: RefCell::new(Vec::new()),
            store_read_gate: RefCell::new(None),
            open_observer: RefCell::new(None),
        }
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

    pub fn observe_next_open(&self) -> oneshot::Receiver<()> {
        let (done, observer) = oneshot::channel();
        *self.open_observer.borrow_mut() = Some(done);
        observer
    }

    pub fn take_deliveries(&self) -> Vec<DeliveredArtifact> {
        std::mem::take(&mut *self.deliveries.borrow_mut())
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
    let mut session = Session::new(boardstudio_core::CoreEngine::validate_open);
    let mut effects = session.submit(Event::Open {
        operation_id: OperationId(1),
        document: board_document(),
    });
    while let Some(effect) = effects.pop() {
        if let Effect::Persist {
            save_attempt_id, ..
        } = effect
        {
            effects.extend(session.complete(Completion::Persist {
                save_attempt_id,
                result: SaveResult::Committed,
            }));
        }
    }
    let accepted = session.read_model().accepted.clone().unwrap();
    let scope = session.scope().unwrap();
    (session, accepted, scope)
}

impl InstalledAdapters for InProcessAdapters {
    /// Pause observation of a real store result without replacing it.
    fn store_read_completed(&self) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(async move {
            let gate = self.store_read_gate.borrow_mut().take();
            if let Some(gate) = gate {
                let _ = gate.entered.send(());
                let _ = gate.release.await;
            }
        })
    }

    fn take_open_observer(&self) -> Option<oneshot::Sender<()>> {
        self.open_observer.borrow_mut().take()
    }

    fn deliver(&self, bytes: &[u8], filename: &str, media_type: Option<&str>) {
        self.deliveries.borrow_mut().push(DeliveredArtifact {
            bytes: bytes.to_vec(),
            filename: filename.to_owned(),
            media_type: media_type.map(str::to_owned),
        });
    }
}
