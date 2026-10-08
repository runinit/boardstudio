use super::{Grouping, TreeContext, TreeKind, build_tree};
use boardstudio_application::ReadModel;
use boardstudio_core::model::{BoardOutlineScene, OutlineBridge, ProjectDoc};
use std::collections::BTreeSet;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

fn reviung_document() -> ProjectDoc {
    serde_json::from_str(include_str!(
        "../../../../core/tests/fixtures/reviung41-outline-original.json"
    ))
    .expect("checked-in Reviung 41 project fixture should deserialize")
}

fn bridge_scene(board_id: &str, matrix_id: &str, bridge_id: &str) -> BoardOutlineScene {
    BoardOutlineScene {
        board_id: board_id.into(),
        source_contours: Vec::new(),
        bridges: vec![OutlineBridge {
            id: bridge_id.into(),
            width: 10.0,
            points: Vec::new(),
            part_ids: Vec::new(),
            matrix_ids: vec![matrix_id.into()],
            authored: false,
        }],
        gaps: Vec::new(),
    }
}

#[test]
fn stored_row_grouping_and_empty_model_projection_remain_defined() {
    let document = reviung_document();
    let mut expanded = super::default_disclosures(&document, "main");
    expanded.extend([
        "half-group:main:Left half".to_owned(),
        "half-group:main:Right half".to_owned(),
        "half:main-right-keys-layout".to_owned(),
    ]);
    let row_grouped = build_tree(
        &document,
        "main",
        Grouping::from_storage(Some("row".into())),
        &expanded,
        &[],
        &[],
    );
    assert!(row_grouped.iter().any(|item| item.kind == TreeKind::Row));

    let empty = ReadModel::default();
    let matrix = TreeContext::Matrix {
        matrix_id: "main-right-keys".into(),
    };
    assert_eq!(super::resolve_selection(&empty, &matrix), None);
    assert_eq!(super::context_for_part(&empty, "missing-part"), None);
    assert_eq!(
        super::context_for_cell(&empty, "main-right-keys", 0, 0),
        None
    );
    assert_eq!(super::context_label(&empty, &matrix), None);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn deleted_cells_are_absent_from_tree_but_disabled_slots_remain() {
    let document = ProjectDoc::empty("tree-deletion", "Tree deletion");
    let matrix = serde_json::from_value(serde_json::json!({
        "id": "matrix", "rows": 1, "columns": 2,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0},
        "definitionId": "switch", "partIds": [], "boardId": "main",
        "cells": [
            {"row": 0, "column": 0, "enabled": false, "deleted": true},
            {"row": 0, "column": 1, "enabled": false}
        ]
    }))
    .unwrap();
    for grouping in [Grouping::Row, Grouping::Column] {
        let expanded = BTreeSet::from([
            "row:matrix:0".to_owned(),
            "column:matrix:0".to_owned(),
            "column:matrix:1".to_owned(),
        ]);
        let mut items = Vec::new();
        super::append_matrix(
            &mut items,
            &document,
            &matrix,
            None,
            &std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            grouping,
            &expanded,
            0,
            false,
            "main",
            &[],
            "matrix",
        );
        assert!(
            !items.iter().any(|item| item.id == "key:matrix:0:0"),
            "a deleted key must not remain selectable as an empty slot"
        );
        assert!(
            items.iter().any(|item| item.id == "key:matrix:0:1"),
            "a disabled key keeps its empty slot for later enabling"
        );
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn explicit_finding_part_route_uses_component_context_without_changing_key_hit_context() {
    let ordinary_key = TreeContext::Key {
        matrix_id: "left-keys".into(),
        row: 0,
        column: 0,
    };
    assert_eq!(
        super::component_context_for_explicit_part("left-keys-SW1", &ordinary_key),
        Some(TreeContext::Component {
            part_id: Some("left-keys-SW1".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        }),
        "an explicit Part finding action enters the component Inspector for that part"
    );
    assert_eq!(
        ordinary_key,
        TreeContext::Key {
            matrix_id: "left-keys".into(),
            row: 0,
            column: 0,
        },
        "the projection leaves ordinary canvas Key context intact"
    );
    assert_eq!(
        super::component_context_for_explicit_part(
            "left-keys-SW1",
            &TreeContext::Board {
                board_id: "left".into(),
            },
        ),
        None,
        "non-part contexts cannot be converted into component selection"
    );
}

#[test]
fn bridge_has_same_board_selection_at_outline_and_owning_matrix_occurrences() {
    let document = reviung_document();
    let matrix_id = "main-right-keys";
    let layout = document
        .layouts
        .iter()
        .find(|layout| layout.matrix_id == matrix_id)
        .expect("fixture has a right-keys layout");
    let bridge_id = "bridge-right-keys";
    let mut expanded = BTreeSet::from([
        "board:main".to_owned(),
        format!("half:{}", layout.id),
        "outline:main".to_owned(),
    ]);
    let items = build_tree(
        &document,
        "main",
        Grouping::Column,
        &expanded,
        &[],
        &[bridge_scene("main", matrix_id, bridge_id)],
    );

    let expected_context = TreeContext::Bridge {
        board_id: "main".into(),
        bridge_id: bridge_id.into(),
    };
    let occurrences: Vec<_> = items
        .iter()
        .filter(|item| {
            item.kind == TreeKind::Bridge && item.context.as_ref() == Some(&expected_context)
        })
        .collect();
    assert_eq!(
        occurrences.len(),
        2,
        "the Outline and owning layout/matrix routes should expose separate rows for one bridge"
    );
    assert_ne!(occurrences[0].id, occurrences[1].id);
    assert!(
        occurrences
            .iter()
            .all(|item| item.context.as_ref() == Some(&expected_context))
    );

    let matrix_occurrence = occurrences
        .iter()
        .find(|item| item.id == format!("half:{}:bridge:{bridge_id}", layout.id))
        .expect("owning matrix route keeps a distinct occurrence key");
    let layout_row = items
        .iter()
        .position(|item| item.id == format!("half:{}", layout.id))
        .expect("owning layout appears in the board tree");
    assert_eq!(matrix_occurrence.level, items[layout_row].level + 1);

    expanded.remove("outline:main");
    let items_with_outline_closed = build_tree(
        &document,
        "main",
        Grouping::Column,
        &expanded,
        &[],
        &[bridge_scene("main", matrix_id, bridge_id)],
    );
    assert!(items_with_outline_closed.iter().any(|item| {
        item.id == format!("half:{}:bridge:{bridge_id}", layout.id)
            && item.context.as_ref() == Some(&expected_context)
    }));
}
