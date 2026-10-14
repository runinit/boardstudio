use super::*;
use boardstudio_web_ui_shared::test_wait::wait_until;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[derive(Clone)]
struct Probe {
    action: Rc<RefCell<Option<super::super::part_placement::ComponentPlacementAction>>>,
    definition: boardstudio_core::model::PartDefinition,
    apply_to_key: bool,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    rsx! {
        PartsInspectorPlacementAction {
            entry: CatalogEntry {
                definition: Rc::new(probe.definition.clone()),
                source: catalogue::CatalogueSource::Imported,
            },
            apply_to_key: probe.apply_to_key,
            busy: false,
            on_place: EventHandler::new(move |action| *probe.action.borrow_mut() = Some(action)),
        }
    }
}

#[derive(Clone)]
struct AddObjectProbe {
    snapshot: AcceptedSnapshot,
    scope: Scope,
    actions: Rc<RefCell<Vec<super::super::part_placement::ComponentPlacementAction>>>,
}

#[component]
fn add_object_host() -> Element {
    let probe = use_context::<Rc<AddObjectProbe>>();
    let query = use_signal(String::new);
    let layout_target = use_signal(|| None);
    let generation = use_signal(|| 0_u64);
    use_context_provider(|| PartsSelectionGeneration(generation));
    rsx! {
        AddObjectComponentChooser {
            snapshot: probe.snapshot.clone(),
            scope: Some(probe.scope.clone()),
            layout_target,
            query,
            on_browse: EventHandler::default(),
            on_place: EventHandler::new(move |action| probe.actions.borrow_mut().push(action)),
        }
    }
}

fn definition(id: &str, kind: &str) -> boardstudio_core::model::PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": kind,
        "kind": kind,
        "courtyard": [{"x": -2.0, "y": -2.0}, {"x": 2.0, "y": 2.0}],
        "pads": []
    }))
    .unwrap()
}

async fn mounted_action(
    root_id: &str,
    definition: boardstudio_core::model::PartDefinition,
    apply_to_key: bool,
) -> (
    Rc<RefCell<Option<super::super::part_placement::ComponentPlacementAction>>>,
    web_sys::HtmlElement,
) {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let action = Rc::new(RefCell::new(None));
    let dom = VirtualDom::new(host);
    dom.provide_root_context(Probe {
        action: action.clone(),
        definition,
        apply_to_key,
    });
    dioxus_web::launch::launch_virtual_dom(dom, dioxus_web::Config::new().rootnode(root.into()));
    wait_until("the Parts Inspector placement control mounts", || {
        document
            .query_selector(&format!("#{root_id} .m1-parts-place-component"))
            .unwrap()
            .is_some()
    })
    .await;
    let button = document
        .query_selector(&format!("#{root_id} .m1-parts-place-component"))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    (action, button)
}

fn add_object_snapshot() -> AcceptedSnapshot {
    let mut document = boardstudio_core::model::ProjectDoc::empty("placement-ui", "Placement UI");
    for (id, name) in [("board-main", "Main"), ("board-mirror", "Mirror")] {
        document.boards.push(boardstudio_core::model::Board {
            id: id.into(),
            name: name.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
    }
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology": "split", "transport": "wired", "instances": [], "boards": [],
            "pairs": [{
                "id": "pair-main-mirror", "members": ["instance-main", "instance-mirror"],
                "mapping": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0},
                "construction": {
                    "method": "printed", "mount": "rigid", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 2.0, "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.2
                },
                "halves": {"sourceBoardId": "board-main", "mirrorBoardId": "board-mirror"}
            }]
        }))
        .unwrap(),
    );
    AcceptedSnapshot {
        token: SnapshotToken(4),
        session_epoch: boardstudio_application::SessionEpoch(3),
        document: Arc::new(document),
        scene: Arc::new(
            serde_json::from_value(serde_json::json!({
                "revision": 0, "transactionId": "placement-ui", "changedIds": [],
                "transforms": [], "matrixScenes": [], "contours": [], "boardContours": [],
                "boardReadiness": [], "findings": [],
                "readiness": {"layout": false, "outline": false, "pcb": false, "case": false}
            }))
            .unwrap(),
        ),
    }
}

#[wasm_bindgen_test]
fn mirror_only_affordance_requires_one_distinct_owned_mirror_board() {
    let snapshot = add_object_snapshot();
    assert!(has_unique_mirror_board_pair(
        &snapshot.document,
        "board-mirror"
    ));
    assert!(!has_unique_mirror_board_pair(
        &snapshot.document,
        "board-main"
    ));

    let mut ambiguous = (*snapshot.document).clone();
    let duplicate_pair = ambiguous.hardware.as_ref().unwrap().pairs[0].clone();
    ambiguous
        .hardware
        .as_mut()
        .unwrap()
        .pairs
        .push(duplicate_pair);
    assert!(!has_unique_mirror_board_pair(&ambiguous, "board-mirror"));

    let mut duplicate_board = (*snapshot.document).clone();
    duplicate_board
        .boards
        .push(duplicate_board.boards[1].clone());
    assert!(!has_unique_mirror_board_pair(
        &duplicate_board,
        "board-mirror"
    ));

    let mut unpaired = (*snapshot.document).clone();
    unpaired.hardware = None;
    assert!(!has_unique_mirror_board_pair(&unpaired, "board-mirror"));
}

async fn mounted_add_object_chooser(
    root_id: &str,
    board_id: &str,
) -> (
    Rc<RefCell<Vec<super::super::part_placement::ComponentPlacementAction>>>,
    web_sys::Element,
) {
    let snapshot = add_object_snapshot();
    let scope = Scope {
        session_epoch: snapshot.session_epoch,
        document_id: snapshot.document.id.clone(),
        board_id: board_id.into(),
        instance_id: None,
    };
    let actions = Rc::new(RefCell::new(Vec::new()));
    let probe = Rc::new(AddObjectProbe {
        snapshot,
        scope,
        actions: actions.clone(),
    });
    let mut ui_definition = definition("catalog:ui-part", "passive");
    ui_definition.name = "Reset switch".into();
    let entry = CatalogEntry {
        definition: Rc::new(ui_definition),
        source: catalogue::CatalogueSource::Imported,
    };
    let entries = Rc::new(vec![entry]);
    let loader = CatalogueLoadTestPort(Rc::new(move |_| {
        let entries = entries.clone();
        Box::pin(async move { Ok(entries) })
    }));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(root_id);
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(add_object_host);
    dom.provide_root_context(probe);
    dom.provide_root_context(loader);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the Add Reset switch action mounts", || {
        root.query_selector("button[aria-label='Add Reset switch']")
            .unwrap()
            .is_some()
    })
    .await;
    (actions, root)
}

#[wasm_bindgen_test]
async fn mounted_add_object_chooser_keeps_default_and_exposes_mirror_only_only_on_mirror_board() {
    let (actions, mirror_root) =
        mounted_add_object_chooser("mirror-only-choice", "board-mirror").await;
    let default = mirror_root
        .query_selector("button[aria-label='Add Reset switch']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let mirror_only = mirror_root
        .query_selector("button[aria-label='Add Reset switch to this half only']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    default.click();
    mirror_only.click();
    assert_eq!(
        *actions.borrow(),
        vec![
            super::super::part_placement::ComponentPlacementAction::AddObject {
                definition_id: "catalog:ui-part".into(),
                kind: boardstudio_core::model::PartKind::Passive,
            },
            super::super::part_placement::ComponentPlacementAction::AddMirrorOnlyObject {
                definition_id: "catalog:ui-part".into(),
                kind: boardstudio_core::model::PartKind::Passive,
            },
        ]
    );
    mirror_root.remove();

    let (_source_actions, source_root) =
        mounted_add_object_chooser("source-only-choice", "board-main").await;
    assert!(
        source_root
            .query_selector(".m1-mirror-only-choice")
            .unwrap()
            .is_none()
    );
    source_root.remove();
}

#[wasm_bindgen_test]
async fn inspector_disables_a_non_input_definition_for_a_selected_key() {
    let passive = definition("imported:passive", "passive");
    assert!(!matrix_input_available(&passive));
    let (action, button) = mounted_action("parts-passive-test", passive, true).await;

    assert!(button.has_attribute("disabled"));
    assert!(
        button
            .parent_element()
            .unwrap()
            .text_content()
            .unwrap()
            .contains("Clear the key selection to place it as a standalone component.")
    );
    button.click();
    assert!(action.borrow().is_none());
}

#[wasm_bindgen_test]
async fn inspector_click_routes_a_switch_to_the_selected_key_action() {
    let mut switch = definition("generator:switch", "switch");
    switch.pads = vec![input_pad("one", "1"), input_pad("two", "2")];
    assert!(matrix_input_available(&switch));
    let (action, button) = mounted_action("parts-switch-test", switch, true).await;

    assert!(!button.has_attribute("disabled"));
    assert_eq!(
        button.text_content().as_deref(),
        Some("Apply to selected key")
    );
    button.click();
    assert_eq!(
        action.borrow().as_ref(),
        Some(
            &super::super::part_placement::ComponentPlacementAction::PartsInspector {
                definition_id: "generator:switch".into(),
                kind: boardstudio_core::model::PartKind::Switch,
            }
        )
    );
}

#[wasm_bindgen_test]
async fn inspector_disables_a_switch_without_independent_press_contacts() {
    let switch = definition("generator:switch-no-input", "switch");
    assert!(!matrix_input_available(&switch));
    let (action, button) = mounted_action("parts-switch-no-input-test", switch, true).await;

    assert!(button.has_attribute("disabled"));
    button.click();
    assert!(action.borrow().is_none());
}

fn input_pad(id: &str, number: &str) -> boardstudio_core::model::Pad {
    boardstudio_core::model::Pad {
        id: id.into(),
        number: number.into(),
        at: boardstudio_core::model::Vec2::default(),
        size: boardstudio_core::model::Vec2 { x: 1.0, y: 1.0 },
        shape: boardstudio_core::model::PadShape::Rect,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    }
}
