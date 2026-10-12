//! Core publishes each generated companion's placement mode so presentation reads
//! the same precedence the solver uses: explicit constraint, authored pose, then
//! follow the key.
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
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: vec![],
    }
}

fn cell(assemblies: Vec<MatrixAssembly>) -> MatrixCell {
    MatrixCell {
        deleted: false,
        row: 0,
        column: 0,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: None,
        assemblies,
        assemblies_local: None,
    }
}

fn assembly(id: &str, definition: &str, x: f64, y: f64) -> MatrixAssembly {
    MatrixAssembly {
        id: id.into(),
        definition_id: definition.into(),
        offset: Vec2 { x, y },
        rotation: None,
        side: None,
    }
}

fn companion_part() -> Part {
    Part {
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
    }
}

fn companion_fixture() -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut doc = ProjectDoc::empty("companions", "Companions");
    doc.definitions = definitions();
    doc.parts.push(companion_part());
    let (mut engine, _, doc) = open(doc);
    let mut matrix = matrix("main", 1, 1);
    matrix.cells = vec![cell(vec![
        assembly("diode", "diode", 2.0, 5.0),
        assembly("led", "led", -3.0, 6.0),
        assembly("sensor", "sensor", 1.0, -4.0),
    ])];
    let (_, mut doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetMatrix {
            matrix,
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
    // Author the LED away from its generated offset.
    let (scene, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: companion("main", 0, 0, "led"),
                at: Vec2 { x: -10.0, y: 8.0 },
            }],
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

fn pose(doc: &ProjectDoc, id: &str) -> Pose2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
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

fn entry<'a>(scene: &'a SceneDelta, part_id: &str) -> &'a MatrixCompanionPlacement {
    piece(scene, "main")
        .companions
        .iter()
        .find(|item| item.part_id == part_id)
        .unwrap_or_else(|| panic!("missing companion {part_id}"))
}

#[test]
fn scene_lists_each_companion_mode() {
    let (_, scene, _) = companion_fixture();
    let companions = &piece(&scene, "main").companions;
    assert_eq!(companions.len(), 3);
    for entry in companions {
        assert_eq!(entry.key_id, member("main", 0, 0));
    }
    assert_eq!(
        entry(&scene, &companion("main", 0, 0, "diode")).mode,
        MatrixCompanionMode::Follows
    );
    assert_eq!(
        entry(&scene, &companion("main", 0, 0, "led")).mode,
        MatrixCompanionMode::Authored
    );
    assert_eq!(
        entry(&scene, &companion("main", 0, 0, "sensor")).mode,
        MatrixCompanionMode::Constrained {
            constraint_id: "sensor-anchor".into(),
            source_part_id: "anchor".into(),
        }
    );
}

#[test]
fn scene_modes_match_the_solver() {
    let (mut engine, _, before) = companion_fixture();
    let key_before = pose(&before, &member("main", 0, 0));
    let led_before = at(&before, &companion("main", 0, 0, "led"));
    let offsets: Vec<(String, Vec2)> = before.matrices[0].cells[0]
        .assemblies
        .iter()
        .map(|assembly| (assembly.id.clone(), assembly.offset))
        .collect();
    let key_delta = Vec2 { x: 5.0, y: 7.0 };
    let (scene, after) = edit(
        &mut engine,
        &before,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 0),
                at: Vec2 {
                    x: key_before.at.x + key_delta.x,
                    y: key_before.at.y + key_delta.y,
                },
            }],
        },
    );
    let key_after = pose(&after, &member("main", 0, 0));
    let (sin, cos) = key_after.rotation.to_radians().sin_cos();
    for entry in &piece(&scene, "main").companions {
        let name = entry.part_id.rsplit('/').next().unwrap();
        let offset = offsets
            .iter()
            .find(|(id, _)| id == name)
            .map(|(_, offset)| *offset)
            .unwrap();
        let predicted = match &entry.mode {
            MatrixCompanionMode::Follows => Vec2 {
                x: key_after.at.x + offset.x * cos - offset.y * sin,
                y: key_after.at.y + offset.x * sin + offset.y * cos,
            },
            MatrixCompanionMode::Authored => Vec2 {
                x: led_before.x + key_delta.x,
                y: led_before.y + key_delta.y,
            },
            MatrixCompanionMode::Constrained { source_part_id, .. } => {
                let anchor = pose(&after, source_part_id);
                Vec2 {
                    x: anchor.at.x + 1.0,
                    y: anchor.at.y - 4.0,
                }
            }
        };
        assert!(
            close(at(&after, &entry.part_id), predicted),
            "{}",
            entry.part_id
        );
    }
}

#[test]
fn disabled_deleted_cells_and_removed_constraints() {
    for deleted in [false, true] {
        let (mut engine, _, doc) = companion_fixture();
        let mut matrix = doc.matrices[0].clone();
        matrix.cells[0].enabled = false;
        matrix.cells[0].deleted = deleted;
        let (scene, _) = edit(
            &mut engine,
            &doc,
            EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        );
        assert!(
            piece(&scene, "main").companions.is_empty(),
            "{} cell must list no companions",
            if deleted { "deleted" } else { "disabled" }
        );
    }

    let (mut engine, _, doc) = companion_fixture();
    let (scene, after) = edit(
        &mut engine,
        &doc,
        EditOperation::RemoveConstraint {
            id: "sensor-anchor".into(),
        },
    );
    assert_eq!(
        entry(&scene, &companion("main", 0, 0, "sensor")).mode,
        MatrixCompanionMode::Follows
    );
    assert!(after.constraints.is_empty());
}

#[test]
fn linked_halves_list_their_own_companions() {
    let mut doc = ProjectDoc::empty("pair", "Pair");
    doc.definitions = definitions();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let (mut engine, _, doc) = open(doc);
    let mut matrix = matrix("left-matrix", 1, 1);
    matrix.board_id = Some("board".into());
    matrix.cells = vec![cell(vec![assembly("diode", "diode", 2.0, 5.0)])];
    let (scene, after) = edit(
        &mut engine,
        &doc,
        EditOperation::CreateMirroredPair {
            left: Layout {
                id: "left-layout".into(),
                name: "Left".into(),
                board_id: "board".into(),
                matrix_id: "left-matrix".into(),
                part_ids: vec![],
                mirror_link: None,
            },
            right: Layout {
                id: "right-layout".into(),
                name: "Right".into(),
                board_id: "board".into(),
                matrix_id: "right-matrix".into(),
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    source_id: "left-layout".into(),
                    axis_x: 10.0,
                }),
            },
            matrix: Box::new(matrix),
            definitions: None,
        },
    );
    for (matrix_id, name) in [
        ("left-matrix", "left-matrix"),
        ("right-matrix", "right-matrix"),
    ] {
        let companions = &piece(&scene, matrix_id).companions;
        assert_eq!(companions.len(), 1, "{matrix_id}");
        assert_eq!(
            companions[0].part_id,
            companion(name, 0, 0, "diode"),
            "{matrix_id}"
        );
        assert_eq!(companions[0].key_id, member(name, 0, 0), "{matrix_id}");
        assert_eq!(companions[0].mode, MatrixCompanionMode::Follows);
    }
    assert!(
        after
            .parts
            .iter()
            .any(|part| part.id == companion("right-matrix", 0, 0, "diode"))
    );
}
