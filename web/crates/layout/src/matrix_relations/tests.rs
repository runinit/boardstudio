//! Relations projection tests: scenes come from a real `CoreEngine`, never hand-built.

use super::*;
use boardstudio_core::CoreEngine;
use boardstudio_core::model::{ConstraintEndpoint, ConstraintRelation};
use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditPhase, SceneDelta};
use serde_json::json;

const MATRIX: &str = "main";

struct World {
    document: ProjectDoc,
    scene: MatrixScene,
    scenes: Vec<MatrixScene>,
}

fn definition(id: &str, kind: &str) -> serde_json::Value {
    json!({
        "id": id, "name": id, "kind": kind,
        "courtyard": [
            { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
            { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
        ],
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
    })
}

fn base() -> ProjectDoc {
    let mut document = serde_json::to_value(ProjectDoc::empty("relations", "Relations")).unwrap();
    document["definitions"] = json!([
        definition("switch", "switch"),
        definition("diode", "passive")
    ]);
    document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
        "partIds": [], "netIds": [], "thickness": 1.6 }]);
    serde_json::from_value(document).unwrap()
}

fn matrix(rows: u32, columns: u32, extra: serde_json::Value) -> serde_json::Value {
    let mut matrix = json!({
        "id": MATRIX, "rows": rows, "columns": columns,
        "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
        "definitionId": "switch", "boardId": "board", "partIds": []
    });
    for (key, value) in extra.as_object().into_iter().flatten() {
        matrix[key] = value.clone();
    }
    matrix
}

fn scene_of(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("{other:?}"),
    }
}

fn apply(
    engine: &mut CoreEngine,
    revision: u64,
    operation: serde_json::Value,
) -> (SceneDelta, ProjectDoc) {
    scene_of(engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: serde_json::from_value(operation).expect("the operation parses"),
        },
    }))
}

fn world_of(delta: SceneDelta, document: ProjectDoc, matrix_id: &str) -> World {
    let scene = delta
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)
        .expect("the matrix has a scene")
        .clone();
    World {
        document,
        scene,
        scenes: delta.matrix_scenes,
    }
}

fn world(matrix_json: serde_json::Value) -> World {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (delta, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "set-matrix", "matrix": matrix_json }),
    );
    world_of(delta, document, MATRIX)
}

fn reopen(document: ProjectDoc, matrix_id: &str) -> World {
    let mut engine = CoreEngine::new();
    let (scene, document) = scene_of(engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document,
    }));
    world_of(scene, document, matrix_id)
}

fn relations(world: &World, context: TreeContext) -> Option<MatrixRelations> {
    matrix_relations(&world.document, &world.scene, &context, &world.scenes)
}

fn mirrored_pair_with_exception(kind: &str, state: &str) -> (World, World) {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (scene, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "create-mirrored-pair",
            "matrix": matrix(1, 1, json!({ "id": "left-matrix",
                "origin": { "x": -20.0, "y": 5.0 }, "mirror": "x" })),
            "left": { "id": "left-layout", "name": "Left half", "boardId": "board",
                      "matrixIds": ["left-matrix"], "partIds": [] },
            "right": { "id": "right-layout", "name": "Right half", "boardId": "board",
                       "matrixIds": ["right-matrix"], "partIds": [],
                       "mirrorLink": { "sourceId": "left-layout", "axisX": 10.0 } } }),
    );
    let mut mirror: serde_json::Value = serde_json::to_value(
        document
            .layouts
            .iter()
            .find(|layout| layout.id == "right-layout")
            .expect("mirror layout"),
    )
    .unwrap();
    mirror["mirrorLink"]["exceptions"] = json!([
        {"kind": "matrix", "mirrorId": "right-matrix", "state": state},
        {"kind": kind, "mirrorId": "matrix/right-matrix/r0c0", "state": state},
    ]);
    let (scene, document) = apply(
        &mut engine,
        scene.revision,
        json!({ "kind": "set-layout", "layout": mirror }),
    );
    (
        world_of(scene.clone(), document.clone(), "left-matrix"),
        world_of(scene, document, "right-matrix"),
    )
}

#[test]
fn a_local_overlay_explains_pose_pinning_on_its_key_and_matrix() {
    let (_, right) = mirrored_pair_with_exception("part", "local");

    let key = relations(&right, key_in("right-matrix", 0, 0)).unwrap();
    assert_eq!(
        value(&key.structure, "Local key"),
        "holds its final pose; carried fields stay shared"
    );

    let matrix = relations(&right, matrix_in("right-matrix")).unwrap();
    assert_eq!(
        value(&matrix.structure, "Local matrix"),
        "keeps its own dimensions and layout; its anchor stays detached"
    );
}

#[test]
fn an_omitted_key_keeps_a_logical_pose_without_resurrecting_parts() {
    let (_, right) = mirrored_pair_with_exception("part", "omitted");

    let key = relations(&right, key_in("right-matrix", 0, 0))
        .expect("logical inspection still projects an Omitted key");
    assert_eq!(
        value(&key.structure, "Omitted key"),
        "live logical pose only; inherited parts stay suppressed. Clear the omission in the Key inspector to rejoin."
    );
    let suppressed = "matrix/right-matrix/r0c0";
    let names_suppressed = |ids: &[String]| ids.iter().any(|id| id == suppressed);
    assert!(
        key.structure
            .iter()
            .all(|item| !names_suppressed(&item.highlight)),
        "the suppressed part is never highlighted"
    );
    assert!(
        key.departures
            .iter()
            .all(|departure| !names_suppressed(&departure.highlight)),
        "the suppressed part is never highlighted"
    );

    let matrix = relations(&right, matrix_in("right-matrix")).unwrap();
    assert_eq!(
        value(&matrix.structure, "Omitted matrix"),
        "live logical poses only; inherited parts stay suppressed"
    );
}

fn anchored_pair() -> (World, World) {
    let mut engine = CoreEngine::new();
    let main = matrix(2, 2, json!({}));
    let thumb = matrix(
        1,
        2,
        json!({"id": "thumb",
               "anchor": {"matrixId": "main", "row": 1, "column": 1,
                          "follow": ["x", "y"]}}),
    );
    let mut document = serde_json::to_value(base()).unwrap();
    document["matrices"] = json!([main, thumb]);
    document["layouts"] = json!([{ "id": "layout", "name": "Layout",
        "boardId": "board", "matrixIds": ["main", "thumb"], "partIds": [] }]);
    let (scene, document) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(document).unwrap(),
    }));
    (
        world_of(scene.clone(), document.clone(), MATRIX),
        world_of(scene, document, "thumb"),
    )
}

#[test]
fn following_a_local_anchor_key_says_it_holds_its_pose() {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (scene, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "create-mirrored-pair",
            "matrix": matrix(1, 1, json!({ "id": "left-matrix",
                "origin": { "x": -20.0, "y": 5.0 }, "mirror": "x" })),
            "left": { "id": "left-layout", "name": "Left half", "boardId": "board",
                      "matrixIds": ["left-matrix"], "partIds": [] },
            "right": { "id": "right-layout", "name": "Right half", "boardId": "board",
                       "matrixIds": ["right-matrix"], "partIds": [],
                       "mirrorLink": { "sourceId": "left-layout", "axisX": 10.0 } } }),
    );
    let mut mirror: serde_json::Value = serde_json::to_value(
        document
            .layouts
            .iter()
            .find(|layout| layout.id == "right-layout")
            .expect("mirror layout"),
    )
    .unwrap();
    mirror["mirrorLink"]["exceptions"] = json!([
        {"kind": "part", "mirrorId": "matrix/right-matrix/r0c0", "state": "local"},
    ]);
    let (scene, _) = apply(
        &mut engine,
        scene.revision,
        json!({ "kind": "set-layout", "layout": mirror }),
    );
    let (scene, document) = apply(
        &mut engine,
        scene.revision,
        json!({ "kind": "add-anchored-matrix", "id": "follower",
                "anchor": {"matrixId": "right-matrix", "row": 0, "column": 0,
                           "follow": ["x", "y", "rotation"]} }),
    );
    let follower = world_of(scene, document, "follower");
    let relations = relations(&follower, matrix_in("follower")).unwrap();
    let item = relations
        .structure
        .iter()
        .find(|item| item.label == "Follows")
        .expect("the follower names its Local anchor key");
    assert_eq!(
        item.value,
        "row 1, column 1 of matrix right-matrix (follows X, Y, rotation); \
         the anchor key is Local and holds its final pose"
    );
}

#[test]
fn anchored_matrices_name_their_follow_key_subset_and_dependants() {
    let (main, thumb) = anchored_pair();
    let thumb_matrix = TreeContext::Matrix {
        matrix_id: "thumb".into(),
    };
    let follows = relations(&thumb, thumb_matrix).unwrap();
    let item = follows
        .structure
        .iter()
        .find(|item| item.label == "Follows")
        .expect("an anchored matrix follows its anchor key");
    assert_eq!(item.value, "row 2, column 2 of matrix main (follows X, Y)");
    assert_eq!(
        item.target,
        Some(TreeContext::Key {
            matrix_id: MATRIX.into(),
            row: 1,
            column: 1,
        })
    );

    let main_matrix = TreeContext::Matrix {
        matrix_id: MATRIX.into(),
    };
    let followed = relations(&main, main_matrix).unwrap();
    let item = followed
        .structure
        .iter()
        .find(|item| item.label == "Followed by")
        .expect("an anchor matrix names its dependants");
    assert_eq!(item.value, "Matrix thumb");
    assert_eq!(
        item.target,
        Some(TreeContext::Matrix {
            matrix_id: "thumb".into(),
        })
    );

    let anchor_key = relations(&main, key(1, 1)).unwrap();
    let item = anchor_key
        .structure
        .iter()
        .find(|item| item.label == "Followed by")
        .expect("an anchor key names the matrices it moves");
    assert_eq!(item.value, "Matrix thumb");
}

fn matrix_context() -> TreeContext {
    TreeContext::Matrix {
        matrix_id: MATRIX.into(),
    }
}

/// Ticket 32 slice, RED: an aggregate anchor renders one chain entry per part
/// from Core facts; part entries navigate to their key and hovering one
/// highlights exactly that part key. The pair is built through Core edits so
/// member parts materialize the way the canvas sees them.
fn aggregate_pair() -> (World, World) {
    let main = matrix(1, 3, json!({}));
    let thumb = matrix(
        1,
        1,
        json!({"id": "thumb",
               "anchor": {"follow": ["x", "y", "rotation"],
                          "aggregate": {"method": "average",
                              "parts": [{"matrixId": "main", "row": 0, "column": 0},
                                        {"matrixId": "main", "row": 0, "column": 2}]}}}),
    );
    // Member parts materialize the scene identities the chain highlights.
    let parts: Vec<serde_json::Value> = ["main/r0c0", "main/r0c1", "main/r0c2", "thumb/r0c0"]
        .iter()
        .enumerate()
        .map(|(index, key)| {
            json!({ "id": format!("matrix/{key}"), "definitionId": "switch",
                    "reference": format!("K{}", index + 1),
                    "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
                    "side": "front" })
        })
        .collect();
    let part_ids: Vec<serde_json::Value> = parts.iter().map(|part| part["id"].clone()).collect();
    let mut document = serde_json::to_value(base()).unwrap();
    document["matrices"] = json!([main, thumb]);
    document["matrices"][0]["partIds"] =
        json!(["matrix/main/r0c0", "matrix/main/r0c1", "matrix/main/r0c2"]);
    document["matrices"][1]["partIds"] = json!(["matrix/thumb/r0c0"]);
    document["boards"][0]["partIds"] = serde_json::Value::Array(part_ids);
    document["parts"] = serde_json::Value::Array(parts);
    document["layouts"] = json!([{ "id": "layout", "name": "Layout",
        "boardId": "board", "matrixIds": ["main", "thumb"], "partIds": [] }]);
    let mut engine = CoreEngine::new();
    let (scene, document) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(document).unwrap(),
    }));
    (
        world_of(scene.clone(), document.clone(), MATRIX),
        world_of(scene, document, "thumb"),
    )
}

#[test]
fn aggregate_anchors_render_one_chain_entry_per_part() {
    let (main, thumb) = aggregate_pair();
    let chain = relations(&thumb, matrix_in("thumb")).unwrap();
    assert_eq!(value(&chain.structure, "Anchor"), "Average of 2 keys");
    let anchor = chain
        .structure
        .iter()
        .find(|item| item.label == "Anchor")
        .expect("the aggregate names its method");
    assert!(anchor.opens_editor, "the Anchor entry opens the Inspector");
    let parts: Vec<&RelationItem> = chain
        .structure
        .iter()
        .filter(|item| item.label == "Anchor part")
        .collect();
    assert_eq!(parts.len(), 2, "one chain entry per part");
    assert_eq!(parts[0].value, "row 1, column 1 of matrix main");
    assert_eq!(parts[0].target, Some(key_in(MATRIX, 0, 0)));
    assert_eq!(parts[0].highlight, vec!["matrix/main/r0c0"]);
    assert!(!parts[0].opens_editor, "part entries navigate");
    assert_eq!(parts[1].value, "row 1, column 3 of matrix main");
    assert_eq!(parts[1].target, Some(key_in(MATRIX, 0, 2)));
    assert_eq!(parts[1].highlight, vec!["matrix/main/r0c2"]);
    let follows = chain
        .structure
        .iter()
        .find(|item| item.label == "Follows")
        .expect("the aggregate names its Follow subset");
    assert_eq!(follows.value, "X, Y, rotation");
    assert_eq!(follows.target, None, "no single key to navigate to");
    assert_eq!(
        follows.highlight,
        vec!["matrix/main/r0c0", "matrix/main/r0c2"],
        "the Follow subset highlights every part"
    );

    let followed = relations(&main, matrix_context()).unwrap();
    assert!(
        followed
            .structure
            .iter()
            .any(|item| item.label == "Followed by" && item.value == "Matrix thumb"),
        "a matrix depended on through a part names its dependant"
    );
    let key_chain = relations(&main, key(0, 0)).unwrap();
    assert!(
        key_chain
            .structure
            .iter()
            .any(|item| item.label == "Followed by" && item.value == "Matrix thumb"),
        "a part key names the aggregate depending on it"
    );
    let row_chain = relations(&thumb, row_in("thumb", 0)).unwrap();
    let frames: Vec<&RelationItem> = row_chain
        .structure
        .iter()
        .filter(|item| item.label == "Anchor frame")
        .collect();
    assert_eq!(frames.len(), 2, "the row chain carries one frame per part");
    assert_eq!(
        frames[0].value,
        "row 1, column 1 of matrix main (follows X, Y, rotation)"
    );
    assert_eq!(frames[0].target, Some(key_in(MATRIX, 0, 0)));
    assert_eq!(frames[0].highlight, vec!["matrix/main/r0c0"]);
}

fn row_in(matrix_id: &str, row: u32) -> TreeContext {
    TreeContext::Row {
        matrix_id: matrix_id.into(),
        row,
    }
}

fn column(column: u32) -> TreeContext {
    TreeContext::Column {
        matrix_id: MATRIX.into(),
        column,
    }
}

fn row(row: u32) -> TreeContext {
    TreeContext::Row {
        matrix_id: MATRIX.into(),
        row,
    }
}

fn key(row: u32, column: u32) -> TreeContext {
    key_in(MATRIX, row, column)
}

fn key_in(matrix_id: &str, row: u32, column: u32) -> TreeContext {
    TreeContext::Key {
        matrix_id: matrix_id.into(),
        row,
        column,
    }
}

fn matrix_in(matrix_id: &str) -> TreeContext {
    TreeContext::Matrix {
        matrix_id: matrix_id.into(),
    }
}

fn value<'a>(items: &'a [RelationItem], label: &str) -> &'a str {
    &items
        .iter()
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("no {label:?} in {items:#?}"))
        .value
}

#[test]
fn a_blank_matrix_shows_structure_at_once_and_no_departures_or_constraints() {
    let world = world(matrix(5, 7, json!({})));

    let matrix = relations(&world, matrix_context()).unwrap();
    assert_eq!(value(&matrix.structure, "Matrix"), "5 × 7, 35 keys");
    assert_eq!(value(&matrix.structure, "Pitch"), "19 × 19 mm");
    assert_eq!(value(&matrix.structure, "Origin"), "0, 0 mm");
    assert_eq!(value(&matrix.structure, "Linked half"), "Independent");
    assert!(matrix.departures.is_empty() && matrix.constraints.is_empty());
    assert_eq!(
        matrix.structure[0].highlight.len(),
        35,
        "the matrix item highlights every key"
    );

    let third = relations(&world, column(2)).unwrap();
    assert_eq!(
        value(&third.structure, "Stagger"),
        "0 mm from Column 2, before turns"
    );
    let inherited: Vec<_> = third
        .structure
        .iter()
        .filter(|item| item.label == "Inherited by")
        .map(|item| item.value.as_str())
        .collect();
    assert_eq!(inherited, ["Column 4", "Column 5", "Column 6", "Column 7"]);
    assert!(third.departures.is_empty() && third.constraints.is_empty());

    let key = relations(&world, key(0, 0)).unwrap();
    assert_eq!(value(&key.structure, "Pitch cell"), "Row 1, Column 1");
    assert!(key.departures.is_empty() && key.constraints.is_empty());
}

#[test]
fn staggers_read_relative_to_the_previous_column_with_a_cumulative_value() {
    let world = world(matrix(
        1,
        3,
        json!({ "columnLayout": [{"stagger": 0.0}, {"stagger": 2.0}, {"stagger": -1.0}]
        }),
    ));

    let first = relations(&world, column(0)).unwrap();
    assert_eq!(
        value(&first.structure, "Stagger"),
        "0 mm from the matrix origin, before turns"
    );
    let second = relations(&world, column(1)).unwrap();
    assert_eq!(
        value(&second.structure, "Stagger"),
        "2 mm from Column 1, cumulative 2 mm, before turns"
    );
    let third = relations(&world, column(2)).unwrap();
    assert_eq!(
        value(&third.structure, "Stagger"),
        "\u{2212}1 mm from Column 2, cumulative 1 mm, before turns"
    );
}

#[test]
fn a_bent_key_reports_its_transported_cumulative_shift_from_core_facts() {
    // Row 0 shifts (10, 0) then bends 90; row 1 shifts (10, 0) into the turned
    // heading. A raw sum would read (20, 0); the kernel transports the second
    // shift through the carried bend, so the matrix-axes cumulative shift at
    // row 1 is (10, 10).
    let world = world(matrix(
        2,
        1,
        json!({ "rowLayout": [{ "shift": { "x": 10.0, "y": 0.0 }, "bend": 90.0 },
                              { "shift": { "x": 10.0, "y": 0.0 } }] }),
    ));

    let first = relations(&world, key(0, 0)).unwrap();
    assert_eq!(
        value(&first.structure, "Cumulative shift"),
        "10, 0 mm in matrix axes"
    );
    assert_eq!(
        value(&first.structure, "Cumulative bend"),
        "90° (row and key only)"
    );

    let key = relations(&world, key(1, 0)).unwrap();
    assert_eq!(value(&key.structure, "Row shift"), "10, 0 mm");
    assert_eq!(value(&key.structure, "Key shift"), "none");
    assert_eq!(value(&key.structure, "Row bend"), "none");
    assert_eq!(
        value(&key.structure, "Cumulative shift"),
        "10, 10 mm in matrix axes"
    );
    assert_eq!(
        value(&key.structure, "Cumulative bend"),
        "90° (row and key only)"
    );
    assert_eq!(
        key.structure
            .iter()
            .find(|item| item.label == "Row shift")
            .and_then(|item| item.target.clone()),
        Some(TreeContext::Row {
            matrix_id: MATRIX.into(),
            row: 1,
        }),
        "a carried contribution navigates to the row that authors it"
    );
}

#[test]
fn a_row_chain_shows_its_padding_and_mixed_key_shifts_as_varies() {
    let world = world(matrix(
        3,
        2,
        json!({
            "rowLayout": [{}, {"padding": 5.0, "shift": {"x": 1.0, "y": 0.0}}, {}],
            "cells": [
                {"row": 1, "column": 0, "enabled": true, "shift": {"x": 3.0, "y": 0.0}},
                {"row": 1, "column": 1, "enabled": true, "shift": {"x": 4.0, "y": 0.0}},
            ],
        }),
    ));

    let row = relations(&world, row(1)).unwrap();
    assert_eq!(value(&row.structure, "Padding"), "5 mm");
    assert_eq!(value(&row.structure, "Row shift"), "1, 0 mm");
    assert_eq!(value(&row.structure, "Key shift"), "Varies");
    assert_eq!(value(&row.structure, "Cumulative shift"), "Varies");
    assert_eq!(
        value(&row.structure, "Cumulative bend"),
        "0° (row and key only)"
    );
    let inherited: Vec<_> = row
        .structure
        .iter()
        .filter(|item| item.label == "Inherited by")
        .map(|item| item.value.as_str())
        .collect();
    assert_eq!(inherited, ["Row 3"]);
}

#[test]
fn a_column_chain_marks_before_turns_totals_and_mixed_shifts_as_varies() {
    let world = world(matrix(
        2,
        2,
        json!({
            "columnLayout": [{}, {"spread": 20.0, "stagger": 2.0, "splay": 15.0,
                                 "rotation": 5.0}],
            "cells": [
                {"row": 0, "column": 1, "enabled": true, "shift": {"x": 3.0, "y": 0.0}},
            ],
        }),
    ));

    let col = relations(&world, column(1)).unwrap();
    assert_eq!(
        value(&col.structure, "Stagger"),
        "2 mm from Column 1, cumulative 2 mm, before turns"
    );
    assert_eq!(value(&col.structure, "Column spread"), "20 mm");
    assert_eq!(
        value(&col.structure, "Column turn"),
        "splay 15°, column rotation 5° (this column only)"
    );
    assert_eq!(value(&col.structure, "Key shift"), "Varies");
    let first = relations(&world, column(0)).unwrap();
    assert_eq!(
        value(&first.structure, "Column spread"),
        "no spread on column 1"
    );
}

#[test]
fn a_matrix_chain_shows_common_values_or_varies() {
    let world = world(matrix(
        2,
        2,
        json!({
            "columnLayout": [{}, {"stagger": 2.0}],
            "rowLayout": [{}, {"shift": {"x": 1.0, "y": 0.0}}],
        }),
    ));

    let summary = relations(&world, matrix_context()).unwrap();
    assert_eq!(value(&summary.structure, "Column stagger"), "Varies");
    assert_eq!(value(&summary.structure, "Column spread"), "19 mm (pitch)");
    assert_eq!(value(&summary.structure, "Row shift"), "Varies");
    assert_eq!(value(&summary.structure, "Key shift"), "none");
}

#[test]
fn selected_parts_in_two_matrices_yield_a_single_context_note() {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (first, _) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "set-matrix", "matrix": matrix(1, 1, json!({"id": "left"})) }),
    );
    let (scene, _document) = apply(
        &mut engine,
        first.revision,
        json!({ "kind": "set-matrix", "matrix": matrix(1, 1, json!({"id": "right"})) }),
    );
    let members: Vec<String> = scene
        .matrix_scenes
        .iter()
        .flat_map(|scene| scene.cells.iter().filter_map(|cell| cell.member_id.clone()))
        .collect();
    assert_eq!(members.len(), 2, "one live member per matrix");
    let context = TreeContext::Key {
        matrix_id: "left".into(),
        row: 0,
        column: 0,
    };
    let note = cross_matrix_note(&scene.matrix_scenes, &context, &members).expect("two matrices");
    assert!(note.contains("Parts in 2 matrices are selected"), "{note}");
    assert!(
        note.contains("select one matrix context for detail"),
        "{note}"
    );
    assert!(
        cross_matrix_note(&scene.matrix_scenes, &context, &members[..1]).is_none(),
        "one matrix stays note-free"
    );
}

#[test]
fn a_column_offset_is_one_departure_on_that_column_only() {
    let world = world(matrix(
        2,
        3,
        json!({ "columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 1.5, "y": 2.0}}]
        }),
    ));

    let offset = relations(&world, column(1)).unwrap();
    assert_eq!(offset.departures.len(), 1);
    let departure = &offset.departures[0];
    assert_eq!(departure.parameter, "Column offset");
    assert_eq!(departure.value, "1.5, 2 mm");
    assert_eq!(
        departure.actions,
        [DepartureAction::Reset, DepartureAction::Convert]
    );
    assert_eq!(departure.target, Some(column(1)));

    assert!(relations(&world, column(0)).unwrap().departures.is_empty());
    assert!(relations(&world, column(2)).unwrap().departures.is_empty());
    let summary = relations(&world, matrix_context()).unwrap();
    assert_eq!(summary.departures.len(), 1);
    assert_eq!(summary.departures[0].parameter, "Columns with an offset");
}

#[test]
fn an_x_only_column_offset_can_reset_but_not_convert() {
    let world = world(matrix(
        2,
        3,
        json!({ "columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 1.5, "y": 0.0}}]
        }),
    ));
    let offset = relations(&world, column(1)).unwrap();
    assert_eq!(offset.departures.len(), 1);
    assert_eq!(offset.departures[0].actions, [DepartureAction::Reset]);
}

#[test]
fn companions_show_their_core_mode_and_a_constrained_one_names_its_source() {
    let assemblies = |ids: [&str; 3]| -> Vec<serde_json::Value> {
        ids.iter()
            .enumerate()
            .map(|(index, id)| {
                json!({ "id": id, "definitionId": "diode", "offset": { "x": 0.0, "y": 8.0 + index as f64 * 3.0 } })
            })
            .collect()
    };
    let generated = world(matrix(
        1,
        1,
        json!({ "cells": [{ "row": 0, "column": 0, "enabled": true,
            "assemblies": assemblies(["a", "b", "c"]) }] }),
    ));
    let ids: Vec<String> = generated
        .scene
        .companions
        .iter()
        .map(|companion| companion.part_id.clone())
        .collect();
    assert_eq!(ids.len(), 3);

    // Core decides the modes: one authored by position override, one by a constraint.
    let mut document = generated.document.clone();
    document
        .parts
        .iter_mut()
        .find(|part| part.id == ids[1])
        .unwrap()
        .properties
        .get_or_insert_with(Default::default)
        .insert("layoutOverride".into(), json!(1));
    document.constraints.push(Constraint::Offset {
        id: "tie".into(),
        source_part_id: "matrix/main/r0c0".into(),
        target_part_id: ids[2].clone(),
        offset: Vec2 { x: 0.0, y: 12.0 },
        rotation: 0.0,
    });
    let world = reopen(document, MATRIX);

    let key = relations(&world, key(0, 0)).unwrap();
    let source = world
        .document
        .parts
        .iter()
        .find(|part| part.id == "matrix/main/r0c0")
        .unwrap()
        .reference
        .clone();
    let modes: Vec<&str> = key
        .structure
        .iter()
        .filter(|item| item.label.starts_with("Companion "))
        .map(|item| item.value.as_str())
        .collect();
    assert_eq!(modes.len(), 3);
    assert!(modes.contains(&"follows key"));
    assert!(modes.contains(&"authored position relative to key"));
    let constrained = format!("explicit constraint takes priority (source {source})");
    assert!(modes.contains(&constrained.as_str()), "{modes:?}");

    assert_eq!(
        key.departures.len(),
        1,
        "only the authored companion departs"
    );
    assert_eq!(key.departures[0].parameter, "Authored companion");
    assert_eq!(key.constraints.len(), 1);
    assert!(key.constraints[0].takes_precedence);
    assert_eq!(key.constraints[0].target.part_id, ids[2]);

    let component = relations(
        &world,
        TreeContext::Component {
            part_id: Some(ids[2].clone()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        },
    )
    .unwrap();
    assert_eq!(value(&component.structure, "Placement"), constrained);
    assert_eq!(component.constraints.len(), 1);
}

#[test]
fn framed_constraints_project_their_relation_kind_and_authored_endpoint_views() {
    let generated = world(matrix(
        1,
        1,
        json!({ "cells": [{ "row": 0, "column": 0, "enabled": true,
            "assemblies": [{ "id": "D1", "definitionId": "diode", "offset": { "x": 0.0, "y": 8.0 } }] }] }),
    ));
    let key_id = generated.scene.cells[0]
        .member_id
        .clone()
        .expect("the matrix creates its key");
    let diode_id = generated.scene.companions[0].part_id.clone();
    let endpoints = || {
        (
            ConstraintEndpoint::Part {
                part_id: key_id.clone(),
            },
            ConstraintEndpoint::Part {
                part_id: diode_id.clone(),
            },
        )
    };
    let cases = [
        (
            "board-offset",
            ConstraintRelation::Offset {
                offset: Vec2 { x: 1.0, y: 2.0 },
                rotation: 3.0,
            },
            ConstraintKind::Offset,
        ),
        (
            "part-offset",
            ConstraintRelation::PartOffset {
                offset: Vec2 { x: 4.0, y: 5.0 },
                rotation: 6.0,
            },
            ConstraintKind::PartOffset,
        ),
        (
            "mirror-line",
            ConstraintRelation::MirrorLine {
                at: Vec2 { x: 7.0, y: -8.0 },
                angle: 37.0,
            },
            ConstraintKind::MirrorLine,
        ),
    ];

    for (id, relation, kind) in cases {
        let (source, target) = endpoints();
        let mut document = generated.document.clone();
        document.constraints.push(Constraint::Framed {
            id: id.into(),
            source: source.clone(),
            target: target.clone(),
            relation: relation.clone(),
        });
        let world = reopen(document, MATRIX);
        let projected = relations(&world, key(0, 0)).expect("the key projects");
        let item = projected
            .constraints
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("the framed constraint {id:?} is listed"));

        assert_eq!(item.kind, kind, "the relation kind is not flattened");
        assert_eq!(
            item.source_endpoint, source,
            "the authored source view survives"
        );
        assert_eq!(
            item.target_endpoint, target,
            "the authored target view survives"
        );
        assert_eq!(item.relation.as_ref(), Some(&relation));
        assert_eq!(item.source.part_id, key_id);
        assert_eq!(item.target.part_id, diode_id);
    }
}

#[test]
fn each_half_of_a_linked_pair_names_its_partner() {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (scene, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "create-mirrored-pair",
            "matrix": matrix(2, 3, json!({ "id": "left-matrix", "origin": { "x": -20.0, "y": 5.0 }, "mirror": "x" })),
            "left": { "id": "left-layout", "name": "Left half", "boardId": "board",
                      "matrixIds": ["left-matrix"], "partIds": [] },
            "right": { "id": "right-layout", "name": "Right half", "boardId": "board",
                       "matrixIds": ["right-matrix"], "partIds": [],
                       "mirrorLink": { "sourceId": "left-layout", "axisX": 10.0 } } }),
    );

    let left = world_of(scene.clone(), document.clone(), "left-matrix");
    let right = world_of(scene, document, "right-matrix");
    let left_matrix = TreeContext::Matrix {
        matrix_id: "left-matrix".into(),
    };
    let right_matrix = TreeContext::Matrix {
        matrix_id: "right-matrix".into(),
    };
    assert_eq!(
        value(
            &relations(&left, left_matrix).unwrap().structure,
            "Linked half"
        ),
        "Right half"
    );
    assert_eq!(
        value(
            &relations(&right, right_matrix).unwrap().structure,
            "Linked half"
        ),
        "Left half"
    );
    let partner = relations(
        &left,
        TreeContext::Key {
            matrix_id: "left-matrix".into(),
            row: 1,
            column: 2,
        },
    )
    .unwrap();
    let link = partner
        .structure
        .iter()
        .find(|item| item.label == "Linked partner")
        .expect("a linked key names its partner key");
    assert_eq!(
        link.target,
        Some(TreeContext::Key {
            matrix_id: "right-matrix".into(),
            row: 1,
            column: 2,
        })
    );
}

#[test]
fn a_context_naming_a_removed_or_foreign_object_projects_nothing() {
    let world = world(matrix(
        2,
        2,
        json!({ "cells": [
            { "row": 0, "column": 1, "enabled": false, "deleted": true },
            { "row": 1, "column": 1, "enabled": false }
        ] }),
    ));

    assert!(relations(&world, key(0, 1)).is_none(), "a removed key");
    assert!(
        relations(&world, key(1, 1)).is_some(),
        "a disabled key still exists"
    );
    assert!(
        relations(&world, column(2)).is_none(),
        "a column past the matrix"
    );
    assert!(
        relations(
            &world,
            TreeContext::Row {
                matrix_id: MATRIX.into(),
                row: 2
            }
        )
        .is_none(),
        "a row past the matrix"
    );
    assert!(
        relations(
            &world,
            TreeContext::Matrix {
                matrix_id: "other".into()
            }
        )
        .is_none(),
        "another matrix"
    );
    let summary = relations(&world, matrix_context()).unwrap();
    assert_eq!(
        value(&summary.structure, "Matrix"),
        "2 × 2, 2 keys, 1 disabled, 1 removed"
    );
}

/// Ticket 16: hovering or focusing an Inspector number field previews, in
/// amber, the keys a change to that field would move. The pure projection
/// below resolves the affected members (including mirrored keys) without
/// writing the document; the canvas hint reads "Moves N keys (n per half)".
use crate::matrix_transform_operation::MatrixTransformField;
use boardstudio_core::model::MatrixSplayAffect;

fn mirrored_grid(rows: u32, columns: u32) -> (World, World) {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (scene, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "create-mirrored-pair",
            "matrix": matrix(rows, columns, json!({ "id": "left-matrix",
                "origin": { "x": -20.0, "y": 5.0 }, "mirror": "x" })),
            "left": { "id": "left-layout", "name": "Left half", "boardId": "board",
                      "matrixIds": ["left-matrix"], "partIds": [] },
            "right": { "id": "right-layout", "name": "Right half", "boardId": "board",
                       "matrixIds": ["right-matrix"], "partIds": [],
                       "mirrorLink": { "sourceId": "left-layout", "axisX": 10.0 } } }),
    );
    (
        world_of(scene.clone(), document.clone(), "left-matrix"),
        world_of(scene, document, "right-matrix"),
    )
}

fn column_in(matrix_id: &str, column: u32) -> TreeContext {
    TreeContext::Column {
        matrix_id: matrix_id.into(),
        column,
    }
}

fn preview(
    world: &World,
    context: TreeContext,
    field: MatrixTransformField,
) -> Option<FieldPreview> {
    field_preview(
        &world.document,
        &world.scenes,
        &context,
        field,
        MatrixSplayAffect::Following,
    )
}

fn sorted(mut parts: Vec<String>) -> Vec<String> {
    parts.sort();
    parts
}

#[test]
fn column_spread_previews_later_columns_on_both_halves() {
    let (left, _) = mirrored_grid(2, 4);
    let preview = preview(
        &left,
        column_in("left-matrix", 1),
        MatrixTransformField::ColumnSpread,
    )
    .expect("spread previews the keys it carries");
    let mut expected = Vec::new();
    for half in ["left-matrix", "right-matrix"] {
        for column in 1..4 {
            for row in 0..2 {
                expected.push(format!("matrix/{half}/r{row}c{column}"));
            }
        }
    }
    assert_eq!(sorted(preview.parts), sorted(expected));
    assert_eq!(preview.hint, "Moves 12 keys (6 per half)");
}

#[test]
fn column_splay_follows_the_splay_affect_toggle() {
    let (left, _) = mirrored_grid(2, 4);
    let carried = field_preview(
        &left.document,
        &left.scenes,
        &column_in("left-matrix", 1),
        MatrixTransformField::ColumnSplay,
        MatrixSplayAffect::Following,
    )
    .expect("a following splay carries");
    assert_eq!(carried.parts.len(), 12);
    let own = field_preview(
        &left.document,
        &left.scenes,
        &column_in("left-matrix", 1),
        MatrixTransformField::ColumnSplay,
        MatrixSplayAffect::Column,
    )
    .expect("a column-only splay stays local");
    let mut expected = Vec::new();
    for half in ["left-matrix", "right-matrix"] {
        for row in 0..2 {
            expected.push(format!("matrix/{half}/r{row}c1"));
        }
    }
    assert_eq!(sorted(own.parts), sorted(expected));
    assert_eq!(own.hint, "Moves 4 keys (2 per half)");
}

#[test]
fn column_offset_stays_on_its_column_on_both_halves() {
    let (left, _) = mirrored_grid(2, 4);
    let preview = preview(
        &left,
        column_in("left-matrix", 2),
        MatrixTransformField::ColumnOffsetY,
    )
    .expect("offset previews its column");
    assert_eq!(preview.parts.len(), 4);
    assert_eq!(preview.hint, "Moves 4 keys (2 per half)");
}

#[test]
fn row_shift_carries_to_later_rows_on_both_halves() {
    let (left, _) = mirrored_grid(3, 2);
    let carried = preview(
        &left,
        row_in("left-matrix", 0),
        MatrixTransformField::RowShiftX,
    )
    .expect("row shift carries");
    assert_eq!(carried.parts.len(), 12);
    assert_eq!(carried.hint, "Moves 12 keys (6 per half)");
    let own_row = preview(
        &left,
        row_in("left-matrix", 2),
        MatrixTransformField::RowOffsetY,
    )
    .expect("row offset stays local");
    assert_eq!(own_row.parts.len(), 4);
    assert_eq!(own_row.hint, "Moves 4 keys (2 per half)");
}

#[test]
fn key_shift_carries_upward_on_both_halves() {
    let (left, _) = mirrored_grid(3, 2);
    let preview = preview(
        &left,
        key_in("left-matrix", 0, 1),
        MatrixTransformField::KeyShiftX,
    )
    .expect("key shift carries upward");
    let mut expected = Vec::new();
    for half in ["left-matrix", "right-matrix"] {
        for row in 0..3 {
            expected.push(format!("matrix/{half}/r{row}c1"));
        }
    }
    assert_eq!(sorted(preview.parts), sorted(expected));
    assert_eq!(preview.hint, "Moves 6 keys (3 per half)");
}

#[test]
fn key_local_edits_preview_both_halves_by_default() {
    let (left, _) = mirrored_grid(2, 2);
    let preview = preview(
        &left,
        key_in("left-matrix", 0, 1),
        MatrixTransformField::KeyOffsetX,
    )
    .expect("a local edit previews its key");
    assert_eq!(
        sorted(preview.parts),
        vec![
            "matrix/left-matrix/r0c1".to_owned(),
            "matrix/right-matrix/r0c1".to_owned(),
        ]
    );
    assert_eq!(preview.hint, "Moves 2 keys (1 per half)");
}

#[test]
fn key_local_edits_stay_put_for_a_local_key() {
    let (left, _) = mirrored_pair_with_exception("part", "local");
    let preview = preview(
        &left,
        key_in("left-matrix", 0, 0),
        MatrixTransformField::KeyRotation,
    )
    .expect("a local key still previews itself");
    assert_eq!(preview.parts, vec!["matrix/left-matrix/r0c0".to_owned()]);
    assert_eq!(preview.hint, "Moves 1 key");
}

#[test]
fn matrix_origin_previews_the_whole_matrix_on_both_halves() {
    let (left, _) = mirrored_grid(2, 4);
    let preview = preview(
        &left,
        matrix_in("left-matrix"),
        MatrixTransformField::OriginX,
    )
    .expect("origin previews the matrix");
    assert_eq!(preview.parts.len(), 16);
    assert_eq!(preview.hint, "Moves 16 keys (8 per half)");
}

#[test]
fn a_local_matrix_edit_never_authors_its_partner() {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (scene, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "create-mirrored-pair",
            "matrix": matrix(1, 1, json!({ "id": "left-matrix",
                "origin": { "x": -20.0, "y": 5.0 }, "mirror": "x" })),
            "left": { "id": "left-layout", "name": "Left half", "boardId": "board",
                      "matrixIds": ["left-matrix"], "partIds": [] },
            "right": { "id": "right-layout", "name": "Right half", "boardId": "board",
                       "matrixIds": ["right-matrix"], "partIds": [],
                       "mirrorLink": { "sourceId": "left-layout", "axisX": 10.0 } } }),
    );
    let mut mirror: serde_json::Value = serde_json::to_value(
        document
            .layouts
            .iter()
            .find(|layout| layout.id == "right-layout")
            .expect("mirror layout"),
    )
    .unwrap();
    // A matrix exception names the matrix record only: unlike the part
    // helper above, no second member exception is valid here.
    mirror["mirrorLink"]["exceptions"] =
        json!([{"kind": "matrix", "mirrorId": "right-matrix", "state": "local"}]);
    let (scene, document) = apply(
        &mut engine,
        scene.revision,
        json!({ "kind": "set-layout", "layout": mirror }),
    );
    let left = world_of(scene, document, "left-matrix");
    let preview = preview(
        &left,
        column_in("left-matrix", 0),
        MatrixTransformField::ColumnSpread,
    )
    .expect("a local matrix still previews itself");
    assert_eq!(preview.parts, vec!["matrix/left-matrix/r0c0".to_owned()]);
    assert_eq!(preview.hint, "Moves 1 key");
}

#[test]
fn an_unmirrored_matrix_has_no_per_half_split() {
    let world = world(matrix(2, 5, json!({})));
    let preview = preview(&world, column(1), MatrixTransformField::ColumnSpread)
        .expect("spread previews the keys it carries");
    assert_eq!(preview.parts.len(), 8);
    assert_eq!(preview.hint, "Moves 8 keys");
}

#[test]
fn non_number_fields_and_foreign_contexts_preview_nothing() {
    let (left, _) = mirrored_grid(2, 2);
    for field in [
        MatrixTransformField::DeleteSelection,
        MatrixTransformField::MatrixMirror,
        MatrixTransformField::KeyEnabled,
        MatrixTransformField::KeyOmit,
        MatrixTransformField::SplayOriginX,
        MatrixTransformField::MatrixDetach,
    ] {
        assert!(
            preview(&left, key_in("left-matrix", 0, 0), field).is_none(),
            "{field:?} previews nothing"
        );
    }
    assert!(
        preview(
            &left,
            TreeContext::Component {
                part_id: None,
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
            MatrixTransformField::ColumnSpread,
        )
        .is_none(),
        "a component context previews nothing"
    );
}

#[test]
fn an_anchor_key_names_the_parts_anchored_to_it() {
    let mut engine = CoreEngine::new();
    let mut document = serde_json::to_value(base()).unwrap();
    document["matrices"] = json!([matrix(2, 2, json!({}))]);
    document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
        "partIds": ["u1"], "netIds": [], "thickness": 1.6 }]);
    document["layouts"] = json!([{ "id": "layout", "name": "Layout",
        "boardId": "board", "matrixIds": [MATRIX], "partIds": ["u1"] }]);
    document["parts"] = json!([{
        "id": "u1", "definitionId": "switch", "reference": "U1",
        "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 }, "side": "front",
        "anchor": { "matrixId": MATRIX, "row": 0, "column": 1,
          "follow": ["x", "y", "rotation"],
          "spread": 0.0, "stagger": 0.0, "splay": 0.0,
          "offset": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } }]);
    let (scene, document) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(document).unwrap(),
    }));
    let world = world_of(scene, document, MATRIX);
    let anchored = relations(&world, key(0, 1)).expect("the anchor key has a chain");
    let item = anchored
        .structure
        .iter()
        .find(|item| item.label == "Anchors part")
        .expect("the anchor key lists its anchored part");
    assert_eq!(item.value, "U1");
    assert_eq!(
        item.target,
        Some(TreeContext::Component {
            part_id: Some("u1".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        })
    );
    let unanchored = relations(&world, key(0, 0)).expect("the plain key has a chain");
    assert!(
        unanchored
            .structure
            .iter()
            .all(|item| item.label != "Anchors part"),
        "a key with no anchored parts names none"
    );
}
