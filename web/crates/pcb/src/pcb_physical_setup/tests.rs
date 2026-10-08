//! Mounted regressions for the physical-setup controller over the real WASM Runtime
//! (Session and Core in process, memory persistence), launched as a real VirtualDom.
//!
//! Coverage limitation, recorded for the root review: the former module held these
//! scenarios as native `#[test]`s inside this wasm32-only parent, so it compiled on
//! neither target and executed never. The tests below restore executable coverage of the
//! behavior the PendingEdits migration changed — silent landing, silent retirement and
//! owner attribution at dispatch and settlement. The scenarios that relied on the native
//! stub's synchronous VirtualDom polling remain uncovered by any executing test:
//! preparation results arriving after their source was replaced, unmount races, and the
//! stale-scope CaseTransport variant. The CasePcbDesign reassignment navigation keeps a
//! pure unit assertion in `controller.rs` (case_reassignment_scope_transition_tests),
//! and wiring-panel equivalents cover the topology landing and undo path in
//! `pcb_wiring/queued_edit_tests.rs`.
use super::*;
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_application::Event;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

type ProposalReply = Result<boardstudio_core::model::ProjectDoc, String>;

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    /// Whether the project-guide stage observes its owner as active.
    active: Rc<Cell<bool>>,
    /// The generation the owner check and the rendered source answer for.
    generation: Rc<Cell<u64>>,
    /// The reply queued before the preparation call that will consume it.
    reply: Rc<RefCell<Option<ProposalReply>>>,
    /// The sender of the preparation call currently parked on a later reply.
    prepare_sender: Rc<RefCell<Option<futures_channel::oneshot::Sender<ProposalReply>>>>,
    /// The mount the host rendered last.
    mount: Rc<RefCell<Option<PhysicalSetupMount>>>,
    /// Bumped by the test to force a host render without a runtime change.
    poke: Rc<RefCell<Option<Signal<u64>>>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let runtime = probe.runtime.clone();
    let version = use_signal(|| 0_u64);
    {
        let runtime = runtime.clone();
        use_hook(move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }));
        });
    }
    let poke = use_signal(|| 0_u64);
    {
        let probe = probe.clone();
        use_hook(move || *probe.poke.borrow_mut() = Some(poke));
    }
    let _ = version();
    let _ = poke();
    let mut generation = use_signal(|| 0_u64);
    if *generation.peek() != probe.generation.get() {
        generation.set(probe.generation.get());
    }
    let is_current = {
        let runtime = runtime.clone();
        let probe = probe.clone();
        Rc::new(move |owner: &OwnerIdentity, strict: bool| {
            let model = runtime.model();
            model.accepted.as_ref().is_some_and(|accepted| {
                owner.generation == probe.generation.get()
                    && owner.document_id == accepted.document.id
                    && owner.session_epoch == accepted.session_epoch
                    && owner.board_id == model.active_board_id
                    && owner.instance_id == model.active_instance_id
                    && (!strict
                        || (owner.token == accepted.token
                            && owner.revision == accepted.document.revision))
            })
        })
    };
    let prepare = {
        let probe = probe.clone();
        Rc::new(
            move |_: boardstudio_core::model::ProjectDoc, _: crate::physical_setup::SetupIntent| {
                let probe = probe.clone();
                // A channel so a reply queued after the task parks still wakes it; a
                // poll_fn alone would leave the parked task waiting forever.
                Box::pin(async move {
                    let (sender, receiver) = futures_channel::oneshot::channel();
                    match probe.reply.borrow_mut().take() {
                        Some(value) => {
                            let _ = sender.send(value);
                        }
                        None => {
                            *probe.prepare_sender.borrow_mut() = Some(sender);
                        }
                    }
                    receiver
                        .await
                        .unwrap_or_else(|_| Err("the preparation gate was dropped".into()))
                }) as super::controller::ProposalFuture
            },
        )
    };
    let mount = use_controller(
        runtime,
        version,
        generation,
        {
            let active = probe.active.clone();
            Rc::new(move || active.get())
        },
        crate::InstanceSelection(use_signal(|| None)),
        is_current,
        prepare,
    );
    *probe.mount.borrow_mut() = Some(mount);
    rsx! {}
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

async fn mounted(id: &str) -> (Probe, web_sys::Element) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, fixture_document(id, 1)).await;
    let probe = Probe {
        runtime,
        active: Rc::new(Cell::new(true)),
        generation: Rc::new(Cell::new(0)),
        reply: Rc::new(RefCell::new(None)),
        prepare_sender: Rc::new(RefCell::new(None)),
        mount: Rc::new(RefCell::new(None)),
        poke: Rc::new(RefCell::new(None)),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(format!("physical-setup-test-{id}").as_str());
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered().await;
    (probe, root)
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(30).await;
}

async fn settle(runtime: &Rc<Runtime>) {
    for _ in 0..12 {
        support::run_pending(runtime).await;
        rendered().await;
    }
}

fn poke(probe: &Probe) {
    if let Some(mut poke) = probe.poke.borrow_mut().as_ref().copied() {
        poke.set(poke() + 1);
    }
}

fn mount(probe: &Probe) -> PhysicalSetupMount {
    probe
        .mount
        .borrow()
        .as_ref()
        .expect("the host rendered")
        .clone()
}

fn reversible(probe: &Probe) -> boardstudio_core::model::ProjectDoc {
    let mut document = (*probe.runtime.model().accepted.unwrap().document).clone();
    document
        .parameters
        .insert("reversibleLayout".into(), serde_json::json!(true));
    document
}

fn accepted_reversible(runtime: &Runtime) -> Option<serde_json::Value> {
    runtime
        .model()
        .accepted
        .unwrap()
        .document
        .parameters
        .get("reversibleLayout")
        .cloned()
}

/// Queue a preparation reply. Before the controller asks for preparation it is consumed
/// by the next call; afterwards it wakes the parked preparation task.
fn queue_reply(probe: &Probe, value: ProposalReply) {
    match probe.prepare_sender.borrow_mut().take() {
        Some(sender) => {
            let _ = sender.send(value);
        }
        None => *probe.reply.borrow_mut() = Some(value),
    }
}

/// Hold the next Core reply, submit a reversible-layout setup, and observe it in flight.
async fn hold_in_flight(probe: &Probe) -> futures_channel::oneshot::Sender<()> {
    queue_reply(probe, Ok(reversible(probe)));
    let (mut entered, release) = support::gate_next_core_reply(&probe.runtime);
    mount(probe).submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
    // The panel submits from a task that first awaits preparation, so the Core request is
    // only held after a turn of the local executor. Yield, drive whatever the turn held,
    // and repeat until the gate reports the request in flight.
    let mut held = false;
    for _ in 0..12 {
        rendered().await;
        support::drive_pending(&probe.runtime);
        if let Ok(Some(())) = entered.try_recv() {
            held = true;
            break;
        }
    }
    assert!(held, "the setup request reaches the gated Core executor");
    poke(probe);
    rendered().await;
    release
}

#[wasm_bindgen_test]
async fn exact_accepted_proposal_lands_silently_without_leaking_to_another_context() {
    let (probe, root) = mounted("setup-landing").await;
    let runtime = probe.runtime.clone();
    let release = hold_in_flight(&probe).await;
    assert!(
        mount(&probe).projection.busy,
        "the setup edit holds the busy gate while in flight"
    );
    release.send(()).unwrap();
    settle(&runtime).await;
    let projection = mount(&probe).projection;
    assert!(!projection.busy, "landing clears busy without a status");
    assert!(
        projection.project_feedback.is_none(),
        "landing shows the accepted value without a status message"
    );
    assert!(
        projection.feedback.is_none(),
        "a project result must not appear in Case"
    );
    assert_eq!(
        accepted_reversible(&runtime),
        Some(serde_json::json!(true)),
        "the exact accepted proposal landed"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn hidden_success_settles_silently_for_its_exact_accepted_proposal() {
    let (probe, root) = mounted("setup-hidden-success").await;
    let runtime = probe.runtime.clone();
    let release = hold_in_flight(&probe).await;
    probe.active.set(false);
    release.send(()).unwrap();
    settle(&runtime).await;
    let projection = mount(&probe).projection;
    assert!(!projection.busy, "the hidden landing still clears busy");
    assert!(projection.project_feedback.is_none());
    assert!(projection.feedback.is_none());
    poke(&probe);
    rendered().await;
    probe.active.set(true);
    poke(&probe);
    rendered().await;
    assert!(
        mount(&probe).projection.project_feedback.is_none(),
        "a success recorded while hidden still shows no status when the view returns"
    );
    assert_eq!(
        accepted_reversible(&runtime),
        Some(serde_json::json!(true)),
        "the accepted proposal still landed"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn an_in_flight_setup_retires_silently_when_its_owner_departs() {
    let (probe, root) = mounted("setup-retire").await;
    let release = hold_in_flight(&probe).await;
    probe.active.set(false);
    poke(&probe);
    rendered().await;
    // The settlement loop answers for a departed owner while the reply is still held.
    settle(&probe.runtime).await;
    let projection = mount(&probe).projection;
    assert!(!projection.busy, "the retired setup clears busy silently");
    assert!(
        projection.project_feedback.is_none(),
        "retirement is silent, per the ADR-0005 amendment"
    );
    assert!(projection.feedback.is_none());
    release.send(()).unwrap();
    settle(&probe.runtime).await;
    assert!(mount(&probe).projection.project_feedback.is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn a_stage_hidden_during_preparation_submits_no_edit() {
    let (probe, root) = mounted("setup-hidden-preparation").await;
    let runtime = probe.runtime.clone();
    mount(&probe).submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
    rendered().await;
    assert!(
        mount(&probe).projection.busy,
        "preparation holds the busy gate"
    );
    probe.active.set(false);
    queue_reply(&probe, Ok(reversible(&probe)));
    settle(&runtime).await;
    let projection = mount(&probe).projection;
    assert!(
        !projection.busy,
        "the hidden stage clears busy without an edit"
    );
    assert!(projection.project_feedback.is_none());
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "the hidden stage fails async admission before any edit begins"
    );
    assert_eq!(
        accepted_reversible(&runtime),
        None,
        "the abandoned preparation applied nothing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn retained_rendered_controls_cannot_retarget_another_project() {
    let (probe, root) = mounted("setup-retarget").await;
    let runtime = probe.runtime.clone();
    let stale = mount(&probe);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document("setup-retarget-b", 1),
    });
    settle(&runtime).await;
    support::take_held_effects(&runtime);
    queue_reply(&probe, Ok(reversible(&probe)));
    stale.submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
    rendered().await;
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "a retained control must not submit an edit for the replaced project"
    );
    assert!(!stale.projection.busy);
    assert!(mount(&probe).projection.project_feedback.is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn retained_rendered_controls_reject_replaced_token_and_generation() {
    // A replaced accepted revision leaves the rendered owner's token and revision stale.
    let (probe, root) = mounted("setup-stale-revision").await;
    let runtime = probe.runtime.clone();
    let stale = mount(&probe);
    runtime.submit(Event::Open {
        operation_id: runtime.operation(),
        document: fixture_document("setup-stale-revision", 2),
    });
    settle(&runtime).await;
    support::take_held_effects(&runtime);
    queue_reply(&probe, Ok(reversible(&probe)));
    stale.submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
    rendered().await;
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "a stale token and revision cannot submit"
    );
    root.remove();

    // An advanced generation leaves the rendered owner stale even with the same source.
    let (probe, root) = mounted("setup-stale-generation").await;
    let runtime = probe.runtime.clone();
    let stale = mount(&probe);
    probe.generation.set(1);
    poke(&probe);
    rendered().await;
    queue_reply(&probe, Ok(reversible(&probe)));
    stale.submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
    rendered().await;
    assert!(
        support::take_held_effects(&runtime).is_empty(),
        "a stale generation cannot submit"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_failed_save_reports_once_for_its_exact_owner() {
    let (probe, root) = mounted("setup-failure").await;
    let runtime = probe.runtime.clone();
    support::fail_next_persist(&runtime, "disk");
    queue_reply(&probe, Ok(reversible(&probe)));
    mount(&probe).submit(PhysicalSetupIntent::ProjectReversibleLayout(true));
    settle(&runtime).await;
    let projection = mount(&probe).projection;
    assert!(
        !projection.busy,
        "the failed setup no longer holds the busy gate"
    );
    assert!(
        projection
            .project_feedback
            .as_deref()
            .is_some_and(|message| message.contains("disk")),
        "the failure reports for its exact owner"
    );
    assert!(
        projection.feedback.is_none(),
        "a project failure must not appear in Case"
    );
    root.remove();
}
