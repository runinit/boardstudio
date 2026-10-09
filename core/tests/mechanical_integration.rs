//! Integrated mechanical verification at the Session/Core seam.
//!
//! These cases exercise the approved public Core contract through real edits: the
//! synchronous mechanical evaluator, the effective mechanical read path and the
//! atomic pair behavior. They complement the per-slice unit tests by driving the
//! same behavior through `CoreEngine` `Edit`/`Undo` and the public mechanical
//! module functions.

use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalDimension, MechanicalEdit, MechanicalFeatureCorrespondence,
    MechanicalMountCollection, MechanicalTarget,
};
use boardstudio_core::mechanical::{MechanicalEditDisposition, evaluate_edit};
use boardstudio_core::model::{
    Board, BoardContours, HardwareConfiguration, HardwareTopology, HardwareTransport,
    MechanicalConfiguration, MechanicalConstructionOwnership, MechanicalFeatureKind,
    MechanicalFeatureValue, Mount, MountKind, PartDefinition, PhysicalBoardInstance, Pose2,
    ProjectDoc, Vec2,
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

#[test]
fn the_evaluator_reports_changed_and_unchanged_without_touching_the_document() {
    let document = document();
    let before = document.clone();

    let changed = evaluate_edit(&document, &[] as &[BoardContours], &plate_thickness(1.8)).unwrap();
    assert_eq!(changed, MechanicalEditDisposition::Changed);

    let unchanged = evaluate_edit(&document, &[], &plate_thickness(1.5)).unwrap();
    assert_eq!(unchanged, MechanicalEditDisposition::Unchanged);

    assert_eq!(
        document, before,
        "evaluation is read-only: it never mutates the accepted document"
    );
}

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
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(mechanical),
        construction_ownership: MechanicalConstructionOwnership::Local,
    }
}

/// Two physical halves on one board, each with its own local mount geometry.
fn pair_document() -> ProjectDoc {
    let mut document = document();
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]),
            mirror_instance("right", vec![mount("mount-1", 7.0, 3.0)]),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    document
}

fn instance_target(id: &str) -> MechanicalTarget {
    MechanicalTarget::Instance {
        instance_id: id.into(),
    }
}

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

/// Links the two halves with an explicit identity mapping (no guessing).
fn link_pair(document: &mut ProjectDoc) {
    boardstudio_core::mechanical::edit::apply(
        document,
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
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                }],
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
}

fn stored_mounts(document: &ProjectDoc, instance_id: &str) -> Vec<Mount> {
    document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.id == instance_id)
        .unwrap()
        .mechanical
        .as_ref()
        .unwrap()
        .mounts
        .clone()
}

fn mount_at(configuration: &MechanicalConfiguration, id: &str) -> Vec2 {
    configuration
        .mounts
        .iter()
        .find(|mount| mount.id == id)
        .unwrap_or_else(|| panic!("no mount {id:?}"))
        .at
}

#[test]
fn a_linked_feature_is_read_from_the_single_authored_value_on_both_halves() {
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    let mut document = pair_document();
    link_pair(&mut document);

    // Move the pair's single authored value. Nothing else is written.
    document.hardware.as_mut().unwrap().pairs[0].features[0].value =
        MechanicalFeatureValue::Mount {
            mount: mount("mount-1", 4.0, 0.0),
        };

    let left: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();

    // The author reads the stored value; the other half reads the reflected one.
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 4.0, y: 0.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -4.0, y: 0.0 }
    );

    // The linked feature has exactly one writable value: neither member stores a
    // second copy that could drift from the pair's authored value.
    assert!(
        stored_mounts(&document, "left")
            .iter()
            .all(|mount| mount.id != "mount-1"),
        "the authoring half must not keep a second writable copy"
    );
    assert!(
        stored_mounts(&document, "right")
            .iter()
            .all(|mount| mount.id != "mount-1"),
        "the linked half must not keep a second writable copy"
    );
}

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
fn an_ordinary_edit_from_either_half_updates_the_single_authored_value() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    fn move_mount(document: &mut ProjectDoc, instance_id: &str, x: f64, y: f64) {
        apply(
            document,
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

// ---------------------------------------------------------------------------
// Mounting-hole template validation and projection-optional edits.
// ---------------------------------------------------------------------------

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
fn an_ordinary_delete_of_a_linked_feature_removes_both_halves() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    link_pair(&mut document);

    // Ordinary Delete is the linked deletion: it removes both counterparts and the
    // relationship itself. Remove-on-this-half is the explicit Omit action.
    apply(
        &mut document,
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
    assert!(
        pair.features.is_empty(),
        "a linked delete removes the relationship, not just one half"
    );
    for instance in ["left", "right"] {
        let configuration = boardstudio_core::mechanical::effective_inputs(
            &document,
            &[],
            &instance_target(instance),
        )
        .unwrap()
        .configuration;
        assert!(
            configuration
                .mounts
                .iter()
                .all(|mount| mount.id != "mount-1"),
            "{instance} no longer shows the deleted linked mount"
        );
    }
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
