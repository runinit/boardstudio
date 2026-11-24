use super::*;

#[test]
fn dimensions_apply_and_report_the_configuration_owner() {
    let mut doc = document();
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )
        .unwrap(),
        vec!["board"]
    );
    assert_eq!(stored(&doc).wall_thickness, 3.0);
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BottomThickness,
                value: 2.5,
            },
        )
        .bottom_thickness,
        2.5
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::PcbThickness,
                value: 1.2,
            },
        )
        .pcb_thickness,
        1.2
    );
}

#[test]
fn a_change_equal_to_the_accepted_value_is_a_no_op() {
    let mut doc = document();
    let before = stored(&doc).clone();
    let changed = apply_change(
        &mut doc,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 2.0,
        },
    )
    .unwrap();
    assert!(changed.is_empty());
    assert_eq!(stored(&doc), &before);
}

#[test]
fn malformed_dimensions_error_and_mutate_nothing() {
    let mut doc = document();
    let before = stored(&doc).clone();
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: -1.0,
            },
        )
        .is_err()
    );
    assert_eq!(stored(&doc), &before);
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BatteryWidth,
                value: 30.0,
            },
        )
        .is_err(),
        "a battery dimension with no envelope errors"
    );
    assert_eq!(stored(&doc), &before);

    set(&mut doc, MechanicalChange::AddOpening);
    let before = stored(&doc).clone();
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetOpeningDimension {
                opening_id: "missing".into(),
                vertex_id: None,
                field: MechanicalDimension::OpeningHeight,
                value: 4.0,
            },
        )
        .is_err()
    );
    assert_eq!(stored(&doc), &before);
}

#[test]
fn enable_and_disable_create_and_clear_the_configuration() {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards.push(board("board", vec![]));

    let changed = apply_change(&mut doc, MechanicalChange::Enable).unwrap();
    assert_eq!(changed, vec!["board"]);
    let created = stored(&doc).clone();
    assert_eq!(created.board_id, "board");
    assert_eq!(created.method, PlateMethod::Printed);
    assert_eq!(created.mount, MechanicalMount::Tray);
    assert_eq!(created.plate_thickness, 1.5);
    assert_eq!(created.wall_thickness, 2.0);
    assert_eq!(created.bottom_foam_thickness, 2.0);
    assert_eq!(created.part_processes.as_ref().unwrap().len(), 4);
    assert_eq!(
        created.closure_mounts,
        Some(vec![]),
        "enabling initializes the absent closure collection from the resolved assembly"
    );

    assert!(
        apply_change(&mut doc, MechanicalChange::Enable)
            .unwrap()
            .is_empty(),
        "enabling an already enabled target is a no-op"
    );

    let changed = apply_change(&mut doc, MechanicalChange::Disable).unwrap();
    assert_eq!(changed, vec!["board"]);
    assert!(doc.mechanical.is_none());
    assert!(
        apply_change(&mut doc, MechanicalChange::Disable)
            .unwrap()
            .is_empty()
    );
}

/// Ported from `enabling_wired_battery_uses_reference_defaults_without_changing_other_settings`.
#[test]
fn enabling_a_wired_battery_uses_reference_defaults_and_preserves_other_settings() {
    let mut doc = document();
    let before = stored(&doc).clone();

    set(&mut doc, MechanicalChange::SetBatteryEnabled(true));

    let after = stored(&doc);
    assert_eq!(
        after.battery,
        Some(MechanicalBattery {
            cable_width: Some(2.0),
            size: Vec3 {
                x: 30.0,
                y: 20.0,
                z: 6.0,
            },
            at: Vec2 { x: 0.0, y: 0.0 },
            cable_exit: Vec2 { x: 0.0, y: 0.0 },
        })
    );
    assert_eq!(after.board_id, before.board_id);
    assert_eq!(after.method, before.method);
    assert_eq!(after.mount, before.mount);
    assert_eq!(after.plate_thickness, before.plate_thickness);
    assert_eq!(after.plate_to_pcb, before.plate_to_pcb);
    assert_eq!(after.bottom_thickness, before.bottom_thickness);
    assert_eq!(after.part_processes, before.part_processes);
    assert_eq!(after.closure_mounts, before.closure_mounts);
}

/// Ported from `removing_battery_preserves_all_non_battery_settings`.
#[test]
fn removing_a_battery_preserves_all_non_battery_settings() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().battery = Some(MechanicalBattery {
        cable_width: None,
        size: Vec3 {
            x: 42.0,
            y: 18.0,
            z: 5.0,
        },
        at: Vec2 { x: 8.0, y: -3.0 },
        cable_exit: Vec2 { x: 40.0, y: -3.0 },
    });
    let before = stored(&doc).clone();

    let changed = apply_change(&mut doc, MechanicalChange::SetBatteryEnabled(false)).unwrap();

    assert_eq!(changed, vec!["board"]);
    let after = stored(&doc);
    assert!(after.battery.is_none());
    // Every user-authored non-battery setting survives; the effective read path may
    // materialize derived fields such as `batteryHeight` or `stabilizers`.
    assert_eq!(after.board_id, before.board_id);
    assert_eq!(after.method, before.method);
    assert_eq!(after.mount, before.mount);
    assert_eq!(after.plate_thickness, before.plate_thickness);
    assert_eq!(after.plate_to_pcb, before.plate_to_pcb);
    assert_eq!(after.pcb_thickness, before.pcb_thickness);
    assert_eq!(after.bottom_thickness, before.bottom_thickness);
    assert_eq!(after.bottom_foam_thickness, before.bottom_foam_thickness);
    assert_eq!(after.wall_thickness, before.wall_thickness);
    assert_eq!(after.clearance, before.clearance);
    assert_eq!(after.profiles, before.profiles);
    assert_eq!(after.closure_mounts, before.closure_mounts);
}

/// Ported from `battery_fields_update_their_own_values_and_reject_missing_or_invalid_envelopes`.
#[test]
fn battery_dimensions_update_their_own_fields_and_reject_invalid_values() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().battery = Some(MechanicalBattery {
        cable_width: None,
        size: Vec3 {
            x: 30.0,
            y: 20.0,
            z: 6.0,
        },
        at: Vec2 { x: 0.0, y: 0.0 },
        cable_exit: Vec2 { x: 0.0, y: 0.0 },
    });
    for (field, value) in [
        (MechanicalDimension::BatteryWidth, 31.0),
        (MechanicalDimension::BatteryDepth, 21.0),
        (MechanicalDimension::BatteryHeight, 7.0),
        (MechanicalDimension::BatteryCableWidth, 2.5),
        (MechanicalDimension::BatteryPositionX, -12.0),
        (MechanicalDimension::BatteryPositionY, 13.0),
        (MechanicalDimension::BatteryCableExitX, 30.0),
        (MechanicalDimension::BatteryCableExitY, -5.0),
    ] {
        set(&mut doc, MechanicalChange::SetDimension { field, value });
    }
    assert_eq!(
        stored(&doc).battery,
        Some(MechanicalBattery {
            cable_width: Some(2.5),
            size: Vec3 {
                x: 31.0,
                y: 21.0,
                z: 7.0,
            },
            at: Vec2 { x: -12.0, y: 13.0 },
            cable_exit: Vec2 { x: 30.0, y: -5.0 },
        })
    );

    let before = stored(&doc).battery.clone();
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BatteryWidth,
                value: 0.0,
            },
        )
        .is_err(),
        "a non-positive envelope dimension is rejected"
    );
    assert_eq!(stored(&doc).battery, before);

    doc.mechanical.as_mut().unwrap().battery = None;
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BatteryWidth,
                value: 1.0,
            },
        )
        .unwrap_err()
        .contains("removed")
    );
}

/// Ported from `wireless_transport_rejects_manual_battery_toggle`.
#[test]
fn wireless_transport_rejects_a_manual_battery_toggle() {
    let mut doc = document();
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::Wireless,
        instances: vec![],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });

    assert!(
        apply_change(&mut doc, MechanicalChange::SetBatteryEnabled(true))
            .unwrap_err()
            .contains("Wireless")
    );
    assert!(stored(&doc).battery.is_none());
}

#[test]
fn closure_hardware_presets_drive_installation_threads_and_lengths_apply() {
    let mut doc = document();
    doc.mechanical.as_mut().unwrap().internal_gasket = Some(default_internal_gasket());

    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureDrive(ScrewDrive::Torx),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .drive,
        ScrewDrive::Torx
    );

    set(
        &mut doc,
        MechanicalChange::SetClosureInsertPreset("m2-4".into()),
    );
    let hardware = &stored(&doc).internal_gasket.as_ref().unwrap().hardware;
    assert_eq!(hardware.id, "custom-m2-4");
    assert_eq!(hardware.thread, "M2 × 0.4");
    assert_eq!(hardware.thread_diameter, 2.0);
    assert_eq!(hardware.pitch, 0.4);
    assert_eq!(hardware.insert_length, 4.0);
    assert_eq!(hardware.insert_diameter, 3.2);
    assert!((hardware.seat_diameter - 2.8).abs() < 1e-9);
    assert!((hardware.seat_lead_diameter - 3.0).abs() < 1e-9);
    assert_eq!(hardware.seat_depth, 4.5);
    assert_eq!(hardware.engagement, 3.5);
    assert_eq!(hardware.hole_diameter, 2.2);
    assert_eq!(hardware.head_diameter, 4.0);
    assert_eq!(hardware.head_height, 1.0);
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetClosureInsertPreset("nope".into()),
        )
        .is_err()
    );

    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureInstallation(InsertInstallation::Tapped),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .installation,
        InsertInstallation::Tapped
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureThread("M3 × 0.5".into()),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .thread,
        "M3 × 0.5"
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureHeadProfile(ScrewHeadProfile::Countersunk),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .head_profile,
        ScrewHeadProfile::Countersunk
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureLengthDatum(ScrewLengthDatum::Overall),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .length_datum,
        ScrewLengthDatum::Overall
    );

    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureScrewLengths(vec![9.0, 11.0]),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .screw_lengths,
        vec![9.0, 11.0]
    );
    assert_eq!(
        set(&mut doc, MechanicalChange::SetClosureFixedLength(Some(9.0)))
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .fixed_length,
        Some(9.0)
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetClosureScrewLengths(vec![10.0, 12.0]),
        )
        .internal_gasket
        .as_ref()
        .unwrap()
        .hardware
        .fixed_length,
        None,
        "a length no longer offered clears the fixed choice"
    );
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::SetClosureFixedLength(Some(13.0)),
        )
        .is_err(),
        "a length outside the offered set is rejected"
    );
    assert!(apply_change(&mut doc, MechanicalChange::SetClosureScrewLengths(vec![]),).is_err());
}

#[test]
fn critical_fits_and_hardware_apply_and_report_their_own_ids() {
    let mut doc = document();
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AddCriticalFit {
                part_id: "plate".into(),
            },
        )
        .unwrap(),
        vec!["board", "case-critical-fit-1"]
    );
    let fit = &stored(&doc).critical_fits.as_ref().unwrap()[0];
    assert_eq!(fit.label, "Critical dimension");
    assert_eq!(fit.from, Vec2 { x: 0.0, y: 0.0 });
    assert_eq!(fit.to, Vec2 { x: 10.0, y: 0.0 });
    assert_eq!(fit.tolerance, "±0.2 mm");

    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetCriticalFitText {
                fit_id: "case-critical-fit-1".into(),
                field: MechanicalCriticalFitTextField::Label,
                value: "Width".into(),
            },
        )
        .critical_fits
        .as_ref()
        .unwrap()[0]
            .label,
        "Width"
    );
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetCriticalFitPoint {
                fit_id: "case-critical-fit-1".into(),
                field: MechanicalDimension::CriticalFitToX,
                value: 15.0,
            },
        )
        .critical_fits
        .as_ref()
        .unwrap()[0]
            .to
            .x,
        15.0
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveCriticalFit {
                fit_id: "case-critical-fit-1".into(),
            },
        )
        .unwrap(),
        vec!["board", "case-critical-fit-1"]
    );
    assert!(stored(&doc).critical_fits.as_ref().unwrap().is_empty());

    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::AddHardware {
                part_id: "plate".into(),
                feature_id: "feature".into(),
            },
        )
        .unwrap(),
        vec!["board", "case-mechanical-hardware-1"]
    );
    let entry = &stored(&doc).hardware.as_ref().unwrap()[0];
    assert_eq!(entry.designation, "Socket screw");
    assert_eq!(entry.thread, "M2 × 0.4");
    assert_eq!(entry.length, 6.0);
    assert_eq!(entry.quantity, 4);
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::SetHardwareDimension {
                hardware_id: "case-mechanical-hardware-1".into(),
                field: MechanicalDimension::HardwareQuantity,
                value: 6.0,
            },
        )
        .unwrap(),
        vec!["board", "case-mechanical-hardware-1"]
    );
    assert_eq!(stored(&doc).hardware.as_ref().unwrap()[0].quantity, 6);
    assert_eq!(
        set(
            &mut doc,
            MechanicalChange::SetHardwareText {
                hardware_id: "case-mechanical-hardware-1".into(),
                field: MechanicalHardwareTextField::Designation,
                value: "M2 screw".into(),
            },
        )
        .hardware
        .as_ref()
        .unwrap()[0]
            .designation,
        "M2 screw"
    );
    assert_eq!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveHardware {
                hardware_id: "case-mechanical-hardware-1".into(),
            },
        )
        .unwrap(),
        vec!["board", "case-mechanical-hardware-1"]
    );
    assert!(stored(&doc).hardware.as_ref().unwrap().is_empty());
    assert!(
        apply_change(
            &mut doc,
            MechanicalChange::RemoveHardware {
                hardware_id: "missing".into(),
            },
        )
        .is_err()
    );
}
