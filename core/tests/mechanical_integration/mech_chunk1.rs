use super::*;

#[test]
fn an_explicit_pair_supports_distinct_left_and_right_pcb_designs() {
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("left-board"));
    document.boards.push(board("right-board"));
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            mirror_instance_on("left", "left-board", vec![mount("mount-1", 2.0, 3.0)]),
            mirror_instance_on("right", "right-board", vec![mount("mount-1", 7.0, 3.0)]),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });

    // The pair map is explicit; it is never guessed from names, coordinates or
    // flip flags. Distinct boards are a supported explicit pair.
    let changed = boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "left-board".into(),
            },
            change: MechanicalChange::LinkMirrorPair {
                pair_id: "pair-left-right".into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction_source: "left".into(),
                correspondences: vec![MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
                }],
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert_eq!(changed, vec!["pair-left-right", "left", "right"]);
    assert_eq!(
        document.hardware.as_ref().unwrap().pairs.len(),
        1,
        "a cross-board pair is accepted"
    );

    let left: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_eq!(left.board_id, "left-board");
    assert_eq!(right.board_id, "right-board");
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 2.0, y: 3.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -2.0, y: 3.0 }
    );
}

#[test]
fn a_settings_only_edit_validates_a_supplied_mounting_hole_template() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::model::PartGenerator;
    use std::collections::BTreeMap;

    let document = document();
    let mut wrong = template();
    // A definition from another source is not the mounting-hole catalogue entry.
    wrong.generator = Some(PartGenerator {
        source: "ceoloide/switch_mx".into(),
        version: "bundled-1".into(),
        parameters: BTreeMap::new(),
    });

    let mut rejected = document.clone();
    let result = apply(
        &mut rejected,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::SetDimension {
                field: MechanicalDimension::PlateThickness,
                value: 1.8,
            },
            mounting_hole: Some(Box::new(wrong)),
        },
    );
    assert!(
        result.is_err(),
        "a settings-only edit must still validate a supplied template"
    );
    assert_eq!(rejected, document, "a rejected template mutates nothing");

    // The same settings-only edit with the normalized entry succeeds.
    let mut accepted = document.clone();
    apply(
        &mut accepted,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::SetDimension {
                field: MechanicalDimension::PlateThickness,
                value: 1.8,
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert_eq!(accepted.mechanical.as_ref().unwrap().plate_thickness, 1.8);
}

/// A pair records its construction source, and closure proposals come from that source even
/// when it is the second member (ADR-0008; generator-parameters ticket 06). The proposals are
/// authored by the recorded source. The source stores no private list, and the other half keeps
/// whatever closure mounts it already stored: linking never deletes hardware a half carries, so
/// anything the user wants merged is linked through a reviewed correspondence instead.

#[test]
fn closure_proposals_come_from_the_recorded_construction_source() {
    use boardstudio_core::mechanical::edit::apply;

    let (mut document, accepted) = two_unconfigured_halves();
    apply(&mut document, &accepted, &enable_edit("a")).unwrap();
    let proposed = stored_closure_mounts(&document, "a").expect("the first half proposes mounts");
    assert!(!proposed.is_empty());
    let link = MechanicalEdit {
        target: instance_target("a"),
        change: MechanicalChange::LinkMirrorPair {
            pair_id: "p".into(),
            members: ["a".into(), "b".into()],
            mapping: Pose2 {
                at: Vec2 { x: 200.0, y: 0.0 },
                rotation: 0.0,
            },
            construction_source: "b".into(),
            correspondences: vec![],
        },
        mounting_hole: Some(Box::new(template())),
    };
    apply(&mut document, &accepted, &link).unwrap();
    apply(&mut document, &accepted, &enable_edit("b")).unwrap();

    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    assert_eq!(
        pair.construction_source.as_deref(),
        Some("b"),
        "the link records the construction source"
    );
    let closures: Vec<_> = pair
        .features
        .iter()
        .filter(|feature| feature.kind == MechanicalFeatureKind::ClosureMount)
        .collect();
    assert!(!closures.is_empty(), "the pair links closure proposals");
    assert!(
        closures.iter().all(|feature| feature.author == "b"),
        "authored by the recorded construction source, not by the first member"
    );
    assert_eq!(
        stored_closure_mounts(&document, "b").map(|mounts| mounts.len()),
        Some(0),
        "the source stores no private closure mount"
    );
    assert_eq!(
        stored_closure_mounts(&document, "a"),
        Some(proposed),
        "the other half keeps the closure mounts it already stored"
    );
}

/// A pair without a recorded construction source (every pair linked before format 5) keeps the
/// first member as its source, so existing projects propose exactly what they did before.

#[test]
fn closure_proposals_fall_back_to_the_first_member_without_a_recorded_source() {
    use boardstudio_core::mechanical::edit::apply;

    let (mut document, accepted) = two_unconfigured_halves();
    apply(&mut document, &accepted, &enable_edit("a")).unwrap();
    let proposed = stored_closure_mounts(&document, "a").expect("the first half proposes mounts");
    let link = MechanicalEdit {
        target: instance_target("a"),
        change: MechanicalChange::LinkMirrorPair {
            pair_id: "p".into(),
            members: ["a".into(), "b".into()],
            mapping: Pose2 {
                at: Vec2 { x: 200.0, y: 0.0 },
                rotation: 0.0,
            },
            construction_source: "b".into(),
            correspondences: vec![],
        },
        mounting_hole: Some(Box::new(template())),
    };
    apply(&mut document, &accepted, &link).unwrap();
    document.hardware.as_mut().unwrap().pairs[0].construction_source = None;
    apply(&mut document, &accepted, &enable_edit("b")).unwrap();

    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    let closures: Vec<_> = pair
        .features
        .iter()
        .filter(|feature| feature.kind == MechanicalFeatureKind::ClosureMount)
        .collect();
    assert_eq!(
        closures.len(),
        proposed.len(),
        "the pair links the proposals once"
    );
    assert!(
        closures.iter().all(|feature| feature.author == "a"),
        "a pair without a recorded source is authored by its first member"
    );
}

#[test]
fn a_reviewed_link_chooses_the_source_geometry_per_feature() {
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    let mut document = pair_document_with_two_mounts();
    // mount-1 is sourced from the second half; mount-2 from the first.
    link_with_correspondences(
        &mut document,
        vec![
            correspondence(
                MechanicalFeatureKind::Mount,
                "mount-1",
                "mount-1",
                Some("right"),
            ),
            correspondence(
                MechanicalFeatureKind::Mount,
                "mount-2",
                "mount-2",
                Some("left"),
            ),
        ],
    )
    .unwrap();

    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    let find = |id: &str| {
        pair.features
            .iter()
            .find(|feature| feature.id == id)
            .unwrap_or_else(|| panic!("no feature {id:?}"))
    };
    let sourced_from_right = find("mount-1");
    assert_eq!(
        sourced_from_right.author, "right",
        "the reviewed source half authors its feature"
    );
    let MechanicalFeatureValue::Mount { mount } = &sourced_from_right.value else {
        panic!("a linked mount stores a mount");
    };
    assert_eq!(mount.at, Vec2 { x: 7.0, y: 3.0 });
    let sourced_from_left = find("mount-2");
    assert_eq!(sourced_from_left.author, "left");
    let MechanicalFeatureValue::Mount { mount } = &sourced_from_left.value else {
        panic!("a linked mount stores a mount");
    };
    assert_eq!(mount.at, Vec2 { x: 4.0, y: 5.0 });

    // The non-source half derives the exact inverse of the source geometry,
    // independently for each feature.
    let left: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: -7.0, y: 3.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: 7.0, y: 3.0 }
    );
    assert_eq!(
        mount_at(&left.configuration, "mount-2"),
        Vec2 { x: 4.0, y: 5.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-2"),
        Vec2 { x: -4.0, y: 5.0 }
    );
}

#[test]
fn a_reviewed_link_rejects_an_unknown_source_half() {
    let mut document = pair_document();
    let before = document.clone();
    let error = link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Mount,
            "mount-1",
            "mount-1",
            Some("ghost"),
        )],
    )
    .unwrap_err();
    assert!(error.contains("source"), "{error}");
    assert_eq!(document, before, "a rejected source mutates nothing");
}

#[test]
fn removing_a_physical_instance_requires_a_paired_instance_target() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    let before = document.clone();

    // A board target cannot identify a physical member to remove.
    let error = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemovePhysicalInstance,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap_err();
    assert!(error.contains("physical-instance target"), "{error}");
    assert_eq!(document, before, "a malformed target mutates nothing");

    // An unpaired instance is a plain topology change, not a pair removal.
    let error = remove_member(&mut document, "left").unwrap_err();
    assert!(error.contains("not part of"), "{error}");
    assert_eq!(document, before, "an ineligible member mutates nothing");
}

#[test]
fn a_removed_opening_identity_is_never_reused() {
    let mut document = document();
    let first = add_opening(&mut document);
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemoveOpening {
                opening_id: first.clone(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let second = add_opening(&mut document);
    assert_ne!(
        second, first,
        "a removed opening identity must not be handed out again"
    );

    // A queued intent naming the removed opening cannot reach the later addition, and
    // retires instead of being redirected.
    let queued = MechanicalEdit {
        target: board_target(),
        change: MechanicalChange::SetOpeningDimension {
            opening_id: first.clone(),
            vertex_id: None,
            field: MechanicalDimension::OpeningHeight,
            value: 9.0,
        },
        mounting_hole: Some(Box::new(template())),
    };
    assert_eq!(
        evaluate_edit(&document, &[], &queued).unwrap(),
        MechanicalEditDisposition::Retire
    );
    assert!(
        boardstudio_core::mechanical::edit::apply(&mut document, &[], &queued).is_err(),
        "a commit against the removed opening is an error, not a redirected change"
    );
}

#[test]
fn identity_reservations_survive_a_document_round_trip() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = document();
    let first = add_opening(&mut document);
    // A save/reopen round trip keeps the reservation monotonic.
    let encoded = serde_json::to_string(&document).unwrap();
    let mut reopened: ProjectDoc = serde_json::from_str(&encoded).unwrap();
    apply(
        &mut reopened,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemoveOpening {
                opening_id: first.clone(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let second = add_opening(&mut reopened);
    assert_ne!(
        second, first,
        "the reservation survives save and reopen so a removed identity is not reused"
    );
}

#[test]
fn one_undo_restores_a_paired_disable_completely() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};

    fn document_of(reply: CoreReply) -> ProjectDoc {
        match reply {
            CoreReply::Scene { document, .. } => *document,
            other => panic!("expected scene: {other:?}"),
        }
    }
    fn mechanical_command(base_revision: u64, edit: MechanicalEdit) -> CoreRequest {
        CoreRequest::Edit {
            id: "mechanical".into(),
            command: EditCommand {
                base_revision,
                transaction_id: "mechanical".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(edit),
                },
            },
        }
    }

    let mut document = pair_document();
    link_pair(&mut document);

    let mut engine = CoreEngine::new();
    let accepted = document_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }));

    let disabled = document_of(engine.handle(mechanical_command(
        accepted.revision,
        MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::Disable,
            mounting_hole: Some(Box::new(template())),
        },
    )));
    assert!(disabled.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(
        disabled
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical
            .is_none()
    );

    // One undo restores the pair, both stacks and the geometry projection.
    let restored = document_of(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(
        restored.hardware.as_ref().unwrap().pairs.len(),
        1,
        "the pair is restored"
    );
    assert!(
        restored
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .all(|instance| instance.mechanical.is_some()),
        "both halves are configured again"
    );
    let left = effective_inputs(&restored, &[], &instance_target("left")).unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
        "the linked geometry reads as before the disable"
    );

    // Redo repeats the whole result.
    let redone = document_of(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert!(redone.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(
        redone
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical
            .is_none()
    );
}

#[test]
fn a_pcb_violation_blocks_only_its_own_board_export() {
    use boardstudio_core::model::{ExportTarget, PrepareExportRequest};
    use std::collections::BTreeMap;

    let mut document = cross_board_closure_document();
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "wide".into(),
            },
            change: MechanicalChange::LinkMirrorPair {
                pair_id: "pair-left-right".into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction_source: "left".into(),
                correspondences: vec![correspondence(
                    MechanicalFeatureKind::ClosureMount,
                    "closure-1",
                    "closure-1",
                    Some("left"),
                )],
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();

    let request = |board_id: &str, contour: Contour| PrepareExportRequest {
        snapshot_token: "token".into(),
        expected_revision: document.revision,
        document: document.clone(),
        target: ExportTarget::Board {
            board_id: board_id.into(),
        },
        contours: vec![contour],
        model_paths: BTreeMap::new(),
    };

    // The reflected closure drill lands outside the narrow board: its fabrication
    // export is blocked.
    let blocked = boardstudio_core::artifact::kicad::prepare_export(request(
        "narrow",
        rect_contour(10.0, 5.0),
    ));
    assert!(blocked.is_err(), "{blocked:?}");

    // The other board's export is unaffected.
    let available = boardstudio_core::artifact::kicad::prepare_export(request(
        "wide",
        rect_contour(40.0, 30.0),
    ));
    assert!(available.is_ok(), "{available:?}");

    // The preview of the affected board stays available.
    let preview = boardstudio_core::artifact::kicad::prepare_preview(request(
        "narrow",
        rect_contour(10.0, 5.0),
    ));
    assert!(preview.is_ok(), "{preview:?}");
}

#[test]
fn rejoining_can_take_the_selected_halves_own_geometry_as_the_source() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    link_pair(&mut document);
    // The right half keeps its own placement as an explicit exception, then drifts.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::MakeFeatureLocal {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    document
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "right")
        .unwrap()
        .mechanical
        .as_mut()
        .unwrap()
        .mounts[0]
        .at = Vec2 { x: -5.0, y: 1.0 };

    // A deliberate own-half source is allowed when that half has geometry.
    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
                source_instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert_eq!(changed, vec!["mount-1", "right", "pair-left-right", "left"]);
    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    assert_eq!(pair.features[0].author, "right");
    let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
        panic!("a linked mount stores a mount");
    };
    assert_eq!(mount.at, Vec2 { x: -5.0, y: 1.0 });

    // The other half now derives the exact inverse of that geometry.
    let left =
        boardstudio_core::mechanical::effective_inputs(&document, &[], &instance_target("left"))
            .unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 5.0, y: 1.0 }
    );
}

#[test]
fn a_linked_gasket_support_reflects_the_authors_resolved_placement() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::features::mirror_support;

    let (mut document, accepted) = gasket_pair_document();
    let author = author_gasket_support(&document, &accepted, "left", "wide");
    let anchor = anchor_of(&author);
    for instance in document.hardware.as_mut().unwrap().instances.iter_mut() {
        instance
            .mechanical
            .as_mut()
            .unwrap()
            .gasket_layout
            .as_mut()
            .unwrap()
            .supports = vec![anchor.clone()];
    }
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::GasketSupport,
            &anchor.id,
            &anchor.id,
            Some("left"),
        )],
    )
    .unwrap();

    // The author keeps its own resolved placement; the other half shows the exact
    // reflection of it, not the same perimeter fraction re-resolved on its own contour.
    let left = effective_inputs(&document, &accepted, &instance_target("left")).unwrap();
    assert_eq!(left.gasket_supports.len(), 1);
    assert_close(
        left.gasket_supports[0].at,
        author.at,
        "the author keeps its placement",
    );
    assert!(left.gasket_supports[0].fit_error.is_none());

    let right = effective_inputs(&document, &accepted, &instance_target("right")).unwrap();
    let expected = mirror_support(
        Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        &author,
    );
    assert_eq!(right.gasket_supports.len(), 1);
    assert_close(
        right.gasket_supports[0].at,
        expected.at,
        "the reflected placement",
    );
    assert_close(
        right.gasket_supports[0].tangent,
        expected.tangent,
        "the reflected tangent",
    );
    assert_close(
        right.gasket_supports[0].normal,
        expected.normal,
        "the reflected normal",
    );
    assert!(
        (right.gasket_supports[0].at.x - author.at.x).abs() > 1.0
            || (right.gasket_supports[0].at.y - author.at.y).abs() > 1.0,
        "the reflection is not the author's own placement"
    );
    assert!(
        right.gasket_supports[0].fit_error.is_some(),
        "the narrow board cannot receive the reflected placement, reported rather than moved"
    );
    assert_close(
        right.gasket_supports[0].at,
        expected.at,
        "the intended placement is retained even when it does not associate",
    );

    // A board-scoped read has no pair context and reports no linked supports.
    let board = effective_inputs(
        &document,
        &accepted,
        &MechanicalTarget::Board {
            board_id: "wide".into(),
        },
    )
    .unwrap();
    assert!(board.gasket_supports.is_empty());
}

#[test]
fn a_linked_gasket_support_reflects_through_the_inverse_map_for_a_second_member_author() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::features::{inverse_pose, mirror_support};

    let (mut document, mut accepted) = gasket_pair_document();
    // The author half resolves on its own accepted outline, so give the second board a
    // projection that can carry supports as well.
    accepted[1].contours = vec![rect_contour(200.0, 150.0)];
    let author = author_gasket_support(&document, &accepted, "right", "narrow");
    let anchor = anchor_of(&author);
    for instance in document.hardware.as_mut().unwrap().instances.iter_mut() {
        instance
            .mechanical
            .as_mut()
            .unwrap()
            .gasket_layout
            .as_mut()
            .unwrap()
            .supports = vec![anchor.clone()];
    }
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::GasketSupport,
            &anchor.id,
            &anchor.id,
            Some("right"),
        )],
    )
    .unwrap();

    let left = effective_inputs(&document, &accepted, &instance_target("left")).unwrap();
    let expected = mirror_support(
        inverse_pose(Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        }),
        &author,
    );
    assert_eq!(left.gasket_supports.len(), 1);
    assert_close(
        left.gasket_supports[0].at,
        expected.at,
        "the inverse reflection",
    );
    assert_close(
        left.gasket_supports[0].tangent,
        expected.tangent,
        "the inverse tangent",
    );
    assert_close(
        left.gasket_supports[0].normal,
        expected.normal,
        "the inverse normal",
    );
    let right = effective_inputs(&document, &accepted, &instance_target("right")).unwrap();
    assert_close(
        right.gasket_supports[0].at,
        author.at,
        "the author keeps its placement",
    );
}

#[test]
fn legacy_local_fields_stay_per_instance_and_errors_do_not_mutate() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = legacy_halves_document();
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::SetBatteryEnabled(true),
            mounting_hole: None,
        },
    )
    .unwrap();
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Suspension,
            },
            mounting_hole: None,
        },
    )
    .unwrap();

    let shared = document
        .hardware
        .as_ref()
        .unwrap()
        .shared_construction
        .as_ref()
        .unwrap();
    assert!(
        shared.battery.is_none(),
        "a local battery never lands in the shared source"
    );
    assert!(
        shared.mounts.is_empty(),
        "a local mount never lands in the shared source"
    );

    let left = effective_inputs(&document, &[], &instance_target("left"))
        .unwrap()
        .configuration;
    assert!(left.battery.is_some(), "the edited half shows its battery");
    assert_eq!(left.mounts.len(), 1, "the edited half shows its mount");
    let right = effective_inputs(&document, &[], &instance_target("right"))
        .unwrap()
        .configuration;
    assert!(
        right.battery.is_none() && right.mounts.is_empty(),
        "the other legacy halves keep their own local values"
    );

    // A malformed value is an error and mutates nothing.
    let before = document.clone();
    let error = set_wall_thickness(&mut document, "left", -1.0).unwrap_err();
    assert!(!error.is_empty());
    assert_eq!(document, before, "a rejected value mutates nothing");
}

#[test]
fn a_linked_change_reports_the_pair_and_both_halves() {
    use boardstudio_core::mechanical::edit::apply;

    let owners = ["pair-left-right", "left", "right"];

    // An ordinary edit of a linked feature from one half changes the pair's single
    // authored value, so the counterpart's derived geometry changed too.
    let mut document = pair_document();
    link_pair(&mut document);
    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "mount-1".into(),
                at: Vec2 { x: -5.0, y: 1.0 },
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            changed.contains(&id.to_string()),
            "a linked edit reports {id}: {changed:?}"
        );
    }
    assert!(changed.contains(&"mount-1".to_string()), "{changed:?}");

    // Make local, omit and rejoin change the pair's records for both halves.
    let mut document = pair_document();
    link_pair(&mut document);
    let local = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::MakeFeatureLocal {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            local.contains(&id.to_string()),
            "Make local reports {id}: {local:?}"
        );
    }
    let omitted = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::OmitFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            omitted.contains(&id.to_string()),
            "Omit reports {id}: {omitted:?}"
        );
    }
    let rejoined = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
                source_instance_id: "left".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            rejoined.contains(&id.to_string()),
            "Rejoin reports {id}: {rejoined:?}"
        );
    }

    // An ordinary delete of a linked feature removes both counterparts.
    let mut document = pair_document();
    link_pair(&mut document);
    let deleted = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::RemoveMount {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "mount-1".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            deleted.contains(&id.to_string()),
            "a linked delete reports {id}: {deleted:?}"
        );
    }
}

#[test]
fn case_findings_are_clear_is_scoped_to_the_target() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = conflicting_pair_document();
    link_pair(&mut document);
    let right = effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert!(
        !right.case_findings_are_clear(),
        "the half that owns the conflict is not clear: {:?}",
        right.findings
    );
    let left = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    assert!(
        left.case_findings_are_clear(),
        "a sibling half's conflict is not this half's problem: {:?}",
        left.findings
    );
    let board = effective_inputs(
        &document,
        &[],
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    assert!(
        !board.case_findings_are_clear(),
        "a board-scoped read still carries the conflict"
    );

    // A pair without a conflict leaves every target clear.
    let mut clean = pair_document();
    link_pair(&mut clean);
    let clean = effective_inputs(&clean, &[], &instance_target("right")).unwrap();
    assert!(clean.case_findings_are_clear(), "{:?}", clean.findings);
}
