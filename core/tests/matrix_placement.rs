//! Core owns the named matrix placement parameters (`SetMatrixPlacement`) and the
//! per-column stagger axis it publishes in the scene. These tests pin the axis
//! maths to the same `location` Core uses to place keys.
use boardstudio_core::{CoreEngine, model::*};

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

fn open(doc: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    (engine, scene, doc)
}

fn edit(
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

fn snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() })).1
}

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]},
        {"id":"led","name":"LED","kind":"passive","courtyard":[],"pads":[]},
        {"id":"sensor","name":"Sensor","kind":"passive","courtyard":[],"pads":[]}
    ]))
    .unwrap()
}

fn matrix(id: &str, rows: u32, columns: u32) -> Matrix {
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

fn fixture(rows: u32, columns: u32) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut doc = ProjectDoc::empty("placement", "Placement");
    doc.definitions = definitions();
    let (mut engine, _, doc) = open(doc);
    let (scene, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: matrix("main", rows, columns),
            definitions: None,
        },
    );
    (engine, scene, doc)
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn companion(matrix: &str, row: u32, column: u32, name: &str) -> String {
    format!("{}/{name}", member(matrix, row, column))
}

fn at(doc: &ProjectDoc, id: &str) -> Vec2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
        .at
}

fn delta(before: Vec2, after: Vec2) -> Vec2 {
    Vec2 {
        x: after.x - before.x,
        y: after.y - before.y,
    }
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

fn piece<'a>(scene: &'a SceneDelta, id: &str) -> &'a MatrixScene {
    scene
        .matrix_scenes
        .iter()
        .find(|item| item.matrix_id == id)
        .unwrap_or_else(|| panic!("missing matrix scene {id}"))
}

fn stagger(column: u32, value: f64) -> EditOperation {
    EditOperation::SetMatrixPlacement {
        matrix_id: "main".into(),
        change: MatrixPlacementChange::ColumnStagger { column, value },
    }
}

/// The spec's derived axis formula, computed independently of `location`.
fn expected_axis(matrix: &Matrix, column: usize) -> Vec2 {
    let splays = &matrix
        .column_layout
        .iter()
        .map(|record| record.splay)
        .collect::<Vec<_>>();
    let theta: f64 = splays.iter().take(column).sum();
    let (sin, cos) = theta.to_radians().sin_cos();
    // R(theta) applied to local (0, 1).
    let mut point = Vec2 { x: -sin, y: cos };
    match matrix.mirror.unwrap_or(Mirror::None) {
        Mirror::X => point.x = -point.x,
        Mirror::Y => point.y = -point.y,
        Mirror::None => {}
    }
    let angle = matrix.rotation.unwrap_or(0.0).to_radians();
    let (sin, cos) = angle.sin_cos();
    Vec2 {
        x: point.x * cos - point.y * sin,
        y: point.x * sin + point.y * cos,
    }
}

fn axis_case(label: &str, configure: impl FnOnce(&mut Matrix)) {
    let (mut engine, _, mut doc) = fixture(1, 3);
    let mut matrix = doc.matrices[0].clone();
    configure(&mut matrix);
    let (mut scene, doc_configured) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    doc = doc_configured;
    for column in 0..3u32 {
        let axis = piece(&scene, "main").columns[column as usize].stagger_axis;
        assert!(
            close(axis, expected_axis(&doc.matrices[0], column as usize)),
            "{label}: column {column} axis {axis:?} disagrees with the derived formula"
        );
        let before = at(&doc, &member("main", 0, column));
        let current = doc.matrices[0]
            .column_layout
            .get(column as usize)
            .map(|record| &record.stagger)
            .copied()
            .unwrap_or(0.0);
        let (next_scene, next_doc) = edit(&mut engine, &doc, stagger(column, current + 1.0));
        let moved = delta(before, at(&next_doc, &member("main", 0, column)));
        assert!(
            close(moved, axis),
            "{label}: raising column {column} moved {moved:?}, expected {axis:?}"
        );
        doc = next_doc;
        scene = next_scene;
    }
}

fn companion_matrix() -> Matrix {
    let mut matrix = matrix("main", 1, 1);
    matrix.cells = vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column: 0,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![
            MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode".into(),
                offset: Vec2 { x: 2.0, y: 5.0 },
                rotation: None,
                side: None,
            },
            MatrixAssembly {
                id: "led".into(),
                definition_id: "led".into(),
                offset: Vec2 { x: -3.0, y: 6.0 },
                rotation: None,
                side: None,
            },
            MatrixAssembly {
                id: "sensor".into(),
                definition_id: "sensor".into(),
                offset: Vec2 { x: 1.0, y: -4.0 },
                rotation: None,
                side: None,
            },
        ],
        assemblies_local: None,
    }];
    matrix
}

fn companion_fixture() -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("companions", "Companions");
    doc.definitions = definitions();
    doc.parts.push(Part {
        anchor: None,
        id: "anchor".into(),
        definition_id: "switch".into(),
        reference: "ANCHOR".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    });
    let (mut engine, _, doc) = open(doc);
    let (_, mut doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: companion_matrix(),
            definitions: None,
        },
    );
    doc.constraints.push(Constraint::Offset {
        id: "sensor-anchor".into(),
        source_part_id: "anchor".into(),
        target_part_id: companion("main", 0, 0, "sensor"),
        offset: Vec2 { x: 1.0, y: -4.0 },
        rotation: 0.0,
    });
    let (mut engine, _, doc) = open(doc);
    // Author the LED away from its generated offset; the property is what makes
    // the companion authored, and the relative pose is what must survive.
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: companion("main", 0, 0, "led"),
                at: Vec2 { x: -10.0, y: 8.0 },
            }],
        },
    );
    (engine, doc)
}

fn matrix_doc() -> ProjectDoc {
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

fn mirrored_pair_request(base: u64, phase: &str) -> String {
    serde_json::json!({
        "kind": "edit", "id": "pair",
        "command": {"baseRevision": base, "transactionId": "pair", "phase": phase, "targetIds": [],
            "operation": {"kind": "create-mirrored-pair",
                "matrix": {"id": "left-matrix", "rows": 2, "columns": 3, "pitch": {"x": 19, "y": 19},
                    "origin": {"x": -20, "y": 5}, "mirror": "x", "definitionId": "switch", "boardId": "board", "partIds": []},
                "left": {"id": "left-layout", "name": "Left half", "boardId": "board", "matrixIds": ["left-matrix"], "partIds": []},
                "right": {"id": "right-layout", "name": "Right half", "boardId": "board", "matrixIds": ["right-matrix"], "partIds": [],
                    "mirrorLink": {"sourceId": "left-layout", "axisX": 10}}
            }
        }
    })
    .to_string()
}

fn linked_pair() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: matrix_doc(),
    });
    let response = engine.request(&mirrored_pair_request(0, "commit"));
    let (_, doc) = scene(serde_json::from_str(&response).unwrap());
    (engine, doc)
}

fn assert_reflected_keys(doc: &ProjectDoc) {
    for left in doc.parts.iter().filter(|part| {
        part.id.starts_with("matrix/left-matrix/") && part.id.matches('/').count() == 2
    }) {
        let right = doc
            .parts
            .iter()
            .find(|part| part.id == left.id.replace("left-matrix", "right-matrix"))
            .unwrap();
        assert!(
            (left.pose.at.x + right.pose.at.x - 20.0).abs() < 1e-8,
            "{left:?} {right:?}"
        );
        assert!((left.pose.at.y - right.pose.at.y).abs() < 1e-8);
        assert!((left.pose.rotation + right.pose.rotation).abs() < 1e-8);
    }
}

fn poses(doc: &ProjectDoc) -> Vec<(String, Vec2, f64)> {
    doc.parts
        .iter()
        .map(|part| (part.id.clone(), part.pose.at, part.pose.rotation))
        .collect()
}

fn assert_same_poses(before: &ProjectDoc, after: &ProjectDoc, label: &str) {
    let (before, after) = (poses(before), poses(after));
    assert_eq!(before.len(), after.len(), "{label}: part count");
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.0, b.0, "{label}: order");
        assert!(
            close(a.1, b.1),
            "{label}: {} moved {:?} -> {:?}",
            a.0,
            a.1,
            b.1
        );
        assert!((a.2 - b.2).abs() < 1e-9, "{label}: {} rotated", a.0);
    }
}

fn convert(matrix_id: &str, column: u32) -> EditOperation {
    EditOperation::SetMatrixPlacement {
        matrix_id: matrix_id.into(),
        change: MatrixPlacementChange::ColumnOffsetToStagger { column },
    }
}

/// Open a 2x3 matrix configured by `configure`, then convert `column`.
fn convert_case(label: &str, column: u32, configure: impl FnOnce(&mut Matrix)) -> Matrix {
    let (mut engine, _, doc) = fixture(2, 3);
    let mut matrix = doc.matrices[0].clone();
    configure(&mut matrix);
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    assert_eq!(
        convert_offset_available(&before.matrices[0], column),
        Ok(()),
        "{label}"
    );
    let (_, after) = edit(
        &mut engine,
        &before,
        convert(&before.matrices[0].id, column),
    );
    assert_same_poses(&before, &after, label);
    assert_eq!(
        after.matrices[0].column_layout[column as usize].offset.y,
        0.0
    );
    after.matrices[0].clone()
}

fn wire_placement(
    engine: &mut CoreEngine,
    before: &ProjectDoc,
    change: serde_json::Value,
) -> (SceneDelta, ProjectDoc) {
    let reply = engine.request(&serde_json::json!({
        "kind": "edit", "id": "named", "command": {
            "baseRevision": before.revision, "transactionId": format!("named-{}", before.revision),
            "phase": "commit", "targetIds": [], "operation": {
                "kind": "set-matrix-placement", "matrixId": "main", "change": change
            }
        }
    }).to_string());
    scene(serde_json::from_str(&reply).unwrap())
}

fn internal_constraint_translation(column: Option<u32>) {
    let (mut engine, _, document) = fixture(3, 3);
    let mut matrix = document.matrices[0].clone();
    matrix.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            offset: Vec2 { x: 1.0, y: 2.0 },
            ..Default::default()
        },
        ColumnLayout::default(),
    ];
    matrix.row_layout = vec![
        RowLayout::default(),
        RowLayout {
            offset: Vec2 { x: 2.0, y: -1.0 },
            ..Default::default()
        },
    ];
    let (_, mut document) = edit(
        &mut engine,
        &document,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    document.constraints.push(Constraint::Offset {
        id: "internal".into(),
        source_part_id: member("main", 0, 1),
        target_part_id: member("main", 1, 1),
        offset: Vec2 { x: 2.0, y: 18.0 },
        rotation: 0.0,
    });
    let (mut engine, _, before) = open(document);
    let requested = Vec2 { x: 4.0, y: -3.0 };
    let change = match column {
        Some(column) => MatrixPlacementChange::ColumnTranslation {
            column,
            delta: requested,
        },
        None => MatrixPlacementChange::MatrixTranslation { delta: requested },
    };
    let operation = EditOperation::SetMatrixPlacement {
        matrix_id: "main".into(),
        change,
    };
    let preview = match engine.handle(CoreRequest::Edit {
        id: "internal-preview".into(),
        command: EditCommand {
            base_revision: before.revision,
            transaction_id: "internal".into(),
            phase: EditPhase::Preview,
            target_ids: vec!["main".into()],
            operation: operation.clone(),
        },
    }) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("representable internal constraint preview: {other:?}"),
    };
    assert_eq!(snapshot(&mut engine), before);
    let assert_translated = |scene: &SceneDelta| {
        for row in 0..3 {
            for col in 0..3 {
                let id = member("main", row, col);
                let original = before.parts.iter().find(|part| part.id == id).unwrap();
                let actual = scene.transforms.iter().find(|part| part.id == id).unwrap();
                let movement = if column.is_none_or(|column| column == col) {
                    requested
                } else {
                    Vec2::default()
                };
                assert!(
                    close(
                        actual.pose.at,
                        Vec2 {
                            x: original.pose.at.x + movement.x,
                            y: original.pose.at.y + movement.y
                        }
                    ),
                    "{id}"
                );
                assert_eq!(actual.pose.rotation, original.pose.rotation);
            }
        }
    };
    assert_translated(&preview);
    let (committed, after) = edit(&mut engine, &before, operation);
    assert_translated(&committed);
    assert_eq!(committed.matrix_scenes, preview.matrix_scenes);
    assert_eq!(after.constraints, before.constraints);
    assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
    assert_eq!(after.matrices[0].cells, before.matrices[0].cells);
    assert_eq!(after.matrices[0].part_ids, before.matrices[0].part_ids);
    assert_eq!(after.matrices[0].pitch, before.matrices[0].pitch);
    if column.is_none() {
        assert_eq!(
            after.matrices[0].column_layout,
            before.matrices[0].column_layout
        );
    }
    for (part, original) in after.parts.iter().zip(&before.parts) {
        assert_eq!(part.id, original.id);
        assert_eq!(part.reference, original.reference);
        assert_eq!(part.definition_id, original.definition_id);
    }
    for (values, original) in after.matrices[0]
        .column_layout
        .iter()
        .zip(&before.matrices[0].column_layout)
    {
        assert_eq!(values.offset, original.offset);
    }
    let (_, undone) = scene(engine.handle(CoreRequest::Undo {
        id: "internal-undo".into(),
    }));
    assert_eq!(undone.parts, before.parts);
    assert_eq!(undone.matrices, before.matrices);
    assert_eq!(undone.constraints, before.constraints);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo {
        id: "internal-redo".into(),
    }));
    assert_eq!(redone.parts, after.parts);
    assert_eq!(redone.matrices, after.matrices);
    assert_eq!(redone.constraints, after.constraints);
}

#[path = "matrix_placement/placement_axes.rs"]
mod placement_axes;
#[path = "matrix_placement/placement_convert.rs"]
mod placement_convert;
#[path = "matrix_placement/placement_origins.rs"]
mod placement_origins;
