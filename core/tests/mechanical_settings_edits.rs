//! Integration tests for `boardstudio_core::mechanical::edit::apply`.
//!
//! These are the mechanical-settings rules exercised through the public operation:
//! a typed [`MechanicalEdit`] against a [`ProjectDoc`], asserting the stored
//! configuration, the reported changed ids and the rejected values. They were
//! ported from the Case controller's `battery_patch_tests` (its behavioural cases)
//! and cover the settings families the operation owns: battery, closure hardware,
//! critical fits, hardware, mount collections, openings, gasket supports,
//! method/mount/frame switches, dimensions, processes, profiles, stabilizers and
//! the switch family.

use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension, MechanicalEdit,
    MechanicalHardwareTextField, MechanicalMountCollection, MechanicalTarget,
};
use boardstudio_core::mechanical::edit::{affects_outline, apply, requires_projection};
use boardstudio_core::mechanical::{
    default_gasket_layout, default_internal_gasket, default_plate_foam_thickness,
};
use boardstudio_core::model::{
    Board, GasketPlacement, HardwareConfiguration, HardwareTopology, HardwareTransport,
    InsertInstallation, KicadSource, MechanicalBattery, MechanicalBottomStyle,
    MechanicalBuiltinProfile, MechanicalConfiguration, MechanicalConstructionOwnership,
    MechanicalGasketAnchor, MechanicalMount, MechanicalPartProcess, MechanicalPartProfile,
    MechanicalProfileSource, MechanicalStabilizerKind, MechanicalStabilizerOverride,
    MechanicalSwitchFamily, MountKind, Operation, OutlineFeature, Part, PartDefinition,
    PartGenerator, PartKind, PhysicalBoardInstance, PlateMethod, Pose2, ProjectDoc, ScrewDrive,
    ScrewHeadProfile, ScrewLengthDatum, Side, Vec2, Vec3,
};
use serde_json::json;
use std::collections::BTreeMap;

fn board(id: &str, part_ids: Vec<String>) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids,
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

/// The stored configuration most settings tests start from.
///
/// `closureMounts` is a deliberate explicit empty collection, so an unrelated
/// settings change never triggers closure initialization or its required template.
fn configuration() -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": "board",
        "method": "printed",
        "mount": "rigid",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.5,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.4,
        "batteryHeight": 4.0,
        "bottomThickness": 3.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.3,
        "bottomStyle": "shell",
        "middleFrame": false,
        "openingAllowance": 0.25,
        "mounts": [],
        "closureMounts": [],
        "profiles": []
    }))
    .unwrap()
}

/// The same configuration with absent closure mounts, for initialization tests.
fn configuration_without_closures() -> MechanicalConfiguration {
    let mut configuration = configuration();
    configuration.closure_mounts = None;
    configuration
}

/// The normalized bundled mounting-hole catalogue entry projection expects.
fn template() -> PartDefinition {
    serde_json::from_value(json!({
        "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
        "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
    }))
    .unwrap()
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board", vec![]));
    document.mechanical = Some(configuration());
    document
}

/// A board whose outline yields at least one suggested closure mount.
fn document_with_outline() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 80.0, y: 60.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    });
    document.boards.push(board("board", vec![]));
    document.boards[0].outline_ids = vec!["outline".into()];
    document.mechanical = Some(configuration_without_closures());
    document
}

fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn instance_target() -> MechanicalTarget {
    MechanicalTarget::Instance {
        instance_id: "left".into(),
    }
}

fn apply_change(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
    apply(
        doc,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change,
            mounting_hole: Some(Box::new(template())),
        },
    )
}

fn apply_instance(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
    apply(
        doc,
        &[],
        &MechanicalEdit {
            target: instance_target(),
            change,
            mounting_hole: Some(Box::new(template())),
        },
    )
}

fn stored(doc: &ProjectDoc) -> &MechanicalConfiguration {
    doc.mechanical.as_ref().expect("stored configuration")
}

fn part_definition(id: &str, kind: PartKind, generator: Option<PartGenerator>) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: BTreeMap::new(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator,
        mechanical_profile: None,
    }
}

fn part(id: &str, definition_id: &str, keycap: Option<Vec2>) -> Part {
    Part {
        anchor: None,
        keycap,
        outline: None,
        id: id.into(),
        definition_id: definition_id.into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn mx_generator() -> PartGenerator {
    PartGenerator {
        source: "ceoloide/switch_mx".into(),
        version: "1".into(),
        parameters: BTreeMap::new(),
    }
}

fn anchor(id: &str, anchor: f64) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        length: Some(12.0),
        width: Some(3.0),
        placement: Some(GasketPlacement::User),
        id: id.into(),
        region_id: id.into(),
        outline_key: id.into(),
        anchor,
        unlinked: false,
    }
}

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
    apply_change(
        &mut doc,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::BottomThickness,
            value: 2.5,
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).bottom_thickness, 2.5);
    apply_change(
        &mut doc,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::PcbThickness,
            value: 1.2,
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).pcb_thickness, 1.2);
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

    apply_change(&mut doc, MechanicalChange::AddOpening).unwrap();
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

    apply_change(&mut doc, MechanicalChange::SetBatteryEnabled(true)).unwrap();

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
        apply_change(&mut doc, MechanicalChange::SetDimension { field, value }).unwrap();
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

    apply_change(
        &mut doc,
        MechanicalChange::SetClosureDrive(ScrewDrive::Torx),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .drive,
        ScrewDrive::Torx
    );

    apply_change(
        &mut doc,
        MechanicalChange::SetClosureInsertPreset("m2-4".into()),
    )
    .unwrap();
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

    apply_change(
        &mut doc,
        MechanicalChange::SetClosureInstallation(InsertInstallation::Tapped),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .installation,
        InsertInstallation::Tapped
    );
    apply_change(
        &mut doc,
        MechanicalChange::SetClosureThread("M3 × 0.5".into()),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .thread,
        "M3 × 0.5"
    );
    apply_change(
        &mut doc,
        MechanicalChange::SetClosureHeadProfile(ScrewHeadProfile::Countersunk),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .head_profile,
        ScrewHeadProfile::Countersunk
    );
    apply_change(
        &mut doc,
        MechanicalChange::SetClosureLengthDatum(ScrewLengthDatum::Overall),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .length_datum,
        ScrewLengthDatum::Overall
    );

    apply_change(
        &mut doc,
        MechanicalChange::SetClosureScrewLengths(vec![9.0, 11.0]),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .screw_lengths,
        vec![9.0, 11.0]
    );
    apply_change(&mut doc, MechanicalChange::SetClosureFixedLength(Some(9.0))).unwrap();
    assert_eq!(
        stored(&doc)
            .internal_gasket
            .as_ref()
            .unwrap()
            .hardware
            .fixed_length,
        Some(9.0)
    );
    apply_change(
        &mut doc,
        MechanicalChange::SetClosureScrewLengths(vec![10.0, 12.0]),
    )
    .unwrap();
    assert_eq!(
        stored(&doc)
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

    apply_change(
        &mut doc,
        MechanicalChange::SetCriticalFitText {
            fit_id: "case-critical-fit-1".into(),
            field: MechanicalCriticalFitTextField::Label,
            value: "Width".into(),
        },
    )
    .unwrap();
    assert_eq!(
        stored(&doc).critical_fits.as_ref().unwrap()[0].label,
        "Width"
    );
    apply_change(
        &mut doc,
        MechanicalChange::SetCriticalFitPoint {
            fit_id: "case-critical-fit-1".into(),
            field: MechanicalDimension::CriticalFitToX,
            value: 15.0,
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).critical_fits.as_ref().unwrap()[0].to.x, 15.0);
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
    apply_change(
        &mut doc,
        MechanicalChange::SetHardwareText {
            hardware_id: "case-mechanical-hardware-1".into(),
            field: MechanicalHardwareTextField::Designation,
            value: "M2 screw".into(),
        },
    )
    .unwrap();
    assert_eq!(
        stored(&doc).hardware.as_ref().unwrap()[0].designation,
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

#[test]
fn mount_collections_keep_explicit_empty_and_initialize_absent_ones() {
    // A deliberate explicit empty collection is preserved and never re-initialized.
    let mut doc = document();
    assert_eq!(stored(&doc).closure_mounts, Some(vec![]));
    apply_change(
        &mut doc,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
    )
    .unwrap();
    assert_eq!(
        stored(&doc).closure_mounts,
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
    apply_change(
        &mut doc,
        MechanicalChange::SetMountPosition {
            collection: MechanicalMountCollection::Suspension,
            mount_id: "case-mechanical-mount-1".into(),
            at: Vec2 { x: 4.0, y: 5.0 },
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).mounts[0].at, Vec2 { x: 4.0, y: 5.0 });
    apply_change(
        &mut doc,
        MechanicalChange::SetMountKind {
            collection: MechanicalMountCollection::Suspension,
            mount_id: "case-mechanical-mount-1".into(),
            kind: MountKind::Boss,
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).mounts[0].kind, MountKind::Boss);
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
    apply_change(
        &mut doc,
        MechanicalChange::RemoveOpeningVertex {
            opening_id: "opening-1".into(),
            vertex_id: "vertex-1".into(),
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).openings.as_ref().unwrap()[0].vertices.len(), 4);
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

    apply_change(
        &mut doc,
        MechanicalChange::SetGasketSupportDimension {
            support_id: "left:0".into(),
            anchors: vec![anchor("left:0", 0.25)],
            field: MechanicalDimension::GasketSupportLength,
            value: 20.0,
        },
    )
    .unwrap();
    assert_eq!(
        stored(&doc).gasket_layout.as_ref().unwrap().supports[0]
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

    apply_change(
        &mut doc,
        MechanicalChange::SetGasketSupportDimension {
            support_id: "left:0".into(),
            anchors,
            field: MechanicalDimension::GasketSupportLength,
            value: 20.0,
        },
    )
    .unwrap();

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
    apply_change(&mut doc, MechanicalChange::SetMethod(PlateMethod::Cnc)).unwrap();
    assert_eq!(stored(&doc).method, PlateMethod::Cnc);
    apply_change(
        &mut doc,
        MechanicalChange::SetBottomStyle(MechanicalBottomStyle::Sheet),
    )
    .unwrap();
    assert_eq!(
        stored(&doc).bottom_style,
        Some(MechanicalBottomStyle::Sheet)
    );
    apply_change(&mut doc, MechanicalChange::SetMiddleFrame(true)).unwrap();
    assert_eq!(stored(&doc).middle_frame, Some(true));
    apply_change(&mut doc, MechanicalChange::SetIntegratedPlateFrame(true)).unwrap();
    assert!(stored(&doc).integrated_plate_frame);

    apply_change(
        &mut doc,
        MechanicalChange::SetMount(MechanicalMount::Gasket),
    )
    .unwrap();
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

    apply_change(
        &mut doc,
        MechanicalChange::SetProcessThickness {
            part_id: "plate".into(),
            thickness: 2.2,
        },
    )
    .unwrap();
    assert_eq!(stored(&doc).plate_thickness, 2.2);
    assert_eq!(
        stored(&doc).part_processes.as_ref().unwrap()[0].thickness,
        2.2
    );
    assert_eq!(
        stored(&doc).part_processes.as_ref().unwrap()[1].thickness,
        3.0,
        "the unrelated bottom target keeps its thickness"
    );

    apply_change(
        &mut doc,
        MechanicalChange::SetProcessMethod {
            part_id: "plate".into(),
            method: PlateMethod::Cnc,
        },
    )
    .unwrap();
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
