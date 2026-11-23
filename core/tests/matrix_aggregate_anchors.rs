//! Ticket 28: aggregate matrix anchors (average / intersection) through the
//! public Core routes. Aggregate anchors resolve N single-key parts to one
//! pose before the matrix Origin/Rotation and Follow apply, exactly as
//! upstream Ergogen resolves `aggregate` before `affect`.
use crate::support::fixture::{
    self, key_part as typed_key, part_at as at, scene_and_document as typed_scene,
    wire_open as open,
};
use crate::support::session::{Session, builders::assert_typed_cell, error, wire_snapshot_doc};
use boardstudio_core::{CoreEngine, model::*};
use serde_json::json;

/// A dependant matrix hanging a (0,-19) relative origin below the anchor mean.
fn thumb_matrix(anchor: MatrixAnchor) -> Matrix {
    Matrix {
        origin: Vec2 { x: 0.0, y: -19.0 },
        board_id: Some("board".into()),
        anchor: Some(anchor),
        ..fixture::matrix("thumb", 1, 3)
    }
}

#[test]
fn aggregate_average_anchor_opens_with_the_part_mean_pose() {
    let thumb = thumb_matrix(average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]));
    let (_, scene, _) = Session::opened(typed_doc(
        typed_main(vec![]),
        vec![thumb],
        vec!["main", "thumb"],
    ));
    // Mean of (0,0) and (38,0) is (19,0); the relative origin hangs (0,-19)
    // below it, so the first thumb key lands on (19,-19) with rotation 0.
    assert_typed_cell(&scene, "thumb", 0, 0, 19.0, -19.0, 0.0);
    assert_typed_cell(&scene, "thumb", 0, 2, 57.0, -19.0, 0.0);
}

#[test]
fn aggregate_intersect_anchor_opens_with_the_heading_crossing_pose() {
    // Second key carries key rotation -90 and an offset to (10,10): its
    // heading axis runs horizontal through y=10 while the first key's axis
    // runs vertical through x=0, crossing at (0,10) like upstream's
    // `o`/`ten` intersect vector.
    let main = typed_main(vec![MatrixCell {
        offset: Some(Vec2 { x: -9.0, y: 10.0 }),
        rotation: Some(-90.0),
        ..fixture::cell(0, 1)
    }]);
    let thumb = thumb_matrix(intersect_anchor(vec![
        part("main", 0, 0),
        part("main", 0, 1),
    ]));
    let mut doc = typed_doc(main, vec![thumb], vec!["main", "thumb"]);
    // The moved key's stored part pose must follow its cell geometry, or
    // resolution would read the stale pose instead of ((10,10),-90).
    doc.parts
        .iter_mut()
        .find(|part| part.id == "matrix/main/r0c1")
        .unwrap()
        .pose = Pose2 {
        at: Vec2 { x: 10.0, y: 10.0 },
        rotation: -90.0,
    };
    let (_, scene, _) = Session::opened(doc);
    // The heading axes cross at (0,10) with the rotation forced to 0; the
    // relative origin hangs (0,-19) below it.
    assert_typed_cell(&scene, "thumb", 0, 0, 0.0, -9.0, 0.0);
}

fn typed_edit(
    session: &mut Session,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    typed_scene(session.commit(doc.revision, operation))
}

fn typed_error(session: &mut Session, doc: &ProjectDoc, operation: EditOperation) -> String {
    error(&session.commit(doc.revision, operation)).to_owned()
}

fn all() -> Follow {
    Follow {
        x: true,
        y: true,
        rotation: true,
    }
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn typed_main(cells: Vec<MatrixCell>) -> Matrix {
    Matrix {
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        cells,
        ..fixture::matrix("main", 1, 3)
    }
}

fn typed_doc(main: Matrix, extra: Vec<Matrix>, order: Vec<&str>) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("aggregate-anchors", "Aggregate anchors");
    doc.definitions = serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap();
    doc.parts = vec![
        typed_key(&member("main", 0, 0), 0.0, 0.0),
        typed_key(&member("main", 0, 1), 19.0, 0.0),
        typed_key(&member("main", 0, 2), 38.0, 0.0),
    ];
    doc.boards.push(Board {
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        ..fixture::board("board", "Board")
    });
    doc.matrices.push(main);
    doc.matrices.extend(extra);
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: order.iter().map(|id| id.to_string()).collect(),
        part_ids: vec![],
        mirror_link: None,
    });
    doc
}

fn part(matrix: &str, row: u32, column: u32) -> AnchorPart {
    AnchorPart {
        matrix_id: matrix.into(),
        row,
        column,
    }
}

/// Aggregate anchors carry inert single-key fields (empty identity); every
/// consumer resolves through `targets()`, so the dummies are never read.
fn average_anchor(parts: Vec<AnchorPart>) -> MatrixAnchor {
    MatrixAnchor {
        matrix_id: String::new(),
        row: 0,
        column: 0,
        follow: all(),
        aggregate: Some(AggregateAnchor {
            method: AggregateMethod::Average,
            parts,
        }),
    }
}

#[test]
fn aggregate_average_follows_part_moves_through_the_mean() {
    let (mut engine, _, doc) = Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
    let (_, doc) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]),
        },
    );
    // Mean of (0,0) and (38,0) with the (0,-19) relative origin below it.
    assert_eq!(at(&doc, &member("thumb", 0, 0)), Vec2 { x: 19.0, y: -19.0 });
    // Moving one part key moves the dependant through the new mean:
    // ((0,0) + (43,7)) / 2 = (21.5,3.5), hanging (0,-19) below.
    let (_, doc) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 43.0, y: 7.0 },
            }],
        },
    );
    assert_eq!(at(&doc, &member("main", 0, 2)), Vec2 { x: 43.0, y: 7.0 });
    assert_eq!(at(&doc, &member("thumb", 0, 0)), Vec2 { x: 21.5, y: -15.5 });
}

#[test]
fn aggregate_evaluation_ignores_document_list_order() {
    let thumb = Matrix {
        origin: Vec2 { x: 0.0, y: -19.0 },
        board_id: Some("board".into()),
        anchor: Some(average_anchor(vec![part("main", 0, 0), part("main", 0, 2)])),
        ..fixture::matrix("thumb", 1, 3)
    };
    let (_, ordered_scene, _) = Session::opened(typed_doc(
        typed_main(vec![]),
        vec![thumb.clone()],
        vec!["main", "thumb"],
    ));
    // A genuinely different document order end to end: the dependant leads
    // both the matrix list and the layout member list, as if authored first.
    let reversed = typed_doc(typed_main(vec![]), vec![thumb], vec!["thumb", "main"]);
    let mut doc = reversed;
    let thumb_index = doc
        .matrices
        .iter()
        .position(|matrix| matrix.id == "thumb")
        .unwrap();
    let thumb = doc.matrices.remove(thumb_index);
    doc.matrices.insert(0, thumb);
    let (_, reversed_scene, _) = Session::opened(doc);
    let order_of = |scene: &SceneDelta| {
        scene
            .matrix_scenes
            .iter()
            .map(|scene| scene.matrix_id.clone())
            .collect::<Vec<_>>()
    };
    // The premise the old list-only permutation never established:
    // evaluation follows dependency order, not document order, so the
    // dependant still evaluates after its parts in both documents.
    assert_eq!(order_of(&ordered_scene), vec!["main", "thumb"]);
    assert_eq!(order_of(&reversed_scene), vec!["main", "thumb"]);
    let cells = |scene: &SceneDelta| {
        scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == "thumb")
            .expect("thumb scene")
            .cells
            .iter()
            .map(|cell| (cell.pose.at.x, cell.pose.at.y, cell.pose.rotation))
            .collect::<Vec<_>>()
    };
    assert_eq!(cells(&ordered_scene), cells(&reversed_scene));
    assert_eq!(cells(&ordered_scene)[0], (19.0, -19.0, 0.0));
}

#[test]
fn aggregate_two_cycle_refuses_naming_the_matrix() {
    let mut doc = typed_doc(typed_main(vec![]), vec![], vec!["main", "thumb"]);
    doc.matrices.push(Matrix {
        board_id: Some("board".into()),
        anchor: Some(average_anchor(vec![part("main", 0, 0)])),
        ..fixture::matrix("thumb", 1, 1)
    });
    // Close the cycle through an aggregate part: main follows thumb.
    doc.matrices
        .iter_mut()
        .find(|matrix| matrix.id == "main")
        .unwrap()
        .anchor = Some(average_anchor(vec![part("thumb", 0, 0)]));
    let mut engine = CoreEngine::new();
    match engine.handle(CoreRequest::Open {
        id: "cycle".into(),
        document: doc,
    }) {
        CoreReply::Error { message, .. } => assert!(
            message.contains("Matrix anchor cycle at"),
            "cycle must name the matrix, got {message:?}"
        ),
        other => panic!("expected a cycle refusal, got: {other:?}"),
    }
}

#[test]
fn aggregate_self_part_and_two_cycle_edits_refuse_atomically() {
    let (mut engine, _, doc) = Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
    let (_, doc) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]),
        },
    );
    // A part naming the owner's own matrix refuses as a self-anchor.
    let message = typed_error(
        &mut engine,
        &doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: average_anchor(vec![part("main", 0, 0), part("thumb", 0, 0)]),
        },
    );
    assert!(
        message.contains("cannot anchor to itself"),
        "unexpected message: {message:?}"
    );
    assert_eq!(engine.snapshot(), doc);
    // Closing a 2-cycle through an aggregate part refuses naming the matrix.
    let message = typed_error(
        &mut engine,
        &doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "main".into(),
            anchor: average_anchor(vec![part("thumb", 0, 0)]),
        },
    );
    assert!(
        message.contains("Matrix anchor cycle at main"),
        "unexpected message: {message:?}"
    );
    assert_eq!(engine.snapshot(), doc);
}

#[test]
fn aggregate_parallel_intersect_refuses_without_mutation() {
    let (mut engine, _, doc) = Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
    let (_, doc) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]),
        },
    );
    // Both parts stand upright: their heading axes are parallel verticals.
    let mut parallel = average_anchor(vec![part("main", 0, 0), part("main", 0, 1)]);
    parallel.aggregate.as_mut().unwrap().method = AggregateMethod::Intersect;
    let message = typed_error(
        &mut engine,
        &doc,
        EditOperation::SetMatrixAnchor {
            matrix_id: "thumb".into(),
            anchor: parallel,
        },
    );
    assert!(
        message.contains("do not intersect"),
        "unexpected message: {message:?}"
    );
    assert_eq!(engine.snapshot(), doc);
}

#[test]
fn aggregate_follow_filters_the_completed_candidate() {
    // Upstream partial-x-only golden (`base_a_r0` at (10,20,30°) with
    // shift [5,0], rotate 10 and affect 'x' resolves to (14.330127,0,0)):
    // the aggregate mean of ((0,0),0°) and ((20,40),60°) is ((10,20),30°),
    // the (5,0) relative origin turned 10° completes the candidate
    // (14.330127,22.5,40), and follow-x-only keeps (14.330127,0,0).
    let main = typed_main(vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column: 1,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: Some(Vec2 { x: 1.0, y: 40.0 }),
        rotation: Some(60.0),
        assemblies: vec![],
        assemblies_local: None,
    }]);
    let mut anchor = average_anchor(vec![part("main", 0, 0), part("main", 0, 1)]);
    anchor.follow = Follow {
        x: true,
        y: false,
        rotation: false,
    };
    let thumb = Matrix {
        origin: Vec2 { x: 5.0, y: 0.0 },
        board_id: Some("board".into()),
        rotation: Some(10.0),
        anchor: Some(anchor),
        ..fixture::matrix("thumb", 1, 1)
    };
    let mut doc = typed_doc(main, vec![thumb], vec!["main", "thumb"]);
    // The second part pose must match its cell geometry exactly, like
    // materialized production parts do.
    doc.parts
        .iter_mut()
        .find(|part| part.id == member("main", 0, 1))
        .unwrap()
        .pose = Pose2 {
        at: Vec2 { x: 20.0, y: 40.0 },
        rotation: 60.0,
    };
    let (_, scene, _) = Session::opened(doc);
    let cell = scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == "thumb")
        .expect("thumb scene")
        .cells
        .first()
        .expect("thumb cell");
    // The golden literal is upstream-observed, so the x comparison uses the
    // upstream 1e-6 precedent (spec §10); unfollowed Y and rotation are
    // exact world zeros from follow filtering, not computed values.
    assert!(
        (cell.pose.at.x - 14.330127).abs() < 1e-6,
        "expected upstream golden x 14.330127, got {}",
        cell.pose.at.x
    );
    assert_eq!(cell.pose.at.y, 0.0);
    assert_eq!(cell.pose.rotation, 0.0);
}

#[test]
fn aggregate_unknown_method_is_refused() {
    let thumb = thumb_matrix(average_anchor(vec![part("main", 0, 0)]));
    let mut document = serde_json::to_value(typed_doc(
        typed_main(vec![]),
        vec![thumb],
        vec!["main", "thumb"],
    ))
    .unwrap();
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "baseline", &document);
    assert_eq!(reply["kind"], "scene", "{reply}");
    let accepted = wire_snapshot_doc(&mut engine);
    document["matrices"][1]["anchor"]["aggregate"]["method"] = json!("median");
    let reply = open(&mut engine, "median", &document);
    assert_eq!(reply["kind"], "error", "{reply}");
    assert!(
        reply["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown variant"),
        "unknown methods must be refused at admission, got {reply}"
    );
    assert_eq!(wire_snapshot_doc(&mut engine), accepted);
}

#[test]
fn aggregate_anchor_refuses_a_real_single_key_alongside_aggregate() {
    // Upstream throws on ref+aggregate (anchor.js:96-98): a document naming a
    // real single key AND an aggregate arm must be refused at admission.
    let thumb = thumb_matrix(average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]));
    // A real single key next to the aggregate arm: both arms are populated,
    // so the union grammar must refuse the document. The refusal lives in
    // anchor deserialization, so the offending document goes through the
    // wire open path while the valid baseline uses the typed path.
    let (mut engine, _, accepted) = Session::opened(typed_doc(
        typed_main(vec![]),
        vec![thumb],
        vec!["main", "thumb"],
    ));
    let mut both = serde_json::to_value(&accepted).unwrap();
    let anchor = &mut both["matrices"][1]["anchor"];
    anchor["matrixId"] = json!("main");
    anchor["row"] = json!(0);
    anchor["column"] = json!(2);
    let mut wire = CoreEngine::new();
    let reply = open(&mut wire, "both-arms", &both);
    assert_eq!(reply["kind"], "error", "{reply}");
    assert!(
        reply["message"]
            .as_str()
            .unwrap_or_default()
            .contains("aggregate"),
        "both-arms refusal must name the aggregate arm, got {reply}"
    );
    assert_eq!(engine.snapshot(), accepted);
}

fn intersect_anchor(parts: Vec<AnchorPart>) -> MatrixAnchor {
    MatrixAnchor {
        matrix_id: String::new(),
        row: 0,
        column: 0,
        follow: all(),
        aggregate: Some(AggregateAnchor {
            method: AggregateMethod::Intersect,
            parts,
        }),
    }
}

fn anchor_of(doc: &ProjectDoc, id: &str) -> Option<MatrixAnchor> {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .and_then(|matrix| matrix.anchor.clone())
}

/// A refused edit pushes no history entry: nothing remains to undo.
fn assert_no_history_entry(engine: &mut Session) {
    assert!(
        matches!(
            engine.undo(),
            CoreReply::Error { message, .. } if message.contains("History is empty")
        ),
        "refused edit must leave no history entry"
    );
}

fn added_thumb(engine: &mut Session, doc: &ProjectDoc) -> (SceneDelta, ProjectDoc) {
    typed_edit(
        engine,
        doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]),
        },
    )
}

#[test]
fn aggregate_add_refuses_unknown_matrix_and_out_of_range_key_atomically() {
    for (target, explanation) in [
        (part("ghost", 0, 0), "anchor target ghost is missing"),
        (part("main", 0, 9), "anchor key matrix/main/r0c9 is missing"),
    ] {
        let (mut engine, _, doc) =
            Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
        let message = typed_error(
            &mut engine,
            &doc,
            EditOperation::AddAnchoredMatrix {
                id: "thumb".into(),
                anchor: average_anchor(vec![part("main", 0, 0), target]),
            },
        );
        assert!(
            message.contains(explanation),
            "unexpected message: {message:?}"
        );
        assert_eq!(engine.snapshot(), doc);
        assert_no_history_entry(&mut engine);
    }
}

#[test]
fn aggregate_set_anchor_refuses_invalid_cardinality_atomically() {
    for (anchor, explanation) in [
        (average_anchor(vec![]), "needs at least one part"),
        (
            intersect_anchor(vec![part("main", 0, 0)]),
            "intersect aggregate needs exactly two parts, got 1",
        ),
        (
            intersect_anchor((0..3).map(|column| part("main", 0, column)).collect()),
            "intersect aggregate needs exactly two parts, got 3",
        ),
    ] {
        let (mut engine, _, opened) =
            Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
        let (_, doc) = added_thumb(&mut engine, &opened);
        let message = typed_error(
            &mut engine,
            &doc,
            EditOperation::SetMatrixAnchor {
                matrix_id: "thumb".into(),
                anchor,
            },
        );
        assert!(
            message.contains(explanation),
            "unexpected message: {message:?}"
        );
        assert_eq!(engine.snapshot(), doc);
        // One Undo returns to the opened document, and nothing remains to undo.
        let undone = fixture::document(engine.undo());
        assert_eq!(undone.matrices, opened.matrices);
        assert_eq!(undone.layouts, opened.layouts);
        assert_eq!(undone.parts, opened.parts);
        assert_no_history_entry(&mut engine);
    }
}

#[test]
fn aggregate_compaction_remaps_every_part_onto_surviving_keys() {
    let (mut engine, _, doc) = Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
    let (_, doc) = added_thumb(&mut engine, &doc);
    // Removing the middle column strands no part: (0,0) stays, (0,2) follows
    // its physical key to (0,1). Nothing detaches.
    let (_, compacted) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 1,
        },
    );
    assert_eq!(
        anchor_of(&compacted, "thumb"),
        Some(average_anchor(vec![part("main", 0, 0), part("main", 0, 1)]))
    );
    // Mean of (0,0) and (19,0) is (9.5,0); the (0,-19) relative origin hangs
    // below it.
    assert_eq!(
        at(&compacted, &member("thumb", 0, 0)),
        Vec2 { x: 9.5, y: -19.0 }
    );
    assert_eq!(
        at(&compacted, &member("thumb", 0, 2)),
        Vec2 { x: 47.5, y: -19.0 }
    );
}

#[test]
fn aggregate_removing_a_part_slot_detaches_keeping_the_world_frame() {
    let (mut engine, _, doc) = Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
    let (_, doc) = added_thumb(&mut engine, &doc);
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    // Column 2 carries part (0,2): removal past any part detaches the whole
    // dependant, keeping its world frame like the single-key slot removal.
    let (_, detached) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::RemoveMatrixColumn {
            matrix_id: "main".into(),
            column: 2,
        },
    );
    assert_eq!(anchor_of(&detached, "thumb"), None);
    let thumb = detached
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix");
    assert_eq!(thumb.origin, Vec2 { x: 19.0, y: -19.0 });
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(
                detached
                    .parts
                    .iter()
                    .find(|part| &part.id == id)
                    .unwrap()
                    .pose,
                *pose,
                "{id}"
            );
        }
    }
    // The detach rides the same edit: one Undo restores the aggregate anchor.
    assert_eq!(
        anchor_of(&fixture::document(engine.undo()), "thumb"),
        anchor_of(&doc, "thumb")
    );
}

#[test]
fn aggregate_shrink_past_a_part_detaches_keeping_the_world_frame() {
    let (mut engine, _, doc) = Session::opened(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
    let (_, doc) = added_thumb(&mut engine, &doc);
    let poses_before: Vec<(String, Pose2)> = doc
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut shrunk = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "main")
        .expect("main matrix")
        .clone();
    shrunk.columns = 2;
    let (_, detached) = typed_edit(&mut engine, &doc, fixture::set_matrix(shrunk));
    // Part (0,2) leaves the grid, so the dependant detaches with its frame:
    // the (19,0) mean with the (0,-19) relative origin below it.
    assert_eq!(anchor_of(&detached, "thumb"), None);
    let thumb = detached
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .expect("thumb matrix");
    assert_eq!(thumb.origin, Vec2 { x: 19.0, y: -19.0 });
    for (id, pose) in &poses_before {
        if id.starts_with("matrix/thumb/") {
            assert_eq!(
                detached
                    .parts
                    .iter()
                    .find(|part| &part.id == id)
                    .unwrap()
                    .pose,
                *pose,
                "{id}"
            );
        }
    }
    // The detach rides the same edit: one Undo restores the aggregate anchor.
    assert_eq!(
        anchor_of(&fixture::document(engine.undo()), "thumb"),
        anchor_of(&doc, "thumb")
    );
}

#[test]
fn aggregate_union_keeps_saved_single_key_anchors_byte_identical() {
    // Saved flat single-key anchors predate the aggregate arm and must keep
    // opening byte-identical under the union grammar.
    let saved = r#"{"matrixId":"main","row":0,"column":2,"follow":["x","y","rotation"]}"#;
    let anchor: MatrixAnchor = serde_json::from_str(saved).unwrap();
    assert_eq!(anchor.matrix_id, "main");
    assert_eq!((anchor.row, anchor.column), (0, 2));
    assert!(anchor.aggregate.is_none());
    assert_eq!(
        serde_json::to_string(&anchor).unwrap(),
        saved,
        "single-arm serialization must stay byte-identical"
    );

    // And end to end: a document carrying that saved anchor opens, and the
    // stored anchor still serializes back to the saved bytes.
    let thumb = Matrix {
        origin: Vec2 { x: 0.0, y: -19.0 },
        board_id: Some("board".into()),
        anchor: Some(serde_json::from_str(saved).unwrap()),
        ..fixture::matrix("thumb", 1, 3)
    };
    let (mut engine, _, _) = Session::opened(typed_doc(
        typed_main(vec![]),
        vec![thumb],
        vec!["main", "thumb"],
    ));
    let stored = engine.snapshot();
    let stored_anchor = stored
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .and_then(|matrix| matrix.anchor.as_ref())
        .expect("stored thumb anchor");
    assert_eq!(serde_json::to_string(stored_anchor).unwrap(), saved);
}
