use boardstudio_application::interactions::snap_part_in_board;
use boardstudio_core::model::{Part, ProjectDoc, Vec2};
use serde_json::json;

fn fixture() -> ProjectDoc {
    let mut document = ProjectDoc::empty("snap", "Snap");
    document.definitions.push(
        serde_json::from_value(json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": [],
            "keycap": {"x": 18.0, "y": 18.0}
        }))
        .unwrap(),
    );
    for (id, x) in [("moving", 0.0), ("right", 30.0), ("left", -30.0)] {
        document.parts.push(
            serde_json::from_value(json!({
                "id": id, "definitionId": "switch", "reference": id,
                "pose": {"at": {"x": x, "y": 0.0}, "rotation": 0.0}, "side": "front"
            }))
            .unwrap(),
        );
    }
    document.boards.push(
        serde_json::from_value(json!({
            "id": "main", "name": "Main", "partIds": ["moving", "right", "left"],
            "outlineIds": [], "netIds": [], "thickness": 1.6
        }))
        .unwrap(),
    );
    document
}

#[test]
fn snap_ties_keep_document_order_and_exclude_other_boards() {
    let mut document = fixture();
    let moving = document.parts[0].clone();
    let snap = snap_part_in_board(&document, "main", &moving, 12.0, None).unwrap();
    assert_eq!(snap.at, Vec2 { x: -12.0, y: 0.0 });
    assert_eq!(snap.label, "left · corner / midpoint / center");
    document.boards[0].part_ids.retain(|id| id != "left");
    let snap = snap_part_in_board(&document, "main", &moving, 12.0, None).unwrap();
    assert_eq!(snap.at, Vec2 { x: 12.0, y: 0.0 });
    assert_eq!(snap.label, "right · corner / midpoint / center");
}

#[test]
fn gap_snap_preserves_edge_alignment_for_rotated_envelopes() {
    let mut document = fixture();
    let moving = Part {
        pose: boardstudio_core::model::Pose2 {
            at: Vec2 { x: 10.9, y: 0.3 },
            rotation: 0.0,
        },
        ..document.parts[0].clone()
    };
    for rotation in [0.0, 90.0] {
        document.parts[1].pose.rotation = rotation;
        let snap = snap_part_in_board(&document, "main", &moving, 2.0, Some(1.0)).unwrap();
        assert!((snap.at.x - 11.0).abs() < 1e-9);
        assert!(snap.at.y.abs() < 1e-9);
        assert_eq!(snap.label, "right · envelope gap 1.00 mm · edges aligned");
    }
}

#[test]
fn legacy_unindexed_board_and_missing_board_keep_their_distinct_policies() {
    let mut document = fixture();
    let moving = document.parts[0].clone();
    assert!(snap_part_in_board(&document, "missing", &moving, 12.0, None).is_none());
    document.boards[0].part_ids.clear();
    let legacy = snap_part_in_board(&document, "main", &moving, 12.0, None);
    assert!(legacy.is_some());
    document.boards.clear();
    assert_eq!(
        snap_part_in_board(&document, "missing", &moving, 12.0, None),
        legacy
    );
}
