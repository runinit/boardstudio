//! Table-driven controller tests for Layout align routing and execution.
//!
//! Resolver scenarios run the real Session and Core through the extracted
//! `align_resolver` and `make_edit` helpers.
use super::super::layout_align_logic::tests::{bent_column_document, key_part};
use super::*;
use boardstudio_model::model::{
    Board, Follow, MatrixAnchor, MatrixCell, MatrixPlacementChange, OutlineFeature,
    OutlineSettings, Pose2,
};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::draft_edits::{DraftEdits, PendingEditResult};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Keyboard");
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "envelope-main".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: boardstudio_model::model::Operation::Add,
    });
    document.boards.push(Board {
        id: "board-main".into(),
        name: "Main".into(),
        outline_ids: vec!["envelope-main".into()],
        part_ids: vec!["a".into(), "b".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch:base",
            "name": "MX switch",
            "kind": "switch",
            "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}, {"x": -3.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap(),
    );
    document.parts.push(key_part("a", "A", 0.0, 0.0));
    document.parts.push(key_part("b", "B", 20.0, 0.0));
    document
}

fn context() -> TreeContext {
    TreeContext::Key {
        matrix_id: "matrix-main".into(),
        row: 0,
        column: 0,
    }
}

fn pose_x(runtime: &Rc<Runtime>, id: &str) -> Option<f64> {
    runtime
        .model()
        .accepted
        .as_ref()?
        .document
        .parts
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.pose.at.x)
}

#[wasm_bindgen_test]
async fn align_moves_the_selection_against_the_accepted_reference() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, fixture()).await;
    let pending = DraftEdits::<_, ()>::default();
    pending.begin(
        &runtime,
        (),
        "layout-align",
        Some("alignment".into()),
        align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
    );
    support::run_pending(&runtime).await;
    assert!(matches!(
        pending.discard_drafts(true).as_slice(),
        [PendingEditResult::Landed { .. }]
    ));
    assert_eq!(pose_x(&runtime, "a"), Some(20.0));
}

#[wasm_bindgen_test]
async fn align_of_a_part_deleted_before_execution_retires_with_a_reason() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, fixture()).await;
    let accepted = runtime.model().accepted.expect("the fixture opens");
    // Hold the delete's Core reply so the align queues behind it.
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let mut without_a = accepted.document.as_ref().clone();
    without_a.parts.retain(|part| part.id != "a");
    without_a.boards[0].part_ids.retain(|id| id != "a");
    support::submit_fixed_command(
        &runtime,
        EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "delete-a".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["a".into()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(without_a),
            },
        },
    );
    support::drive_pending(&runtime);
    entered.await.expect("the delete reached Core");
    let pending = DraftEdits::<_, ()>::default();
    pending.begin(
        &runtime,
        (),
        "layout-align",
        Some("alignment".into()),
        align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
    );
    assert!(
        pending.is_pending(&()),
        "a one-shot align stays pending until it settles"
    );
    support::drive_pending(&runtime);
    release.send(()).expect("release the held delete");
    for _ in 0..50 {
        support::run_pending(&runtime).await;
        if !pending.is_pending(&()) {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    match pending.discard_drafts(true).as_slice() {
        [PendingEditResult::Failed { message, .. }] => assert!(
            message.contains("no longer exists"),
            "the reason is explained: {message}"
        ),
        other => panic!("expected a retirement with a reason, got {other:?}"),
    }
    assert_eq!(pose_x(&runtime, "a"), None);
}

fn accepted_poses(runtime: &Rc<Runtime>, ids: &[&str]) -> Vec<Pose2> {
    let accepted = runtime.model().accepted.expect("an accepted document");
    ids.iter()
        .map(|id| {
            accepted
                .document
                .parts
                .iter()
                .find(|part| &part.id == id)
                .unwrap_or_else(|| panic!("part {id} exists"))
                .pose
        })
        .collect()
}

async fn commit_placement(runtime: &Rc<Runtime>, operation: EditOperation, target: &str) {
    let revision = runtime
        .model()
        .accepted
        .expect("an accepted document")
        .document
        .revision;
    support::submit_fixed_command(
        runtime,
        EditCommand {
            base_revision: revision,
            transaction_id: format!("ticket-22-{target}"),
            phase: EditPhase::Commit,
            target_ids: vec![target.to_owned()],
            operation,
        },
    );
    support::run_pending(runtime).await;
}

/// Ticket 22 first slice: a Column Align on a bent column routes through Core's
/// exact column edit (never client column offsets), so every target moves rigidly.
#[wasm_bindgen_test::wasm_bindgen_test]
async fn column_align_on_a_bent_column_moves_targets_rigidly() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, bent_column_document()).await;
    commit_placement(
        &runtime,
        EditOperation::SetMatrixPlacement {
            matrix_id: "m".into(),
            change: MatrixPlacementChange::KeyBend {
                row: 1,
                column: 0,
                value: 30.0,
            },
        },
        "m",
    )
    .await;
    let moving = vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()];
    let before = accepted_poses(&runtime, &["matrix/m/r0c0", "matrix/m/r1c0"]);
    assert!(
        (before[0].rotation - before[1].rotation).abs() > 1.0,
        "the bend turns the second key: {before:?}"
    );
    let pending = DraftEdits::<_, ()>::default();
    pending.begin(
        &runtime,
        (),
        "layout-align",
        Some("alignment".into()),
        align_resolver(
            TreeContext::Column {
                matrix_id: "m".into(),
                column: 0,
            },
            moving,
            "ref-part".into(),
            AlignCommand::Left,
        ),
    );
    support::run_pending(&runtime).await;
    match pending.discard_drafts(true).as_slice() {
        [PendingEditResult::Landed { .. }] => {}
        other => panic!("a rigid column align lands, got {other:?}"),
    }
    let after = accepted_poses(&runtime, &["matrix/m/r0c0", "matrix/m/r1c0"]);
    let delta = |index: usize| Vec2 {
        x: after[index].at.x - before[index].at.x,
        y: after[index].at.y - before[index].at.y,
    };
    let first = delta(0);
    let second = delta(1);
    assert!(
        first.x.hypot(first.y) > 1e-9,
        "the align moves the column: {before:?} -> {after:?}"
    );
    assert!(
        (first.x - second.x).abs() < 1e-9 && (first.y - second.y).abs() < 1e-9,
        "both bent-column targets share one rigid translation: {before:?} -> {after:?}"
    );
    assert_eq!(after[0].rotation, before[0].rotation);
    assert_eq!(after[1].rotation, before[1].rotation);
}

/// Ticket 22 first slice: an anchored/Follow refusal surfaces with the document
/// and revision unchanged.
#[wasm_bindgen_test::wasm_bindgen_test]
async fn anchored_follow_refusal_leaves_the_document_unchanged() {
    let runtime = support::new_runtime();
    let mut document = bent_column_document();
    document.matrices.retain(|matrix| matrix.id == "m");
    document.matrices[0].rows = 1;
    document.matrices[0].columns = 2;
    document.matrices[0].part_ids = vec!["matrix/m/r0c0".into(), "matrix/m/r0c1".into()];
    document.matrices[0].cells = document.matrices[0]
        .cells
        .iter()
        .filter(|cell| cell.row == 0)
        .cloned()
        .collect();
    document.matrices[0].cells.push(MatrixCell {
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
        assemblies: vec![],
        assemblies_local: None,
    });
    document.parts.retain(|part| part.id != "matrix/m/r1c0");
    document
        .parts
        .push(key_part("matrix/m/r0c1", "SW2", 19.05, 0.0));
    document.boards[0].part_ids = vec![
        "matrix/m/r0c0".into(),
        "matrix/m/r0c1".into(),
        "ref-part".into(),
    ];
    support::open_document(&runtime, document).await;
    commit_placement(
        &runtime,
        EditOperation::AddAnchoredMatrix {
            id: "dep".into(),
            anchor: MatrixAnchor {
                matrix_id: "m".into(),
                row: 0,
                column: 1,
                follow: Follow {
                    x: false,
                    y: true,
                    rotation: true,
                },
                aggregate: None,
            },
        },
        "dep",
    )
    .await;
    let accepted = runtime.model().accepted.expect("an accepted document");
    let moving: Vec<String> = accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "dep")
        .expect("the anchored matrix exists")
        .part_ids
        .clone();
    assert!(!moving.is_empty(), "the anchored matrix materializes parts");
    let revision = accepted.document.revision;
    let parts = accepted.document.parts.clone();
    let pending = DraftEdits::<_, ()>::default();
    pending.begin(
        &runtime,
        (),
        "layout-align",
        Some("alignment".into()),
        align_resolver(
            TreeContext::Matrix {
                matrix_id: "dep".into(),
            },
            moving,
            "ref-part".into(),
            AlignCommand::Left,
        ),
    );
    support::run_pending(&runtime).await;
    match pending.discard_drafts(true).as_slice() {
        [PendingEditResult::Failed { message, .. }] => assert!(
            message.contains("Follow"),
            "the refusal names the Follow restriction: {message}"
        ),
        other => panic!("an unfollowed align refuses, got {other:?}"),
    }
    let after = runtime.model().accepted.expect("an accepted document");
    assert_eq!(after.document.revision, revision);
    assert_eq!(after.document.parts, parts);
}
