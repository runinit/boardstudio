//! Ticket 17: the canvas anchor badge draws the right badge, label and target
//! for each selection kind, and clears when the selection clears.

use super::*;
use boardstudio_core::layout_kernel::ComposedFrame;
use boardstudio_core::model::{MatrixSceneCell, Pose2};

fn frame() -> ComposedFrame {
    ComposedFrame {
        origin: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
        axis_x: Vec2 { x: 1.0, y: 0.0 },
        axis_y: Vec2 { x: 0.0, y: 1.0 },
    }
}

fn cell(row: u32, column: u32, x: f64, y: f64) -> MatrixSceneCell {
    MatrixSceneCell {
        row,
        column,
        enabled: true,
        member_id: Some(format!("matrix/main/r{row}c{column}")),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        shift_frame: frame(),
        offset_frame: frame(),
        row_shift: Vec2::default(),
        key_shift: Vec2::default(),
        row_bend: 0.0,
        key_bend: 0.0,
        cumulative_shift: Vec2::default(),
        cumulative_bend: 0.0,
    }
}

fn thumb_cell() -> MatrixSceneCell {
    MatrixSceneCell {
        row: 0,
        column: 0,
        enabled: true,
        member_id: Some("matrix/thumb/r0c0".to_owned()),
        pose: Pose2 {
            at: Vec2 { x: 60.0, y: 0.0 },
            rotation: 0.0,
        },
        shift_frame: frame(),
        offset_frame: frame(),
        row_shift: Vec2::default(),
        key_shift: Vec2::default(),
        row_bend: 0.0,
        key_bend: 0.0,
        cumulative_shift: Vec2::default(),
        cumulative_bend: 0.0,
    }
}

/// A 2x3 `main` plus a 1x1 `thumb` anchored to `main` R1C1, on a stored-gap
/// split pair (axis 0, gap 40): the Left half origin sits at (-20, 0).
fn document() -> ProjectDoc {
    let mut document = serde_json::to_value(ProjectDoc::empty("badge", "Badge")).unwrap();
    document["boards"] = serde_json::json!([{ "id": "board", "name": "Board", "outlineIds": [],
        "partIds": [], "netIds": [], "thickness": 1.6 }]);
    document["parts"] = serde_json::json!([
        { "id": "u1", "definitionId": "switch", "reference": "U1",
          "pose": { "at": { "x": -5.0, "y": 2.0 }, "rotation": 0.0 }, "side": "front",
          "anchor": { "matrixId": "main", "row": 0, "column": 0,
            "follow": ["x", "y", "rotation"],
            "spread": 5.0, "stagger": 2.0, "splay": 0.0,
            "offset": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "u2", "definitionId": "switch", "reference": "U2",
          "pose": { "at": { "x": 10.0, "y": 5.0 }, "rotation": 0.0 }, "side": "front" }
    ]);
    document["layouts"] = serde_json::json!([
        { "id": "left", "name": "Left half", "boardId": "board",
          "matrixIds": ["main"], "partIds": ["u1", "u2"] },
        { "id": "right", "name": "Right half", "boardId": "board",
          "matrixIds": ["thumb"], "partIds": [],
          "mirrorLink": { "sourceId": "left", "axisX": 0.0,
            "splitGap": 40.0, "spacing": 40.0,
            "correspondences": [], "exceptions": [] } }
    ]);
    document["matrices"] = serde_json::json!([
        { "id": "main", "name": "Main", "rows": 2, "columns": 3,
          "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": -10.0, "y": 0.0 },
          "definitionId": "switch", "boardId": "board", "partIds": [],
          "cells": [] },
        { "id": "thumb", "name": "Thumb", "rows": 1, "columns": 1,
          "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 60.0, "y": 0.0 },
          "definitionId": "switch", "boardId": "board", "partIds": [],
          "anchor": { "matrixId": "main", "row": 0, "column": 0,
            "follow": ["x", "y", "rotation"] },
          "cells": [] }
    ]);
    serde_json::from_value(document).unwrap()
}

fn basis(column: u32, x: f64, y: f64) -> boardstudio_core::model::MatrixColumnBasis {
    boardstudio_core::model::MatrixColumnBasis {
        column,
        base: Vec2 { x, y },
        rotation: 0.0,
        relative_origin: Vec2 { x: 0.0, y: 0.0 },
        spread_axis: None,
        splay_origin: Vec2 { x: 0.0, y: 0.0 },
        splay_angle: 0.0,
        custom_origin: false,
        axis_x: Vec2 { x: 1.0, y: 0.0 },
        axis_y: Vec2 { x: 0.0, y: 1.0 },
        stagger_axis: Vec2 { x: 0.0, y: 1.0 },
    }
}

fn scenes() -> Vec<MatrixScene> {
    vec![
        MatrixScene {
            matrix_id: "main".to_owned(),
            cells: vec![
                cell(0, 0, -30.0, 0.0),
                cell(0, 1, -11.0, 0.0),
                cell(0, 2, 8.0, 0.0),
                cell(1, 0, -30.0, 19.0),
                cell(1, 1, -11.0, 19.0),
                cell(1, 2, 8.0, 19.0),
            ],
            columns: vec![
                basis(0, -30.0, 0.0),
                basis(1, -11.0, 0.0),
                basis(2, 8.0, 0.0),
            ],
            companions: vec![],
        },
        MatrixScene {
            matrix_id: "thumb".to_owned(),
            cells: vec![thumb_cell()],
            columns: vec![basis(0, 60.0, 0.0)],
            companions: vec![],
        },
    ]
}

fn badge(context: TreeContext) -> Option<AnchorBadge> {
    let document = document();
    let scenes = scenes();
    let refs: Vec<&MatrixScene> = scenes.iter().collect();
    anchor_badge(&document, &refs, Some(&context))
}

#[test]
fn column_badges_the_previous_column_it_carries() {
    let badge = badge(TreeContext::Column {
        matrix_id: "main".into(),
        column: 2,
    })
    .expect("a carried column has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -11.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: 8.0, y: 0.0 });
    assert_eq!(
        badge.highlight,
        vec!["matrix/main/r0c1".to_owned(), "matrix/main/r1c1".to_owned()]
    );
    assert_eq!(badge.label, "Column 2 · spread, stagger, splay carry");
    assert!(badge.tied, "the dashed tie runs to the selection");
}

#[test]
fn first_column_badges_the_matrix_origin() {
    let badge = badge(TreeContext::Column {
        matrix_id: "main".into(),
        column: 0,
    })
    .expect("the first column has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -30.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -30.0, y: 0.0 });
    assert!(badge.highlight.is_empty());
    assert_eq!(badge.label, "Main origin");
}

#[test]
fn first_column_of_an_anchored_matrix_names_what_the_frame_follows() {
    let badge = badge(TreeContext::Column {
        matrix_id: "thumb".into(),
        column: 0,
    })
    .expect("the first column has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: 60.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: 60.0, y: 0.0 });
    assert_eq!(badge.highlight, vec!["matrix/main/r0c0".to_owned()]);
    assert_eq!(
        badge.label,
        "Thumb origin (← Main R1C1 · follows X, Y, rotation)"
    );
}

#[test]
fn anchored_matrix_badges_its_anchor_key() {
    let badge = badge(TreeContext::Matrix {
        matrix_id: "thumb".into(),
    })
    .expect("an anchored matrix has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -30.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: 60.0, y: 0.0 });
    assert_eq!(badge.highlight, vec!["matrix/main/r0c0".to_owned()]);
    assert_eq!(badge.label, "Main R1C1 · follows X, Y, rotation");
    assert!(badge.tied, "the dashed tie runs to the selection");
}

#[test]
fn unanchored_matrix_badges_the_half_origin() {
    let badge = badge(TreeContext::Matrix {
        matrix_id: "main".into(),
    })
    .expect("an unanchored matrix has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -20.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -30.0, y: 0.0 });
    assert!(badge.highlight.is_empty());
    assert_eq!(badge.label, "Left half origin · default");
    assert!(
        badge.tied,
        "the dashed tie runs from the half origin to the matrix"
    );
}

#[test]
fn key_badges_the_key_below_it() {
    let badge = badge(TreeContext::Key {
        matrix_id: "main".into(),
        row: 1,
        column: 2,
    })
    .expect("a carried key has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: 8.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: 8.0, y: 19.0 });
    assert_eq!(badge.highlight, vec!["matrix/main/r0c2".to_owned()]);
    assert_eq!(badge.label, "Key R1C3 · carried from below");
    assert!(badge.tied, "the dashed tie runs to the selection");
}

#[test]
fn first_row_key_badges_its_column_base_without_a_tie() {
    let badge = badge(TreeContext::Key {
        matrix_id: "main".into(),
        row: 0,
        column: 1,
    })
    .expect("a first-row key has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -11.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -11.0, y: 0.0 });
    assert!(badge.highlight.is_empty());
    assert_eq!(badge.label, "Column 2 base");
    assert!(!badge.tied, "the badge sits on the selection itself");
}

#[test]
fn row_badges_the_row_below_it() {
    let badge = badge(TreeContext::Row {
        matrix_id: "main".into(),
        row: 1,
    })
    .expect("a carried row has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -30.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -30.0, y: 19.0 });
    assert_eq!(
        badge.highlight,
        vec![
            "matrix/main/r0c0".to_owned(),
            "matrix/main/r0c1".to_owned(),
            "matrix/main/r0c2".to_owned(),
        ]
    );
    assert_eq!(badge.label, "Row 1 · padding, shift, bend carry");
    assert!(badge.tied, "the dashed tie runs to the selection");
}

#[test]
fn first_row_badges_the_column_bases_without_a_tie() {
    let badge = badge(TreeContext::Row {
        matrix_id: "main".into(),
        row: 0,
    })
    .expect("the first row has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -30.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -30.0, y: 0.0 });
    assert_eq!(badge.label, "Column bases");
    assert!(!badge.tied, "the badge sits on the selection itself");
}

#[test]
fn cell_component_context_badges_like_its_key() {
    let badge = badge(TreeContext::Component {
        part_id: Some("matrix/main/r1c2".into()),
        matrix_id: Some("main".into()),
        row: Some(1),
        column: Some(2),
        assembly_id: None,
    })
    .expect("a key component has an anchor badge");
    assert_eq!(badge.label, "Key R1C3 · carried from below");
    assert_eq!(badge.highlight, vec!["matrix/main/r0c2".to_owned()]);
}

#[test]
fn stale_anchor_has_no_badge() {
    let document = document();
    let mut gone = scenes();
    gone[0]
        .cells
        .retain(|cell| !(cell.row == 0 && cell.column == 0));
    let refs: Vec<&MatrixScene> = gone.iter().collect();
    let badge = anchor_badge(
        &document,
        &refs,
        Some(&TreeContext::Matrix {
            matrix_id: "thumb".into(),
        }),
    );
    assert_eq!(badge, None, "a removed anchor key leaves no badge");
}

#[test]
fn clearing_the_selection_clears_the_badge() {
    let document = document();
    let scenes = scenes();
    let refs: Vec<&MatrixScene> = scenes.iter().collect();
    assert_eq!(
        anchor_badge(&document, &refs, None),
        None,
        "no selection means no badge"
    );
    assert_eq!(
        anchor_badge(
            &document,
            &refs,
            Some(&TreeContext::Board {
                board_id: "board".into()
            })
        ),
        None,
        "a board selection is not a matrix object"
    );
}

fn part_context(part_id: &str) -> TreeContext {
    TreeContext::Component {
        part_id: Some(part_id.to_owned()),
        matrix_id: None,
        row: None,
        column: None,
        assembly_id: None,
    }
}

#[test]
fn anchored_part_badges_its_anchor_key_with_follow_and_tie() {
    let badge = badge(part_context("u1")).expect("an anchored part has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -30.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -25.0, y: 2.0 });
    assert!(badge.tied);
    assert_eq!(badge.highlight, vec!["matrix/main/r0c0".to_owned()]);
    assert_eq!(badge.label, "Main R1C1 · follows X, Y, rotation");
}

#[test]
fn detached_part_badges_its_half_origin_like_an_unanchored_matrix() {
    let badge = badge(part_context("u2")).expect("a detached part has an anchor badge");
    assert_eq!(badge.anchor, Vec2 { x: -20.0, y: 0.0 });
    assert_eq!(badge.selection, Vec2 { x: -10.0, y: 5.0 });
    assert!(badge.tied);
    assert!(badge.highlight.is_empty());
    assert_eq!(badge.label, "Left half origin · default");
}

#[test]
fn matrix_member_part_keeps_its_key_badge() {
    assert!(badge(part_context("matrix/main/r0c0")).is_none());
    assert!(badge(part_context("missing-part")).is_none());
}
