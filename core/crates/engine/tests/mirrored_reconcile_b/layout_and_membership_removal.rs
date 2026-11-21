use super::*;

#[test]
fn local_mirror_layout_edit_survives_source_reconciliation() {
    let mut document = fixed_geometry_fixture().unwrap();
    let source_part_id = "local-layout-source-part";
    let mirror_part_id = "local-layout-mirror-part";
    let mut source_part = document.parts.by_id("matrix/source-matrix/r0c0").clone();
    source_part.id = source_part_id.into();
    source_part.reference = "LOCAL-SOURCE".into();
    let mut mirror_part = document.parts.by_id("matrix/mirror-matrix/r0c0").clone();
    mirror_part.id = mirror_part_id.into();
    mirror_part.reference = "LOCAL-MIRROR".into();
    document.parts.extend([source_part, mirror_part]);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push(source_part_id.into());
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .push(mirror_part_id.into());
    let halves = document.first_halves_mut();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_part_id.into(),
            mirror_id: mirror_part_id.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-local-layout".into(),
            mirror_id: "mirror-local-layout".into(),
        },
    ]);
    halves.exceptions.extend([
        serde_json::from_value(serde_json::json!({
            "kind": "part", "mirrorId": mirror_part_id, "state": "local"
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "kind": "layout", "mirrorId": "mirror-local-layout", "state": "local"
        }))
        .unwrap(),
    ]);
    document.layouts = vec![
        Layout {
            id: "source-local-layout".into(),
            name: "Source layout".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["source-matrix".into()],
            part_ids: vec![source_part_id.into()],
            mirror_link: None,
        },
        Layout {
            id: "mirror-local-layout".into(),
            name: "Mirror layout".into(),
            board_id: "mirror-board".into(),
            matrix_ids: vec!["mirror-matrix".into()],
            part_ids: vec![mirror_part_id.into(), "geometry-probe".into()],
            mirror_link: None,
        },
    ];

    let requested = Layout {
        name: "Authored local name".into(),
        part_ids: vec!["geometry-probe".into()],
        ..document.layouts[1].clone()
    };
    crate::support::apply_for_test(
        &mut document,
        &[],
        &EditOperation::SetLayout {
            layout: requested.clone(),
        },
    )
    .expect("the local mirror layout edit applies");
    reopen(&mut document).expect("the local layout does not need source projection");

    assert_eq!(document.layouts[1].name, requested.name);
    assert_eq!(document.layouts[1].part_ids, requested.part_ids);
}

#[test]
fn omitted_mirror_layout_stays_absent_after_source_edit_and_reconciliation() {
    let mut document = fixed_geometry_fixture().unwrap();
    let mut source_part = document.parts.by_id("matrix/source-matrix/r0c0").clone();
    source_part.id = "omitted-layout-source-part".into();
    source_part.reference = "OMITTED-SOURCE".into();
    document.parts.push(source_part);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .push("omitted-layout-source-part".into());
    let halves = document.first_halves_mut();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Layout,
        source_id: "source-omitted-layout".into(),
        mirror_id: "mirror-omitted-layout".into(),
    });
    halves.exceptions.push(
        serde_json::from_value(serde_json::json!({
            "kind": "layout", "mirrorId": "mirror-omitted-layout", "state": "omitted"
        }))
        .unwrap(),
    );
    document.layouts.push(Layout {
        id: "source-omitted-layout".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![],
        mirror_link: None,
    });

    let requested = Layout {
        name: "Edited source layout".into(),
        part_ids: vec!["omitted-layout-source-part".into()],
        ..document.layouts[0].clone()
    };
    crate::support::apply_for_test(
        &mut document,
        &[],
        &EditOperation::SetLayout {
            layout: requested.clone(),
        },
    )
    .expect("the source layout edit applies");
    reopen(&mut document).expect("an omitted mirror layout stays absent");

    assert_eq!(document.layouts[0], requested);
    assert!(
        document
            .layouts
            .iter()
            .all(|layout| layout.id != "mirror-omitted-layout")
    );
}

#[test]
fn legacy_layout_link_syncs_components_without_changing_an_unrelated_layout() {
    let mut document = fixture_document().unwrap();
    for id in [
        "legacy-source-matrix",
        "legacy-target-matrix",
        "unrelated-matrix",
    ] {
        boardstudio_core_kernel::matrix::set_matrix(
            &mut document,
            &fixture_matrix(id, "source-board"),
        )
        .unwrap();
    }
    let mut source = document.parts.by_id("matrix/source-matrix/r0c0").clone();
    source.id = "legacy-source-part".into();
    source.reference = "J3".into();
    source.pose = Pose2 {
        at: Vec2 { x: 12.0, y: -4.0 },
        rotation: 30.0,
    };
    let mut unrelated = source.clone();
    unrelated.id = "legacy-unrelated-part".into();
    unrelated.reference = "R7".into();
    unrelated.pose = Pose2 {
        at: Vec2 { x: -9.0, y: 21.0 },
        rotation: 72.0,
    };
    document.parts.extend([source.clone(), unrelated.clone()]);
    let source_board = document.boards.by_id_mut("source-board");
    source_board
        .part_ids
        .extend([source.id.clone(), unrelated.id.clone()]);
    document.layouts = vec![
        Layout {
            id: "legacy-source-layout".into(),
            name: "Legacy source".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["legacy-source-matrix".into()],
            part_ids: vec![source.id.clone()],
            mirror_link: None,
        },
        Layout {
            id: "legacy-target-layout".into(),
            name: "Legacy mirror".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["legacy-target-matrix".into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                correspondences: vec![MirroredCorrespondence {
                    kind: MirroredItemKind::Matrix,
                    source_id: "legacy-source-matrix".into(),
                    mirror_id: "legacy-target-matrix".into(),
                    companion_mode: None,
                    companion_tweak: None,
                }],
                exceptions: vec![],
                source_id: "legacy-source-layout".into(),
                axis_x: 50.0,
                split_gap: None,
                spacing: None,
            }),
        },
        Layout {
            id: "unrelated-layout".into(),
            name: "Unrelated local".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["unrelated-matrix".into()],
            part_ids: vec![unrelated.id.clone()],
            mirror_link: None,
        },
    ];

    boardstudio_core_kernel::matrix::layout::validate(&document).unwrap();
    let unrelated_layout_before = document.layouts[2].clone();
    let unrelated_part_before = unrelated.clone();
    let unrelated_owner_ids_before: Vec<_> = document
        .boards
        .iter()
        .filter(|board| board.part_ids.contains(&unrelated.id))
        .map(|board| board.id.clone())
        .collect();
    boardstudio_core_kernel::matrix::layout::sync_components(&mut document).unwrap();
    boardstudio_core_kernel::matrix::layout::validate(&document).unwrap();

    let target_id = "legacy-target-layout::mirror::legacy-source-part";
    let mirrored = document
        .parts
        .iter()
        .find(|part| part.id == target_id)
        .expect("the linked independent component is materialized");
    assert_eq!(mirrored.reference, "J3_M");
    assert_eq!(
        mirrored.pose,
        Pose2 {
            at: Vec2 { x: 88.0, y: -4.0 },
            rotation: 150.0,
        }
    );
    assert_eq!(
        mirrored.properties.as_ref().unwrap()["boardstudio.mirroredComponentSource"],
        serde_json::json!("legacy-source-part")
    );
    assert_eq!(document.layouts[1].part_ids, vec![target_id.to_owned()]);
    assert_eq!(document.layouts[2], unrelated_layout_before);
    assert_eq!(
        document.parts.by_id("legacy-unrelated-part"),
        &unrelated_part_before
    );
    let unrelated_owner_ids_after: Vec<_> = document
        .boards
        .iter()
        .filter(|board| board.part_ids.contains(&unrelated.id))
        .map(|board| board.id.clone())
        .collect();
    assert_eq!(unrelated_owner_ids_after, unrelated_owner_ids_before);
}

#[test]
fn changed_source_matrix_reprojects_existing_mirror_geometry_and_keeps_its_identity() {
    let source = matrix("left-matrix", "left-board", 2, 3, Vec2 { x: 20.0, y: 7.0 });
    let mirror = matrix("right-matrix", "right-board", 1, 1, Vec2 { x: 5.0, y: 1.0 });
    let correspondence = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: source.id.clone(),
        mirror_id: mirror.id.clone(),
    };

    let projected = project_matrix(
        &source,
        &mirror,
        &correspondence,
        Pose2 {
            at: Vec2 { x: 200.0, y: 0.0 },
            rotation: 0.0,
        },
    )
    .expect("the existing matrix correspondence can be projected");

    assert_eq!(projected.id, "right-matrix");
    assert_eq!(projected.board_id.as_deref(), Some("right-board"));
    assert_eq!(projected.rows, 2);
    assert_eq!(projected.columns, 3);
    assert_eq!(projected.origin, Vec2 { x: 180.0, y: 7.0 });
}

#[test]
fn a_rotated_pair_map_reprojects_matrix_origin_and_heading_once() {
    let mut source = matrix("left-matrix", "left-board", 2, 3, Vec2 { x: 5.0, y: 2.0 });
    source.rotation = Some(15.0);
    let mirror = matrix("right-matrix", "right-board", 1, 1, Vec2 { x: 5.0, y: 1.0 });
    let correspondence = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: source.id.clone(),
        mirror_id: mirror.id.clone(),
    };

    let projected = project_matrix(
        &source,
        &mirror,
        &correspondence,
        Pose2 {
            at: Vec2 { x: 20.0, y: -3.0 },
            rotation: 90.0,
        },
    )
    .expect("the existing matrix correspondence can be projected");

    assert_eq!(projected.id, "right-matrix");
    assert_eq!(projected.rows, 2);
    assert_eq!(projected.columns, 3);
    assert_eq!(projected.origin, Vec2 { x: 18.0, y: -8.0 });
    assert_eq!(projected.rotation, Some(75.0));
}

#[test]
fn changed_source_part_pose_keeps_existing_mirror_identity_and_source_parameters() {
    let source = part(
        "left/part",
        "source-definition",
        "U1",
        Pose2 {
            at: Vec2 { x: 20.0, y: 7.0 },
            rotation: 35.0,
        },
        "back",
        9,
    );
    let mirror = part(
        "right/part",
        "mirror-definition",
        "U9",
        Pose2 {
            at: Vec2 { x: 5.0, y: 1.0 },
            rotation: 0.0,
        },
        "front",
        2,
    );
    let correspondence = MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: source.id.clone(),
        mirror_id: mirror.id.clone(),
    };

    let projected = project_part(
        &source,
        &mirror,
        &correspondence,
        Pose2 {
            at: Vec2 { x: 200.0, y: 0.0 },
            rotation: 0.0,
        },
        // The fixture definitions are custom, so the full heading rule applies.
        false,
    )
    .expect("the existing part correspondence can be projected");

    assert_eq!(projected.id, "right/part");
    assert_eq!(projected.definition_id, "mirror-definition");
    assert_eq!(projected.reference, "U9");
    assert_eq!(
        projected.pose,
        Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 145.0,
        }
    );
    assert_eq!(projected.side, source.side);
    assert_eq!(projected.generator_parameters, source.generator_parameters);
    assert_eq!(projected.properties, source.properties);
}
