use super::*;

#[test]
fn local_mirror_layout_edit_survives_source_reconciliation() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let source_part_id = "local-layout-source-part";
    let mirror_part_id = "local-layout-mirror-part";
    let mut source_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c0")
        .unwrap()
        .clone();
    source_part.id = source_part_id.into();
    source_part.reference = "LOCAL-SOURCE".into();
    let mut mirror_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c0")
        .unwrap()
        .clone();
    mirror_part.id = mirror_part_id.into();
    mirror_part.reference = "LOCAL-MIRROR".into();
    document.parts.extend([source_part, mirror_part]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push(source_part_id.into());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push(mirror_part_id.into());
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the local layout does not need source projection");

    assert_eq!(document.layouts[1].name, requested.name);
    assert_eq!(document.layouts[1].part_ids, requested.part_ids);
}

#[test]
fn omitted_mirror_layout_stays_absent_after_source_edit_and_reconciliation() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let mut source_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c0")
        .unwrap()
        .clone();
    source_part.id = "omitted-layout-source-part".into();
    source_part.reference = "OMITTED-SOURCE".into();
    document.parts.push(source_part);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push("omitted-layout-source-part".into());
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
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
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("an omitted mirror layout stays absent");

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
    let mut source = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c0")
        .unwrap()
        .clone();
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
    let source_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap();
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
        document
            .parts
            .iter()
            .find(|part| part.id == "legacy-unrelated-part")
            .unwrap(),
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

#[test]
fn membership_transition_accepts_a_valid_identity_operation_as_a_no_op() {
    let (document, _) = fixed_geometry_fixture().expect("the fixture is constructed");
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&document).is_ok());
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        let matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .expect("the fixture contains both matrices");
        assert!(boardstudio_core_kernel::matrix::valid_matrix(matrix, &document).is_ok());
    }

    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    let halves = pair.halves.as_ref().unwrap();
    let mut context = MembershipContext {
        pair_id: pair.id.clone(),
        source_board_id: halves.source_board_id.clone(),
        mirror_board_id: halves.mirror_board_id.clone(),
        matrix_pair: Some(
            boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
            },
        ),
        mapping: pair.mapping,
        pair_right: halves.clone(),
        associations: halves
            .correspondences
            .iter()
            .map(|association| TransitionAssociation {
                kind: association.kind,
                source_id: association.source_id.clone(),
                mirror_id: association.mirror_id.clone(),
                parent_source_id: None,
                parent_mirror_id: None,
                state: TransitionState::Linked,
                explicit_state: false,
                raw_mode_override: association.companion_mode,
                raw_tweak: association.companion_tweak,
                effective_mode: association.companion_mode.unwrap_or(halves.companion_mode),
                effective_tweak: association.companion_tweak,
            })
            .collect(),
        owners: BTreeMap::new(),
        pending_rejoins: BTreeMap::new(),
        pending_pair_transitions: BTreeMap::new(),
        pending_pair_baseline: None,
    };
    context.associations[1].state = TransitionState::Local;
    context.associations[1].explicit_state = true;
    let constrained_key = context
        .associations
        .iter_mut()
        .find(|association| association.mirror_id == "matrix/mirror-matrix/r1c1")
        .expect("the constrained mirror key has a private association");
    constrained_key.state = TransitionState::Local;
    constrained_key.explicit_state = true;
    let mut source_identity_map = BTreeMap::new();
    for id in &document.matrices[0].part_ids {
        source_identity_map.insert(id.clone(), Some(id.clone()));
    }
    let mirror_identity_map: BTreeMap<_, _> = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mut final_owners = BTreeMap::new();
    for part in &document.parts {
        let owner = document
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&part.id))
            .expect("every fixture part has one owner");
        final_owners.insert(part.id.clone(), FinalOwner::Board(owner.id.clone()));
    }
    context.owners = final_owners.clone();
    let explicit_removal_closure = BTreeSet::new();
    let planned_additions = [];

    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &document,
        post_source: &document,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &planned_additions,
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_removal_closure,
    });

    assert!(
        result.is_ok(),
        "valid no-op transition was rejected: {result:?}"
    );
    let output = result.unwrap();
    assert_eq!(output.draft, document);
    assert!(output.changed_document_ids.is_empty());
    assert!(output.changed_field_paths.is_empty());
    assert_eq!(output.context, context);
    assert_eq!(output.final_owners, final_owners);
}

#[test]
fn membership_transition_removes_a_whole_paired_matrix_from_a_source_operation() {
    let (mut before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the paired matrix fixture is valid");
    boardstudio_core_kernel::matrix::set_matrix(
        &mut before,
        &fixture_matrix("unrelated-matrix", "mirror-board"),
    )
    .expect("the unrelated matrix is materialized through Core");
    let unrelated_matrix = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "unrelated-matrix")
        .unwrap()
        .clone();
    let unrelated_member = unrelated_matrix.part_ids[0].clone();
    for id in &unrelated_matrix.part_ids {
        context
            .owners
            .insert(id.clone(), FinalOwner::Board("mirror-board".into()));
    }
    let source_member = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        .unwrap()
        .part_ids[0]
        .clone();
    let mirror_member = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        .unwrap()
        .part_ids[0]
        .clone();
    let source_layout = Layout {
        id: "source-layout".into(),
        name: "Source matrix layout".into(),
        board_id: context.source_board_id.clone(),
        matrix_ids: vec![
            context
                .matrix_pair
                .as_ref()
                .unwrap()
                .source_id
                .as_str()
                .to_owned(),
        ],
        part_ids: vec![source_member],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout".into(),
        name: "Target matrix layout".into(),
        board_id: context.mirror_board_id.clone(),
        matrix_ids: vec![
            context
                .matrix_pair
                .as_ref()
                .unwrap()
                .mirror_id
                .as_str()
                .to_owned(),
        ],
        part_ids: vec![mirror_member],
        mirror_link: None,
    };
    let unrelated_layout = Layout {
        id: "unrelated-layout".into(),
        name: "Unrelated nonempty layout".into(),
        board_id: context.mirror_board_id.clone(),
        matrix_ids: vec![unrelated_matrix.id.clone()],
        part_ids: vec![unrelated_member.clone()],
        mirror_link: None,
    };
    before.layouts = vec![source_layout, mirror_layout, unrelated_layout.clone()];
    let unrelated_envelope = OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "unrelated-layout-envelope".into(),
        part_ids: vec![unrelated_member.clone()],
        margin: 1.5,
        operation: boardstudio_model::model::Operation::Add,
    };
    before.outline.push(unrelated_envelope.clone());
    before
        .boards
        .iter_mut()
        .find(|board| board.id == context.mirror_board_id)
        .unwrap()
        .outline_ids
        .push("unrelated-layout-envelope".into());
    let source_net_id = format!(
        "matrix/{}/net/empty",
        context.matrix_pair.as_ref().unwrap().source_id.as_str()
    );
    let mirror_net_id = format!(
        "matrix/{}/net/empty",
        context.matrix_pair.as_ref().unwrap().mirror_id.as_str()
    );
    for (board_id, net_id) in [
        (context.source_board_id.as_str(), source_net_id.as_str()),
        (context.mirror_board_id.as_str(), mirror_net_id.as_str()),
    ] {
        before
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .net_ids
            .push(net_id.into());
        before.nets.push(Net {
            id: net_id.into(),
            name: "unused matrix net".into(),
            pins: vec![],
        });
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("the pre-edit pair is a valid materialized project");

    let mut post_source = before.clone();
    let source_changed = crate::support::apply_for_test(
        &mut post_source,
        &[],
        &EditOperation::RemoveMatrix {
            id: context
                .matrix_pair
                .as_ref()
                .unwrap()
                .source_id
                .as_str()
                .to_owned(),
        },
    )
    .expect("the ordinary source RemoveMatrix operation succeeds");
    assert!(
        source_changed
            .iter()
            .any(|id| id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
    );
    assert!(
        !post_source
            .matrices
            .iter()
            .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
    );

    let source_members: BTreeSet<_> = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        .expect("the source matrix exists in the pre-edit baseline")
        .part_ids
        .iter()
        .cloned()
        .collect();
    let mirror_members: BTreeSet<_> = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        .expect("the mirror matrix exists in the pre-edit baseline")
        .part_ids
        .iter()
        .cloned()
        .collect();
    let source_identity_map: BTreeMap<_, _> = source_members
        .iter()
        .cloned()
        .map(|id| (id, None))
        .collect();
    let mirror_identity_map: BTreeMap<_, _> = mirror_members
        .iter()
        .cloned()
        .map(|id| (id, None))
        .collect();
    let mut final_owners = context.owners.clone();
    for id in source_members.iter().chain(&mirror_members) {
        final_owners.insert(id.clone(), FinalOwner::Absent);
    }
    let explicit_removal_closure: BTreeSet<_> = source_members
        .iter()
        .chain(&mirror_members)
        .cloned()
        .collect();

    let incomplete_closure: BTreeSet<_> =
        explicit_removal_closure.iter().skip(1).cloned().collect();
    assert!(
        prepare_membership_transition_kernel(MembershipTransitionInput {
            dispatch: crate::support::test_dispatch(),
            valid_before: &before,
            post_source: &post_source,
            context: &context,
            baseline_kind: BaselineKind::MaterializedCurrentSchema,
            source_identity_map: &source_identity_map,
            mirror_identity_map: &mirror_identity_map,
            planned_additions: &[],
            final_owners: &final_owners,
            explicit_removal_closure: &incomplete_closure,
        })
        .is_err()
    );
    assert!(
        !post_source
            .matrices
            .iter()
            .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
    );

    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_removal_closure,
    })
    .expect("removing the source matrix projects its paired mirror removal");

    assert!(
        !output
            .draft
            .matrices
            .iter()
            .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
    );
    assert!(
        !output
            .draft
            .matrices
            .iter()
            .any(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
    );
    assert!(
        output
            .draft
            .parts
            .iter()
            .all(|part| !mirror_members.contains(&part.id))
    );
    assert_eq!(
        output
            .draft
            .matrices
            .iter()
            .find(|matrix| matrix.id == unrelated_matrix.id),
        Some(&unrelated_matrix)
    );
    assert_eq!(
        output
            .draft
            .layouts
            .iter()
            .find(|layout| layout.id == unrelated_layout.id),
        Some(&unrelated_layout)
    );
    assert!(
        !output
            .draft
            .layouts
            .iter()
            .any(|layout| layout.id == "mirror-layout")
    );
    assert!(!output.draft.nets.iter().any(|net| net.id == mirror_net_id));
    let retained_net = output
        .draft
        .nets
        .iter()
        .find(|net| net.id == "retained-second-key-net")
        .expect("the nonempty unrelated net survives target cleanup");
    assert_eq!(
        retained_net.pins,
        vec![Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into(),
        }]
    );
    assert!(
        !output
            .draft
            .constraints
            .iter()
            .any(|constraint| { constraint.id() == "retained-second-key-offset" })
    );
    let mut expected_retained_envelope = post_source
        .outline
        .iter()
        .find(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { id, .. }
                if id == "retained-keys-envelope")
        })
        .expect("the source operation leaves the mirror envelope in its preimage")
        .clone();
    if let OutlineFeature::PartEnvelope { part_ids, .. } = &mut expected_retained_envelope {
        part_ids.retain(|id| !mirror_members.contains(id));
    }
    assert!(output.draft.outline.contains(&expected_retained_envelope));
    assert!(output.draft.outline.contains(&unrelated_envelope));
    assert!(
        output
            .draft
            .boards
            .iter()
            .any(|board| board.part_ids.contains(&unrelated_member))
    );
    assert!(
        output
            .changed_document_ids
            .iter()
            .any(|id| id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
    );
    assert!(
        output
            .changed_document_ids
            .contains(&"mirror-layout".to_owned())
    );
    assert!(output.changed_document_ids.contains(&mirror_net_id));
    for id in &mirror_members {
        assert!(output.changed_document_ids.contains(id));
    }
    let expected_target_delta: BTreeSet<_> = mirror_members
        .iter()
        .cloned()
        .chain([
            context
                .matrix_pair
                .as_ref()
                .unwrap()
                .mirror_id
                .as_str()
                .to_owned(),
            "mirror-layout".into(),
            mirror_net_id.clone(),
            context.mirror_board_id.clone(),
            "retained-keys-envelope".into(),
            "retained-second-key-net".into(),
            "retained-second-key-offset".into(),
        ])
        .collect();
    assert_eq!(
        output.changed_document_ids,
        expected_target_delta.into_iter().collect::<Vec<_>>()
    );
    assert!(
        source_members
            .iter()
            .all(|id| !output.changed_document_ids.contains(id))
    );
    assert!(output.context.associations.iter().all(|association| {
        association.source_id != context.matrix_pair.as_ref().unwrap().source_id.as_str()
            && association.mirror_id != context.matrix_pair.as_ref().unwrap().mirror_id.as_str()
            && !source_members.contains(&association.source_id)
            && !mirror_members.contains(&association.mirror_id)
    }));
    assert_eq!(output.final_owners, final_owners);
    assert_eq!(
        output.context.pair_right.correspondences, context.pair_right.correspondences,
        "the private retirement leaves persisted pair provenance byte-for-byte intact"
    );
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &output.draft,
        post_source: &output.draft,
        context: &output.context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &BTreeMap::new(),
        mirror_identity_map: &BTreeMap::new(),
        planned_additions: &[],
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the retired matrix transition converges in its updated context");
    assert_eq!(repeated.draft, output.draft);
    assert!(repeated.changed_document_ids.is_empty());
    assert_eq!(repeated.context, output.context);
}

#[test]
fn membership_transition_retires_a_whole_pair_but_preserves_local_mirror_parts() {
    let (before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the paired matrix fixture is valid");
    let source_members: BTreeSet<_> = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().source_id.as_str())
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    let mirror_members: BTreeSet<_> = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    let local_association = context
        .associations
        .iter_mut()
        .find(|association| {
            association.kind == MirroredItemKind::Part
                && association.source_id == "matrix/source-matrix/r0c3"
        })
        .expect("the selected key has a paired Part association");
    local_association.state = TransitionState::Local;
    local_association.explicit_state = true;
    let local_source_id = local_association.source_id.clone();
    let local_mirror_id = local_association.mirror_id.clone();
    let local_child_source_id = format!("{local_source_id}/diode");
    let local_child_mirror_id = format!("{local_mirror_id}/diode");
    let local_child = context
        .associations
        .iter_mut()
        .find(|association| association.source_id == local_child_source_id)
        .expect("the Local key has a materialized companion association");
    local_child.state = TransitionState::Local;
    local_child.explicit_state = false;
    let retained_parts = BTreeSet::from([local_mirror_id.clone(), local_child_mirror_id.clone()]);

    let mut before = before;
    before.nets.push(Net {
        id: "retained-local-net".into(),
        name: "retained Local PadID net".into(),
        pins: vec![Pin {
            part_id: local_child_mirror_id.clone(),
            pad_id: "pad-two".into(),
        }],
    });
    before
        .boards
        .iter_mut()
        .find(|board| board.id == context.mirror_board_id)
        .unwrap()
        .net_ids
        .push("retained-local-net".into());
    before.constraints.push(Constraint::Offset {
        id: "retained-local-constraint".into(),
        source_part_id: local_mirror_id.clone(),
        target_part_id: local_child_mirror_id.clone(),
        offset: Vec2 { x: 1.25, y: -2.5 },
        rotation: 7.0,
    });
    before.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: boardstudio_model::model::OutlineSettings::default(),
        id: "retained-local-outline".into(),
        part_ids: vec![local_mirror_id.clone(), local_child_mirror_id.clone()],
        margin: 1.75,
        operation: boardstudio_model::model::Operation::Add,
    });
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("the retained Local dependency controls are valid before removal");

    let mut post_source = before.clone();
    crate::support::apply_for_test(
        &mut post_source,
        &[],
        &EditOperation::RemoveMatrix {
            id: context
                .matrix_pair
                .as_ref()
                .unwrap()
                .source_id
                .as_str()
                .to_owned(),
        },
    )
    .expect("the source operation removes its matrix through Core");
    let source_identity_map: BTreeMap<_, _> = source_members
        .iter()
        .cloned()
        .map(|id| (id, None))
        .collect();
    let mirror_identity_map: BTreeMap<_, _> = mirror_members
        .iter()
        .cloned()
        .map(|id| {
            let mapped = retained_parts.contains(&id).then(|| id.clone());
            (id, mapped)
        })
        .collect();
    let mut final_owners = context.owners.clone();
    for id in &source_members {
        final_owners.insert(id.clone(), FinalOwner::Absent);
    }
    for id in &mirror_members {
        final_owners.insert(
            id.clone(),
            if retained_parts.contains(id) {
                FinalOwner::Board(context.mirror_board_id.clone())
            } else {
                FinalOwner::Absent
            },
        );
    }
    let explicit_removal_closure: BTreeSet<_> = source_members
        .iter()
        .chain(
            mirror_members
                .iter()
                .filter(|id| !retained_parts.contains(*id)),
        )
        .cloned()
        .collect();
    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_removal_closure,
    })
    .expect("a Local mirror identity survives paired matrix retirement");

    assert!(!output.draft.matrices.iter().any(|matrix| matrix.id
        == context.matrix_pair.as_ref().unwrap().source_id.as_str()
        || matrix.id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str()));
    assert!(
        !output
            .draft
            .parts
            .iter()
            .any(|part| part.id == local_source_id)
    );
    assert_eq!(
        output
            .draft
            .parts
            .iter()
            .find(|part| part.id == local_mirror_id),
        before.parts.iter().find(|part| part.id == local_mirror_id)
    );
    assert_eq!(
        output
            .draft
            .parts
            .iter()
            .find(|part| part.id == local_child_mirror_id),
        before
            .parts
            .iter()
            .find(|part| part.id == local_child_mirror_id)
    );
    assert!(
        output
            .draft
            .boards
            .iter()
            .find(|board| board.id == context.mirror_board_id)
            .unwrap()
            .part_ids
            .contains(&local_mirror_id)
    );
    assert!(
        output
            .draft
            .boards
            .iter()
            .find(|board| board.id == context.mirror_board_id)
            .unwrap()
            .part_ids
            .contains(&local_child_mirror_id)
    );
    assert!(output.draft.nets.iter().any(|net| {
        net.id == "retained-local-net"
            && net
                .pins
                .iter()
                .any(|pin| pin.part_id == local_child_mirror_id && pin.pad_id == "pad-two")
    }));
    assert!(output.draft.constraints.iter().any(|constraint| {
        constraint.id() == "retained-local-constraint"
            && constraint.source() == local_mirror_id
            && constraint.target() == local_child_mirror_id
    }));
    assert!(output.draft.outline.iter().any(|feature| {
        feature.id() == "retained-local-outline"
            && matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.contains(&local_mirror_id) && part_ids.contains(&local_child_mirror_id))
    }));
    assert!(output.context.associations.iter().any(|association| {
        association.source_id == local_source_id
            && association.mirror_id == local_mirror_id
            && association.state == TransitionState::Local
    }));
    assert_eq!(output.final_owners, final_owners);
    assert!(
        output
            .changed_document_ids
            .iter()
            .any(|id| id == context.matrix_pair.as_ref().unwrap().mirror_id.as_str())
    );
    assert!(!output.changed_document_ids.contains(&local_mirror_id));
    assert!(!output.changed_document_ids.contains(&local_child_mirror_id));

    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &output.draft,
        post_source: &output.draft,
        context: &output.context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &BTreeMap::new(),
        mirror_identity_map: &BTreeMap::new(),
        planned_additions: &[],
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("retained Local ownership converges after matrix retirement");
    assert_eq!(repeated.draft, output.draft);
    assert!(repeated.changed_document_ids.is_empty());
    assert_eq!(repeated.context, output.context);
}

#[test]
fn membership_transition_rejects_incomplete_and_aliased_maps_before_staging() {
    let (before, post_source, context, source_map, final_owners, closure) =
        membership_compaction_fixture().expect("the valid transition fixture is constructed");
    let mirror_map: BTreeMap<_, _> = source_map
        .iter()
        .map(|(old, new)| {
            let old_mirror = old.replace("matrix/source-matrix/", "matrix/mirror-matrix/");
            let new_mirror = new
                .as_ref()
                .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
            (old_mirror, new_mirror)
        })
        .collect();
    let original_before = before.clone();
    let original_post = post_source.clone();
    let original_context = context.clone();

    let mut incomplete = source_map.clone();
    incomplete.pop_first();
    let error = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &incomplete,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    })
    .expect_err("an incomplete historical source map is refused");
    assert!(
        error.contains("does not cover every old matrix member"),
        "{error}"
    );

    let mut aliased = source_map.clone();
    let mut survivors = aliased
        .iter()
        .filter_map(|(old, new)| new.as_ref().map(|_| old.clone()));
    let first = survivors.next().unwrap();
    let second = survivors.next().unwrap();
    let target = aliased[&first].clone();
    aliased.insert(second, target);
    let error = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &aliased,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    })
    .expect_err("an ambiguous survivor map is refused");
    assert!(error.contains("not injective"), "{error}");

    let mut non_finite_context = context.clone();
    let association = non_finite_context
        .associations
        .iter_mut()
        .find(|association| association.kind == MirroredItemKind::Part)
        .unwrap();
    association.raw_tweak = Some(MirroredCompanionTweak {
        x: f64::NAN,
        y: 0.0,
        rotation: 0.0,
    });
    association.effective_mode = MirroredCompanionMode::LiteralMirrorWithTweak;
    association.effective_tweak = association.raw_tweak;
    let error = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &non_finite_context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    })
    .expect_err("non-finite captured settings are refused");
    assert!(error.contains("invalid settings"), "{error}");

    let mut stale_before = before.clone();
    stale_before.hardware.as_mut().unwrap().pairs[0]
        .mapping
        .at
        .x += 1.0;
    let error = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &stale_before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    })
    .expect_err("a stale pre-edit pair binding is refused");
    assert!(
        error.contains("changed outside its private context"),
        "{error}"
    );

    let mut constrained_before = before.clone();
    let constraint = Constraint::Offset {
        id: "surviving-linked-placement".into(),
        source_part_id: "geometry-probe".into(),
        target_part_id: "matrix/mirror-matrix/r0c0/diode".into(),
        offset: Vec2 { x: 99.0, y: -35.0 },
        rotation: 90.0,
    };
    constrained_before.constraints.push(constraint.clone());
    let mut constrained_post_source = post_source.clone();
    constrained_post_source.constraints.push(constraint);
    let error = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &constrained_before,
        post_source: &constrained_post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    })
    .expect_err("a surviving linked constraint is refused atomically");
    assert!(error.contains("constrained geometry"), "{error}");
    assert!(
        constrained_before
            .constraints
            .iter()
            .any(|constraint| constraint.id() == "surviving-linked-placement")
    );
    assert_eq!(before, original_before);
    assert_eq!(post_source, original_post);
    assert_eq!(context, original_context);
}

#[test]
fn document_diff_reports_idless_nested_binding_paths() {
    let (mut before, _) = fixed_geometry_fixture().expect("the fixture is valid");
    let mut keymap = boardstudio_core_mechanical::keymap::KeymapConfiguration::default();
    keymap.layers[0].bindings.insert(
        "matrix/mirror-matrix/r0c0".into(),
        boardstudio_core_mechanical::keymap::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    );
    before.keymap = Some(keymap.clone());
    let mut after = before.clone();
    keymap.layers[0].bindings.insert(
        "matrix/mirror-matrix/r0c0".into(),
        boardstudio_core_mechanical::keymap::KeyBinding::KeyPress {
            keycode: "B".into(),
        },
    );
    after.keymap = Some(keymap);
    let (ids, paths) = changed_document_fields(&before, &after).unwrap();
    assert!(ids.is_empty());
    assert_eq!(paths.len(), 1);
    assert!(paths[0].contains("keymap"));
    assert!(paths[0].contains("matrix/mirror-matrix/r0c0"));
}
