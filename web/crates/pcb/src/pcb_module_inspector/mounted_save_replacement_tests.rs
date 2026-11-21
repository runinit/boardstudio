use super::*;
use boardstudio_application::{
    Completion, Effect, OperationId, SaveResult, Scope as AppScope, Session,
};
use boardstudio_core::CoreEngine;
use boardstudio_model::model::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::wait_until_step;
use gloo_timers::future::TimeoutFuture;
use std::{collections::VecDeque, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlInputElement;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

struct Probe {
    runtime: Rc<Runtime>,
    initial_scope: AppScope,
    module_id: String,
}

fn mounted_editor_host() -> dioxus::prelude::Element {
    let probe = use_context::<Rc<Probe>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    use_hook(|| {
        probe.runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version.set(version() + 1);
        }));
    });
    let _ = version();
    let selected_context = use_signal(|| {
        Some(super::super::objects::ScopedTreeContext {
            scope: probe.initial_scope.clone(),
            context: super::super::objects::TreeContext::MountedModule {
                board_id: probe.initial_scope.board_id.clone(),
                module_id: probe.module_id.clone(),
            },
        })
    });
    let model = probe.runtime.model();
    let Some(snapshot) = model.accepted else {
        return rsx! { p { "No accepted project" } };
    };
    let Some(scope) = probe.runtime.scope() else {
        return rsx! { p { "No current project scope" } };
    };
    if scope != probe.initial_scope {
        return rsx! {
            p { id: "replacement-project", "{snapshot.document.name}" }
        };
    }
    super::inspector(InspectorInput {
        runtime: probe.runtime.clone(),
        snapshot,
        scope,
        module_id: probe.module_id.clone(),
        selected_context,
        on_place_component: EventHandler::default(),
    })
}

fn module_document(id: &str, name: &str, x: f64) -> ProjectDoc {
    let mut document = ProjectDoc::empty(id, name);
    document.boards.push(Board {
        id: "board-collision".into(),
        name: "Main board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    document.module_definitions.push(ModuleDefinition {
        id: "definition-collision".into(),
        catalogue_row: None,
        name: "Test module".into(),
        family: "test".into(),
        variant: "test".into(),
        source: HardwareSource {
            repository: "test".into(),
            revision: "1".into(),
            path: "test.module.json".into(),
            license: "test".into(),
            sha256: None,
            upstream_status: None,
        },
        board: ModuleBoard {
            contours: Vec::new(),
            holes: Vec::new(),
            thickness: Some(1.6),
            pcb_source: None,
            round_drills: Vec::new(),
        },
        mounts: Vec::new(),
        volumes: Vec::new(),
        openings: Vec::new(),
        models: Vec::new(),
        candidate_models: Vec::new(),
        gates: Vec::new(),
        interfaces: Vec::new(),
        electrical: ModuleElectrical {
            protocol: ModuleProtocol::Nonstandard,
            required_signals: Vec::new(),
            logic_voltage: None,
            current_ma: None,
            i2c_address: None,
            pullup_ohms: None,
            driver: None,
            rotary_profile: None,
        },
        constituents: Vec::new(),
        circuit: None,
    });
    document.modules.push(MountedModule {
        id: "placement-collision".into(),
        definition_id: "definition-collision".into(),
        host_board_id: "board-collision".into(),
        host_instance_id: None,
        host_face: Side::Front,
        facing_face: Side::Front,
        at: Vec2 { x, y: 2.0 },
        rotation: 0.0,
        gap: 1.0,
        attachment: ModuleAttachment::Board,
        detached: false,
        connection: None,
        service_clearance: 0.0,
        mount_supports: Vec::new(),
    });
    document
}

fn accepted(document: ProjectDoc) -> (Session, CoreEngine) {
    let mut session = Session::new(boardstudio_core::CoreEngine::validate_open);
    let mut core = CoreEngine::new();
    let effects = session.submit(boardstudio_application::Event::Open {
        operation_id: OperationId(1),
        document,
    });
    let mut effects = VecDeque::from(effects);
    while let Some(effect) = effects.pop_front() {
        match effect {
            Effect::Core {
                request_id,
                executor_epoch,
                request,
                ..
            } => effects.extend(session.complete(Completion::Core {
                request_id,
                executor_epoch,
                reply: Box::new(core.handle(*request)),
            })),
            Effect::Persist {
                save_attempt_id, ..
            } => {
                effects.extend(session.complete(Completion::Persist {
                    save_attempt_id,
                    result: SaveResult::Committed,
                }));
            }
            _ => {}
        }
    }
    assert!(session.read_model().accepted.is_some());
    (session, core)
}

async fn settle() {
    TimeoutFuture::new(80).await;
}

async fn with_stage_timeout<T>(future: impl std::future::Future<Output = T>, stage: &str) -> T {
    let mut future = Box::pin(future);
    let mut timeout = Box::pin(TimeoutFuture::new(5_000));
    let completed = std::future::poll_fn(|context| {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(context) {
            return std::task::Poll::Ready(Some(value));
        }
        if let std::task::Poll::Ready(()) = timeout.as_mut().poll(context) {
            return std::task::Poll::Ready(None);
        }
        std::task::Poll::Pending
    })
    .await;
    completed.unwrap_or_else(|| panic!("timed out during mounted Save stage: {stage}"))
}

async fn wait_for_persisted_module_x(
    runtime: &Runtime,
    expected_revision: u64,
    expected_x: f64,
) -> ProjectDoc {
    for _ in 0..100 {
        if let Some(document) = with_stage_timeout(
            runtime.store.load_document("old-module-project".into()),
            "IndexedDB read request/transaction completion",
        )
        .await
        .expect("read old project from IndexedDB")
            && document.revision == expected_revision
            && document.modules[0].at.x == expected_x
        {
            return document;
        }
        TimeoutFuture::new(10).await;
    }
    panic!(
        "old-project IndexedDB save did not reach revision {expected_revision} with X={expected_x}"
    );
}

#[wasm_bindgen_test]
async fn module_face_choice_and_detached_checkbox_are_keyboard_accessible() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    let (session, core) = accepted(module_document(
        "module-control-project",
        "Control project",
        1.0,
    ));
    let runtime = support::new_runtime();
    support::install(&runtime, session, core);
    let probe = Rc::new(Probe {
        initial_scope: runtime.scope().unwrap(),
        runtime: runtime.clone(),
        module_id: "placement-collision".into(),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_editor_host);
    dom.provide_root_context(probe);
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("module face choice mounts", || {
        by_role_name(&root, "button", "Module host face").is_some()
    })
    .await;
    let face = by_role_name(&root, "button", "Module host face").unwrap();
    face.focus().unwrap();
    let event = web_sys::KeyboardEventInit::new();
    event.set_key("Enter");
    event.set_bubbles(true);
    event.set_cancelable(true);
    face.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
    )
    .unwrap();
    wait_until("module face list opens", || {
        by_role_name(&root, "listbox", "Module host face").is_some()
    })
    .await;
    let list = by_role_name(&root, "listbox", "Module host face").unwrap();
    let event = web_sys::KeyboardEventInit::new();
    event.set_key("End");
    event.set_bubbles(true);
    event.set_cancelable(true);
    list.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
    )
    .unwrap();
    wait_until("back face is focused", || {
        by_role_name(&root, "option", "Back")
            .is_some_and(|option| option.matches(":focus").unwrap())
    })
    .await;
    let back = by_role_name(&root, "option", "Back").unwrap();
    let event = web_sys::KeyboardEventInit::new();
    event.set_key("Enter");
    event.set_bubbles(true);
    event.set_cancelable(true);
    back.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
    )
    .unwrap();
    wait_until("selected face is shown and focus returns", || {
        face.text_content()
            .is_some_and(|text| text.contains("Back"))
            && face.matches(":focus").unwrap()
    })
    .await;
    let detached = by_role_name(&root, "checkbox", "Detached").unwrap();
    assert_eq!(
        detached.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    detached.click();
    wait_until("detached toggles", || {
        detached.get_attribute("aria-checked").as_deref() == Some("true")
    })
    .await;
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn placement_fields_queue_two_commits_keep_the_draft_and_undo_in_order() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let runtime = support::new_runtime();
    support::open_document(
        &runtime,
        module_document("module-queue-project", "Queue project", 1.0),
    )
    .await;
    let probe = Rc::new(Probe {
        initial_scope: runtime.scope().unwrap(),
        runtime: runtime.clone(),
        module_id: "placement-collision".into(),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_editor_host);
    dom.provide_root_context(probe);
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    let x = by_role_name(&root, "spinbutton", "X (mm)")
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    let save = by_role_name(&root, "button", "Save placement").unwrap();
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let input_event = web_sys::EventInit::new();
    input_event.set_bubbles(true);
    x.set_value("7.25");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap())
        .unwrap();
    settle().await;
    save.click();
    settle().await;
    support::drive_pending(&runtime);
    with_stage_timeout(entered, "first placement Core gate")
        .await
        .unwrap();
    settle().await;
    assert!(!save.has_attribute("disabled"));
    assert!(!x.disabled());
    assert_eq!(x.value(), "7.25");
    x.set_value("9.5");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap())
        .unwrap();
    settle().await;
    save.click();
    settle().await;
    assert_eq!(x.value(), "9.5");
    release.send(()).unwrap();
    wait_until_step("the queued placement commits land", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.modules[0].at.x == 9.5 && x.value() == "9.5"
    })
    .await;
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step(
        "the first Undo restores the earlier accepted coordinate",
        async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.modules[0].at.x == 7.25
                && x.value() == "7.25"
        },
    )
    .await;
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step(
        "the second Undo restores the baseline coordinate",
        async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.modules[0].at.x == 1.0
        },
    )
    .await;
    root.remove();
}

#[wasm_bindgen_test]
async fn circuit_removal_disables_siblings_and_rejects_a_retained_sibling_handler() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let mut document = module_document("module-removal-project", "Copy project", 1.0);
    document.module_definitions[0].circuit = Some(ModuleCircuit {
        definitions: Vec::new(),
        parts: Vec::new(),
        nets: Vec::new(),
        ports: BTreeMap::new(),
        adaptations: Vec::new(),
    });
    for id in ["circuit/first", "circuit/sibling"] {
        document.embedded_circuits.push(EmbeddedCircuit {
            id: id.into(),
            definition_id: "definition-collision".into(),
            host_board_id: "board-collision".into(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            ports: BTreeMap::new(),
        });
    }
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    let probe = Rc::new(Probe {
        initial_scope: runtime.scope().unwrap(),
        runtime: runtime.clone(),
        module_id: "placement-collision".into(),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_editor_host);
    dom.provide_root_context(probe);
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    let first = by_role_name(&root, "button", "Remove copy circuit/first").unwrap();
    let sibling = by_role_name(&root, "button", "Remove copy circuit/sibling").unwrap();
    let (entered, release) = support::gate_next_core_reply(&runtime);
    first.click();
    settle().await;
    support::drive_pending(&runtime);
    with_stage_timeout(entered, "circuit removal Core gate")
        .await
        .unwrap();
    settle().await;
    assert!(first.has_attribute("disabled"));
    assert!(sibling.has_attribute("disabled"));

    // Invoke the retained rendered sibling handler despite the browser's native
    // disabled-button admission. The handler must also enforce the shared action.
    sibling.remove_attribute("disabled").unwrap();
    sibling.click();
    sibling.set_attribute("disabled", "").unwrap();
    settle().await;
    release.send(()).unwrap();
    wait_until_step(
        "the circuit removal lands and its siblings re-enable",
        async || {
            support::run_pending(&runtime).await;
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .embedded_circuits
                .len()
                == 1
                && by_role_name(&root, "button", "Remove copy circuit/first").is_none()
                && by_role_name(&root, "button", "Remove copy circuit/sibling").is_some()
                && !sibling.has_attribute("disabled")
        },
    )
    .await;
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        accepted
            .document
            .embedded_circuits
            .iter()
            .map(|copy| copy.id.as_str())
            .collect::<Vec<_>>(),
        vec!["circuit/sibling"],
        "the retained sibling handler must not queue another removal"
    );
    assert!(
        by_role_name(&root, "button", "Remove copy circuit/first").is_none()
            && by_role_name(&root, "button", "Remove copy circuit/sibling").is_some(),
        "the removed row unmounts without a dropped-Signal settlement write"
    );
    assert!(
        !sibling.has_attribute("disabled"),
        "the shared control re-enables after drain"
    );
    assert!(!root.text_content().unwrap().contains("Saved"));
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn newer_placement_returning_to_baseline_survives_landing_and_clean_accepted_updates() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    let runtime = support::new_runtime();
    support::open_document(
        &runtime,
        module_document("module-baseline-project", "Baseline project", 1.0),
    )
    .await;
    let probe = Rc::new(Probe {
        initial_scope: runtime.scope().unwrap(),
        runtime: runtime.clone(),
        module_id: "placement-collision".into(),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_editor_host);
    dom.provide_root_context(probe);
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;
    let x = by_role_name(&root, "spinbutton", "X (mm)")
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    let save = by_role_name(&root, "button", "Save placement").unwrap();
    let input = web_sys::EventInit::new();
    input.set_bubbles(true);
    let (entered, release) = support::gate_next_core_reply(&runtime);
    x.set_value("7");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input).unwrap())
        .unwrap();
    settle().await;
    save.click();
    settle().await;
    support::drive_pending(&runtime);
    with_stage_timeout(entered, "return-to-baseline placement Core gate")
        .await
        .unwrap();
    x.set_value("1");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input).unwrap())
        .unwrap();
    settle().await;
    release.send(()).unwrap();
    wait_until_step(
        "the earlier accepted placement commits under the newer draft",
        async || {
            support::run_pending(&runtime).await;
            runtime.model().accepted.unwrap().document.modules[0].at.x == 7.0
        },
    )
    .await;
    assert_eq!(
        x.value(),
        "1",
        "returning to the previous accepted value is still a newer draft"
    );

    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step("the Undo restores the baseline coordinate", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.modules[0].at.x == 1.0
    })
    .await;
    assert_eq!(x.value(), "1");
    runtime.submit(boardstudio_application::Event::Redo {
        operation_id: runtime.operation(),
    });
    wait_until_step("the clean draft follows Undo and Redo", async || {
        support::run_pending(&runtime).await;
        x.value() == "7"
    })
    .await;

    let _external = boardstudio_web_runtime::edit_ticket::EditTicket::begin(
        &runtime,
        "external-module-placement",
        Some("placement".into()),
        EditResolver::new("pcb-module", |accepted| {
            let mut instance = accepted.document.modules[0].clone();
            instance.at.x = 9.0;
            Resolution::submit(
                vec![instance.id.clone()],
                EditOperation::SetMountedModule {
                    instance: Box::new(instance),
                    definition: None,
                    host_connector_definition: None,
                },
            )
        }),
    );
    wait_until_step("the external accepted update lands", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.modules[0].at.x == 9.0 && x.value() == "9"
    })
    .await;
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn failed_placement_restores_accepted_coordinates() {
    use boardstudio_web_test_runtime::project_name_test_support as support;
    for preserve_draft in [false, true] {
        let runtime = support::new_runtime();
        support::open_document(
            &runtime,
            module_document("module-failure-project", "Queue project", 1.0),
        )
        .await;
        let probe = Rc::new(Probe {
            initial_scope: runtime.scope().unwrap(),
            runtime: runtime.clone(),
            module_id: "placement-collision".into(),
        });
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_editor_host);
        dom.provide_root_context(probe);
        let _mounted = MountedFixture::new(root.clone(), dom);
        settle().await;
        let x = by_role_name(&root, "spinbutton", "X (mm)")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let save = by_role_name(&root, "button", "Save placement").unwrap();
        support::fail_next_core_reply(&runtime, "injected placement refusal");
        let input_event = web_sys::EventInit::new();
        input_event.set_bubbles(true);
        x.set_value("7.25");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap())
            .unwrap();
        settle().await;
        save.click();
        settle().await;
        if preserve_draft {
            x.set_value("8");
            x.dispatch_event(
                &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
            )
            .unwrap();
            settle().await;
        }
        wait_until_step(
            "the failed placement restores the accepted coordinate",
            async || {
                support::run_pending(&runtime).await;
                runtime.model().accepted.unwrap().document.modules[0].at.x == 1.0
                    && x.value() == if preserve_draft { "8" } else { "1" }
                    && root
                        .text_content()
                        .is_some_and(|text| text.contains("injected placement refusal"))
            },
        )
        .await;
        runtime.unsubscribe();
        root.remove();
    }
}

#[wasm_bindgen_test]
async fn mounted_save_persist_settlement_cannot_cross_replacement_with_colliding_ids() {
    let (session, core) = accepted(module_document("old-module-project", "Old project", 1.0));
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let initial = session
        .read_model()
        .accepted
        .as_ref()
        .expect("old project accepted")
        .document
        .clone();
    runtime
        .store
        .save_document(&initial, &std::collections::BTreeMap::new())
        .await
        .expect("seed the old project's durable baseline");
    boardstudio_web_test_runtime::project_name_test_support::install(&runtime, session, core);
    let initial_scope = runtime.scope().expect("old project has scope");
    let probe = Rc::new(Probe {
        runtime: runtime.clone(),
        initial_scope,
        module_id: "placement-collision".into(),
    });
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("mounted-module-save-replacement-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_editor_host);
    dom.provide_root_context(probe);
    let _mounted = MountedFixture::new(root.clone(), dom);
    settle().await;

    let (entered, release) =
        boardstudio_web_test_runtime::project_name_test_support::gate_next_persist(&runtime);
    let operation = boardstudio_web_test_runtime::project_name_test_support::observe_next(&runtime);
    let x = by_role_name(&root, "spinbutton", "X (mm)")
        .expect("mounted placement X input exists")
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    x.focus().unwrap();
    x.set_value("7.25");
    let input = web_sys::EventInit::new();
    input.set_bubbles(true);
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input).unwrap())
        .unwrap();
    settle().await;
    assert_eq!(
        by_role_name(&root, "spinbutton", "X (mm)")
            .expect("placement X input remains mounted")
            .dyn_into::<HtmlInputElement>()
            .unwrap()
            .value(),
        "7.25",
        "the mounted X field retains the dispatched draft value after rerender"
    );
    by_role_name(&root, "button", "Save placement")
        .expect("Save placement button exists")
        .click();
    settle().await;
    assert!(
        !by_role_name(&root, "button", "Save placement")
            .expect("Save placement button remains mounted")
            .has_attribute("disabled"),
        "placement fields queue freely while their earlier ticket is pending"
    );
    assert_eq!(
        by_role_name(&root, "spinbutton", "X (mm)")
            .expect("placement X input remains mounted during Save")
            .dyn_into::<HtmlInputElement>()
            .unwrap()
            .value(),
        "7.25",
        "the mounted draft survives the Save handler's rerender"
    );
    boardstudio_web_test_runtime::project_name_test_support::drive_pending(&runtime);
    with_stage_timeout(
        entered,
        "mounted Save click dispatched but actual Persist gate was not entered",
    )
    .await
    .expect("the mounted Save reaches the actual Session Persist effect");

    let saving = runtime.model();
    let saving_snapshot = saving
        .accepted
        .expect("old project remains accepted while saving");
    assert_eq!(saving.lifecycle, Lifecycle::Saving);
    assert_eq!(saving_snapshot.document.id, "old-module-project");
    assert_eq!(
        saving_snapshot.document.modules[0].at.x, 1.0,
        "the accepted read model remains the last durable document while Persist is pending"
    );
    let saved_revision = saving_snapshot.document.revision + 1;

    let (replacement, _replacement_core) = accepted(module_document(
        "new-module-project",
        "Replacement project",
        4.0,
    ));
    boardstudio_web_test_runtime::project_name_test_support::replace_session(&runtime, replacement);
    settle().await;
    assert_eq!(
        root.query_selector("#replacement-project")
            .unwrap()
            .expect("replacement owner retires the old mounted editor")
            .text_content()
            .as_deref(),
        Some("Replacement project")
    );
    assert!(
        by_role_name(&root, "button", "Save placement").is_none(),
        "old placement feedback/editor is absent in the replacement owner"
    );
    let replacement_before = runtime.model().accepted.expect("replacement accepted");
    assert_eq!(replacement_before.document.id, "new-module-project");
    assert_eq!(replacement_before.document.modules[0].at.x, 4.0);
    let revision = replacement_before.document.revision;

    release
        .send(())
        .expect("release the old project persistence");
    let persisted = wait_for_persisted_module_x(&runtime, saved_revision, 7.25).await;
    assert_eq!(persisted.id, "old-module-project");
    assert_eq!(persisted.revision, saved_revision);
    assert_eq!(persisted.modules[0].at.x, 7.25);
    let replacement_after = runtime
        .model()
        .accepted
        .expect("replacement remains accepted");
    assert_eq!(replacement_after.document.id, "new-module-project");
    assert_eq!(replacement_after.document.revision, revision);
    assert_eq!(replacement_after.document.modules[0].at.x, 4.0);
    assert!(
        operation.borrow().is_none(),
        "after the old IndexedDB write completes, the replacement Session has no old-operation feedback"
    );
    assert!(
        by_role_name(&root, "button", "Save placement")
            .and_then(|button| button.parent_element())
            .and_then(|actions| actions.next_element_sibling())
            .is_none_or(|next| !next.matches("p[role='status']").unwrap()),
        "a landed placement shows no status message"
    );
    let _ = runtime
        .store
        .delete_project("old-module-project".into())
        .await;
    let _ = document.body().unwrap().remove_child(&root);
    runtime.unsubscribe();
}
