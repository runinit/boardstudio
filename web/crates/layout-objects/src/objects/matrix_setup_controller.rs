//! Editor-lifetime owner for Matrix Setup preparation, submission, and exact acknowledgement.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
use self::native_detached::spawn_local;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
use self::native_setup_types::{
    MatrixSetupCreateRequest, MatrixSetupMount, MatrixSetupOwner, MatrixSetupProjection,
};
use super::matrix_preparation::{PendingSettlement, matrix_source, prepare_matrix_definitions};
#[cfg(target_arch = "wasm32")]
use super::matrix_setup::{
    MatrixSetupCreateRequest, MatrixSetupMount, MatrixSetupOwner, MatrixSetupProjection,
};
use super::{ScopedTreeContext, TreeContext};
use crate::{
    matrix_setup_operation::{MatrixSetupRequest, next_matrix_id},
    runtime::Runtime,
};
use boardstudio_application::{Event, Scope, SelectionMode};
use boardstudio_web_runtime::operation_outcomes::Settlement;
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// Mounted-owner lifecycle types for native tests. The wasm presentation module owns these
/// shapes in production; this mirror keeps the hook's native compilation free of DOM imports.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub(crate) mod native_setup_types {
    use boardstudio_application::{Scope, SnapshotToken};
    use dioxus::prelude::*;

    use crate::matrix_setup_operation::MatrixSetupPreset;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MatrixSetupOwner {
        pub editor_instance_id: u64,
        pub open_id: u64,
        pub scope_generation: u64,
        pub scope: Scope,
        pub board_id: String,
        pub snapshot_token: SnapshotToken,
        pub revision: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MatrixSetupProjection {
        pub owner: MatrixSetupOwner,
        pub editable: bool,
        pub can_cancel: bool,
        pub status: Option<String>,
        pub error: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MatrixSetupCreateRequest {
        pub owner: MatrixSetupOwner,
        pub rows: u32,
        pub columns: u32,
        pub preset: MatrixSetupPreset,
    }

    #[derive(Clone, PartialEq)]
    pub struct MatrixSetupMount {
        pub projection: Option<MatrixSetupProjection>,
        pub can_open: bool,
        pub on_open: EventHandler<()>,
        pub on_cancel: EventHandler<MatrixSetupOwner>,
        pub on_create: EventHandler<MatrixSetupCreateRequest>,
    }
}

/// Detached-task queue for native lifecycle tests, mirroring the browser microtask queue.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod native_detached {
    use std::{
        cell::RefCell,
        future::Future,
        pin::Pin,
        task::{Context, Waker},
    };

    thread_local! {
        static TASKS: RefCell<Vec<Pin<Box<dyn Future<Output = ()>>>>> = RefCell::default();
    }

    pub fn spawn_local(task: impl Future<Output = ()> + 'static) {
        TASKS.with_borrow_mut(|tasks| tasks.push(Box::pin(task)));
    }

    pub fn poll_detached() {
        TASKS.with_borrow_mut(|tasks| {
            tasks.retain_mut(|task| {
                task.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            });
        });
    }

    pub fn clear_detached() {
        TASKS.with_borrow_mut(Vec::clear);
    }
}

use super::matrix_setup_logic::create_matrix_resolver;

/// Called once for the Editor lifetime. The form may be hidden while its admitted SetMatrix
/// completes; that operation always settles against its captured scope and never retargets.
pub fn use_matrix_setup(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    anchor_scope: Signal<Option<Scope>>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
) -> MatrixSetupMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let open_id = use_signal(|| 0u64);
    let open = use_signal(|| None::<MatrixSetupOwner>);
    let preparing = use_signal(|| None::<MatrixSetupOwner>);
    let error = use_signal(|| None::<String>);
    let status = use_signal(|| None::<String>);
    // The committed SetMatrix is observed through its ticket; the waiting entry is the
    // pending state. No separate disabled flag is kept: availability derives from it.
    let waiting = use_signal(|| None::<PendingSettlement<(MatrixSetupOwner, String)>>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut waiting = waiting;
            let mut selected_context = selected_context;
            let mut anchor_scope = anchor_scope;
            move |(_, workspace, scope_generation)| {
                settle_pending(
                    &runtime,
                    workspace,
                    scope_generation,
                    &mut SetupSettlement {
                        waiting: &mut waiting,
                        open: &mut open,
                        error: &mut error,
                        status: &mut status,
                        selected_context: &mut selected_context,
                        anchor_scope: &mut anchor_scope,
                    },
                );
                if waiting.read().is_none()
                    && open.read().as_ref().is_some_and(|owner| {
                        workspace != Workspace::Layout
                            || owner.scope_generation != scope_generation
                            || runtime.scope().as_ref() != Some(&owner.scope)
                    })
                {
                    open.set(None);
                    preparing.set(None);
                    error.set(None);
                    status.set(None);
                }
            }
        },
    ));

    let on_open = use_callback({
        let runtime = runtime.clone();
        let mut open = open;
        let mut open_id = open_id;
        let mut error = error;
        let mut status = status;
        move |_| {
            if waiting.read().is_some() || preparing.read().is_some() {
                return;
            }
            let current_workspace = workspace();
            let current_generation = scope_generation();
            let model = runtime.model();
            let scope = runtime.scope();
            let Some((snapshot, scope, board_id)) =
                matrix_source(&runtime, &model, scope.as_ref(), current_workspace)
            else {
                error.set(Some(
                    "Open a saved, editable board in Layout before creating a matrix.".into(),
                ));
                return;
            };
            let id = open_id()
                .checked_add(1)
                .expect("matrix setup identity exhausted");
            open_id.set(id);
            error.set(None);
            status.set(None);
            open.set(Some(MatrixSetupOwner {
                editor_instance_id,
                open_id: id,
                scope_generation: current_generation,
                scope,
                board_id,
                snapshot_token: snapshot.token,
                revision: snapshot.document.revision,
            }));
        }
    });

    let on_cancel = use_callback({
        let mut open = open;
        let mut error = error;
        let mut status = status;
        let mut preparing = preparing;
        move |owner: MatrixSetupOwner| {
            if open.read().as_ref() == Some(&owner) && waiting.read().is_none() {
                open.set(None);
                preparing.set(None);
                error.set(None);
                status.set(None);
            }
        }
    });

    let on_create = use_callback({
        let runtime = runtime.clone();
        let alive = alive.clone();
        let mut preparing = preparing;
        let mut error = error;
        let mut status = status;
        move |request: MatrixSetupCreateRequest| {
            if open.read().as_ref() != Some(&request.owner)
                || waiting.read().is_some()
                || preparing.read().is_some()
                || request.owner.editor_instance_id != editor_instance_id
                || workspace() != Workspace::Layout
                || scope_generation() != request.owner.scope_generation
            {
                return;
            }
            if request.rows == 0
                || request.columns == 0
                || request
                    .rows
                    .checked_mul(request.columns)
                    .is_none_or(|cells| cells > 4096)
            {
                error.set(Some(
                    "Rows and columns must be positive, with at most 4096 cells.".into(),
                ));
                return;
            }
            let model = runtime.model();
            let live_scope = runtime.scope();
            let Some((snapshot, scope, board_id)) =
                matrix_source(&runtime, &model, live_scope.as_ref(), workspace())
            else {
                error.set(Some(
                    "The selected board is no longer ready for matrix creation.".into(),
                ));
                return;
            };
            if scope != request.owner.scope
                || board_id != request.owner.board_id
                || snapshot.token != request.owner.snapshot_token
                || snapshot.document.revision != request.owner.revision
            {
                error.set(Some("The accepted board changed. Close Matrix Setup and reopen it before creating a matrix.".into()));
                return;
            }
            let owner = request.owner.clone();
            let accepted = snapshot.clone();
            let reversible = accepted
                .document
                .parameters
                .get("reversibleLayout")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let matrix_id = next_matrix_id(
                &accepted.document.matrices,
                &accepted.document.definitions,
                &accepted.document.parts,
                request.preset.as_str(),
                reversible,
            );
            preparing.set(Some(owner.clone()));
            error.set(None);
            status.set(Some("Preparing matrix definitions…".into()));
            let runtime = runtime.clone();
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut waiting = waiting;
            let mut open = open;
            let alive = alive.clone();
            spawn_local(async move {
                if !alive.get() {
                    return;
                }
                let result = prepare_matrix_definitions(
                    matrix_id,
                    board_id,
                    MatrixSetupRequest {
                        rows: request.rows,
                        columns: request.columns,
                        preset: request.preset,
                    },
                    reversible,
                    || alive.get(),
                )
                .await;
                if !alive.get() {
                    return;
                }
                if !preparing
                    .read()
                    .as_ref()
                    .is_some_and(|active| active == &owner)
                {
                    return;
                }
                let model = runtime.model();
                let current_scope = runtime.scope();
                let current_workspace = workspace();
                let current_generation = scope_generation();
                let Some((current, scope, current_board)) =
                    matrix_source(&runtime, &model, current_scope.as_ref(), current_workspace)
                else {
                    preparing.set(None);
                    open.set(None);
                    error.set(None);
                    status.set(None);
                    return;
                };
                if current_workspace != Workspace::Layout
                    || current_generation != owner.scope_generation
                    || scope != owner.scope
                    || current_board != owner.board_id
                    || current.token != accepted.token
                    || current.document.revision != accepted.document.revision
                    || open.read().as_ref() != Some(&owner)
                {
                    preparing.set(None);
                    open.set(None);
                    error.set(None);
                    status.set(None);
                    return;
                }
                let (matrix, definitions) = match result {
                    Ok(Some(prepared)) => (prepared.matrix, prepared.definitions),
                    Ok(None) => return,
                    Err(message) => {
                        preparing.set(None);
                        error.set(Some(message));
                        status.set(None);
                        return;
                    }
                };
                if matrix.board_id.as_deref() != Some(current_board.as_str())
                    || definitions.iter().any(|definition| {
                        definition.generator.is_none()
                            || definition.id.is_empty()
                            || current
                                .document
                                .definitions
                                .iter()
                                .any(|existing| existing.id == definition.id)
                    })
                {
                    preparing.set(None);
                    error.set(Some("The prepared matrix definition set is incomplete or conflicts with the accepted document.".into()));
                    status.set(None);
                    return;
                }
                let matrix_id = matrix.id.clone();
                let ticket = runtime.resolve_edit(
                    "layout-matrix-setup",
                    Some("matrix".into()),
                    create_matrix_resolver(current_board.clone(), matrix, definitions),
                );
                waiting.set(Some(PendingSettlement {
                    ticket,
                    owner: (owner, matrix_id),
                }));
                preparing.set(None);
                status.set(None);
            });
        }
    });

    let create_pending = waiting.read().is_some();
    let projection = open.read().clone().map(|owner| {
        let idle = !create_pending && preparing.read().is_none();
        MatrixSetupProjection {
            editable: idle && setup_owner_live(&runtime, &owner, workspace(), scope_generation()),
            can_cancel: !create_pending,
            owner,
            status: status.read().clone(),
            error: error.read().clone(),
        }
    });
    let current_model = runtime.model();
    let can_open = !create_pending
        && preparing.read().is_none()
        && open.read().is_none()
        && matrix_source(
            &runtime,
            &current_model,
            runtime.scope().as_ref(),
            workspace(),
        )
        .is_some();
    MatrixSetupMount {
        projection,
        can_open,
        on_open,
        on_cancel,
        on_create,
    }
}

fn setup_owner_live(
    runtime: &Runtime,
    owner: &MatrixSetupOwner,
    workspace: Workspace,
    scope_generation: u64,
) -> bool {
    if workspace != Workspace::Layout || owner.scope_generation != scope_generation {
        return false;
    }
    let model = runtime.model();
    matrix_source(runtime, &model, runtime.scope().as_ref(), workspace).is_some_and(
        |(snapshot, scope, board_id)| {
            scope == owner.scope
                && board_id == owner.board_id
                && snapshot.token == owner.snapshot_token
                && snapshot.document.revision == owner.revision
        },
    )
}

struct SetupSettlement<'a> {
    waiting: &'a mut Signal<Option<PendingSettlement<(MatrixSetupOwner, String)>>>,
    open: &'a mut Signal<Option<MatrixSetupOwner>>,
    error: &'a mut Signal<Option<String>>,
    status: &'a mut Signal<Option<String>>,
    selected_context: &'a mut Signal<Option<ScopedTreeContext>>,
    anchor_scope: &'a mut Signal<Option<Scope>>,
}

fn settle_pending(
    runtime: &Rc<Runtime>,
    workspace: Workspace,
    scope_generation: u64,
    signals: &mut SetupSettlement<'_>,
) {
    let model = runtime.model();
    let Some(((owner, matrix_id), settlement)) =
        PendingSettlement::finish(signals.waiting, |(owner, _)| {
            // A hidden or departed owner cannot reconcile selection or publish feedback.
            workspace == Workspace::Layout
                && scope_generation == owner.scope_generation
                && runtime.scope().as_ref() == Some(&owner.scope)
                && model.accepted.as_ref().is_some_and(|snapshot| {
                    snapshot.session_epoch == owner.scope.session_epoch
                        && snapshot.document.id == owner.scope.document_id
                })
        })
    else {
        return;
    };
    signals.status.set(None);
    match settlement {
        Settlement::Retired => {
            signals.open.set(None);
            signals.error.set(None);
        }
        Settlement::Landed { .. } => {
            signals.open.set(None);
            signals.error.set(None);
            if model.active_board_id == owner.board_id {
                let context = TreeContext::Matrix {
                    matrix_id: matrix_id.clone(),
                };
                let Some(ids) = super::resolve_selection(&model, &context) else {
                    signals.error.set(Some(
                        "The new matrix could not be selected from the saved board.".into(),
                    ));
                    return;
                };
                signals.selected_context.set(Some(ScopedTreeContext {
                    scope: owner.scope.clone(),
                    context,
                }));
                signals.anchor_scope.set(None);
                runtime.submit(Event::SelectParts {
                    operation_id: runtime.operation(),
                    part_ids: ids,
                    range_part_ids: Vec::new(),
                    mode: SelectionMode::Replace,
                });
            }
        }
        Settlement::Failed { message } => {
            signals.error.set(Some(message));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod native_tests {
    //! Mounted Matrix Setup owner lifecycle with deterministic detached async ports. Template
    //! preparation is staged through the preparation seam and persistence through the headless
    //! test Runtime, so the suite runs against the real Session plus CoreEngine natively.
    use super::super::matrix_preparation::{clear_test_templates, stage_test_templates};
    use super::native_detached::{clear_detached, poll_detached};
    use super::*;
    use crate::matrix_setup_operation::MatrixSetupPreset;
    use boardstudio_application::SelectionMode;
    use boardstudio_core::model::{Board, ProjectDoc};
    use boardstudio_web_runtime::core_services::CoreServices;
    use std::cell::RefCell;
    use std::task::{Context, Waker};

    use super::super::test_fixtures::catalogue;

    fn services() -> CoreServices {
        CoreServices {
            validate_open: boardstudio_core::CoreEngine::validate_open,
            open_scratch: || {
                let mut engine = boardstudio_core::CoreEngine::new();
                Box::new(move |request| engine.handle(request))
            },
        }
    }

    fn document(id: &str, revision: u64, with_instance: bool) -> ProjectDoc {
        let mut document = ProjectDoc::empty(id, id);
        document.revision = revision;
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        if with_instance {
            document.hardware = Some(
                serde_json::from_value(serde_json::json!({"instances":[{
                    "id":"primary", "name":"Primary", "boardId":"board", "half":"single", "role":"standalone", "flipped":false, "constructionLinked":false
                }]})).unwrap(),
            );
        }
        document
    }

    fn open_document(runtime: &Runtime, id: &str, revision: u64, with_instance: bool) {
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: document(id, revision, with_instance),
        });
    }

    #[derive(Clone)]
    struct Probe {
        runtime: Rc<Runtime>,
        version: Rc<Cell<u64>>,
        workspace: Rc<Cell<Workspace>>,
        latest: Rc<RefCell<Option<MatrixSetupMount>>>,
    }

    impl Probe {
        fn mount(&self) -> MatrixSetupMount {
            self.latest.borrow().as_ref().unwrap().clone()
        }

        fn edits(&self) -> usize {
            self.runtime
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::ResolveEdit { .. }))
                .count()
        }

        fn create(&self) {
            let mount = self.mount();
            mount.on_create.call(MatrixSetupCreateRequest {
                owner: mount.projection.unwrap().owner,
                rows: 2,
                columns: 3,
                preset: MatrixSetupPreset::MxSolder,
            });
        }

        fn hold_pending_operation(&self) {
            self.runtime.hold_next_save();
        }

        fn release_save(&self) {
            self.runtime.release_save();
        }

        fn release_pending_operation(&self) {
            if self.runtime.save_entered() {
                self.runtime.release_save();
            }
            poll_detached();
        }
    }

    fn host() -> Element {
        let probe = use_context::<Probe>();
        let mut version = use_signal(|| 0u64);
        let mut workspace = use_signal(|| Workspace::Layout);
        let selected = use_signal(|| None::<ScopedTreeContext>);
        let anchor = use_signal(|| None::<Scope>);
        let generation = use_signal(|| 1u64);
        if *version.peek() != probe.version.get() {
            version.set(probe.version.get());
        }
        if *workspace.peek() != probe.workspace.get() {
            workspace.set(probe.workspace.get());
        }
        let mount = use_matrix_setup(
            probe.runtime.clone(),
            version,
            selected,
            anchor,
            workspace,
            generation,
        );
        *probe.latest.borrow_mut() = Some(mount.clone());
        rsx! { div {
            if mount.projection.is_some() {
                p { "Matrix setup open" }
            }
        } }
    }

    fn flush(probe: &Probe, dom: &mut VirtualDom) {
        probe.version.set(probe.version.get() + 1);
        dom.mark_dirty(ScopeId::APP);
        for _ in 0..6 {
            dom.render_immediate_to_vec();
            let mut work = std::pin::pin!(dom.wait_for_work());
            let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        }
    }

    fn mounted() -> (Probe, VirtualDom) {
        clear_detached();
        clear_test_templates();
        let probe = Probe {
            runtime: Runtime::with_core(services()),
            version: Rc::default(),
            workspace: Rc::new(Cell::new(Workspace::Layout)),
            latest: Rc::default(),
        };
        open_document(&probe.runtime, "A", 10, false);
        let mut dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        flush(&probe, &mut dom);
        (probe, dom)
    }

    fn open(probe: &Probe, dom: &mut VirtualDom) {
        probe.mount().on_open.call(());
        flush(probe, dom);
        assert!(probe.mount().projection.is_some());
    }

    fn prepared() {
        stage_test_templates(Ok(catalogue()));
    }

    fn selections(runtime: &Runtime) -> Vec<(Vec<String>, SelectionMode)> {
        runtime
            .events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::SelectParts { part_ids, mode, .. } => Some((part_ids.clone(), *mode)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn hiding_workspace_retires_admitted_owner_without_selection() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        poll_detached();
        // Edit outcomes settle synchronously at commit; the held save only gates durability.
        // Hiding the editor therefore retires the admitted owner at once instead of
        // retaining it, and the retirement must never select into the hidden workspace.
        probe.workspace.set(Workspace::Case);
        flush(&probe, &mut dom);
        assert!(
            probe.mount().projection.is_none(),
            "hiding releases the admitted owner"
        );
        probe.release_pending_operation();
        flush(&probe, &mut dom);
        assert!(probe.mount().projection.is_none());
        probe.workspace.set(Workspace::Layout);
        flush(&probe, &mut dom);
        assert!(probe.mount().can_open);
        assert!(
            selections(&probe.runtime).is_empty(),
            "retiring a hidden owner must not select"
        );
    }

    #[test]
    fn admitted_creation_selects_created_matrix_once_across_save_completion() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        poll_detached();
        flush(&probe, &mut dom);
        let created_part_ids = probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .matrices
            .last()
            .unwrap()
            .part_ids
            .clone();
        // The headless test Runtime persists the opened board synchronously, so durability
        // gating is observed through the saved revision: the created matrix waits for its
        // held save round-trip while the exact terminal already selected it exactly once.
        let saved_before = probe
            .runtime
            .saved_document("A")
            .expect("opening persists the board");
        assert_eq!(
            saved_before.revision, 10,
            "the created matrix waits for its save round-trip"
        );
        assert!(
            !saved_before
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids == created_part_ids)
        );
        assert_eq!(
            selections(&probe.runtime),
            vec![(created_part_ids.clone(), SelectionMode::Replace)]
        );
        probe.release_save();
        poll_detached();
        flush(&probe, &mut dom);
        flush(&probe, &mut dom);
        let saved_after = probe
            .runtime
            .saved_document("A")
            .expect("the completed save persists the matrix");
        assert_eq!(saved_after.revision, 11);
        assert!(
            saved_after
                .matrices
                .iter()
                .any(|matrix| matrix.part_ids == created_part_ids)
        );
        assert_eq!(
            selections(&probe.runtime),
            vec![(created_part_ids, SelectionMode::Replace)],
            "the saved completion must not select twice"
        );
        assert!(probe.mount().can_open);
    }

    #[test]
    fn opening_lower_revision_project_after_completion_retires_old_owner_without_reselecting() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.create();
        poll_detached();
        assert_eq!(probe.edits(), 1);
        flush(&probe, &mut dom);
        // The admitted edit reaches its exact terminal synchronously while A is current,
        // selecting the created matrix once. Opening B must then retire the old owner
        // without leaking a second selection into the replacement revision.
        let created_part_ids = probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .matrices
            .last()
            .unwrap()
            .part_ids
            .clone();
        assert_eq!(
            selections(&probe.runtime),
            vec![(created_part_ids, SelectionMode::Replace)]
        );
        open_document(&probe.runtime, "B", 1, false);
        flush(&probe, &mut dom);
        assert!(
            probe.mount().can_open,
            "completed A must not block lower-revision B"
        );
        assert_eq!(
            selections(&probe.runtime).len(),
            1,
            "the retired project must not select into its replacement"
        );
    }

    #[test]
    fn closing_session_retires_admitted_slot_without_reselecting() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        poll_detached();
        assert_eq!(probe.edits(), 1);
        flush(&probe, &mut dom);
        // The exact terminal settles while A is current, selecting the created matrix once.
        assert_eq!(selections(&probe.runtime).len(), 1);
        probe.runtime.submit(Event::Close {
            operation_id: probe.runtime.operation(),
        });
        probe.release_pending_operation();
        flush(&probe, &mut dom);
        // Session is terminal after Close: the same probe cannot reopen or project, and no
        // further selection fires on the released slot.
        assert!(!probe.mount().can_open, "closed session cannot reopen");
        assert!(
            probe.mount().projection.is_none(),
            "closed completion releases its slot"
        );
        assert_eq!(selections(&probe.runtime).len(), 1);
    }

    #[test]
    fn detached_normalization_after_editor_unmount_never_touches_disposed_signals() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.create();
        poll_detached();
        drop(dom);
        prepared();
        poll_detached();
        assert_eq!(probe.edits(), 0);
    }

    #[test]
    fn admitted_outcome_survives_editor_unmount_until_exact_terminal() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.hold_pending_operation();
        prepared();
        probe.create();
        poll_detached();
        assert_eq!(probe.edits(), 1);
        drop(dom);
        probe.release_pending_operation();
    }

    #[test]
    fn changed_accepted_token_during_normalization_never_submits() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.create();
        poll_detached();
        open_document(&probe.runtime, "A", 11, false);
        prepared();
        poll_detached();
        flush(&probe, &mut dom);
        assert_eq!(probe.edits(), 0);
    }
}
