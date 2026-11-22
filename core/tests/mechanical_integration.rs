//! Integrated mechanical verification at the Session/Core seam.
//!
//! These cases exercise the approved public Core contract through real edits: the
//! synchronous mechanical evaluator, the effective mechanical read path and the
//! atomic pair behavior. They complement the per-slice unit tests by driving the
//! same behavior through `CoreEngine` `Edit`/`Undo` and the public mechanical
//! module functions.

use crate::fixtures::mechanical_configuration as configuration;
use crate::fixtures::mounting_hole_template as template;
use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalDimension, MechanicalEdit, MechanicalFeatureCorrespondence,
    MechanicalMountCollection, MechanicalTarget,
};
#[path = "mechanical_integration/mech_chunk0.rs"]
mod mech_chunk0;
#[path = "mechanical_integration/mech_chunk1.rs"]
mod mech_chunk1;
#[path = "mechanical_integration/mech_chunk2.rs"]
mod mech_chunk2;
use boardstudio_core::mechanical::{MechanicalEditDisposition, evaluate_edit};
use boardstudio_core::model::{
    Board, BoardContours, CaseBody, CaseKind, Contour, HardwareConfiguration, HardwareTopology,
    HardwareTransport, MechanicalConfiguration, MechanicalConstructionOwnership,
    MechanicalFeatureKind, MechanicalFeatureValue, MechanicalOpening, Mount, MountKind, Operation,
    OutlineFeature, PhysicalBoardInstance, Pose2, ProjectDoc, Vec2,
};
use serde_json::json;

fn board(id: &str) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

/// The stored configuration the evaluator cases start from.

fn hardware(instances: Vec<PhysicalBoardInstance>) -> HardwareConfiguration {
    HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances,
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    }
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board"));
    document.mechanical = Some(configuration());
    document
}

fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn edit(change: MechanicalChange) -> MechanicalEdit {
    MechanicalEdit {
        target: board_target(),
        change,
        mounting_hole: None,
    }
}

fn plate_thickness(value: f64) -> MechanicalEdit {
    edit(MechanicalChange::SetDimension {
        field: MechanicalDimension::PlateThickness,
        value,
    })
}

// ---------------------------------------------------------------------------
// Linked mirror pairs: the single authored value and its derived counterpart.
// ---------------------------------------------------------------------------

fn mount(id: &str, x: f64, y: f64) -> Mount {
    Mount {
        id: id.into(),
        at: Vec2 { x, y },
        kind: MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    }
}

fn mirror_instance(id: &str, mounts: Vec<Mount>) -> PhysicalBoardInstance {
    mirror_instance_on(id, "board", mounts)
}

fn mirror_instance_on(id: &str, board_id: &str, mounts: Vec<Mount>) -> PhysicalBoardInstance {
    let mut mechanical = configuration();
    mechanical.board_id = board_id.into();
    mechanical.mounts = mounts;
    instance(
        id,
        board_id,
        Some(mechanical),
        MechanicalConstructionOwnership::Local,
    )
}

fn instance(
    id: &str,
    board_id: &str,
    mechanical: Option<MechanicalConfiguration>,
    construction_ownership: MechanicalConstructionOwnership,
) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical,
        construction_ownership,
        mechanical_identity_reservation: None,
    }
}

/// Two physical halves on one board, each with its own local mount geometry.
fn pair_document() -> ProjectDoc {
    let mut document = document();
    document.hardware = Some(hardware(vec![
        mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]),
        mirror_instance("right", vec![mount("mount-1", 7.0, 3.0)]),
    ]));
    document
}

fn instance_target(id: &str) -> MechanicalTarget {
    MechanicalTarget::Instance {
        instance_id: id.into(),
    }
}

fn with_template(target: MechanicalTarget, change: MechanicalChange) -> MechanicalEdit {
    MechanicalEdit {
        target,
        change,
        mounting_hole: Some(Box::new(template())),
    }
}

/// Links the two halves with an explicit zero-origin reflection (no guessing).
fn link_pair(document: &mut ProjectDoc) {
    link_with_correspondences(
        document,
        vec![correspondence(
            MechanicalFeatureKind::Mount,
            "mount-1",
            "mount-1",
            None,
        )],
    )
    .unwrap();
}

fn linked_pair_document() -> ProjectDoc {
    let mut document = pair_document();
    link_pair(&mut document);
    document
}

fn pair(document: &ProjectDoc) -> &boardstudio_core::model::MechanicalMirrorPair {
    &document.hardware.as_ref().unwrap().pairs[0]
}

fn apply_with_template(
    document: &mut ProjectDoc,
    target: MechanicalTarget,
    change: MechanicalChange,
) -> Result<Vec<String>, String> {
    boardstudio_core::mechanical::edit::apply(document, &[], &with_template(target, change))
}

fn instance_inputs(
    document: &ProjectDoc,
    id: &str,
) -> boardstudio_core::mechanical::MechanicalInputs {
    boardstudio_core::mechanical::effective_inputs(document, &[], &instance_target(id))
        .unwrap_or_else(|error| panic!("effective mechanical inputs for {id}: {error:?}"))
}

fn move_mount(at: Vec2) -> MechanicalChange {
    MechanicalChange::SetMountPosition {
        collection: MechanicalMountCollection::Suspension,
        mount_id: "mount-1".into(),
        at,
    }
}

fn delete_mount() -> MechanicalChange {
    MechanicalChange::RemoveMount {
        collection: MechanicalMountCollection::Suspension,
        mount_id: "mount-1".into(),
    }
}

fn local_mount(id: &str) -> MechanicalChange {
    MechanicalChange::MakeFeatureLocal {
        feature_id: "mount-1".into(),
        instance_id: id.into(),
    }
}

fn stored_mounts(document: &ProjectDoc, instance_id: &str) -> Vec<Mount> {
    stored_configuration(document, instance_id).mounts.clone()
}

fn mount_at(configuration: &MechanicalConfiguration, id: &str) -> Vec2 {
    configuration
        .mounts
        .iter()
        .find(|mount| mount.id == id)
        .unwrap_or_else(|| panic!("no mount {id:?}"))
        .at
}

// ---------------------------------------------------------------------------
// Mounting-hole template validation and projection-optional edits.
// ---------------------------------------------------------------------------

/// A legacy half with no stored configuration of its own, on its own board.
fn unconfigured_legacy(id: &str) -> PhysicalBoardInstance {
    instance(
        id,
        &format!("board-{id}"),
        None,
        MechanicalConstructionOwnership::Legacy,
    )
}

fn stored_closure_mounts(document: &ProjectDoc, instance_id: &str) -> Option<Vec<Mount>> {
    stored_instance(document, instance_id)
        .mechanical
        .as_ref()
        .and_then(|configuration| configuration.closure_mounts.clone())
}

/// Two legacy halves, each on its own board, and the accepted contours of both.
fn two_unconfigured_halves() -> (ProjectDoc, Vec<BoardContours>) {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board-a"));
    document.boards.push(board("board-b"));
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Split,
        transport: HardwareTransport::Wired,
        ..hardware(vec![unconfigured_legacy("a"), unconfigured_legacy("b")])
    });
    let accepted = vec![
        BoardContours {
            board_id: "board-a".into(),
            contours: vec![small_rect()],
        },
        BoardContours {
            board_id: "board-b".into(),
            contours: vec![small_rect()],
        },
    ];
    (document, accepted)
}

fn enable_edit(id: &str) -> MechanicalEdit {
    with_template(instance_target(id), MechanicalChange::Enable)
}

fn small_rect() -> Contour {
    rect_contour(10.0, 5.0)
}

fn closure_mounts(document: &ProjectDoc) -> Vec<Mount> {
    document
        .mechanical
        .as_ref()
        .unwrap()
        .closure_mounts
        .clone()
        .expect("closure mounts were initialized")
}

fn pair_document_with_two_mounts() -> ProjectDoc {
    let mut document = document();
    document.hardware = Some(hardware(vec![
        mirror_instance(
            "left",
            vec![mount("mount-1", 2.0, 3.0), mount("mount-2", 4.0, 5.0)],
        ),
        mirror_instance(
            "right",
            vec![mount("mount-1", 7.0, 3.0), mount("mount-2", 9.0, 5.0)],
        ),
    ]));
    document
}

fn correspondence(
    kind: MechanicalFeatureKind,
    first: &str,
    second: &str,
    source: Option<&str>,
) -> MechanicalFeatureCorrespondence {
    MechanicalFeatureCorrespondence {
        kind,
        first_feature_id: first.into(),
        second_feature_id: second.into(),
        source_instance_id: source.map(str::to_string),
    }
}

fn link_with_correspondences(
    document: &mut ProjectDoc,
    correspondences: Vec<MechanicalFeatureCorrespondence>,
) -> Result<Vec<String>, String> {
    boardstudio_core::mechanical::edit::apply(
        document,
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
                correspondences,
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
}

fn remove_member(document: &mut ProjectDoc, instance_id: &str) -> Result<Vec<String>, String> {
    boardstudio_core::mechanical::edit::apply(
        document,
        &[],
        &MechanicalEdit {
            target: instance_target(instance_id),
            change: MechanicalChange::RemovePhysicalInstance,
            mounting_hole: Some(Box::new(template())),
        },
    )
}

/// Gives the fixture board a real outline so export readiness is meaningful.
fn with_board_outline(document: &mut ProjectDoc) {
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 80.0, y: 60.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    });
    document.boards[0].outline_ids = vec!["outline".into()];
    document.case_bodies.push(CaseBody {
        features: None,
        openings: None,
        id: "case-body".into(),
        name: "Case body".into(),
        board_id: "board".into(),
        kind: CaseKind::Tray,
        thickness: 3.0,
        clearance: 0.3,
        material_id: None,
        z: None,
        wall_height: None,
        wall_thickness: Some(2.0),
        mounts: None,
        pcb_source: None,
        gasket: None,
    });
}

/// Two halves where the linked mount reflects onto the second half's local hardware.
fn conflicting_pair_document() -> ProjectDoc {
    let mut document = document();
    with_board_outline(&mut document);
    document.hardware = Some(hardware(vec![
        mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]),
        mirror_instance(
            "right",
            vec![
                mount("mount-1", 7.0, 3.0),
                mount("mount-encoder", -2.0, 3.0),
            ],
        ),
    ]));
    document
}

fn add_opening(document: &mut ProjectDoc) -> String {
    use boardstudio_core::mechanical::edit::apply;
    apply(
        document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::AddOpening,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    document
        .mechanical
        .as_ref()
        .unwrap()
        .openings
        .as_ref()
        .unwrap()
        .last()
        .unwrap()
        .id
        .clone()
}

fn opening_json(id: &str) -> MechanicalOpening {
    serde_json::from_value(json!({
        "id": id,
        "vertices": [
            { "id": "vertex-1", "point": { "x": -2.0, "y": -2.0 } },
            { "id": "vertex-2", "point": { "x": 2.0, "y": -2.0 } },
            { "id": "vertex-3", "point": { "x": 2.0, "y": 2.0 } },
            { "id": "vertex-4", "point": { "x": -2.0, "y": 2.0 } }
        ],
        "z": 0.0,
        "height": 1.0
    }))
    .unwrap()
}

/// Two halves whose authored opening has the same identity on both sides.
fn pair_document_with_openings() -> ProjectDoc {
    let mut document = pair_document();
    for instance in document.hardware.as_mut().unwrap().instances.iter_mut() {
        instance.mechanical.as_mut().unwrap().openings = Some(vec![opening_json("opening-1")]);
    }
    document
}

fn add_instance_opening(document: &mut ProjectDoc, instance_id: &str) -> String {
    use boardstudio_core::mechanical::edit::apply;
    apply(
        document,
        &[],
        &MechanicalEdit {
            target: instance_target(instance_id),
            change: MechanicalChange::AddOpening,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap()
    .into_iter()
    .find(|id| id.starts_with("opening-"))
    .expect("a new opening identity is reported")
}

fn rect_contour(half_width: f64, half_height: f64) -> Contour {
    Contour {
        hole: false,
        points: vec![
            Vec2 {
                x: -half_width,
                y: -half_height,
            },
            Vec2 {
                x: half_width,
                y: -half_height,
            },
            Vec2 {
                x: half_width,
                y: half_height,
            },
            Vec2 {
                x: -half_width,
                y: half_height,
            },
        ],
    }
}

/// Two halves on distinct boards, linked by a closure mount that reflects off the
/// narrower board.
fn cross_board_closure_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("wide"));
    document.boards.push(board("narrow"));
    let mut left = mirror_instance_on("left", "wide", vec![]);
    let mut right = mirror_instance_on("right", "narrow", vec![]);
    left.mechanical.as_mut().unwrap().closure_mounts = Some(vec![mount("closure-1", 30.0, 0.0)]);
    right.mechanical.as_mut().unwrap().closure_mounts = Some(vec![mount("closure-1", 30.0, 0.0)]);
    document.hardware = Some(hardware(vec![left, right]));
    document
}

/// One unpaired physical half that already carries an authored opening.
fn document_with_an_opening() -> ProjectDoc {
    let mut document = document();
    let mut instance = mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]);
    instance.mechanical.as_mut().unwrap().openings = Some(vec![opening_json("opening-1")]);
    document.hardware = Some(hardware(vec![instance]));
    document
}

/// Compares two points within floating-point tolerance.
fn assert_close(actual: Vec2, expected: Vec2, label: &str) {
    assert!(
        (actual.x - expected.x).abs() < 1e-9 && (actual.y - expected.y).abs() < 1e-9,
        "{label}: expected {expected:?}, got {actual:?}"
    );
}

/// Two halves on distinct boards whose mechanical stack uses discrete gasket supports.
fn gasket_pair_document() -> (ProjectDoc, Vec<BoardContours>) {
    gasket_pair("wide", "narrow", (10.0, 5.0))
}

fn gasket_pair(
    left: &str,
    right: &str,
    right_extent: (f64, f64),
) -> (ProjectDoc, Vec<BoardContours>) {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board(left));
    let mut accepted = vec![BoardContours {
        board_id: left.into(),
        contours: vec![rect_contour(200.0, 150.0)],
    }];
    if right != left {
        document.boards.push(board(right));
        accepted.push(BoardContours {
            board_id: right.into(),
            contours: vec![rect_contour(right_extent.0, right_extent.1)],
        });
    }
    let instances = [("left", left), ("right", right)].map(|(id, board)| {
        let mut configuration = configuration();
        configuration.board_id = board.into();
        configuration.mount = boardstudio_core::model::MechanicalMount::Gasket;
        configuration.gasket_layout = Some(boardstudio_core::mechanical::default_gasket_layout());
        instance(
            id,
            board,
            Some(configuration),
            MechanicalConstructionOwnership::Local,
        )
    });
    document.hardware = Some(hardware(instances.into()));
    (document, accepted)
}

/// The author's own resolved support, used to state the expected reflection.
fn author_gasket_support(
    document: &ProjectDoc,
    accepted: &[BoardContours],
    instance_id: &str,
    board_id: &str,
) -> boardstudio_core::model::MechanicalGasketSupport {
    let mut author_document = document.clone();
    author_document.physical_instance_id = Some(instance_id.into());
    author_document.mechanical = Some(stored_configuration(document, instance_id).clone());
    let contours = accepted
        .iter()
        .find(|entry| entry.board_id == board_id)
        .unwrap()
        .contours
        .clone();
    let assembly = boardstudio_core::mechanical::resolve(&author_document, &contours);
    assert!(
        !assembly.generation_blocked,
        "the author's assembly resolves: {:?}",
        assembly.diagnostics
    );
    assembly
        .gasket_supports
        .first()
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "the fixture resolves gasket supports: {:?}",
                assembly.diagnostics
            )
        })
}

fn anchor_of(
    support: &boardstudio_core::model::MechanicalGasketSupport,
) -> boardstudio_core::model::MechanicalGasketAnchor {
    boardstudio_core::model::MechanicalGasketAnchor {
        length: None,
        width: None,
        placement: Some(boardstudio_core::model::GasketPlacement::User),
        id: support.id.clone(),
        region_id: support.region_id.clone(),
        outline_key: support.outline_key.clone(),
        anchor: support.anchor,
        unlinked: false,
    }
}

/// One member's state for a feature, by member feature id.
fn features_member_state(
    pair: &boardstudio_core::model::MechanicalMirrorPair,
    instance_id: &str,
    feature_id: &str,
) -> Option<boardstudio_core::model::MechanicalFeatureState> {
    boardstudio_core::mechanical::features::member_state(pair, instance_id, feature_id)
}

/// A legacy-ownership configuration whose hardware, critical fits and profiles are
/// distinguishable by `marker`.
fn legacy_configuration(marker: &str, plate_thickness: f64) -> MechanicalConfiguration {
    let mut value = serde_json::to_value(configuration()).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert("boardId".into(), json!("board"));
    object.insert("plateThickness".into(), json!(plate_thickness));
    object.insert(
        "hardware".into(),
        json!([{
            "id": format!("hw-{marker}"), "partId": "plate", "featureId": "mount-1",
            "designation": marker, "thread": "M3", "length": 6.0, "quantity": 1
        }]),
    );
    object.insert(
        "criticalFits".into(),
        json!([{
            "id": format!("fit-{marker}"), "partId": "plate", "label": marker,
            "from": {"x": 0.0, "y": 0.0}, "to": {"x": 1.0, "y": 1.0}, "tolerance": "0.2"
        }]),
    );
    object.insert(
        "profiles".into(),
        json!([{
            "definitionId": format!("def-{marker}"), "source": marker, "cutouts": [],
            "plateToPcb": 3.5
        }]),
    );
    serde_json::from_value(value).unwrap()
}

fn legacy_instance(id: &str, marker: &str, plate_thickness: f64) -> PhysicalBoardInstance {
    instance(
        id,
        "board",
        Some(legacy_configuration(marker, plate_thickness)),
        MechanicalConstructionOwnership::Legacy,
    )
}

/// Three legacy members whose first member is the fallback shared source.
fn legacy_document() -> ProjectDoc {
    let mut document = document();
    document.hardware = Some(hardware(vec![
        legacy_instance("a", "a", 1.5),
        legacy_instance("b", "b", 9.9),
        legacy_instance("c", "c", 9.9),
    ]));
    document
}

fn convert(document: &mut ProjectDoc, ids: &[&str]) -> Result<Vec<String>, String> {
    boardstudio_core::mechanical::edit::apply(
        document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::MaterializeLegacyConstruction {
                instance_ids: ids.iter().map(|id| (*id).to_string()).collect(),
            },
            mounting_hole: None,
        },
    )
}

fn stored_instance<'a>(document: &'a ProjectDoc, id: &str) -> &'a PhysicalBoardInstance {
    document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.id == id)
        .unwrap_or_else(|| panic!("missing physical instance {id}"))
}

fn stored_configuration<'a>(document: &'a ProjectDoc, id: &str) -> &'a MechanicalConfiguration {
    stored_instance(document, id)
        .mechanical
        .as_ref()
        .unwrap_or_else(|| panic!("missing mechanical configuration for {id}"))
}

/// Two halves on one board whose mechanical stack uses discrete gasket supports.
fn same_board_gasket_pair_document() -> (ProjectDoc, Vec<BoardContours>) {
    gasket_pair("board", "board", (200.0, 150.0))
}

/// Links one resolved gasket support between the halves, choosing the source half.
fn link_gasket_support(
    document: &mut ProjectDoc,
    accepted: &[BoardContours],
    source: &str,
    board: &str,
) -> boardstudio_core::model::MechanicalGasketSupport {
    let author = author_gasket_support(document, accepted, source, board);
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
        document,
        vec![correspondence(
            MechanicalFeatureKind::GasketSupport,
            &anchor.id,
            &anchor.id,
            Some(source),
        )],
    )
    .unwrap();
    author
}

/// Three legacy halves that own no per-instance configuration and share one source.
fn legacy_halves_document() -> ProjectDoc {
    let mut document = document();
    let mut shared = configuration();
    shared.wall_thickness = 2.0;
    shared.internal_gasket = Some(boardstudio_core::mechanical::default_internal_gasket());
    let half = |id| instance(id, "board", None, MechanicalConstructionOwnership::Legacy);
    document.hardware = Some(HardwareConfiguration {
        shared_construction: Some(shared),
        ..hardware(vec![half("left"), half("right"), half("third")])
    });
    document
}

fn set_wall_thickness(
    document: &mut ProjectDoc,
    instance_id: &str,
    value: f64,
) -> Result<Vec<String>, String> {
    boardstudio_core::mechanical::edit::apply(
        document,
        &[],
        &MechanicalEdit {
            target: instance_target(instance_id),
            change: MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value,
            },
            mounting_hole: None,
        },
    )
}
