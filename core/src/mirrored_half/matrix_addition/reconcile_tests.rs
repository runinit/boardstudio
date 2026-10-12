//! A matrix the forward addition (ticket 145) mirrored is an ordinary corresponded matrix: a
//! later edit of its source half reaches the mirror through the existing reconciliation.
//!
//! This module is a child of `reconcile` so it can build a `Projection`; the fixtures are the
//! forward transition's own.

use super::{PartProjection, Projection, reconcile};
use crate::mirrored_half::matrix_addition::tests::{
    add_matrix, halves, matrix, new_matrix, op, pair_document,
};
use crate::model::{MirroredItemKind, ProjectDoc};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The new source matrix, after its half's edit, with its mirror reconciled from it.
fn reconciled(document: &ProjectDoc, edit: impl Fn(&mut crate::model::Matrix)) -> ProjectDoc {
    let mut document = document.clone();
    let mut source = matrix(&document, "left-thumbs").clone();
    edit(&mut source);
    crate::matrix::set_matrix(&mut document, &source).expect("the source edit applies");

    let pair = document.hardware.as_ref().unwrap().pairs[0].clone();
    let mirror = matrix(&document, "right-thumbs").clone();
    let source = matrix(&document, "left-thumbs").clone();
    let keys: BTreeSet<String> = source
        .cells
        .iter()
        .filter(|cell| cell.enabled && !cell.deleted)
        .map(|cell| format!("matrix/left-thumbs/r{}c{}", cell.row, cell.column))
        .collect();
    let parts = halves(&document)
        .correspondences
        .iter()
        .filter(|item| item.kind == MirroredItemKind::Part && keys.contains(&item.source_id))
        .map(|item| PartProjection {
            source_id: item.source_id.clone(),
            mirror_id: item.mirror_id.clone(),
        })
        .collect();
    let mut final_owners = BTreeMap::new();
    for id in &source.part_ids {
        final_owners.insert(id.clone(), "left".to_owned());
    }
    for id in &mirror.part_ids {
        final_owners.insert(id.clone(), "right".to_owned());
    }
    let projection = Projection {
        preserve_matrix_geometry: false,
        source_board_id: "left".into(),
        mirror_board_id: "right".into(),
        source_matrix_id: "left-thumbs".into(),
        mirror_matrix_id: "right-thumbs".into(),
        mapping: pair.mapping,
        parts,
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: Default::default(),
        applied_hosted_operations: Default::default(),
        applied_hosted_circuit_additions: Default::default(),
        hosted_removal_baselines: Default::default(),
        hosted_omission_baselines: Default::default(),
        outline_removal_baselines: Default::default(),
        locally_protected_mirror_parts: BTreeSet::new(),
        final_owners,
    };
    reconcile(&mut document, &pair.id, &projection).expect("the mirror reconciles from its source");
    document
}

/// The edit as the creation path would see it: the same source matrix, mirrored at creation.
fn reference(request: &Value, edit: impl Fn(&mut crate::model::Matrix)) -> ProjectDoc {
    let mut document = pair_document_source_only();
    crate::apply(&mut document, &[], &op(request.clone())).unwrap();
    let mut source = matrix(&document, "left-thumbs").clone();
    edit(&mut source);
    crate::matrix::set_matrix(&mut document, &source).unwrap();
    let mut request = json!({
        "kind": "create-mirrored-half", "pairId": "pair", "sourceInstanceId": "left",
        "mode": {"kind": "new-board", "boardId": "right", "instanceId": "right"},
        "mapping": {"at": {"x": 200, "y": 0}, "rotation": 0},
        "idRule": "board-prefix", "companionMode": "literal-mirror"});
    request["companionTweak"] = Value::Null;
    request.as_object_mut().unwrap().remove("companionTweak");
    crate::apply(&mut document, &[], &op(request)).unwrap();
    document
}

/// The pair document's source half before the pair exists: its source parts and matrices only.
fn pair_document_source_only() -> ProjectDoc {
    let mut document = pair_document();
    let source_board = document
        .boards
        .iter()
        .find(|board| board.id == "left")
        .unwrap()
        .clone();
    // Rebuild a pair-free document from the paired one: drop the mirror and its record.
    let mirror_parts: BTreeSet<String> = document
        .boards
        .iter()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    document
        .parts
        .retain(|part| !mirror_parts.contains(&part.id));
    document.boards.retain(|board| board.id == source_board.id);
    document
        .matrices
        .retain(|m| m.board_id.as_deref() == Some("left"));
    document.nets.clear();
    let hardware = document.hardware.as_mut().unwrap();
    hardware.pairs.clear();
    hardware.instances.retain(|instance| instance.id == "left");
    hardware.boards.retain(|entry| entry.board_id == "left");
    document
}

#[test]
fn a_source_edit_of_a_forward_mirrored_matrix_reaches_its_mirror() {
    let request = new_matrix("left-thumbs", "left", (10.0, 120.0), 3);
    let document = add_matrix(&pair_document(), &request);
    assert!(
        halves(&document)
            .correspondences
            .iter()
            .any(|item| item.kind == MirroredItemKind::Matrix && item.source_id == "left-thumbs")
    );
    let edit = |matrix: &mut crate::model::Matrix| {
        matrix.origin.x += 5.0;
        matrix.origin.y -= 7.5;
        matrix.rotation = Some(9.0);
        matrix.cells[0].offset = Some(crate::model::Vec2 { x: 1.0, y: 2.0 });
    };
    let after = reconciled(&document, edit);
    let expected = reference(&request, edit);

    let reflected = matrix(&after, "right-thumbs");
    assert_eq!(
        serde_json::to_value(reflected).unwrap(),
        serde_json::to_value(matrix(&expected, "right-thumbs")).unwrap(),
        "the mirror matrix follows its source"
    );
    // The edit really moved something.
    let before = matrix(&document, "right-thumbs");
    assert!((reflected.origin.x - before.origin.x).abs() > 1.0);
    for id in &reflected.part_ids {
        let actual = after.parts.iter().find(|part| &part.id == id).unwrap();
        let wanted = expected.parts.iter().find(|part| &part.id == id).unwrap();
        assert_eq!(actual.pose, wanted.pose, "{id}");
    }
}
