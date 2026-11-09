use super::{Grouping, TreeContext, TreeKind, build_tree};
use boardstudio_application::ReadModel;
use boardstudio_core::model::{BoardOutlineScene, OutlineBridge, ProjectDoc};
use std::collections::BTreeSet;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

fn synthetic_outline_document() -> ProjectDoc {
    serde_json::from_str(include_str!(
        "../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .expect("synthetic outline fixture should deserialize")
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
    let document = synthetic_outline_document();
    let mut expanded = super::default_disclosures(&document, "main");
    expanded.extend([
        "half-group:main:Left half".to_owned(),
        "half-group:main:Right half".to_owned(),
        "half:matrix-layout".to_owned(),
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
        matrix_id: "matrix".into(),
    };
    assert_eq!(super::resolve_selection(&empty, &matrix), None);
    assert_eq!(super::context_for_part(&empty, "missing-part"), None);
    assert_eq!(super::context_for_cell(&empty, "matrix", 0, 0), None);
    assert_eq!(super::context_label(&empty, &matrix), None);
}

fn part_context_model() -> ReadModel {
    use boardstudio_application::{AcceptedSnapshot, SessionEpoch, SnapshotToken};
    use serde_json::json;
    use std::sync::Arc;

    let mut document = synthetic_outline_document();
    let ids = [
        "key-1",
        "key-10",
        "key-10/diode/path",
        "key-10/unknown",
        "disabled",
        "disabled/diode",
        "standalone",
        "off-board",
    ];
    let part = document.parts[0].clone();
    document.parts = ids
        .iter()
        .map(|id| {
            let mut part = part.clone();
            part.id = (*id).into();
            part
        })
        .collect();
    document.boards[0].part_ids = ids[..7].iter().map(|id| (*id).into()).collect();
    document.boards[0].part_ids.push("missing".into());
    document.matrices = vec![serde_json::from_value(json!({
        "id": "matrix", "rows": 1, "columns": 3,
        "pitch": {"x": 19, "y": 19}, "origin": {"x": 0, "y": 0},
        "definitionId": "switch", "partIds": &ids[..6], "boardId": "main",
        "cells": [
            {"row": 0, "column": 0, "enabled": true, "assemblies": [{"id": "diode", "definitionId": "diode", "offset": {"x": 0, "y": 0}}]},
            {"row": 0, "column": 1, "enabled": true, "assemblies": [{"id": "diode/path", "definitionId": "diode", "offset": {"x": 0, "y": 0}}]},
            {"row": 0, "column": 2, "enabled": false, "assemblies": [{"id": "diode", "definitionId": "diode", "offset": {"x": 0, "y": 0}}]}
        ]
    })).unwrap()];
    let scene = serde_json::from_value(json!({
        "revision": 0, "transactionId": "context-test", "changedIds": [],
        "transforms": [], "matrixScenes": [{
            "matrixId": "matrix", "columns": [], "cells": [
                {"row": 0, "column": 0, "enabled": true, "memberId": "key-1", "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "shiftFrame": {"origin": {"x": 0, "y": 0}, "rotation": 0, "axisX": {"x": 1, "y": 0}, "axisY": {"x": 0, "y": 1}}, "offsetFrame": {"origin": {"x": 0, "y": 0}, "rotation": 0, "axisX": {"x": 1, "y": 0}, "axisY": {"x": 0, "y": 1}}},
                {"row": 0, "column": 1, "enabled": true, "memberId": "key-10", "pose": {"at": {"x": 19, "y": 0}, "rotation": 0}, "shiftFrame": {"origin": {"x": 19, "y": 0}, "rotation": 0, "axisX": {"x": 1, "y": 0}, "axisY": {"x": 0, "y": 1}}, "offsetFrame": {"origin": {"x": 19, "y": 0}, "rotation": 0, "axisX": {"x": 1, "y": 0}, "axisY": {"x": 0, "y": 1}}},
                {"row": 0, "column": 2, "enabled": false, "memberId": "disabled", "pose": {"at": {"x": 38, "y": 0}, "rotation": 0}, "shiftFrame": {"origin": {"x": 38, "y": 0}, "rotation": 0, "axisX": {"x": 1, "y": 0}, "axisY": {"x": 0, "y": 1}}, "offsetFrame": {"origin": {"x": 38, "y": 0}, "rotation": 0, "axisX": {"x": 1, "y": 0}, "axisY": {"x": 0, "y": 1}}}
            ]
        }], "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
        "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
    })).unwrap();
    ReadModel {
        accepted: Some(AcceptedSnapshot {
            token: SnapshotToken(1),
            session_epoch: SessionEpoch(1),
            document: Arc::new(document),
            scene: Arc::new(scene),
        }),
        active_board_id: "main".into(),
        ..ReadModel::default()
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn part_context_preserves_exact_primary_and_assembly_membership() {
    let model = part_context_model();
    assert_eq!(
        super::context_for_part(&model, "key-10"),
        Some(TreeContext::Key {
            matrix_id: "matrix".into(),
            row: 0,
            column: 1,
        })
    );
    assert_eq!(
        super::context_for_part(&model, "key-10/diode/path"),
        Some(TreeContext::Component {
            part_id: Some("key-10/diode/path".into()),
            matrix_id: Some("matrix".into()),
            row: Some(0),
            column: Some(1),
            assembly_id: Some("diode/path".into()),
        })
    );
    for id in [
        "key-10/unknown",
        "disabled",
        "disabled/diode",
        "off-board",
        "missing",
    ] {
        assert_eq!(super::context_for_part(&model, id), None, "{id}");
    }
    assert_eq!(
        super::context_for_part(&model, "standalone"),
        Some(TreeContext::Component {
            part_id: Some("standalone".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        })
    );
    for (id, expected) in [
        ("standalone", Some(vec!["standalone".to_owned()])),
        ("off-board", Some(vec![])),
        ("missing", None),
    ] {
        let context = TreeContext::Component {
            part_id: Some(id.into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        };
        assert_eq!(super::resolve_selection(&model, &context), expected, "{id}");
    }
    assert_eq!(
        super::resolve_selection(
            &model,
            &TreeContext::Key {
                matrix_id: "matrix".into(),
                row: 0,
                column: 2
            }
        ),
        Some(vec![]),
        "a disabled primary stays an empty selection"
    );
    assert_eq!(
        super::resolve_selection(
            &model,
            &TreeContext::Matrix {
                matrix_id: "matrix".into()
            }
        ),
        Some(vec![
            "key-1".into(),
            "key-10".into(),
            "key-10/diode/path".into()
        ]),
        "bulk selection preserves enabled primary and declared companion order"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn bulk_eligibility_preserves_context_rules_without_per_part_lookups() {
    for variant in 0..6 {
        let mut model = part_context_model();
        let snapshot = model.accepted.as_mut().unwrap();
        let document = std::sync::Arc::make_mut(&mut snapshot.document);
        match variant {
            1 => document.matrices[0].board_id = None,
            2 => document.matrices[0].board_id = Some("other-board".into()),
            3 => std::sync::Arc::make_mut(&mut snapshot.scene)
                .matrix_scenes
                .clear(),
            4 => {
                document.boards[0].part_ids.reverse();
                document.boards[0].part_ids.push("key-10".into());
                document.parts.retain(|part| part.id != "key-10");
            }
            5 => document.boards[0].part_ids.push("key-1".into()),
            _ => {}
        }
        let ids = document.boards[0].part_ids.clone();
        let expected: Vec<_> = ids
            .iter()
            .filter(|id| super::context_for_part(&model, id).is_some())
            .cloned()
            .collect();
        super::CONTEXT_LOOKUPS.set(0);
        assert_eq!(
            super::eligible_live_ids(&model),
            expected,
            "variant {variant}"
        );
        assert_eq!(
            super::CONTEXT_LOOKUPS.get(),
            0,
            "bulk lookup must not resolve every part separately"
        );
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn column_selection_visits_cell_configuration_once() {
    let mut model = part_context_model();
    let snapshot = model.accepted.as_mut().unwrap();
    let document = std::sync::Arc::make_mut(&mut snapshot.document);
    let matrix = &mut document.matrices[0];
    matrix.cells = (0..64)
        .map(|column| {
            let mut cell = matrix.cells[0].clone();
            cell.column = column;
            cell.enabled = true;
            cell
        })
        .collect();
    matrix.columns = 64;
    let scene = &mut std::sync::Arc::make_mut(&mut snapshot.scene).matrix_scenes[0];
    scene.cells = (0..64)
        .map(|column| {
            let mut cell = scene.cells[0].clone();
            cell.column = column;
            cell.enabled = true;
            cell
        })
        .collect();
    super::CELL_CONFIG_VISITS.set(0);
    assert!(
        super::resolve_selection(
            &model,
            &TreeContext::Column {
                matrix_id: "matrix".into(),
                column: 63,
            }
        )
        .is_some()
    );
    assert!(
        super::CELL_CONFIG_VISITS.get() <= 64,
        "column lookup repeatedly scans configuration: {} visits",
        super::CELL_CONFIG_VISITS.get()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn matrix_context_validation_skips_member_projection_and_preserves_empty_contexts() {
    for variant in 0..6 {
        let mut model = part_context_model();
        match variant {
            1 => {
                std::sync::Arc::make_mut(&mut model.accepted.as_mut().unwrap().document).matrices
                    [0]
                .board_id = None
            }
            2 => {
                std::sync::Arc::make_mut(&mut model.accepted.as_mut().unwrap().document).matrices
                    [0]
                .board_id = Some("other-board".into())
            }
            3 => std::sync::Arc::make_mut(&mut model.accepted.as_mut().unwrap().scene)
                .matrix_scenes
                .clear(),
            4 => model.active_board_id = "missing-board".into(),
            5 => model.accepted = None,
            _ => {}
        }
        for context in [
            TreeContext::Matrix {
                matrix_id: "matrix".into(),
            },
            TreeContext::Row {
                matrix_id: "matrix".into(),
                row: 0,
            },
            TreeContext::Row {
                matrix_id: "matrix".into(),
                row: 1,
            },
            TreeContext::Column {
                matrix_id: "matrix".into(),
                column: 2,
            },
            TreeContext::Column {
                matrix_id: "matrix".into(),
                column: 3,
            },
            TreeContext::Key {
                matrix_id: "matrix".into(),
                row: 0,
                column: 2,
            },
            TreeContext::Matrix {
                matrix_id: "missing".into(),
            },
        ] {
            let expected = super::resolve_selection(&model, &context).is_some();
            super::CELL_CONFIG_VISITS.set(0);
            assert_eq!(
                super::context_is_resolvable(&model, &context),
                expected,
                "{context:?}"
            );
            assert_eq!(
                super::CELL_CONFIG_VISITS.get(),
                0,
                "validity checks must not project matrix members"
            );
        }
    }
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
        matrix_id: "synthetic-matrix".into(),
        row: 0,
        column: 0,
    };
    assert_eq!(
        super::component_context_for_explicit_part("synthetic-switch-1", &ordinary_key),
        Some(TreeContext::Component {
            part_id: Some("synthetic-switch-1".into()),
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
            matrix_id: "synthetic-matrix".into(),
            row: 0,
            column: 0,
        },
        "the projection leaves ordinary canvas Key context intact"
    );
    assert_eq!(
        super::component_context_for_explicit_part(
            "synthetic-switch-1",
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
    let document = synthetic_outline_document();
    let matrix_id = "matrix";
    let layout = document
        .layouts
        .iter()
        .find(|layout| layout.matrix_ids.iter().any(|id| id == matrix_id))
        .expect("synthetic fixture has a matrix layout");
    let bridge_id = "bridge-synthetic-matrix";
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
