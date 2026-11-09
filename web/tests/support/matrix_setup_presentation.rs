use boardstudio_application::{ReadModel, Scope};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeContext {
    Matrix { matrix_id: String },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedTreeContext {
    pub scope: Scope,
    pub context: TreeContext,
}
fn resolve_selection(model: &ReadModel, context: &TreeContext) -> Option<Vec<String>> {
    let TreeContext::Matrix { matrix_id } = context;
    model
        .accepted
        .as_ref()?
        .document
        .matrices
        .iter()
        .find(|matrix| &matrix.id == matrix_id)
        .map(|matrix| matrix.part_ids.clone())
}
#[path = "../../src/presentation/objects/matrix_setup_controller.rs"]
mod controller;
#[path = "../../src/presentation/objects/matrix_setup.rs"]
mod matrix_setup;

pub mod parts {
    use boardstudio_core::model::PartDefinition;
    pub async fn load_matrix_templates(_: bool) -> Result<Vec<PartDefinition>, String> {
        std::future::poll_fn(|_| {
            crate::TEMPLATES.with_borrow_mut(|reply| {
                reply
                    .take()
                    .map_or(std::task::Poll::Pending, std::task::Poll::Ready)
            })
        })
        .await
    }
    pub async fn normalize_matrix_definition(
        definition: PartDefinition,
    ) -> Result<PartDefinition, String> {
        Ok(definition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{matrix_setup_operation::MatrixSetupPreset, runtime};
    use boardstudio_application::Event;
    use dioxus::prelude::*;
    use std::{
        cell::{Cell, RefCell},
        future::Future,
        rc::Rc,
        task::{Context, Waker},
    };

    #[derive(Clone)]
    struct Probe {
        runtime: Rc<runtime::Runtime>,
        version: Rc<Cell<u64>>,
        workspace: Rc<Cell<&'static str>>,
        latest: Rc<RefCell<Option<matrix_setup::MatrixSetupMount>>>,
    }
    impl Probe {
        fn mount(&self) -> matrix_setup::MatrixSetupMount {
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
            mount
                .on_create
                .call(matrix_setup::MatrixSetupCreateRequest {
                    owner: mount.projection.unwrap().owner,
                    rows: 2,
                    columns: 3,
                    preset: MatrixSetupPreset::MxSolder,
                });
        }
        fn hold_pending_operation(&self) {
            self.runtime.hold_next_core();
            self.runtime.hold_next_save();
        }
        fn release_core(&self) {
            self.runtime.release_core();
        }
        fn release_save(&self) {
            self.runtime.release_save();
        }
        fn release_pending_operation(&self) {
            if self.runtime.core_entered() {
                self.runtime.release_core();
            }
            if self.runtime.save_entered() {
                self.runtime.release_save();
            }
            crate::poll_detached();
        }
    }
    fn host() -> Element {
        let probe = use_context::<Probe>();
        let mut version = use_signal(|| 0u64);
        let mut workspace = use_signal(|| "Layout");
        let selected = use_signal(|| None::<ScopedTreeContext>);
        let anchor = use_signal(|| None::<Scope>);
        let generation = use_signal(|| 1u64);
        if *version.peek() != probe.version.get() {
            version.set(probe.version.get());
        }
        if *workspace.peek() != probe.workspace.get() {
            workspace.set(probe.workspace.get());
        }
        let mount = controller::use_matrix_setup(
            probe.runtime.clone(),
            version,
            selected,
            anchor,
            workspace,
            generation,
        );
        *probe.latest.borrow_mut() = Some(mount.clone());
        rsx! { div {
            if let Some(projection) = mount.projection {
                matrix_setup::MatrixSetup { projection, on_cancel: mount.on_cancel, on_create: mount.on_create }
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
        crate::TASKS.with_borrow_mut(Vec::clear);
        crate::TEMPLATES.with_borrow_mut(|reply| *reply = None);
        let probe = Probe {
            runtime: runtime::Runtime::new(),
            version: Rc::default(),
            workspace: Rc::new(Cell::new("Layout")),
            latest: Rc::default(),
        };
        crate::open_document(&probe.runtime, "A", 10, false);
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
        crate::TEMPLATES.with_borrow_mut(|reply| *reply = Some(Ok(crate::catalogue())));
    }

    #[test]
    fn configured_board_instance_can_open_matrix_setup() {
        let (probe, mut dom) = mounted();
        crate::open_document(&probe.runtime, "A", 10, true);
        probe.runtime.submit(Event::Navigate {
            operation_id: probe.runtime.operation(),
            board_id: "board".into(),
            instance_id: Some("primary".into()),
        });
        flush(&probe, &mut dom);
        assert!(
            probe.mount().can_open,
            "a current configured physical instance must admit logical board creation"
        );
        open(&probe, &mut dom);
    }
    #[test]
    fn nonexistent_physical_instance_cannot_open_matrix_setup() {
        let (probe, mut dom) = mounted();
        probe.runtime.submit(Event::Navigate {
            operation_id: probe.runtime.operation(),
            board_id: "board".into(),
            instance_id: Some("missing".into()),
        });
        flush(&probe, &mut dom);
        assert!(!probe.mount().can_open);
    }
    #[test]
    fn hidden_pending_waits_for_exact_terminal_then_releases_without_selection() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        crate::poll_detached();
        probe.workspace.set("Case");
        flush(&probe, &mut dom);
        assert!(
            probe.mount().projection.is_some(),
            "hiding must retain the admitted owner"
        );
        probe.release_pending_operation();
        flush(&probe, &mut dom);
        assert!(probe.mount().projection.is_none());
        probe.workspace.set("Layout");
        flush(&probe, &mut dom);
        assert!(probe.mount().can_open);
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::SelectParts { .. }))
        );
    }
    #[test]
    fn current_saved_completion_selects_the_created_matrix_exactly_once() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        crate::poll_detached();
        probe.release_core();
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
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::SelectParts { .. })),
            "terminal completion alone cannot select an unsaved proposal"
        );
        probe.release_save();
        crate::poll_detached();
        flush(&probe, &mut dom);
        flush(&probe, &mut dom);
        let events = probe.runtime.events.borrow();
        let selections: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::SelectParts { part_ids, mode, .. } => Some((part_ids, mode)),
                _ => None,
            })
            .collect();
        assert_eq!(selections.len(), 1);
        assert_eq!(selections[0].0, &created_part_ids);
        assert_eq!(
            *selections[0].1,
            boardstudio_application::SelectionMode::Replace
        );
        assert!(probe.mount().can_open);
    }
    #[test]
    fn completed_old_project_retires_before_replacement_revision_checks() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        crate::poll_detached();
        assert_eq!(probe.edits(), 1);
        crate::open_document(&probe.runtime, "B", 1, false);
        flush(&probe, &mut dom);
        assert!(
            !probe.mount().can_open,
            "pending exact outcome is still retained"
        );
        probe.release_pending_operation();
        flush(&probe, &mut dom);
        assert!(
            probe.mount().can_open,
            "completed A must not block lower-revision B"
        );
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::SelectParts { .. }))
        );
    }
    #[test]
    fn closed_session_completion_releases_its_slot() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.hold_pending_operation();
        probe.create();
        crate::poll_detached();
        probe.runtime.submit(Event::Close {
            operation_id: probe.runtime.operation(),
        });
        probe.release_pending_operation();
        flush(&probe, &mut dom);
        crate::open_document(&probe.runtime, "B", 1, false);
        flush(&probe, &mut dom);
        assert!(probe.mount().can_open);
    }
    #[test]
    fn detached_normalization_after_editor_unmount_never_touches_disposed_signals() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.create();
        crate::poll_detached();
        drop(dom);
        prepared();
        crate::poll_detached();
        assert_eq!(probe.edits(), 0);
    }
    #[test]
    fn admitted_outcome_survives_editor_unmount_until_exact_terminal() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.hold_pending_operation();
        prepared();
        probe.create();
        crate::poll_detached();
        assert_eq!(probe.edits(), 1);
        drop(dom);
        probe.release_pending_operation();
    }
    #[test]
    fn changed_accepted_token_during_normalization_never_submits() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.create();
        crate::poll_detached();
        crate::open_document(&probe.runtime, "A", 11, false);
        prepared();
        crate::poll_detached();
        flush(&probe, &mut dom);
        assert_eq!(probe.edits(), 0);
    }
}
