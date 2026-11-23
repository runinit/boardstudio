use super::in_process_support::InProcessAdapters;
pub use crate::runtime::test_hooks::{
    deliver_completion, drive_pending, fail_storage_initialization, fixed_command_event, navigate,
    observe, observe_next, replace_document, replace_session, run_effect, run_pending,
    set_storage_initializing, submit_fixed_command, take_held_effects,
};
use crate::runtime::{Runtime, test_hooks};
use boardstudio_application::{Event, Session};
use boardstudio_model::model::ProjectDoc;
use std::rc::Rc;

pub fn new_runtime() -> Rc<Runtime> {
    test_hooks::new_runtime().expect("browser runtime fixture initializes")
}

/// Replace the runtime's session and install in-process adapters.
/// Saves keep running through the production browser-store port (gated by
/// `gate_next_persist` / `fail_next_persist`); call `install_memory_persistence` to save
/// into memory instead.
pub fn install(runtime: &Runtime, session: Session) {
    let adapters = Rc::new(InProcessAdapters::new(test_hooks::store(runtime)));
    let persistence = adapters.persistence();
    test_hooks::install(runtime, session, adapters, persistence);
}

/// Route saves into the in-process memory store. Requires `install` to have run.
pub fn install_memory_persistence(runtime: &Runtime) {
    installed(runtime).use_memory_saves();
}

/// Install the in-process adapters with memory saves and open `document` through the real
/// Session and Core, leaving it accepted, saved and ready. Browser tests that need an
/// opened project start here instead of giving Runtime a fabricated read model.
pub async fn open_document(runtime: &Rc<Runtime>, document: ProjectDoc) {
    install(runtime, Session::new());
    install_memory_persistence(runtime);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document,
    });
    run_pending(runtime).await;
}

pub(super) fn installed(runtime: &Runtime) -> Rc<InProcessAdapters> {
    let adapters: Rc<dyn std::any::Any> = test_hooks::installed(runtime);
    adapters.downcast().expect("in-process adapter type")
}

pub fn gate_open_saved_result(
    runtime: &Runtime,
) -> (
    futures_channel::oneshot::Receiver<()>,
    futures_channel::oneshot::Sender<()>,
) {
    installed(runtime).gate_next_store_read()
}

pub fn track_import_file(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
    installed(runtime).observe_next_open()
}

pub fn track_open_saved(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
    installed(runtime).observe_next_open()
}

pub fn track_next_persist_target(runtime: &Runtime) -> futures_channel::oneshot::Receiver<()> {
    installed(runtime).persistence().observe_next_target()
}

pub fn fail_next_persist(runtime: &Runtime, reason: impl Into<String>) {
    installed(runtime).persistence().fail_next_save(reason);
}

pub fn gate_next_persist(
    runtime: &Runtime,
) -> (
    futures_channel::oneshot::Receiver<()>,
    futures_channel::oneshot::Sender<()>,
) {
    installed(runtime).persistence().gate_next_save()
}

/// The copy a completed save left in the in-process memory store.
pub fn saved_document(runtime: &Runtime, project_id: &str) -> Option<ProjectDoc> {
    installed(runtime).persistence().saved_document(project_id)
}
