//! Regressions mount the actual production hook, not a copy of its admission/async logic.
use super::*;
use boardstudio_application::{Event, TerminalOutcome};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    task::{Context, Poll, Waker},
};

#[derive(Clone)]
struct Probe {
    runtime: Rc<crate::runtime::Runtime>,
    active: Rc<Cell<bool>>,
    generation: Rc<Cell<u64>>,
    reply: Rc<RefCell<Option<Result<boardstudio_core::model::ProjectDoc, String>>>>,
    latest: Rc<RefCell<Option<PhysicalSetupMount>>>,
}
impl Probe {
    fn new() -> Self {
        let runtime = crate::runtime::Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: fixture_document("A", 1),
        });
        Self {
            runtime,
            active: Rc::new(Cell::new(true)),
            generation: Rc::new(Cell::new(1)),
            reply: Rc::new(RefCell::new(None)),
            latest: Rc::new(RefCell::new(None)),
        }
    }
    fn mount(&self) -> PhysicalSetupMount {
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
    fn changed_proposal(&self) -> boardstudio_core::model::ProjectDoc {
        let mut doc = (*self.runtime.model().accepted.unwrap().document).clone();
        doc.parameters
            .insert("reversibleLayout".into(), serde_json::json!(true));
        doc
    }
    fn replace_project(&self, id: &str, revision: u64) {
        self.runtime.submit(Event::Open {
            operation_id: self.runtime.operation(),
            document: fixture_document(id, revision),
        });
    }
    fn release_pending_operation(&self) {
        if self.runtime.core_entered() {
            self.runtime.release_core();
        } else if self.runtime.save_entered() {
            self.runtime.release_save();
        }
        crate::poll_detached();
    }
}
fn fixture_document(id: &str, revision: u64) -> boardstudio_core::model::ProjectDoc {
    let mut document: boardstudio_core::model::ProjectDoc = serde_json::from_str(include_str!(
        "../../../../../core/tests/fixtures/reviung41-outline-original.json"
    ))
    .expect("checked-in Reviung fixture is a valid saved project");
    document.id = id.into();
    document.revision = revision;
    document
}
fn host() -> Element {
    let probe = use_context::<Probe>();
    let version = use_signal(|| 0u64);
    let mut generation = use_signal(|| 1u64);
    if *generation.peek() != probe.generation.get() {
        generation.set(probe.generation.get());
    }
    let current = probe.clone();
    let prepare = probe.clone();
    let mount = use_controller(
        probe.runtime.clone(),
        version,
        generation,
        {
            let active = probe.active.clone();
            Rc::new(move || active.get())
        },
        crate::InstanceSelection,
        Rc::new(move |owner: &OwnerIdentity, strict| {
            let model = current.runtime.model();
            let accepted = model.accepted.unwrap();
            owner.generation == current.generation.get()
                && owner.document_id == accepted.document.id
                && owner.session_epoch == accepted.session_epoch
                && owner.board_id == model.active_board_id
                && match owner.context {
                    OwnerContext::ProjectGuide => owner.instance_id == model.active_instance_id,
                    OwnerContext::CaseInspector => current
                        .runtime
                        .scope()
                        .is_some_and(|scope| scope.instance_id == owner.instance_id),
                }
                && (!strict
                    || (owner.token == accepted.token
                        && owner.revision == accepted.document.revision))
        }),
        Rc::new(move |_, _| {
            let reply = prepare.reply.clone();
            Box::pin(std::future::poll_fn(move |_| {
                match reply.borrow_mut().take() {
                    Some(value) => Poll::Ready(value),
                    None => Poll::Pending,
                }
            }))
        }),
    );
    *probe.latest.borrow_mut() = Some(mount.clone());
    controller::project_setup_controls(mount)
}
fn flush(dom: &mut VirtualDom) {
    dom.mark_dirty(ScopeId::APP);
    for _ in 0..4 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = std::future::Future::poll(work.as_mut(), &mut Context::from_waker(Waker::noop()));
    }
}
fn mounted() -> (Probe, VirtualDom) {
    crate::TASKS.with_borrow_mut(Vec::clear);
    let probe = Probe::new();
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    flush(&mut dom);
    (probe, dom)
}
fn send(mount: &PhysicalSetupMount) {
    mount.submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
}

#[test]
fn retained_rendered_controls_cannot_retarget_another_project() {
    let (probe, mut dom) = mounted();
    let old = probe.mount();
    probe.replace_project("B", 1);
    flush(&mut dom);
    *probe.reply.borrow_mut() = Some(Ok(probe.changed_proposal()));
    send(&old);
    crate::poll_detached();
    assert_eq!(
        probe.edits(),
        0,
        "retained A control must not submit an Edit for B"
    );
}
#[test]
fn retained_rendered_controls_reject_replaced_token_and_generation() {
    for (token, generation, revision) in [(2, 1, 1), (1, 2, 1), (1, 1, 2)] {
        let (probe, mut dom) = mounted();
        let old = probe.mount();
        if token != 1 || revision != 1 {
            probe.replace_project("A", revision);
        }
        probe.generation.set(generation);
        flush(&mut dom);
        *probe.reply.borrow_mut() = Some(Ok(probe.changed_proposal()));
        send(&old);
        crate::poll_detached();
        assert_eq!(
            probe.edits(),
            0,
            "stale rendered source cannot be refreshed at dispatch"
        );
    }
}
#[test]
fn closing_project_stage_during_normalization_prevents_submission() {
    let (probe, mut dom) = mounted();
    let proposal = probe.changed_proposal();
    send(&probe.mount());
    crate::poll_detached();
    probe.active.set(false);
    flush(&mut dom);
    *probe.reply.borrow_mut() = Some(Ok(proposal));
    crate::poll_detached();
    assert_eq!(
        probe.edits(),
        0,
        "hidden Project stage must fail async admission"
    );
}
#[test]
fn delayed_preparation_error_and_noop_do_not_appear_under_new_source() {
    for failure in [true, false] {
        let (probe, mut dom) = mounted();
        let unchanged = (*probe.runtime.model().accepted.unwrap().document).clone();
        send(&probe.mount());
        crate::poll_detached();
        probe.replace_project("B", 1);
        flush(&mut dom);
        *probe.reply.borrow_mut() = Some(if failure {
            Err("old failure".into())
        } else {
            Ok(unchanged)
        });
        crate::poll_detached();
        flush(&mut dom);
        assert!(
            probe.mount().projection.project_feedback.is_none(),
            "B must not display A preparation result"
        );
    }
}
#[test]
fn terminal_results_keep_exact_observer_but_never_retarget_feedback() {
    for result in [
        TerminalOutcome::PersistenceFailed("old disk".into()),
        TerminalOutcome::Completed,
        TerminalOutcome::Cancelled,
    ] {
        let (probe, mut dom) = mounted();
        match &result {
            TerminalOutcome::Completed => probe.runtime.hold_next_save(),
            TerminalOutcome::PersistenceFailed(reason) => {
                probe.runtime.fail_next_save(reason.clone())
            }
            TerminalOutcome::Cancelled => probe.runtime.hold_next_core(),
        }
        *probe.reply.borrow_mut() = Some(Ok(probe.changed_proposal()));
        send(&probe.mount());
        crate::poll_detached();
        assert_eq!(probe.edits(), 1);
        probe.replace_project("B", 1);
        flush(&mut dom);
        probe.release_pending_operation();
        flush(&mut dom);
        assert!(
            probe.mount().projection.project_feedback.is_none(),
            "B must not display A terminal result"
        );
    }
}

#[test]
fn case_transport_keeps_the_rendered_scope() {
    let (probe, mut dom) = mounted();
    let old = probe.mount();
    probe.replace_project("B", 1);
    flush(&mut dom);
    *probe.reply.borrow_mut() = Some(Ok(probe.changed_proposal()));
    old.submit(PhysicalSetupIntent::CaseTransport(
        boardstudio_core::model::HardwareTransport::Wired,
    ));
    crate::poll_detached();
    assert_eq!(probe.edits(), 0);
}
#[test]
fn exact_accepted_proposal_lands_silently_without_leaking_to_another_context() {
    let (probe, mut dom) = mounted();
    let proposal = probe.changed_proposal();
    probe.runtime.hold_next_save();
    *probe.reply.borrow_mut() = Some(Ok(proposal.clone()));
    send(&probe.mount());
    crate::poll_detached();
    assert_eq!(probe.edits(), 1);
    probe.release_pending_operation();
    flush(&mut dom);
    assert!(
        probe.mount().projection.project_feedback.is_none(),
        "landing shows the accepted value without a status message"
    );
    assert!(
        probe.mount().projection.feedback.is_none(),
        "Project result must not appear in Case"
    );
    assert!(!probe.mount().projection.busy);
}
#[test]
fn hidden_guide_keeps_observer_and_restores_only_same_owner_result() {
    let (probe, mut dom) = mounted();
    probe.runtime.fail_next_save("disk");
    *probe.reply.borrow_mut() = Some(Ok(probe.changed_proposal()));
    send(&probe.mount());
    crate::poll_detached();
    probe.active.set(false);
    flush(&mut dom);
    probe.release_pending_operation();
    flush(&mut dom);
    assert!(probe.mount().projection.project_feedback.is_none());
    probe.active.set(true);
    flush(&mut dom);
    assert_eq!(
        probe.mount().projection.project_feedback.as_deref(),
        Some("Physical setup failed: disk")
    );
    assert_eq!(probe.edits(), 1, "restoring the view must not resubmit");
}
#[test]
fn detached_normalization_and_outcome_survive_unmount_without_signal_access() {
    for submitted in [false, true] {
        let (probe, dom) = mounted();
        let proposal = probe.changed_proposal();
        if submitted {
            probe.runtime.hold_next_core();
            *probe.reply.borrow_mut() = Some(Ok(proposal.clone()));
        }
        send(&probe.mount());
        crate::poll_detached();
        drop(dom);
        if submitted {
            probe.release_pending_operation();
        } else {
            *probe.reply.borrow_mut() = Some(Ok(proposal));
            crate::poll_detached();
            assert_eq!(probe.edits(), 0);
        }
    }
}

#[test]
fn hidden_success_settles_silently_for_its_exact_accepted_proposal() {
    let (probe, mut dom) = mounted();
    let proposal = probe.changed_proposal();
    probe.runtime.hold_next_save();
    *probe.reply.borrow_mut() = Some(Ok(proposal.clone()));
    send(&probe.mount());
    crate::poll_detached();
    probe.active.set(false);
    flush(&mut dom);
    probe.release_pending_operation();
    flush(&mut dom);
    assert!(probe.mount().projection.project_feedback.is_none());
    probe.active.set(true);
    flush(&mut dom);
    assert!(
        probe.mount().projection.project_feedback.is_none(),
        "a success recorded while hidden still shows no status when the view returns"
    );
    assert_eq!(probe.edits(), 1);
}
