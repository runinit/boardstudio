// Mirrored-half prepared-transaction integration tests, split into the
// submodules under `mirrored_prepare/` so no file exceeds ~1,600 LOC.
// Shared fixtures/helpers live in `support/`; each submodule inherits this
// crate's imports via `use super::*`.
#[path = "support/matrix.rs"]
pub mod matrix_fixtures;
pub mod support;

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
            let halves = paired.hardware.as_mut().unwrap().pairs[0]
                .halves
                .as_mut()
                .unwrap();
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
            let halves = after.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap();
            for id in &after
                .matrices
                .iter()
                .find(|matrix| matrix.id == "left-keys")
                .unwrap()
                .part_ids
            {
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
            let mut expected_overlays = before.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .exceptions
                .clone();
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
