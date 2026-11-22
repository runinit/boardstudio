//! Common project fixtures and Core edit drivers for the integration suites.
use boardstudio_core::{CoreEngine, model::*};
use serde_json::{Value, json};

pub(crate) fn outline_document() -> ProjectDoc {
    let mut document: ProjectDoc =
        serde_json::from_str(include_str!("fixtures/synthetic-outline.json")).unwrap();
    document.format_version = boardstudio_core::document_format::CURRENT_VERSION;
    document
}

pub(crate) fn scene_only(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}

pub(crate) fn outline_edit(
    engine: &mut CoreEngine,
    revision: u64,
    operation: serde_json::Value,
) -> SceneDelta {
    let operation = serde_json::from_value(operation).expect("outline command contract");
    scene_only(engine.handle(CoreRequest::Edit {
        id: "outline-edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "outline-edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

pub(crate) fn outline_snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => *document,
        reply => panic!("{reply:?}"),
    }
}

pub(crate) fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

pub(crate) fn open(doc: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    (engine, scene, doc)
}

pub(crate) fn edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    scene(engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

pub(crate) fn matrix_snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() })).1
}

pub(crate) fn preview(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> SceneDelta {
    match engine.handle(CoreRequest::Edit {
        id: "preview".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "preview".into(),
            phase: EditPhase::Preview,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}

pub(crate) fn matrix_definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]},
        {"id":"led","name":"LED","kind":"passive","courtyard":[],"pads":[]},
        {"id":"sensor","name":"Sensor","kind":"passive","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

pub(crate) fn matrix(id: &str, rows: u32, columns: u32) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

pub(crate) fn json_open(engine: &mut CoreEngine, id: &str, document: &Value) -> Value {
    json_request(engine, json!({"kind":"open","id":id,"document":document}))
}

pub(crate) fn json_snapshot(engine: &mut CoreEngine) -> Value {
    json_request(engine, json!({"kind":"snapshot","id":"snapshot"}))["document"].clone()
}

pub(crate) fn json_request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

pub(crate) fn matrix_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("p", "Project");
    doc.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "edge".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc
}

pub(crate) fn authored_main_matrix() -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![
            matrix_member("main", 0, 0),
            matrix_member("main", 0, 1),
            matrix_member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

pub(crate) fn matrix_member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

pub(crate) fn key_part(id: &str, x: f64, y: f64) -> Part {
    Part {
        anchor: None,
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

pub(crate) fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    }
}

pub(crate) fn mechanical_configuration() -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": "board",
        "method": "printed",
        "mount": "rigid",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.5,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.4,
        "batteryHeight": 4.0,
        "bottomThickness": 3.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.3,
        "bottomStyle": "shell",
        "middleFrame": false,
        "openingAllowance": 0.25,
        "mounts": [],
        "closureMounts": [],
        "profiles": []
    }))
    .unwrap()
}

pub(crate) fn mounting_hole_template() -> PartDefinition {
    serde_json::from_value(json!({
        "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
        "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
    }))
    .unwrap()
}

pub(crate) fn part_position(doc: &ProjectDoc, id: &str) -> Vec2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
        .at
}

pub(crate) fn matrix_companion<'a>(scene: &'a SceneDelta, id: &str) -> &'a MatrixScene {
    scene
        .matrix_scenes
        .iter()
        .find(|item| item.matrix_id == id)
        .unwrap_or_else(|| panic!("missing matrix scene {id}"))
}

pub(crate) fn linked_outline_settings() -> serde_json::Value {
    serde_json::json!({
        "corners": "sharp", "size": 2.0, "bridgeWidth": 10.0,
        "repair": {
            "enabled": true, "maximumGapSpan": 20.0,
            "minimumConnectionWidth": 2.0, "edgeClearance": 0.0, "keepGaps": []
        }
    })
}
