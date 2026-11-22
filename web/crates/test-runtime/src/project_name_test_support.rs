use super::in_process_support::InProcessAdapters;
pub use crate::runtime::test_hooks::{
    deliver_completion, drive_pending, fail_storage_initialization, fixed_command_event, navigate,
    observe, observe_next, replace_document, replace_session, run_effect, run_pending,
    set_storage_initializing, submit_fixed_command, take_held_effects,
};
use crate::runtime::{Runtime, test_hooks};
use boardstudio_application::{Event, Session};
use boardstudio_model::model::{ArchiveReply, ProjectDoc};
use std::rc::Rc;

pub fn new_runtime() -> Rc<Runtime> {
    test_hooks::new_runtime(super::engine_core_services())
        .expect("browser runtime fixture initializes")
}

/// Replace the runtime's session and route Core execution through an in-process
/// `CoreEngine`. Saves keep running through the production browser-store port (gated by
/// `gate_next_persist` / `fail_next_persist`); call `install_memory_persistence` to save
/// into memory instead.
pub fn install(runtime: &Runtime, session: Session, core: boardstudio_core::CoreEngine) {
    let adapters = Rc::new(InProcessAdapters::new(core, test_hooks::store(runtime)));
    let current = adapters.current_core();
    let persistence = adapters.persistence();
    test_hooks::install(runtime, session, adapters, current, persistence);
}

/// Route saves into the in-process memory store. Requires `install` to have run.
pub fn install_memory_persistence(runtime: &Runtime) {
    installed(runtime).use_memory_saves();
    test_hooks::clear_saved_tokens(runtime);
}

/// Install the in-process adapters with memory saves and open `document` through the real
/// Session and Core, leaving it accepted, saved and ready. Browser tests that need an
/// opened project start here instead of giving Runtime a fabricated read model.
pub async fn open_document(runtime: &Rc<Runtime>, document: ProjectDoc) {
    install(
        runtime,
        Session::new(boardstudio_core::CoreEngine::validate_open),
        boardstudio_core::CoreEngine::new(),
    );
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

pub fn gate_import_archive(
    runtime: &Runtime,
    document: &ProjectDoc,
) -> (
    futures_channel::oneshot::Receiver<()>,
    futures_channel::oneshot::Sender<()>,
) {
    let core = installed(runtime).current_core();
    let reply = ArchiveReply::Unpacked {
        project_json: serde_json::to_string(document).expect("fixture document serializes"),
        assets: Vec::new(),
    };
    core.reply_to_next_archive(boardstudio_web_host::host::ArchiveResult {
        metadata: serde_json::to_string(&reply).expect("archive reply serializes"),
        buffers: Vec::new(),
    });
    core.gate_next_archive()
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

/// Fail the next Core reply with the supplied reason.
pub fn fail_next_core_reply(runtime: &Runtime, reason: impl Into<String>) {
    installed(runtime).current_core().fail_next_reply(reason);
}

/// Hold the next Core reply until the returned release sender fires. The entered
/// receiver observes the request reaching the executor.
pub fn gate_next_core_reply(
    runtime: &Runtime,
) -> (
    futures_channel::oneshot::Receiver<()>,
    futures_channel::oneshot::Sender<()>,
) {
    installed(runtime).current_core().gate_next_reply()
}

pub fn gate_next_core_reply_then_fail(
    runtime: &Runtime,
    _reason: impl Into<String>,
) -> (
    futures_channel::oneshot::Receiver<()>,
    futures_channel::oneshot::Sender<()>,
) {
    installed(runtime)
        .current_core()
        .gate_next_reply_then_fail(_reason)
}

/// The copy a completed save left in the in-process memory store.
pub fn saved_document(runtime: &Runtime, project_id: &str) -> Option<ProjectDoc> {
    installed(runtime).persistence().saved_document(project_id)
}

/// The in-process Core executor currently installed on the runtime.
pub fn in_process_core(runtime: &Runtime) -> Rc<super::in_process_support::InProcessCore> {
    installed(runtime).current_core()
}
