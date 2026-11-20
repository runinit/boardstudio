#[path = "support/base.rs"]
pub mod base_fixtures;
#[path = "support/reconcile.rs"]
pub mod reconcile_fixtures;
pub mod support;

use base_fixtures::*;
use boardstudio_core_mirrored_half::mirrored_half::reconcile::*;
use boardstudio_model::model::*;
use reconcile_fixtures::*;
use std::collections::{BTreeMap, BTreeSet};
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

#[test]
fn membership_transition_expands_with_planned_pair_identities_and_references() {
    let (before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the linked matrix fixture is valid");
    for (source_id, explicit_state) in [
        ("matrix/source-matrix/r1c1", true),
        ("matrix/source-matrix/r1c1/diode", false),
    ] {
        let association = context
            .associations
            .iter_mut()
            .find(|association| association.source_id == source_id)
            .unwrap();
        association.state = TransitionState::Local;
        association.explicit_state = explicit_state;
    }
    let mut post_source = before.clone();
    let source_matrix = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mut expanded = source_matrix.clone();
    expanded.columns = 5;
    for row in 0..2 {
        expanded.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column: 4,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode".into(),
                offset: Vec2 { x: 2.0, y: 3.0 },
                rotation: Some(30.0),
                side: Some(Side::Back),
            }],
            assemblies_local: None,
        });
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut post_source, &expanded)
        .expect("the planned source additions materialize");

    let source_identity_map: BTreeMap<_, _> = source_matrix
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_identity_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mut additions = Vec::new();
    for row in 0..2 {
        let source_key = format!("matrix/source-matrix/r{row}c4");
        let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
        for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
            (
                source_key.clone(),
                mirror_key.clone(),
                None,
                None,
                format!("EXP_M_R{row}C4"),
            ),
            (
                format!("{source_key}/diode"),
                format!("{mirror_key}/diode"),
                Some(source_key.clone()),
                Some(mirror_key.clone()),
                format!("EXP_M_D_R{row}C4"),
            ),
        ] {
            let source_part = post_source
                .parts
                .iter()
                .find(|part| part.id == source_id)
                .unwrap();
            additions.push(PlannedAddition {
                source_id,
                mirror_id,
                source_reference: source_part.reference.clone(),
                mirror_reference,
                parent_source_id,
                parent_mirror_id,
            });
        }
    }
    let mut final_owners: BTreeMap<_, _> = post_source
        .parts
        .iter()
        .map(|part| {
            let board = post_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .unwrap();
            (part.id.clone(), FinalOwner::Board(board.id.clone()))
        })
        .collect();
    for addition in &additions {
        final_owners.insert(
            addition.mirror_id.clone(),
            FinalOwner::Board("mirror-board".into()),
        );
    }

    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("a complete planned expansion is projected atomically");
    let mirror = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert_eq!((mirror.rows, mirror.columns), (2, 5));
    assert_eq!(mirror.part_ids.len(), 20);
    for addition in &additions {
        assert_eq!(
            output
                .draft
                .parts
                .iter()
                .find(|part| part.id == addition.mirror_id)
                .unwrap()
                .reference,
            addition.mirror_reference
        );
    }
    assert_eq!(output.context_delta.added.len(), 4);
    assert_eq!(output.context.associations.len(), 21);
    assert_eq!(output.context.pair_right, context.pair_right);
}

#[test]
fn membership_transition_chains_two_by_three_expansion_and_two_by_two_trim() {
    let assert_pose = |document: &ProjectDoc, id: &str, expected: Pose2| {
        let actual = document
            .parts
            .iter()
            .find(|part| part.id == id)
            .unwrap()
            .pose;
        assert!(
            (actual.at.x - expected.at.x).abs() < 1e-9,
            "{id}: {actual:?}"
        );
        assert!(
            (actual.at.y - expected.at.y).abs() < 1e-9,
            "{id}: {actual:?}"
        );
        assert!(
            (actual.rotation - expected.rotation).abs() < 1e-9,
            "{id}: {actual:?}"
        );
    };
    let (mut before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the linked matrix fixture is valid");
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut before,
        "source-matrix",
        3,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .expect("the source fixture narrows to three columns");
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut before,
        "mirror-matrix",
        3,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .expect("the mirror fixture narrows to three columns");
    let mut initial_source = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    initial_source.origin = Vec2 { x: 3.0, y: -2.0 };
    initial_source.edge_gap = Some(Vec2 { x: 4.25, y: 6.5 });
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &initial_source)
        .expect("the initial source frame materializes at the approved 2x3 origin");
    let matrix_association = context
        .pair_right
        .correspondences
        .iter()
        .find(|association| association.kind == MirroredItemKind::Matrix)
        .unwrap();
    let projected_initial = project_matrix(
        before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap(),
        before
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap(),
        matrix_association,
        context.mapping,
    )
    .expect("the initial mirror frame is derived from the source frame");
    let mut initial_mirror = projected_initial;
    assert_eq!(
        initial_mirror.edge_gap,
        Some(Vec2 { x: 4.25, y: 6.5 }),
        "matrix projection preserves a nondefault edge gap"
    );
    for (row, column, x, y) in [(0, 1, -93.0, 62.0), (1, 1, -113.0, 42.0)] {
        let cell = initial_mirror
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap();
        cell.offset = Some(Vec2 { x, y });
        cell.rotation = Some(-90.0);
        let diode = cell
            .assemblies
            .iter_mut()
            .find(|assembly| assembly.id == "diode")
            .unwrap();
        diode.offset = Vec2 { x: 2.0, y: 3.0 };
        diode.rotation = Some(30.0);
        diode.side = Some(Side::Back);
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &initial_mirror)
        .expect("both Local keys and diode children are materialized independently");
    let mirror_members_now = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .clone();
    before.nets.push(boardstudio_model::model::Net {
        id: "net-clean".into(),
        name: "PROBE OMITTED LINKED".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r0c2/diode".into(),
                pad_id: "pad-one".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    for (id, part_id, pad_id) in [
        ("net-local-r0", "matrix/mirror-matrix/r0c1/diode", "pad-one"),
        ("net-local-r1", "matrix/mirror-matrix/r1c1/diode", "pad-two"),
    ] {
        before.nets.push(boardstudio_model::model::Net {
            id: id.into(),
            name: format!("LOCAL DEPENDENCY {id}"),
            pins: vec![boardstudio_model::model::Pin {
                part_id: part_id.into(),
                pad_id: pad_id.into(),
            }],
        });
    }
    before
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .net_ids
        .extend([
            "net-clean".into(),
            "net-local-r0".into(),
            "net-local-r1".into(),
        ]);
    before
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "constraint-omit".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c2/diode".into(),
            offset: Vec2 { x: 95.0, y: -57.0 },
            rotation: 60.0,
        });
    for (id, target_part_id, offset) in [
        (
            "constraint-local-r0",
            "matrix/mirror-matrix/r0c1/diode",
            Vec2 { x: 38.0, y: 51.0 },
        ),
        (
            "constraint-local-r1",
            "matrix/mirror-matrix/r1c1/diode",
            Vec2 { x: 38.0, y: 71.0 },
        ),
    ] {
        before
            .constraints
            .push(boardstudio_model::model::Constraint::Offset {
                id: id.into(),
                source_part_id: "geometry-probe".into(),
                target_part_id: target_part_id.into(),
                offset,
                rotation: 30.0,
            });
    }
    let mut outline_members: Vec<_> = mirror_members_now.into_iter().collect();
    outline_members.push("geometry-probe".into());
    before
        .outline
        .push(boardstudio_model::model::OutlineFeature::PartEnvelope {
            id: "outline-mirror".into(),
            connections: vec![
                OutlineConnection {
                    id: "local-r0-connection".into(),
                    width: 1.25,
                    points: vec![OutlineControlPoint {
                        at: Vec2 { x: 2.0, y: 3.0 },
                        part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                    }],
                },
                OutlineConnection {
                    id: "local-r1-connection".into(),
                    width: 1.5,
                    points: vec![OutlineControlPoint {
                        at: Vec2 { x: 2.0, y: 3.0 },
                        part_id: Some("matrix/mirror-matrix/r1c1/diode".into()),
                    }],
                },
            ],
            settings: boardstudio_model::model::OutlineSettings {
                corners: boardstudio_model::model::CornerStyle::Fillet,
                size: 3.5,
                bridge_width: 7.25,
                repair: Some(boardstudio_model::model::OutlineRepairSettings {
                    enabled: true,
                    maximum_gap_span: 12.0,
                    minimum_connection_width: 1.25,
                    edge_clearance: 0.75,
                    keep_gaps: vec![boardstudio_model::model::ProtectedOutlineGap {
                        id: "local-r0-keep-gap".into(),
                        points: vec![OutlineControlPoint {
                            at: Vec2 { x: 1.5, y: 2.5 },
                            part_id: Some("matrix/mirror-matrix/r0c1/diode".into()),
                        }],
                    }],
                }),
            },
            part_ids: outline_members,
            margin: 1.75,
            operation: boardstudio_model::model::Operation::Add,
        });
    before
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("outline-mirror".into());
    before.outline.retain(|feature| {
        !matches!(feature, boardstudio_model::model::OutlineFeature::PartEnvelope { id, .. } if id == "retained-keys-envelope")
    });
    before
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .retain(|id| id != "retained-keys-envelope");
    if let Some(boardstudio_model::model::Constraint::Offset { offset, .. }) = before.constraints.iter_mut()
        .find(|constraint| matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset"))
    {
        *offset = Vec2 { x: 36.0, y: 68.0 };
    }
    let source_members = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .part_ids
        .clone();
    let mirror_members = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .clone();
    let halves = before.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.retain(|association| {
        association.kind == MirroredItemKind::Matrix
            || source_members.contains(&association.source_id)
    });
    context.associations.retain(|association| {
        association.kind == MirroredItemKind::Matrix
            || source_members.contains(&association.source_id)
                && mirror_members.contains(&association.mirror_id)
    });
    context
        .owners
        .retain(|id, _| before.parts.iter().any(|part| part.id == *id));
    context.pair_right = halves.clone();
    for (source_id, state) in [
        ("matrix/source-matrix/r0c1", TransitionState::Local),
        ("matrix/source-matrix/r0c1/diode", TransitionState::Local),
        ("matrix/source-matrix/r1c1", TransitionState::Local),
        ("matrix/source-matrix/r1c1/diode", TransitionState::Local),
        ("matrix/source-matrix/r0c2", TransitionState::Omitted),
        ("matrix/source-matrix/r0c2/diode", TransitionState::Omitted),
    ] {
        let association = context
            .associations
            .iter_mut()
            .find(|association| association.source_id == source_id)
            .expect("the three-column context contains each state");
        association.state = state;
        association.explicit_state = !source_id.ends_with("/diode");
    }
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());

    let local_ids = BTreeSet::from([
        "matrix/mirror-matrix/r0c1/diode".to_owned(),
        "matrix/mirror-matrix/r1c1/diode".to_owned(),
    ]);
    let mut local_keymap = KeymapConfiguration::default();
    local_keymap.layers[0].bindings.insert(
        "matrix/mirror-matrix/r0c1/diode".into(),
        KeyBinding::KeyPress {
            keycode: "LOCAL-R0".into(),
        },
    );
    local_keymap.layers[0].sensors.insert(
        "matrix/mirror-matrix/r1c1/diode".into(),
        EncoderBinding {
            clockwise: KeyBinding::KeyPress {
                keycode: "LOCAL-R1-CW".into(),
            },
            counterclockwise: KeyBinding::KeyPress {
                keycode: "LOCAL-R1-CCW".into(),
            },
        },
    );
    before.keymap = Some(local_keymap);
    let mut local_keycaps = KeycapConfiguration::default();
    local_keycaps.keys.insert(
        "matrix/mirror-matrix/r0c1/diode".into(),
        KeycapKeySettings {
            legend: Some("LOCAL R0".into()),
            ..KeycapKeySettings::default()
        },
    );
    local_keycaps.keys.insert(
        "matrix/mirror-matrix/r1c1/diode".into(),
        KeycapKeySettings {
            legend: Some("LOCAL R1".into()),
            ..KeycapKeySettings::default()
        },
    );
    before.keycaps = Some(local_keycaps);
    let local_part_payloads: BTreeMap<_, _> = before
        .parts
        .iter()
        .filter(|part| local_ids.contains(&part.id))
        .map(|part| (part.id.clone(), part.clone()))
        .collect();
    let local_net_pins: BTreeMap<_, _> = before
        .nets
        .iter()
        .filter(|net| net.id == "net-local-r0" || net.id == "net-local-r1")
        .map(|net| (net.id.clone(), net.pins.clone()))
        .collect();
    let local_constraints: Vec<_> = before
        .constraints
        .iter()
        .filter(|constraint| {
            local_ids.contains(constraint.source()) || local_ids.contains(constraint.target())
        })
        .cloned()
        .collect();
    let (local_connections, local_outline_settings) = match before
        .outline
        .iter()
        .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, .. } if id == "outline-mirror"))
        .unwrap()
    {
        OutlineFeature::PartEnvelope { connections, settings, .. } => {
            (connections.clone(), settings.clone())
        }
        _ => unreachable!(),
    };
    let local_bindings: BTreeMap<_, _> = before.keymap.as_ref().unwrap().layers[0]
        .bindings
        .iter()
        .filter(|(id, _)| local_ids.contains(*id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect();
    let local_sensors: BTreeMap<_, _> = before.keymap.as_ref().unwrap().layers[0]
        .sensors
        .iter()
        .filter(|(id, _)| local_ids.contains(*id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect();
    let local_keycap_settings: BTreeMap<_, _> = before
        .keycaps
        .as_ref()
        .unwrap()
        .keys
        .iter()
        .filter(|(id, _)| local_ids.contains(*id))
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect();
    let assert_local_retention = |document: &ProjectDoc, phase: &str| {
        for (id, expected) in &local_part_payloads {
            let actual: Vec<_> = document
                .parts
                .iter()
                .filter(|part| part.id == *id)
                .collect();
            assert_eq!(
                actual,
                vec![expected],
                "{phase}: full Local Part payload {id}"
            );
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.part_ids.contains(id))
                    .count(),
                1,
                "{phase}: Local board registration {id}"
            );
            assert_eq!(
                document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == "mirror-matrix")
                    .unwrap()
                    .part_ids
                    .iter()
                    .filter(|member| *member == id)
                    .count(),
                1,
                "{phase}: Local matrix registration {id}"
            );
        }
        let actual_net_pins: BTreeMap<_, _> = document
            .nets
            .iter()
            .filter(|net| net.id == "net-local-r0" || net.id == "net-local-r1")
            .map(|net| (net.id.clone(), net.pins.clone()))
            .collect();
        assert_eq!(
            actual_net_pins, local_net_pins,
            "{phase}: Local dependency nets"
        );
        let actual_constraints: Vec<_> = document
            .constraints
            .iter()
            .filter(|constraint| {
                local_ids.contains(constraint.source()) || local_ids.contains(constraint.target())
            })
            .cloned()
            .collect();
        assert_eq!(
            actual_constraints, local_constraints,
            "{phase}: Local constraints"
        );
        let (actual_connections, actual_settings) = match document
            .outline
            .iter()
            .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { id, .. } if id == "outline-mirror"))
            .unwrap()
        {
            OutlineFeature::PartEnvelope { connections, settings, .. } => (connections, settings),
            _ => unreachable!(),
        };
        assert_eq!(
            actual_connections, &local_connections,
            "{phase}: Local outline controls"
        );
        assert_eq!(
            actual_settings, &local_outline_settings,
            "{phase}: nondefault outline settings and Local keep-gap reference"
        );
        let actual_bindings: BTreeMap<_, _> = document.keymap.as_ref().unwrap().layers[0]
            .bindings
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        assert_eq!(
            actual_bindings, local_bindings,
            "{phase}: Local key bindings"
        );
        let actual_sensors: BTreeMap<_, _> = document.keymap.as_ref().unwrap().layers[0]
            .sensors
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        assert_eq!(
            actual_sensors, local_sensors,
            "{phase}: Local sensor bindings"
        );
        let actual_keycaps: BTreeMap<_, _> = document
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .iter()
            .filter(|(id, _)| local_ids.contains(*id))
            .map(|(id, value)| (id.clone(), value.clone()))
            .collect();
        assert_eq!(
            actual_keycaps, local_keycap_settings,
            "{phase}: Local keycap settings"
        );
    };
    assert_local_retention(&before, "before");

    let source_before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mirror_before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let mut expanded_source = source_before.clone();
    expanded_source.columns = 4;
    expanded_source.origin = Vec2 { x: 5.0, y: 1.0 };
    for row in 0..2 {
        expanded_source.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column: 3,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode".into(),
                offset: Vec2 { x: 2.0, y: 3.0 },
                rotation: Some(30.0),
                side: Some(Side::Back),
            }],
            assemblies_local: None,
        });
    }
    let mut expanded_source_document = before.clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut expanded_source_document, &expanded_source)
        .expect("the post-source expansion is materialized");
    let source_identity_map: BTreeMap<_, _> = source_before
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_identity_map: BTreeMap<_, _> = mirror_before
        .part_ids
        .iter()
        .map(|id| {
            let omitted =
                id == "matrix/mirror-matrix/r0c2" || id == "matrix/mirror-matrix/r0c2/diode";
            (id.clone(), (!omitted).then(|| id.clone()))
        })
        .collect();
    let mut additions = Vec::new();
    for row in 0..2 {
        let source_key = format!("matrix/source-matrix/r{row}c3");
        let mirror_key = format!("matrix/mirror-matrix/r{row}c3");
        for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
            (
                source_key.clone(),
                mirror_key.clone(),
                None,
                None,
                format!("CHAIN_M_R{row}C3"),
            ),
            (
                format!("{source_key}/diode"),
                format!("{mirror_key}/diode"),
                Some(source_key.clone()),
                Some(mirror_key.clone()),
                format!("CHAIN_M_D_R{row}C3"),
            ),
        ] {
            let source_part = expanded_source_document
                .parts
                .iter()
                .find(|part| part.id == source_id)
                .unwrap();
            additions.push(PlannedAddition {
                source_id,
                mirror_id,
                source_reference: source_part.reference.clone(),
                mirror_reference,
                parent_source_id,
                parent_mirror_id,
            });
        }
    }
    let omitted_key_id = "matrix/mirror-matrix/r0c2";
    let omitted_mirror_id = "matrix/mirror-matrix/r0c2/diode";
    let mut expanded_owners: BTreeMap<_, _> = expanded_source_document
        .parts
        .iter()
        .map(|part| {
            let owner = expanded_source_document
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .unwrap();
            (part.id.clone(), FinalOwner::Board(owner.id.clone()))
        })
        .collect();
    for addition in &additions {
        expanded_owners.insert(
            addition.mirror_id.clone(),
            FinalOwner::Board("mirror-board".into()),
        );
    }
    expanded_owners.insert(omitted_key_id.into(), FinalOwner::Absent);
    expanded_owners.insert(omitted_mirror_id.into(), FinalOwner::Absent);
    let expanded = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &expanded_source_document,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &expanded_owners,
        explicit_removal_closure: &BTreeSet::from([omitted_key_id.into(), omitted_mirror_id.into()]),
    })
    .expect("the approved expansion removes the initial Omitted companion and adds four planned identities");
    assert_local_retention(&expanded.draft, "expanded");
    let cleaned_net = expanded
        .draft
        .nets
        .iter()
        .find(|net| net.id == "net-clean")
        .unwrap();
    assert_eq!(
        cleaned_net.pins,
        vec![
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into()
            },
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r1c2/diode".into(),
                pad_id: "pad-two".into()
            },
        ]
    );
    assert!(expanded.context.associations.iter().any(|association| {
        association.state == TransitionState::Omitted && association.mirror_id == omitted_mirror_id
    }));
    assert!(!expanded.draft.constraints.iter().any(|constraint| {
        matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "constraint-omit")
    }));
    let repeat_source_map: BTreeMap<_, _> = expanded
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeat_mirror_map: BTreeMap<_, _> = expanded
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &expanded.draft,
        post_source: &expanded.draft,
        context: &expanded.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &repeat_source_map,
        mirror_identity_map: &repeat_mirror_map,
        planned_additions: &[],
        final_owners: &expanded.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the exact private expansion state repeats without changes");
    assert_local_retention(&repeated.draft, "repeated expansion");
    assert_eq!(repeated.draft, expanded.draft);
    assert_eq!(repeated.context, expanded.context);
    assert!(repeated.changed_document_ids.is_empty());
    assert!(repeated.changed_field_paths.is_empty());

    let mut shrink_before = expanded.draft.clone();
    shrink_before
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "constraint-linked".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r1c2/diode".into(),
            offset: Vec2 { x: 72.0, y: -59.0 },
            rotation: 60.0,
        });
    let probe = shrink_before
        .parts
        .iter()
        .find(|part| part.id == "geometry-probe")
        .unwrap();
    let linked_endpoint = shrink_before
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r1c2/diode")
        .unwrap();
    assert_eq!(
        probe.pose,
        Pose2 {
            at: Vec2 { x: 4.0, y: 2.0 },
            rotation: 0.0
        }
    );
    assert_pose(
        &shrink_before,
        "matrix/mirror-matrix/r1c2/diode",
        Pose2 {
            at: Vec2 { x: 76.0, y: -57.0 },
            rotation: 60.0,
        },
    );
    assert!((linked_endpoint.pose.at.x - probe.pose.at.x - 72.0).abs() < 1e-9);
    assert!((linked_endpoint.pose.at.y - probe.pose.at.y + 59.0).abs() < 1e-9);

    let mut trimmed_source = shrink_before.clone();
    let source_matrix = shrink_before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mut trimmed = source_matrix.clone();
    trimmed.columns = 2;
    trimmed.cells.retain(|cell| cell.column < 2);
    boardstudio_core_kernel::matrix::set_matrix(&mut trimmed_source, &trimmed)
        .expect("the post-source trim is materialized");
    let source_identity_map: BTreeMap<_, _> = source_matrix
        .part_ids
        .iter()
        .map(|id| {
            let (_, column, _) =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                    id,
                    "source-matrix",
                )
                .unwrap();
            (id.clone(), (column < 2).then(|| id.clone()))
        })
        .collect();
    let mirror_matrix = shrink_before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let mirror_identity_map: BTreeMap<_, _> = mirror_matrix
        .part_ids
        .iter()
        .map(|id| {
            let (_, column, _) =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                    id,
                    "mirror-matrix",
                )
                .unwrap();
            (id.clone(), (column < 2).then(|| id.clone()))
        })
        .collect();
    let mut removal_closure = BTreeSet::new();
    for (id, mapped) in &source_identity_map {
        if mapped.is_none() {
            removal_closure.insert(id.clone());
            removal_closure.insert(id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
        }
    }
    removal_closure.insert(omitted_mirror_id.into());
    let mut final_owners = BTreeMap::new();
    for part in &shrink_before.parts {
        let removed = source_identity_map.get(&part.id) == Some(&None)
            || mirror_identity_map.get(&part.id) == Some(&None)
            || part.id == omitted_mirror_id;
        let owner = if removed {
            FinalOwner::Absent
        } else {
            let board = trimmed_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id));
            board
                .map(|board| FinalOwner::Board(board.id.clone()))
                .unwrap_or(FinalOwner::Absent)
        };
        final_owners.insert(part.id.clone(), owner);
    }
    for id in expanded.context.owners.keys() {
        final_owners.entry(id.clone()).or_insert(FinalOwner::Absent);
    }
    let trimmed = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &shrink_before,
        post_source: &trimmed_source,
        context: &expanded.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &removal_closure,
    })
    .expect("the private expansion draft and full context chain into the two-column trim");
    assert_local_retention(&trimmed.draft, "trimmed");
    let final_mirror = trimmed
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert_eq!((final_mirror.rows, final_mirror.columns), (2, 2));
    assert_eq!(final_mirror.part_ids.len(), 8);
    assert_eq!(
        trimmed
            .draft
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/source-matrix/"))
            .count(),
        8
    );
    assert_eq!(
        trimmed
            .draft
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/mirror-matrix/"))
            .count(),
        8
    );
    assert_eq!(trimmed.context_delta.retired.len(), 8);
    let expected_shrink_ids: BTreeSet<_> = [
        "constraint-linked",
        "matrix/mirror-matrix/r0c3",
        "matrix/mirror-matrix/r0c3/diode",
        "matrix/mirror-matrix/r1c2",
        "matrix/mirror-matrix/r1c2/diode",
        "matrix/mirror-matrix/r1c3",
        "matrix/mirror-matrix/r1c3/diode",
        "mirror-board",
        "mirror-matrix",
        "net-clean",
        "outline-mirror",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(
        trimmed
            .changed_document_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected_shrink_ids
    );
    assert_eq!(trimmed.changed_document_ids.len(), 11);
    assert_eq!(
        trimmed
            .final_owners
            .keys()
            .filter(|id| id.starts_with("matrix/source-matrix/")
                || id.starts_with("matrix/mirror-matrix/"))
            .count(),
        32
    );
    assert_eq!(trimmed.context.associations.len(), 9);
    assert_eq!(
        trimmed
            .context
            .associations
            .iter()
            .filter(|association| association.state == TransitionState::Linked)
            .count(),
        5
    );
    assert_eq!(
        trimmed
            .context
            .associations
            .iter()
            .filter(|association| association.state == TransitionState::Local)
            .count(),
        4
    );
    assert_eq!(
        trimmed
            .draft
            .nets
            .iter()
            .find(|net| net.id == "net-clean")
            .unwrap()
            .pins,
        vec![boardstudio_model::model::Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into()
        },]
    );
    for id in [
        "matrix/mirror-matrix/r0c1/diode",
        "matrix/mirror-matrix/r1c1/diode",
    ] {
        assert!(trimmed.draft.parts.iter().any(|part| part.id == id));
        assert!(trimmed.context.associations.iter().any(|association| {
            association.mirror_id == id && association.state == TransitionState::Local
        }));
    }
    for id in [
        "matrix/source-matrix/r0c1/diode",
        "matrix/source-matrix/r1c1/diode",
    ] {
        let association = trimmed
            .context
            .associations
            .iter()
            .find(|association| association.source_id == id)
            .unwrap();
        assert_eq!(association.state, TransitionState::Local);
        assert!(!association.explicit_state, "{id} remains inherited");
    }
    let retired_omitted_child = trimmed
        .context_delta
        .retired
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c2/diode")
        .unwrap();
    assert_eq!(retired_omitted_child.state, TransitionState::Omitted);
    assert!(!retired_omitted_child.explicit_state);
    assert_eq!(trimmed.context.pair_right, context.pair_right);
    let final_source = trimmed
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let repeat_source_map: BTreeMap<_, _> = final_source
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeat_mirror_map: BTreeMap<_, _> = final_mirror
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &trimmed.draft,
        post_source: &trimmed.draft,
        context: &trimmed.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &repeat_source_map,
        mirror_identity_map: &repeat_mirror_map,
        planned_additions: &[],
        final_owners: &trimmed.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the exact post-shrink draft and context repeat without changes");
    assert_local_retention(&repeated.draft, "repeated trim");
    assert_eq!(repeated.draft, trimmed.draft);
    assert_eq!(repeated.context, trimmed.context);
    assert!(repeated.changed_document_ids.is_empty());
    assert!(repeated.changed_field_paths.is_empty());
}

#[test]
fn membership_transition_trims_only_explicitly_removed_linked_closures() {
    let (mut before, _, mut context, _, _, _) =
        membership_compaction_fixture().expect("the linked matrix fixture is valid");
    for (source_id, state) in [
        ("matrix/source-matrix/r0c0/diode", TransitionState::Local),
        ("matrix/source-matrix/r1c1", TransitionState::Local),
        ("matrix/source-matrix/r0c3", TransitionState::Local),
        ("matrix/source-matrix/r0c2", TransitionState::Omitted),
    ] {
        let mirror_id = source_id.replace("source-matrix", "mirror-matrix");
        for association in &mut context.associations {
            if (association.source_id == source_id && association.mirror_id == mirror_id)
                || (association.parent_source_id.as_deref() == Some(source_id)
                    && association.parent_mirror_id.as_deref() == Some(mirror_id.as_str()))
            {
                if association.explicit_state && association.source_id != source_id {
                    continue;
                }
                association.state = state;
                association.explicit_state = association.source_id == source_id;
                context.owners.insert(
                    association.mirror_id.clone(),
                    match state {
                        TransitionState::Local => {
                            FinalOwner::Board(context.mirror_board_id.clone())
                        }
                        TransitionState::Omitted => FinalOwner::Absent,
                        TransitionState::Linked => {
                            FinalOwner::Board(context.mirror_board_id.clone())
                        }
                    },
                );
            }
        }
        context.pair_right.exceptions.push(MirroredException {
            kind: MirroredItemKind::Part,
            mirror_id,
            state: match state {
                TransitionState::Local => MirroredExceptionState::Local,
                TransitionState::Omitted => MirroredExceptionState::Omitted,
                TransitionState::Linked => unreachable!(),
            },
            companion_mode: None,
            companion_tweak: None,
            removed_copper: None,
        });
    }
    before.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .clone_from(&context.pair_right);
    let mut sentinel_matrix = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    for (row, column, x, y, rotation) in [(0, 0, 7.25, -9.5, 121.0), (1, 1, -3.5, 11.25, -37.0)] {
        let cell = sentinel_matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap();
        let assembly = cell
            .assemblies
            .iter_mut()
            .find(|item| item.id == "diode")
            .unwrap();
        assembly.offset = Vec2 { x, y };
        assembly.rotation = Some(rotation);
        assembly.side = Some(Side::Front);
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &sentinel_matrix)
        .expect("Local child sentinels remain a valid matrix");
    let omitted_mirror_ids = [
        "matrix/mirror-matrix/r0c2",
        "matrix/mirror-matrix/r0c2/diode",
    ];
    before
        .parts
        .retain(|part| !omitted_mirror_ids.contains(&part.id.as_str()));
    before
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|part_id| !omitted_mirror_ids.contains(&part_id.as_str()));
    for omitted_mirror_id in omitted_mirror_ids {
        assert!(before.parts.iter().all(|part| part.id != omitted_mirror_id));
        assert!(before.boards.iter().all(|board| {
            !board
                .part_ids
                .iter()
                .any(|part_id| part_id == omitted_mirror_id)
        }));
    }
    assert_eq!(
        context
            .pair_right
            .exceptions
            .iter()
            .filter(|exception| {
                context.pair_right.overlay_source(exception) == Some("matrix/source-matrix/r0c2")
                    && exception.mirror_id == "matrix/mirror-matrix/r0c2"
                    && exception.state == MirroredExceptionState::Omitted
            })
            .count(),
        1,
        "the owner record alone expresses the inherited Omitted closure"
    );
    assert!(context.pair_right.exceptions.iter().all(|exception| {
        context.pair_right.overlay_source(exception) != Some("matrix/source-matrix/r0c2/diode")
    }));
    assert!(
        context.pair_right.correspondences.iter().any(|record| {
            record.kind == MirroredItemKind::Part
                && record.source_id == "matrix/source-matrix/r0c2/diode"
                && record.mirror_id == "matrix/mirror-matrix/r0c2/diode"
                && record.companion_mode == Some(MirroredCompanionMode::LiteralMirror)
        }),
        "the inherited child keeps its correspondence and per-item projection settings"
    );
    before.nets.push(Net {
        id: "retained-local-net".into(),
        name: "RETAINED LOCAL NET".into(),
        pins: vec![Pin {
            part_id: "matrix/mirror-matrix/r0c3/diode".into(),
            pad_id: "pad-two".into(),
        }],
    });
    before
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .net_ids
        .push("retained-local-net".into());
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("the exact parent Omitted association owns its absent child closure");
    let source_before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mirror_before = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    let mut post_source = before.clone();
    let mut trimmed = source_before.clone();
    trimmed.columns = 2;
    trimmed.cells.retain(|cell| cell.column < 2);
    boardstudio_core_kernel::matrix::set_matrix(&mut post_source, &trimmed)
        .expect("the source dimension trim is materialized");

    let mut source_identity_map = BTreeMap::new();
    for id in &source_before.part_ids {
        let (_, column, _) =
            boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                id,
                "source-matrix",
            )
            .unwrap();
        source_identity_map.insert(id.clone(), (column < 2).then(|| id.clone()));
    }
    let mirror_identity_map: BTreeMap<_, _> = mirror_before
        .part_ids
        .iter()
        .map(|id| {
            let (_, column, _) =
                boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                    id,
                    "mirror-matrix",
                )
                .unwrap();
            let locally_retained = id.starts_with("matrix/mirror-matrix/r0c3");
            let omitted = context.associations.iter().any(|association| {
                association.mirror_id == *id && association.state == TransitionState::Omitted
            });
            (
                id.clone(),
                ((column < 2 || locally_retained) && !omitted).then(|| id.clone()),
            )
        })
        .collect();
    let mut explicit_removal_closure = BTreeSet::new();
    for (source_id, target_id) in &source_identity_map {
        if target_id.is_none() {
            explicit_removal_closure.insert(source_id.clone());
            explicit_removal_closure
                .insert(source_id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
        }
    }
    let mut final_owners = context.owners.clone();
    for part in &before.parts {
        let locally_retained = context.associations.iter().any(|association| {
            association.state == TransitionState::Local
                && association.mirror_id == part.id
                && mirror_identity_map
                    .get(&part.id)
                    .is_some_and(Option::is_some)
        });
        let is_removed_matrix_part = (source_identity_map.get(&part.id) == Some(&None)
            || mirror_identity_map.get(&part.id) == Some(&None))
            && !locally_retained;
        let owner = if is_removed_matrix_part {
            FinalOwner::Absent
        } else {
            let board = post_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id));
            board
                .map(|board| FinalOwner::Board(board.id.clone()))
                .unwrap_or(FinalOwner::Absent)
        };
        final_owners.insert(part.id.clone(), owner);
    }

    let output = prepare_membership_transition(MembershipTransitionInput {
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
    .expect("the explicit trim removes the corresponding target closures");
    let mirror = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert_eq!((mirror.rows, mirror.columns), (2, 2));
    assert_eq!(mirror.part_ids.len(), 8);
    assert_eq!(output.context_delta.retired.len(), 8);
    assert_eq!(output.context.associations.len(), 9);
    for removed_source_id in [
        "matrix/source-matrix/r0c2",
        "matrix/source-matrix/r0c2/diode",
        "matrix/source-matrix/r0c3",
        "matrix/source-matrix/r0c3/diode",
    ] {
        assert!(
            output
                .context
                .pair_right
                .exceptions
                .iter()
                .all(|exception| {
                    exception.mirror_id
                        != removed_source_id.replace("source-matrix", "mirror-matrix")
                }),
            "source removal cleans only the exception for removed item {removed_source_id}"
        );
        assert!(
            output
                .context
                .pair_right
                .correspondences
                .iter()
                .all(|record| { record.source_id != removed_source_id })
        );
        assert!(
            output
                .context
                .associations
                .iter()
                .all(|association| { association.source_id != removed_source_id })
        );
    }
    for id in [
        "matrix/mirror-matrix/r0c3",
        "matrix/mirror-matrix/r0c3/diode",
    ] {
        assert!(
            output.draft.parts.iter().any(|part| part.id == id),
            "Local geometry {id} is retained"
        );
        assert!(
            output
                .draft
                .boards
                .iter()
                .find(|board| board.id == "mirror-board")
                .unwrap()
                .part_ids
                .contains(&id.to_owned())
        );
        assert!(
            !mirror.part_ids.contains(&id.to_owned()),
            "out-of-bounds Local geometry is detached from matrix membership"
        );
        assert_eq!(
            output.final_owners.get(id),
            Some(&FinalOwner::Board("mirror-board".into()))
        );
    }
    assert_eq!(
        output
            .draft
            .nets
            .iter()
            .find(|net| net.id == "retained-local-net")
            .unwrap()
            .pins,
        vec![Pin {
            part_id: "matrix/mirror-matrix/r0c3/diode".into(),
            pad_id: "pad-two".into(),
        }]
    );
    for row in 0..2 {
        for column in 0..2 {
            let id = format!("matrix/mirror-matrix/r{row}c{column}");
            let before_part = before.parts.iter().find(|part| part.id == id).unwrap();
            let after_part = output
                .draft
                .parts
                .iter()
                .find(|part| part.id == id)
                .unwrap();
            assert_eq!(after_part.properties, before_part.properties);
            assert_eq!(
                after_part.generator_parameters,
                before_part.generator_parameters
            );
        }
        for column in 2..4 {
            if row == 0 && column == 3 {
                continue;
            }
            for id in [
                format!("matrix/mirror-matrix/r{row}c{column}"),
                format!("matrix/mirror-matrix/r{row}c{column}/diode"),
            ] {
                assert!(!output.draft.parts.iter().any(|part| part.id == id));
                assert_eq!(output.final_owners.get(&id), Some(&FinalOwner::Absent));
            }
        }
    }
    for (row, column, x, y, rotation) in [(0, 0, 7.25, -9.5, 121.0), (1, 1, -3.5, 11.25, -37.0)] {
        let cell = mirror
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .unwrap();
        let assembly = cell
            .assemblies
            .iter()
            .find(|item| item.id == "diode")
            .unwrap();
        assert_eq!(assembly.offset, Vec2 { x, y });
        assert_eq!(assembly.rotation, Some(rotation));
        assert_eq!(assembly.side, Some(Side::Front));
        let id = format!("matrix/mirror-matrix/r{row}c{column}/diode");
        let before_part = before.parts.iter().find(|part| part.id == id).unwrap();
        let after_part = output
            .draft
            .parts
            .iter()
            .find(|part| part.id == id)
            .unwrap();
        assert_eq!(
            after_part, before_part,
            "Local child payload {id} is preserved"
        );
    }
    let round_tripped: ProjectDoc =
        serde_json::from_slice(&serde_json::to_vec(&output.draft).unwrap()).unwrap();
    assert_eq!(round_tripped, output.draft);
    assert_eq!(
        round_tripped.hardware.as_ref().unwrap().pairs[0]
            .halves
            .as_ref()
            .unwrap(),
        &output.context.pair_right
    );
    let repeated_source_map: BTreeMap<_, _> = round_tripped
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated_mirror_map: BTreeMap<_, _> = round_tripped
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &round_tripped,
        post_source: &round_tripped,
        context: &output.context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &repeated_source_map,
        mirror_identity_map: &repeated_mirror_map,
        planned_additions: &[],
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("source retirement converges after a document round trip");
    assert_eq!(repeated.draft, round_tripped);
    assert_eq!(repeated.context, output.context);
    assert!(repeated.changed_document_ids.is_empty());
    assert!(repeated.changed_field_paths.is_empty());
}

#[test]
fn membership_transition_compacts_ids_and_carries_logical_payload_and_settings() {
    let (mut before, _, context, identity_map, final_owners, explicit_closure) =
        membership_compaction_fixture().expect("the materialized compaction fixture is valid");
    add_membership_cleanup_sentinels(&mut before);
    let mut post_source = before.clone();
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut post_source,
        "source-matrix",
        1,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .expect("the source operation remaps populated dependency sentinels");
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("populated before evidence remains canonical");
    let mirror_identity_map: BTreeMap<_, _> = identity_map
        .iter()
        .map(|(old, new)| {
            let mirror_old = old.replace("matrix/source-matrix/", "matrix/mirror-matrix/");
            let mirror_new = new
                .as_ref()
                .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/"));
            (mirror_old, mirror_new)
        })
        .collect();
    let additions = [];
    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &explicit_closure,
    })
    .expect("the explicit axis-removal operation compacts the mirror");

    let removed = "matrix/mirror-matrix/r0c1/diode";
    let shifted = "matrix/mirror-matrix/r0c2/diode";
    assert_eq!(
        output.draft.keymap.as_ref().unwrap().layers[0]
            .bindings
            .get(removed),
        Some(&KeyBinding::KeyPress {
            keycode: "B".into()
        }),
        "the shifted identity carries its payload across the removed key"
    );
    assert_eq!(
        output.draft.keymap.as_ref().unwrap().layers[0]
            .sensors
            .get(removed)
            .unwrap()
            .clockwise,
        KeyBinding::KeyPress {
            keycode: "E".into()
        }
    );
    assert!(
        !output.draft.keymap.as_ref().unwrap().layers[0]
            .sensors
            .contains_key(shifted)
    );
    assert!(
        !output
            .draft
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .contains_key(shifted)
    );
    assert_eq!(
        output
            .draft
            .keycaps
            .as_ref()
            .unwrap()
            .keys
            .get(removed)
            .unwrap()
            .legend
            .as_deref(),
        Some("shifted sentinel")
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().instances[1].controller_part_id,
        None
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().boards[0]
            .locks
            .get(&format!("direct/{removed}"))
            .map(String::as_str),
        None
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().boards[0]
            .assignments
            .get(&format!("link/{removed}"))
            .map(String::as_str),
        Some("assignment sentinel")
    );
    assert!(
        !output.draft.hardware.as_ref().unwrap().boards[0]
            .key_bindings
            .contains_key(shifted)
    );
    assert_eq!(
        output.draft.hardware.as_ref().unwrap().boards[0]
            .key_bindings
            .get(removed)
            .map(String::as_str),
        Some("binding sentinel")
    );
    assert_eq!(output.draft.embedded_circuits[0].part_ids, vec![removed]);
    assert!(
        output
            .draft
            .modules
            .iter()
            .find(|module| module.id == "closure-module")
            .unwrap()
            .connection
            .is_none()
    );
    assert_eq!(
        output
            .draft
            .modules
            .iter()
            .find(|module| module.id == "shifted-module")
            .unwrap()
            .connection
            .as_ref()
            .unwrap()
            .host_connector_part_id,
        removed
    );
    let OutlineFeature::PartEnvelope {
        part_ids,
        connections,
        settings,
        ..
    } = output
        .draft
        .outline
        .iter()
        .find(|feature| feature.id() == "closure-envelope")
        .unwrap()
    else {
        panic!("the closure envelope remains a part envelope");
    };
    let current_members: BTreeSet<_> = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    assert_eq!(
        part_ids.iter().cloned().collect::<BTreeSet<_>>(),
        current_members
    );
    assert_eq!(connections[0].points[0].part_id, None);
    assert_eq!(connections[0].points[1].part_id.as_deref(), Some(removed));
    assert_eq!(
        settings.repair.as_ref().unwrap().keep_gaps[0].points[0].part_id,
        None
    );
    let outline = output
        .draft
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    let version = outline
        .versions
        .iter()
        .find(|version| version.id == "closure-version")
        .unwrap();
    assert!(
        matches!(&version.geometry.features[0], OutlineFeature::PartEnvelope { part_ids, connections, .. }
        if part_ids == &vec![removed.to_owned()]
            && connections[0].points[0].part_id.is_none()
            && connections[0].points[1].part_id.as_deref() == Some(removed))
    );
    assert_eq!(version.geometry.bridges[0].part_ids, vec![removed]);
    assert_eq!(
        version.geometry.protected_gaps[0].points[0]
            .part_id
            .as_deref(),
        Some(removed)
    );
    assert_eq!(
        outline.generated_last_valid.as_ref().unwrap().bridges[0].part_ids,
        vec![removed]
    );

    let mirror = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap();
    assert_eq!((mirror.rows, mirror.columns), (2, 3));
    assert_eq!(mirror.part_ids.len(), 12);
    let d_diode = output
        .draft
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c2/diode")
        .unwrap();
    assert_eq!(
        d_diode.pose,
        Pose2 {
            at: Vec2 { x: 98.0, y: -56.0 },
            rotation: 75.0
        }
    );
    assert_eq!(
        d_diode.properties.as_ref().unwrap()["logical-payload"],
        serde_json::json!("matrix/mirror-matrix/r0c3/diode")
    );
    let moved_diode = output
        .context
        .associations
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c2/diode")
        .unwrap();
    assert_eq!(moved_diode.mirror_id, "matrix/mirror-matrix/r0c2/diode");
    assert_eq!(
        moved_diode.raw_mode_override,
        Some(MirroredCompanionMode::LiteralMirrorWithTweak)
    );
    assert_eq!(
        moved_diode.raw_tweak,
        Some(MirroredCompanionTweak {
            x: 1.0,
            y: -2.0,
            rotation: 15.0
        })
    );
    let preserved_a = output
        .context
        .associations
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
        .unwrap();
    assert_eq!(preserved_a.raw_mode_override, None);
    assert_eq!(
        preserved_a.effective_mode,
        MirroredCompanionMode::LiteralMirror
    );
    let preserved_c = output
        .context
        .associations
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c1/diode")
        .unwrap();
    assert_eq!(
        preserved_c.raw_mode_override,
        Some(MirroredCompanionMode::LiteralMirror)
    );
    assert_eq!(preserved_c.raw_tweak, None);
    let retired_b = output
        .context_delta
        .retired
        .iter()
        .find(|association| association.source_id == "matrix/source-matrix/r0c1/diode")
        .unwrap();
    assert_eq!(
        retired_b.raw_mode_override,
        Some(MirroredCompanionMode::DuplicatePosition)
    );
    assert!(
        output
            .changed_document_ids
            .contains(&"mirror-board".to_owned())
    );
    let expected_changed_ids: BTreeSet<_> = std::iter::once("mirror-board".to_owned())
        .chain(std::iter::once("mirror-matrix".to_owned()))
        .chain([
            "closure-circuit".to_owned(),
            "closure-envelope".to_owned(),
            "closure-module".to_owned(),
            "retained-second-key-net".to_owned(),
            "retained-second-key-offset".to_owned(),
            "retained-keys-envelope".to_owned(),
            "shifted-module".to_owned(),
        ])
        .chain((0..2).flat_map(|row| {
            (1..4).flat_map(move |column| {
                let key = format!("matrix/mirror-matrix/r{row}c{column}");
                [key.clone(), format!("{key}/diode")]
            })
        }))
        .collect();
    assert_eq!(
        output
            .changed_document_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        expected_changed_ids
    );
    assert_eq!(output.context_delta.retired.len(), 4);
    assert_eq!(output.context_delta.relocated.len(), 8);
    assert_eq!(output.context.associations.len(), 13);
    assert_eq!(output.context_delta.owners, output.final_owners);
    assert_eq!(output.draft.definitions, post_source.definitions);
    assert_eq!(
        output
            .draft
            .parts
            .iter()
            .find(|part| part.id == "geometry-probe"),
        post_source
            .parts
            .iter()
            .find(|part| part.id == "geometry-probe")
    );
    let retained_net = output
        .draft
        .nets
        .iter()
        .find(|net| net.id == "retained-second-key-net")
        .unwrap();
    assert_eq!(retained_net.name, "RETAINED SECOND KEY");
    assert_eq!(
        retained_net.pins,
        vec![boardstudio_model::model::Pin {
            part_id: "geometry-probe".into(),
            pad_id: "pad-two".into(),
        }]
    );
    assert!(!output.draft.constraints.iter().any(|constraint| {
        matches!(constraint, boardstudio_model::model::Constraint::Offset { id, .. } if id == "retained-second-key-offset")
    }));
    for id in [
        "matrix/source-matrix/r0c3",
        "matrix/source-matrix/r0c3/diode",
        "matrix/source-matrix/r1c3",
        "matrix/source-matrix/r1c3/diode",
        "matrix/mirror-matrix/r0c3",
        "matrix/mirror-matrix/r0c3/diode",
        "matrix/mirror-matrix/r1c3",
        "matrix/mirror-matrix/r1c3/diode",
    ] {
        assert_eq!(
            output.final_owners.get(id),
            Some(&FinalOwner::Absent),
            "{id}"
        );
    }

    let remapped = output.context.clone();
    let next_map: BTreeMap<_, _> = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let next_mirror_map: BTreeMap<_, _> = output
        .draft
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let repeated = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &output.draft,
        post_source: &output.draft,
        context: &remapped,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &next_map,
        mirror_identity_map: &next_mirror_map,
        planned_additions: &additions,
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .expect("the chained private state is valid for an identity repeat");
    assert_eq!(repeated.draft, output.draft);
    assert_eq!(repeated.context, output.context);
    assert!(repeated.changed_document_ids.is_empty());
    assert!(repeated.changed_field_paths.is_empty());
}

#[test]
fn independent_transition_rejects_wrong_board_final_owner() {
    let (before, post_source, context, source_map, mut final_owners, closure) =
        membership_compaction_fixture().unwrap();
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let id = "matrix/mirror-matrix/r1c1/diode";
    final_owners.insert(id.into(), FinalOwner::Board("source-board".into()));
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    });
    assert!(
        result.is_err(),
        "a contradictory final owner for the second-row companion must be rejected"
    );
}

#[test]
fn independent_transition_rejects_inconsistent_raw_effective_settings() {
    let (before, post_source, mut context, source_map, final_owners, closure) =
        membership_compaction_fixture().unwrap();
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let child = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == "matrix/source-matrix/r0c3/diode")
        .unwrap();
    assert_eq!(
        child.raw_mode_override,
        Some(MirroredCompanionMode::LiteralMirrorWithTweak)
    );
    assert_eq!(
        child.raw_tweak,
        Some(MirroredCompanionTweak {
            x: 1.0,
            y: -2.0,
            rotation: 15.0
        })
    );
    child.effective_mode = MirroredCompanionMode::DuplicatePosition;
    child.effective_tweak = None;
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    });
    assert!(
        result.is_err(),
        "effective settings must agree with the preserved raw override and tweak"
    );
}

#[test]
fn membership_transition_rejects_missing_second_retained_child_owner_atomically() {
    let (before, post_source, context, source_map, mut final_owners, closure) =
        membership_compaction_fixture().expect("the materialized compaction fixture is valid");
    let missing = "matrix/mirror-matrix/r1c1/diode";
    assert!(final_owners.remove(missing).is_some());
    let before_snapshot = before.clone();
    let post_source_snapshot = post_source.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &source_map
            .iter()
            .map(|(old, new)| {
                (
                    old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                    new.as_ref()
                        .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
                )
            })
            .collect(),
        planned_additions: &[],
        final_owners: &final_owners,
        explicit_removal_closure: &closure,
    });
    assert!(result.is_err());
    assert_eq!(before, before_snapshot);
    assert_eq!(post_source, post_source_snapshot);
    assert_eq!(context, context_snapshot);
}

#[test]
fn repair_fresh_refuses_invalid_before_pad_identity() {
    let (mut before, _, context, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
    let net = before
        .nets
        .iter_mut()
        .find(|n| n.id == "retained-second-key-net")
        .unwrap();
    assert_eq!(net.pins[0].pad_id, "pad-one");
    net.pins[0].pad_id = "1".into();
    let part = before
        .parts
        .iter()
        .find(|p| p.id == net.pins[0].part_id)
        .unwrap();
    let definition = before
        .definitions
        .iter()
        .find(|d| d.id == part.definition_id)
        .unwrap();
    assert!(definition.pads.iter().any(|p| p.number == "1"));
    assert!(!definition.pads.iter().any(|p| p.id == "1"));
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    let source_map = before
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_map = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &context.owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot);
    assert_eq!(context, context_snapshot);
    assert!(
        result.is_err(),
        "a touched historical net referencing Pad.number instead of actual Pad.id must refuse before staging"
    );
}

#[test]
fn repair_fresh_refuses_chained_identity_ambiguous_actual_board_owner() {
    let (before, post, context, source_map, owners, closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let output = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
    .expect("establish an actually returned private draft and full context first");
    let mut invalid = output.draft.clone();
    let second_child = "matrix/mirror-matrix/r1c1/diode";
    invalid
        .boards
        .iter_mut()
        .find(|b| b.id == "source-board")
        .unwrap()
        .part_ids
        .push(second_child.into());
    assert_eq!(
        invalid
            .boards
            .iter()
            .filter(|b| b.part_ids.iter().any(|id| id == second_child))
            .count(),
        2
    );
    let source_map = invalid
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_map = invalid
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let snapshot = invalid.clone();
    let context_snapshot = output.context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &invalid,
        post_source: &invalid,
        context: &output.context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &output.final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(invalid, snapshot);
    assert_eq!(output.context, context_snapshot);
    assert!(
        result.is_err(),
        "a chained no-op must validate the second child has exactly one actual board owner"
    );
}

#[test]
fn repair_fresh_refuses_wrong_historical_owner_context() {
    let (before, post, mut context, source_map, owners, closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let second_child = "matrix/mirror-matrix/r1c1/diode";
    assert_eq!(
        context.owners[second_child],
        FinalOwner::Board("mirror-board".into())
    );
    context.owners.insert(
        second_child.into(),
        FinalOwner::Board("source-board".into()),
    );
    assert!(
        before
            .boards
            .iter()
            .find(|b| b.id == "mirror-board")
            .unwrap()
            .part_ids
            .iter()
            .any(|id| id == second_child)
    );
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let before_snapshot = before.clone();
    let post_snapshot = post.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    });
    assert_eq!(before, before_snapshot);
    assert_eq!(post, post_snapshot);
    assert_eq!(context, context_snapshot);
    assert!(
        result.is_err(),
        "the valid-before owner context must match the historical board registration, even when final owners are correct"
    );
}

#[test]
fn repair_fresh_omission_cleans_idless_dependencies() {
    let (mut before, _, mut context, source_map, mut owners, mut closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let parent = "matrix/source-matrix/r0c0";
    let target_parent = "matrix/mirror-matrix/r0c0";
    let target_child = "matrix/mirror-matrix/r0c0/diode";
    for association in context
        .associations
        .iter_mut()
        .filter(|a| a.source_id == parent || a.parent_source_id.as_deref() == Some(parent))
    {
        association.state = TransitionState::Omitted;
        association.explicit_state = association.parent_source_id.is_none();
    }
    let mut keymap = KeymapConfiguration::default();
    keymap.layers[0].bindings.insert(
        target_child.into(),
        KeyBinding::KeyPress {
            keycode: "OMITTED_SENTINEL".into(),
        },
    );
    before.keymap = Some(keymap);
    let mut post = before.clone();
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut post,
        "source-matrix",
        1,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )
    .unwrap();
    let mut mirror_map: BTreeMap<_, _> = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    for id in [target_parent, target_child] {
        mirror_map.insert(id.into(), None);
        owners.insert(id.into(), FinalOwner::Absent);
        closure.insert(id.into());
    }
    let output = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
    .expect("a complete explicit Omitted closure under the accepted source operation is supported");
    assert!(!output.draft.parts.iter().any(|p| p.id == target_child));
    assert_eq!(output.final_owners[target_child], FinalOwner::Absent);
    assert!(
        !output.draft.keymap.as_ref().unwrap().layers[0]
            .bindings
            .contains_key(target_child),
        "omitted geometry removal must clean populated keymap dependencies too"
    );
}

#[test]
fn repair_fresh_accepts_shifted_local_parent_with_explicit_omitted_child() {
    let (before, post, mut context, source_map, mut owners, mut closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let parent = "matrix/source-matrix/r1c2";
    let target_child = "matrix/mirror-matrix/r1c2/diode";
    let parent_record = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == parent)
        .unwrap();
    parent_record.state = TransitionState::Local;
    parent_record.explicit_state = true;
    let child = context
        .associations
        .iter_mut()
        .find(|a| a.parent_source_id.as_deref() == Some(parent))
        .unwrap();
    child.state = TransitionState::Omitted;
    child.explicit_state = true;
    assert_eq!(
        child.parent_mirror_id.as_deref(),
        Some("matrix/mirror-matrix/r1c2")
    );
    let mut mirror_map: BTreeMap<_, _> = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    mirror_map.insert(target_child.into(), None);
    owners.insert("matrix/mirror-matrix/r1c1/diode".into(), FinalOwner::Absent);
    closure.insert(target_child.into());
    let result = prepare_membership_transition_kernel(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    });
    assert!(
        result.is_ok(),
        "a valid explicit Omitted child override must survive its shifted Local parent compaction policy: {:?}",
        result.err()
    );
}

#[test]
fn repair_fresh_refuses_native_and_legacy_matrix_overlap() {
    let (mut before, _, mut context, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
    let mut third = before
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .clone();
    third.id = "legacy-third-matrix".into();
    third.part_ids.clear();
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &third).unwrap();
    before.layouts = vec![
        Layout {
            id: "legacy-source-layout".into(),
            name: "Legacy source".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["source-matrix".into()],
            part_ids: vec![],
            mirror_link: None,
        },
        Layout {
            id: "legacy-third-layout".into(),
            name: "Legacy third".into(),
            board_id: "source-board".into(),
            matrix_ids: vec!["legacy-third-matrix".into()],
            part_ids: vec![],
            mirror_link: Some(LayoutMirrorLink {
                correspondences: vec![MirroredCorrespondence {
                    kind: MirroredItemKind::Matrix,
                    source_id: "source-matrix".into(),
                    mirror_id: "legacy-third-matrix".into(),
                    companion_mode: None,
                    companion_tweak: None,
                }],
                exceptions: vec![],
                source_id: "legacy-source-layout".into(),
                axis_x: 10.0,
                split_gap: None,
                spacing: None,
            }),
        },
    ];
    before
        .constraints
        .retain(|constraint| constraint.id() != "retained-second-key-offset");
    boardstudio_core_kernel::matrix::layout::validate(&before)
        .expect("the legacy pair alone is valid on one board");
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)
        .expect("the native record alone is valid");
    for part in &before.parts {
        let board = before
            .boards
            .iter()
            .find(|b| b.part_ids.contains(&part.id))
            .unwrap();
        context
            .owners
            .insert(part.id.clone(), FinalOwner::Board(board.id.clone()));
    }
    let source_map = before
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_map = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &context.owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert!(
        result.is_err(),
        "a matrix with both valid native and same-board legacy links must refuse this transition"
    );
}

#[test]
fn repair_fresh_refuses_unbound_planned_source_reference() {
    let (
        before,
        post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        mut additions,
        final_owners,
    ) = membership_expansion_fixture();

    additions[0].source_reference = "SUPPLIED_BUT_NOT_ACTUAL_SOURCE_REFERENCE".into();

    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert!(
        result.is_err(),
        "planned source reference must bind the actual post-source Part"
    );
}

#[test]
fn repair_fresh_refuses_missing_planned_child_parent() {
    let (
        before,
        post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        mut additions,
        final_owners,
    ) = membership_expansion_fixture();

    additions[1].parent_source_id = Some("matrix/source-matrix/r0c99".into());
    additions[1].parent_mirror_id = Some("matrix/mirror-matrix/r0c99".into());

    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert!(
        result.is_err(),
        "planned child parent closure must exist and match before staging"
    );
}

#[test]
fn repair_fresh_refuses_duplicate_planned_mirror_references() {
    let (
        before,
        post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        mut additions,
        final_owners,
    ) = membership_expansion_fixture();

    additions[2].mirror_reference = additions[0].mirror_reference.clone();

    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert!(
        result.is_err(),
        "distinct planned additions cannot publish duplicate references"
    );
}

#[test]
fn repair_fresh_final_diff_reports_real_module_and_circuit_ids() {
    let (mut before, _, _, _, _, _) = repair_fresh_coherent_compaction_fixture().unwrap();
    add_membership_cleanup_sentinels(&mut before);
    let mut after = before.clone();
    after
        .modules
        .iter_mut()
        .find(|m| m.id == "shifted-module")
        .unwrap()
        .connection
        .as_mut()
        .unwrap()
        .host_connector_part_id = "matrix/mirror-matrix/r0c1/diode".into();
    after
        .embedded_circuits
        .iter_mut()
        .find(|c| c.id == "closure-circuit")
        .unwrap()
        .part_ids = vec!["matrix/mirror-matrix/r0c1/diode".into()];
    let (ids, paths) = changed_document_fields(&before, &after).unwrap();
    assert!(
        ids.contains(&"shifted-module".into()) && ids.contains(&"closure-circuit".into()),
        "changed dependency objects with actual IDs need real IDs, while idless maps use paths: IDs={ids:?}, paths={paths:?}"
    );
}

#[test]
fn repair_fresh_final_addition_copies_nondefault_source_parameters() {
    let (
        before,
        mut post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        additions,
        final_owners,
    ) = membership_expansion_fixture();

    let source_id = additions[0].source_id.clone();
    let mirror_id = additions[0].mirror_id.clone();
    let source = post_source
        .parts
        .iter_mut()
        .find(|p| p.id == source_id)
        .unwrap();
    source.properties = Some(BTreeMap::from([(
        "new-source-label".into(),
        serde_json::json!("ADDED SOURCE LABEL"),
    )]));
    source.generator_parameters = Some(BTreeMap::from([(
        "new-source-parameter".into(),
        serde_json::json!(17),
    )]));
    source.outline = Some(boardstudio_model::model::PartOutline {
        excluded: true,
        margin: Some(2.75),
        allow_body_overhang: true,
    });
    let source_snapshot = source.clone();
    assert!(!source_snapshot.properties.as_ref().unwrap().is_empty());
    assert!(
        !source_snapshot
            .generator_parameters
            .as_ref()
            .unwrap()
            .is_empty()
    );
    let output = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post_source,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_identity_map,
        mirror_identity_map: &mirror_identity_map,
        planned_additions: &additions,
        final_owners: &final_owners,
        explicit_removal_closure: &BTreeSet::new(),
    })
    .unwrap();
    let mirror = output
        .draft
        .parts
        .iter()
        .find(|p| p.id == mirror_id)
        .unwrap();
    assert_eq!(
        (
            &mirror.properties,
            &mirror.generator_parameters,
            &mirror.outline,
        ),
        (
            &source_snapshot.properties,
            &source_snapshot.generator_parameters,
            &source_snapshot.outline,
        ),
        "new linked materialized Part must copy nondefault labels, parameters, and PartOutline"
    );
}

#[test]
fn consolidated_fresh_refuses_missing_historical_constraint_endpoint() {
    let (mut before, _, context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();
    let Constraint::Offset { target_part_id, .. } = before
        .constraints
        .iter_mut()
        .find(|c| c.id() == "retained-second-key-offset")
        .unwrap()
    else {
        unreachable!()
    };
    *target_part_id = "missing-historical-endpoint".into();
    assert!(
        !before
            .parts
            .iter()
            .any(|p| p.id == "missing-historical-endpoint")
    );
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&before).is_ok());
    for matrix in &before.matrices {
        boardstudio_core_kernel::matrix::valid_matrix(matrix, &before).unwrap();
    }

    let mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "valid-before touched constraint endpoints must exist before identity success"
    );
}

#[test]
fn consolidated_fresh_refuses_missing_actual_child_parent_context() {
    let (before, _, mut context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();
    let child = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == "matrix/source-matrix/r1c3/diode")
        .unwrap();
    assert_eq!(
        child.parent_source_id.as_deref(),
        Some("matrix/source-matrix/r1c3")
    );
    assert_eq!(
        child.parent_mirror_id.as_deref(),
        Some("matrix/mirror-matrix/r1c3")
    );
    child.parent_source_id = None;
    child.parent_mirror_id = None;

    let mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "a canonical historical child cannot lose its actual parent closure"
    );
}

#[test]
fn consolidated_fresh_refuses_phantom_historical_association() {
    let (before, _, mut context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();
    let mut phantom = context
        .associations
        .iter()
        .find(|a| a.source_id == "matrix/source-matrix/r0c0")
        .unwrap()
        .clone();
    phantom.source_id = "matrix/source-matrix/r99c99".into();
    phantom.mirror_id = "matrix/mirror-matrix/r99c99".into();
    assert!(
        !before
            .parts
            .iter()
            .any(|p| p.id == phantom.source_id || p.id == phantom.mirror_id)
    );
    context.associations.push(phantom);

    let mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "historical association identities must bind actual members/owners, including extra records"
    );
}

#[test]
fn consolidated_fresh_refuses_duplicate_same_board_registration() {
    let (mut before, _, context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();
    let id = "matrix/mirror-matrix/r1c1/diode";
    assert_eq!(
        before
            .boards
            .iter()
            .flat_map(|b| &b.part_ids)
            .filter(|p| p.as_str() == id)
            .count(),
        1
    );
    before
        .boards
        .iter_mut()
        .find(|b| b.id == "mirror-board")
        .unwrap()
        .part_ids
        .push(id.into());
    assert_eq!(
        before
            .boards
            .iter()
            .flat_map(|b| &b.part_ids)
            .filter(|p| p.as_str() == id)
            .count(),
        2
    );

    let mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "Board final owner must have exactly one actual registration, even within one board"
    );
}

#[test]
fn consolidated_fresh_refuses_nonfinite_historical_local_pose() {
    let (mut before, _, context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();
    let part = before
        .parts
        .iter_mut()
        .find(|p| p.id == "matrix/mirror-matrix/r1c1/diode")
        .unwrap();
    part.pose.at.x = f64::INFINITY;
    assert!(!part.pose.at.x.is_finite());

    let mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "historical retained Local geometry must be finite before every success"
    );
}

/// The finite check is one `Constraint::is_finite`, so every kind a transition can meet
/// must be refused by it with the message that names the constraint.
#[test]
fn consolidated_fresh_names_a_nonfinite_constraint_of_every_kind() {
    for kind in ["offset", "part-offset", "mirror"] {
        for nonfinite in [f64::INFINITY, f64::NAN] {
            let (mut before, _, context, source_map, owners, _) =
                consolidated_fresh_identity_fixture().unwrap();
            let existing = before
                .constraints
                .iter_mut()
                .find(|c| c.id() == "retained-second-key-offset")
                .unwrap();
            let (id, source_part_id, target_part_id) = (
                existing.id().to_owned(),
                existing.source().to_owned(),
                existing.target().to_owned(),
            );
            *existing = match kind {
                "offset" => Constraint::Offset {
                    id,
                    source_part_id,
                    target_part_id,
                    offset: Vec2 {
                        x: nonfinite,
                        y: 1.0,
                    },
                    rotation: 0.0,
                },
                "part-offset" => Constraint::PartOffset {
                    id,
                    source_part_id,
                    target_part_id,
                    offset: Vec2 { x: 1.0, y: 1.0 },
                    rotation: nonfinite,
                },
                _ => Constraint::Mirror {
                    id,
                    source_part_id,
                    target_part_id,
                    axis: MirrorAxis::Vertical,
                    coordinate: nonfinite,
                },
            };
            let mirror_map: BTreeMap<_, _> = before
                .matrices
                .iter()
                .find(|m| m.id == "mirror-matrix")
                .unwrap()
                .part_ids
                .iter()
                .map(|id| (id.clone(), Some(id.clone())))
                .collect();
            let message = prepare_membership_transition(MembershipTransitionInput {
                dispatch: crate::support::test_dispatch(),
                valid_before: &before,
                post_source: &before,
                context: &context,
                baseline_kind: BaselineKind::MaterializedCurrentSchema,
                source_identity_map: &source_map,
                mirror_identity_map: &mirror_map,
                planned_additions: &[],
                final_owners: &owners,
                explicit_removal_closure: &BTreeSet::new(),
            })
            .map(|_| ())
            .unwrap_err();
            assert!(
                message.contains("retained-second-key-offset")
                    && message.contains("non-finite geometry"),
                "{kind} {nonfinite}: {message}"
            );
        }
    }
}

#[test]
fn consolidated_fresh_refuses_local_mirror_deletion_on_identity() {
    let (before, _, context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().unwrap();

    let mut mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    mirror_map.insert("matrix/mirror-matrix/r1c1/diode".into(), None);

    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );

    assert!(
        result.is_err(),
        "identity success cannot silently ignore a Local child's None mirror mapping"
    );
}

#[test]
fn consolidated_fresh_refuses_materialized_omitted_child_on_chained_identity() {
    let (before, post, context, source_map, owners, closure) =
        repair_fresh_coherent_compaction_fixture().unwrap();
    let mirror_map = source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect();
    let output = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &post,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &closure,
    })
    .expect("valid preceding compaction and private context control");
    let before = output.draft;
    let mut context = output.context;
    let owners = output.final_owners;
    let source_map = before
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let mut mirror_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|m| m.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();

    let child = context
        .associations
        .iter_mut()
        .find(|a| a.source_id == "matrix/source-matrix/r1c2/diode")
        .unwrap();
    child.state = TransitionState::Omitted;
    child.explicit_state = true;
    let id = child.mirror_id.clone();
    assert!(before.parts.iter().any(|p| p.id == id));
    assert_eq!(
        owners.get(&id),
        Some(&FinalOwner::Board("mirror-board".into()))
    );
    mirror_map.insert(id.clone(), None);
    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::ChainedPrivateDraft,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_eq!(before, snapshot);
    assert_eq!(context, context_snapshot);
    assert!(
        result.is_err(),
        "chained Omitted state must bind absent geometry/owner before no-op success"
    );
}

#[test]
fn durable_local_exception_is_atomic_and_repeats_through_parent_kernel() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let mirror_id = projection
        .locally_protected_mirror_parts
        .iter()
        .next()
        .unwrap()
        .clone();
    let selected = PartProjection {
        source_id: mirror_id.replace("mirror-matrix", "source-matrix"),
        mirror_id,
    };
    let other_retained = projection
        .locally_protected_mirror_parts
        .iter()
        .find(|id| **id != selected.mirror_id)
        .unwrap()
        .clone();
    let target = document
        .parts
        .iter_mut()
        .find(|part| part.id == selected.mirror_id)
        .unwrap();
    target.reference = "LOCAL-OWNER".into();
    target.pose = Pose2 {
        at: Vec2 { x: -23.5, y: 41.25 },
        rotation: 117.0,
    };
    target.properties.get_or_insert_with(BTreeMap::new).insert(
        "localOwnerPayload".into(),
        serde_json::json!({"revision": 9}),
    );
    let expected_local = target.clone();

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    for mirror_id in [&selected.mirror_id, &other_retained] {
        halves.exceptions.push(MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            mirror_id: mirror_id.clone(),
            state: MirroredExceptionState::Local,
            removed_copper: None,
        });
    }

    // This is a positive parent-kernel control: the selected Local owner is already present
    // in the validated retained-member partition. The child transition regressions separately
    // prove construction of that partition from the durable source/mirror exception.
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the durable Local exception and its matrix item are valid");
    let serialized = serde_json::to_vec(&document).unwrap();
    document = serde_json::from_slice(&serialized).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the Local association and visible geometry survive document round-trip");
    let pair_before = pair_state_bytes(&document);
    let independent_context_before = projection.applied_independent_additions.borrow().clone();
    let hosted_context_before = projection.applied_hosted_operations.borrow().clone();
    let circuit_context_before = projection.applied_hosted_circuit_additions.borrow().clone();
    let hosted_removal_before = projection.hosted_removal_baselines.borrow().clone();
    let hosted_omission_before = projection.hosted_omission_baselines.borrow().clone();
    let outline_removal_before = projection.outline_removal_baselines.borrow().clone();

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the exact durable Local owner state reconciles successfully");
    let first_output = document.clone();
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == selected.mirror_id),
        Some(&expected_local),
        "Local owns the complete target payload while the linked matrix path runs"
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    let repeated = reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the same complete owner state is repeatable");
    assert!(
        repeated.is_empty(),
        "repeat reconciliation reports no changed IDs"
    );
    assert_eq!(document, first_output);
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        *projection.applied_independent_additions.borrow(),
        independent_context_before
    );
    assert_eq!(
        *projection.applied_hosted_operations.borrow(),
        hosted_context_before
    );
    assert_eq!(
        *projection.applied_hosted_circuit_additions.borrow(),
        circuit_context_before
    );
    assert_eq!(
        *projection.hosted_removal_baselines.borrow(),
        hosted_removal_before
    );
    assert_eq!(
        *projection.hosted_omission_baselines.borrow(),
        hosted_omission_before
    );
    assert_eq!(
        *projection.outline_removal_baselines.borrow(),
        outline_removal_before
    );

    let source_matrix_before = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    let linked_before: BTreeMap<_, _> = document
        .parts
        .iter()
        .filter(|part| {
            part.id.starts_with("matrix/mirror-matrix/")
                && part.id != selected.mirror_id
                && part.id != other_retained
        })
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let local_matrix_cell = selected
        .source_id
        .strip_prefix("matrix/source-matrix/")
        .unwrap();
    let (local_row, local_column) = local_matrix_cell.split_once('c').unwrap();
    let local_row = local_row.strip_prefix('r').unwrap().parse::<u32>().unwrap();
    let local_column = local_column.parse::<u32>().unwrap();
    let local_slot_before = source_matrix_before
        .cells
        .iter()
        .find(|cell| cell.row == local_row && cell.column == local_column)
        .unwrap();
    assert!(local_slot_before.enabled);

    let mut edited_source_matrix = source_matrix_before;
    edited_source_matrix.origin.x += 7.0;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &edited_source_matrix)
        .expect("an ordinary Core matrix edit materializes before reconciliation");
    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("the Local exception remains valid during a linked source matrix edit");
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == selected.mirror_id),
        Some(&expected_local),
        "the Local target keeps its visible geometry after the ordinary source edit"
    );
    assert!(
        linked_before.iter().any(|(id, pose)| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .is_some_and(|part| part.pose != *pose)
        }),
        "unselected linked matrix members still follow the ordinary source edit"
    );

    let mut invalid_projection = projection.clone();
    invalid_projection.final_owners.remove(&selected.mirror_id);
    let mut invalid_document = first_output.clone();
    let invalid_document_before = invalid_document.clone();
    let invalid_context_before = invalid_projection
        .applied_hosted_operations
        .borrow()
        .clone();
    assert!(
        reconcile(
            &mut invalid_document,
            "pair-source-mirror",
            &invalid_projection
        )
        .is_err(),
        "an incomplete owner partition is refused"
    );
    assert_eq!(invalid_document, invalid_document_before);
    assert_eq!(
        *invalid_projection.applied_hosted_operations.borrow(),
        invalid_context_before
    );
}

#[test]
fn durable_omitted_independent_part_stays_absent_through_parent_kernel() {
    let (mut document, projection) = fixed_geometry_fixture().unwrap();
    let mirror_id = "matrix/mirror-matrix/r0c1";
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .retain(|id| id != mirror_id);

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    document.parts.retain(|part| part.id != mirror_id);
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: mirror_id.into(),
        state: MirroredExceptionState::Omitted,
        removed_copper: None,
    });
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the source-bound Omitted target may be physically absent");
    let serialized = serde_json::to_vec(&document).unwrap();
    document = serde_json::from_slice(&serialized).unwrap();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("the source-bound Omitted absence survives document round-trip");
    let pair_before = pair_state_bytes(&document);
    let linked_before: BTreeMap<_, _> = document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("matrix/mirror-matrix/") && part.id != mirror_id)
        .map(|part| (part.id.clone(), part.pose))
        .collect();
    let mut source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    source_matrix.origin.x += 5.0;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix)
        .expect("an ordinary Core matrix edit changes the omitted item's source");

    reconcile(&mut document, "pair-source-mirror", &projection)
        .expect("a repeated reconciliation leaves the selected Omitted item absent");
    assert!(document.parts.iter().all(|part| part.id != mirror_id));
    assert!(
        document
            .boards
            .iter()
            .all(|board| !board.part_ids.iter().any(|id| id == mirror_id))
    );
    assert!(
        linked_before.iter().any(|(id, pose)| {
            document
                .parts
                .iter()
                .find(|part| part.id == *id)
                .is_some_and(|part| part.pose != *pose)
        }),
        "linked matrix members continue to follow the ordinary source edit"
    );
    let first_output = document.clone();
    assert!(
        reconcile(&mut document, "pair-source-mirror", &projection)
            .unwrap()
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(pair_state_bytes(&document), pair_before);

    let mut invalid_projection = projection.clone();
    invalid_projection
        .final_owners
        .insert(mirror_id.into(), "source-board".into());
    let mut invalid_document = document.clone();
    assert!(
        reconcile(
            &mut invalid_document,
            "pair-source-mirror",
            &invalid_projection
        )
        .is_err(),
        "Omitted absence does not waive the exact final board owner"
    );
    assert_eq!(invalid_document, document);
}
