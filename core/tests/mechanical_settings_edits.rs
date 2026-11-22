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

#[path = "mechanical_settings_edits/settings_dimensions.rs"]
mod settings_dimensions;
#[path = "mechanical_settings_edits/settings_mounts.rs"]
mod settings_mounts;
#[path = "mechanical_settings_edits/settings_profiles.rs"]
mod settings_profiles;
