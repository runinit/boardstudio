use super::*;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, ReadModel, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{Board, ProjectDoc, Readiness, SceneDelta};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct DimensionProbe {
    committed: Rc<RefCell<Vec<f64>>>,
}

fn mounted_dimension_host() -> Element {
    let probe = use_context::<DimensionProbe>();
    let committed = probe.committed.clone();
    let on_commit = use_callback(move |value: f64| committed.borrow_mut().push(value));
    rsx! {
        div { id: "outline-dimension-test-root",
            OutlineDimension {
                label: "Outline margin",
                value: 4.0,
                minimum: 0.0,
                editable: true,
                on_commit,
            }
        }
    }
}

fn mount_dimension() -> (DimensionProbe, web_sys::Element) {
    let probe = DimensionProbe {
        committed: Rc::default(),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("outline-dimension-mount");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_dimension_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

async fn settle_dimension() {
    gloo_timers::future::TimeoutFuture::new(35).await;
}

fn dimension_input() -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(
            "#outline-dimension-mount #outline-dimension-test-root input[aria-label='Outline margin']",
        )
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
}

fn send_input(input: &web_sys::HtmlInputElement, value: &str) {
    input.set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
        .unwrap();
}

fn send_key(input: &web_sys::HtmlInputElement, key: &str) {
    let init = web_sys::KeyboardEventInit::new();
    init.set_key(key);
    init.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
}

#[wasm_bindgen_test]
async fn outline_dimension_enter_commits_and_escape_restores_accepted_value() {
    let (probe, root) = mount_dimension();
    settle_dimension().await;
    let input = dimension_input();

    input.focus().unwrap();
    send_input(&input, "5.25");
    send_key(&input, "Enter");
    settle_dimension().await;
    assert_eq!(probe.committed.borrow().as_slice(), &[5.25]);

    input.focus().unwrap();
    send_input(&input, "9");
    send_key(&input, "Escape");
    settle_dimension().await;
    assert_eq!(input.value(), "4");
    input.blur().unwrap();
    settle_dimension().await;
    assert_eq!(
        probe.committed.borrow().as_slice(),
        &[5.25],
        "Escape restores the accepted baseline without submitting"
    );
    root.remove();
}

#[derive(Clone)]
struct InspectorProbe {
    runtime: Rc<crate::runtime::Runtime>,
    scope: Scope,
}

fn mounted_outline_inspector_host() -> Element {
    let probe = use_context::<InspectorProbe>();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let workspace = use_signal(|| "Layout");
    let generation = use_signal(|| 1u64);
    let selected = use_signal(|| {
        Some(super::super::objects::ScopedTreeContext {
            scope: probe.scope.clone(),
            context: super::super::objects::TreeContext::Outline {
                board_id: probe.scope.board_id.clone(),
            },
        })
    });
    let (projection, _) =
        use_outline_lifecycle(probe.runtime.clone(), selected, workspace, generation);
    let _ = version();
    rsx! {
        style { {include_str!("../../assets/m1.css")} }
        if let Some(projection) = projection {
            OutlineVersionInspector { projection }
        }
    }
}

fn mounted_outline_inspector() -> (InspectorProbe, web_sys::Element) {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "outline-inspector-doc".into(),
        board_id: "outline-board".into(),
        instance_id: None,
    };
    let mut document = ProjectDoc::empty("outline-inspector-doc", "Outline fixture");
    document.revision = 9;
    document.boards.push(Board {
        id: scope.board_id.clone(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(13),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 9,
            transaction_id: "outline-inspector-fixture".into(),
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
    let model = ReadModel {
        lifecycle: Lifecycle::Ready,
        durability: Durability::Saved { revision: 9 },
        accepted: Some(snapshot),
        active_board_id: scope.board_id.clone(),
        ..ReadModel::default()
    };
    let runtime = crate::runtime::Runtime::new().expect("browser Runtime initializes");
    runtime.set_layout_component_inspector_test_state(model, Some(scope.clone()));
    let probe = InspectorProbe { runtime, scope };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("outline-inspector-mount");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_outline_inspector_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

#[wasm_bindgen_test]
async fn mounted_outline_inspector_generates_through_the_production_owner() {
    let (probe, root) = mounted_outline_inspector();
    settle_dimension().await;
    let button = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("#outline-inspector-mount button.m1-outline-action")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    button.click();
    settle_dimension().await;
    let events = probe.runtime.take_layout_component_inspector_test_events();
    let operation = match events.as_slice() {
        [boardstudio_application::Event::Edit { operation_id, .. }] => *operation_id,
        _ => panic!("the production owner should submit exactly one edit"),
    };
    let created = match events.as_slice() {
        [boardstudio_application::Event::Edit { command, .. }] => {
            let boardstudio_core::model::EditOperation::ReplaceDocument { document } =
                &command.operation
            else {
                panic!("automatic outline creation uses the existing document edit")
            };
            document
                .boards
                .iter()
                .find(|board| board.id == probe.scope.board_id)
                .is_some_and(|board| {
                    board.outline_ids.iter().any(|id| {
                        document.outline.iter().any(|feature| {
                            feature.id() == id
                                && matches!(
                                    feature,
                                    boardstudio_core::model::OutlineFeature::PartEnvelope { .. }
                                )
                        })
                    })
                })
        }
        _ => false,
    };
    assert!(
        created,
        "the mounted Inspector emits a generated feature edit"
    );
    assert!(
        probe
            .runtime
            .settle_layout_component_inspector_test_operation(
                operation,
                boardstudio_application::TerminalOutcome::Rejected("fixture rejection".into()),
            )
    );
    settle_dimension().await;
    root.remove();
}
