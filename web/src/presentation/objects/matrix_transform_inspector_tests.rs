use super::*;
use crate::presentation::objects::{
    ScopedTreeContext, TreeContext, use_workspace_matrix_transform,
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, ReadModel, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{
    Board, EditOperation, Matrix, MatrixAssembly, MatrixCell, MatrixScene, MatrixSceneCell, Part,
    PartDefinition, PartKind, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
};
use std::{rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    selected: ScopedTreeContext,
    root_id: &'static str,
}

#[component]
fn matrix_transform_host() -> Element {
    let probe = use_context::<Probe>();
    let version = use_signal(|| 0u64);
    let selected_context = use_signal(|| Some(probe.selected.clone()));
    let workspace = use_signal(|| "Layout");
    let scope_generation = use_signal(|| 1u64);
    let splay_affect = use_signal(|| boardstudio_core::model::MatrixSplayAffect::Column);
    let mount = use_workspace_matrix_transform(
        probe.runtime.clone(),
        version,
        selected_context,
        workspace,
        scope_generation,
        splay_affect,
        "Layout",
    );
    rsx! {
        MatrixTransformInspector { mount, on_pick_splay_origin: |_| {} }
    }
}

fn definition(id: &str, name: &str) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: name.into(),
        kind: PartKind::Controller,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    }
}

fn fixture() -> (ReadModel, ScopedTreeContext) {
    let scope = Scope {
        session_epoch: SessionEpoch(81),
        document_id: "matrix-transform-mounted".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let mut document = ProjectDoc::empty(&scope.document_id, "Matrix transform fixture");
    document.revision = 12;
    document.boards.push(Board {
        id: scope.board_id.clone(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["key-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions = vec![
        definition("key-definition", "Key"),
        definition("assembly-current/definition/led", "Current LED snapshot"),
        definition("assembly-other/definition/led", "Unrelated LED snapshot"),
        definition("replacement-definition", "Replacement LED"),
    ];
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: "key-part".into(),
        definition_id: "key-definition".into(),
        reference: "K1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.matrices.push(Matrix {
        id: "matrix".into(),
        name: Some("Keys".into()),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "key-definition".into(),
        part_ids: vec!["key-part".into()],
        board_id: Some(scope.board_id.clone()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: vec![MatrixCell {
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("key-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "led".into(),
                definition_id: "assembly-current/definition/led".into(),
                offset: Vec2::default(),
                rotation: None,
                side: None,
            }],
            assemblies_local: None,
        }],
    });
    let scene = SceneDelta {
        module_scenes: vec![],
        revision: 12,
        transaction_id: "matrix-transform-mounted-fixture".into(),
        changed_ids: vec![],
        transforms: vec![],
        matrix_scenes: vec![MatrixScene {
            matrix_id: "matrix".into(),
            cells: vec![MatrixSceneCell {
                row: 0,
                column: 0,
                enabled: true,
                member_id: Some("key-part".into()),
                pose: Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                },
            }],
            columns: vec![],
        }],
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
    };
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(13),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(scene),
    };
    let model = ReadModel {
        lifecycle: Lifecycle::Ready,
        durability: Durability::Saved { revision: 12 },
        accepted: Some(snapshot),
        active_board_id: scope.board_id.clone(),
        ..ReadModel::default()
    };
    let selected = ScopedTreeContext {
        scope,
        context: TreeContext::Key {
            matrix_id: "matrix".into(),
            row: 0,
            column: 0,
        },
    };
    (model, selected)
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(60).await;
}

#[wasm_bindgen_test]
async fn mounted_attached_component_choices_are_current_item_scoped_and_emit_the_selected_replacement()
 {
    let (model, selected) = fixture();
    let runtime = Runtime::new().expect("browser Runtime initializes");
    runtime.set_layout_component_inspector_test_state(model, Some(selected.scope.clone()));
    let probe = Probe {
        runtime,
        selected,
        root_id: "matrix-transform-inspector-mounted-test",
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(probe.root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let mut dom = VirtualDom::new(matrix_transform_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    settle().await;

    let selector = document
        .query_selector(&format!(
            "#{} select[aria-label='Replace led']",
            probe.root_id
        ))
        .unwrap()
        .expect("the mounted Inspector renders the attached LED selector");
    assert!(
        selector
            .query_selector("option[value='assembly-current/definition/led']")
            .unwrap()
            .is_some(),
        "the selector must retain this attached item's current snapshot"
    );
    assert!(
        selector
            .query_selector("option[value='replacement-definition']")
            .unwrap()
            .is_some(),
        "the choice list must include the actual replacement definition"
    );
    assert!(
        selector
            .query_selector("option[value='assembly-other/definition/led']")
            .unwrap()
            .is_none(),
        "the choice list must exclude an unrelated assembly snapshot"
    );

    js_sys::Reflect::set(
        selector.as_ref(),
        &"value".into(),
        &"replacement-definition".into(),
    )
    .unwrap();
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    selector
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
        .unwrap();
    settle().await;
    let events = probe.runtime.take_layout_component_inspector_test_events();
    assert!(
        matches!(
            events.as_slice(),
            [Event::Edit { command, .. }]
                if matches!(&command.operation, EditOperation::SetMatrix { matrix, .. }
                    if matrix.id == "matrix" && matrix.cells.iter().find(|cell| cell.row == 0 && cell.column == 0)
                        .is_some_and(|cell| cell.assemblies.iter().any(|assembly|
                            assembly.id == "led" && assembly.definition_id == "replacement-definition")))
        ),
        "the mounted controller must submit the selected replacement against the current key: {events:?}"
    );
    root.remove();
}
