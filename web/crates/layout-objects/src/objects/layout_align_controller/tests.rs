//! Table-driven controller tests for Layout align routing and execution.
//!
//! Workspace-route admission is a pure truth table; resolver scenarios run the
//! real Session and Core through the extracted `align_resolver` and `make_edit`
//! helpers so every direction and context keeps its browser coverage.
use super::*;
use boardstudio_model::model::{
    Board, Follow, Layout, Matrix, MatrixAnchor, MatrixCell, MatrixPlacementChange, OutlineFeature,
    OutlineSettings, Pose2, Side,
};
use boardstudio_web_test_runtime::project_name_test_support as support;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn align_route_is_exactly_scoped_to_its_workspace_owner() {
    // (workspace, owner, admitted): admission is exactly scoped to the owning workspace.
    for (workspace, owner, admitted) in [
        ("Layout", "Layout", true),
        ("PCB", "PCB", true),
        ("PCB", "Layout", false),
        ("Layout", "PCB", false),
        ("Parts", "Parts", false),
        ("Layout", "Parts", false),
    ] {
        assert_eq!(
            workspace_route_matches(workspace, owner),
            admitted,
            "route ({workspace}, {owner})"
        );
    }
}

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
    let mut pending = PendingEdits::default();
    pending.begin(
        &runtime,
        (),
        "layout-align",
        Some("alignment".into()),
        align_resolver(context(), vec!["a".into()], "b".into(), AlignCommand::Left),
    );
    support::run_pending(&runtime).await;
    assert!(matches!(
        pending.settle(true).as_slice(),
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
    let mut pending = PendingEdits::default();
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
    match pending.settle(true).as_slice() {
        [PendingEditResult::Failed { message, .. }] => assert!(
            message.contains("no longer exists"),
            "the reason is explained: {message}"
        ),
        other => panic!("expected a retirement with a reason, got {other:?}"),
    }
    assert_eq!(pose_x(&runtime, "a"), None);
}

fn switch_definition() -> serde_json::Value {
    serde_json::json!({
        "id": "switch:base",
        "name": "MX switch",
        "kind": "switch",
        "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
        "pads": []
    })
}

fn key_part(id: &str, reference: &str, x: f64, y: f64) -> Part {
    Part {
        keycap: None,
        outline: None,
        anchor: None,
        id: id.into(),
        definition_id: "switch:base".into(),
        reference: reference.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn bent_column_document() -> ProjectDoc {
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
        part_ids: vec![
            "matrix/m/r0c0".into(),
            "matrix/m/r1c0".into(),
            "ref-part".into(),
        ],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
        .definitions
        .push(serde_json::from_value(switch_definition()).unwrap());
    document
        .parts
        .push(key_part("matrix/m/r0c0", "SW1", 0.0, 0.0));
    document
        .parts
        .push(key_part("matrix/m/r1c0", "SW2", 0.0, 19.05));
    document.parts.push(key_part("ref-part", "REF", 40.0, 0.0));
    document.matrices.push(Matrix {
        id: "m".into(),
        name: None,
        rows: 2,
        columns: 1,
        pitch: Vec2 { x: 19.05, y: 19.05 },
        origin: Vec2::default(),
        definition_id: "switch:base".into(),
        part_ids: vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()],
        board_id: Some("board-main".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![
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
                assemblies: vec![],
                assemblies_local: None,
            },
            MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 1,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            },
        ],
    });
    document.layouts.push(Layout {
        id: "layout".into(),
        name: "Main".into(),
        board_id: "board-main".into(),
        matrix_ids: vec!["m".into()],
        part_ids: vec!["ref-part".into()],
        mirror_link: None,
    });
    document
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

/// The Core operation family `make_edit` submits for each alignable context.
#[derive(Debug, PartialEq)]
enum EditFamily {
    ColumnTranslation,
    MatrixTranslation,
    None,
    MoveParts,
}

fn edit_family(edit: &Option<(EditOperation, Vec<String>)>) -> EditFamily {
    match edit {
        Some((
            EditOperation::SetMatrixPlacement {
                change: MatrixPlacementChange::ColumnTranslation { .. },
                ..
            },
            _,
        )) => EditFamily::ColumnTranslation,
        Some((
            EditOperation::SetMatrixPlacement {
                change: MatrixPlacementChange::MatrixTranslation { .. },
                ..
            },
            _,
        )) => EditFamily::MatrixTranslation,
        Some((EditOperation::MoveParts { .. }, _)) => EditFamily::MoveParts,
        _ => EditFamily::None,
    }
}

/// Ticket 22: the Align consumer submits Core's exact edits, never client
/// column offsets. Column goes through `ColumnTranslation`, Matrix through its
/// frame, Row stays unavailable, and keys keep the `MoveParts` pose route.
#[wasm_bindgen_test::wasm_bindgen_test]
fn align_submits_the_exact_core_edits() {
    let document = bent_column_document();
    let delta = Vec2 { x: -3.0, y: 1.5 };
    let moving = vec!["matrix/m/r0c0".into(), "matrix/m/r1c0".into()];
    // (context, moving subset, expected Core operation family).
    let cases: Vec<(TreeContext, Vec<String>, EditFamily)> = vec![
        (
            TreeContext::Column {
                matrix_id: "m".into(),
                column: 0,
            },
            moving.clone(),
            EditFamily::ColumnTranslation,
        ),
        (
            TreeContext::Matrix {
                matrix_id: "m".into(),
            },
            moving.clone(),
            EditFamily::MatrixTranslation,
        ),
        (
            TreeContext::Row {
                matrix_id: "m".into(),
                row: 0,
            },
            moving.clone(),
            EditFamily::None,
        ),
        (
            TreeContext::Key {
                matrix_id: "m".into(),
                row: 0,
                column: 0,
            },
            moving[..1].to_vec(),
            EditFamily::MoveParts,
        ),
    ];
    for (context, moving, expected) in &cases {
        let edit = make_edit(&document, context, moving, delta);
        assert_eq!(
            edit_family(&edit),
            *expected,
            "{context:?} must route through {expected:?}, got {edit:?}"
        );
    }
    // A bent column must Align rigidly with its exact translation and target.
    match make_edit(&document, &cases[0].0, &cases[0].1, delta) {
        Some((
            EditOperation::SetMatrixPlacement {
                matrix_id,
                change: MatrixPlacementChange::ColumnTranslation { column, delta },
            },
            targets,
        )) => {
            assert_eq!(matrix_id, "m");
            assert_eq!(column, 0);
            assert_eq!(delta, Vec2 { x: -3.0, y: 1.5 });
            assert_eq!(targets, vec!["m".to_owned()]);
        }
        other => panic!("a bent column must Align rigidly, got {other:?}"),
    }
    // Keys keep the MoveParts pose route with the exact translated pose.
    match make_edit(&document, &cases[3].0, &cases[3].1, delta) {
        Some((EditOperation::MoveParts { positions }, _)) => {
            assert_eq!(positions.len(), 1);
            assert_eq!(positions[0].at, Vec2 { x: -3.0, y: 1.5 });
        }
        other => panic!("keys keep the MoveParts pose route, got {other:?}"),
    }
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
    let mut pending = PendingEdits::default();
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
    match pending.settle(true).as_slice() {
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
    let mut pending = PendingEdits::default();
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
    match pending.settle(true).as_slice() {
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
