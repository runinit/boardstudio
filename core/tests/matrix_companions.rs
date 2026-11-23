//! Core publishes each generated companion's placement mode so presentation reads
//! the same precedence the solver uses: explicit constraint, authored pose, then
//! follow the key.
use crate::support::fixture::{self, part_at as at, part_pose as pose};
use crate::support::session::{commit_edit, open_engine};
use boardstudio_core::{CoreEngine, model::*};

fn cell(assemblies: Vec<MatrixAssembly>) -> MatrixCell {
    MatrixCell {
        assemblies,
        ..fixture::cell(0, 0)
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
    fixture::part("anchor", "switch", "ANCHOR")
}

fn companion_fixture() -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut doc = ProjectDoc::empty("companions", "Companions");
    doc.definitions = fixture::companion_definitions();
    doc.parts.push(companion_part());
    let (mut engine, _, doc) = open_engine(doc);
    let mut matrix = fixture::matrix("main", 1, 1);
    matrix.cells = vec![cell(vec![
        assembly("diode", "diode", 2.0, 5.0),
        assembly("led", "led", -3.0, 6.0),
        assembly("sensor", "sensor", 1.0, -4.0),
    ])];
    let (_, mut doc) = commit_edit(&mut engine, &doc, fixture::set_matrix(matrix));
    doc.constraints.push(Constraint::Offset {
        id: "sensor-anchor".into(),
        source_part_id: "anchor".into(),
        target_part_id: companion("main", 0, 0, "sensor"),
        offset: Vec2 { x: 1.0, y: -4.0 },
        rotation: 0.0,
    });
    let (mut engine, _, doc) = open_engine(doc);
    // Author the LED away from its generated offset.
    let (scene, doc) = commit_edit(
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

fn close(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

fn entry<'a>(scene: &'a SceneDelta, part_id: &str) -> &'a MatrixCompanionPlacement {
    fixture::matrix_scene(scene, "main")
        .companions
        .iter()
        .find(|item| item.part_id == part_id)
        .unwrap_or_else(|| panic!("missing companion {part_id}"))
}

#[test]
fn scene_lists_each_companion_mode() {
    let (_, scene, _) = companion_fixture();
    let companions = &fixture::matrix_scene(&scene, "main").companions;
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
    let (scene, after) = commit_edit(
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
    for entry in &fixture::matrix_scene(&scene, "main").companions {
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

    let (scene, after) = commit_edit(&mut engine, &before, fixture::set_matrix(changed));

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

    let (_, after) = commit_edit(&mut engine, &before, fixture::set_matrix(changed));

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

    let (_, after) = commit_edit(&mut engine, &before, fixture::set_matrix(changed));

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
    let (mut engine, _, before) = open_engine(before);
    let authored = companion("main", 0, 0, "led");
    let mut changed = before.matrices[0].clone();
    changed.origin = Vec2 { x: 4.0, y: 6.0 };

    let (_, after) = commit_edit(&mut engine, &before, fixture::set_matrix(changed));

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

        let (_, after) = commit_edit(&mut engine, &before, fixture::set_matrix(changed));

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
        let (scene, _) = commit_edit(&mut engine, &doc, fixture::set_matrix(matrix));
        assert!(
            fixture::matrix_scene(&scene, "main").companions.is_empty(),
            "{} cell must list no companions",
            if deleted { "deleted" } else { "disabled" }
        );
    }

    let (mut engine, _, doc) = companion_fixture();
    let (scene, after) = commit_edit(
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
