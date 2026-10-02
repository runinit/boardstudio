use super::{Grouping, TreeContext, TreeKind, build_tree};
use boardstudio_core::model::{BoardOutlineScene, OutlineBridge, ProjectDoc};
use std::collections::BTreeSet;

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
