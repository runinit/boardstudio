//! Matrix Setup preparation and synchronous submission against the accepted source.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
use self::native_detached::spawn_local;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
use self::native_setup_types::{
    MatrixSetupCreateRequest, MatrixSetupMount, MatrixSetupOwner, MatrixSetupProjection,
};
use super::matrix_preparation::{
    fail_creation, matrix_dimensions_valid, matrix_source, owner_source_matches,
    prepare_matrix_definitions, prepared_definitions_error, reversible_layout_param,
};
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
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let open = use_signal(|| None::<MatrixSetupOwner>);
    let mut flight_seq = use_signal(|| 0_u64);
    let preparing = use_signal(|| None::<(u64, MatrixSetupOwner)>);
    let error = use_signal(|| None::<String>);
    let status = use_signal(|| None::<String>);

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let runtime = runtime.clone();
            let mut open = open;
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            move |(_, workspace, _)| {
                // Commits land synchronously, so the open entry only needs
                // reactive cleanup when its scope leaves the Layout workspace.
                if open.read().as_ref().is_some_and(|owner| {
                    workspace != Workspace::Layout || runtime.scope().as_ref() != Some(&owner.scope)
                }) {
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
        let mut error = error;
        let mut status = status;
        move |_| {
            if preparing.read().is_some() {
                return;
            }
            let current_workspace = workspace();
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
            error.set(None);
            status.set(None);
            open.set(Some(MatrixSetupOwner {
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
            // Cancelling also releases a preparation flight: its completion guard
            // compares both sequence id and owner against `preparing`, so a retired
            // flight cannot hijack a reopened form on the same snapshot.
            if open.read().as_ref() == Some(&owner) {
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
                || preparing.read().is_some()
                || workspace() != Workspace::Layout
            {
                return;
            }
            if !matrix_dimensions_valid(request.rows, request.columns) {
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
            if !owner_source_matches(
                &request.owner.scope,
                request.owner.snapshot_token,
                request.owner.revision,
                &scope,
                snapshot,
            ) {
                error.set(Some("The accepted board changed. Close Matrix Setup and reopen it before creating a matrix.".into()));
                return;
            }
            let owner = request.owner.clone();
            let accepted = snapshot.clone();
            let reversible = reversible_layout_param(&accepted.document);
            let matrix_id = next_matrix_id(
                &accepted.document.matrices,
                &accepted.document.definitions,
                &accepted.document.parts,
                request.preset.as_str(),
                reversible,
            );
            let flight_id = flight_seq().wrapping_add(1);
            flight_seq.set(flight_id);
            preparing.set(Some((flight_id, owner.clone())));
            error.set(None);
            status.set(Some("Preparing matrix definitions…".into()));
            let runtime = runtime.clone();
            let mut preparing = preparing;
            let mut error = error;
            let mut status = status;
            let mut open = open;
            let mut selected_context = selected_context;
            let mut anchor_scope = anchor_scope;
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
                    .is_some_and(|(seq, active)| *seq == flight_id && active == &owner)
                {
                    return;
                }
                let model = runtime.model();
                let current_scope = runtime.scope();
                let current_workspace = workspace();
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
                        fail_creation(&mut preparing, &mut error, &mut status, message);
                        return;
                    }
                };
                if let Some(message) = prepared_definitions_error(
                    current,
                    &current_board,
                    matrix.board_id.as_deref(),
                    &definitions,
                ) {
                    fail_creation(&mut preparing, &mut error, &mut status, message);
                    return;
                }
                let matrix_id = matrix.id.clone();
                // The preparation above is genuinely asynchronous (template loading
                // and normalization); the edit itself commits synchronously, so its
                // outcome is handled inline and selection follows at once.
                match runtime.commit(
                    "layout-matrix-setup",
                    create_matrix_resolver(current_board.clone(), matrix, definitions),
                ) {
                    Ok(_) => {
                        preparing.set(None);
                        status.set(None);
                        open.set(None);
                        error.set(None);
                        // A hidden or departed owner cannot reconcile selection.
                        if workspace() == Workspace::Layout
                            && runtime.scope().as_ref() == Some(&owner.scope)
                            && runtime.model().active_board_id == owner.board_id
                        {
                            let context = TreeContext::Matrix {
                                matrix_id: matrix_id.clone(),
                            };
                            let Some(ids) = super::resolve_selection(&runtime.model(), &context)
                            else {
                                error.set(Some(
                                    "The new matrix could not be selected from the saved board."
                                        .into(),
                                ));
                                return;
                            };
                            selected_context.set(Some(ScopedTreeContext {
                                scope: owner.scope.clone(),
                                context,
                            }));
                            anchor_scope.set(None);
                            runtime.submit(Event::SelectParts {
                                operation_id: runtime.operation(),
                                part_ids: ids,
                                range_part_ids: Vec::new(),
                                mode: SelectionMode::Replace,
                            });
                        }
                    }
                    Err(message) => {
                        fail_creation(&mut preparing, &mut error, &mut status, message);
                    }
                }
            });
        }
    });

    let projection = open.read().clone().map(|owner| MatrixSetupProjection {
        editable: preparing.read().is_none() && setup_owner_live(&runtime, &owner, workspace()),
        can_cancel: true,
        owner,
        status: status.read().clone(),
        error: error.read().clone(),
    });
    let current_model = runtime.model();
    let can_open = preparing.read().is_none()
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

fn setup_owner_live(runtime: &Runtime, owner: &MatrixSetupOwner, workspace: Workspace) -> bool {
    if workspace != Workspace::Layout || runtime.scope().as_ref() != Some(&owner.scope) {
        return false;
    }
    let model = runtime.model();
    matrix_source(runtime, &model, runtime.scope().as_ref(), workspace).is_some_and(
        |(snapshot, scope, _)| {
            owner_source_matches(
                &owner.scope,
                owner.snapshot_token,
                owner.revision,
                &scope,
                snapshot,
            )
        },
    )
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
    use std::cell::RefCell;
    use std::task::{Context, Waker};

    use super::super::test_fixtures::{catalogue, open_board_document as open_document};

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
            runtime: Runtime::new(),
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
        // The edit commits synchronously once preparation lands, so hiding before the
        // preparation completes retires the owner instead: nothing commits and the
        // retirement must never select into the hidden workspace.
        probe.workspace.set(Workspace::Case);
        flush(&probe, &mut dom);
        assert!(
            probe.mount().projection.is_none(),
            "hiding releases the admitted owner"
        );
        poll_detached();
        flush(&probe, &mut dom);
        assert!(probe.mount().projection.is_none());
        probe.release_pending_operation();
        flush(&probe, &mut dom);
        probe.workspace.set(Workspace::Layout);
        flush(&probe, &mut dom);
        assert!(probe.mount().can_open);
        assert!(
            selections(&probe.runtime).is_empty(),
            "retiring a hidden owner must not select"
        );
        assert_eq!(
            probe.edits(),
            0,
            "hidden preparation never reaches the synchronous commit"
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

    #[test]
    fn cancelled_flight_cannot_commit_after_reopen_on_same_snapshot() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        let first = probe.mount().projection.unwrap().owner;
        probe.create();
        poll_detached();
        // Cancel clears `preparing`, then reopen on the same snapshot creates an
        // equal MatrixSetupOwner and starts a second preparation flight (3x4).
        probe.mount().on_cancel.call(first);
        flush(&probe, &mut dom);
        open(&probe, &mut dom);
        let second = probe.mount();
        second.on_create.call(MatrixSetupCreateRequest {
            owner: second.projection.unwrap().owner,
            rows: 3,
            columns: 4,
            preset: MatrixSetupPreset::MxSolder,
        });
        // Wait: do not poll both flights at once; stage one reply so only flight 1
        // observes it, proving flight 1's stale flight_id is rejected while flight 2
        // stays armed and commits 3x4 on its reply.
        prepared();
        poll_detached();
        flush(&probe, &mut dom);
        assert_eq!(probe.edits(), 0);
        assert!(probe.mount().projection.is_some());
        prepared();
        poll_detached();
        flush(&probe, &mut dom);
        assert_eq!(probe.edits(), 1);
        let accepted = probe.runtime.model().accepted.unwrap();
        let created = accepted.document.matrices.last().unwrap();
        assert_eq!((created.rows, created.columns), (3, 4));
    }
}
