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

#[test]
fn stagger_axis_matches_location() {
    axis_case("plain", |_| {});
    axis_case("rotated", |matrix| matrix.rotation = Some(30.0));
    axis_case("mirror_x", |matrix| matrix.mirror = Some(Mirror::X));
    axis_case("mirror_y", |matrix| matrix.mirror = Some(Mirror::Y));
    axis_case("default splay origins", |matrix| {
        let values = vec![10.0, -15.0, 5.0];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.splay = 0.0;
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.splay = value;
        }
    });
    // A custom origin on column 1 is "on the column itself" for column 1, "on a
    // later column" for column 0 and "on an earlier column" for column 2.
    axis_case("custom origin on column 1", |matrix| {
        {
            let values = vec![10.0, -15.0, 5.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        {
            let values = (vec![None, Some(Vec2 { x: 25.0, y: -12.0 }), None])
                .into_iter()
                .map(|value| value.unwrap_or_default())
                .collect::<Vec<_>>();
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
    });
}

#[test]
fn column_stagger_moves_this_and_following_columns() {
    let (mut engine, _, doc) = fixture(1, 3);
    let mut matrix = doc.matrices[0].clone();
    {
        let values = vec![0.0, 2.0, -1.0];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let before: Vec<_> = (0..3)
        .map(|column| at(&doc, &member("main", 0, column)))
        .collect();
    let (_, after) = edit(&mut engine, &doc, stagger(1, 5.0));
    assert_eq!(
        after.matrices[0]
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0, 5.0, -1.0]
    );
    assert_eq!(at(&after, &member("main", 0, 0)), before[0]);
    assert!(close(
        delta(before[1], at(&after, &member("main", 0, 1))),
        Vec2 { x: 0.0, y: 3.0 }
    ));
    assert!(close(
        delta(before[2], at(&after, &member("main", 0, 2))),
        Vec2 { x: 0.0, y: 3.0 }
    ));
}

#[test]
fn column_offset_moves_only_its_column() {
    let (mut engine, _, doc) = fixture(2, 3);
    let before: Vec<_> = (0..2u32)
        .flat_map(|row| (0..3u32).map(move |column| (row, column)))
        .map(|(row, column)| at(&doc, &member("main", row, column)))
        .collect();
    let (_, after) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 1,
                value: Vec2 { x: 4.0, y: -2.0 },
            },
        },
    );
    let mut index = 0;
    for row in 0..2u32 {
        for column in 0..3u32 {
            let moved = delta(before[index], at(&after, &member("main", row, column)));
            index += 1;
            if column == 1 {
                assert!(close(moved, Vec2 { x: 4.0, y: -2.0 }), "{row}/{column}");
            } else {
                assert!(close(moved, Vec2::default()), "{row}/{column}");
            }
        }
    }
    assert_eq!(
        after.matrices[0].column_layout[1].offset,
        Vec2 { x: 4.0, y: -2.0 }
    );
}

#[test]
fn row_offset_moves_only_its_row() {
    let (mut engine, _, doc) = fixture(2, 3);
    let before: Vec<_> = (0..2u32)
        .flat_map(|row| (0..3u32).map(move |column| (row, column)))
        .map(|(row, column)| at(&doc, &member("main", row, column)))
        .collect();
    let (_, after) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::RowOffset {
                row: 1,
                value: Vec2 { x: 3.0, y: 5.0 },
            },
        },
    );
    let mut index = 0;
    for row in 0..2u32 {
        for column in 0..3u32 {
            let moved = delta(before[index], at(&after, &member("main", row, column)));
            index += 1;
            if row == 1 {
                assert!(close(moved, Vec2 { x: 3.0, y: 5.0 }), "{row}/{column}");
            } else {
                assert!(close(moved, Vec2::default()), "{row}/{column}");
            }
        }
    }
    assert_eq!(
        after.matrices[0].row_layout[1].offset,
        Vec2 { x: 3.0, y: 5.0 }
    );
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

#[test]
fn companions_follow_placement_changes() {
    let (mut engine, before) = companion_fixture();
    let key_before = at(&before, &member("main", 0, 0));
    let diode_before = at(&before, &companion("main", 0, 0, "diode"));
    let led_before = at(&before, &companion("main", 0, 0, "led"));
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 0,
                value: Vec2 { x: 4.0, y: 7.0 },
            },
        },
    );
    let key_delta = delta(key_before, at(&after, &member("main", 0, 0)));
    assert!(close(key_delta, Vec2 { x: 4.0, y: 7.0 }));
    assert!(close(
        delta(diode_before, at(&after, &companion("main", 0, 0, "diode"))),
        key_delta
    ));
    assert!(close(
        delta(led_before, at(&after, &companion("main", 0, 0, "led"))),
        key_delta
    ));
    // An explicit constraint takes priority over the attachment.
    assert!(close(
        at(&after, &companion("main", 0, 0, "sensor")),
        Vec2 { x: 1.0, y: -4.0 }
    ));
    assert_eq!(after.constraints, before.constraints);
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

#[test]
fn linked_halves_sync() {
    for (matrix_id, value) in [("left-matrix", 3.0), ("right-matrix", 4.0)] {
        let (mut engine, before) = linked_pair();
        let (_, after) = edit(
            &mut engine,
            &before,
            EditOperation::SetMatrixPlacement {
                matrix_id: matrix_id.into(),
                change: MatrixPlacementChange::ColumnStagger { column: 0, value },
            },
        );
        assert_eq!(after.matrices[0].column_layout[0].stagger, value);
        assert_eq!(after.matrices[1].column_layout[0].stagger, value);
        assert_reflected_keys(&after);
    }
}

#[test]
fn invalid_changes_error() {
    let (mut engine, _, doc) = fixture(1, 3);
    let before = snapshot(&mut engine);

    for operation in [
        stagger(3, 1.0),
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 0,
                value: Vec2 {
                    x: f64::NAN,
                    y: 0.0,
                },
            },
        },
        EditOperation::SetMatrixPlacement {
            matrix_id: "missing".into(),
            change: MatrixPlacementChange::RowOffset {
                row: 0,
                value: Vec2::default(),
            },
        },
    ] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "bad".into(),
            command: EditCommand {
                base_revision: doc.revision,
                transaction_id: "bad".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        });
        assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
        assert_eq!(snapshot(&mut engine), before);
    }

    // Setting the current value is a valid no-op.
    let (reply, after) = edit(&mut engine, &doc, stagger(0, 0.0));
    assert!(reply.changed_ids.is_empty());
    assert_eq!(after.matrices, doc.matrices);
}

#[test]
fn preview_and_undo() {
    let (mut engine, _, doc) = fixture(1, 3);
    let operation = stagger(0, 2.0);
    let preview = match engine.handle(CoreRequest::Edit {
        id: "preview".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "preview".into(),
            phase: EditPhase::Preview,
            target_ids: vec![],
            operation: operation.clone(),
        },
    }) {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    };
    assert_eq!(snapshot(&mut engine).matrices, doc.matrices);
    let (committed, after) = edit(&mut engine, &doc, operation);
    assert_eq!(after.matrices[0].column_layout[0].stagger, 2.0);
    assert_eq!(preview.matrix_scenes, committed.matrix_scenes);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, doc.matrices);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.matrices, after.matrices);
}

#[test]
fn changed_ids_are_precise() {
    let mut doc = ProjectDoc::empty("placement", "Placement");
    doc.definitions = definitions();
    doc.parts.push(Part {
        id: "spare".into(),
        definition_id: "switch".into(),
        reference: "SPARE".into(),
        pose: Pose2 {
            at: Vec2 { x: 100.0, y: 100.0 },
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
    let mut matrix = matrix("main", 1, 3);
    matrix.cells = vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column: 1,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies: vec![MatrixAssembly {
            id: "diode".into(),
            definition_id: "diode".into(),
            offset: Vec2 { x: 2.0, y: 5.0 },
            rotation: None,
            side: None,
        }],
        assemblies_local: None,
    }];
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let (reply, _) = edit(&mut engine, &doc, stagger(1, 2.0));
    for id in [
        "main".to_string(),
        member("main", 0, 1),
        member("main", 0, 2),
        companion("main", 0, 1, "diode"),
    ] {
        assert!(
            reply.changed_ids.contains(&id),
            "missing {id}: {:?}",
            reply.changed_ids
        );
    }
    assert!(!reply.changed_ids.contains(&member("main", 0, 0)));
    assert!(!reply.changed_ids.iter().any(|id| id == "spare"));
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

#[test]
fn convert_preserves_poses_on_an_unsplayed_column() {
    let matrix = convert_case("middle", 1, |matrix| {
        {
            let values = vec![Vec2::default(), Vec2 { x: 1.5, y: -9.525 }, Vec2::default()];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.offset = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.offset = value;
            }
        }
        {
            let values = vec![0.0, 2.0, 1.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.stagger = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.stagger = value;
            }
        }
    });
    assert_eq!(
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0, 2.0 - 9.525, 1.0 + 9.525]
    );
    assert_eq!(matrix.column_layout[1].offset.x, 1.5);
}

#[test]
fn convert_preserves_poses_on_the_last_column_and_a_custom_origin() {
    let matrix = convert_case("last", 2, |matrix| {
        let values = vec![Vec2::default(), Vec2::default(), Vec2 { x: 0.0, y: 4.0 }];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    });
    assert_eq!(
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>(),
        vec![0.0, 0.0, 4.0]
    );
    convert_case("custom origin with preceding splay", 1, |matrix| {
        matrix.rotation = Some(20.0);
        matrix.mirror = Some(Mirror::X);
        {
            let values = vec![10.0, 0.0, 5.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        {
            let values = (vec![None, Some(Vec2 { x: 25.0, y: -12.0 }), None])
                .into_iter()
                .map(|value| value.unwrap_or_default())
                .collect::<Vec<_>>();
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
        {
            let values = vec![Vec2::default(), Vec2 { x: 0.5, y: 3.0 }, Vec2::default()];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.offset = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.offset = value;
            }
        }
    });
    // Converting column 1: a splayed default-origin neighbour (column 2) must not move.
    convert_case("splay on a later column only", 1, |matrix| {
        {
            let values = vec![0.0, 0.0, 12.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        {
            let values = vec![Vec2::default(), Vec2 { x: 0.0, y: -2.0 }, Vec2::default()];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.offset = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.offset = value;
            }
        }
    });
}

#[test]
fn convert_errors_on_a_splayed_default_origin_and_changes_nothing() {
    let (mut engine, _, doc) = fixture(1, 3);
    let mut matrix = doc.matrices[0].clone();
    {
        let values = vec![0.0, 15.0, 0.0];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.splay = 0.0;
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.splay = value;
        }
    }
    {
        let values = vec![Vec2::default(), Vec2 { x: 0.0, y: 3.0 }, Vec2::default()];
        matrix.column_layout.resize_with(
            values.len().max(matrix.column_layout.len()),
            Default::default,
        );
        for record in &mut matrix.column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in matrix.column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let reason = "Converting requires zero Splay, Column rotation and carried Bend";
    assert_eq!(
        convert_offset_available(&before.matrices[0], 1),
        Err(reason.to_string())
    );
    let reply = engine.handle(CoreRequest::Edit {
        id: "convert".into(),
        command: EditCommand {
            base_revision: before.revision,
            transaction_id: "convert".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: convert("main", 1),
        },
    });
    match reply {
        CoreReply::Error { message, .. } => assert_eq!(message, reason),
        other => panic!("expected an error: {other:?}"),
    }
    assert_eq!(snapshot(&mut engine), before);
}

#[test]
fn convert_applies_to_both_linked_halves() {
    let (mut engine, doc) = linked_pair();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrixPlacement {
            matrix_id: "left-matrix".into(),
            change: MatrixPlacementChange::ColumnOffset {
                column: 1,
                value: Vec2 { x: 0.0, y: -9.525 },
            },
        },
    );
    let (_, after) = edit(&mut engine, &doc, convert("left-matrix", 1));
    assert_same_poses(&doc, &after, "linked");
    for id in ["left-matrix", "right-matrix"] {
        let matrix = after.matrices.iter().find(|item| item.id == id).unwrap();
        assert_eq!(matrix.column_layout[1].offset.y, 0.0, "{id}");
        assert_eq!(matrix.column_layout[1].stagger, -9.525, "{id}");
        assert_eq!(matrix.column_layout[2].stagger, 9.525, "{id}");
    }
    assert_reflected_keys(&after);
}

/// Save and reopen: the placement parameters survive a JSON round trip through a fresh
/// engine with nothing normalized, including an old-toolbar `column_offsets[c].y`.
#[test]
fn placement_survives_save_and_reopen_unnormalized() {
    let mut doc = ProjectDoc::empty("roundtrip", "Roundtrip");
    doc.definitions = definitions();
    let (mut engine, _, doc) = open(doc);
    let mut configured = companion_matrix();
    configured.rows = 2;
    configured.columns = 3;
    {
        let values = vec![0.0, 4.5, -2.0];
        configured.column_layout.resize_with(
            values.len().max(configured.column_layout.len()),
            Default::default,
        );
        for record in &mut configured.column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in configured.column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    {
        let values = vec![
            Vec2::default(),
            Vec2 { x: 0.0, y: -9.525 },
            Vec2 { x: 1.5, y: 0.0 },
        ];
        configured.column_layout.resize_with(
            values.len().max(configured.column_layout.len()),
            Default::default,
        );
        for record in &mut configured.column_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in configured.column_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    {
        let values = vec![Vec2::default(), Vec2 { x: 2.0, y: 1.0 }];
        let count = values.len().max(configured.row_layout.len());
        configured.row_layout.resize_with(count, Default::default);
        for record in &mut configured.row_layout {
            record.offset = Vec2::default();
        }
        for (record, value) in configured.row_layout.iter_mut().zip(values) {
            record.offset = value;
        }
    }
    configured.cells[0].offset = Some(Vec2 { x: 0.75, y: -0.5 });
    configured.cells[0].rotation = Some(5.0);
    let (_, laid_out) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: configured,
            definitions: None,
        },
    );

    let saved = serde_json::to_string(&laid_out).unwrap();
    let reloaded: ProjectDoc = serde_json::from_str(&saved).unwrap();
    let (mut reopened_engine, _, reopened) = open(reloaded);
    let again = snapshot(&mut reopened_engine);

    assert_eq!(laid_out.matrices, reopened.matrices, "matrix values");
    assert_eq!(laid_out.matrices, again.matrices, "after the open settles");
    assert_same_poses(&laid_out, &reopened, "reopened poses");
    assert_same_poses(&laid_out, &again, "settled poses");
    assert_eq!(
        reopened.matrices[0].column_layout[1].offset,
        Vec2 { x: 0.0, y: -9.525 },
        "the old-toolbar offset is kept, not converted"
    );
}
