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
