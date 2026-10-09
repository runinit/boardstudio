//! Native adapter tests: one presentation action becomes one typed Core outline edit,
//! and Core settles it against the accepted document.
//!
//! These replace the removed web `plan_action` rule tests. The rules they used to pin
//! (identity allocation, polygon validation, retirement, no-op) now live in Core and are
//! covered by `core/tests/outline_edits.rs`; what is pinned here is the adapter contract:
//! target selection, the captured creation seed, the reviewed source shape, the explicit
//! connection width, and the admission/settlement mapping.
use crate::outline::adapter::{self, Admission};
use crate::outline::planner::{OutlineAction, OutlinePointTarget};
use crate::outline_settings::{OutlineEdit, generated_feature, reference_outline_settings};
use boardstudio_application::{AcceptedSnapshot, Scope, SessionEpoch};
use boardstudio_core::model::{
    Board, BoardOutline, Contour, EditOperation, EditPhase, Operation, OutlineFeature,
    OutlineOwnership, OutlineProvenance, OutlineSnapshot, OutlineVersion, ProjectDoc, Readiness,
    SceneDelta, Vec2,
};
use boardstudio_core::outline_edit::{OutlineChange, OutlineEditDisposition, OutlineEditTarget};
use boardstudio_web_ui_model::tree::TreeContext;
use std::sync::Arc;

const BOARD: &str = "board";

fn board() -> Board {
    Board {
        id: BOARD.into(),
        name: "Main".into(),
        outline_ids: vec!["generated".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

fn generated_envelope() -> OutlineFeature {
    OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: reference_outline_settings(),
        id: "generated".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: Operation::Add,
    }
}

fn polygon(id: &str, x: f64) -> OutlineFeature {
    OutlineFeature::Polygon {
        id: id.into(),
        anchor_part_id: None,
        points: vec![
            Vec2 { x, y: 0.0 },
            Vec2 {
                x: x + 10.0,
                y: 0.0,
            },
            Vec2 {
                x: x + 10.0,
                y: 10.0,
            },
            Vec2 { x, y: 10.0 },
        ],
        operation: Operation::Add,
    }
}

/// A board with a Generated envelope and one fixed named version ("v1") active.
fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("adapter-doc", "Adapter");
    document.boards.push(board());
    document.outline.push(generated_envelope());
    document.board_outlines.push(BoardOutline {
        board_id: BOARD.into(),
        active_version_id: Some("v1".into()),
        versions: vec![OutlineVersion {
            id: "v1".into(),
            name: "Edited outline 1".into(),
            source: OutlineProvenance {
                revision: 0,
                version_id: None,
            },
            geometry: OutlineSnapshot {
                features: vec![polygon("base", 0.0), polygon("second", 30.0)],
                settings: reference_outline_settings(),
                expected_regions: 1,
                bridges: vec![],
                protected_gaps: vec![],
            },
            ownership: OutlineOwnership::Fixed,
        }],
        generated_last_valid: None,
        retired_version_ids: vec![],
    });
    document
}

fn scene(source_contours: Vec<Contour>) -> SceneDelta {
    SceneDelta {
        module_scenes: vec![],
        revision: 0,
        transaction_id: String::new(),
        changed_ids: vec![],
        transforms: vec![],
        matrix_scenes: vec![],
        contours: vec![],
        board_contours: vec![],
        board_readiness: vec![],
        board_outline_scenes: vec![boardstudio_core::model::BoardOutlineScene {
            board_id: BOARD.into(),
            source_contours,
            bridges: vec![],
            gaps: vec![],
        }],
        finding_markers: vec![],
        findings: vec![],
        readiness: Readiness {
            layout: false,
            outline: false,
            pcb: false,
            case_ready: false,
        },
    }
}

fn scope() -> Scope {
    Scope {
        session_epoch: SessionEpoch(1),
        document_id: "adapter-doc".into(),
        board_id: BOARD.into(),
        instance_id: None,
    }
}

fn snapshot_with(document: ProjectDoc, source_contours: Vec<Contour>) -> AcceptedSnapshot {
    AcceptedSnapshot {
        token: boardstudio_application::SnapshotToken(1),
        session_epoch: SessionEpoch(1),
        document: Arc::new(document),
        scene: Arc::new(scene(source_contours)),
    }
}

fn snapshot(document: ProjectDoc) -> AcceptedSnapshot {
    snapshot_with(document, vec![])
}

fn envelope() -> (
    Scope,
    boardstudio_application::SnapshotToken,
    u64,
    u64,
    String,
) {
    (
        scope(),
        boardstudio_application::SnapshotToken(1),
        0,
        1,
        BOARD.into(),
    )
}

fn version_context(version_id: &str) -> TreeContext {
    TreeContext::OutlineVersion {
        board_id: BOARD.into(),
        version_id: Some(version_id.into()),
    }
}

fn edit_for(
    document: &ProjectDoc,
    action: &OutlineAction,
    seed: u64,
) -> boardstudio_core::outline_edit::OutlineEdit {
    adapter::outline_edit(&snapshot(document.clone()), action, seed)
        .expect("the action maps to one typed outline edit")
}

/// Generated is active: the board owns its source directly.
fn generated_active() -> ProjectDoc {
    let mut document = document();
    document.board_outlines[0].active_version_id = None;
    document
}

/// A board whose Generated outline composes from an authored polygon, so Core's
/// generated-source projection is non-empty.
fn generated_polygon() -> ProjectDoc {
    let mut document = ProjectDoc::empty("adapter-doc", "Adapter");
    let mut board = board();
    board.outline_ids = vec!["gen-poly".into()];
    document.boards.push(board);
    document.outline.push(polygon("gen-poly", 0.0));
    document
}

fn polygon_contour() -> Contour {
    let OutlineFeature::Polygon { points, .. } = polygon("gen-poly", 0.0) else {
        unreachable!()
    };
    Contour {
        points,
        hole: false,
    }
}

#[test]
fn activation_of_a_gone_version_settles_as_a_retirement_with_a_reason() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::Activate {
        scope,
        token,
        revision,
        generation,
        context: version_context("missing"),
        require_selected_version: true,
        board_id,
        version_id: Some("missing".into()),
    };
    let document = document();
    let edit = edit_for(&document, &action, 1);
    assert!(matches!(
        boardstudio_core::outline_edit::evaluate(&document, &edit),
        Ok(OutlineEditDisposition::Retire)
    ));
    assert_eq!(
        adapter::retire_reason(&action),
        "That outline version no longer exists."
    );
    // Admission drops a doomed ticket at dispatch instead of queueing it.
    assert_eq!(
        adapter::admission(&snapshot(document), &edit),
        Admission::Ignore
    );
}

#[test]
fn activating_generated_one_action_is_one_board_target_edit() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::Activate {
        scope,
        token,
        revision,
        generation,
        context: TreeContext::Outline {
            board_id: BOARD.into(),
        },
        require_selected_version: false,
        board_id,
        version_id: None,
    };
    let document = document();
    let edit = edit_for(&document, &action, 2);
    assert!(matches!(edit.target, OutlineEditTarget::Board { .. }));
    assert!(matches!(edit.change, OutlineChange::Activate));
    assert_eq!(edit.seed, 2, "the creation seed travels with the edit");
    assert!(edit.reviewed.is_none());
    let mut accepted = document.clone();
    boardstudio_core::outline_edit::apply(&mut accepted, &edit).unwrap();
    assert!(accepted.board_outlines[0].active_version_id.is_none());
    // One operation, one revision: the in-place generated edit is not a replacement.
    let mut original = document.clone();
    original.revision += 1;
    assert_eq!(
        accepted.revision, document.revision,
        "an activation does not bump the revision"
    );
}

#[test]
fn a_connection_action_supplies_the_board_bridge_width_explicitly() {
    let (scope, token, revision, generation, board_id) = envelope();
    let document = generated_active();
    let action = OutlineAction::AddConnection {
        scope,
        token,
        revision,
        generation,
        board_id,
        context: TreeContext::Outline {
            board_id: BOARD.into(),
        },
        points: vec![Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: 5.0, y: 1.0 }],
    };
    let edit = edit_for(&document, &action, 3);
    let OutlineChange::AddConnection { connection } = &edit.change else {
        panic!("a connection add is one typed connection change");
    };
    assert_eq!(connection.width, reference_outline_settings().bridge_width);
    assert_eq!(
        adapter::admission(&snapshot(document.clone()), &edit),
        Admission::Submit
    );
    let mut accepted = document;
    boardstudio_core::outline_edit::apply(&mut accepted, &edit).unwrap();
    let OutlineFeature::PartEnvelope { connections, .. } =
        generated_feature(&accepted, &accepted.boards[0]).unwrap()
    else {
        panic!("the generated envelope owns connections");
    };
    assert_eq!(connections.len(), 1);
    assert_eq!(
        connections[0].width,
        reference_outline_settings().bridge_width
    );
}

#[test]
fn add_feature_on_generated_submits_one_feature_change_and_core_names_the_version() {
    let (scope, token, revision, generation, board_id) = envelope();
    let document = generated_polygon();
    let action = OutlineAction::AddFeature {
        scope,
        token,
        revision,
        generation,
        board_id: board_id.clone(),
        context: TreeContext::Outline { board_id },
        feature: polygon("outline-manual", 12.0),
    };
    let edit = edit_for(&document, &action, 4);
    assert!(matches!(edit.target, OutlineEditTarget::Board { .. }));
    assert!(matches!(edit.change, OutlineChange::AddFeature { .. }));
    let mut accepted = document.clone();
    boardstudio_core::outline_edit::apply(&mut accepted, &edit).unwrap();
    assert_eq!(
        accepted.board_outlines[0].active_version_id.as_deref(),
        Some("outline-version-4")
    );
    let created = accepted.board_outlines[0]
        .versions
        .iter()
        .find(|version| version.id == "outline-version-4")
        .expect("Core created and selected the version");
    assert!(
        created.name.starts_with("Edited outline "),
        "Core owns the created version name, not the adapter: {}",
        created.name
    );
}

#[test]
fn malformed_perimeter_points_surface_a_core_error_and_change_nothing() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::EditPerimeter {
        scope,
        token,
        revision,
        generation,
        board_id,
        context: TreeContext::Outline {
            board_id: BOARD.into(),
        },
        target: OutlinePointTarget::Generated { contour: 0 },
        points: vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 1.0, y: 0.0 }],
        phase: EditPhase::Commit,
        transaction_id: "t".into(),
    };
    let document = generated_active();
    let edit = edit_for(&document, &action, 5);
    let error = boardstudio_core::outline_edit::evaluate(&document, &edit)
        .expect_err("two points is malformed submitted geometry");
    assert!(error.contains("three finite points"), "{error}");
    assert_eq!(
        adapter::admission(&snapshot(document.clone()), &edit),
        Admission::Malformed
    );
    let mut untouched = document.clone();
    assert!(boardstudio_core::outline_edit::apply(&mut untouched, &edit).is_err());
    assert_eq!(untouched, document, "a malformed edit mutates nothing");
    // The hook still submits it so Core's own message reaches the user.
    match adapter::settle(&snapshot(document), &edit, &action, String::new()) {
        boardstudio_application::Resolution::Retire(message) => {
            assert!(message.contains("three finite points"))
        }
        other => panic!("a malformed edit surfaces Core's error, got {other:?}"),
    }
}

#[test]
fn a_generated_perimeter_edit_captures_the_reviewed_source_shape() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::EditPerimeter {
        scope,
        token,
        revision,
        generation,
        board_id,
        context: TreeContext::Outline {
            board_id: BOARD.into(),
        },
        target: OutlinePointTarget::Generated { contour: 0 },
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 20.0, y: 0.0 },
            Vec2 { x: 0.0, y: 20.0 },
        ],
        phase: EditPhase::Commit,
        transaction_id: "t".into(),
    };
    let document = generated_polygon();
    let reviewed = vec![polygon_contour()];
    let reviewed_snapshot = snapshot_with(document.clone(), reviewed.clone());
    let edit = adapter::outline_edit(&reviewed_snapshot, &action, 6).unwrap();
    assert_eq!(
        edit.reviewed,
        Some(reviewed),
        "the reviewed generated shape travels with the pending edit"
    );
    // Only the reviewed source differs: with none captured the same document and action
    // are admissible, so the shape the user reviewed is what retires the queued edit.
    let mut without_review = edit.clone();
    without_review.reviewed = None;
    assert_ne!(
        adapter::admission(&reviewed_snapshot, &without_review),
        Admission::Ignore,
        "the document and target are otherwise eligible for this edit"
    );
    let changed = snapshot_with(
        document,
        vec![Contour {
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 11.0, y: 0.0 },
                Vec2 { x: 11.0, y: 10.0 },
            ],
            hole: false,
        }],
    );
    assert_eq!(
        adapter::admission(&changed, &edit),
        Admission::Ignore,
        "a relevant source change retires the reviewed edit"
    );
}

#[test]
fn a_version_edit_retires_when_another_version_becomes_active() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::SetFeature {
        scope,
        token,
        revision,
        generation,
        board_id,
        context: version_context("v1"),
        version_id: Some("v1".into()),
        before: polygon("second", 30.0),
        after: polygon("second", 35.0),
    };
    let document = document();
    let edit = edit_for(&document, &action, 7);
    assert_eq!(
        adapter::admission(&snapshot(document.clone()), &edit),
        Admission::Submit
    );
    // Version B became active before the queued edit ran; A is never redirected at B.
    let mut activated = document.clone();
    activated.board_outlines[0].versions.push(OutlineVersion {
        id: "v2".into(),
        name: "Edited outline 2".into(),
        source: OutlineProvenance {
            revision: 0,
            version_id: Some("v1".into()),
        },
        geometry: OutlineSnapshot {
            features: vec![polygon("base", 0.0)],
            settings: reference_outline_settings(),
            expected_regions: 1,
            bridges: vec![],
            protected_gaps: vec![],
        },
        ownership: OutlineOwnership::Fixed,
    });
    activated.board_outlines[0].active_version_id = Some("v2".into());
    assert_eq!(
        adapter::admission(&snapshot(activated.clone()), &edit),
        Admission::Ignore,
        "an edit on A retires once B is active"
    );
    let before = activated.board_outlines[0].versions[0].clone();
    assert!(matches!(
        boardstudio_core::outline_edit::evaluate(&activated, &edit),
        Ok(OutlineEditDisposition::Retire)
    ));
    assert_eq!(
        activated.board_outlines[0].versions[0], before,
        "a retired edit never changes another version"
    );
}

#[test]
fn unrelated_metadata_changes_do_not_retire_a_queued_edit() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::SetFeature {
        scope,
        token,
        revision,
        generation,
        board_id,
        context: version_context("v1"),
        version_id: Some("v1".into()),
        before: polygon("second", 30.0),
        after: polygon("second", 35.0),
    };
    let document = document();
    let edit = edit_for(&document, &action, 8);
    let mut renamed_unrelated = document.clone();
    renamed_unrelated.name = "Renamed project".into();
    renamed_unrelated.board_outlines[0].versions[0].name = "Edited outline 7".into();
    assert_eq!(
        adapter::admission(&snapshot(renamed_unrelated), &edit),
        Admission::Submit,
        "unrelated names and metadata survive"
    );
}

#[test]
fn two_queued_field_edits_carry_distinct_seeds_and_both_settle_in_order() {
    let (scope, token, revision, generation, board_id) = envelope();
    let mut document = generated_active();
    document.board_outlines = vec![];
    let first = {
        let (scope, token, revision, generation, board_id) =
            (scope.clone(), token, revision, generation, board_id.clone());
        OutlineAction::Update {
            scope,
            token,
            revision,
            generation,
            board_id,
            edit: OutlineEdit::SetMargin(3.0),
        }
    };
    let second = OutlineAction::Update {
        scope,
        token,
        revision,
        generation,
        board_id,
        edit: OutlineEdit::SetMargin(4.0),
    };
    let first_edit = adapter::outline_edit(&snapshot(document.clone()), &first, 11).unwrap();
    let second_edit = adapter::outline_edit(&snapshot(document.clone()), &second, 12).unwrap();
    assert_ne!(first_edit.seed, second_edit.seed);
    let mut accepted = document;
    boardstudio_core::outline_edit::apply(&mut accepted, &first_edit).unwrap();
    // The second edit is re-evaluated against the document the first one landed on.
    assert_eq!(
        adapter::admission(&snapshot(accepted.clone()), &second_edit),
        Admission::Submit
    );
    boardstudio_core::outline_edit::apply(&mut accepted, &second_edit).unwrap();
    let OutlineFeature::PartEnvelope { margin, .. } =
        generated_feature(&accepted, &accepted.boards[0]).unwrap()
    else {
        panic!("the generated envelope owns the margin");
    };
    assert_eq!(
        *margin, 4.0,
        "the later field edit wins in submission order"
    );
}

#[test]
fn copy_names_and_identities_come_from_the_accepted_document() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::Copy {
        scope,
        token,
        revision,
        generation,
        board_id,
    };
    let document = document();
    let edit = edit_for(&document, &action, 9);
    let OutlineChange::Copy { version_id, name } = &edit.change else {
        panic!("a copy action is one typed copy change");
    };
    assert_eq!(version_id, "outline-version-9");
    assert_eq!(name, "Edited outline 2");
    assert!(matches!(edit.target, OutlineEditTarget::Board { .. }));
}

#[test]
fn freeze_is_one_typed_edit_that_keeps_the_linked_version_and_selects_the_copy() {
    let (scope, token, revision, generation, board_id) = envelope();
    let mut document = document();
    document.board_outlines[0].versions[0].ownership = OutlineOwnership::Linked;
    let action = OutlineAction::Freeze {
        scope,
        token,
        revision,
        generation,
        board_id,
        version_id: "v1".into(),
    };
    let edit = edit_for(&document, &action, 12);
    assert!(matches!(edit.target, OutlineEditTarget::Version { .. }));
    let OutlineChange::Freeze { name } = &edit.change else {
        panic!("a freeze action is one typed freeze change");
    };
    assert_eq!(name, "Edited outline 2");
    let mut accepted = document.clone();
    let changed = boardstudio_core::outline_edit::apply(&mut accepted, &edit).unwrap();
    let state = &accepted.board_outlines[0];
    assert_eq!(state.versions.len(), 2, "one operation creates one copy");
    let linked = state
        .versions
        .iter()
        .find(|version| version.id == "v1")
        .expect("the linked refinement is retained");
    assert_eq!(linked.ownership, OutlineOwnership::Linked);
    let frozen = state
        .versions
        .iter()
        .find(|version| version.id != "v1")
        .expect("the fixed copy exists");
    assert_eq!(frozen.ownership, OutlineOwnership::Fixed);
    assert_eq!(state.active_version_id.as_deref(), Some(frozen.id.as_str()));
    assert_eq!(
        changed,
        vec![BOARD.to_owned(), frozen.id.clone()],
        "the changed owners are the board and the created copy"
    );
}

#[test]
fn a_gap_focus_is_not_an_edit() {
    let (scope, token, revision, generation, board_id) = envelope();
    let action = OutlineAction::FocusGap {
        scope,
        token,
        revision,
        generation,
        board_id,
        context: version_context("v1"),
        gap_id: "gap-1".into(),
    };
    assert!(
        adapter::outline_edit(&snapshot(document()), &action, 10).is_none(),
        "focus stays presentation-only and creates no document history"
    );
}

#[test]
fn a_draft_connection_width_is_refused_by_core_rather_than_defaulted() {
    // The adapter supplies the board's bridge width; a non-positive width on the wire is
    // malformed and Core refuses it with no mutation.
    let edit = boardstudio_core::outline_edit::OutlineEdit {
        target: OutlineEditTarget::Board {
            board_id: BOARD.into(),
        },
        change: OutlineChange::AddConnection {
            connection: boardstudio_core::model::OutlineConnection {
                id: "outline-connection-1".into(),
                width: 0.0,
                points: vec![
                    boardstudio_core::model::OutlineControlPoint {
                        at: Vec2 { x: 0.0, y: 0.0 },
                        part_id: None,
                    },
                    boardstudio_core::model::OutlineControlPoint {
                        at: Vec2 { x: 1.0, y: 0.0 },
                        part_id: None,
                    },
                ],
            },
        },
        seed: 1,
        reviewed: None,
    };
    let document = generated_active();
    let error = boardstudio_core::outline_edit::evaluate(&document, &edit)
        .expect_err("a zero width is malformed");
    assert!(error.contains("positive finite"), "{error}");
    let operation = EditOperation::ApplyOutlineEdit {
        edit: Box::new(edit),
    };
    assert!(matches!(operation, EditOperation::ApplyOutlineEdit { .. }));
}
