//! Table-driven controller tests for PCB wiring resolution and part-net admission.
//!
//! Mounted scenarios share one probe host and notice helper.
use super::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core_mechanical::electrical::{
    ElectricalMode, ElectricalPlan, ElectricalPlanRequest,
};
use boardstudio_model::model::{Readiness, SceneDelta};
use futures_channel::oneshot;
use std::{cell::RefCell, collections::VecDeque, sync::Arc};
use wasm_bindgen_test::wasm_bindgen_test;

type Reply = oneshot::Sender<Result<ElectricalPlan, String>>;

struct Probe {
    version: Cell<u64>,
    input: RefCell<Option<WiringInput>>,
    replies: RefCell<VecDeque<Reply>>,
    calls: Cell<usize>,
    latest: RefCell<Option<PcbWiringMount>>,
}

fn board_input(board_id: &str, token: u64) -> WiringInput {
    board_input_with_controller(board_id, token, 0, None)
}

fn board_input_with_controller(
    board_id: &str,
    token: u64,
    revision: u64,
    controller_part_id: Option<&str>,
) -> WiringInput {
    let mut document = ProjectDoc::empty("resolver-test", "Resolver test");
    document.revision = revision;
    document.boards.push(boardstudio_model::model::Board {
        id: board_id.into(),
        name: board_id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.hardware = Some(boardstudio_model::model::HardwareConfiguration {
        boards: vec![boardstudio_model::model::ElectricalBoardConfiguration {
            board_id: board_id.into(),
            controller_part_id: controller_part_id.map(str::to_owned),
            ..Default::default()
        }],
        ..Default::default()
    });
    let identity = WiringPlanIdentity {
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: document.id.clone(),
            board_id: board_id.into(),
            instance_id: None,
        },
        token: SnapshotToken(token),
        revision,
        executor_epoch: 1,
    };
    let accepted = AcceptedSnapshot {
        token: SnapshotToken(token),
        session_epoch: SessionEpoch(1),
        document: Arc::new(document.clone()),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: document.revision,
            transaction_id: "resolver-test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        }),
    };
    let scope = identity.scope.clone();
    (identity, accepted, scope)
}

fn test_plan(board_id: &str, revision: u64) -> ElectricalPlan {
    let mut plan = boardstudio_core_mechanical::electrical::resolve(ElectricalPlanRequest {
        document: ProjectDoc::empty("resolver-test", "Resolver test"),
        instance_id: None,
        mode: ElectricalMode::Matrix,
        locks: Default::default(),
        controller_profile: None,
        board_id: Some(board_id.into()),
        controller_part_id: None,
    });
    plan.revision = revision;
    plan
}

fn host() -> Element {
    let probe = use_context::<Rc<Probe>>();
    let mut version = use_signal(|| probe.version.get());
    if *version.peek() != probe.version.get() {
        version.set(probe.version.get());
    }
    let current_probe = probe.clone();
    let current: Rc<dyn Fn() -> Option<WiringInput>> =
        Rc::new(move || current_probe.input.borrow().clone());
    let resolver_probe = probe.clone();
    let resolver: WiringResolver = Rc::new(move |_, _| {
        resolver_probe.calls.set(resolver_probe.calls.get() + 1);
        let (sender, receiver) = oneshot::channel();
        resolver_probe.replies.borrow_mut().push_back(sender);
        Box::pin(async move {
            receiver
                .await
                .unwrap_or_else(|_| Err("test resolver reply dropped".into()))
        })
    });
    let mount = use_pcb_wiring_resolution_owner(version, current, resolver);
    *probe.latest.borrow_mut() = Some(mount.clone());
    let identity = probe
        .input
        .borrow()
        .as_ref()
        .map(|(identity, _, _)| identity.clone());
    let notice = identity
        .as_ref()
        .map(|identity| wiring_resolution_notice(&mount.resolution, identity));
    rsx! {
        div {
            match notice {
                Some(WiringResolutionNotice::Pending) => rsx! { p { role: "status", "Resolving wiring…" } },
                Some(WiringResolutionNotice::Failed(message)) => rsx! { p { role: "alert", "{message}" } },
                _ => rsx! {},
            }
        }
    }
}

fn flush(dom: &mut VirtualDom) {
    dom.mark_all_dirty();
    for _ in 0..5 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = work
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
    }
}

async fn tick(dom: &mut VirtualDom) {
    gloo_timers::future::TimeoutFuture::new(20).await;
    flush(dom);
}

async fn until_calls(probe: &Probe, dom: &mut VirtualDom, expected: usize) {
    for _ in 0..20 {
        if probe.calls.get() >= expected {
            return;
        }
        tick(dom).await;
    }
    panic!(
        "resolver was called {} times, expected {expected}",
        probe.calls.get()
    );
}

fn reply(probe: &Probe, result: Result<ElectricalPlan, String>) {
    reply_at(probe, 0, result);
}

fn reply_at(probe: &Probe, index: usize, result: Result<ElectricalPlan, String>) {
    probe
        .replies
        .borrow_mut()
        .remove(index)
        .expect("resolver call should have a pending reply")
        .send(result)
        .expect("mounted owner should still be awaiting the reply");
}

fn mounted_probe(input: WiringInput) -> (Rc<Probe>, VirtualDom) {
    let probe = Rc::new(Probe {
        version: Cell::new(0),
        input: RefCell::new(Some(input)),
        replies: RefCell::default(),
        calls: Cell::new(0),
        latest: RefCell::default(),
    });
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    flush(&mut dom);
    (probe, dom)
}

fn notice_of(probe: &Probe) -> Option<WiringResolutionNotice> {
    probe.latest.borrow().as_ref().map(|mount| {
        wiring_resolution_notice(&mount.resolution, &probe.input.borrow().as_ref().unwrap().0)
    })
}

#[wasm_bindgen_test]
async fn mounted_resolution_failure_is_visible_retryable_and_ignores_old_board_reply() {
    let (probe, mut dom) = mounted_probe(board_input("left", 1));

    until_calls(&probe, &mut dom, 1).await;
    assert_eq!(notice_of(&probe), Some(WiringResolutionNotice::Pending));
    reply(&probe, Err("injected Core resolution failure".into()));
    tick(&mut dom).await;
    assert_eq!(
        notice_of(&probe),
        Some(WiringResolutionNotice::Failed(
            "injected Core resolution failure".into()
        ))
    );
    assert_eq!(
        WiringResolutionNotice::Failed("injected Core resolution failure".into()).role(),
        "alert"
    );

    probe
        .latest
        .borrow()
        .as_ref()
        .expect("mounted owner should publish retry action")
        .on_resolve
        .call(());
    until_calls(&probe, &mut dom, 2).await;
    reply(&probe, Ok(test_plan("left", 0)));
    tick(&mut dom).await;
    assert!(matches!(
        probe.latest.borrow().as_ref().unwrap().resolution,
        PcbWiringResolution::Current { .. }
    ));

    // Start a forced Left request, then let the accepted input advance to Right. The Right
    // request succeeds first; the delayed old-board reply must not replace that result.
    probe.latest.borrow().as_ref().unwrap().on_resolve.call(());
    until_calls(&probe, &mut dom, 3).await;
    *probe.input.borrow_mut() = Some(board_input("right", 2));
    probe.version.set(1);
    tick(&mut dom).await;
    until_calls(&probe, &mut dom, 4).await;
    reply_at(&probe, 1, Ok(test_plan("right", 0)));
    tick(&mut dom).await;
    assert!(matches!(
        probe.latest.borrow().as_ref().unwrap().resolution,
        PcbWiringResolution::Current { ref identity, .. }
            if identity.scope.board_id == "right"
    ));
    reply(&probe, Ok(test_plan("left", 0)));
    tick(&mut dom).await;
    assert!(matches!(
        probe.latest.borrow().as_ref().unwrap().resolution,
        PcbWiringResolution::Current { ref identity, .. }
            if identity.scope.board_id == "right"
    ));
}

#[wasm_bindgen_test]
async fn mounted_resolution_ignores_old_same_board_reply_after_controller_revision_changes() {
    let (probe, mut dom) =
        mounted_probe(board_input_with_controller("left", 10, 10, Some("left/U1")));

    until_calls(&probe, &mut dom, 1).await;

    // Model a controller selection edit accepted on the same board: both the snapshot token
    // and document revision advance, while the controller ID in the saved board config changes.
    *probe.input.borrow_mut() = Some(board_input_with_controller("left", 11, 11, Some("left/U2")));
    probe.version.set(1);
    tick(&mut dom).await;
    until_calls(&probe, &mut dom, 2).await;

    let mut new_plan = test_plan("left", 11);
    new_plan.controller_part_id = Some("left/U2".into());
    reply_at(&probe, 1, Ok(new_plan));
    tick(&mut dom).await;
    assert!(matches!(
        probe.latest.borrow().as_ref().unwrap().resolution,
        PcbWiringResolution::Current { ref identity, ref plan }
            if identity.scope.board_id == "left"
                && identity.token == SnapshotToken(11)
                && identity.revision == 11
                && plan.revision == 11
                && plan.controller_part_id.as_deref() == Some("left/U2")
    ));

    let mut old_plan = test_plan("left", 10);
    old_plan.controller_part_id = Some("left/U1".into());
    reply(&probe, Ok(old_plan));
    tick(&mut dom).await;
    assert!(matches!(
        probe.latest.borrow().as_ref().unwrap().resolution,
        PcbWiringResolution::Current { ref identity, ref plan }
            if identity.scope.board_id == "left"
                && identity.token == SnapshotToken(11)
                && identity.revision == 11
                && plan.revision == 11
                && plan.controller_part_id.as_deref() == Some("left/U2")
    ));
}
