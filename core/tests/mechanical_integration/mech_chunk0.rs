use super::*;

#[test]
fn the_evaluator_retires_a_vanished_target_and_errors_on_a_malformed_value() {
    let document = document();

    let vanished_instance = evaluate_edit(
        &document,
        &[],
        &crate::support::session::mechanical_edit(
            instance_target("gone"),
            MechanicalChange::Enable,
            false,
        ),
    );
    assert_eq!(
        vanished_instance.unwrap(),
        MechanicalEditDisposition::Retire
    );

    let vanished_board = evaluate_edit(
        &document,
        &[],
        &crate::support::session::mechanical_edit(
            MechanicalTarget::Board {
                board_id: "other".into(),
            },
            MechanicalChange::Enable,
            false,
        ),
    );
    assert_eq!(vanished_board.unwrap(), MechanicalEditDisposition::Retire);

    // A structurally malformed value is a Core error, never a silent retirement.
    let malformed = evaluate_edit(&document, &[], &plate_thickness(-1.0));
    assert!(malformed.is_err(), "got {malformed:?}");
}

#[test]
fn an_ordinary_edit_from_either_half_updates_the_single_authored_value() {
    let mut document = linked_pair_document();
    // Edit from the non-authoring half first, then update the same value from the other side.
    for (author, other, at, reflected) in [
        (
            "right",
            "left",
            Vec2 { x: -5.0, y: 1.0 },
            Vec2 { x: 5.0, y: 1.0 },
        ),
        (
            "left",
            "right",
            Vec2 { x: 6.0, y: 2.0 },
            Vec2 { x: -6.0, y: 2.0 },
        ),
    ] {
        apply_with_template(&mut document, instance_target(author), move_mount(at)).unwrap();
        let features = &pair(&document).features;
        assert_eq!(features.len(), 1, "still exactly one relationship");
        let feature = &features[0];
        let MechanicalFeatureValue::Mount { mount } = &feature.value else {
            panic!("a linked mount stores a mount");
        };
        assert_eq!(feature.author, author, "the editing half authors the value");
        assert_eq!(mount.at, at, "{author}: stored pose");
        for (id, expected) in [(author, at), (other, reflected)] {
            assert_eq!(
                mount_at(&instance_inputs(&document, id).configuration, "mount-1"),
                expected,
                "{id}: effective pose"
            );
            assert!(
                stored_mounts(&document, id)
                    .iter()
                    .all(|mount| mount.id != "mount-1"),
                "{id}: no private mount"
            );
        }
    }
}

#[test]
fn a_linked_closure_mount_projects_generated_holes_on_both_halves() {
    use boardstudio_core::mechanical::clearance::project_owned_closure_clearance;

    let mut document = pair_document();
    for instance in document.hardware.as_mut().unwrap().instances.iter_mut() {
        instance.mechanical.as_mut().unwrap().closure_mounts =
            Some(vec![mount("closure-1", 3.0, 4.0)]);
    }
    apply_with_template(
        &mut document,
        board_target(),
        MechanicalChange::LinkMirrorPair {
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
    let projected =
        project_owned_closure_clearance(document.clone(), &fixture::mounting_hole()).unwrap();
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
    let mut configuration = fixture::mechanical("board");
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
    apply_with_template(
        &mut document,
        board_target(),
        MechanicalChange::SetDimension {
            field: MechanicalDimension::Clearance,
            value: 0.4,
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
        &crate::support::session::mechanical_edit(
            board_target(),
            MechanicalChange::SetMountCollection {
                collection: MechanicalMountCollection::Closure,
                mounts: vec![],
            },
            false,
        ),
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
    use boardstudio_core::mechanical::features;
    use boardstudio_core::model::MechanicalFeatureState;

    let mut document = linked_pair_document();
    // Make the right half's mount an explicit per-feature exception.
    apply_with_template(
        &mut document,
        instance_target("right"),
        local_mount("right"),
    )
    .unwrap();

    // Ordinary delete on the localized half affects only that half.
    apply_with_template(&mut document, instance_target("right"), delete_mount()).unwrap();

    let pair = pair(&document);
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
    let mut document = linked_pair_document();
    remove_member(&mut document, "right").unwrap();

    let disposition = evaluate_edit(
        &document,
        &[],
        &with_template(
            instance_target("right"),
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 4.0,
            },
        ),
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
    use serde_json::Value;
    let mut engine = boardstudio_core::CoreEngine::new();
    let mut document = serde_json::to_value(pair_document()).unwrap();
    let request = |engine: &mut boardstudio_core::CoreEngine, value: Value| {
        serde_json::from_str::<Value>(&engine.request(&value.to_string())).unwrap()
    };
    let accepted = request(
        &mut engine,
        json!({"kind":"open","id":"valid","document":document}),
    );
    assert_eq!(accepted["kind"], "scene");
    document["mirroredPairs"] = json!([{
        "id":"invalid-pair","source":{"kind":"secondBoard","id":"board"},
        "destination":{"kind":"secondBoard","id":"ghost"},
        "mapping":{"at":{"x":0,"y":0},"rotation":0},"overrides":[]
    }]);
    for candidate in [
        json!({"kind":"open","id":"open","document":document}),
        json!({"kind":"edit","id":"replace","command":{
            "baseRevision":accepted["document"]["revision"],"transactionId":"replace","phase":"commit",
            "targetIds":[],"operation":{"kind":"replace-document","document":document}
        }}),
    ] {
        let refusal = request(&mut engine, candidate);
        assert_eq!(refusal["kind"], "error");
        assert!(
            refusal["message"]
                .as_str()
                .unwrap()
                .contains("invalid-pair")
        );
        let snapshot = request(&mut engine, json!({"kind":"snapshot","id":"snapshot"}));
        assert_eq!(snapshot["document"], accepted["document"]);
    }
}

#[test]
fn disabling_an_unpaired_target_retires_intents_for_its_removed_openings() {
    let mut document = document();
    let opening = add_opening(&mut document);
    apply_with_template(&mut document, board_target(), MechanicalChange::Disable).unwrap();
    // While the configuration is gone the intent retires, so nothing redirects.
    assert_eq!(
        evaluate_edit(
            &document,
            &[],
            &with_template(
                board_target(),
                MechanicalChange::SetOpeningDimension {
                    opening_id: opening.clone(),
                    vertex_id: None,
                    field: MechanicalDimension::OpeningHeight,
                    value: 5.0,
                }
            ),
        )
        .unwrap(),
        MechanicalEditDisposition::Retire
    );
    // Re-enabled defaults carry no authored openings, so it still retires.
    apply_with_template(&mut document, board_target(), MechanicalChange::Enable).unwrap();
    assert_eq!(
        evaluate_edit(
            &document,
            &[],
            &with_template(
                board_target(),
                MechanicalChange::SetOpeningDimension {
                    opening_id: opening,
                    vertex_id: None,
                    field: MechanicalDimension::OpeningHeight,
                    value: 5.0,
                }
            ),
        )
        .unwrap(),
        MechanicalEditDisposition::Retire
    );
}

#[test]
fn a_local_addition_never_takes_an_omitted_pair_owned_identity() {
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
    apply_with_template(
        &mut document,
        instance_target("left"),
        MechanicalChange::OmitFeatureMember {
            feature_id: "opening-1".into(),
            instance_id: "left".into(),
        },
    )
    .unwrap();
    let hidden = instance_inputs(&document, "left");
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

    apply_with_template(
        &mut document,
        instance_target("left"),
        MechanicalChange::RejoinFeatureMember {
            feature_id: "opening-1".into(),
            instance_id: "left".into(),
            source_instance_id: "right".into(),
        },
    )
    .unwrap();
    let rejoined = instance_inputs(&document, "left");
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
    let mut document = linked_pair_document();
    apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::Disable,
    )
    .unwrap();

    // The half still exists, but it is ineligible: the pending edit retires.
    let pending = with_template(
        instance_target("right"),
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 4.0,
        },
    );
    assert_eq!(
        evaluate_edit(&document, &[], &pending).unwrap(),
        MechanicalEditDisposition::Retire
    );

    // Re-enabling the half is still possible, and it does not relink the pair.
    let changed = apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::Enable,
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
    let mut document = linked_pair_document();

    // A generic addition on one half becomes a linked feature, not a private copy.
    let changed = apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::AddMount {
            collection: MechanicalMountCollection::Suspension,
        },
    )
    .unwrap();
    let added_id = changed
        .iter()
        .find(|id| id.starts_with("case-mechanical-mount-"))
        .expect("the addition is reported")
        .clone();
    let pair = pair(&document);
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
    apply_with_template(
        &mut document,
        instance_target("right"),
        MechanicalChange::SetMountPosition {
            collection: MechanicalMountCollection::Suspension,
            mount_id: added_id.clone(),
            at: Vec2 { x: 5.0, y: 1.0 },
        },
    )
    .unwrap();
    let (left, right) = both_instance_inputs(&document);
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
fn a_legacy_halves_shared_field_edit_reads_back() {
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
        let inputs = instance_inputs(&document, id);
        assert_eq!(
            inputs.configuration.wall_thickness, 4.0,
            "{id} reads the edited shared value"
        );
        assert_eq!(
            stored_instance(&document, id).construction_ownership,
            MechanicalConstructionOwnership::Legacy,
            "{id} keeps its legacy ownership"
        );
    }
}

#[test]
fn every_shared_construction_family_lands_in_the_legacy_source() {
    use boardstudio_core::model::MechanicalMount;

    fn legacy_ownership(document: &ProjectDoc, id: &str) -> MechanicalConstructionOwnership {
        stored_instance(document, id).construction_ownership
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
            &crate::support::session::mechanical_edit(
                instance_target("left"),
                change.clone(),
                false,
            ),
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
            let inputs = instance_inputs(&document, id);
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
