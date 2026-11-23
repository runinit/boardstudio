//! Shared unit-test fixtures for mechanical resolution tests.
//!
//! Every builder here replaces two or more previously duplicated copies.
//! Builders whose key configuration values differ across suites keep distinct
//! names (for example [`configuration`] versus [`rigid_configuration`]);
//! callers override individual fields when a test depends on specific metadata
//! or aliasing.

use crate::model::*;
use serde_json::json;

pub(crate) fn board(id: &str, part_ids: Vec<String>, thickness: f64) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids,
        net_ids: vec![],
        thickness,
        traces: vec![],
        vias: vec![],
    }
}

pub(crate) fn document_with(board: Board, hardware: HardwareConfiguration) -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board);
    document.hardware = Some(hardware);
    document
}

/// Tray configuration used by the inputs and features suites.
pub(crate) fn configuration(board_id: &str) -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": board_id,
        "method": "printed",
        "mount": "tray",
        "plateThickness": 1.5,
        "plateFoamThickness": 1.0,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.5,
        "batteryHeight": 0.0,
        "bottomThickness": 2.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.2,
        "profiles": []
    }))
    .unwrap()
}

/// Resolution-suite configuration: the tray configuration with a battery height.
pub(crate) fn config() -> MechanicalConfiguration {
    let mut configuration = configuration("board");
    configuration.battery_height = 3.0;
    configuration
}

/// Rigid configuration shared by the replacement and identity suites.
pub(crate) fn rigid_configuration(board_id: &str) -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": board_id,
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
        "mounts": [],
        "closureMounts": [],
        "profiles": []
    }))
    .expect("valid mechanical fixture")
}

/// Edit-suite configuration: the rigid configuration with shell styling.
pub(crate) fn edit_configuration() -> MechanicalConfiguration {
    let mut configuration = rigid_configuration("board");
    configuration.bottom_style = Some(MechanicalBottomStyle::Shell);
    configuration.middle_frame = Some(false);
    configuration.opening_allowance = Some(0.25);
    configuration
}

pub(crate) fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        hole: false,
        points: vec![
            Vec2 { x: x0, y: y0 },
            Vec2 { x: x1, y: y0 },
            Vec2 { x: x1, y: y1 },
            Vec2 { x: x0, y: y1 },
        ],
    }
}

pub(crate) fn part_at(id: &str, definition_id: &str, x: f64, y: f64) -> Part {
    Part {
        anchor: None,
        id: id.into(),
        definition_id: definition_id.into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    }
}

pub(crate) fn part_full(
    id: &str,
    definition_id: &str,
    reference: &str,
    x: f64,
    y: f64,
    rotation: f64,
    side: Side,
) -> Part {
    Part {
        anchor: None,
        id: id.into(),
        definition_id: definition_id.into(),
        reference: reference.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation,
        },
        side,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    }
}

/// The normalized bundled mounting-hole catalogue entry tests project.
pub(crate) fn mounting_hole_template() -> PartDefinition {
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

pub(crate) fn instance(
    id: &str,
    board_id: &str,
    ownership: MechanicalConstructionOwnership,
    mechanical: Option<MechanicalConfiguration>,
) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: "left".into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical,
        construction_ownership: ownership,
        mechanical_identity_reservation: None,
    }
}

pub(crate) fn hardware(
    instances: Vec<PhysicalBoardInstance>,
    shared: Option<MechanicalConfiguration>,
    transport: HardwareTransport,
) -> HardwareConfiguration {
    HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport,
        instances,
        boards: vec![],
        shared_construction: shared,
        pairs: vec![],
    }
}

pub(crate) fn mount(id: &str, x: f64, y: f64) -> Mount {
    Mount {
        id: id.into(),
        at: Vec2 { x, y },
        kind: MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    }
}

pub(crate) fn opening(id: &str) -> MechanicalOpening {
    MechanicalOpening {
        id: id.into(),
        vertices: vec![
            MechanicalOpeningVertex {
                id: "v0".into(),
                point: Vec2 { x: 0.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "v1".into(),
                point: Vec2 { x: 3.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "v2".into(),
                point: Vec2 { x: 3.0, y: 1.0 },
            },
            MechanicalOpeningVertex {
                id: "v3".into(),
                point: Vec2 { x: 1.0, y: 2.5 },
            },
        ],
        z: 1.25,
        height: 3.5,
        next_vertex_identity: 0,
    }
}

pub(crate) fn anchor(id: &str) -> MechanicalGasketAnchor {
    anchor_at(id, 0.25)
}

pub(crate) fn anchor_at(id: &str, anchor: f64) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        length: Some(12.0),
        width: Some(3.0),
        placement: Some(GasketPlacement::User),
        id: id.into(),
        region_id: "region".into(),
        outline_key: "outline".into(),
        anchor,
        unlinked: false,
    }
}

pub(crate) fn battery() -> MechanicalBattery {
    serde_json::from_value(json!({
        "size": { "x": 30.0, "y": 20.0, "z": 6.0 },
        "at": { "x": 1.0, "y": 2.0 },
        "cableExit": { "x": 3.0, "y": 2.0 }
    }))
    .unwrap()
}

pub(crate) fn profile(id: &str) -> MechanicalPartProfile {
    serde_json::from_value(json!({
        "definitionId": id, "source": "test", "cutouts": [], "plateToPcb": 3.5
    }))
    .unwrap()
}

pub(crate) fn stabilizer() -> MechanicalStabilizerOverride {
    serde_json::from_value(json!({
        "partId": "part", "kind": "pcb-mount", "units": 2.0
    }))
    .unwrap()
}

pub(crate) fn hardware_specification(id: &str) -> MechanicalHardwareSpecification {
    serde_json::from_value(json!({
        "id": id, "partId": "part", "featureId": "feature", "designation": "M2",
        "thread": "M2", "length": 8.0, "quantity": 1
    }))
    .unwrap()
}

pub(crate) fn critical_fit(id: &str) -> MechanicalCriticalFit {
    serde_json::from_value(json!({
        "id": id, "partId": "part", "label": "fit",
        "from": { "x": 0.0, "y": 0.0 }, "to": { "x": 1.0, "y": 1.0 },
        "tolerance": "0.1"
    }))
    .unwrap()
}

pub(crate) fn gasket_layout(supports: Vec<MechanicalGasketAnchor>) -> MechanicalGasketLayout {
    MechanicalGasketLayout {
        auto_size: Some(true),
        adhesive_thickness: Some(0.1),
        minimum_foam_thickness: Some(0.5),
        preset_id: None,
        material: Some("Poron".into()),
        length: 99.0,
        width: 3.0,
        thickness: 2.0,
        compression: 0.15,
        supports,
    }
}

pub(crate) fn pose(at_x: f64, at_y: f64, rotation: f64) -> Pose2 {
    Pose2 {
        at: Vec2 { x: at_x, y: at_y },
        rotation,
    }
}

pub(crate) fn mirror_pair(
    id: &str,
    members: [&str; 2],
    mapping: Pose2,
    construction: MechanicalConstruction,
    features: Vec<MechanicalFeature>,
) -> MechanicalMirrorPair {
    MechanicalMirrorPair {
        id: id.into(),
        members: [members[0].into(), members[1].into()],
        mapping,
        construction,
        construction_source: None,
        halves: None,
        features,
    }
}

/// Pair construction used by the inputs-suite overlay tests.
pub(crate) fn pair_construction() -> MechanicalConstruction {
    serde_json::from_value(json!({
        "method": "cnc", "mount": "rigid", "integratedPlateFrame": true,
        "plateThickness": 7.0, "plateFoamThickness": 0.5,
        "bottomFoamThickness": 0.25, "bottomThickness": 3.0,
        "plateToPcb": 4.0, "wallThickness": 2.5, "clearance": 0.3
    }))
    .unwrap()
}

#[track_caller]
pub(crate) fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[track_caller]
pub(crate) fn assert_point(actual: Vec2, expected: Vec2) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}
