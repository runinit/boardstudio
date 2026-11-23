use crate::support::fixture::{self, document as scene};
use crate::support::session::builders::{companion_cell, move_key, move_parts, offset_constraint};
use boardstudio_core::CoreEngine;
use boardstudio_core::EditOutcome;
use boardstudio_core::model::*;

fn edit(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> ProjectDoc {
    scene(engine.commit_edit(fixture::commit("edit", "move", doc.revision, operation)))
}

fn fixture() -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("key-move", "Key move");
    doc.definitions = serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]},
        {"id":"led","name":"LED","kind":"passive","courtyard":[],"pads":[]}
    ]))
    .unwrap();
    let doc = scene(engine.open(doc));
    let matrix = Matrix {
        cells: vec![companion_cell(0, 0)],
        ..fixture::matrix("main", 1, 1)
    };
    let doc = edit(&mut engine, &doc, fixture::set_matrix(matrix));
    (engine, doc)
}

fn at(doc: &ProjectDoc, suffix: &str) -> Vec2 {
    doc.parts
        .iter()
        .find(|part| part.id == format!("matrix/main/r0c0{suffix}"))
        .unwrap()
        .pose
        .at
}

fn assert_companions_follow(before: &ProjectDoc, after: &ProjectDoc) {
    let delta = Vec2 {
        x: at(after, "").x - at(before, "").x,
        y: at(after, "").y - at(before, "").y,
    };
    for suffix in ["/diode", "/led"] {
        assert_eq!(
            at(after, suffix),
            Vec2 {
                x: at(before, suffix).x + delta.x,
                y: at(before, suffix).y + delta.y
            },
            "{suffix} must follow its key"
        );
    }
}

#[test]
fn moving_matrix_key_moves_its_diode_and_led() {
    let (mut engine, before) = fixture();
    let after = edit(&mut engine, &before, move_key("matrix/main/r0c0", 7.0, 9.0));
    assert_companions_follow(&before, &after);
    assert_eq!(
        after.matrices[0].cells[0].assemblies,
        before.matrices[0].cells[0].assemblies
    );
    assert_eq!(
        after.matrices[0].cells[0].offset,
        Some(Vec2 { x: 7.0, y: 9.0 })
    );
    let undone = scene(engine.undo());
    assert_eq!(undone.parts, before.parts);
    let redone = scene(engine.redo());
    assert_eq!(redone.parts, after.parts);
}

#[test]
fn setting_matrix_key_offset_moves_its_diode_and_led() {
    let (mut engine, before) = fixture();
    let mut matrix = before.matrices[0].clone();
    matrix.cells[0].offset = Some(Vec2 { x: 7.0, y: 9.0 });
    let after = edit(&mut engine, &before, fixture::set_matrix(matrix));
    assert_companions_follow(&before, &after);
}

#[test]
fn key_move_preview_restores_document_and_moves_companions() {
    let (mut engine, before) = fixture();
    let reply = engine.apply_edit(fixture::edit(
        "preview",
        "preview",
        before.revision,
        EditPhase::Preview,
        move_key("matrix/main/r0c0", 7.0, 9.0),
    ));
    let Ok(EditOutcome::Preview(preview)) = reply else {
        panic!("expected preview")
    };
    for suffix in ["", "/diode", "/led"] {
        let transformed = preview
            .transforms
            .iter()
            .find(|transform| transform.id == format!("matrix/main/r0c0{suffix}"))
            .unwrap();
        assert_eq!(
            transformed.pose.at,
            Vec2 {
                x: at(&before, suffix).x + 7.0,
                y: at(&before, suffix).y + 9.0
            }
        );
    }
    let snapshot = scene(engine.snapshot());
    assert_eq!(snapshot, before);
}

#[test]
fn companion_explicit_move_is_preserved_when_key_is_also_moved() {
    let (mut engine, before) = fixture();
    let after = edit(
        &mut engine,
        &before,
        move_parts(&[
            ("matrix/main/r0c0/diode", 11.0, 12.0),
            ("matrix/main/r0c0", 7.0, 9.0),
        ]),
    );
    assert_eq!(at(&after, "/diode"), Vec2 { x: 11.0, y: 12.0 });
    assert_eq!(at(&after, "/led"), Vec2 { x: 4.0, y: 15.0 });
}

#[test]
fn independently_positioned_companion_keeps_its_offset_when_key_moves() {
    let (mut engine, before) = fixture();
    let authored = edit(
        &mut engine,
        &before,
        move_key("matrix/main/r0c0/diode", 11.0, 12.0),
    );
    let after = edit(
        &mut engine,
        &authored,
        move_key("matrix/main/r0c0", 7.0, 9.0),
    );
    assert_companions_follow(&authored, &after);
}

#[test]
fn key_move_absorbs_legacy_absolute_override_into_cell_offset() {
    let (_, mut legacy) = fixture();
    let key = legacy
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/main/r0c0")
        .unwrap();
    key.pose.at = Vec2 { x: 4.0, y: 3.0 };
    key.properties
        .get_or_insert_default()
        .insert("layoutOverride".into(), serde_json::json!(1));
    let mut engine = CoreEngine::new();
    let before = scene(engine.open(legacy));
    let after = edit(&mut engine, &before, move_key("matrix/main/r0c0", 7.0, 9.0));
    assert_eq!(at(&after, ""), Vec2 { x: 7.0, y: 9.0 });
    assert_eq!(
        after.matrices[0].cells[0].offset,
        Some(Vec2 { x: 7.0, y: 9.0 })
    );
}

#[test]
fn matrix_key_move_respects_rotated_splayed_and_mirrored_cell_coordinates() {
    for mirror in [Mirror::None, Mirror::X, Mirror::Y] {
        let (mut engine, before) = fixture();
        let mut matrix = before.matrices[0].clone();
        matrix.rotation = Some(30.0);
        matrix.mirror = Some(mirror);
        matrix
            .column_layout
            .resize_with(matrix.column_layout.len().max(1), Default::default);
        for record in &mut matrix.column_layout {
            record.splay = 0.0;
        }
        matrix.column_layout[0].splay = 20.0;
        let before = edit(&mut engine, &before, fixture::set_matrix(matrix));
        let initial = at(&before, "");
        let after = edit(
            &mut engine,
            &before,
            EditOperation::MoveParts {
                positions: vec![Position {
                    id: "matrix/main/r0c0".into(),
                    at: Vec2 {
                        x: initial.x + 7.0,
                        y: initial.y + 9.0,
                    },
                }],
            },
        );
        for suffix in ["", "/diode", "/led"] {
            let original = at(&before, suffix);
            let moved = at(&after, suffix);
            assert!((moved.x - original.x - 7.0).abs() < 1e-9);
            assert!((moved.y - original.y - 9.0).abs() < 1e-9);
        }
    }
}

#[test]
fn offset_constrained_key_moves_its_companions_and_keeps_constraint() {
    let (mut engine, before) = constraint_source_fixture();
    let after = edit(&mut engine, &before, move_key("matrix/main/r0c0", 7.0, 9.0));
    assert_companions_follow(&before, &after);
    let Constraint::Offset { offset, .. } = after.constraints[0] else {
        panic!("expected offset constraint")
    };
    assert_eq!(offset, Vec2 { x: 7.0, y: 9.0 });
    assert_eq!(after.matrices[0].cells[0].offset, Some(offset));
}

fn constraint_source_fixture() -> (CoreEngine, ProjectDoc) {
    let (_, mut doc) = fixture();
    let mut anchor = doc.parts[0].clone();
    anchor.id = "anchor".into();
    anchor.reference = "ANCHOR".into();
    anchor.pose.at = Vec2::default();
    doc.parts.push(anchor);
    doc.constraints.push(offset_constraint(
        "key-anchor",
        "anchor",
        "matrix/main/r0c0",
        0.0,
        0.0,
    ));
    let mut engine = CoreEngine::new();
    let doc = scene(engine.open(doc));
    (engine, doc)
}

#[test]
fn moving_constraint_source_and_key_together_moves_companions_by_final_key_delta() {
    let (mut engine, before) = constraint_source_fixture();
    let after = edit(
        &mut engine,
        &before,
        move_parts(&[("anchor", 3.0, 4.0), ("matrix/main/r0c0", 7.0, 9.0)]),
    );
    assert_companions_follow(&before, &after);
}

#[test]
fn moving_only_constraint_source_moves_matrix_key_companions() {
    let (mut engine, before) = constraint_source_fixture();
    let after = edit(&mut engine, &before, move_key("anchor", 3.0, 4.0));
    assert_companions_follow(&before, &after);
}

#[test]
fn moving_constraint_source_preview_restores_parametric_document() {
    let (mut engine, before) = constraint_source_fixture();
    let reply = engine.apply_edit(fixture::edit(
        "preview",
        "preview",
        before.revision,
        EditPhase::Preview,
        move_key("anchor", 3.0, 4.0),
    ));
    let Ok(EditOutcome::Preview(preview)) = reply else {
        panic!("expected preview")
    };
    for suffix in ["", "/diode", "/led"] {
        let transformed = preview
            .transforms
            .iter()
            .find(|transform| transform.id == format!("matrix/main/r0c0{suffix}"))
            .unwrap();
        assert_eq!(
            transformed.pose.at,
            Vec2 {
                x: at(&before, suffix).x + 3.0,
                y: at(&before, suffix).y + 4.0
            }
        );
    }
    assert_eq!(fixture::outline_snapshot(&mut engine), before);
    let after = edit(&mut engine, &before, move_key("anchor", 3.0, 4.0));
    assert_companions_follow(&before, &after);
    let undone = scene(engine.undo());
    assert_eq!(undone.parts, before.parts);
    assert_eq!(undone.matrices, before.matrices);
    let redone = scene(engine.redo());
    assert_eq!(redone.parts, after.parts);
    assert_eq!(redone.matrices, after.matrices);
}

#[test]
fn constrained_key_propagation_preserves_explicit_companion_destination() {
    let (mut engine, before) = constraint_source_fixture();
    let after = edit(
        &mut engine,
        &before,
        move_parts(&[("anchor", 3.0, 4.0), ("matrix/main/r0c0/diode", 11.0, 12.0)]),
    );
    assert_eq!(at(&after, "/diode"), Vec2 { x: 11.0, y: 12.0 });
    assert_eq!(at(&after, "/led"), Vec2 { x: 0.0, y: 10.0 });
}

#[test]
fn constrained_key_propagation_preserves_authored_companion_offset() {
    let (mut engine, before) = constraint_source_fixture();
    let authored = edit(
        &mut engine,
        &before,
        move_key("matrix/main/r0c0/diode", 11.0, 12.0),
    );
    let after = edit(&mut engine, &authored, move_key("anchor", 3.0, 4.0));
    assert_companions_follow(&authored, &after);
}

#[test]
fn constrained_key_propagation_respects_companions_own_constraint() {
    let (_, mut doc) = constraint_source_fixture();
    let mut fixed = doc
        .parts
        .iter()
        .find(|part| part.id == "anchor")
        .unwrap()
        .clone();
    fixed.id = "fixed".into();
    fixed.reference = "FIXED".into();
    doc.parts.push(fixed);
    doc.constraints.push(Constraint::Offset {
        id: "diode-fixed".into(),
        source_part_id: "fixed".into(),
        target_part_id: "matrix/main/r0c0/diode".into(),
        offset: Vec2 { x: 2.0, y: 5.0 },
        rotation: 0.0,
    });
    let mut engine = CoreEngine::new();
    let before = scene(engine.open(doc));
    let after = edit(&mut engine, &before, move_key("anchor", 3.0, 4.0));
    assert_eq!(at(&after, "/diode"), at(&before, "/diode"));
    assert_eq!(at(&after, "/led"), Vec2 { x: 0.0, y: 10.0 });
}

#[test]
fn opening_constrained_key_reconciles_companion_placement_and_rotation() {
    let (_, mut doc) = constraint_source_fixture();
    let anchor = doc
        .parts
        .iter_mut()
        .find(|part| part.id == "anchor")
        .unwrap();
    anchor.pose = Pose2 {
        at: Vec2 { x: 3.0, y: 4.0 },
        rotation: 30.0,
    };
    let mut engine = CoreEngine::new();
    let after = scene(engine.open(doc));
    let (sin, cos) = 30_f64.to_radians().sin_cos();
    for (suffix, offset) in [
        ("/diode", Vec2 { x: 2.0, y: 5.0 }),
        ("/led", Vec2 { x: -3.0, y: 6.0 }),
    ] {
        let actual = at(&after, suffix);
        assert!((actual.x - (3.0 + offset.x * cos - offset.y * sin)).abs() < 1e-9);
        assert!((actual.y - (4.0 + offset.x * sin + offset.y * cos)).abs() < 1e-9);
    }
    assert_eq!(after.matrices[0].cells[0].rotation, Some(30.0));
}

#[test]
fn direct_key_move_preserves_companions_authored_constraint() {
    let (_, mut doc) = constraint_source_fixture();
    doc.constraints.clear();
    doc.constraints.push(offset_constraint(
        "diode-anchor",
        "anchor",
        "matrix/main/r0c0/diode",
        11.0,
        12.0,
    ));
    let diode = doc
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/main/r0c0/diode")
        .unwrap();
    diode
        .properties
        .get_or_insert_default()
        .insert("layoutOverride".into(), serde_json::json!(1));
    let mut engine = CoreEngine::new();
    let before = scene(engine.open(doc));
    let after = edit(&mut engine, &before, move_key("matrix/main/r0c0", 3.0, 4.0));
    assert_eq!(at(&after, "/diode"), at(&before, "/diode"));
    assert_eq!(after.constraints, before.constraints);
}

fn assembly_chain_fixture() -> (CoreEngine, ProjectDoc) {
    let (mut engine, doc) = constraint_source_fixture();
    let mut matrix = doc.matrices[0].clone();
    matrix.columns = 3;
    for column in 1..3 {
        let mut cell = matrix.cells[0].clone();
        cell.column = column;
        matrix.cells.push(cell);
    }
    let mut doc = edit(&mut engine, &doc, fixture::set_matrix(matrix));
    for column in 1..3 {
        doc.constraints.push(offset_constraint(
            &format!("assembly-chain-{column}"),
            &format!("matrix/main/r0c{}/diode", column - 1),
            &format!("matrix/main/r0c{column}"),
            17.0,
            -5.0,
        ));
    }
    let doc = scene(engine.open(doc));
    (engine, doc)
}

fn assert_chain_shifted(before: &ProjectDoc, after: &ProjectDoc, delta: Vec2) {
    for part in &before.parts {
        if !part.id.starts_with("matrix/main/") {
            continue;
        }
        let moved = after
            .parts
            .iter()
            .find(|candidate| candidate.id == part.id)
            .unwrap();
        assert_eq!(
            moved.pose.at,
            Vec2 {
                x: part.pose.at.x + delta.x,
                y: part.pose.at.y + delta.y
            },
            "{} must follow the entire assembly dependency chain",
            part.id
        );
    }
}

#[test]
fn assembly_dependency_chain_settles_every_key_and_companion_in_commit() {
    let (mut engine, before) = assembly_chain_fixture();
    let after = edit(&mut engine, &before, move_key("anchor", 3.0, 4.0));
    assert_chain_shifted(&before, &after, Vec2 { x: 3.0, y: 4.0 });
}

#[test]
fn assembly_dependency_chain_settles_in_open() {
    let (_, before) = assembly_chain_fixture();
    let mut changed = before.clone();
    changed
        .parts
        .iter_mut()
        .find(|part| part.id == "anchor")
        .unwrap()
        .pose
        .at = Vec2 { x: 3.0, y: 4.0 };
    let mut engine = CoreEngine::new();
    let after = scene(engine.open(changed));
    assert_chain_shifted(&before, &after, Vec2 { x: 3.0, y: 4.0 });
}

#[test]
fn assembly_dependency_chain_preview_settles_and_restores_document() {
    let (mut engine, before) = assembly_chain_fixture();
    let reply = engine.apply_edit(fixture::edit(
        "preview",
        "preview",
        before.revision,
        EditPhase::Preview,
        move_key("anchor", 3.0, 4.0),
    ));
    let Ok(EditOutcome::Preview(preview)) = reply else {
        panic!("expected preview")
    };
    for part in &before.parts {
        if !part.id.starts_with("matrix/main/") {
            continue;
        }
        let moved = preview
            .transforms
            .iter()
            .find(|transform| transform.id == part.id)
            .unwrap();
        assert_eq!(
            moved.pose.at,
            Vec2 {
                x: part.pose.at.x + 3.0,
                y: part.pose.at.y + 4.0
            },
            "{} must settle in preview",
            part.id
        );
    }
    assert_eq!(fixture::outline_snapshot(&mut engine), before);
}

#[test]
fn assembly_parent_constraint_cycle_is_rejected_without_mutating_document() {
    let (mut engine, before) = fixture();
    let mut cyclic = before.clone();
    cyclic.constraints.push(offset_constraint(
        "assembly-cycle",
        "matrix/main/r0c0/diode",
        "matrix/main/r0c0",
        -2.0,
        -5.0,
    ));
    let reply = engine.open(cyclic);
    let Err(message) = reply else {
        panic!("expected dependency cycle rejection")
    };
    assert!(message.contains("cycle"), "{message}");
    assert_eq!(fixture::outline_snapshot(&mut engine), before);
}

#[test]
fn independent_assembly_constraint_breaks_implicit_parent_dependency_cycle() {
    let (_, mut doc) = constraint_source_fixture();
    doc.constraints.clear();
    doc.constraints.push(offset_constraint(
        "key-from-diode",
        "matrix/main/r0c0/diode",
        "matrix/main/r0c0",
        -2.0,
        -5.0,
    ));
    doc.constraints.push(offset_constraint(
        "diode-from-anchor",
        "anchor",
        "matrix/main/r0c0/diode",
        2.0,
        5.0,
    ));
    let mut engine = CoreEngine::new();
    let before = scene(engine.open(doc));
    let after = edit(&mut engine, &before, move_key("anchor", 3.0, 4.0));
    assert_companions_follow(&before, &after);
}

#[test]
fn assembly_dependency_chain_uses_authored_companion_offset() {
    let (mut engine, before) = assembly_chain_fixture();
    let authored = edit(
        &mut engine,
        &before,
        move_key("matrix/main/r0c0/diode", 11.0, 12.0),
    );
    let after = edit(&mut engine, &authored, move_key("anchor", 3.0, 4.0));
    assert_chain_shifted(&authored, &after, Vec2 { x: 3.0, y: 4.0 });
}

#[test]
fn assembly_dependency_chain_respects_explicit_companion_destination() {
    let (mut engine, before) = assembly_chain_fixture();
    let after = edit(
        &mut engine,
        &before,
        move_parts(&[("anchor", 3.0, 4.0), ("matrix/main/r0c0/diode", 11.0, 12.0)]),
    );
    assert_eq!(at(&after, "/diode"), Vec2 { x: 11.0, y: 12.0 });
    let last = after
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r0c2")
        .unwrap();
    assert_eq!(last.pose.at, Vec2 { x: 47.0, y: 7.0 });
}
