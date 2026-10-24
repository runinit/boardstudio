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

fn world_of(scene: SceneDelta, document: ProjectDoc, matrix_id: &str) -> World {
    let scene = scene
        .matrix_scenes
        .into_iter()
        .find(|scene| scene.matrix_id == matrix_id)
        .expect("the matrix has a scene");
    World { document, scene }
}

fn world(matrix_json: serde_json::Value) -> World {
    let mut engine = CoreEngine::new();
    let (opened, _) = scene_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: base(),
    }));
    let (scene, document) = apply(
        &mut engine,
        opened.revision,
        json!({ "kind": "set-matrix", "matrix": matrix_json }),
    );
    world_of(scene, document, MATRIX)
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
    matrix_relations(&world.document, &world.scene, &context)
}

fn matrix_context() -> TreeContext {
    TreeContext::Matrix {
        matrix_id: MATRIX.into(),
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
    TreeContext::Key {
        matrix_id: MATRIX.into(),
        row,
        column,
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
