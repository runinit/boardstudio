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
    use boardstudio_application::{Event, TerminalOutcome};
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
                .filter(|event| matches!(event, Event::Edit { .. }))
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
        fn settle(&self) {
            let id = self
                .runtime
                .events
                .borrow()
                .iter()
                .find_map(|event| match event {
                    Event::Edit { operation_id, .. } => Some(*operation_id),
                    _ => None,
                })
                .unwrap();
            assert!(
                self.runtime.outcomes.settle(id, TerminalOutcome::Completed),
                "exact admitted observer must survive hidden/unmounted owner"
            );
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
        let mut model = runtime::model("A", 1, 10);
        let accepted = model.accepted.as_mut().unwrap();
        let document = std::sync::Arc::make_mut(&mut accepted.document);
        document.hardware = Some(serde_json::from_value(serde_json::json!({"instances":[{
            "id":"primary", "name":"Primary", "boardId":"board", "half":"single", "role":"standalone", "flipped":false, "constructionLinked":false
        }]})).unwrap());
        model.active_instance_id = Some("primary".into());
        *probe.runtime.model.borrow_mut() = model;
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
        probe.runtime.model.borrow_mut().active_instance_id = Some("missing".into());
        flush(&probe, &mut dom);
        assert!(!probe.mount().can_open);
    }
    #[test]
    fn hidden_pending_waits_for_exact_terminal_then_releases_without_selection() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.create();
        crate::poll_detached();
        probe.workspace.set("Case");
        flush(&probe, &mut dom);
        assert!(
            probe.mount().projection.is_some(),
            "hiding must retain the admitted owner"
        );
        probe.settle();
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
        probe.create();
        crate::poll_detached();
        let mut matrix = probe
            .runtime
            .events
            .borrow()
            .iter()
            .find_map(|event| match event {
                Event::Edit { command, .. } => match &command.operation {
                    boardstudio_core::model::EditOperation::SetMatrix { matrix, .. } => {
                        Some(matrix.clone())
                    }
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        matrix.part_ids = vec!["created-key".into()];
        let mut model = runtime::model("A", 2, 11);
        std::sync::Arc::make_mut(&mut model.accepted.as_mut().unwrap().document)
            .matrices
            .push(matrix);
        model.durability = boardstudio_application::Durability::Saving { revision: 11 };
        *probe.runtime.model.borrow_mut() = model;
        probe.settle();
        flush(&probe, &mut dom);
        assert!(
            !probe
                .runtime
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::SelectParts { .. })),
            "terminal completion alone cannot select an unsaved proposal"
        );
        probe.runtime.model.borrow_mut().durability =
            boardstudio_application::Durability::Saved { revision: 11 };
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
        assert_eq!(selections[0].0, &["created-key"]);
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
        probe.create();
        crate::poll_detached();
        assert_eq!(probe.edits(), 1);
        *probe.runtime.model.borrow_mut() = runtime::model("B", 2, 1);
        flush(&probe, &mut dom);
        assert!(
            !probe.mount().can_open,
            "pending exact outcome is still retained"
        );
        probe.settle();
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
    fn completed_creation_without_accepted_snapshot_releases_its_slot() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        prepared();
        probe.create();
        crate::poll_detached();
        *probe.runtime.model.borrow_mut() = Default::default();
        probe.settle();
        flush(&probe, &mut dom);
        *probe.runtime.model.borrow_mut() = runtime::model("B", 2, 1);
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
        prepared();
        probe.create();
        crate::poll_detached();
        assert_eq!(probe.edits(), 1);
        drop(dom);
        probe.settle();
    }
    #[test]
    fn changed_accepted_token_during_normalization_never_submits() {
        let (probe, mut dom) = mounted();
        open(&probe, &mut dom);
        probe.create();
        crate::poll_detached();
        *probe.runtime.model.borrow_mut() = runtime::model("A", 2, 11);
        prepared();
        crate::poll_detached();
        flush(&probe, &mut dom);
        assert_eq!(probe.edits(), 0);
    }
}
