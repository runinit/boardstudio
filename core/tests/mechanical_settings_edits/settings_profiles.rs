use super::*;

/// Ported from `first_switch_profile_family_assignment_is_treated_as_a_family_change`.
#[test]
fn the_first_switch_profile_assignment_counts_as_a_family_change() {
    let mut doc = document();
    doc.boards[0].part_ids = vec!["sw".into(), "sw2".into()];
    doc.definitions.push(part_definition(
        "switch-mx",
        PartKind::Switch,
        Some(mx_generator()),
    ));
    doc.definitions.push(part_definition(
        "switch-mx-2",
        PartKind::Switch,
        Some(mx_generator()),
    ));
    doc.parts.push(part("sw", "switch-mx", None));
    doc.parts.push(part("sw2", "switch-mx-2", None));

    apply_change(
        &mut doc,
        MechanicalChange::AssignSwitchProfile {
            definition_id: "switch-mx".into(),
            family: MechanicalSwitchFamily::ChocV1,
        },
    )
    .unwrap();
    assert_eq!(
        stored(&doc).plate_thickness,
        1.3,
        "the first assignment adopts the family's plate thickness"
    );

    apply_change(
        &mut doc,
        MechanicalChange::AssignSwitchProfile {
            definition_id: "switch-mx-2".into(),
            family: MechanicalSwitchFamily::ChocV1,
        },
    )
    .unwrap();
    assert_eq!(
        stored(&doc).plate_thickness,
        1.3,
        "a later same-family assignment keeps the existing thickness"
    );
}

#[test]
fn stabilizer_profile_and_kind_apply() {
    let mut doc = document();
    doc.boards[0].part_ids = vec!["wide".into(), "stab".into()];
    doc.definitions.push(part_definition(
        "switch-mx",
        PartKind::Switch,
        Some(mx_generator()),
    ));
    doc.definitions
        .push(part_definition("stab-mx-2u", PartKind::Custom, None));
    doc.parts
        .push(part("wide", "switch-mx", Some(Vec2 { x: 40.0, y: 20.0 })));
    doc.parts.push(part("stab", "stab-mx-2u", None));

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AssignStabilizerProfile {
                definition_id: "stab-mx-2u".into(),
                source: MechanicalBuiltinProfile::MxStab2u,
            },
        )
        .unwrap(),
        vec!["board", "stab-mx-2u"]
    );
    assert_eq!(stored(&doc).profiles.len(), 1);
    assert_eq!(stored(&doc).profiles[0].definition_id, "stab-mx-2u");
    assert!(stored(&doc).profiles[0].switch_family.is_none());

    let changed = apply_change(
        &mut doc,
        MechanicalChange::SetStabilizerKind(MechanicalStabilizerOverride {
            profile: None,
            part_id: "wide".into(),
            kind: MechanicalStabilizerKind::PcbMount,
            units: 2.0,
            rotation: None,
        }),
    )
    .unwrap();
    assert_eq!(changed, vec!["board", "wide"]);
    assert_eq!(stored(&doc).stabilizers.as_ref().unwrap().len(), 1);
}

/// Ported from `stabilizer_profile_accepts_a_placed_custom_definition`.
#[test]
fn a_placed_custom_stabilizer_definition_accepts_a_profile() {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards
        .push(board("board", vec!["stabilizer-part".into()]));
    doc.parts.push(part("stabilizer-part", "stab-mx-2u", None));
    doc.definitions
        .push(part_definition("stab-mx-2u", PartKind::Custom, None));
    doc.mechanical = Some(configuration());

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AssignStabilizerProfile {
                definition_id: "stab-mx-2u".into(),
                source: MechanicalBuiltinProfile::MxStab2u,
            },
        )
        .unwrap(),
        vec!["board", "stab-mx-2u"]
    );
    assert_eq!(stored(&doc).profiles.len(), 1);
}

/// Ported from
/// `assigned_custom_profile_updates_geometry_and_removes_by_current_definition`.
#[test]
fn profile_update_and_removal_follow_the_current_definition() {
    let mut doc = document();
    doc.boards[0].part_ids = vec!["part".into()];
    let mut definition = part_definition("imported", PartKind::Custom, None);
    definition.kicad_source = Some(KicadSource {
        format_version: 1,
        source: "(footprint \"custom\")".into(),
    });
    doc.definitions.push(definition);
    doc.parts.push(part("part", "imported", None));

    let unrelated = MechanicalPartProfile {
        source_geometry: None,
        pcb_holes: None,
        clearance_volumes: None,
        openings: None,
        clearances: None,
        supported_thickness: None,
        switch_family: Some(MechanicalSwitchFamily::Mx),
        definition_id: "library-switch".into(),
        source: "Library MX fit".into(),
        cutouts: vec![],
        plate_to_pcb: 3.5,
    };
    doc.mechanical
        .as_mut()
        .unwrap()
        .profiles
        .push(unrelated.clone());

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AssignImportedGeometryProfile {
                definition_id: "imported".into(),
            },
        )
        .unwrap(),
        vec!["board", "imported"]
    );
    assert_eq!(stored(&doc).profiles.len(), 2);
    assert_eq!(stored(&doc).profiles[1].source, "KiCad imported");

    let mut updated = stored(&doc).profiles[1].clone();
    updated.cutouts = vec![vec![
        Vec2 { x: -9.4, y: 9.5 },
        Vec2 { x: 9.5, y: 9.5 },
        Vec2 { x: 9.5, y: -9.5 },
    ]];
    updated.source_geometry = Some(MechanicalProfileSource {
        text: "(footprint \"custom\")".into(),
        sha256: "sha".into(),
        mappings: vec![],
        source_ids: vec!["geometry-70".into()],
    });
    assert_eq!(
        apply_change(&mut doc, MechanicalChange::UpdateProfile(updated.clone())).unwrap(),
        vec!["board", "imported"]
    );
    assert_eq!(stored(&doc).profiles[1], updated);

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveProfile {
                definition_id: "imported".into(),
            },
        )
        .unwrap(),
        vec!["board", "imported"]
    );
    assert_eq!(stored(&doc).profiles, vec![unrelated.clone()]);
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveProfile {
                definition_id: "imported".into(),
            },
        )
        .unwrap_err()
        .contains("no longer assigned")
    );

    apply_change(
        &mut doc,
        MechanicalChange::RemoveProfile {
            definition_id: "library-switch".into(),
        },
    )
    .unwrap();
    assert!(stored(&doc).profiles.is_empty());
}

/// Ported from `profile_assignment_rejects_a_different_board_and_duplicate_target`.
#[test]
fn profile_assignment_rejects_a_part_on_another_board_and_a_duplicate_target() {
    // The switch definition is placed on another board, so it is not an eligible
    // profile target for this board.
    let mut other = document();
    other.boards.push(board("other", vec!["sw".into()]));
    other.definitions.push(part_definition(
        "switch-mx",
        PartKind::Switch,
        Some(mx_generator()),
    ));
    other.parts.push(part("sw", "switch-mx", None));
    let before = stored(&other).clone();
    assert!(
        apply_change(
            &mut other,
            MechanicalChange::AssignSwitchProfile {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::Mx,
            },
        )
        .unwrap_err()
        .contains("not available")
    );
    assert_eq!(stored(&other), &before);

    // The same definition cannot be assigned twice.
    let mut duplicate = document();
    duplicate.boards[0].part_ids = vec!["sw".into()];
    duplicate.definitions.push(part_definition(
        "switch-mx",
        PartKind::Switch,
        Some(mx_generator()),
    ));
    duplicate.parts.push(part("sw", "switch-mx", None));
    duplicate
        .mechanical
        .as_mut()
        .unwrap()
        .profiles
        .push(MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: Some(MechanicalSwitchFamily::Mx),
            definition_id: "switch-mx".into(),
            source: "fixture".into(),
            cutouts: vec![],
            plate_to_pcb: 3.5,
        });
    assert!(
        apply_change(
            &mut duplicate,
            MechanicalChange::AssignSwitchProfile {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::Mx,
            },
        )
        .unwrap_err()
        .contains("already assigned")
    );
}

#[test]
fn an_instance_target_persists_to_that_instance() {
    let mut doc = document();
    let mut instance = PhysicalBoardInstance {
        mechanical_identity_reservation: None,
        id: "left".into(),
        name: "Left".into(),
        board_id: "board".into(),
        half: "left".into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(configuration()),
        construction_ownership: MechanicalConstructionOwnership::Local,
    };
    instance.mechanical.as_mut().unwrap().wall_thickness = 2.0;
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![instance],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });

    let changed = apply_instance(
        &mut doc,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
    )
    .unwrap();
    assert_eq!(changed, vec!["left"]);
    assert_eq!(
        doc.hardware.as_ref().unwrap().instances[0]
            .mechanical
            .as_ref()
            .unwrap()
            .wall_thickness,
        3.0
    );
    assert_eq!(
        stored(&doc).wall_thickness,
        2.0,
        "the canonical configuration is untouched"
    );
}

#[test]
fn vanished_or_ineligible_targets_error() {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards.push(board("board", vec![]));
    // An eligible board with no stored configuration resolves the Core-owned
    // defaults, so a settings edit applies to them instead of erroring. The adapter
    // no longer needs its own `initial_settings` builder.
    let changed = apply_change(
        &mut doc,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
    )
    .unwrap();
    assert_eq!(changed, vec!["board"]);
    assert_eq!(stored(&doc).wall_thickness, 3.0);
    // A board that is not in the document stays ineligible.
    let missing = apply(
        &mut doc,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "ghost".into(),
            },
            change: MechanicalChange::Enable,
            mounting_hole: None,
        },
    );
    assert!(missing.is_err(), "{missing:?}");
    assert!(
        apply_instance(&mut doc, MechanicalChange::Enable).is_err(),
        "an unknown instance is unavailable"
    );
}

#[test]
fn outline_classification_covers_the_projection_changes() {
    let pure: Vec<MechanicalChange> = vec![
        MechanicalChange::SetBatteryEnabled(true),
        MechanicalChange::SetClosureDrive(ScrewDrive::Torx),
        MechanicalChange::SetClosureInsertPreset("m2-4".into()),
        MechanicalChange::AddCriticalFit {
            part_id: "plate".into(),
        },
        MechanicalChange::AddHardware {
            part_id: "plate".into(),
            feature_id: "feature".into(),
        },
        MechanicalChange::SetMethod(PlateMethod::Cnc),
        MechanicalChange::SetMount(MechanicalMount::Gasket),
        MechanicalChange::SetBottomStyle(MechanicalBottomStyle::Shell),
        MechanicalChange::SetMiddleFrame(true),
        MechanicalChange::SetIntegratedPlateFrame(true),
        MechanicalChange::AddProcessOverride {
            part_id: "plate".into(),
        },
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
        MechanicalChange::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value: 2.0,
        },
        MechanicalChange::SetStabilizerKind(MechanicalStabilizerOverride {
            profile: None,
            part_id: "wide".into(),
            kind: MechanicalStabilizerKind::PcbMount,
            units: 2.0,
            rotation: None,
        }),
    ];
    for change in pure {
        assert!(
            !affects_outline(&change),
            "pure settings edit {change:?} must not rebuild outlines"
        );
    }

    let projection: Vec<MechanicalChange> = vec![
        MechanicalChange::Enable,
        MechanicalChange::Disable,
        MechanicalChange::InitializeClosures,
        MechanicalChange::AddOpening,
        MechanicalChange::RemoveOpening {
            opening_id: "opening-2".into(),
        },
        MechanicalChange::AddOpeningVertex {
            opening_id: "opening-2".into(),
        },
        MechanicalChange::RemoveOpeningVertex {
            opening_id: "opening-2".into(),
            vertex_id: "vertex-1".into(),
        },
        MechanicalChange::SetOpeningDimension {
            opening_id: "opening-2".into(),
            vertex_id: None,
            field: MechanicalDimension::OpeningHeight,
            value: 12.0,
        },
        MechanicalChange::AddMount {
            collection: MechanicalMountCollection::Closure,
        },
        MechanicalChange::SetMountCollection {
            collection: MechanicalMountCollection::Closure,
            mounts: vec![],
        },
        MechanicalChange::SetMountDimension {
            collection: MechanicalMountCollection::Closure,
            mount_id: "m".into(),
            field: MechanicalDimension::MountHoleDiameter,
            value: 2.4,
        },
        MechanicalChange::SetMountPosition {
            collection: MechanicalMountCollection::Closure,
            mount_id: "m".into(),
            at: Vec2 { x: 1.0, y: 2.0 },
        },
        MechanicalChange::SetMountKind {
            collection: MechanicalMountCollection::Closure,
            mount_id: "m".into(),
            kind: MountKind::Boss,
        },
        MechanicalChange::RemoveMount {
            collection: MechanicalMountCollection::Closure,
            mount_id: "m".into(),
        },
        MechanicalChange::SetDimension {
            field: MechanicalDimension::Clearance,
            value: 0.4,
        },
        MechanicalChange::SetGasketSupportPlacement {
            support_id: "s".into(),
            anchors: vec![],
        },
        MechanicalChange::ResetGasketPlacement,
        MechanicalChange::AssignSwitchProfile {
            definition_id: "switch-mx".into(),
            family: MechanicalSwitchFamily::Mx,
        },
        MechanicalChange::AssignStabilizerProfile {
            definition_id: "stab".into(),
            source: MechanicalBuiltinProfile::MxStab2u,
        },
        MechanicalChange::AssignImportedGeometryProfile {
            definition_id: "imported".into(),
        },
        MechanicalChange::RemoveProfile {
            definition_id: "switch-mx".into(),
        },
        MechanicalChange::SetSwitchFamily {
            definition_id: "switch-mx".into(),
            family: MechanicalSwitchFamily::Mx,
        },
    ];
    for change in projection {
        assert!(
            affects_outline(&change),
            "projection change {change:?} must rebuild outlines"
        );
        assert_eq!(affects_outline(&change), requires_projection(&change));
    }
}
