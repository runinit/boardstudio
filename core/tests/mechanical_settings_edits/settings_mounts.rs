use super::*;

#[test]
fn mount_collections_keep_explicit_empty_and_initialize_absent_ones() {
    // A deliberate explicit empty collection is preserved and never re-initialized.
    let mut doc = document();
    assert_eq!(stored(&doc).closure_mounts, Some(vec![]));
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )
        .closure_mounts,
        Some(vec![]),
        "an unrelated change preserves a deliberate explicit empty collection"
    );
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetMountCollection {
                collection: MechanicalMountCollection::Closure,
                mounts: vec![],
            },
        )
        .unwrap()
        .is_empty(),
        "repeating the explicit empty value is a no-op"
    );

    // An absent collection becomes the explicit empty value on request.
    let mut absent = document_with_outline();
    assert!(stored(&absent).closure_mounts.is_none());
    assert_eq!(
        apply_change(
            &mut absent,
            MechanicalChange::SetMountCollection {
                collection: MechanicalMountCollection::Closure,
                mounts: vec![],
            },
        )
        .unwrap(),
        vec!["board"]
    );
    assert_eq!(stored(&absent).closure_mounts, Some(vec![]));
    assert!(
        absent
            .parts
            .iter()
            .all(|part| !part.id.starts_with("case-closure/")),
        "an explicit empty collection projects no clearance parts"
    );

    // A closure mount added to an explicit empty collection projects its clearance part.
    let changed = apply_change(
        &mut doc,
        MechanicalChange::AddMount {
            collection: MechanicalMountCollection::Closure,
        },
    )
    .unwrap();
    assert!(changed.contains(&"board".to_string()));
    assert!(changed.contains(&"case-mechanical-mount-1".to_string()));
    assert!(
        changed.iter().any(|id| id.starts_with("case-closure/")),
        "the added closure mount projects a generated clearance part"
    );
    assert_eq!(stored(&doc).closure_mounts.as_ref().unwrap().len(), 1);
}

#[test]
fn mount_editing_reports_the_mount_id() {
    let mut doc = document();
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Suspension,
            },
        )
        .unwrap(),
        vec!["board", "case-mechanical-mount-1"]
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetMountDimension {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "case-mechanical-mount-1".into(),
                field: MechanicalDimension::MountHoleDiameter,
                value: 3.0,
            },
        )
        .unwrap(),
        vec!["board", "case-mechanical-mount-1"]
    );
    assert_eq!(stored(&doc).mounts[0].hole_diameter, 3.0);
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "case-mechanical-mount-1".into(),
                at: Vec2 { x: 4.0, y: 5.0 },
            },
        )
        .mounts[0]
            .at,
        Vec2 { x: 4.0, y: 5.0 }
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetMountKind {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "case-mechanical-mount-1".into(),
                kind: MountKind::Boss,
            },
        )
        .mounts[0]
            .kind,
        MountKind::Boss
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveMount {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "case-mechanical-mount-1".into(),
            },
        )
        .unwrap(),
        vec!["board", "case-mechanical-mount-1"]
    );
    assert!(stored(&doc).mounts.is_empty());
}

#[test]
fn openings_and_vertices_apply_and_report_opening_ids() {
    let mut doc = document();
    assert_eq!(
        apply_change(&mut doc, MechanicalChange::AddOpening).unwrap(),
        vec!["board", "opening-1"]
    );
    let opening = &stored(&doc).openings.as_ref().unwrap()[0];
    assert_eq!(opening.id, "opening-1");
    assert_eq!(opening.z, 0.0);
    assert_eq!(opening.height, 10.0);
    assert_eq!(opening.vertices.len(), 4);
    assert_eq!(opening.vertices[0].id, "vertex-1");
    assert_eq!(opening.vertices[0].point, Vec2 { x: -2.5, y: -2.5 });

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetOpeningDimension {
                opening_id: "opening-1".into(),
                vertex_id: None,
                field: MechanicalDimension::OpeningHeight,
                value: 12.0,
            },
        )
        .unwrap(),
        vec!["board", "opening-1"]
    );
    assert_eq!(stored(&doc).openings.as_ref().unwrap()[0].height, 12.0);
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AddOpeningVertex {
                opening_id: "opening-1".into(),
            },
        )
        .unwrap(),
        vec!["board", "opening-1", "vertex-5"]
    );
    assert_eq!(stored(&doc).openings.as_ref().unwrap()[0].vertices.len(), 5);
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::RemoveOpeningVertex {
                opening_id: "opening-1".into(),
                vertex_id: "vertex-1".into(),
            },
        )
        .openings
        .as_ref()
        .unwrap()[0]
            .vertices
            .len(),
        4
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveOpening {
                opening_id: "opening-1".into(),
            },
        )
        .unwrap(),
        vec!["board", "opening-1"]
    );
    assert!(stored(&doc).openings.as_ref().unwrap().is_empty());
}

#[test]
fn gasket_support_dimension_placement_unlinked_and_reset_apply() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().mount = MechanicalMount::Gasket;
    doc.mechanical.as_mut().unwrap().internal_gasket = Some(default_internal_gasket());
    let mut layout = default_gasket_layout();
    layout.supports = vec![anchor("left:0", 0.25)];
    doc.mechanical.as_mut().unwrap().gasket_layout = Some(layout);

    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetGasketSupportDimension {
                support_id: "left:0".into(),
                anchors: vec![anchor("left:0", 0.25)],
                field: MechanicalDimension::GasketSupportLength,
                value: 20.0,
            },
        )
        .gasket_layout
        .as_ref()
        .unwrap()
        .supports[0]
            .length
            .unwrap(),
        20.0
    );
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetGasketSupportDimension {
                support_id: "left:0".into(),
                anchors: vec![anchor("left:0", 0.25)],
                field: MechanicalDimension::GasketSupportLength,
                value: 4.0,
            },
        )
        .is_err()
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetGasketSupportPlacement {
                support_id: "left:0".into(),
                anchors: vec![anchor("left:0", 0.4)],
            },
        )
        .unwrap(),
        vec!["board", "left:0"]
    );
    assert_eq!(
        stored(&doc).gasket_layout.as_ref().unwrap().supports[0].anchor,
        0.4
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetGasketSupportUnlinked {
                support_id: "left:0".into(),
                pair_id: None,
                anchors: vec![anchor("left:0", 0.4)],
            },
        )
        .unwrap(),
        vec!["board", "left:0"]
    );
    assert!(
        stored(&doc).gasket_layout.as_ref().unwrap().supports[0].unlinked,
        "an unlinked support keeps its saved placement"
    );
    assert_eq!(
        apply_change(&mut doc, MechanicalChange::ResetGasketPlacement).unwrap(),
        vec!["board"]
    );
    assert!(
        stored(&doc)
            .gasket_layout
            .as_ref()
            .unwrap()
            .supports
            .is_empty()
    );
}

/// Ported from `legacy_gasket_support_resize_preserves_saved_anchor_metadata`.
#[test]
fn legacy_gasket_support_resize_preserves_saved_anchor_metadata() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().mount = MechanicalMount::Gasket;
    doc.mechanical.as_mut().unwrap().part_processes = Some(vec![]);
    let anchors = vec![anchor("left:0", 0.25), anchor("right:0", 0.75)];
    let mut layout = default_gasket_layout();
    layout.supports = anchors.clone();
    doc.mechanical.as_mut().unwrap().gasket_layout = Some(layout);

    set(
        &mut doc,
        MechanicalChange::SetGasketSupportDimension {
            support_id: "left:0".into(),
            anchors,
            field: MechanicalDimension::GasketSupportLength,
            value: 20.0,
        },
    );

    let supports = &stored(&doc).gasket_layout.as_ref().unwrap().supports;
    assert_eq!(supports.len(), 2);
    for support in supports {
        assert_eq!(support.length, Some(20.0));
        assert_eq!(support.width, Some(3.0), "the width is untouched");
        assert_eq!(
            support.placement,
            Some(GasketPlacement::User),
            "the saved user placement is preserved"
        );
        assert!(!support.unlinked);
    }
    assert_eq!(supports[0].anchor, 0.25);
    assert_eq!(supports[1].anchor, 0.75);
}

#[test]
fn method_mount_style_and_frame_switches_apply() {
    let mut doc = document();
    assert_eq!(
        set(&mut doc, MechanicalChange::SetMethod(PlateMethod::Cnc)).method,
        PlateMethod::Cnc
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetBottomStyle(MechanicalBottomStyle::Sheet),
        )
        .bottom_style,
        Some(MechanicalBottomStyle::Sheet)
    );
    assert_eq!(
        set(&mut doc, MechanicalChange::SetMiddleFrame(true)).middle_frame,
        Some(true)
    );
    set(&mut doc, MechanicalChange::SetIntegratedPlateFrame(true));
    assert!(stored(&doc).integrated_plate_frame);

    set(
        &mut doc,
        MechanicalChange::SetMount(MechanicalMount::Gasket),
    );
    let gasket = stored(&doc);
    assert_eq!(gasket.mount, MechanicalMount::Gasket);
    assert!(gasket.gasket_layout.is_some());
    assert!(gasket.internal_gasket.is_some());
    assert_eq!(gasket.gasket_travel, Some(0.1));
    assert!(
        !gasket.integrated_plate_frame,
        "a gasket mount clears the integrated plate frame"
    );
    assert_eq!(gasket.bottom_style, Some(MechanicalBottomStyle::Shell));
    assert_eq!(gasket.middle_frame, Some(false));
}

/// Ported from
/// `manufacturing_process_edits_route_standard_thickness_and_preserve_other_targets`.
#[test]
fn process_overrides_route_standard_thickness_and_preserve_other_targets() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().part_processes = Some(vec![
        MechanicalPartProcess {
            part_id: "plate".into(),
            method: PlateMethod::Printed,
            material: "ABS".into(),
            thickness: 1.5,
            constraints_version: "2026-09-24".into(),
        },
        MechanicalPartProcess {
            part_id: "bottom".into(),
            method: PlateMethod::Printed,
            material: "PLA".into(),
            thickness: 3.0,
            constraints_version: "2026-09-24".into(),
        },
    ]);

    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetProcessThickness {
                part_id: "plate".into(),
                thickness: 2.2,
            },
        )
        .plate_thickness,
        2.2
    );
    assert_eq!(
        stored(&doc).part_processes.as_ref().unwrap()[0].thickness,
        2.2
    );
    assert_eq!(
        stored(&doc).part_processes.as_ref().unwrap()[1].thickness,
        3.0,
        "the unrelated bottom target keeps its thickness"
    );

    set(
        &mut doc,
        MechanicalChange::SetProcessMethod {
            part_id: "plate".into(),
            method: PlateMethod::Cnc,
        },
    );
    let processes = stored(&doc).part_processes.as_ref().unwrap();
    assert_eq!(stored(&doc).method, PlateMethod::Cnc);
    assert_eq!(processes[0].material, "Aluminium");
    assert_eq!(processes[1].material, "Aluminium");
    assert_eq!(processes[0].constraints_version, "2026-09-24");

    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetProcessMaterial {
                part_id: "plate".into(),
                material: "PLA".into(),
            },
        )
        .is_err(),
        "PLA is not available for CNC"
    );

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveProcessOverride {
                part_id: "plate".into(),
            },
        )
        .unwrap(),
        vec!["board", "plate"]
    );
    let remaining = stored(&doc).part_processes.as_ref().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].part_id, "bottom");
}

/// Ported from `plate_process_thickness_matches_dimension_profile_and_default_foam_sync`.
#[test]
fn plate_process_thickness_matches_the_dimension_route_and_foam_default() {
    let mut base = document();
    {
        let mechanical = base.mechanical.as_mut().unwrap();
        mechanical.plate_foam_thickness = default_plate_foam_thickness(3.5);
        mechanical.profiles.push(MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: Some(MechanicalSwitchFamily::Mx),
            definition_id: "mx-switch".into(),
            source: "fixture MX profile".into(),
            cutouts: vec![],
            plate_to_pcb: 3.5,
        });
        mechanical.part_processes = Some(vec![MechanicalPartProcess {
            part_id: "plate".into(),
            method: PlateMethod::Printed,
            material: "PLA".into(),
            thickness: 1.5,
            constraints_version: "2026-09-24".into(),
        }]);
    }

    let mut process_route = base.clone();
    let mut dimension_route = base;
    apply_change(
        &mut process_route,
        MechanicalChange::SetProcessThickness {
            part_id: "plate".into(),
            thickness: 2.5,
        },
    )
    .unwrap();
    apply_change(
        &mut dimension_route,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value: 2.5,
        },
    )
    .unwrap();

    assert_eq!(
        stored(&process_route).plate_to_pcb,
        stored(&dimension_route).plate_to_pcb
    );
    assert_eq!(
        stored(&process_route).profiles,
        stored(&dimension_route).profiles
    );
    assert_eq!(
        stored(&process_route).plate_foam_thickness,
        stored(&dimension_route).plate_foam_thickness
    );
    assert_eq!(
        stored(&process_route).part_processes,
        stored(&dimension_route).part_processes
    );
    assert_eq!(stored(&process_route).plate_to_pcb, 2.5);
    assert_eq!(stored(&process_route).plate_foam_thickness, 2.3);
    assert_eq!(stored(&dimension_route).plate_to_pcb, 2.5);
    assert_eq!(stored(&dimension_route).plate_foam_thickness, 2.3);
}

#[test]
fn switch_profile_assignment_and_family_change_apply() {
    let mut doc = document();
    doc.boards[0].part_ids = vec!["sw".into()];
    doc.definitions.push(part_definition(
        "switch-mx",
        PartKind::Switch,
        Some(mx_generator()),
    ));
    doc.parts.push(part("sw", "switch-mx", None));

    let changed = apply_change(
        &mut doc,
        MechanicalChange::AssignSwitchProfile {
            definition_id: "switch-mx".into(),
            family: MechanicalSwitchFamily::ChocV1,
        },
    )
    .unwrap();
    assert_eq!(changed, vec!["board", "switch-mx"]);
    assert_eq!(stored(&doc).plate_thickness, 1.3);
    assert_eq!(stored(&doc).plate_to_pcb, 2.2);
    assert_eq!(stored(&doc).plate_foam_thickness, 2.0);
    assert_eq!(stored(&doc).profiles.len(), 1);
    assert_eq!(
        stored(&doc).profiles[0].switch_family,
        Some(MechanicalSwitchFamily::ChocV1)
    );

    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::AssignSwitchProfile {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::ChocV1,
            },
        )
        .unwrap_err()
        .contains("already assigned")
    );

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetSwitchFamily {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::Mx,
            },
        )
        .unwrap(),
        vec!["board", "switch-mx"]
    );
    assert_eq!(stored(&doc).plate_thickness, 1.5);
    assert_eq!(stored(&doc).plate_to_pcb, 3.5);
    assert_eq!(
        stored(&doc).profiles[0].switch_family,
        Some(MechanicalSwitchFamily::Mx)
    );

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveProfile {
                definition_id: "switch-mx".into(),
            },
        )
        .unwrap(),
        vec!["board", "switch-mx"]
    );
    assert!(stored(&doc).profiles.is_empty());
}
