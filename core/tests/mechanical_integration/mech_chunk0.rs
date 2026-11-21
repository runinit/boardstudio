use super::*;

#[test]
fn the_evaluator_retires_a_vanished_target_and_errors_on_a_malformed_value() {
    let document = document();

    let vanished_instance = evaluate_edit(
        &document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Instance {
                instance_id: "gone".into(),
            },
            change: MechanicalChange::Enable,
            mounting_hole: None,
        },
    );
    assert_eq!(
        vanished_instance.unwrap(),
        MechanicalEditDisposition::Retire
    );

    let vanished_board = evaluate_edit(
        &document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "other".into(),
            },
            change: MechanicalChange::Enable,
            mounting_hole: None,
        },
    );
    assert_eq!(vanished_board.unwrap(), MechanicalEditDisposition::Retire);

    // A structurally malformed value is a Core error, never a silent retirement.
    let malformed = evaluate_edit(&document, &[], &plate_thickness(-1.0));
    assert!(malformed.is_err(), "got {malformed:?}");
}

#[test]
fn an_ordinary_edit_from_either_half_updates_the_single_authored_value() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    fn move_mount(document: &mut ProjectDoc, instance_id: &str, x: f64, y: f64) {
        apply(
            document,
            &[],
            &MechanicalEdit {
                target: instance_target(instance_id),
                change: MechanicalChange::SetMountPosition {
                    collection: MechanicalMountCollection::Suspension,
                    mount_id: "mount-1".into(),
                    at: Vec2 { x, y },
                },
                mounting_hole: Some(Box::new(template())),
            },
        )
        .unwrap();
    }

    fn authored_mount(document: &ProjectDoc) -> (String, Vec2) {
        let pair = &document.hardware.as_ref().unwrap().pairs[0];
        let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
            panic!("a linked mount stores a mount");
        };
        (pair.features[0].author.clone(), mount.at)
    }

    let mut document = pair_document();
    link_pair(&mut document);

    // Edit the linked feature from the non-authoring half.
    move_mount(&mut document, "right", -5.0, 1.0);
    let (author, at) = authored_mount(&document);
    assert_eq!(
        author, "right",
        "the editing half's frame authors the value"
    );
    assert_eq!(at, Vec2 { x: -5.0, y: 1.0 });
    let left: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -5.0, y: 1.0 }
    );
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 5.0, y: 1.0 }
    );
    assert!(
        stored_mounts(&document, "left")
            .iter()
            .all(|mount| mount.id != "mount-1")
    );
    assert!(
        stored_mounts(&document, "right")
            .iter()
            .all(|mount| mount.id != "mount-1")
    );

    // The reverse direction updates the same single value.
    move_mount(&mut document, "left", 6.0, 2.0);
    let (author, at) = authored_mount(&document);
    assert_eq!(author, "left");
    assert_eq!(at, Vec2 { x: 6.0, y: 2.0 });
    let left: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 6.0, y: 2.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -6.0, y: 2.0 }
    );
    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    assert_eq!(pair.features.len(), 1, "still exactly one relationship");
}

#[test]
fn a_linked_closure_mount_projects_generated_holes_on_both_halves() {
    use boardstudio_core::mechanical::clearance::project_owned_closure_clearance;

    let mut document = pair_document();
    for instance in document.hardware.as_mut().unwrap().instances.iter_mut() {
        instance.mechanical.as_mut().unwrap().closure_mounts =
            Some(vec![mount("closure-1", 3.0, 4.0)]);
    }
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::LinkMirrorPair {
                pair_id: "pair-left-right".into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction_source: "left".into(),
                correspondences: vec![MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::ClosureMount,
                    first_feature_id: "closure-1".into(),
                    second_feature_id: "closure-1".into(),
                    source_instance_id: None,
                }],
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();

    // Neither half stores its linked closure mount; the projection still derives the
    // generated clearance hole for both halves from the pair's single authored value.
    for instance in &document.hardware.as_ref().unwrap().instances {
        assert!(
            instance
                .mechanical
                .as_ref()
                .unwrap()
                .closure_mounts
                .as_ref()
                .unwrap()
                .iter()
                .all(|mount| mount.id != "closure-1"),
            "linked geometry must not be stored on the half"
        );
    }
    let projected = project_owned_closure_clearance(document.clone(), &template()).unwrap();
    let generated: Vec<&str> = projected
        .parts
        .iter()
        .map(|part| part.id.as_str())
        .filter(|id| id.starts_with("case-closure/"))
        .collect();
    assert_eq!(
        generated.len(),
        2,
        "both reflected linked mounts produce a generated hole: {generated:?}"
    );
}

#[test]
fn removing_the_last_generated_hole_needs_no_mounting_hole_template() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = document();
    let mut configuration = configuration();
    configuration.closure_mounts = Some(vec![mount("closure-1", 4.0, 0.0)]);
    document.mechanical = Some(configuration);

    // First edit creates the generated clearance hole.
    document
        .outline
        .push(boardstudio_core::model::OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: boardstudio_core::model::Operation::Add,
        });
    document.boards[0].outline_ids = vec!["outline".into()];
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::SetDimension {
                field: MechanicalDimension::Clearance,
                value: 0.4,
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert!(
        document
            .parts
            .iter()
            .any(|part| part.id.starts_with("case-closure/")),
        "the generated hole exists before removal"
    );

    // Removing the final generated hole creates or updates nothing, so it needs no
    // template even though it is a projection change.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::SetMountCollection {
                collection: MechanicalMountCollection::Closure,
                mounts: vec![],
            },
            mounting_hole: None,
        },
    )
    .unwrap();
    assert!(
        document
            .parts
            .iter()
            .all(|part| !part.id.starts_with("case-closure/")),
        "the generated hole is removed without a template"
    );
}

#[test]
fn a_local_delete_affects_only_that_half() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::features;
    use boardstudio_core::model::MechanicalFeatureState;

    let mut document = pair_document();
    link_pair(&mut document);
    // Make the right half's mount an explicit per-feature exception.
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

    // Ordinary delete on the localized half affects only that half.
    apply(
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

    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    assert_eq!(pair.features.len(), 1, "the relationship survives");
    assert_eq!(
        features::member_state(pair, "right", "mount-1"),
        Some(MechanicalFeatureState::Local)
    );
    let left =
        boardstudio_core::mechanical::effective_inputs(&document, &[], &instance_target("left"))
            .unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 2.0, y: 3.0 }
    );
    let right =
        boardstudio_core::mechanical::effective_inputs(&document, &[], &instance_target("right"))
            .unwrap();
    assert!(
        right
            .configuration
            .mounts
            .iter()
            .all(|mount| mount.id != "mount-1"),
        "the localized half's delete stays local"
    );
}

/// Enabling a legacy second half is not a no-op for closures: it resolves from the shared
/// source, so it already has a configuration, but its closure list is absent. Enable
/// initializes that list from its own outline (generator-parameters ticket 06, finding B).

#[test]
fn enabling_a_legacy_second_half_initializes_its_closure_mounts() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::effective_inputs;

    let (mut document, accepted) = two_unconfigured_halves();
    apply(&mut document, &accepted, &enable_edit("a")).unwrap();
    let first = stored_closure_mounts(&document, "a").expect("the first half proposes mounts");
    assert!(!first.is_empty());

    apply(&mut document, &accepted, &enable_edit("b")).unwrap();
    let second = effective_inputs(&document, &accepted, &instance_target("b"))
        .unwrap()
        .configuration
        .closure_mounts;
    assert_eq!(
        second.map(|mounts| mounts.len()),
        Some(first.len()),
        "the second half initialized its own closure mounts"
    );
    assert!(
        stored_closure_mounts(&document, "b").is_some(),
        "the second half stores its closure list"
    );
}

#[test]
fn the_evaluator_retires_an_edit_that_targets_a_removed_member() {
    let mut document = pair_document();
    link_pair(&mut document);
    remove_member(&mut document, "right").unwrap();

    let disposition = evaluate_edit(
        &document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 4.0,
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert_eq!(disposition, MechanicalEditDisposition::Retire);
}

/// Open and ReplaceDocument refuse a malformed mirrored-half record with a located message, and
/// the engine keeps the accepted document. A stored record is no more trusted than an edit's.
/// Open checks the record directly. ReplaceDocument reaches the same check through the pair
/// validation that closes its topology reconciliation.

#[test]
fn a_malformed_mirrored_half_record_is_refused_on_open_and_replace() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};

    let mut engine = CoreEngine::new();
    let refused = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: paired_document_with_halves("ghost"),
    });
    let CoreReply::Error { message, .. } = refused else {
        panic!("Open must refuse the record: {refused:?}");
    };
    assert!(message.contains("\"pair-left-right\""), "{message}");
    assert!(message.contains("halves.mirrorBoardId"), "{message}");
    assert!(message.contains("\"ghost\""), "{message}");

    // A consistent record opens and replaces.
    let accepted = match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: paired_document_with_halves("board"),
    }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("a consistent record opens: {other:?}"),
    };
    assert!(
        accepted.hardware.as_ref().unwrap().pairs[0]
            .halves
            .is_some()
    );

    let replace = |engine: &mut CoreEngine, replacement: ProjectDoc| {
        engine.handle(CoreRequest::Edit {
            id: "replace".into(),
            command: EditCommand {
                base_revision: accepted.revision,
                transaction_id: "replace".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
            },
        })
    };
    let refused = replace(&mut engine, paired_document_with_halves("ghost"));
    let CoreReply::Error { message, .. } = refused else {
        panic!("ReplaceDocument must refuse the record: {refused:?}");
    };
    assert!(message.contains("halves.mirrorBoardId"), "{message}");
    // The refusal changed nothing: the same consistent replacement still applies on the same
    // base revision.
    assert!(matches!(
        replace(&mut engine, paired_document_with_halves("board")),
        CoreReply::Scene { .. }
    ));
}

#[test]
fn disabling_an_unpaired_target_retires_intents_for_its_removed_openings() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = document();
    let opening = add_opening(&mut document);
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::Disable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    // While the configuration is gone the intent retires, so nothing redirects.
    assert_eq!(
        evaluate_edit(
            &document,
            &[],
            &MechanicalEdit {
                target: board_target(),
                change: MechanicalChange::SetOpeningDimension {
                    opening_id: opening.clone(),
                    vertex_id: None,
                    field: MechanicalDimension::OpeningHeight,
                    value: 5.0,
                },
                mounting_hole: Some(Box::new(template())),
            },
        )
        .unwrap(),
        MechanicalEditDisposition::Retire
    );
    // Re-enabled defaults carry no authored openings, so it still retires.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::Enable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert_eq!(
        evaluate_edit(
            &document,
            &[],
            &MechanicalEdit {
                target: board_target(),
                change: MechanicalChange::SetOpeningDimension {
                    opening_id: opening,
                    vertex_id: None,
                    field: MechanicalDimension::OpeningHeight,
                    value: 5.0,
                },
                mounting_hole: Some(Box::new(template())),
            },
        )
        .unwrap(),
        MechanicalEditDisposition::Retire
    );
}

#[test]
fn a_local_addition_never_takes_an_omitted_pair_owned_identity() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = pair_document_with_openings();
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Opening,
            "opening-1",
            "opening-1",
            Some("right"),
        )],
    )
    .unwrap();
    // An explicit omission hides the linked opening on this half only.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::OmitFeatureMember {
                feature_id: "opening-1".into(),
                instance_id: "left".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let hidden = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    assert!(
        !hidden
            .configuration
            .openings
            .as_ref()
            .is_some_and(|openings| openings.iter().any(|opening| opening.id == "opening-1")),
        "the omission hides the linked opening"
    );

    // The pair still owns that identity for this half, so a local addition must not
    // take it: rejoining would otherwise leave two openings under one identity.
    let new_id = add_instance_opening(&mut document, "left");
    assert_ne!(
        new_id, "opening-1",
        "an omitted pair-owned identity stays reserved for its member"
    );

    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::RejoinFeatureMember {
                feature_id: "opening-1".into(),
                instance_id: "left".into(),
                source_instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let rejoined = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let ids: Vec<&str> = rejoined
        .configuration
        .openings
        .as_ref()
        .unwrap()
        .iter()
        .map(|opening| opening.id.as_str())
        .collect();
    let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "every visible opening identity stays unique: {ids:?}"
    );
}

#[test]
fn a_pending_edit_to_a_disabled_half_retires_and_enable_still_works() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    link_pair(&mut document);
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::Disable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();

    // The half still exists, but it is ineligible: the pending edit retires.
    let pending = MechanicalEdit {
        target: instance_target("right"),
        change: MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 4.0,
        },
        mounting_hole: Some(Box::new(template())),
    };
    assert_eq!(
        evaluate_edit(&document, &[], &pending).unwrap(),
        MechanicalEditDisposition::Retire
    );

    // Re-enabling the half is still possible, and it does not relink the pair.
    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::Enable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert!(!changed.is_empty(), "{changed:?}");
    let hardware = document.hardware.as_ref().unwrap();
    assert!(hardware.pairs.is_empty(), "Enable never relinks");
    assert!(
        hardware
            .instances
            .iter()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical
            .is_some(),
        "the half is configured again"
    );
}

#[test]
fn a_legacy_instance_serializes_without_an_identity_reservation() {
    let document = document_with_an_opening();
    let encoded = serde_json::to_value(&document).unwrap();
    let instance = &encoded["hardware"]["instances"][0];
    assert!(
        instance.get("mechanicalIdentityReservation").is_none(),
        "a half that never reserved an identity keeps its serialized shape: {instance}"
    );

    // A reservation that exists round-trips and keeps working after reload.
    let mut document = document_with_an_opening();
    document.hardware.as_mut().unwrap().instances[0].mechanical_identity_reservation =
        Some(boardstudio_core::model::MechanicalIdentityReservation {
            next_opening_identity: 4,
            next_vertex_identity: 7,
            next_critical_fit_identity: 0,
            next_hardware_identity: 0,
            next_mount_identity: 0,
        });
    let reloaded: ProjectDoc =
        serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
    let stored = reloaded.hardware.as_ref().unwrap().instances[0]
        .mechanical_identity_reservation
        .unwrap();
    assert_eq!(stored.next_opening_identity, 4);
    assert_eq!(stored.next_vertex_identity, 7);
}

#[test]
fn a_generic_addition_on_a_paired_half_joins_the_pair() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = pair_document();
    link_pair(&mut document);

    // A generic addition on one half becomes a linked feature, not a private copy.
    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Suspension,
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let added_id = changed
        .iter()
        .find(|id| id.starts_with("case-mechanical-mount-"))
        .expect("the addition is reported")
        .clone();
    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    assert_eq!(pair.features.len(), 2, "the addition joined the pair");
    let added = pair
        .features
        .iter()
        .find(|feature| feature.id == added_id)
        .expect("the addition is a pair feature");
    assert_eq!(added.author, "right");
    assert_eq!(added.kind, MechanicalFeatureKind::Mount);
    assert_eq!(
        features_member_state(pair, "left", &added_id),
        Some(boardstudio_core::model::MechanicalFeatureState::Linked)
    );

    // Moving it from the authoring half reflects onto the other half, and neither half
    // stores a private copy.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Suspension,
                mount_id: added_id.clone(),
                at: Vec2 { x: 5.0, y: 1.0 },
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let left = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right = effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_close(
        mount_at(&right.configuration, &added_id),
        Vec2 { x: 5.0, y: 1.0 },
        "the authoring half keeps the added mount",
    );
    assert_close(
        mount_at(&left.configuration, &added_id),
        Vec2 { x: -5.0, y: 1.0 },
        "the other half derives the reflected addition",
    );
    for instance in ["left", "right"] {
        assert!(
            stored_mounts(&document, instance)
                .iter()
                .all(|mount| mount.id != added_id),
            "{instance} stores no private copy of the linked addition"
        );
    }
}

#[test]
fn legacy_conversion_rejects_ineligible_selections_without_mutating() {
    let mut document = legacy_document();
    // A locally owned member is not legacy.
    document
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .push(mirror_instance("local", vec![]));
    let before = document.clone();

    for ids in [Vec::new(), vec!["a", "a"], vec!["ghost"], vec!["local"]] {
        let error = convert(&mut document, &ids).unwrap_err();
        assert!(!error.is_empty(), "{ids:?}");
        assert_eq!(document, before, "{ids:?} must not mutate the document");
    }
}

#[test]
fn generated_gasket_geometry_uses_the_inverse_reflection_for_a_second_member_author() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::inputs::effective_document;

    let (mut document, accepted) = same_board_gasket_pair_document();
    link_gasket_support(&mut document, &accepted, "right");

    let reflected = effective_inputs(&document, &accepted, &instance_target("left"))
        .unwrap()
        .gasket_supports
        .first()
        .cloned()
        .expect("the linked support is reflected for the read path");
    let author = author_gasket_support(&document, &accepted, "right", "board");
    assert!(
        (reflected.at.x - author.at.x).abs() > 1.0,
        "the reflected placement differs from the author's own"
    );

    let (left_document, _) = effective_document(&document, &instance_target("left")).unwrap();
    let assembly = boardstudio_core::mechanical::resolve(&left_document, &accepted[0].contours);
    assert!(
        !assembly.generation_blocked,
        "the assembly resolves: {:?}",
        assembly.diagnostics
    );
    let generated = assembly
        .gasket_supports
        .iter()
        .find(|support| support.id == reflected.id)
        .unwrap_or_else(|| panic!("no generated support {}", reflected.id));
    assert_close(generated.at, reflected.at, "generated placement");
    assert_close(generated.tangent, reflected.tangent, "generated tangent");
    assert_close(generated.normal, reflected.normal, "generated normal");
}

#[test]
fn a_legacy_halves_shared_field_edit_reads_back() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = legacy_halves_document();
    let changed = set_wall_thickness(&mut document, "left", 4.0).unwrap();
    assert!(!changed.is_empty(), "the edit is a change: {changed:?}");

    // The shared source the legacy halves overlay must carry the edit.
    assert_eq!(
        document
            .hardware
            .as_ref()
            .unwrap()
            .shared_construction
            .as_ref()
            .unwrap()
            .wall_thickness,
        4.0,
        "the legacy shared source carries the edited value"
    );

    // Every legacy half that shares the source reads the new value, with no silent local
    // override and without converting ownership.
    for id in ["left", "right", "third"] {
        let inputs = effective_inputs(&document, &[], &instance_target(id)).unwrap();
        assert_eq!(
            inputs.configuration.wall_thickness, 4.0,
            "{id} reads the edited shared value"
        );
        assert_eq!(
            document
                .hardware
                .as_ref()
                .unwrap()
                .instances
                .iter()
                .find(|instance| instance.id == id)
                .unwrap()
                .construction_ownership,
            MechanicalConstructionOwnership::Legacy,
            "{id} keeps its legacy ownership"
        );
    }
}

#[test]
fn every_shared_construction_family_lands_in_the_legacy_source() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::model::MechanicalMount;

    fn legacy_ownership(document: &ProjectDoc, id: &str) -> MechanicalConstructionOwnership {
        document
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap()
            .construction_ownership
    }

    type Check = fn(&MechanicalConfiguration) -> bool;
    let cases: Vec<(&str, MechanicalChange, Check)> = vec![
        (
            "wall thickness",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 4.0,
            },
            |configuration| configuration.wall_thickness == 4.0,
        ),
        (
            "plate thickness",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::PlateThickness,
                value: 2.0,
            },
            |configuration| configuration.plate_thickness == 2.0,
        ),
        (
            "plate foam",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::PlateFoamThickness,
                value: 0.8,
            },
            |configuration| configuration.plate_foam_thickness == 0.8,
        ),
        (
            "bottom foam",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BottomFoamThickness,
                value: 0.9,
            },
            |configuration| configuration.bottom_foam_thickness == 0.9,
        ),
        (
            "bottom thickness",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BottomThickness,
                value: 4.5,
            },
            |configuration| configuration.bottom_thickness == 4.5,
        ),
        (
            "clearance",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::Clearance,
                value: 0.45,
            },
            |configuration| configuration.clearance == 0.45,
        ),
        (
            "opening allowance",
            MechanicalChange::SetDimension {
                field: MechanicalDimension::OpeningAllowance,
                value: 0.35,
            },
            |configuration| configuration.opening_allowance == Some(0.35),
        ),
        (
            "method (manufacturing)",
            MechanicalChange::SetMethod(boardstudio_core::model::PlateMethod::Cnc),
            |configuration| configuration.method == boardstudio_core::model::PlateMethod::Cnc,
        ),
        (
            "middle frame",
            MechanicalChange::SetMiddleFrame(true),
            |configuration| configuration.middle_frame == Some(true),
        ),
        (
            "integrated plate frame",
            MechanicalChange::SetIntegratedPlateFrame(true),
            |configuration| configuration.integrated_plate_frame,
        ),
        (
            "closure drive (recipe)",
            MechanicalChange::SetClosureDrive(boardstudio_core::model::ScrewDrive::Torx),
            |configuration| {
                configuration
                    .internal_gasket
                    .as_ref()
                    .is_some_and(|settings| {
                        settings.hardware.drive == boardstudio_core::model::ScrewDrive::Torx
                    })
            },
        ),
        (
            "mount",
            MechanicalChange::SetMount(MechanicalMount::Gasket),
            |configuration| configuration.mount == MechanicalMount::Gasket,
        ),
    ];

    for (label, change, check) in cases {
        let mut document = legacy_halves_document();
        let changed = boardstudio_core::mechanical::edit::apply(
            &mut document,
            &[],
            &MechanicalEdit {
                target: instance_target("left"),
                change: change.clone(),
                mounting_hole: None,
            },
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert!(!changed.is_empty(), "{label} is a change: {changed:?}");

        let shared = document
            .hardware
            .as_ref()
            .unwrap()
            .shared_construction
            .as_ref()
            .unwrap();
        assert!(check(shared), "{label}: the shared source carries the edit");

        for id in ["left", "right", "third"] {
            let inputs = effective_inputs(&document, &[], &instance_target(id)).unwrap();
            assert!(
                check(&inputs.configuration),
                "{label}: {id} reads the edited value back"
            );
            assert_eq!(
                legacy_ownership(&document, id),
                MechanicalConstructionOwnership::Legacy,
                "{label}: {id} keeps legacy ownership"
            );
        }
    }
}
