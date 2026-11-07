//! Ticket 011: mounted draft-safety tests for the Layout explicit-submit owners.
//!
//! Seam: real editor components under a real Runtime (Session + Core via
//! `open_document`), driven through the real `Runtime::begin_update` gate at
//! both Update Now openers. The entry buttons are fixture chrome calling
//! `begin_update(UpdateOpener::Toast|Header)` — the opener is the entry-point
//! distinction at the gate; the production toast/header buttons live behind
//! `pub(super)` in the frozen notifications/header modules (008 proved that
//! chrome, 010 owns trusted activation). The deferral is observed through the
//! production `Runtime::update_dialog()` state and scripted `UpdateSignals`
//! adoptions, and the area/remedy copy is asserted on the production
//! `UpdateBlocker::UnresolvedDraft` values that feed 008's `blocker_message`.
//! No injected invalid marker, save label, or worker label is used as proof.

use super::inspector::Inspector;
use super::inspector::{LayoutComponentInspector, LayoutComponentInspectorLifetime};
use super::*;
use crate::runtime::Runtime;
use crate::runtime::project_name_test_support as support;
use boardstudio_application::{Event, Scope, SelectionMode};
use boardstudio_core::model::{Board, Part, Pose2, ProjectDoc, Side, Vec2};
use boardstudio_web_host::host::{UpdateSignals, UpdateWatch};
use boardstudio_web_host::offline::{AdoptError, UpdateSnapshot};
use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog, UpdateOpener};
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::wait_until;
use dioxus::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlInputElement;

type Watchers = Rc<RefCell<Vec<Option<Rc<dyn Fn()>>>>>;

struct FakeUpdateSignals {
    snapshot: Rc<RefCell<UpdateSnapshot>>,
    watchers: Watchers,
    adoptions: Rc<Cell<u32>>,
}

impl FakeUpdateSignals {
    fn new(snapshot: UpdateSnapshot) -> Self {
        Self {
            snapshot: Rc::new(RefCell::new(snapshot)),
            watchers: Rc::new(RefCell::new(Vec::new())),
            adoptions: Rc::new(Cell::new(0)),
        }
    }
}

impl UpdateSignals for FakeUpdateSignals {
    fn snapshot(&self) -> UpdateSnapshot {
        *self.snapshot.borrow()
    }

    fn watch(&self, on_change: Rc<dyn Fn()>) -> Box<dyn UpdateWatch> {
        struct Stop;
        impl UpdateWatch for Stop {
            fn stop(&mut self) {}
        }
        self.watchers.borrow_mut().push(Some(on_change));
        Box::new(Stop)
    }

    fn adopt_waiting(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AdoptError>>>> {
        let adoptions = self.adoptions.clone();
        Box::pin(async move {
            adoptions.set(adoptions.get() + 1);
            Ok(())
        })
    }

    fn reload(&self) {}
}

fn waiting_update() -> UpdateSnapshot {
    UpdateSnapshot {
        waiting: true,
        installing: false,
        controlled: true,
    }
}

fn mount_owner(make: fn() -> Element, runtime: &Rc<Runtime>) -> web_sys::Element {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(make);
    dom.provide_root_context(runtime.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    root
}

fn legacy_position_fixture() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    use_hook(move || {
        let subscriber = runtime.clone();
        subscriber.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }));
    });
    let _ = version();
    rsx! {
        Inspector {}
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| runtime.begin_update(UpdateOpener::Toast)
            },
            "Update Now (toast)"
        }
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| runtime.begin_update(UpdateOpener::Header)
            },
            "Update Now (header)"
        }
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| runtime.cancel_update_dialog()
            },
            "Cancel update"
        }
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| {
                    runtime.submit(Event::SelectParts {
                        operation_id: runtime.operation(),
                        part_ids: vec![],
                        range_part_ids: vec![],
                        mode: SelectionMode::Replace,
                    });
                }
            },
            "Deselect"
        }
    }
}

async fn mounted_legacy_position() -> (Rc<Runtime>, web_sys::Element, Rc<FakeUpdateSignals>) {
    let runtime = support::new_runtime();
    let mut document = ProjectDoc::empty("legacy-position-draft-doc", "Legacy position");
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap(),
    );
    document.parts.push(Part {
        id: "a".into(),
        definition_id: "switch:base".into(),
        reference: "A".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["a".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    support::open_document(&runtime, document).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["a".into()],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    let signals = Rc::new(FakeUpdateSignals::new(waiting_update()));
    runtime.observe_update_waiting(signals.clone());
    let root = mount_owner(legacy_position_fixture, &runtime);
    wait_until("the legacy position Inspector renders", || {
        by_role_name(&root, "spinbutton", "X (mm)").is_some()
    })
    .await;
    (runtime, root, signals)
}

fn type_number(root: &web_sys::Element, label: &str, value: &str) -> HtmlInputElement {
    let field = by_role_name(root, "spinbutton", label)
        .unwrap_or_else(|| panic!("the {label} field mounts"))
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    field.set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    field
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
        .unwrap();
    field
}

fn field_value(root: &web_sys::Element, label: &str) -> String {
    by_role_name(root, "spinbutton", label)
        .unwrap_or_else(|| panic!("the {label} field mounts"))
        .dyn_into::<HtmlInputElement>()
        .unwrap()
        .value()
}

fn deferred_draft_blockers(runtime: &Rc<Runtime>) -> Vec<UpdateBlocker> {
    match runtime.update_dialog() {
        Some(UpdateDialog::Deferred { blockers }) => blockers,
        other => panic!("expected a deferred update dialog, got {other:?}"),
    }
}

fn assert_legacy_position_deferral(blockers: &[UpdateBlocker]) {
    assert_eq!(
        blockers.len(),
        1,
        "only the legacy position draft defers: {blockers:?}"
    );
    match &blockers[0] {
        UpdateBlocker::UnresolvedDraft { area, remedy } => {
            assert_eq!(area, "Legacy position");
            assert_eq!(
                remedy,
                "Press Enter or Apply position to save, or press Escape to cancel"
            );
        }
        other => panic!("expected the legacy position draft blocker, got {other:?}"),
    }
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_valid_draft_defers_update_now_at_the_toast() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_legacy_position_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the draft never adopts");
    assert_eq!(
        field_value(&root, "X (mm)"),
        "12.5",
        "the draft text survives the deferral"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_valid_draft_defers_update_now_at_the_header() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "Y (mm)", "-3.25");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (header)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_legacy_position_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the draft never adopts");
    assert_eq!(
        field_value(&root, "Y (mm)"),
        "-3.25",
        "the draft text survives the deferral"
    );
    root.remove();
}

fn assert_blockers_contain_legacy_position(blockers: &[UpdateBlocker]) {
    assert!(
        blockers.iter().any(|blocker| matches!(
            blocker,
            UpdateBlocker::UnresolvedDraft { area, remedy }
            if area == "Legacy position"
                && remedy == "Press Enter or Apply position to save, or press Escape to cancel"
        )),
        "the legacy position draft stays blocking: {blockers:?}"
    );
}

fn accepted_position_x(runtime: &Rc<Runtime>) -> f64 {
    runtime
        .model()
        .accepted
        .expect("an accepted document is open")
        .document
        .parts[0]
        .pose
        .at
        .x
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_invalid_draft_defers_update_now() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    // Realistic invalid text: the commit path refuses it, so no submission
    // exists and no invalid marker is set anywhere in this form.
    type_number(&root, "X (mm)", "not-a-number");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_legacy_position_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the invalid draft never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_invalid_draft_defers_update_now_at_the_header() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "not-a-number");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (header)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_legacy_position_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the invalid draft never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_apply_landing_clears_the_deferral() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Apply position")
        .unwrap()
        .click();
    support::run_pending(&runtime).await;
    wait_until("the apply lands", || accepted_position_x(&runtime) == 12.5).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the landed apply adopts", || signals.adoptions.get() == 1).await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after landing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_revert_to_baseline_clears_the_key() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    type_number(&root, "X (mm)", "0");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (header)")
        .unwrap()
        .click();
    wait_until("the reverted draft adopts", || signals.adoptions.get() == 1).await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after revert"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_failed_apply_keeps_newer_typing_deferred() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "5");
    support::run_pending(&runtime).await;
    support::fail_next_persist(&runtime, "legacy position durable write failure");
    by_role_name(&root, "button", "Apply position")
        .unwrap()
        .click();
    // Newer input before the older submission settles as a failure.
    type_number(&root, "X (mm)", "7");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_blockers_contain_legacy_position(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the failed save never adopts");
    assert_eq!(
        field_value(&root, "X (mm)"),
        "7",
        "the newer draft text survives the failure"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_newer_typing_survives_an_older_settlement() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Apply position")
        .unwrap()
        .click();
    type_number(&root, "X (mm)", "7");
    support::run_pending(&runtime).await;
    assert_eq!(
        accepted_position_x(&runtime),
        5.0,
        "the older submission lands first"
    );
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_blockers_contain_legacy_position(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the newer draft never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_deselect_retires_the_owner() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Deselect").unwrap().click();
    wait_until("the inspector shows its empty branch", || {
        by_role_name(&root, "spinbutton", "X (mm)").is_none()
    })
    .await;
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the retired owner adopts", || signals.adoptions.get() == 1).await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after deselect"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_hidden_instance_keeps_blocking() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    // Hidden but mounted: the out-of-sight draft stays protected.
    let panel = root
        .query_selector("aside[aria-label='Inspect']")
        .unwrap()
        .expect("the inspect panel mounts");
    panel.set_attribute("style", "display: none").unwrap();
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_legacy_position_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "a hidden owner still blocks");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_cancel_preserves_value_without_reload() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_legacy_position_deferral(&deferred_draft_blockers(&runtime));
    let href_before = web_sys::window()
        .unwrap()
        .location()
        .href()
        .unwrap_or_default();
    by_role_name(&root, "button", "Cancel update")
        .unwrap()
        .click();
    assert!(
        runtime.update_dialog().is_none(),
        "cancellation clears the warning"
    );
    assert_eq!(
        web_sys::window()
            .unwrap()
            .location()
            .href()
            .unwrap_or_default(),
        href_before,
        "cancellation never reloads"
    );
    assert_eq!(
        field_value(&root, "X (mm)"),
        "12.5",
        "cancellation preserves the exact draft value"
    );
    assert_eq!(signals.adoptions.get(), 0, "cancellation never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_legacy_position_in_flight_submission_exempts_only_the_current_draft() {
    let (runtime, root, signals) = mounted_legacy_position().await;
    type_number(&root, "X (mm)", "12.5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Apply position")
        .unwrap()
        .click();
    // The submission is in flight and the draft still matches it: the draft
    // leg reports resolved, so only the pending-work leg defers adoption.
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the pending-work warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_eq!(
        deferred_draft_blockers(&runtime),
        vec![UpdateBlocker::PendingWork],
        "only the in-flight submission blocks"
    );
    assert_eq!(
        signals.adoptions.get(),
        0,
        "the in-flight edit never adopts"
    );
    // Newer typing while the older submission is still in flight re-raises
    // the draft leg alongside the pending-work leg.
    type_number(&root, "X (mm)", "14");
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the newer draft also defers", || {
        matches!(runtime.update_dialog(), Some(UpdateDialog::Deferred { .. }))
    })
    .await;
    assert_blockers_contain_legacy_position(&deferred_draft_blockers(&runtime));
    support::run_pending(&runtime).await;
    root.remove();
}

fn layout_constraint_fixture() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let hook_runtime = runtime.clone();
    use_hook(move || {
        let subscriber = hook_runtime.clone();
        subscriber.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }));
    });
    let _ = version();
    let workspace = use_signal(|| "Layout");
    let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 1u64);
    let adapter =
        use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    let lifetime = use_hook(|| Rc::new(LayoutComponentInspectorLifetime::default()));
    let inspector_tab = use_signal(super::layout_workspace::LayoutInspectorTab::default);
    let inspect_open = use_signal(|| true);
    let mut show = use_signal(|| true);
    let pending_edits =
        use_signal(super::layout_component_edits::LayoutComponentInspectorEdits::default);
    let x_draft = use_signal(String::new);
    let y_draft = use_signal(String::new);
    let margin_draft = use_signal(String::new);
    let x_failure = use_signal(|| None::<String>);
    let y_failure = use_signal(|| None::<String>);
    let margin_failure = use_signal(|| None::<String>);
    let error = use_signal(|| None::<String>);
    let layout_pending = use_signal(|| false);
    let constraint_pending = use_signal(|| false);
    let remove_pending = use_signal(|| false);
    let mirrored_half_pending = use_signal(|| false);
    let model = runtime.model();
    let scope = runtime.scope();
    let next_context = scope.and_then(|scope| {
        objects::context_for_part(&model, "selected-part")
            .map(|context| objects::ScopedTreeContext { scope, context })
    });
    if selected_context.peek().clone() != next_context {
        selected_context.set(next_context.clone());
    }
    let owner_key =
        layout_component_inspector_owner_key(&model, workspace(), next_context.as_ref());
    let generation = lifetime.update(owner_key);
    let projection = layout_component_inspector_projection(
        &model,
        next_context.as_ref(),
        generation,
        scope_generation(),
    );
    let layout_owner = current_layout_owner(&runtime, workspace, &adapter);
    let action_runtime = runtime.clone();
    let action_adapter = adapter.clone();
    let action_lifetime = lifetime.clone();
    let action_workspace = workspace;
    let action_pending_edits = pending_edits;
    let on_action = EventHandler::new(move |action| {
        dispatch_layout_component_inspector_action(
            &action_runtime,
            &action_adapter,
            &layout_owner,
            &action_lifetime,
            (action_workspace, inspect_open),
            action_pending_edits,
            action,
        );
    });
    rsx! {
        if show() {
            if let Some(projection) = projection {
                LayoutComponentInspector {
                    projection,
                    inspector_tab,
                    pending_edits,
                    x: x_draft,
                    y: y_draft,
                    margin: margin_draft,
                    x_failure,
                    y_failure,
                    margin_failure,
                    error,
                    layout_pending,
                    constraint_pending,
                    remove_pending,
                    mirrored_half_pending,
                    on_action,
                }
            }
        }
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| runtime.begin_update(UpdateOpener::Toast)
            },
            "Update Now (toast)"
        }
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| runtime.begin_update(UpdateOpener::Header)
            },
            "Update Now (header)"
        }
        button {
            onclick: {
                let runtime = use_context::<Rc<Runtime>>();
                move |_| runtime.cancel_update_dialog()
            },
            "Cancel update"
        }
        button {
            onclick: move |_| show.set(!show()),
            "Toggle inspector view"
        }
    }
}

async fn mounted_layout_constraint() -> (Rc<Runtime>, web_sys::Element, Rc<FakeUpdateSignals>) {
    let runtime = support::new_runtime();
    let mut document = ProjectDoc::empty("layout-constraint-draft-doc", "Layout constraint");
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap(),
    );
    for (id, reference, x) in [("selected-part", "S", 0.0), ("source-part", "R", 19.0)] {
        document.parts.push(Part {
            id: id.into(),
            definition_id: "switch:base".into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            anchor: None,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
    }
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["selected-part".into(), "source-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    support::open_document(&runtime, document).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["selected-part".into()],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    support::run_pending(&runtime).await;
    let signals = Rc::new(FakeUpdateSignals::new(waiting_update()));
    runtime.observe_update_waiting(signals.clone());
    let root = mount_owner(layout_constraint_fixture, &runtime);
    wait_until("the constraint Offset X field renders", || {
        by_role_name(&root, "spinbutton", "Offset X (mm)").is_some()
    })
    .await;
    (runtime, root, signals)
}

fn assert_layout_constraint_deferral(blockers: &[UpdateBlocker]) {
    assert_eq!(
        blockers.len(),
        1,
        "only the layout constraint draft defers: {blockers:?}"
    );
    match &blockers[0] {
        UpdateBlocker::UnresolvedDraft { area, remedy } => {
            assert_eq!(area, "Layout constraint");
            assert_eq!(
                remedy,
                "Save or remove the constraint, or restore its saved values"
            );
        }
        other => panic!("expected the layout constraint draft blocker, got {other:?}"),
    }
}

fn assert_blockers_contain_layout_constraint(blockers: &[UpdateBlocker]) {
    assert!(
        blockers.iter().any(|blocker| matches!(
            blocker,
            UpdateBlocker::UnresolvedDraft { area, remedy }
            if area == "Layout constraint"
                && remedy == "Save or remove the constraint, or restore its saved values"
        )),
        "the layout constraint draft stays blocking: {blockers:?}"
    );
}

fn accepted_constraint_offset_x(runtime: &Rc<Runtime>) -> Option<f64> {
    use boardstudio_core::model::{Constraint, ConstraintRelation};
    let model = runtime.model();
    let snapshot = model.accepted.as_ref()?;
    match snapshot
        .document
        .constraint_driving_part("selected-part")
        .ok()??
    {
        Constraint::Offset { offset, .. } | Constraint::PartOffset { offset, .. } => Some(offset.x),
        Constraint::Framed {
            relation: ConstraintRelation::Offset { offset, .. },
            ..
        }
        | Constraint::Framed {
            relation: ConstraintRelation::PartOffset { offset, .. },
            ..
        } => Some(offset.x),
        _ => None,
    }
}

async fn save_constraint_offset(runtime: &Rc<Runtime>, root: &web_sys::Element, value: &str) {
    type_number(root, "Offset X (mm)", value);
    support::run_pending(runtime).await;
    by_role_name(root, "button", "Add constraint")
        .expect("the Add constraint control mounts")
        .click();
    support::run_pending(runtime).await;
    wait_until("the constraint save lands", || {
        accepted_constraint_offset_x(runtime).is_some()
    })
    .await;
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_invalid_draft_defers_update_now() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    // Realistic invalid text: the Save path refuses it, so no submission
    // exists and no invalid marker is set anywhere in this form.
    type_number(&root, "Offset X (mm)", "not-a-number");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_layout_constraint_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the invalid draft never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_invalid_draft_defers_update_now_at_the_header() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "not-a-number");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (header)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_layout_constraint_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the invalid draft never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_save_landing_clears_the_deferral() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    save_constraint_offset(&runtime, &root, "5").await;
    assert_eq!(
        accepted_constraint_offset_x(&runtime),
        Some(5.0),
        "the save lands the typed value"
    );
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the landed save adopts", || signals.adoptions.get() == 1).await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after landing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_revert_to_baseline_clears_the_key() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    type_number(&root, "Offset X (mm)", "0");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (header)")
        .unwrap()
        .click();
    wait_until("the reverted draft adopts", || signals.adoptions.get() == 1).await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after revert"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_failed_save_keeps_blocking() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    support::fail_next_persist(&runtime, "constraint durable write failure");
    by_role_name(&root, "button", "Add constraint")
        .expect("the Add constraint control mounts")
        .click();
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_blockers_contain_layout_constraint(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the failed save never adopts");
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "5",
        "the failed draft text survives"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_newer_typing_survives_an_older_settlement() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Add constraint")
        .expect("the Add constraint control mounts")
        .click();
    // Newer input before the older submission lands.
    type_number(&root, "Offset X (mm)", "7");
    support::run_pending(&runtime).await;
    assert_eq!(
        accepted_constraint_offset_x(&runtime),
        Some(5.0),
        "the older submission lands first"
    );
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_blockers_contain_layout_constraint(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the newer draft never adopts");
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "7",
        "the newer draft text survives the older settlement"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_accepted_update_preserves_the_open_draft() {
    use boardstudio_core::model::{
        Constraint, ConstraintRelation, EditCommand, EditOperation, EditPhase,
    };
    let (runtime, root, signals) = mounted_layout_constraint().await;
    save_constraint_offset(&runtime, &root, "5").await;
    // An open draft over a saved constraint.
    type_number(&root, "Offset X (mm)", "8");
    support::run_pending(&runtime).await;
    // An accepted update moves the constraint underneath the open draft
    // (the 007 clobber shape): the draft must survive, not be rewritten.
    let snapshot = runtime
        .model()
        .accepted
        .expect("an accepted document is open");
    let mut document = (*snapshot.document).clone();
    let mut replaced = false;
    for constraint in document.constraints.iter_mut() {
        match constraint {
            Constraint::Framed {
                relation: ConstraintRelation::Offset { offset, .. },
                ..
            }
            | Constraint::Framed {
                relation: ConstraintRelation::PartOffset { offset, .. },
                ..
            } => {
                offset.x = 6.0;
                replaced = true;
            }
            Constraint::Offset { offset, .. } | Constraint::PartOffset { offset, .. } => {
                offset.x = 6.0;
                replaced = true;
            }
            _ => {}
        }
    }
    assert!(replaced, "the fixture save produces an offset constraint");
    support::submit_fixed_command(
        &runtime,
        EditCommand {
            base_revision: document.revision,
            transaction_id: "011-unrelated-constraint-bump".into(),
            phase: EditPhase::Commit,
            target_ids: vec![document.id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    );
    support::run_pending(&runtime).await;
    assert_eq!(
        accepted_constraint_offset_x(&runtime),
        Some(6.0),
        "the accepted update lands underneath the draft"
    );
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "8",
        "the accepted update never rewrites the open draft"
    );
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_blockers_contain_layout_constraint(&deferred_draft_blockers(&runtime));
    assert_eq!(
        signals.adoptions.get(),
        0,
        "the preserved draft never adopts"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_unmount_retires_the_registration() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Toggle inspector view")
        .unwrap()
        .click();
    wait_until("the inspector unmounts", || {
        by_role_name(&root, "spinbutton", "Offset X (mm)").is_none()
    })
    .await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the retired owner adopts", || signals.adoptions.get() == 1).await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after unmount"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_hidden_instance_keeps_blocking() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    // Hidden but mounted: the out-of-sight draft stays protected.
    let section = root
        .query_selector("section[aria-label='Component inspector']")
        .unwrap()
        .expect("the inspector section mounts");
    section.set_attribute("style", "display: none").unwrap();
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_layout_constraint_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "a hidden owner still blocks");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_cancel_preserves_value_without_reload() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_layout_constraint_deferral(&deferred_draft_blockers(&runtime));
    let href_before = web_sys::window()
        .unwrap()
        .location()
        .href()
        .unwrap_or_default();
    by_role_name(&root, "button", "Cancel update")
        .unwrap()
        .click();
    assert!(
        runtime.update_dialog().is_none(),
        "cancellation clears the warning"
    );
    assert_eq!(
        web_sys::window()
            .unwrap()
            .location()
            .href()
            .unwrap_or_default(),
        href_before,
        "cancellation never reloads"
    );
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "5",
        "cancellation preserves the exact draft value"
    );
    assert_eq!(signals.adoptions.get(), 0, "cancellation never adopts");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_remove_landing_clears_the_deferral() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    save_constraint_offset(&runtime, &root, "5").await;
    by_role_name(&root, "button", "Remove")
        .expect("the Remove control mounts once a constraint is saved")
        .click();
    support::run_pending(&runtime).await;
    wait_until("the removal lands", || {
        accepted_constraint_offset_x(&runtime).is_none()
    })
    .await;
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "0",
        "the landed Remove resets the form to the no-constraint defaults"
    );
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the removed constraint adopts", || {
        signals.adoptions.get() == 1
    })
    .await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after the Remove landing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_remove_resolves_an_open_draft() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    save_constraint_offset(&runtime, &root, "5").await;
    // An open draft over the saved constraint, then the owner's own Remove
    // instead of Save: the landed Remove still resolves the draft.
    type_number(&root, "Offset X (mm)", "9");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Remove")
        .expect("the Remove control mounts once a constraint is saved")
        .click();
    support::run_pending(&runtime).await;
    wait_until("the removal lands", || {
        accepted_constraint_offset_x(&runtime).is_none()
    })
    .await;
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "0",
        "the landed Remove discards the open draft for the defaults"
    );
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the removed constraint adopts", || {
        signals.adoptions.get() == 1
    })
    .await;
    assert!(
        runtime.update_dialog().is_none(),
        "no deferral remains after the Remove landing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_in_flight_save_exempts_only_the_current_draft() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Add constraint")
        .expect("the Add constraint control mounts")
        .click();
    // The Save is in flight and the draft still matches it: the draft leg
    // reports resolved, so only the pending-work leg defers adoption.
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the pending-work warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_eq!(
        deferred_draft_blockers(&runtime),
        vec![UpdateBlocker::PendingWork],
        "only the in-flight submission blocks"
    );
    assert_eq!(
        signals.adoptions.get(),
        0,
        "the in-flight edit never adopts"
    );
    // Newer typing while the older submission is still in flight re-raises
    // the draft leg alongside the pending-work leg.
    type_number(&root, "Offset X (mm)", "7");
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the newer draft also defers", || {
        matches!(runtime.update_dialog(), Some(UpdateDialog::Deferred { .. }))
    })
    .await;
    assert_blockers_contain_layout_constraint(&deferred_draft_blockers(&runtime));
    support::run_pending(&runtime).await;
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_valid_draft_defers_update_now_at_the_toast() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset X (mm)", "5");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (toast)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_layout_constraint_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the draft never adopts");
    assert_eq!(
        field_value(&root, "Offset X (mm)"),
        "5",
        "the draft text survives the deferral"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_layout_constraint_valid_draft_defers_update_now_at_the_header() {
    let (runtime, root, signals) = mounted_layout_constraint().await;
    type_number(&root, "Offset Y (mm)", "-4");
    support::run_pending(&runtime).await;
    by_role_name(&root, "button", "Update Now (header)")
        .unwrap()
        .click();
    wait_until("the deferral warning mounts", || {
        runtime.update_dialog().is_some()
    })
    .await;
    assert_layout_constraint_deferral(&deferred_draft_blockers(&runtime));
    assert_eq!(signals.adoptions.get(), 0, "the draft never adopts");
    assert_eq!(
        field_value(&root, "Offset Y (mm)"),
        "-4",
        "the draft text survives the deferral"
    );
    root.remove();
}
