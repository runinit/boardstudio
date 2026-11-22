use super::*;
use boardstudio_application::{EditResolver, Resolution};

pub trait InstalledAdapters: std::any::Any {
    fn deliver(&self, bytes: &[u8], filename: &str, media_type: Option<&str>);
    fn take_open_observer(&self) -> Option<futures_channel::oneshot::Sender<()>>;
    fn store_read_completed(&self) -> Pin<Box<dyn Future<Output = ()> + '_>>;
}

pub fn installed(runtime: &Runtime) -> Rc<dyn InstalledAdapters> {
    runtime
        .in_process_adapters
        .borrow()
        .clone()
        .expect("in-process adapters are installed")
}

pub fn new_runtime(services: CoreServices) -> Result<Rc<Runtime>, String> {
    Runtime::new_with_restoration(false, services)
}

pub fn store(runtime: &Runtime) -> BrowserStore {
    runtime.store.clone()
}
pub fn clear_saved_tokens(runtime: &Runtime) {
    runtime.saved_tokens.borrow_mut().clear();
}
pub fn has_artifacts(runtime: &Runtime) -> bool {
    !runtime.artifacts.borrow().is_empty()
}

pub fn install(
    runtime: &Runtime,
    session: Session,
    adapters: Rc<dyn InstalledAdapters>,
    persistence: Rc<dyn DocumentPersistence>,
) {
    runtime.saved_tokens.borrow_mut().clear();
    if let Some(accepted) = session.read_model().accepted.as_ref()
        && let Some(token) = runtime.store.observed_token(&accepted.document.id)
    {
        runtime
            .saved_tokens
            .borrow_mut()
            .insert(accepted.document.id.clone(), token);
    }
    *runtime.session.borrow_mut() = session;
    *runtime.persistence.borrow_mut() = persistence;
    *runtime.in_process_adapters.borrow_mut() = Some(adapters);
    runtime.changed();
}

pub fn set_storage_initializing(runtime: &Runtime, initializing: bool) {
    runtime.storage_initializing.set(initializing);
    runtime.changed();
}

pub fn fail_storage_initialization(runtime: &Runtime, error: &str) {
    runtime.storage_initializing.set(true);
    runtime.storage_initialization_failed(error);
}

/// Commit `document` as one `ReplaceDocument` edit through the real Session and Core,
/// driving every pending effect, and return the snapshot it was accepted as.
pub async fn replace_document(
    runtime: &Rc<Runtime>,
    transaction_id: &str,
    target_id: &str,
    document: ProjectDoc,
) -> AcceptedSnapshot {
    let base_revision = runtime
        .model()
        .accepted
        .expect("an accepted document is open")
        .document
        .revision;
    submit_fixed_command(
        runtime,
        boardstudio_model::model::EditCommand {
            base_revision,
            transaction_id: transaction_id.to_owned(),
            phase: boardstudio_model::model::EditPhase::Commit,
            target_ids: vec![target_id.to_owned()],
            operation: boardstudio_model::model::EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    );
    run_pending(runtime).await;
    runtime
        .model()
        .accepted
        .expect("the edited document stays accepted")
}

/// Submit a fixed command through the same resolver route as a production commit.
pub fn submit_fixed_command(runtime: &Rc<Runtime>, command: boardstudio_model::model::EditCommand) {
    runtime.submit(fixed_command_event(runtime.operation(), command));
}

/// Build a resolver event for tests that need to submit through their own observer.
pub fn fixed_command_event(
    operation_id: OperationId,
    command: boardstudio_model::model::EditCommand,
) -> Event {
    let boardstudio_model::model::EditCommand {
        transaction_id,
        target_ids,
        operation,
        ..
    } = command;
    Event::ResolveEdit {
        operation_id,
        label: "test fixed command".into(),
        resolver: EditResolver::new("test fixed command", move |_| {
            if transaction_id.is_empty() {
                Resolution::submit(target_ids.clone(), operation.clone())
            } else {
                Resolution::submit_with_transaction_id(
                    target_ids.clone(),
                    transaction_id.clone(),
                    operation.clone(),
                )
            }
        }),
    }
}

/// Make `board_id` the active board through the real Session.
pub async fn navigate(runtime: &Rc<Runtime>, board_id: &str) {
    runtime.submit(Event::Navigate {
        operation_id: runtime.operation(),
        board_id: board_id.to_owned(),
        instance_id: None,
    });
    run_pending(runtime).await;
}

/// The effects Session asked Runtime to run that no test has driven yet. Generation and
/// export jobs are outside the adapter, so tests read these to see what Session started
/// (an empty list means nothing was submitted). Taking them also keeps a later
/// `run_pending` from starting the job. Effects are only held once the adapter is
/// installed, so a runtime without it is a test mistake, not an empty answer.
pub fn take_held_effects(runtime: &Runtime) -> Vec<Effect> {
    installed(runtime);
    std::mem::take(&mut *runtime.held_effects.borrow_mut())
}

/// Deliver a completion to Session as the executor that ran the effect would have. Tests
/// use it for the generation jobs the adapter does not run; the effects it produces are
/// held like those of a submitted event.
pub fn deliver_completion(runtime: &Rc<Runtime>, completion: Completion) {
    installed(runtime);
    let effects = runtime.complete(completion);
    runtime.held_effects.borrow_mut().extend(effects);
}

pub fn replace_session(runtime: &Runtime, session: Session) {
    *runtime.session.borrow_mut() = session;
    runtime.changed();
}

pub fn observe(
    runtime: &Runtime,
    operation: OperationId,
) -> crate::operation_outcomes::OutcomeSlot {
    runtime.operation_outcomes.observe(operation)
}

pub fn observe_next(runtime: &Runtime) -> crate::operation_outcomes::OutcomeSlot {
    runtime
        .operation_outcomes
        .observe(OperationId(runtime.next_operation.get()))
}

/// Drive one real effect, returning its continuations so a test can pause between them.
pub async fn run_effect(runtime: &Rc<Runtime>, effect: Effect) -> Vec<Effect> {
    runtime.run(effect).await
}

pub async fn run_pending(runtime: &Rc<Runtime>) {
    loop {
        let pending = std::mem::take(&mut *runtime.held_effects.borrow_mut());
        if !pending.is_empty() {
            // Keep Runtime effects off the mounted test's WASM poll stack too.
            runtime.drive(pending);
        }
        let drives = std::mem::take(&mut *runtime.in_process_drives.borrow_mut());
        if drives.is_empty() {
            break;
        }
        for finished in drives {
            finished
                .await
                .expect("in-process effect driver was dropped");
        }
    }
}

pub fn drive_pending(runtime: &Rc<Runtime>) {
    let effects = std::mem::take(&mut *runtime.held_effects.borrow_mut());
    runtime.drive(effects);
}
