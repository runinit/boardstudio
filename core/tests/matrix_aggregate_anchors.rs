//! Ticket 28: aggregate matrix anchors (average / intersection) through the
//! public Core routes. Aggregate anchors resolve N single-key parts to one
//! pose before the matrix Origin/Rotation and Follow apply, exactly as
//! upstream Ergogen resolves `aggregate` before `affect`.
use boardstudio_core::{CoreEngine, model::*};
use serde_json::{Value, json};

fn request(engine: &mut CoreEngine, input: Value) -> Value {
    serde_json::from_str(&engine.request(&input.to_string())).unwrap()
}

fn open(engine: &mut CoreEngine, id: &str, document: &Value) -> Value {
    request(engine, json!({"kind":"open","id":id,"document":document}))
}

fn document() -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "aggregate-anchors",
        "Aggregate anchors",
    ))
    .unwrap();
    document["definitions"] = json!([{
        "id": "def", "name": "Switch", "kind": "switch",
        "courtyard": [], "pads": []
    }]);
    document
}

fn board(document: &mut Value, part_ids: Value) {
    document["boards"] = json!([{
        "id": "board", "name": "Board", "outlineIds": [],
        "partIds": part_ids, "netIds": [], "thickness": 1.6
    }]);
    document["nets"] = json!([]);
    document["constraints"] = json!([]);
    document["layouts"] = json!([{
        "id": "layout", "name": "Layout", "boardId": "board",
        "matrixIds": ["main", "thumb"], "partIds": []
    }]);
}

fn key(document: &mut Value, id: &str, reference: &str, x: f64, y: f64, rotation: f64) {
    document["parts"].as_array_mut().unwrap().push(json!({
        "id": id, "definitionId": "def", "reference": reference,
        "pose": {"at": {"x": x, "y": y}, "rotation": rotation}, "side": "front"
    }));
}

fn main_matrix(document: &mut Value, cells: Value) {
    document["parts"] = json!([]);
    // Anchor resolution reads these part poses, so they must match the
    // matrix geometry exactly like materialized production parts do.
    key(document, "matrix/main/r0c0", "SW1", 0.0, 0.0, 0.0);
    key(document, "matrix/main/r0c1", "SW2", 19.0, 0.0, 0.0);
    key(document, "matrix/main/r0c2", "SW3", 38.0, 0.0, 0.0);
    document["matrices"] = json!([{
        "id": "main", "rows": 1, "columns": 3,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0},
        "definitionId": "def",
        "partIds": ["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"],
        "boardId": "board", "cells": cells
    }]);
}

fn thumb(document: &mut Value, aggregate: Value) {
    document["matrices"].as_array_mut().unwrap().push(json!({
        "id": "thumb", "rows": 1, "columns": 3,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": -19},
        "definitionId": "def", "partIds": [], "boardId": "board",
        "anchor": {
            "matrixId": "main", "row": 0, "column": 0,
            "follow": ["x", "y", "rotation"],
            "aggregate": aggregate
        }
    }));
}

fn scene_cell(reply: &Value, matrix: &str, row: u64, column: u64) -> (f64, f64, f64) {
    let scene = reply["scene"]["matrixScenes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scene| scene["matrixId"] == matrix)
        .unwrap_or_else(|| panic!("missing scene {matrix}"));
    let cell = scene["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["row"] == row && cell["column"] == column)
        .unwrap_or_else(|| panic!("missing cell ({row},{column}) in {scene}"));
    (
        cell["pose"]["at"]["x"].as_f64().unwrap(),
        cell["pose"]["at"]["y"].as_f64().unwrap(),
        cell["pose"]["rotation"].as_f64().unwrap(),
    )
}

fn assert_cell(reply: &Value, matrix: &str, row: u64, column: u64, x: f64, y: f64, rotation: f64) {
    let (ax, ay, ar) = scene_cell(reply, matrix, row, column);
    assert!(
        (ax - x).abs() < 1e-9 && (ay - y).abs() < 1e-9 && (ar - rotation).abs() < 1e-9,
        "cell {matrix}({row},{column}): expected ({x}, {y}, {rotation}), got ({ax}, {ay}, {ar})"
    );
}

#[test]
fn aggregate_average_anchor_opens_with_the_part_mean_pose() {
    let mut document = document();
    main_matrix(&mut document, json!([]));
    thumb(
        &mut document,
        json!({
            "method": "average",
            "parts": [
                {"matrixId": "main", "row": 0, "column": 0},
                {"matrixId": "main", "row": 0, "column": 2}
            ]
        }),
    );
    board(
        &mut document,
        json!(["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"]),
    );
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "average", &document);
    assert_eq!(reply["kind"], "scene", "{reply}");
    // Mean of (0,0) and (38,0) is (19,0); the relative origin hangs (0,-19)
    // below it, so the first thumb key lands on (19,-19) with rotation 0.
    assert_cell(&reply, "thumb", 0, 0, 19.0, -19.0, 0.0);
    assert_cell(&reply, "thumb", 0, 2, 57.0, -19.0, 0.0);
}

#[test]
fn aggregate_intersect_anchor_opens_with_the_heading_crossing_pose() {
    let mut document = document();
    // Second key carries key rotation -90 and an offset to (10,10): its
    // heading axis runs horizontal through y=10 while the first key's axis
    // runs vertical through x=0, crossing at (0,10) like upstream's
    // `o`/`ten` intersect vector.
    main_matrix(
        &mut document,
        json!([
            {"row": 0, "column": 1, "enabled": true,
             "offset": {"x": -9.0, "y": 10.0}, "rotation": -90.0}
        ]),
    );
    thumb(
        &mut document,
        json!({
            "method": "intersect",
            "parts": [
                {"matrixId": "main", "row": 0, "column": 0},
                {"matrixId": "main", "row": 0, "column": 1}
            ]
        }),
    );
    board(
        &mut document,
        json!(["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"]),
    );
    // The moved key's stored part pose must follow its cell geometry, or
    // resolution would read the stale pose instead of ((10,10),-90).
    document["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "matrix/main/r0c1")
        .unwrap()["pose"] = json!({"at": {"x": 10.0, "y": 10.0}, "rotation": -90.0});
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "intersect", &document);
    assert_eq!(reply["kind"], "scene", "{reply}");
    // The heading axes cross at (0,10) with the rotation forced to 0; the
    // relative origin hangs (0,-19) below it.
    assert_cell(&reply, "thumb", 0, 0, 0.0, -9.0, 0.0);
}

fn typed_scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

fn typed_open(doc: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, doc) = typed_scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    (engine, scene, doc)
}

fn typed_edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    typed_scene(engine.handle(CoreRequest::Edit {
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

fn typed_error(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> String {
    match engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected error: {other:?}"),
    }
}

fn typed_snapshot(engine: &mut CoreEngine) -> ProjectDoc {
    typed_scene(engine.handle(CoreRequest::Snapshot { id: "snap".into() })).1
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

fn typed_key(id: &str, x: f64, y: f64) -> Part {
    Part {
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn typed_main(cells: Vec<MatrixCell>) -> Matrix {
    Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells,
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
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
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

fn at(doc: &ProjectDoc, id: &str) -> Vec2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
        .at
}

fn rotation_of(doc: &ProjectDoc, id: &str) -> f64 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
        .rotation
}

#[test]
fn aggregate_average_follows_part_moves_through_the_mean() {
    let (mut engine, _, doc) = typed_open(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
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
        id: "thumb".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 { x: 0.0, y: -19.0 },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: Some(average_anchor(vec![part("main", 0, 0), part("main", 0, 2)])),
        cells: vec![],
    };
    let (_, ordered_scene, _) = typed_open(typed_doc(
        typed_main(vec![]),
        vec![thumb.clone()],
        vec!["main", "thumb"],
    ));
    // The dependant is listed before its parts here; evaluation still visits
    // every part target first.
    let reversed = {
        let mut doc = typed_doc(typed_main(vec![]), vec![thumb], vec!["main", "thumb"]);
        let thumb_index = doc
            .matrices
            .iter()
            .position(|matrix| matrix.id == "thumb")
            .unwrap();
        let thumb = doc.matrices.remove(thumb_index);
        doc.matrices.insert(0, thumb);
        doc
    };
    let (_, reversed_scene, _) = typed_open(reversed);
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
        id: "thumb".into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: Some(average_anchor(vec![part("main", 0, 0)])),
        cells: vec![],
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
    let (mut engine, _, doc) = typed_open(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
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
    assert_eq!(typed_snapshot(&mut engine), doc);
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
    assert_eq!(typed_snapshot(&mut engine), doc);
}

#[test]
fn aggregate_parallel_intersect_refuses_without_mutation() {
    let (mut engine, _, doc) = typed_open(typed_doc(typed_main(vec![]), vec![], vec!["main"]));
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
    assert_eq!(typed_snapshot(&mut engine), doc);
}

#[test]
fn aggregate_follow_filters_the_completed_candidate() {
    // The second part carries key rotation 90, so the aggregate pose is
    // ((0+38)/2, 0, (0+90)/2) = (19,0,45): the naive rotation mean.
    let main = typed_main(vec![MatrixCell {
        shift: None,
        bend: None,
        deleted: false,
        row: 0,
        column: 2,
        enabled: true,
        definition_id: None,
        variant: None,
        offset: None,
        rotation: Some(90.0),
        assemblies: vec![],
        assemblies_local: None,
    }]);
    let (mut engine, _, doc) = typed_open(typed_doc(main, vec![], vec!["main"]));
    let mut anchor = average_anchor(vec![part("main", 0, 0), part("main", 0, 2)]);
    anchor.follow = Follow {
        x: true,
        y: false,
        rotation: false,
    };
    let (_, doc) = typed_edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor,
        },
    );
    // The completed candidate turns the (0,-19) relative origin by the mean
    // 45-degree heading: x = 19 + 19*sin(45). Follow keeps only X, so the
    // unfollowed Y and rotation read world zero — the same shape as the
    // upstream partial-x-only observation (14.330127,0,0).
    let expected = 19.0 + 19.0 * std::f64::consts::FRAC_1_SQRT_2;
    let actual = at(&doc, &member("thumb", 0, 0));
    assert!(
        (actual.x - expected).abs() < 1e-9,
        "expected x {expected}, got {}",
        actual.x
    );
    assert_eq!(actual.y, 0.0);
    assert_eq!(rotation_of(&doc, &member("thumb", 0, 0)), 0.0);
}

#[test]
fn aggregate_unknown_method_is_refused() {
    let mut document = document();
    main_matrix(&mut document, json!([]));
    thumb(
        &mut document,
        json!({
            "method": "average",
            "parts": [{"matrixId": "main", "row": 0, "column": 0}]
        }),
    );
    board(
        &mut document,
        json!(["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"]),
    );
    document["matrices"][1]["anchor"]["aggregate"]["method"] = json!("median");
    let mut engine = CoreEngine::new();
    let reply = open(&mut engine, "median", &document);
    assert_eq!(reply["kind"], "error", "{reply}");
    assert!(
        reply["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown variant"),
        "unknown methods must be refused at admission, got {reply}"
    );
}
