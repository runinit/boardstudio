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
    Board, BoardContours, CaseBody, CaseKind, Contour, HardwareConfiguration, HardwareTopology,
    HardwareTransport, MechanicalConfiguration, MechanicalConstructionOwnership,
    MechanicalFeatureKind, MechanicalFeatureValue, MechanicalOpening, Mount, MountKind, Operation,
    OutlineFeature, PartDefinition, PhysicalBoardInstance, Pose2, ProjectDoc, Vec2,
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
        mechanical_identity_reservation: None,
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
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
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
        &[],
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
                    source_instance_id: None,
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
        &[],
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
        &[],
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
fn an_ordinary_delete_of_a_linked_feature_removes_both_halves() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    link_pair(&mut document);

    // Ordinary Delete is the linked deletion: it removes both counterparts and the
    // relationship itself. Remove-on-this-half is the explicit Omit action.
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

#[test]
fn accepted_contours_drive_absent_closure_mount_initialization() {
    use boardstudio_core::mechanical::edit::apply;

    // The document's own outline is a large board...
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
    document.boards.push(board("board"));
    document.boards[0].outline_ids = vec!["outline".into()];
    let mut configuration = configuration();
    configuration.closure_mounts = None;
    document.mechanical = Some(configuration);

    // ...but the accepted scene projection for that board is a small outline.
    let accepted = vec![BoardContours {
        board_id: "board".into(),
        contours: vec![small_rect()],
    }];

    let initialization = MechanicalEdit {
        target: board_target(),
        change: MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
        mounting_hole: Some(Box::new(template())),
    };

    let mut from_accepted = document.clone();
    apply(&mut from_accepted, &accepted, &initialization).unwrap();
    let mut from_recomputed = document.clone();
    apply(&mut from_recomputed, &[], &initialization).unwrap();

    let accepted_mounts = closure_mounts(&from_accepted);
    let recomputed_mounts = closure_mounts(&from_recomputed);
    assert!(
        !accepted_mounts.is_empty(),
        "the accepted outline proposes closure mounts"
    );
    assert!(
        accepted_mounts
            .iter()
            .all(|mount| mount.at.x.abs() < 15.0 && mount.at.y.abs() < 10.0),
        "initialization follows the accepted projection: {accepted_mounts:?}"
    );
    assert!(
        recomputed_mounts
            .iter()
            .any(|mount| mount.at.x.abs() > 15.0),
        "the document's own outline would have proposed wider mounts: {recomputed_mounts:?}"
    );
    assert_ne!(
        accepted_mounts, recomputed_mounts,
        "the accepted projection is what initialization consumes"
    );
}

fn small_rect() -> Contour {
    Contour {
        hole: false,
        points: vec![
            Vec2 { x: -10.0, y: -5.0 },
            Vec2 { x: 10.0, y: -5.0 },
            Vec2 { x: 10.0, y: 5.0 },
            Vec2 { x: -10.0, y: 5.0 },
        ],
    }
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
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            mirror_instance(
                "left",
                vec![mount("mount-1", 2.0, 3.0), mount("mount-2", 4.0, 5.0)],
            ),
            mirror_instance(
                "right",
                vec![mount("mount-1", 7.0, 3.0), mount("mount-2", 9.0, 5.0)],
            ),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
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

#[test]
fn a_reviewed_link_chooses_the_source_geometry_per_feature() {
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    let mut document = pair_document_with_two_mounts();
    // mount-1 is sourced from the second half; mount-2 from the first.
    link_with_correspondences(
        &mut document,
        vec![
            correspondence(
                MechanicalFeatureKind::Mount,
                "mount-1",
                "mount-1",
                Some("right"),
            ),
            correspondence(
                MechanicalFeatureKind::Mount,
                "mount-2",
                "mount-2",
                Some("left"),
            ),
        ],
    )
    .unwrap();

    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    let find = |id: &str| {
        pair.features
            .iter()
            .find(|feature| feature.id == id)
            .unwrap_or_else(|| panic!("no feature {id:?}"))
    };
    let sourced_from_right = find("mount-1");
    assert_eq!(
        sourced_from_right.author, "right",
        "the reviewed source half authors its feature"
    );
    let MechanicalFeatureValue::Mount { mount } = &sourced_from_right.value else {
        panic!("a linked mount stores a mount");
    };
    assert_eq!(mount.at, Vec2 { x: 7.0, y: 3.0 });
    let sourced_from_left = find("mount-2");
    assert_eq!(sourced_from_left.author, "left");
    let MechanicalFeatureValue::Mount { mount } = &sourced_from_left.value else {
        panic!("a linked mount stores a mount");
    };
    assert_eq!(mount.at, Vec2 { x: 4.0, y: 5.0 });

    // The non-source half derives the exact inverse of the source geometry,
    // independently for each feature.
    let left: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: -7.0, y: 3.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: 7.0, y: 3.0 }
    );
    assert_eq!(
        mount_at(&left.configuration, "mount-2"),
        Vec2 { x: 4.0, y: 5.0 }
    );
    assert_eq!(
        mount_at(&right.configuration, "mount-2"),
        Vec2 { x: -4.0, y: 5.0 }
    );
}

#[test]
fn a_reviewed_link_rejects_an_unknown_source_half() {
    let mut document = pair_document();
    let before = document.clone();
    let error = link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Mount,
            "mount-1",
            "mount-1",
            Some("ghost"),
        )],
    )
    .unwrap_err();
    assert!(error.contains("source"), "{error}");
    assert_eq!(document, before, "a rejected source mutates nothing");
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

#[test]
fn removing_a_paired_member_materializes_the_survivor_and_clears_the_pair() {
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};

    let mut document = pair_document();
    link_pair(&mut document);
    let before: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let survivor_geometry = mount_at(&before.configuration, "mount-1");

    let changed = remove_member(&mut document, "right").unwrap();

    let hardware = document.hardware.as_ref().unwrap();
    assert_eq!(hardware.instances.len(), 1, "the removed half is gone");
    assert_eq!(hardware.instances[0].id, "left");
    assert!(hardware.pairs.is_empty(), "the pair is cleared");
    let survivor = &hardware.instances[0];
    assert_eq!(
        survivor.construction_ownership,
        MechanicalConstructionOwnership::Local,
        "the survivor owns its materialized construction locally"
    );
    assert_eq!(
        mount_at(survivor.mechanical.as_ref().unwrap(), "mount-1"),
        survivor_geometry,
        "the survivor keeps the geometry it showed"
    );
    assert_eq!(
        survivor.mechanical.as_ref().unwrap().plate_thickness,
        before.configuration.plate_thickness
    );
    for id in ["pair-left-right", "left", "right"] {
        assert!(changed.contains(&id.to_string()), "{id} in {changed:?}");
    }

    // The survivor's effective read still shows the same geometry.
    let after: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("left")).unwrap();
    assert_eq!(mount_at(&after.configuration, "mount-1"), survivor_geometry);
}

#[test]
fn removing_a_physical_instance_requires_a_paired_instance_target() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    let before = document.clone();

    // A board target cannot identify a physical member to remove.
    let error = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemovePhysicalInstance,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap_err();
    assert!(error.contains("physical-instance target"), "{error}");
    assert_eq!(document, before, "a malformed target mutates nothing");

    // An unpaired instance is a plain topology change, not a pair removal.
    let error = remove_member(&mut document, "left").unwrap_err();
    assert!(error.contains("not part of"), "{error}");
    assert_eq!(document, before, "an ineligible member mutates nothing");
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

#[test]
fn a_topology_replacement_never_leaves_a_dangling_pair() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};

    let mut document = pair_document();
    link_pair(&mut document);

    let mut engine = CoreEngine::new();
    let accepted = match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    };

    // The incoming document drops the right half but still lists the pair.
    let mut replacement = accepted.clone();
    replacement
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .retain(|instance| instance.id != "right");
    assert_eq!(
        replacement.hardware.as_ref().unwrap().pairs.len(),
        1,
        "the incoming document still lists the pair"
    );

    let replaced = match engine.handle(CoreRequest::Edit {
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
    }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    };

    let hardware = replaced.hardware.as_ref().unwrap();
    assert_eq!(hardware.instances.len(), 1);
    assert!(
        hardware.pairs.is_empty(),
        "a pair whose member is gone cannot survive the replacement"
    );
    let survivor = &hardware.instances[0];
    assert_eq!(
        survivor.construction_ownership,
        MechanicalConstructionOwnership::Local
    );
    assert_eq!(
        mount_at(survivor.mechanical.as_ref().unwrap(), "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
        "the survivor keeps the geometry it showed"
    );
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
        gasket: None,
    });
}

/// Two halves where the linked mount reflects onto the second half's local hardware.
fn conflicting_pair_document() -> ProjectDoc {
    let mut document = document();
    with_board_outline(&mut document);
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]),
            mirror_instance(
                "right",
                vec![
                    mount("mount-1", 7.0, 3.0),
                    mount("mount-encoder", -2.0, 3.0),
                ],
            ),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    document
}

#[test]
fn a_valid_asymmetric_clash_is_retained_with_a_located_finding() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::mechanical::{MechanicalInputs, effective_inputs};
    use boardstudio_core::model::{CoreReply, CoreRequest, Scope};

    let mut document = conflicting_pair_document();
    link_pair(&mut document);

    // The reflected mount is exactly where the right half's own hardware sits.
    let right: MechanicalInputs =
        effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert_eq!(
        mount_at(&right.configuration, "mount-1"),
        Vec2 { x: -2.0, y: 3.0 },
        "the intended geometry stays exactly where the pair puts it"
    );
    let finding = right
        .findings
        .iter()
        .find(|finding| finding.target_ids.contains(&"mount-1".to_string()))
        .unwrap_or_else(|| panic!("no located finding: {:?}", right.findings));
    assert_eq!(finding.scope, Scope::Case);
    assert_eq!(
        finding.target_ids,
        vec![
            "board".to_string(),
            "right".to_string(),
            "mount-1".to_string()
        ],
        "the finding locates the board, the half and the feature"
    );

    // The accepted scene carries the same located finding, so readiness and the
    // export gates see accepted state rather than a transient assembly warning.
    let mut engine = CoreEngine::new();
    let scene = match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: document.clone(),
    }) {
        CoreReply::Scene { scene, .. } => scene,
        other => panic!("expected scene: {other:?}"),
    };
    let located = scene
        .findings
        .iter()
        .find(|finding| finding.id == "mechanical:pair:pair-left-right:right:mount-1:fit")
        .unwrap_or_else(|| panic!("no accepted pair finding: {:?}", scene.findings));
    assert_eq!(located.scope, Scope::Case);
    assert_eq!(
        located.target_ids,
        vec![
            "board".to_string(),
            "right".to_string(),
            "mount-1".to_string()
        ],
        "the accepted finding locates board, half and feature"
    );
    let readiness = scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == "board")
        .expect("board readiness");
    assert!(
        readiness.pcb,
        "a Case-only clash must not block the board's manufacturing export"
    );

    // Without the local hardware there is no clash finding at all: the conflict, not
    // the pairing, is what is reported.
    let mut control = document.clone();
    control
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "right")
        .unwrap()
        .mechanical
        .as_mut()
        .unwrap()
        .mounts
        .retain(|mount| mount.id != "mount-encoder");
    let control_findings =
        boardstudio_core::mechanical::effective_inputs(&control, &[], &instance_target("right"))
            .unwrap()
            .findings;
    assert!(
        control_findings.is_empty(),
        "the same pair without a local conflict reports nothing: {control_findings:?}"
    );
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

#[test]
fn a_removed_opening_identity_is_never_reused() {
    let mut document = document();
    let first = add_opening(&mut document);
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemoveOpening {
                opening_id: first.clone(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let second = add_opening(&mut document);
    assert_ne!(
        second, first,
        "a removed opening identity must not be handed out again"
    );

    // A queued intent naming the removed opening cannot reach the later addition, and
    // retires instead of being redirected.
    let queued = MechanicalEdit {
        target: board_target(),
        change: MechanicalChange::SetOpeningDimension {
            opening_id: first.clone(),
            vertex_id: None,
            field: MechanicalDimension::OpeningHeight,
            value: 9.0,
        },
        mounting_hole: Some(Box::new(template())),
    };
    assert_eq!(
        evaluate_edit(&document, &[], &queued).unwrap(),
        MechanicalEditDisposition::Retire
    );
    assert!(
        boardstudio_core::mechanical::edit::apply(&mut document, &[], &queued).is_err(),
        "a commit against the removed opening is an error, not a redirected change"
    );
}

#[test]
fn a_removed_opening_vertex_identity_is_never_reused() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = document();
    let opening = add_opening(&mut document);
    let add_vertex = |document: &mut ProjectDoc| {
        apply(
            document,
            &[],
            &MechanicalEdit {
                target: board_target(),
                change: MechanicalChange::AddOpeningVertex {
                    opening_id: opening.clone(),
                },
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
            .iter()
            .find(|candidate| candidate.id == opening)
            .unwrap()
            .vertices
            .last()
            .unwrap()
            .id
            .clone()
    };
    let first = add_vertex(&mut document);
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemoveOpeningVertex {
                opening_id: opening.clone(),
                vertex_id: first.clone(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let second = add_vertex(&mut document);
    assert_ne!(
        second, first,
        "a removed vertex identity must not be handed out again"
    );

    let queued = MechanicalEdit {
        target: board_target(),
        change: MechanicalChange::SetOpeningDimension {
            opening_id: opening.clone(),
            vertex_id: Some(first.clone()),
            field: MechanicalDimension::OpeningPointX,
            value: 4.0,
        },
        mounting_hole: Some(Box::new(template())),
    };
    assert_eq!(
        evaluate_edit(&document, &[], &queued).unwrap(),
        MechanicalEditDisposition::Retire
    );
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

#[test]
fn a_new_local_opening_never_collides_with_a_visible_pair_owned_identity() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = pair_document_with_openings();
    // The opening is authored by the second half, so it leaves both raw
    // configurations and becomes visible on both through the pair.
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
    let left = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    assert!(
        left.configuration
            .openings
            .as_ref()
            .is_some_and(|openings| openings.iter().any(|opening| opening.id == "opening-1")),
        "the non-author half shows the pair-owned opening"
    );

    // A new local addition must not take the visible pair-owned identity, or a queued
    // intent naming the pair feature could reach the new opening instead.
    let new_id = add_instance_opening(&mut document, "left");
    assert_ne!(
        new_id, "opening-1",
        "the visible pair-owned identity is reserved on the member"
    );
    let after = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    let ids: Vec<&str> = after
        .configuration
        .openings
        .as_ref()
        .unwrap()
        .iter()
        .map(|opening| opening.id.as_str())
        .collect();
    assert!(
        ids.contains(&"opening-1"),
        "the linked opening survives: {ids:?}"
    );
    assert!(ids.contains(&new_id.as_str()));
    assert_eq!(ids.len(), 2);

    // A commit against the pair-owned identity still lands on the linked opening, not
    // the local addition.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::SetOpeningDimension {
                opening_id: "opening-1".into(),
                vertex_id: None,
                field: MechanicalDimension::OpeningHeight,
                value: 7.0,
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    let MechanicalFeatureValue::Opening { opening } = &pair.features[0].value else {
        panic!("a linked opening stores an opening");
    };
    assert_eq!(
        opening.height, 7.0,
        "the edit reached the pair's authored value, not the local addition"
    );
}

#[test]
fn identity_reservations_survive_a_document_round_trip() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = document();
    let first = add_opening(&mut document);
    // A save/reopen round trip keeps the reservation monotonic.
    let encoded = serde_json::to_string(&document).unwrap();
    let mut reopened: ProjectDoc = serde_json::from_str(&encoded).unwrap();
    apply(
        &mut reopened,
        &[],
        &MechanicalEdit {
            target: board_target(),
            change: MechanicalChange::RemoveOpening {
                opening_id: first.clone(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let second = add_opening(&mut reopened);
    assert_ne!(
        second, first,
        "the reservation survives save and reopen so a removed identity is not reused"
    );
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
fn disabling_a_paired_half_unlinks_it_and_keeps_the_other_half() {
    use boardstudio_core::mechanical::edit::apply;
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = pair_document();
    link_pair(&mut document);
    let before_left = effective_inputs(&document, &[], &instance_target("left")).unwrap();

    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::Disable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();

    assert!(
        changed.contains(&"pair-left-right".to_string()),
        "the dissolved pair is reported: {changed:?}"
    );
    let hardware = document.hardware.as_ref().unwrap();
    assert!(hardware.pairs.is_empty(), "the pair is cleared");
    assert_eq!(hardware.instances.len(), 2, "both instances remain");
    let right = hardware
        .instances
        .iter()
        .find(|instance| instance.id == "right")
        .unwrap();
    assert!(
        right.mechanical.is_none(),
        "only the selected half's mechanical stack is disabled"
    );
    let left = hardware
        .instances
        .iter()
        .find(|instance| instance.id == "left")
        .unwrap();
    assert_eq!(
        left.construction_ownership,
        MechanicalConstructionOwnership::Local,
        "the surviving half owns its materialized construction"
    );
    assert_eq!(
        mount_at(left.mechanical.as_ref().unwrap(), "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
        "the surviving half keeps the geometry it showed"
    );
    let after_left = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    assert_eq!(
        mount_at(&after_left.configuration, "mount-1"),
        Vec2 { x: 2.0, y: 3.0 }
    );
    assert_eq!(
        after_left.configuration.plate_thickness,
        before_left.configuration.plate_thickness
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
fn one_undo_restores_a_paired_disable_completely() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};

    fn document_of(reply: CoreReply) -> ProjectDoc {
        match reply {
            CoreReply::Scene { document, .. } => *document,
            other => panic!("expected scene: {other:?}"),
        }
    }
    fn mechanical_command(base_revision: u64, edit: MechanicalEdit) -> CoreRequest {
        CoreRequest::Edit {
            id: "mechanical".into(),
            command: EditCommand {
                base_revision,
                transaction_id: "mechanical".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::ApplyMechanicalEdit {
                    edit: Box::new(edit),
                },
            },
        }
    }

    let mut document = pair_document();
    link_pair(&mut document);

    let mut engine = CoreEngine::new();
    let accepted = document_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }));

    let disabled = document_of(engine.handle(mechanical_command(
        accepted.revision,
        MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::Disable,
            mounting_hole: Some(Box::new(template())),
        },
    )));
    assert!(disabled.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(
        disabled
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical
            .is_none()
    );

    // One undo restores the pair, both stacks and the geometry projection.
    let restored = document_of(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(
        restored.hardware.as_ref().unwrap().pairs.len(),
        1,
        "the pair is restored"
    );
    assert!(
        restored
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .all(|instance| instance.mechanical.is_some()),
        "both halves are configured again"
    );
    let left = effective_inputs(&restored, &[], &instance_target("left")).unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 2.0, y: 3.0 },
        "the linked geometry reads as before the disable"
    );

    // Redo repeats the whole result.
    let redone = document_of(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert!(redone.hardware.as_ref().unwrap().pairs.is_empty());
    assert!(
        redone
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical
            .is_none()
    );
}

#[test]
fn a_case_conflict_on_one_half_does_not_scope_to_the_other_half() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = conflicting_pair_document();
    link_pair(&mut document);

    let right = effective_inputs(&document, &[], &instance_target("right")).unwrap();
    assert!(
        right
            .findings
            .iter()
            .any(|finding| finding.target_ids.contains(&"right".to_string())),
        "the conflicting half carries the finding: {:?}",
        right.findings
    );
    let left = effective_inputs(&document, &[], &instance_target("left")).unwrap();
    assert!(
        left.findings.is_empty(),
        "the other half on the same PCB is unaffected: {:?}",
        left.findings
    );
    // A board-scoped read still sees it, because the violation is on that board.
    let board = effective_inputs(
        &document,
        &[],
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    assert!(!board.findings.is_empty());
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
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![left, right],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    document
}

#[test]
fn a_pcb_violation_blocks_only_its_own_board_export() {
    use boardstudio_core::model::{ExportTarget, PrepareExportRequest};
    use std::collections::BTreeMap;

    let mut document = cross_board_closure_document();
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "wide".into(),
            },
            change: MechanicalChange::LinkMirrorPair {
                pair_id: "pair-left-right".into(),
                members: ["left".into(), "right".into()],
                mapping: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                construction_source: "left".into(),
                correspondences: vec![correspondence(
                    MechanicalFeatureKind::ClosureMount,
                    "closure-1",
                    "closure-1",
                    Some("left"),
                )],
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();

    let request = |board_id: &str, contour: Contour| PrepareExportRequest {
        snapshot_token: "token".into(),
        expected_revision: document.revision,
        document: document.clone(),
        target: ExportTarget::Board {
            board_id: board_id.into(),
        },
        contours: vec![contour],
        model_paths: BTreeMap::new(),
    };

    // The reflected closure drill lands outside the narrow board: its fabrication
    // export is blocked.
    let blocked = boardstudio_core::artifact::kicad::prepare_export(request(
        "narrow",
        rect_contour(10.0, 5.0),
    ));
    assert!(blocked.is_err(), "{blocked:?}");

    // The other board's export is unaffected.
    let available = boardstudio_core::artifact::kicad::prepare_export(request(
        "wide",
        rect_contour(40.0, 30.0),
    ));
    assert!(available.is_ok(), "{available:?}");

    // The preview of the affected board stays available.
    let preview = boardstudio_core::artifact::kicad::prepare_preview(request(
        "narrow",
        rect_contour(10.0, 5.0),
    ));
    assert!(preview.is_ok(), "{preview:?}");
}

#[test]
fn a_case_only_conflict_never_blocks_manufacturing_export() {
    use boardstudio_core::model::{ExportTarget, PrepareExportRequest};
    use std::collections::BTreeMap;

    let mut document = conflicting_pair_document();
    link_pair(&mut document);

    let export = boardstudio_core::artifact::kicad::prepare_export(PrepareExportRequest {
        snapshot_token: "token".into(),
        expected_revision: document.revision,
        document: document.clone(),
        target: ExportTarget::Board {
            board_id: "board".into(),
        },
        contours: vec![rect_contour(40.0, 30.0)],
        model_paths: BTreeMap::new(),
    });
    assert!(
        export.is_ok(),
        "a Case-only clash must not block the board's manufacturing export: {export:?}"
    );
}

#[test]
fn rejoining_can_take_the_selected_halves_own_geometry_as_the_source() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document();
    link_pair(&mut document);
    // The right half keeps its own placement as an explicit exception, then drifts.
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
    document
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "right")
        .unwrap()
        .mechanical
        .as_mut()
        .unwrap()
        .mounts[0]
        .at = Vec2 { x: -5.0, y: 1.0 };

    // A deliberate own-half source is allowed when that half has geometry.
    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
                source_instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    assert_eq!(changed, vec!["mount-1", "right", "pair-left-right", "left"]);
    let pair = &document.hardware.as_ref().unwrap().pairs[0];
    assert_eq!(pair.features[0].author, "right");
    let MechanicalFeatureValue::Mount { mount } = &pair.features[0].value else {
        panic!("a linked mount stores a mount");
    };
    assert_eq!(mount.at, Vec2 { x: -5.0, y: 1.0 });

    // The other half now derives the exact inverse of that geometry.
    let left =
        boardstudio_core::mechanical::effective_inputs(&document, &[], &instance_target("left"))
            .unwrap();
    assert_eq!(
        mount_at(&left.configuration, "mount-1"),
        Vec2 { x: 5.0, y: 1.0 }
    );
}

#[test]
fn a_re_enabled_half_never_reuses_an_identity_its_removed_stack_reserved() {
    use boardstudio_core::mechanical::edit::apply;

    let mut document = pair_document_with_openings();
    link_with_correspondences(
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::Opening,
            "opening-1",
            "opening-1",
            Some("left"),
        )],
    )
    .unwrap();
    // The right half is unlinked and disabled (Q14), which removes its stack.
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
    assert!(
        document
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "right")
            .unwrap()
            .mechanical_identity_reservation
            .is_some(),
        "the removed stack's reservations are kept on the half"
    );

    // Re-enabling builds fresh defaults, and the next authored addition must still avoid
    // the identity the removed stack reserved.
    apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::Enable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let new_id = add_instance_opening(&mut document, "right");
    assert_ne!(
        new_id, "opening-1",
        "a re-enabled half must not hand out a removed opening identity"
    );

    // The same holds for a half that was disabled without ever being paired.
    let mut unpaired = document_with_an_opening();
    apply(
        &mut unpaired,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::Disable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    apply(
        &mut unpaired,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::Enable,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    let new_id = add_instance_opening(&mut unpaired, "left");
    assert_ne!(
        new_id, "opening-1",
        "an unpaired disabled half keeps its reservations too"
    );
}

/// One unpaired physical half that already carries an authored opening.
fn document_with_an_opening() -> ProjectDoc {
    let mut document = document();
    let mut instance = mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]);
    instance.mechanical.as_mut().unwrap().openings = Some(vec![opening_json("opening-1")]);
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![instance],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    document
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
        });
    let reloaded: ProjectDoc =
        serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
    let stored = reloaded.hardware.as_ref().unwrap().instances[0]
        .mechanical_identity_reservation
        .unwrap();
    assert_eq!(stored.next_opening_identity, 4);
    assert_eq!(stored.next_vertex_identity, 7);
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
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("wide"));
    document.boards.push(board("narrow"));
    let gasket = |board_id: &str| {
        let mut configuration = configuration();
        configuration.board_id = board_id.into();
        configuration.mount = boardstudio_core::model::MechanicalMount::Gasket;
        configuration.gasket_layout = Some(boardstudio_core::mechanical::default_gasket_layout());
        configuration
    };
    let mut left = mirror_instance_on("left", "wide", vec![]);
    let mut right = mirror_instance_on("right", "narrow", vec![]);
    left.mechanical = Some(gasket("wide"));
    right.mechanical = Some(gasket("narrow"));
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![left, right],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    let accepted = vec![
        BoardContours {
            board_id: "wide".into(),
            contours: vec![rect_contour(200.0, 150.0)],
        },
        BoardContours {
            board_id: "narrow".into(),
            contours: vec![rect_contour(10.0, 5.0)],
        },
    ];
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
    author_document.mechanical = Some(
        document
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .unwrap()
            .mechanical
            .clone()
            .unwrap(),
    );
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

#[test]
fn a_linked_gasket_support_reflects_the_authors_resolved_placement() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::features::mirror_support;

    let (mut document, accepted) = gasket_pair_document();
    let author = author_gasket_support(&document, &accepted, "left", "wide");
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
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::GasketSupport,
            &anchor.id,
            &anchor.id,
            Some("left"),
        )],
    )
    .unwrap();

    // The author keeps its own resolved placement; the other half shows the exact
    // reflection of it, not the same perimeter fraction re-resolved on its own contour.
    let left = effective_inputs(&document, &accepted, &instance_target("left")).unwrap();
    assert_eq!(left.gasket_supports.len(), 1);
    assert_close(
        left.gasket_supports[0].at,
        author.at,
        "the author keeps its placement",
    );
    assert!(left.gasket_supports[0].fit_error.is_none());

    let right = effective_inputs(&document, &accepted, &instance_target("right")).unwrap();
    let expected = mirror_support(
        Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        &author,
    );
    assert_eq!(right.gasket_supports.len(), 1);
    assert_close(
        right.gasket_supports[0].at,
        expected.at,
        "the reflected placement",
    );
    assert_close(
        right.gasket_supports[0].tangent,
        expected.tangent,
        "the reflected tangent",
    );
    assert_close(
        right.gasket_supports[0].normal,
        expected.normal,
        "the reflected normal",
    );
    assert!(
        (right.gasket_supports[0].at.x - author.at.x).abs() > 1.0
            || (right.gasket_supports[0].at.y - author.at.y).abs() > 1.0,
        "the reflection is not the author's own placement"
    );
    assert!(
        right.gasket_supports[0].fit_error.is_some(),
        "the narrow board cannot receive the reflected placement, reported rather than moved"
    );
    assert_close(
        right.gasket_supports[0].at,
        expected.at,
        "the intended placement is retained even when it does not associate",
    );

    // A board-scoped read has no pair context and reports no linked supports.
    let board = effective_inputs(
        &document,
        &accepted,
        &MechanicalTarget::Board {
            board_id: "wide".into(),
        },
    )
    .unwrap();
    assert!(board.gasket_supports.is_empty());
}

#[test]
fn a_linked_gasket_support_reflects_through_the_inverse_map_for_a_second_member_author() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::features::{inverse_pose, mirror_support};

    let (mut document, mut accepted) = gasket_pair_document();
    // The author half resolves on its own accepted outline, so give the second board a
    // projection that can carry supports as well.
    accepted[1].contours = vec![rect_contour(200.0, 150.0)];
    let author = author_gasket_support(&document, &accepted, "right", "narrow");
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
        &mut document,
        vec![correspondence(
            MechanicalFeatureKind::GasketSupport,
            &anchor.id,
            &anchor.id,
            Some("right"),
        )],
    )
    .unwrap();

    let left = effective_inputs(&document, &accepted, &instance_target("left")).unwrap();
    let expected = mirror_support(
        inverse_pose(Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        }),
        &author,
    );
    assert_eq!(left.gasket_supports.len(), 1);
    assert_close(
        left.gasket_supports[0].at,
        expected.at,
        "the inverse reflection",
    );
    assert_close(
        left.gasket_supports[0].tangent,
        expected.tangent,
        "the inverse tangent",
    );
    assert_close(
        left.gasket_supports[0].normal,
        expected.normal,
        "the inverse normal",
    );
    let right = effective_inputs(&document, &accepted, &instance_target("right")).unwrap();
    assert_close(
        right.gasket_supports[0].at,
        author.at,
        "the author keeps its placement",
    );
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
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: "board".into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(legacy_configuration(marker, plate_thickness)),
        construction_ownership: MechanicalConstructionOwnership::Legacy,
        mechanical_identity_reservation: None,
    }
}

/// Three legacy members whose first member is the fallback shared source.
fn legacy_document() -> ProjectDoc {
    let mut document = document();
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            legacy_instance("a", "a", 1.5),
            legacy_instance("b", "b", 9.9),
            legacy_instance("c", "c", 9.9),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
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

fn stored_configuration<'a>(document: &'a ProjectDoc, id: &str) -> &'a MechanicalConfiguration {
    document
        .hardware
        .as_ref()
        .unwrap()
        .instances
        .iter()
        .find(|instance| instance.id == id)
        .unwrap()
        .mechanical
        .as_ref()
        .unwrap()
}

#[test]
fn converting_legacy_members_materializes_the_overlay_and_keeps_the_third_baseline() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = legacy_document();
    let before_c = effective_inputs(&document, &[], &instance_target("c"))
        .unwrap()
        .configuration;
    // The second member's stored values conflict with the overlay it actually uses.
    let before_b = effective_inputs(&document, &[], &instance_target("b"))
        .unwrap()
        .configuration;
    assert_eq!(before_b.hardware.as_ref().unwrap()[0].designation, "a");
    assert_eq!(before_b.critical_fits.as_ref().unwrap()[0].label, "a");
    assert_eq!(before_b.profiles[0].source, "a");
    assert_eq!(before_b.plate_thickness, 1.5);

    let changed = convert(&mut document, &["a", "b"]).unwrap();
    assert_eq!(changed, vec!["board", "a", "b"]);

    let hardware = document.hardware.as_ref().unwrap();
    for id in ["a", "b"] {
        let instance = hardware
            .instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap();
        assert_eq!(
            instance.construction_ownership,
            MechanicalConstructionOwnership::Local,
            "{id} owns its construction locally after conversion"
        );
    }
    assert_eq!(
        hardware
            .instances
            .iter()
            .find(|instance| instance.id == "c")
            .unwrap()
            .construction_ownership,
        MechanicalConstructionOwnership::Legacy,
        "an unselected member keeps the legacy ownership"
    );

    // The selected member keeps exactly the values it showed, including the overlaid
    // hardware, critical fits and profiles rather than its stale stored ones.
    let b_stored = stored_configuration(&document, "b");
    assert_eq!(b_stored.hardware.as_ref().unwrap()[0].designation, "a");
    assert_eq!(b_stored.critical_fits.as_ref().unwrap()[0].label, "a");
    assert_eq!(b_stored.profiles[0].source, "a");
    assert_eq!(b_stored.plate_thickness, 1.5);
    assert_eq!(
        b_stored.hardware.as_ref().unwrap()[0].designation,
        before_b.hardware.as_ref().unwrap()[0].designation
    );

    // The prior common construction was captured as the legacy source before conversion.
    let captured = hardware
        .shared_construction
        .as_ref()
        .expect("the legacy baseline is captured");
    assert_eq!(captured.plate_thickness, 1.5);
    assert_eq!(captured.hardware.as_ref().unwrap()[0].designation, "a");

    // The third legacy member keeps its baseline exactly.
    let after_c = effective_inputs(&document, &[], &instance_target("c"))
        .unwrap()
        .configuration;
    assert_eq!(after_c, before_c, "the third legacy baseline is preserved");

    // A later edit to a converted member cannot feed back into that baseline.
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("a"),
            change: MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 5.0,
            },
            mounting_hole: None,
        },
    )
    .unwrap();
    let after_edit_c = effective_inputs(&document, &[], &instance_target("c"))
        .unwrap()
        .configuration;
    assert_eq!(
        after_edit_c, before_c,
        "converted members never feed back into the legacy source"
    );
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
fn one_undo_restores_legacy_ownership_and_the_previous_overlay_state() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::model::{CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase};

    fn document_of(reply: CoreReply) -> ProjectDoc {
        match reply {
            CoreReply::Scene { document, .. } => *document,
            other => panic!("expected scene: {other:?}"),
        }
    }

    let mut engine = CoreEngine::new();
    let accepted = document_of(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: legacy_document(),
    }));
    let before_c = effective_inputs(&accepted, &[], &instance_target("c"))
        .unwrap()
        .configuration;

    let converted = document_of(engine.handle(CoreRequest::Edit {
        id: "convert".into(),
        command: EditCommand {
            base_revision: accepted.revision,
            transaction_id: "convert".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::ApplyMechanicalEdit {
                edit: Box::new(MechanicalEdit {
                    target: board_target(),
                    change: MechanicalChange::MaterializeLegacyConstruction {
                        instance_ids: vec!["a".into()],
                    },
                    mounting_hole: None,
                }),
            },
        },
    }));
    assert_eq!(
        converted
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "a")
            .unwrap()
            .construction_ownership,
        MechanicalConstructionOwnership::Local
    );
    assert!(
        converted
            .hardware
            .as_ref()
            .unwrap()
            .shared_construction
            .is_some()
    );

    let restored = document_of(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(
        restored
            .hardware
            .as_ref()
            .unwrap()
            .instances
            .iter()
            .find(|instance| instance.id == "a")
            .unwrap()
            .construction_ownership,
        MechanicalConstructionOwnership::Legacy
    );
    assert!(
        restored
            .hardware
            .as_ref()
            .unwrap()
            .shared_construction
            .is_none(),
        "the captured baseline is rolled back with the conversion"
    );
    assert_eq!(
        effective_inputs(&restored, &[], &instance_target("c"))
            .unwrap()
            .configuration,
        before_c
    );
}

/// Two halves on one board whose mechanical stack uses discrete gasket supports.
fn same_board_gasket_pair_document() -> (ProjectDoc, Vec<BoardContours>) {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board"));
    let gasket = || {
        let mut configuration = configuration();
        configuration.mount = boardstudio_core::model::MechanicalMount::Gasket;
        configuration.gasket_layout = Some(boardstudio_core::mechanical::default_gasket_layout());
        configuration
    };
    let mut left = mirror_instance("left", vec![]);
    let mut right = mirror_instance("right", vec![]);
    left.mechanical = Some(gasket());
    right.mechanical = Some(gasket());
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![left, right],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    let accepted = vec![BoardContours {
        board_id: "board".into(),
        contours: vec![rect_contour(200.0, 150.0)],
    }];
    (document, accepted)
}

/// Links one resolved gasket support between the halves, choosing the source half.
fn link_gasket_support(document: &mut ProjectDoc, accepted: &[BoardContours], source: &str) {
    let (author_board, author) = if source == "left" {
        (
            "left",
            author_gasket_support(document, accepted, "left", "board"),
        )
    } else {
        (
            "right",
            author_gasket_support(document, accepted, "right", "board"),
        )
    };
    let _ = author_board;
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
}

#[test]
fn generated_gasket_geometry_uses_the_reflected_author_placement() {
    use boardstudio_core::mechanical::effective_inputs;
    use boardstudio_core::mechanical::inputs::effective_document;

    let (mut document, accepted) = same_board_gasket_pair_document();
    link_gasket_support(&mut document, &accepted, "left");

    // The read path reports the author's placement reflected into the other half.
    let reflected = effective_inputs(&document, &accepted, &instance_target("right"))
        .unwrap()
        .gasket_supports
        .first()
        .cloned()
        .expect("the linked support is reflected for the read path");
    let author = author_gasket_support(&document, &accepted, "left", "board");
    assert!(
        (reflected.at.x - author.at.x).abs() > 1.0,
        "the reflected placement differs from the author's own"
    );

    // The generated assembly for that half uses the same resolved placement instead of
    // re-resolving the author's perimeter fraction on its own contour.
    let (right_document, _) = effective_document(&document, &instance_target("right")).unwrap();
    let assembly = boardstudio_core::mechanical::resolve(&right_document, &accepted[0].contours);
    assert!(
        !assembly.generation_blocked,
        "the assembly resolves: {:?}",
        assembly.diagnostics
    );
    let generated = assembly
        .gasket_supports
        .iter()
        .find(|support| support.id == reflected.id)
        .unwrap_or_else(|| {
            panic!(
                "no generated support {}: {:?}",
                reflected.id, assembly.gasket_supports
            )
        });
    assert_close(generated.at, reflected.at, "generated placement");
    assert_close(generated.tangent, reflected.tangent, "generated tangent");
    assert_close(generated.normal, reflected.normal, "generated normal");
    assert_eq!(
        generated.fit_error, reflected.fit_error,
        "the located association is reported by the generated geometry too"
    );
    assert_eq!(generated.pair_id, reflected.pair_id);
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

/// Three legacy halves that own no per-instance configuration and share one source.
fn legacy_halves_document() -> ProjectDoc {
    let mut document = document();
    let mut shared = configuration();
    shared.wall_thickness = 2.0;
    shared.internal_gasket = Some(boardstudio_core::mechanical::default_internal_gasket());
    let half = |id: &str| PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: "board".into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: None,
        construction_ownership: MechanicalConstructionOwnership::Legacy,
        mechanical_identity_reservation: None,
    };
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![half("left"), half("right"), half("third")],
        boards: vec![],
        shared_construction: Some(shared),
        pairs: vec![],
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

#[test]
fn legacy_local_fields_stay_per_instance_and_errors_do_not_mutate() {
    use boardstudio_core::mechanical::effective_inputs;

    let mut document = legacy_halves_document();
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::SetBatteryEnabled(true),
            mounting_hole: None,
        },
    )
    .unwrap();
    boardstudio_core::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("left"),
            change: MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Suspension,
            },
            mounting_hole: None,
        },
    )
    .unwrap();

    let shared = document
        .hardware
        .as_ref()
        .unwrap()
        .shared_construction
        .as_ref()
        .unwrap();
    assert!(
        shared.battery.is_none(),
        "a local battery never lands in the shared source"
    );
    assert!(
        shared.mounts.is_empty(),
        "a local mount never lands in the shared source"
    );

    let left = effective_inputs(&document, &[], &instance_target("left"))
        .unwrap()
        .configuration;
    assert!(left.battery.is_some(), "the edited half shows its battery");
    assert_eq!(left.mounts.len(), 1, "the edited half shows its mount");
    let right = effective_inputs(&document, &[], &instance_target("right"))
        .unwrap()
        .configuration;
    assert!(
        right.battery.is_none() && right.mounts.is_empty(),
        "the other legacy halves keep their own local values"
    );

    // A malformed value is an error and mutates nothing.
    let before = document.clone();
    let error = set_wall_thickness(&mut document, "left", -1.0).unwrap_err();
    assert!(!error.is_empty());
    assert_eq!(document, before, "a rejected value mutates nothing");
}

#[test]
fn a_linked_change_reports_the_pair_and_both_halves() {
    use boardstudio_core::mechanical::edit::apply;

    let owners = ["pair-left-right", "left", "right"];

    // An ordinary edit of a linked feature from one half changes the pair's single
    // authored value, so the counterpart's derived geometry changed too.
    let mut document = pair_document();
    link_pair(&mut document);
    let changed = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "mount-1".into(),
                at: Vec2 { x: -5.0, y: 1.0 },
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            changed.contains(&id.to_string()),
            "a linked edit reports {id}: {changed:?}"
        );
    }
    assert!(changed.contains(&"mount-1".to_string()), "{changed:?}");

    // Make local, omit and rejoin change the pair's records for both halves.
    let mut document = pair_document();
    link_pair(&mut document);
    let local = apply(
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
    for id in owners {
        assert!(
            local.contains(&id.to_string()),
            "Make local reports {id}: {local:?}"
        );
    }
    let omitted = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::OmitFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            omitted.contains(&id.to_string()),
            "Omit reports {id}: {omitted:?}"
        );
    }
    let rejoined = apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: instance_target("right"),
            change: MechanicalChange::RejoinFeatureMember {
                feature_id: "mount-1".into(),
                instance_id: "right".into(),
                source_instance_id: "left".into(),
            },
            mounting_hole: Some(Box::new(template())),
        },
    )
    .unwrap();
    for id in owners {
        assert!(
            rejoined.contains(&id.to_string()),
            "Rejoin reports {id}: {rejoined:?}"
        );
    }

    // An ordinary delete of a linked feature removes both counterparts.
    let mut document = pair_document();
    link_pair(&mut document);
    let deleted = apply(
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
    for id in owners {
        assert!(
            deleted.contains(&id.to_string()),
            "a linked delete reports {id}: {deleted:?}"
        );
    }
}
