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
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn cell(assemblies: Vec<MatrixAssembly>) -> MatrixCell {
    MatrixCell {
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
fn set_matrix_origin_and_rotation_carry_an_authored_companion_with_its_key() {
    let (mut engine, _, before) = companion_fixture();
    let key = member("main", 0, 0);
    let authored = companion("main", 0, 0, "led");
    let mut changed = before.matrices[0].clone();
    changed.origin = Vec2 { x: 11.0, y: -7.0 };
    changed.rotation = Some(90.0);

    let (scene, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrix {
            matrix: changed,
            definitions: None,
        },
    );

    // The authored offset (-10, 8) rotates by 90 degrees with its key.
    assert_eq!(pose(&after, &key).at, Vec2 { x: 11.0, y: -7.0 });
    assert_eq!(pose(&after, &key).rotation, 90.0);
    assert_eq!(
        pose(&after, &authored),
        Pose2 {
            at: Vec2 { x: 3.0, y: -17.0 },
            rotation: 90.0,
        }
    );
    for id in [
        "main".to_string(),
        key,
        companion("main", 0, 0, "diode"),
        authored,
        companion("main", 0, 0, "sensor"),
    ] {
        assert!(
            scene.changed_ids.contains(&id),
            "SetMatrix stopped reporting {id}: {:?}",
            scene.changed_ids
        );
    }
}

#[test]
fn set_matrix_translation_carries_an_authored_companion_bit_exactly() {
    let (mut engine, _, before) = companion_fixture();
    let key = member("main", 0, 0);
    let authored = companion("main", 0, 0, "led");
    let mut changed = before.matrices[0].clone();
    changed.origin = Vec2 { x: 13.2, y: -4.1 };

    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrix {
            matrix: changed,
            definitions: None,
        },
    );

    let (was_key, now_key) = (pose(&before, &key), pose(&after, &key));
    let (was, now) = (pose(&before, &authored), pose(&after, &authored));
    assert_eq!(now.at.x, was.at.x + (now_key.at.x - was_key.at.x));
    assert_eq!(now.at.y, was.at.y + (now_key.at.y - was_key.at.y));
    assert_eq!(now.rotation, was.rotation);
}

#[test]
fn set_matrix_mirror_reflects_an_authored_companions_key_relative_pose() {
    let (mut engine, _, before) = companion_fixture();
    let key = member("main", 0, 0);
    let authored = companion("main", 0, 0, "led");
    let mut changed = before.matrices[0].clone();
    changed.mirror = Some(Mirror::X);

    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrix {
            matrix: changed,
            definitions: None,
        },
    );

    assert_eq!(pose(&after, &key).at, pose(&before, &key).at);
    assert_eq!(
        pose(&after, &authored),
        Pose2 {
            at: Vec2 { x: 10.0, y: 8.0 },
            rotation: 180.0,
        }
    );
}

#[test]
fn set_matrix_carries_a_locked_authored_companion_with_its_key() {
    let (_, _, mut before) = companion_fixture();
    before
        .parts
        .iter_mut()
        .find(|part| part.id == companion("main", 0, 0, "led"))
        .unwrap()
        .locked = Some(true);
    let (mut engine, _, before) = open(before);
    let authored = companion("main", 0, 0, "led");
    let mut changed = before.matrices[0].clone();
    changed.origin = Vec2 { x: 4.0, y: 6.0 };

    let (_, after) = edit(
        &mut engine,
        &before,
        EditOperation::SetMatrix {
            matrix: changed,
            definitions: None,
        },
    );

    assert_eq!(pose(&after, &authored).at, Vec2 { x: -6.0, y: 14.0 });
    assert_eq!(
        after
            .parts
            .iter()
            .find(|part| part.id == authored)
            .unwrap()
            .locked,
        Some(true)
    );
}

#[test]
fn disabled_or_deleted_cells_do_not_leave_companions_for_set_matrix_to_carry() {
    for (enabled, deleted) in [(false, false), (true, true)] {
        let (mut engine, _, before) = companion_fixture();
        let key = member("main", 0, 0);
        let authored = companion("main", 0, 0, "led");
        let mut changed = before.matrices[0].clone();
        changed.cells[0].enabled = enabled;
        changed.cells[0].deleted = deleted;
        changed.origin = Vec2 { x: 7.0, y: -2.0 };

        let (_, after) = edit(
            &mut engine,
            &before,
            EditOperation::SetMatrix {
                matrix: changed,
                definitions: None,
            },
        );

        assert!(!after.parts.iter().any(|part| part.id == key));
        assert!(!after.parts.iter().any(|part| part.id == authored));
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
                matrix_ids: vec!["left-matrix".into()],
                part_ids: vec![],
                mirror_link: None,
            },
            right: Layout {
                id: "right-layout".into(),
                name: "Right".into(),
                board_id: "board".into(),
                matrix_ids: vec!["right-matrix".into()],
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    correspondences: vec![],
                    exceptions: vec![],
                    source_id: "left-layout".into(),
                    axis_x: 10.0,
                    split_gap: None,
                    spacing: None,
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

    let source_diode = companion("left-matrix", 0, 0, "diode");
    let (_, authored) = edit(
        &mut engine,
        &after,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: source_diode.clone(),
                at: Vec2 { x: 8.0, y: 9.0 },
            }],
        },
    );
    let mut moved_matrix = authored
        .matrices
        .iter()
        .find(|matrix| matrix.id == "left-matrix")
        .unwrap()
        .clone();
    // Ticket 09: creation stored the half frame (source frame at x = 9), so
    // the board intent (4, 2) is authored as (-5, 2) in storage. Ticket 11:
    // creation stores max(requested 2 mm, 40 mm default, minimum), so the
    // stored gap is 40 mm while the origins still convert with the request.
    let stored_gap = authored
        .layouts
        .iter()
        .find(|layout| layout.id == "right-layout")
        .and_then(|layout| layout.mirror_link.as_ref())
        .and_then(|link| link.split_gap);
    assert_eq!(stored_gap, Some(40.0));
    moved_matrix.origin = Vec2 { x: -5.0, y: 2.0 };
    let (_, carried) = edit(
        &mut engine,
        &authored,
        EditOperation::SetMatrix {
            matrix: moved_matrix,
            definitions: None,
        },
    );
    // Ticket 10: the authored placement overlaps the halves (key envelopes
    // meet at clear -8 mm), so the committed edit re-spaces in the same Undo
    // entry. Ticket 11: the floor is now the stored 40 mm, which already
    // covers the 20 mm minimum, so no spacing is stored. The authored diode
    // was placed at (8, 9) against the creation key, which now evaluates 19
    // mm farther out, so the rigidly carried offset grows by the same 19 mm.
    let carried_gap = carried
        .layouts
        .iter()
        .find(|layout| layout.id == "right-layout")
        .and_then(|layout| layout.mirror_link.as_ref())
        .and_then(|link| link.spacing);
    assert_eq!(carried_gap, None);
    assert_eq!(pose(&carried, &source_diode).at, Vec2 { x: 12.0, y: 11.0 });
    assert_eq!(
        pose(&carried, &member("right-matrix", 0, 0)).at,
        Vec2 { x: 35.0, y: 2.0 }
    );
    assert!(
        carried
            .parts
            .iter()
            .any(|part| part.id == companion("right-matrix", 0, 0, "diode"))
    );
}
