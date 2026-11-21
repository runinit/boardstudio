// Mirrored-half prepared-transaction integration tests, split into the
// submodules under `mirrored_prepare/` so no file exceeds ~1,600 LOC.
// Shared fixtures/helpers live in `support/`; each submodule inherits this
// crate's imports via `use super::*`.
#[path = "mirrored_reconcile_a/fixture_rows.rs"]
pub mod fixture_rows;
#[path = "support/matrix.rs"]
pub mod matrix_fixtures;
pub mod support;
use fixture_rows::{FixturePair, FixtureRows};

use crate::matrix_fixtures::pair_document;
use crate::support::{
    linked_endpoint, part_by_id as part, part_by_id_mut as part_mut, prepare_transaction,
};
use boardstudio_core_kernel::migrate::CURRENT_VERSION;
use boardstudio_model::model::{
    CaseBody, Constraint, ConstraintEndpoint, ConstraintRelation, CoreReply, CoreRequest,
    EditOperation, Position, ProjectDoc, Vec2,
};
use serde_json::json;

// The shared fixture also serves the integration target, which calls with_construction_features.
#[allow(dead_code)]
#[path = "../../../tests/support/synthetic_half.rs"]
mod synthetic_half_fixture;

#[path = "mirrored_prepare/composition_and_tombstones.rs"]
mod composition_and_tombstones;
#[path = "mirrored_prepare/open_and_preview.rs"]
mod open_and_preview;
#[path = "mirrored_prepare/open_repairs_and_mirror_moves.rs"]
mod open_repairs_and_mirror_moves;

#[test]
fn paired_embed_reserves_references_from_the_applied_operation_in_definition_order() {
    let before = pair_document();
    let mut dispatch = support::test_dispatch();
    dispatch.apply_unscripted = |draft, operation| {
        assert!(
            !matches!(operation, EditOperation::EmbedModuleCircuit { .. }),
            "the transaction must embed exactly once through its ledger-backed dispatch"
        );
        boardstudio_core_engine::apply_unscripted_for_test(draft, operation)
    };
    let members = ["Z", "A"].map(|id| {
        json!({
            "id":id, "definitionId":"d", "reference":"TP1", "side":"front",
            "pose":{"at":{"x":1,"y":2},"rotation":0}
        })
    });
    for board in ["left", "right"] {
        let operation = serde_json::from_value(json!({
            "kind":"embed-module-circuit","id":format!("{board}-probe"),"hostBoardId":board,
            "pose":{"at":{"x":10,"y":12},"rotation":0},"side":"front","joins":{},
            "definition":{"id":"probe","name":"Probe","family":"test","variant":"v",
                "source":{"repository":"https://example.invalid/probe","revision":"r","path":"probe.kicad_pcb","license":"MIT"},
                "board":{"contours":[]},"mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],
                "interfaces":[],"electrical":{"protocol":"pass-through","requiredSignals":[],"logicVoltage":3.3},
                "constituents":[],"circuit":{"definitions":[{"id":"d","name":"Probe","kind":"custom","courtyard":[],"pads":[]}],
                "parts":members,"nets":[],"ports":{},"adaptations":[]}}
        })).unwrap();
        let (draft, changed) =
            boardstudio_core_mirrored_half::mirrored_half::prepare_transaction_with_ledger(
                &before,
                &[],
                &operation,
                &Default::default(),
                dispatch,
            )
            .unwrap();
        for (id, expected) in [
            ("left-probe/Z", "TP1"),
            ("left-probe/A", "TP2"),
            ("right-probe/Z", "TP3"),
            ("right-probe/A", "TP4"),
        ] {
            let id = format!("embedded/{id}");
            assert_eq!(part(&draft, &id).reference, expected);
            assert!(changed.contains(&id));
        }
        let saved: ProjectDoc =
            serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
        let (opened, findings) = support::prepare_open(&saved, &[]).unwrap();
        assert_eq!(opened, saved);
        assert!(findings.is_empty());
    }
}

#[test]
fn sparse_axis_edits_preserve_survivors_local_overrides_and_history() {
    use boardstudio_model::model::{EditCommand, EditPhase};
    for row_axis in [true, false] {
        for (board, removed_axis) in [("left", 0), ("right", 0), ("left", 2), ("right", 2)] {
            let mut source = matrix_fixtures::source_half();
            let mut matrix = source.matrices[0].clone();
            // The far edge moves into a survivor's old slot. Reverse the saved
            // row order below so an in-place lookup cannot use move order as provenance.
            if row_axis {
                matrix.rows += 1;
            } else {
                matrix.columns += 1;
            }
            let mut edge: Vec<_> = matrix
                .cells
                .iter()
                .filter(|cell| (if row_axis { cell.row } else { cell.column }) == 1)
                .cloned()
                .collect();
            for cell in &mut edge {
                if row_axis {
                    cell.row += 1;
                } else {
                    cell.column += 1;
                }
            }
            matrix.cells.extend(edge);
            for cell in &mut matrix.cells {
                if (if row_axis { cell.row } else { cell.column }) == removed_axis {
                    cell.enabled = false;
                    cell.deleted = true;
                    cell.assemblies.clear();
                }
            }
            boardstudio_core_kernel::matrix::set_matrix(&mut source, &matrix).unwrap();
            let mut paired = matrix_fixtures::create_from(
                source,
                json!({"at":{"x":200,"y":0},"rotation":0}),
                "literal-mirror",
                None,
            );
            let halves = paired.first_halves_mut();
            halves
                .correspondences
                .sort_by(|a, b| b.source_id.cmp(&a.source_id));
            halves.exceptions.push(
                serde_json::from_value(
                    json!({"kind":"part","mirrorId":"right/U1","state":"local"}),
                )
                .unwrap(),
            );
            let middle = if row_axis { "r1c0" } else { "r0c1" };
            let local_id = format!("matrix/right-keys/{middle}/diode");
            halves.exceptions.push(
                serde_json::from_value(json!({"kind":"part","mirrorId":local_id,"state":"local"}))
                    .unwrap(),
            );
            part_mut(&mut paired, &local_id).locked = Some(true);
            part_mut(&mut paired, "right/U1").pose.at.x = 281.0;
            let mut engine = open_and_preview::open_pair_engine(paired);
            let before = engine.document_for_test().clone();
            let command = |index, revision, id: &str| {
                let matrix_id = format!("{board}-keys");
                CoreRequest::Edit {
                    id: id.into(),
                    command: EditCommand {
                        base_revision: revision,
                        transaction_id: id.into(),
                        phase: EditPhase::Commit,
                        target_ids: vec![],
                        operation: if row_axis {
                            EditOperation::RemoveMatrixRow {
                                matrix_id,
                                row: index,
                            }
                        } else {
                            EditOperation::RemoveMatrixColumn {
                                matrix_id,
                                column: index,
                            }
                        },
                    },
                }
            };
            let reply = engine.handle(command(removed_axis, before.revision, "compact"));
            let CoreReply::Scene { scene, .. } = reply else {
                panic!("{reply:?}")
            };
            let after = engine.document_for_test().clone();
            for side in ["left", "right"] {
                let matrix = after
                    .matrices
                    .iter()
                    .find(|m| m.id == format!("{side}-keys"))
                    .unwrap();
                assert_eq!((matrix.rows, matrix.columns), (2, 2));
                let (old, new) = if row_axis {
                    ("r2c0", "r0c0")
                } else {
                    ("r0c2", "r0c0")
                };
                for suffix in ["", "/diode"] {
                    let old = format!("matrix/{side}-keys/{old}{suffix}");
                    let new = format!("matrix/{side}-keys/{new}{suffix}");
                    assert!(after.parts.iter().all(|p| p.id != old));
                    assert!(matrix.part_ids.contains(&new));
                    assert!(scene.changed_ids.contains(&matrix.id));
                    if removed_axis == 0 {
                        assert!(scene.changed_ids.contains(&new));
                        assert!(scene.changed_ids.contains(&old));
                    } else {
                        assert!(!scene.changed_ids.contains(&old));
                    }
                }
            }
            let halves = after.first_halves();
            for id in &after.matrices.by_id("left-keys").part_ids {
                let target = id.replace("matrix/left-keys/", "matrix/right-keys/");
                assert_eq!(
                    halves
                        .correspondences
                        .iter()
                        .filter(|link| link.source_id == *id && link.mirror_id == target)
                        .count(),
                    1
                );
            }
            assert!(!scene.changed_ids.iter().any(|id| id == "right/U1"));
            let saved: ProjectDoc =
                serde_json::from_slice(&serde_json::to_vec(&after).unwrap()).unwrap();
            let (opened, findings) = support::prepare_open(&saved, &[]).unwrap();
            assert_eq!(opened, saved);
            assert!(findings.is_empty());
            assert_eq!(part(&after, "right/U1"), part(&before, "right/U1"));
            let mut expected_overlays = before.first_halves().exceptions.clone();
            let after_id = if removed_axis == 0 {
                "matrix/right-keys/r0c0/diode"
            } else {
                &local_id
            };
            expected_overlays
                .iter_mut()
                .find(|e| e.mirror_id == local_id)
                .unwrap()
                .mirror_id = after_id.to_owned();
            assert_eq!(halves.exceptions.len(), expected_overlays.len());
            assert!(
                expected_overlays
                    .iter()
                    .all(|e| halves.exceptions.contains(e))
            );
            assert_eq!(part(&after, after_id).locked, Some(true));
            if removed_axis == 0 {
                assert_ne!(
                    part(&after, &local_id).locked,
                    Some(true),
                    "the successor cannot inherit its predecessor's Local payload"
                );
            }
            // Walk both exits from the undone state: replay, or branch with a fresh edit.
            // An invalid axis between Undo and Redo must leave both stacks intact.
            let mut revision = after.revision;
            for (request, expected, undo_depth, accepted) in [
                (CoreRequest::Undo { id: "undo".into() }, &before, 0, true),
                (
                    command(u32::MAX, revision + 1, "invalid"),
                    &before,
                    0,
                    false,
                ),
                (CoreRequest::Redo { id: "redo".into() }, &after, 1, true),
                (
                    CoreRequest::Undo {
                        id: "undo-again".into(),
                    },
                    &before,
                    0,
                    true,
                ),
                (
                    command(removed_axis, revision + 3, "reapply"),
                    &after,
                    1,
                    true,
                ),
            ] {
                let reply = engine.handle(request);
                assert_eq!(
                    matches!(reply, CoreReply::Scene { .. }),
                    accepted,
                    "{reply:?}"
                );
                revision += u64::from(accepted);
                let mut expected = expected.clone();
                expected.revision = revision;
                assert_eq!(engine.document_for_test(), &expected);
                assert_eq!(engine.undo_for_test().len(), undo_depth);
                assert_eq!(engine.redo_for_test().len(), 1 - undo_depth);
            }
        }
    }
}

#[path = "mirrored_prepare/cutover.rs"]
mod cutover;

#[test]
fn paired_outline_deletion_preserves_neighbor_versions() {
    let mut source = matrix_fixtures::source_half();
    let versions = ["active", "neighbor"].map(|name| json!({
        "id":format!("left/{name}"), "name":name, "source":{"revision":0},
        "geometry":{"features":[], "settings":{"corners":"fillet","size":2,"bridgeWidth":5},"expectedRegions":1}
    }));
    source.board_outlines.push(
        serde_json::from_value(json!({
            "boardId":"left", "activeVersionId":"left/active", "versions":versions
        }))
        .unwrap(),
    );
    let mut paired = matrix_fixtures::create_from(
        source,
        json!({"at":{"x":200,"y":0},"rotation":0}),
        "literal-mirror",
        None,
    );
    for state in &mut paired.board_outlines {
        state
            .retired_version_ids
            .push(format!("{}/previous", state.board_id));
        if state.board_id == "right" {
            let mut local = state.versions[1].clone();
            local.id = "right/unpaired".into();
            state.generated_last_valid = Some(local.geometry.clone());
            state.versions.push(local);
        }
    }
    let delete = |author: &str, name: &str| {
        serde_json::from_value(json!({"kind":"apply-outline-edit","edit":{
        "target":{"kind":"version","boardId":author,"versionId":format!("{author}/{name}")},
        "change":"delete","seed":4}}))
        .unwrap()
    };
    let mut engine = open_and_preview::open_pair_engine(paired.clone());
    let before = engine.document_for_test().clone();
    let request = delete("right", "active");
    let (expected, _) = prepare_transaction(&before, &[], &request).unwrap();
    let commit = CoreRequest::Edit {
        id: "delete".into(),
        command: boardstudio_model::model::EditCommand {
            base_revision: before.revision,
            transaction_id: "delete".into(),
            phase: boardstudio_model::model::EditPhase::Commit,
            target_ids: vec!["right/active".into()],
            operation: request,
        },
    };
    for (request, snapshot) in [
        (commit, &expected),
        (CoreRequest::Undo { id: "undo".into() }, &before),
        (CoreRequest::Redo { id: "redo".into() }, &expected),
    ] {
        let reply = engine.handle(request);
        assert!(matches!(reply, CoreReply::Scene { .. }), "{reply:?}");
        assert_eq!(
            engine.document_for_test().board_outlines,
            snapshot.board_outlines
        );
        assert_eq!(
            engine.document_for_test().first_halves(),
            snapshot.first_halves()
        );
    }
    for (author, expected_state) in [("right", Some("omitted")), ("left", None)] {
        let mut local = paired.clone();
        let overlay = serde_json::from_value(
            json!({"kind":"outline-version", "mirrorId":"right/active", "state":"local"}),
        )
        .unwrap();
        local.first_halves_mut().exceptions.push(overlay);
        let (after, _) = prepare_transaction(&local, &[], &delete(author, "active")).unwrap();
        let target = after
            .board_outlines
            .iter()
            .find(|s| s.board_id == "right")
            .unwrap();
        let source = after
            .board_outlines
            .iter()
            .find(|s| s.board_id == "left")
            .unwrap();
        assert_eq!(
            target.versions.iter().any(|v| v.id == "right/active"),
            author == "left"
        );
        assert_eq!(
            source.versions.iter().any(|v| v.id == "left/active"),
            author == "right"
        );
        if let Some(state) = expected_state {
            assert_eq!(target.active_version_id, None);
            assert_eq!(
                serde_json::to_value(&after.first_halves().exceptions[0]).unwrap()["state"],
                state
            );
            assert_eq!(
                after.first_halves().correspondences,
                local.first_halves().correspondences
            );
        } else {
            assert!(after.first_halves().exceptions.is_empty());
            assert_eq!(
                target,
                local
                    .board_outlines
                    .iter()
                    .find(|s| s.board_id == "right")
                    .unwrap()
            );
        }
        assert_eq!(support::prepare_open(&after, &[]).unwrap(), (after, vec![]));
    }
    for (author, name) in [
        ("left", "active"),
        ("right", "active"),
        ("left", "neighbor"),
        ("right", "neighbor"),
    ] {
        let operation = delete(author, name);
        let (after, changed) = prepare_transaction(&paired, &[], &operation).unwrap();
        for board in &after.board_outlines {
            let original = paired
                .board_outlines
                .iter()
                .find(|state| state.board_id == board.board_id)
                .unwrap();
            let deleted = format!("{}/{name}", board.board_id);
            let mut expected = original.clone();
            if board.board_id == "right" {
                expected.generated_last_valid = None;
            }
            expected.versions.retain(|v| v.id != deleted);
            expected.retired_version_ids.push(deleted.clone());
            if name == "active" {
                expected.active_version_id = None;
            }
            assert_eq!(*board, expected);
            assert!(changed.contains(&board.board_id));
        }
        let remaining: Vec<_> = paired
            .first_halves()
            .correspondences
            .iter()
            .filter(|link| link.source_id != format!("left/{name}"))
            .cloned()
            .collect();
        assert_eq!(after.first_halves().correspondences, remaining);
        assert_eq!(
            support::prepare_open(&after, &[]).unwrap(),
            (after.clone(), Vec::new())
        );
        assert!(prepare_transaction(&after, &[], &operation).is_err());
    }
}

#[test]
fn matrix_growth_keeps_selected_key_state_until_rejoin() {
    let source_id = "matrix/left-keys/r1c1";
    let mirror_id = "matrix/right-keys/r1c1";
    let start = pair_document();
    for state in ["local", "omitted"] {
        let select = |state| {
            serde_json::from_value(json!({"kind":"set-half-exception","pairId":"pair",
            "itemKind":"part","mirrorId":mirror_id,"state":state}))
            .unwrap()
        };
        let (mut before, _) = prepare_transaction(&start, &[], &select(state)).unwrap();
        let retained = before.parts.iter().find(|p| p.id == mirror_id).cloned();
        assert_eq!(retained.is_some(), state == "local");
        part_mut(&mut before, source_id).properties = Some(std::collections::BTreeMap::from([(
            "edited".into(),
            json!(true),
        )]));
        let mut matrix = before.matrices.by_id("left-keys").clone();
        matrix.columns += 1;
        for row in 0..matrix.rows {
            let mut cell = matrix
                .cells
                .iter()
                .find(|c| c.column == 0 && c.row == row)
                .unwrap()
                .clone();
            cell.column = 2;
            matrix.cells.push(cell);
        }
        let (grown, _) = prepare_transaction(
            &before,
            &[],
            &serde_json::from_value(json!({"kind":"set-matrix","matrix":matrix})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            grown.parts.iter().find(|p| p.id == mirror_id),
            retained.as_ref()
        );
        for row in 0..2 {
            for suffix in ["", "/diode"] {
                let new = format!("matrix/right-keys/r{row}c2{suffix}");
                assert_eq!(
                    grown
                        .first_halves()
                        .correspondences
                        .iter()
                        .filter(|link| link.mirror_id == new
                            && link.source_id == new.replace("right-keys", "left-keys"))
                        .count(),
                    1
                );
                assert!(grown.matrices.by_id("right-keys").part_ids.contains(&new));
            }
        }
        let (after, _) = prepare_transaction(&grown, &[], &select("linked")).unwrap();
        for suffix in ["", "/diode"] {
            let source = part(&after, &format!("{source_id}{suffix}"));
            let mirror = part(&after, &format!("{mirror_id}{suffix}"));
            assert_eq!(source.properties, mirror.properties);
            assert_eq!(source.generator_parameters, mirror.generator_parameters);
            assert_eq!(source.side, mirror.side);
        }
        assert!(after.first_halves().exceptions.is_empty());
        let (opened, findings) = support::prepare_open(&after, &[]).unwrap();
        assert_eq!(opened, after);
        assert!(findings.is_empty());
    }
}

#[test]
fn constraint_endpoints_follow_local_rejoin_and_omit() {
    use boardstudio_model::model::{HalfExceptionState, MirroredItemKind};
    let mut doc = pair_document();
    open_and_preview::add_linked_independent_part(&mut doc);
    let literal = |id: &str| ConstraintEndpoint::Part { part_id: id.into() };
    let linked = linked_endpoint("pair", "left/RST", "right/RST");
    open_and_preview::add_unpaired_mirror_board_part(&mut doc);
    let constraints = |selected: ConstraintEndpoint| {
        vec![
            Constraint::Framed {
                id: "incoming".into(),
                source: literal("left/U1"),
                target: selected.clone(),
                relation: ConstraintRelation::Offset {
                    offset: Vec2 { x: 236.0, y: 20.0 },
                    rotation: 0.0,
                },
            },
            Constraint::Framed {
                id: "outgoing".into(),
                source: selected,
                target: literal("right/FREE"),
                relation: ConstraintRelation::Offset {
                    offset: Vec2 { x: 10.0, y: 20.0 },
                    rotation: 0.0,
                },
            },
        ]
    };
    doc.constraints = constraints(linked.clone());
    let select = |doc: &ProjectDoc, state| {
        prepare_transaction(
            doc,
            &[],
            &EditOperation::SetHalfException {
                pair_id: "pair".into(),
                kind: MirroredItemKind::Part,
                mirror_id: "right/RST".into(),
                state,
            },
        )
        .unwrap()
        .0
    };
    let local = select(&doc, HalfExceptionState::Local);
    assert_eq!(local.constraints, constraints(literal("right/RST")));
    let rejoined = select(&local, HalfExceptionState::Linked);
    assert_eq!(rejoined.constraints, constraints(linked)[1..]);
    let omitted = select(&rejoined, HalfExceptionState::Omitted);
    assert!(omitted.constraints.is_empty());
    assert!(
        select(&omitted, HalfExceptionState::Linked)
            .constraints
            .is_empty()
    );
}

#[test]
fn deleting_an_independent_source_retires_its_link_and_preserves_local_payload() {
    let mut start = pair_document();
    open_and_preview::add_linked_independent_part(&mut start);
    for state in ["linked", "local", "omitted"] {
        let before = if state == "linked" {
            start.clone()
        } else {
            let op = serde_json::from_value(json!({"kind":"set-half-exception","pairId":"pair",
                "itemKind":"part","mirrorId":"right/RST","state":state}))
            .unwrap();
            prepare_transaction(&start, &[], &op).unwrap().0
        };
        let operation = EditOperation::RemoveParts {
            ids: vec!["left/RST".into()],
        };
        let (after, _) = prepare_transaction(&before, &[], &operation).unwrap();
        let mut expected = before.first_halves().clone();
        expected
            .correspondences
            .retain(|row| row.source_id != "left/RST");
        expected
            .exceptions
            .retain(|row| row.mirror_id != "right/RST");
        assert_eq!(after.first_halves(), &expected);
        assert!(!after.parts.iter().any(|p| p.id == "left/RST"));
        assert_eq!(
            after.parts.iter().find(|p| p.id == "right/RST"),
            before
                .parts
                .iter()
                .find(|p| p.id == "right/RST" && state == "local")
        );
        assert_eq!(
            after
                .boards
                .by_id("right")
                .part_ids
                .iter()
                .any(|p| p == "right/RST"),
            state == "local"
        );
        assert_eq!(support::prepare_open(&after, &[]).unwrap(), (after, vec![]));
    }
}
