use boardstudio_application::interactions::{
    DragSnapOptions, MagneticSnapState, normalize_drag, snap_part_in_board,
};
use boardstudio_core::model::{Part, Position, ProjectDoc, Vec2};
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

#[test]
fn component_drag_pulls_smoothly_toward_the_drag_start_grid_anchor() {
    let document = fixture();
    let mut state = MagneticSnapState::default();
    let start = vec![Position {
        id: "moving".into(),
        at: Vec2::default(),
    }];
    let (positions, guide) = normalize_drag(
        &document,
        &start,
        vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 4.0, y: 0.0 },
        }],
        DragSnapOptions {
            board_id: "main",
            pitch: Vec2 { x: 40.0, y: 40.0 },
            fraction: 0.25,
            geometry_snap: false,
            gap: None,
            targets: &["moving".into()],
            alt: false,
            pixels_per_world: Some(1.0),
            state: &mut state,
        },
    );

    assert!((positions[0].at.x - 0.416).abs() < 1e-9);
    assert!(guide.is_none());

    let (zoomed_positions, _) = normalize_drag(
        &document,
        &start,
        vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 2.0, y: 0.0 },
        }],
        DragSnapOptions {
            board_id: "main",
            pitch: Vec2 { x: 40.0, y: 40.0 },
            fraction: 0.25,
            geometry_snap: false,
            gap: None,
            targets: &["moving".into()],
            alt: false,
            pixels_per_world: Some(2.0),
            state: &mut state,
        },
    );
    assert!((zoomed_positions[0].at.x * 2.0 - positions[0].at.x).abs() < 1e-9);

    let off_grid_start = vec![Position {
        id: "moving".into(),
        at: Vec2 { x: 1.3, y: 0.0 },
    }];
    let (anchored, _) = normalize_drag(
        &document,
        &off_grid_start,
        vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 11.3, y: 0.0 },
        }],
        DragSnapOptions {
            board_id: "main",
            pitch: Vec2 { x: 40.0, y: 40.0 },
            fraction: 0.25,
            geometry_snap: false,
            gap: None,
            targets: &["moving".into()],
            alt: false,
            pixels_per_world: Some(1.0),
            state: &mut state,
        },
    );
    assert_eq!(anchored[0].at.x, 11.3);
}

#[test]
fn component_drag_captures_exactly_and_releases_after_the_wider_hysteresis_radius() {
    let document = fixture();
    let start = vec![Position {
        id: "moving".into(),
        at: Vec2::default(),
    }];
    let targets = vec!["moving".into()];
    let mut state = MagneticSnapState::default();
    let sample = |x, alt, state: &mut MagneticSnapState| {
        normalize_drag(
            &document,
            &start,
            vec![Position {
                id: "moving".into(),
                at: Vec2 { x, y: 0.0 },
            }],
            DragSnapOptions {
                board_id: "main",
                pitch: Vec2 { x: 20.0, y: 20.0 },
                fraction: 1.0,
                geometry_snap: false,
                gap: None,
                targets: &targets,
                alt,
                pixels_per_world: Some(1.0),
                state,
            },
        )
        .0[0]
            .at
    };

    assert_eq!(sample(2.0, false, &mut state).x, 0.0);
    assert_eq!(sample(13.0, false, &mut state).x, 0.0);
    assert!((sample(15.0, false, &mut state).x - 18.24).abs() < 1e-9);

    assert_eq!(sample(2.0, false, &mut state).x, 0.0);
    assert_eq!(sample(4.0, true, &mut state).x, 4.0);
    assert!(sample(13.0, false, &mut state).x > 13.0);
}

#[test]
fn an_eligible_geometry_target_wins_even_when_the_grid_is_closer() {
    let document = fixture();
    let mut state = MagneticSnapState::default();
    let start = vec![Position {
        id: "moving".into(),
        at: Vec2 { x: 4.0, y: 0.0 },
    }];
    let (positions, guide) = normalize_drag(
        &document,
        &start,
        vec![Position {
            id: "moving".into(),
            at: Vec2 { x: 23.5, y: 0.0 },
        }],
        DragSnapOptions {
            board_id: "main",
            pitch: Vec2 { x: 20.0, y: 20.0 },
            fraction: 1.0,
            geometry_snap: true,
            gap: None,
            targets: &["moving".into()],
            alt: false,
            pixels_per_world: Some(1.0),
            state: &mut state,
        },
    );

    assert_eq!(positions[0].at.x, 21.0);
    assert!(guide.is_some());
}

#[test]
fn multi_component_drag_keeps_its_relative_placement() {
    let document = fixture();
    let mut state = MagneticSnapState::default();
    let start = vec![
        Position {
            id: "moving".into(),
            at: Vec2::default(),
        },
        Position {
            id: "right".into(),
            at: Vec2 { x: 30.0, y: 0.0 },
        },
    ];
    let (positions, guide) = normalize_drag(
        &document,
        &start,
        vec![
            Position {
                id: "moving".into(),
                at: Vec2 { x: 19.9, y: 0.0 },
            },
            Position {
                id: "right".into(),
                at: Vec2 { x: 49.9, y: 0.0 },
            },
        ],
        DragSnapOptions {
            board_id: "main",
            pitch: Vec2 { x: 20.0, y: 20.0 },
            fraction: 1.0,
            geometry_snap: true,
            gap: None,
            targets: &["moving".into(), "right".into()],
            alt: false,
            pixels_per_world: Some(1.0),
            state: &mut state,
        },
    );

    assert!(guide.is_none());
    assert_eq!(positions[0].at.x, 20.0);
    assert_eq!(positions[1].at.x - positions[0].at.x, 30.0);
}
