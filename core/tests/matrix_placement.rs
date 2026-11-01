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
        anchor: None,
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

#[test]
fn curved_key_world_move_changes_only_its_independent_offset() {
    let (mut engine, _, doc) = fixture(3, 2);
    let mut matrix = doc.matrices[0].clone();
    matrix.column_layout = vec![ColumnLayout {
        rotation: 90.0,
        ..Default::default()
    }];
    matrix.row_layout = vec![RowLayout {
        bend: 90.0,
        ..Default::default()
    }];
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let id = member("main", 1, 0);
    let start = at(&before, &id);
    let (changed, after) = edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: id.clone(),
                at: Vec2 {
                    x: start.x + 4.0,
                    y: start.y - 3.0,
                },
            }],
        },
    );
    assert!(close(
        at(&after, &id),
        Vec2 {
            x: start.x + 4.0,
            y: start.y - 3.0
        }
    ));
    for part in &before.parts {
        if part.id != id {
            assert_eq!(after.parts.iter().find(|p| p.id == part.id).unwrap(), part);
        }
    }
    let cell = after.matrices[0]
        .cells
        .iter()
        .find(|c| c.row == 1 && c.column == 0)
        .unwrap();
    assert!(close(cell.offset.unwrap(), Vec2 { x: -4.0, y: 3.0 }));
    assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
    assert_eq!(
        after.matrices[0].column_layout,
        before.matrices[0].column_layout
    );
    assert_eq!(changed.changed_ids, vec!["main".to_owned(), id]);
}

#[test]
fn named_row_shift_carries_later_keys_without_changing_offsets() {
    let (mut engine, _, before) = fixture(3, 2);
    let reply = engine.request(&serde_json::json!({
        "kind": "edit", "id": "shift", "command": {
            "baseRevision": before.revision, "transactionId": "shift", "phase": "commit", "targetIds": [],
            "operation": { "kind": "set-matrix-placement", "matrixId": "main",
                "change": { "kind": "row-shift", "row": 1, "value": { "x": 4.0, "y": -3.0 } } }
        }
    }).to_string());
    let (changed, after) = scene(serde_json::from_str(&reply).unwrap());
    for row in 0..3 {
        for column in 0..2 {
            let id = member("main", row, column);
            assert!(close(
                delta(at(&before, &id), at(&after, &id)),
                if row == 0 {
                    Vec2::default()
                } else {
                    Vec2 { x: 4.0, y: -3.0 }
                }
            ));
        }
    }
    assert_eq!(
        after.matrices[0].row_layout[1].shift,
        Vec2 { x: 4.0, y: -3.0 }
    );
    assert!(after.matrices[0].cells.iter().all(|c| c.offset.is_none()));
    assert_eq!(changed.changed_ids.len(), 5);
}

#[test]
fn unchanged_splay_and_base_origin_do_not_write_or_create_history() {
    let (mut engine, _, before) = fixture(2, 2);
    for change in [
        MatrixSplayChange::Angle {
            angle: 0.0,
            affect: MatrixSplayAffect::Column,
        },
        MatrixSplayChange::Angle {
            angle: 0.0,
            affect: MatrixSplayAffect::Following,
        },
        MatrixSplayChange::Origin { world: None },
    ] {
        let (changed, after) = edit(
            &mut engine,
            &before,
            EditOperation::SetMatrixSplay {
                matrix_id: "main".into(),
                column: 1,
                change,
            },
        );
        assert_eq!(after, before);
        assert!(changed.changed_ids.is_empty());
    }
    for operation in [
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 0),
                at: at(&before, &member("main", 0, 0)),
            }],
        },
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnTranslation {
                column: 1,
                delta: Vec2::default(),
            },
        },
    ] {
        let (changed, after) = edit(&mut engine, &before, operation);
        assert_eq!(after, before);
        assert!(changed.changed_ids.is_empty());
    }
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert!(
        undone.matrices.is_empty(),
        "no-op edits must leave the setup as the only Undo entry"
    );
    let (_, _, mut authored) = fixture(1, 1);
    authored.parts[0].properties =
        Some(serde_json::from_value(serde_json::json!({"layoutOverride":1})).unwrap());
    let (mut engine, _, before) = open(authored);
    let id = member("main", 0, 0);
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: id.clone(),
                at: at(&before, &id),
            }],
        },
    );
    assert_eq!(after, before, "a no-op must retain the authored key record");
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

#[test]
fn named_spread_padding_and_bends_preserve_independent_values_and_identity() {
    let (mut engine, _, before) = fixture(3, 3);
    let (_, spread) = wire_placement(
        &mut engine,
        &before,
        serde_json::json!({"kind":"column-spread","column":1,"value":23.0}),
    );
    assert_eq!(at(&spread, &member("main", 0, 1)), Vec2 { x: 23.0, y: 0.0 });
    assert_eq!(at(&spread, &member("main", 0, 2)), Vec2 { x: 42.0, y: 0.0 });
    let (_, padded) = wire_placement(
        &mut engine,
        &spread,
        serde_json::json!({"kind":"row-padding","row":1,"value":7.0}),
    );
    assert_eq!(at(&padded, &member("main", 2, 0)), Vec2 { x: 0.0, y: 26.0 });
    let (_, bent) = wire_placement(
        &mut engine,
        &padded,
        serde_json::json!({"kind":"row-bend","row":1,"value":90.0}),
    );
    assert!(close(
        at(&bent, &member("main", 2, 0)),
        Vec2 { x: -19.0, y: 7.0 }
    ));
    let (_, shifted) = wire_placement(
        &mut engine,
        &bent,
        serde_json::json!({"kind":"key-shift","row":1,"column":0,"value":{"x":3.0,"y":2.0}}),
    );
    assert!(close(
        at(&shifted, &member("main", 2, 0)),
        Vec2 { x: -16.0, y: 9.0 }
    ));
    assert_eq!(
        at(&shifted, &member("main", 1, 1)),
        Vec2 { x: 23.0, y: 7.0 }
    );
    let (_, turned) = wire_placement(
        &mut engine,
        &shifted,
        serde_json::json!({"kind":"key-bend","row":1,"column":0,"value":90.0}),
    );
    assert!(close(
        at(&turned, &member("main", 2, 0)),
        Vec2 { x: 3.0, y: -10.0 }
    ));
    assert_eq!(turned.matrices[0].pitch, before.matrices[0].pitch);
    assert_eq!(turned.matrices[0].part_ids, before.matrices[0].part_ids);
    for part in &turned.parts {
        let original = before.parts.iter().find(|p| p.id == part.id).unwrap();
        assert_eq!(part.reference, original.reference);
        assert_eq!(part.definition_id, original.definition_id);
    }
    let (unchanged, same) = wire_placement(
        &mut engine,
        &turned,
        serde_json::json!({"kind":"key-bend","row":1,"column":0,"value":90.0}),
    );
    assert_eq!(same, turned);
    assert!(unchanged.changed_ids.is_empty());
    let (_, defaulted) = wire_placement(
        &mut engine,
        &same,
        serde_json::json!({"kind":"column-spread","column":1,"value":null}),
    );
    assert_eq!(defaulted.matrices[0].column_layout[1].spread, None);
    assert_eq!(
        at(&defaulted, &member("main", 0, 1)),
        Vec2 { x: 19.0, y: 0.0 }
    );
}

#[test]
fn hold_origin_solves_the_final_point_with_partial_follow_and_refuses_atomically() {
    let (mut engine, _, doc) = open(matrix_doc());
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: matrix("anchor", 1, 1),
            definitions: None,
        },
    );
    let mut target = matrix("main", 3, 3);
    target.origin = Vec2 { x: 9.0, y: 3.0 };
    target.column_layout = vec![ColumnLayout {
        rotation: 90.0,
        ..Default::default()
    }];
    target.row_layout = vec![RowLayout {
        bend: 35.0,
        ..Default::default()
    }];
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix: target,
            definitions: None,
        },
    );
    let mut doc = doc;
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: vec!["anchor".into(), "main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    let target = doc.matrices.iter_mut().find(|m| m.id == "main").unwrap();
    target.anchor = Some(MatrixAnchor {
        matrix_id: "anchor".into(),
        row: 0,
        column: 0,
        follow: Follow {
            x: false,
            y: true,
            rotation: true,
        },
        aggregate: None,
    });
    let (mut engine, before_scene, before) = open(doc);
    assert_eq!(
        piece(&before_scene, "main").columns[0].splay_origin,
        Vec2 { x: 0.0, y: 3.0 }
    );
    let (after_scene, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixSplay {
            matrix_id: "main".into(),
            column: 0,
            change: MatrixSplayChange::Origin {
                world: Some(Vec2 { x: 2.0, y: 5.0 }),
            },
        },
    );
    assert!(close(
        piece(&after_scene, "main").columns[0].splay_origin,
        Vec2 { x: 2.0, y: 5.0 }
    ));
    assert_same_poses(&before, &after, "representable partial-Follow hold");
    let changed = after.matrices.iter().find(|m| m.id == "main").unwrap();
    assert!(close(
        changed.column_layout[0].splay_origin,
        Vec2 { x: 2.0, y: -2.0 }
    ));
    assert!(changed.column_layout[0].spread.is_none());
    assert!(close(changed.origin, Vec2 { x: 9.0, y: 7.0 }));
    let refused = engine.handle(CoreRequest::Edit {
        id: "refuse".into(),
        command: EditCommand {
            base_revision: after.revision,
            transaction_id: "refuse".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetMatrixSplay {
                matrix_id: "main".into(),
                column: 0,
                change: MatrixSplayChange::Origin {
                    world: Some(Vec2 { x: 2.0, y: 6.0 }),
                },
            },
        },
    });
    assert!(matches!(refused, CoreReply::Error { message, .. } if message.contains("Follow")));
    assert_eq!(snapshot(&mut engine), after);
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.matrices, before.matrices);
    assert_eq!(undone.parts, before.parts);
}

#[test]
fn swing_origin_changes_only_the_origin_and_swing_reset_restores_poses() {
    let (mut engine, _, doc) = fixture(3, 3);
    let mut matrix = doc.matrices[0].clone();
    matrix.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            rotation: 90.0,
            ..Default::default()
        },
    ];
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let request = |revision, world: serde_json::Value| {
        serde_json::json!({
        "kind":"edit","id":"swing","command":{"baseRevision":revision,"transactionId":format!("swing-{revision}"),"phase":"commit","targetIds":[],
        "operation":{"kind":"set-matrix-splay","matrixId":"main","column":1,"change":{"kind":"origin-swing","world":world}}}
    }).to_string()
    };
    let reply = engine.request(&request(
        before.revision,
        serde_json::json!({"x":22.0,"y":2.0}),
    ));
    let (swing_scene, after) = scene(serde_json::from_str(&reply).unwrap());
    let mut expected = before.matrices[0].clone();
    expected.column_layout[1].splay_origin = Vec2 { x: 3.0, y: 2.0 };
    assert_eq!(after.matrices[0], expected);
    assert!(close(
        piece(&swing_scene, "main").columns[1].splay_origin,
        Vec2 { x: 22.0, y: 2.0 }
    ));
    for row in 0..3 {
        assert!(close(
            delta(
                at(&before, &member("main", row, 1)),
                at(&after, &member("main", row, 1))
            ),
            Vec2 { x: 5.0, y: -1.0 }
        ));
        assert_eq!(
            at(&before, &member("main", row, 2)),
            at(&after, &member("main", row, 2))
        );
    }
    let reply = engine.request(&request(after.revision, serde_json::Value::Null));
    let (_, reset) = scene(serde_json::from_str(&reply).unwrap());
    assert_eq!(reset.matrices, before.matrices);
    assert_eq!(reset.parts, before.parts);
}

#[test]
fn relative_origin_field_holds_keys_by_writing_only_the_nearest_steps() {
    let (mut engine, _, doc) = fixture(3, 3);
    let mut matrix = doc.matrices[0].clone();
    matrix.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            splay: 90.0,
            ..Default::default()
        },
    ];
    let (_, before) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    let reply=engine.request(&serde_json::json!({"kind":"edit","id":"relative","command":{
        "baseRevision":before.revision,"transactionId":"relative","phase":"commit","targetIds":[],
        "operation":{"kind":"set-matrix-splay","matrixId":"main","column":1,"change":{"kind":"origin-relative","value":{"x":2.0,"y":3.0}}}
    }}).to_string());
    let (_, after) = scene(serde_json::from_str(&reply).unwrap());
    assert_same_poses(&before, &after, "relative field holds");
    assert_eq!(
        after.matrices[0].column_layout[1].splay_origin,
        Vec2 { x: 2.0, y: 3.0 }
    );
    assert_eq!(after.matrices[0].column_layout[1].spread, Some(14.0));
    assert!(close(
        Vec2 {
            x: after.matrices[0].column_layout[1].stagger,
            y: 0.0
        },
        Vec2 { x: -1.0, y: 0.0 }
    ));
    assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
    assert_eq!(after.matrices[0].cells, before.matrices[0].cells);
}

#[test]
fn rigid_column_align_and_origin_hold_use_steps_under_bend_splay_and_mirrors() {
    for mirror in [None, Some(Mirror::X), Some(Mirror::Y)] {
        for column in 0..3 {
            let (mut engine, _, doc) = fixture(3, 3);
            let mut matrix = doc.matrices[0].clone();
            matrix.mirror = mirror;
            matrix.rotation = Some(31.0);
            matrix.origin = Vec2 { x: 7.0, y: -11.0 };
            matrix.row_layout = vec![RowLayout {
                bend: 23.0,
                shift: Vec2 { x: 2.0, y: 3.0 },
                ..Default::default()
            }];
            matrix.column_layout = (0..3)
                .map(|c| ColumnLayout {
                    splay: 17.0,
                    rotation: 13.0,
                    stagger: f64::from(c),
                    splay_origin: Vec2 { x: 3.0, y: -5.0 },
                    ..Default::default()
                })
                .collect();
            let (_, before) = edit(
                &mut engine,
                &doc,
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
            );
            let (_, after) = edit(
                &mut engine,
                &before,
                EditOperation::SetMatrixPlacement {
                    matrix_id: "main".into(),
                    change: MatrixPlacementChange::ColumnTranslation {
                        column,
                        delta: Vec2 { x: 4.0, y: -3.0 },
                    },
                },
            );
            for row in 0..3 {
                for col in 0..3 {
                    let id = member("main", row, col);
                    assert!(
                        close(
                            delta(at(&before, &id), at(&after, &id)),
                            if col == column {
                                Vec2 { x: 4.0, y: -3.0 }
                            } else {
                                Vec2::default()
                            }
                        ),
                        "{mirror:?} c{column}: {id}"
                    );
                }
            }
            assert_eq!(after.matrices[0].row_layout, before.matrices[0].row_layout);
            assert_eq!(after.matrices[0].cells, before.matrices[0].cells);
            for c in 0..3 {
                assert_eq!(
                    after.matrices[0].column_layout[c].offset,
                    before.matrices[0].column_layout[c].offset
                );
            }
            let (held_scene, held) = edit(
                &mut engine,
                &after,
                EditOperation::SetMatrixSplay {
                    matrix_id: "main".into(),
                    column,
                    change: MatrixSplayChange::Origin {
                        world: Some(Vec2 { x: 43.0, y: -27.0 }),
                    },
                },
            );
            assert!(close(
                piece(&held_scene, "main").columns[column as usize].splay_origin,
                Vec2 { x: 43.0, y: -27.0 }
            ));
            assert_same_poses(&after, &held, "mirrored curved hold");
            for c in 0..3 {
                if c != column as usize && c != column as usize + 1 {
                    assert_eq!(
                        held.matrices[0].column_layout[c],
                        after.matrices[0].column_layout[c]
                    );
                }
            }
            let (_, reset) = edit(
                &mut engine,
                &held,
                EditOperation::SetMatrixSplay {
                    matrix_id: "main".into(),
                    column,
                    change: MatrixSplayChange::Origin { world: None },
                },
            );
            assert_same_poses(&held, &reset, "reset holds");
        }
    }
}

#[test]
fn exact_align_refuses_to_move_a_local_key_without_changing_state() {
    let (_, mut doc) = linked_pair();
    doc.layouts
        .iter_mut()
        .find_map(|l| l.mirror_link.as_mut())
        .unwrap()
        .exceptions
        .push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id: member("right-matrix", 0, 1),
            state: MirroredExceptionState::Local,
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    let (mut engine, _, before) = open(doc);
    let reply = engine.handle(CoreRequest::Edit {
        id: "align".into(),
        command: EditCommand {
            base_revision: before.revision,
            transaction_id: "align".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetMatrixPlacement {
                matrix_id: "right-matrix".into(),
                change: MatrixPlacementChange::ColumnTranslation {
                    column: 1,
                    delta: Vec2 { x: 4.0, y: -3.0 },
                },
            },
        },
    });
    assert!(
        matches!(&reply,CoreReply::Error {message,..} if message.contains("Local")),
        "{reply:?}"
    );
    assert_eq!(snapshot(&mut engine), before);
    assert!(
        matches!(engine.handle(CoreRequest::Undo {id:"undo".into()}),CoreReply::Error {message,..} if message.contains("History is empty"))
    );
}

#[test]
fn named_placement_boundary_refuses_nonfinite_and_out_of_bounds_values_atomically() {
    let (mut engine, _, before) = fixture(3, 3);
    for change in [
        MatrixPlacementChange::ColumnSpread {
            column: 0,
            value: Some(3.0),
        },
        MatrixPlacementChange::ColumnSpread {
            column: 3,
            value: Some(3.0),
        },
        MatrixPlacementChange::ColumnSpread {
            column: 1,
            value: Some(f64::NAN),
        },
        MatrixPlacementChange::RowPadding {
            row: 0,
            value: Some(3.0),
        },
        MatrixPlacementChange::RowPadding {
            row: 1,
            value: Some(f64::INFINITY),
        },
        MatrixPlacementChange::RowShift {
            row: 3,
            value: Vec2::default(),
        },
        MatrixPlacementChange::RowShift {
            row: 1,
            value: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
        },
        MatrixPlacementChange::RowBend {
            row: 1,
            value: f64::NEG_INFINITY,
        },
        MatrixPlacementChange::KeyShift {
            row: 0,
            column: 3,
            value: Vec2::default(),
        },
        MatrixPlacementChange::KeyShift {
            row: 0,
            column: 0,
            value: Vec2 {
                x: 0.0,
                y: f64::INFINITY,
            },
        },
        MatrixPlacementChange::KeyBend {
            row: 3,
            column: 0,
            value: 0.0,
        },
        MatrixPlacementChange::KeyBend {
            row: 0,
            column: 0,
            value: f64::NAN,
        },
    ] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "invalid".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "invalid".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::SetMatrixPlacement {
                    matrix_id: "main".into(),
                    change,
                },
            },
        });
        assert!(matches!(reply, CoreReply::Error { .. }));
        assert_eq!(snapshot(&mut engine), before);
    }
    for change in [
        MatrixSplayChange::OriginRelative {
            value: Vec2 {
                x: f64::NAN,
                y: 0.0,
            },
        },
        MatrixSplayChange::OriginSwing {
            world: Some(Vec2 {
                x: 0.0,
                y: f64::INFINITY,
            }),
        },
        MatrixSplayChange::Origin {
            world: Some(Vec2 {
                x: f64::NEG_INFINITY,
                y: 0.0,
            }),
        },
    ] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "invalid-origin".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "invalid-origin".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::SetMatrixSplay {
                    matrix_id: "main".into(),
                    column: 1,
                    change,
                },
            },
        });
        assert!(matches!(reply, CoreReply::Error { .. }));
        assert_eq!(snapshot(&mut engine), before);
    }
}

#[test]
fn carried_bend_carries_authored_companions_and_keeps_constraint_precedence() {
    let (mut engine, before) = companion_fixture();
    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::KeyBend {
                row: 0,
                column: 0,
                value: 90.0,
            },
        },
    );
    assert!(close(
        at(&after, &companion("main", 0, 0, "led")),
        Vec2 { x: -8.0, y: -10.0 }
    ));
    assert!(close(
        at(&after, &companion("main", 0, 0, "diode")),
        Vec2 { x: -5.0, y: 2.0 }
    ));
    assert_eq!(
        at(&after, &companion("main", 0, 0, "sensor")),
        at(&before, &companion("main", 0, 0, "sensor"))
    );
    assert_eq!(after.constraints, before.constraints);
    assert_eq!(
        after.matrices[0].cells[0].assemblies,
        before.matrices[0].cells[0].assemblies
    );
}

#[test]
fn exact_column_align_refuses_external_constraint_without_retuning_it() {
    let (_, _, mut document) = fixture(2, 2);
    document.constraints.push(Constraint::Offset {
        id: "held".into(),
        source_part_id: member("main", 0, 0),
        target_part_id: member("main", 1, 1),
        offset: Vec2 { x: 19.0, y: 19.0 },
        rotation: 0.0,
    });
    let (mut engine, _, before) = open(document);
    for phase in [EditPhase::Preview, EditPhase::Commit] {
        let reply = engine.handle(CoreRequest::Edit {
            id: "align".into(),
            command: EditCommand {
                base_revision: before.revision,
                transaction_id: "align".into(),
                phase,
                target_ids: vec![],
                operation: EditOperation::SetMatrixPlacement {
                    matrix_id: "main".into(),
                    change: MatrixPlacementChange::ColumnTranslation {
                        column: 1,
                        delta: Vec2 { x: 4.0, y: -3.0 },
                    },
                },
            },
        });
        assert!(matches!(reply,CoreReply::Error {message,..} if message.contains("constraint")));
        assert_eq!(snapshot(&mut engine), before);
    }
    assert!(
        matches!(engine.handle(CoreRequest::Undo {id:"refusal-history".into()}),CoreReply::Error {message,..} if message.contains("History is empty"))
    );
}

#[test]
fn swing_origin_at_column_zero_uses_mirrored_incoming_axes() {
    for (mirror, expected) in [
        (Mirror::X, Vec2 { x: -3.0, y: 2.0 }),
        (Mirror::Y, Vec2 { x: 3.0, y: -2.0 }),
    ] {
        let (mut engine, _, doc) = fixture(2, 2);
        let mut matrix = doc.matrices[0].clone();
        matrix.mirror = Some(mirror);
        matrix.origin = Vec2 { x: 7.0, y: -11.0 };
        matrix.column_layout = vec![ColumnLayout {
            rotation: 90.0,
            ..Default::default()
        }];
        let (_, before) = edit(
            &mut engine,
            &doc,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        );
        let (_, after) = edit(
            &mut engine,
            &before,
            EditOperation::SetMatrixSplay {
                matrix_id: "main".into(),
                column: 0,
                change: MatrixSplayChange::OriginSwing {
                    world: Some(Vec2 { x: 10.0, y: -9.0 }),
                },
            },
        );
        assert_eq!(after.matrices[0].column_layout[0].splay_origin, expected);
        assert!(close(
            at(&after, &member("main", 0, 0)),
            Vec2 { x: 8.0, y: -6.0 }
        ));
        assert_eq!(after.matrices[0].origin, before.matrices[0].origin);
        assert_eq!(after.matrices[0].column_layout.len(), 1);
    }
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

#[test]
fn matrix_translation_preserves_internal_constraint_without_retuning() {
    internal_constraint_translation(None);
}

#[test]
fn column_translation_preserves_internal_constraint_without_retuning() {
    internal_constraint_translation(Some(1));
}
