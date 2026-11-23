use super::*;

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

fn opening(id: &str) -> MechanicalOpening {
    MechanicalOpening {
        id: id.into(),
        vertices: vec![
            MechanicalOpeningVertex {
                id: format!("{id}-a"),
                point: Vec2 { x: 0.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: format!("{id}-b"),
                point: Vec2 { x: 3.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: format!("{id}-c"),
                point: Vec2 { x: 1.0, y: 1.0 },
            },
        ],
        z: 1.0,
        height: 2.0,
        // Not zero, so a later vertex addition cannot mint an id these fixtures
        // already use. (Zero means "unset", and Core then reserves its own.)
        next_vertex_identity: 3,
    }
}

fn feature_instance(
    doc: &ProjectDoc,
    id: &str,
    board_id: &str,
    flipped: bool,
    mounts: Vec<Mount>,
    openings: Vec<MechanicalOpening>,
) -> PhysicalBoardInstance {
    let mut configuration = mechanical(doc, 4.2);
    configuration.board_id = board_id.into();
    configuration.mounts = mounts;
    configuration.openings = Some(openings);
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "custom-role".into(),
        flipped,
        controller_part_id: None,
        mechanical: Some(configuration),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

/// A reviewed explicit link with no extra reviewed choice: the map comes from the
/// halves' declared transforms and the construction source is the first member.
fn link_intent(pair_id: &str, first: &str, second: &str) -> SetupIntent {
    SetupIntent::LinkMirrorPair {
        pair_id: pair_id.into(),
        first_instance_id: first.into(),
        second_instance_id: second.into(),
        mapping: None,
        construction_source: None,
        correspondences: None,
    }
}

fn mounting_hole_template() -> PartDefinition {
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

fn two_board_document() -> ProjectDoc {
    let mut doc = document();
    doc.boards.push(Board {
        id: "board-b".into(),
        name: "Board B".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc
}

fn split_hardware(
    left: PhysicalBoardInstance,
    right: PhysicalBoardInstance,
) -> HardwareConfiguration {
    HardwareConfiguration {
        topology: HardwareTopology::Split,
        transport: HardwareTransport::None,
        instances: vec![left, right],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    }
}

fn same_board_halves(doc: &ProjectDoc, with_opening: bool) -> [PhysicalBoardInstance; 2] {
    [("left", false, 2.0), ("right", true, -2.0)].map(|(id, flipped, x)| {
        feature_instance(
            doc,
            id,
            "board-a",
            flipped,
            vec![mount("mount-1", x, 1.0)],
            with_opening
                .then(|| opening("opening-1"))
                .into_iter()
                .collect(),
        )
    })
}

fn reversible_document_with_mount() -> (ProjectDoc, MechanicalConfiguration) {
    let mut doc = document();
    doc.parameters
        .insert("reversibleLayout".into(), Value::Bool(true));
    let mut settings = mechanical(&doc, 4.2);
    settings.mounts = vec![mount("mount-1", 12.0, 3.0)];
    (doc, settings)
}

fn propose_split(doc: &ProjectDoc) -> PhysicalSetupProposal {
    propose_with_mirror_pair(
        doc,
        SetupIntent::Topology {
            board_id: "board-a".into(),
            selected_instance_id: None,
            split: true,
            new_primary_id: "left-new".into(),
            new_secondary_id: "right-new".into(),
        },
        keep_definition,
    )
    .unwrap()
}

fn shared_mount_and_opening(
    mount_id: &str,
    opening_id: &str,
) -> Vec<MechanicalFeatureCorrespondence> {
    [
        (MechanicalFeatureKind::Mount, mount_id),
        (MechanicalFeatureKind::Opening, opening_id),
    ]
    .map(|(kind, id)| MechanicalFeatureCorrespondence {
        kind,
        first_feature_id: id.into(),
        second_feature_id: id.into(),
        source_instance_id: None,
    })
    .into()
}

fn assert_reflection_round_trip(mapping: Pose2, point: Vec2, reflected: Vec2) {
    let mapped = pair::forward(mapping, point);
    assert_eq!(mapped, reflected);
    assert_ne!(mapped, point, "the mapping is not the identity");
    assert_eq!(pair::inverse(mapping, mapped), point, "round-trip");
}

#[test]
fn a_reviewed_mapping_links_distinct_board_halves_with_no_representable_declared_transform() {
    let mut doc = two_board_document();
    // Both halves are upright and scoped to different boards: the declared
    // transforms imply no map at all, so only a reviewed map can link them.
    let left = feature_instance(
        &doc,
        "left",
        "board-a",
        false,
        vec![mount("mount-a", 2.0, 1.0)],
        vec![],
    );
    let right = feature_instance(
        &doc,
        "right",
        "board-b",
        false,
        vec![mount("mount-b", 12.0, 4.0)],
        vec![],
    );
    doc.hardware = Some(split_hardware(left, right));
    let reviewed_mapping = Pose2 {
        at: Vec2 { x: 30.0, y: -5.0 },
        rotation: 180.0,
    };
    let reviewed = MechanicalFeatureCorrespondence {
        kind: MechanicalFeatureKind::Mount,
        first_feature_id: "mount-a".into(),
        second_feature_id: "mount-b".into(),
        source_instance_id: None,
    };
    let accepted = doc.clone();

    let proposal = propose_with_mirror_pair(
        &doc,
        SetupIntent::LinkMirrorPair {
            pair_id: "pair-reviewed".into(),
            first_instance_id: "left".into(),
            second_instance_id: "right".into(),
            mapping: Some(reviewed_mapping),
            construction_source: Some("right".into()),
            correspondences: Some(vec![reviewed.clone()]),
        },
        keep_definition,
    )
    .unwrap();

    assert_eq!(doc, accepted, "accepted input remains immutable");
    let link = proposal
        .mirror_pair
        .expect("an explicit reviewed map makes the halves eligible");
    assert_eq!(link.members, ["left".to_string(), "right".to_string()]);
    assert_eq!(link.mapping, reviewed_mapping, "the reviewed map is stored");
    assert_ne!(
        link.mapping,
        Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        "the review's translated/rotated map is not an inferred identity transform"
    );
    assert_eq!(link.construction_source, "right");
    assert_eq!(link.correspondences, vec![reviewed]);
}

#[test]
fn a_malformed_reviewed_link_is_an_error_not_a_silently_different_pair() {
    let mut doc = document();
    let [left, right] = same_board_halves(&doc, false);
    doc.hardware = Some(split_hardware(left, right));

    let cases = [
        SetupIntent::LinkMirrorPair {
            pair_id: "pair-1".into(),
            first_instance_id: "left".into(),
            second_instance_id: "right".into(),
            mapping: Some(Pose2 {
                at: Vec2 {
                    x: f64::NAN,
                    y: 0.0,
                },
                rotation: 0.0,
            }),
            construction_source: None,
            correspondences: None,
        },
        SetupIntent::LinkMirrorPair {
            pair_id: "pair-1".into(),
            first_instance_id: "left".into(),
            second_instance_id: "right".into(),
            mapping: None,
            construction_source: Some("other".into()),
            correspondences: None,
        },
        SetupIntent::LinkMirrorPair {
            pair_id: "pair-1".into(),
            first_instance_id: "left".into(),
            second_instance_id: "right".into(),
            mapping: None,
            construction_source: None,
            correspondences: Some(vec![MechanicalFeatureCorrespondence {
                kind: MechanicalFeatureKind::Mount,
                first_feature_id: "mount-missing".into(),
                second_feature_id: "mount-1".into(),
                source_instance_id: None,
            }]),
        },
    ];
    for intent in cases {
        let outcome = propose_with_mirror_pair(&doc, intent.clone(), keep_definition);
        assert!(
            outcome.is_err(),
            "a malformed reviewed link must not propose {intent:?}"
        );
    }
}

#[test]
fn a_reviewed_candidate_link_lists_the_derived_map_and_durable_matches() {
    let doc = document();
    let mut with_hardware = doc.clone();
    with_hardware.hardware = Some(HardwareConfiguration {
        instances: same_board_halves(&doc, true).into(),
        ..Default::default()
    });

    let review = review_mirror_pair_link(&with_hardware, "pair-1", "left", "right")
        .unwrap()
        .expect("opposite-flip halves have a reviewable candidate link");
    assert_eq!(review.members, ["left".to_string(), "right".to_string()]);
    assert_eq!(
        review.derived_mapping,
        Some(Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        }),
        "the declared transforms imply the physical reflection"
    );
    assert_eq!(review.construction_source, "left");
    assert_eq!(
        review.correspondences,
        shared_mount_and_opening("mount-1", "opening-1"),
        "the candidate matches durable feature ids only"
    );
    assert!(
        with_hardware.hardware.as_ref().unwrap().pairs.is_empty(),
        "a review never establishes the pair"
    );
}

#[test]
fn a_reviewed_candidate_for_translated_board_frames_still_lists_matches_without_a_map() {
    let mut doc = two_board_document();
    let left = feature_instance(
        &doc,
        "left",
        "board-a",
        false,
        vec![mount("mount-shared", 2.0, 1.0)],
        vec![],
    );
    let right = feature_instance(
        &doc,
        "right",
        "board-b",
        false,
        vec![mount("mount-shared", 12.0, 4.0)],
        vec![],
    );
    doc.hardware = Some(split_hardware(left, right));

    let review = review_mirror_pair_link(&doc, "pair-1", "left", "right")
        .unwrap()
        .expect("distinct boards with no declared transform are still reviewable");
    assert_eq!(
        review.derived_mapping, None,
        "an upright translated frame needs the review to supply its map"
    );
    assert_eq!(
        review.correspondences.len(),
        1,
        "the durable-id matches are still shown so the review can choose"
    );
}

#[test]
fn a_reviewed_candidate_is_absent_for_ineligible_targets_and_blank_ids_are_malformed() {
    let doc = document();
    let mut with_hardware = doc.clone();
    with_hardware.hardware = Some(HardwareConfiguration {
        instances: same_board_halves(&doc, false).into(),
        ..Default::default()
    });

    assert!(
        review_mirror_pair_link(&with_hardware, "pair-1", "left", "left")
            .unwrap()
            .is_none(),
        "one half cannot link with itself"
    );
    assert!(
        review_mirror_pair_link(&with_hardware, "pair-1", "left", "missing")
            .unwrap()
            .is_none(),
        "a missing half is ineligible"
    );
    assert!(
        review_mirror_pair_link(&doc, "pair-1", "left", "right")
            .unwrap()
            .is_none(),
        "a document without physical instances has no candidate"
    );
    with_hardware.hardware.as_mut().unwrap().pairs = vec![stored_pair(
        &with_hardware,
        "pair-existing",
        "left",
        "right",
    )];
    assert!(
        review_mirror_pair_link(&with_hardware, "pair-new", "left", "right")
            .unwrap()
            .is_none(),
        "an instance already owned by a pair is ineligible"
    );
    assert!(
        review_mirror_pair_link(&with_hardware, "  ", "left", "right").is_err(),
        "a blank pair id is malformed, not an ineligible target"
    );
}

#[test]
fn an_eligible_setup_proposal_establishes_its_pair_on_the_proposed_document() {
    let (mut doc, mut settings) = reversible_document_with_mount();
    settings.openings = Some(vec![opening("opening-1")]);
    doc.mechanical = Some(settings);
    let accepted = doc.clone();

    let proposal = propose_split(&doc);
    assert_eq!(doc, accepted, "accepted input remains immutable");
    let link = proposal
        .mirror_pair
        .as_ref()
        .expect("opposite-flip halves propose a mechanical pair");
    assert_eq!(link.pair_id, "pair/right-new");
    assert_eq!(
        link.members,
        ["left-new".to_string(), "right-new".to_string()]
    );
    assert_eq!(link.construction_source, "left-new");
    // The mapping is the physical board-local X reflection about zero.
    assert_eq!(
        link.mapping,
        Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        }
    );
    // An asymmetric point so a symmetric fixture cannot hide a double reflection.
    let point = Vec2 { x: 4.5, y: -1.25 };
    assert_reflection_round_trip(link.mapping, point, Vec2 { x: -4.5, y: -1.25 });
    assert_eq!(
        link.correspondences,
        shared_mount_and_opening("mount-1", "opening-1"),
        "shared mounts and openings are covered by durable id"
    );
    let proposal_hardware = proposal.document.hardware.as_ref().unwrap();
    assert!(!proposal_hardware.instances[0].flipped);
    assert!(
        proposal_hardware.instances[1].flipped,
        "the reversible second half is flipped"
    );
    assert_eq!(
        proposal_hardware.instances[1]
            .mechanical
            .as_ref()
            .unwrap()
            .mounts,
        proposal_hardware.instances[0]
            .mechanical
            .as_ref()
            .unwrap()
            .mounts,
        "the eligible second half keeps its authored clone for the reviewed link"
    );
    assert!(
        proposal_hardware.pairs.is_empty(),
        "the proposal document alone never carries the pair"
    );
    assert!(
        proposal_hardware
            .instances
            .iter()
            .all(|instance| instance.construction_ownership
                == MechanicalConstructionOwnership::Local),
        "the new halves start owning their own configuration, before the link \
         promotes them to the pair"
    );

    let document = submit_physical_setup(&doc, &proposal)
        .expect("the eligible pair is established through Core");

    let hardware = document.hardware.as_ref().unwrap();
    assert_eq!(hardware.instances.len(), 2);
    assert_eq!(
        hardware.pairs.len(),
        1,
        "the new halves and their pair land in one submission"
    );
    let pair = &hardware.pairs[0];
    assert_eq!(pair.id, "pair/right-new");
    assert_eq!(
        pair.members,
        ["left-new".to_string(), "right-new".to_string()]
    );
    assert_eq!(
        pair.construction,
        boardstudio_core_mechanical::mechanical::features::shared_construction(&mechanical(
            &doc, 4.2
        )),
        "Core derives the pair's shared construction from the reviewed source"
    );
    assert!(
        pair.features
            .iter()
            .any(|feature| feature.kind == MechanicalFeatureKind::Mount),
        "the shared mount is linked"
    );
    let ownership = |id: &str| {
        hardware
            .instances
            .iter()
            .find(|instance| instance.id == id)
            .unwrap()
            .construction_ownership
    };
    assert_eq!(ownership("left-new"), MechanicalConstructionOwnership::Pair);
    assert_eq!(
        ownership("right-new"),
        MechanicalConstructionOwnership::Pair
    );
    // The linked value is stored once, with its author; the counterpart's visible
    // geometry is derived by Core's effective read rather than persisted twice.
    let feature = pair
        .features
        .iter()
        .find(|feature| feature.kind == MechanicalFeatureKind::Mount)
        .expect("the shared mount is linked");
    assert_eq!(feature.author, "left-new");
    assert_eq!(
        feature.value,
        boardstudio_model::model::MechanicalFeatureValue::Mount {
            mount: mount("mount-1", 12.0, 3.0),
        }
    );
    let right = boardstudio_core_mechanical::mechanical::effective_inputs(
        &document,
        &[],
        &MechanicalTarget::Instance {
            instance_id: "right-new".into(),
        },
    )
    .expect("the mirrored half stays resolvable");
    assert_eq!(
        right.configuration.mounts[0].at,
        Vec2 { x: -12.0, y: 3.0 },
        "the counterpart mount is the reflection of the authored one"
    );
}

#[test]
fn a_setup_proposal_without_a_pair_needs_no_mounting_hole() {
    // Equal declared transforms imply no representable map, so this split has no
    // eligible pair and therefore needs no mounting-hole template.
    let mut doc = document();
    let settings = mechanical(&doc, 4.2);
    doc.mechanical = Some(settings.clone());
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![make_instance(
            "primary",
            "board-a",
            "unibody",
            "standalone",
            "Keyboard",
            true,
            settings,
        )],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    let proposal = propose_split(&doc);
    assert!(proposal.mirror_pair.is_none());
    let expected = proposal.document.clone();
    assert!(proposal.into_mirrored_operation(None).unwrap().is_none());
    assert_eq!(proposal.document, expected);
}

#[test]
fn a_pair_application_failure_aborts_the_whole_proposal() {
    let (mut doc, settings) = reversible_document_with_mount();
    doc.mechanical = Some(settings);
    let proposal = propose_split(&doc);
    assert!(proposal.mirror_pair.is_some());
    assert!(
        proposal.into_mirrored_operation(None).is_err(),
        "a pair that needs projection without its template publishes nothing"
    );
}

fn stored_pair(doc: &ProjectDoc, id: &str, first: &str, second: &str) -> MechanicalMirrorPair {
    MechanicalMirrorPair {
        id: id.into(),
        members: [first.into(), second.into()],
        mapping: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        construction: boardstudio_core_mechanical::mechanical::features::shared_construction(
            &mechanical(doc, 4.2),
        ),
        construction_source: None,
        halves: None,
        features: vec![],
    }
}

#[test]
fn distinct_pcb_halves_propose_an_explicit_non_identity_mapping_scoped_to_their_boards() {
    let mut doc = two_board_document();
    let left = feature_instance(
        &doc,
        "left",
        "board-a",
        false,
        vec![
            mount("mount-shared", 2.0, 1.0),
            mount("mount-left", 8.0, 1.0),
        ],
        vec![opening("opening-shared")],
    );
    let right = feature_instance(
        &doc,
        "right",
        "board-b",
        true,
        vec![
            mount("mount-shared", -2.0, 1.0),
            mount("mount-right", -8.0, 1.0),
        ],
        vec![opening("opening-shared")],
    );
    doc.hardware = Some(split_hardware(left, right));
    let accepted = doc.clone();

    let proposal = propose_with_mirror_pair(
        &doc,
        link_intent("pair-1", "left", "right"),
        keep_definition,
    )
    .unwrap();

    assert_eq!(doc, accepted, "accepted input remains immutable");
    assert_eq!(
        proposal.document, accepted,
        "an explicit link never reassigns an instance's board"
    );
    let link = proposal
        .mirror_pair
        .expect("opposite-flip distinct halves propose a mechanical pair");
    assert_eq!(link.members, ["left".to_string(), "right".to_string()]);
    assert_eq!(link.construction_source, "left");
    // The declared map is the same board-local X reflection; it is non-identity
    // and is stored explicitly rather than read from names or an axis.
    assert_eq!(
        link.mapping,
        Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        }
    );
    let point = Vec2 { x: 2.75, y: 6.5 };
    assert_reflection_round_trip(link.mapping, point, Vec2 { x: -2.75, y: 6.5 });
    assert_eq!(
        link.correspondences,
        shared_mount_and_opening("mount-shared", "opening-shared"),
        "only the shared durable ids are matched, never part coordinates"
    );
    let hardware = proposal.document.hardware.as_ref().unwrap();
    assert_eq!(hardware.instances[0].board_id, "board-a");
    assert_eq!(hardware.instances[1].board_id, "board-b");
}

#[test]
fn a_layout_only_pair_proposes_no_mechanical_pair_and_invents_no_instances() {
    let mut doc = document();
    doc.layouts.push(Layout {
        id: "left-layout".into(),
        name: "Left".into(),
        board_id: "board-a".into(),
        matrix_ids: vec!["matrix-a".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "right-layout".into(),
        name: "Right".into(),
        board_id: "board-a".into(),
        matrix_ids: vec!["matrix-b".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            correspondences: vec![],
            exceptions: vec![],
            source_id: "left-layout".into(),
            axis_x: 12.0,
            split_gap: None,
            spacing: None,
        }),
    });
    let accepted = doc.clone();

    let proposal = propose_with_mirror_pair(
        &doc,
        link_intent("pair-1", "left", "right"),
        keep_definition,
    )
    .unwrap();

    assert_eq!(proposal.mirror_pair, None);
    assert_eq!(proposal.document, accepted);
    assert!(
        proposal.document.hardware.is_none(),
        "a layout-only pair never invents physical instances or splits a board"
    );
}

// -- Equivalence: physical setup against CreateMirroredHalf (ticket 26) ---------------

/// The accepted starting document: one board with reversible layout on, and the unibody
/// instance a user has before splitting, carrying the board's linked mount and opening.
fn reversible_split_start() -> ProjectDoc {
    let (mut doc, mut settings) = reversible_document_with_mount();
    settings.openings = Some(vec![opening("opening-1")]);
    settings.closure_mounts = Some(vec![
        mount("closure-1", 4.0, 20.0),
        mount("closure-2", 20.0, 4.0),
    ]);
    doc.mechanical = Some(settings.clone());
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![make_instance(
            "left-new",
            "board-a",
            "unibody",
            "standalone",
            "Keyboard",
            false,
            settings,
        )],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    doc
}

/// Side A: the physical-setup split, reversible layout on, submitted through Core.
fn physical_setup_split(start: &ProjectDoc) -> ProjectDoc {
    let proposal = propose_with_mirror_pair(
        start,
        SetupIntent::Topology {
            board_id: "board-a".into(),
            selected_instance_id: Some("left-new".into()),
            split: true,
            new_primary_id: "left-new".into(),
            new_secondary_id: "right-new".into(),
        },
        keep_definition,
    )
    .expect("the split proposal is accepted");
    submit_physical_setup(start, &proposal)
        .expect("the physical-setup pair is established through Core")
}

fn submit_physical_setup(
    start: &ProjectDoc,
    proposal: &PhysicalSetupProposal,
) -> Result<ProjectDoc, String> {
    let (target_ids, operation) = proposal
        .into_mirrored_operation(Some(&mounting_hole_template()))?
        .ok_or("physical setup did not produce a mirrored-half operation")?;
    let mut engine = CoreEngine::new();
    engine.open(start.clone())?;
    let command = EditCommand {
        base_revision: start.revision,
        transaction_id: "physical-setup-create".into(),
        phase: EditPhase::Commit,
        target_ids,
        operation,
    };
    engine
        .commit_edit(command)
        .map(|committed| committed.document)
}

/// Side B: the Core `CreateMirroredHalf` ReuseBoard operation on the same starting document,
/// with the flip map, physical setup's labels and pair id, and the same mounting-hole template.
fn reuse_board_half(start: &ProjectDoc) -> ProjectDoc {
    use boardstudio_model::model::{
        MirroredHalfLabel, MirroredHalfLabels, MirroredHalfMode, MirroredHalfPhysicalSetup,
        MirroredIdRule,
    };
    let source = start.hardware.as_ref().unwrap().instances[0].clone();
    let shared_construction = source.mechanical.clone().unwrap();
    let mut engine = CoreEngine::new();
    engine
        .open(start.clone())
        .expect("Core opens the starting document");
    let command = EditCommand {
        base_revision: start.revision,
        transaction_id: "equivalence-half".into(),
        phase: EditPhase::Commit,
        target_ids: vec![],
        operation: EditOperation::CreateMirroredHalf {
            pair_id: "pair/right-new".into(),
            source_instance_id: "left-new".into(),
            labels: Some(MirroredHalfLabels {
                source: Some(MirroredHalfLabel {
                    name: Some("Left half".into()),
                    half: Some("left".into()),
                    role: Some("central".into()),
                }),
                mirror: Some(MirroredHalfLabel {
                    name: Some("Right half".into()),
                    half: Some("right".into()),
                    role: Some("peripheral".into()),
                }),
            }),
            physical_setup: Some(MirroredHalfPhysicalSetup {
                topology: HardwareTopology::Split,
                transport: HardwareTransport::Wireless,
                shared_construction,
                source_instance: source,
            }),
            companion_mode: Default::default(),
            companion_tweak: None,
            mode: MirroredHalfMode::ReuseBoard {
                instance_id: "right-new".into(),
            },
            mapping: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            id_rule: MirroredIdRule::BoardPrefix,
            mounting_hole: Some(Box::new(mounting_hole_template())),
        },
    };
    engine
        .commit_edit(command)
        .expect("Core accepts CreateMirroredHalf")
        .document
}

/// Every JSON leaf difference between two values, as (JSON pointer, A, B). A missing side
/// is `None`. Arrays of equal length compare element by element; any other mismatch is one row.
fn json_differences(
    path: &str,
    a: Option<&Value>,
    b: Option<&Value>,
    out: &mut Vec<(String, Option<Value>, Option<Value>)>,
) {
    match (a, b) {
        (Some(Value::Object(x)), Some(Value::Object(y))) => {
            let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for key in keys {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                json_differences(&format!("{path}/{escaped}"), x.get(key), y.get(key), out);
            }
        }
        (Some(Value::Array(x)), Some(Value::Array(y))) if x.len() == y.len() => {
            for (index, (p, q)) in x.iter().zip(y).enumerate() {
                json_differences(&format!("{path}/{index}"), Some(p), Some(q), out);
            }
        }
        _ if a == b => {}
        _ => out.push((path.into(), a.cloned(), b.cloned())),
    }
}

/// A short, comparable rendering of one side of a difference row.
fn show(value: &Option<Value>) -> String {
    match value {
        None => "<absent>".into(),
        Some(Value::Object(_)) => "object".into(),
        Some(other) => other.to_string(),
    }
}

/// Ticket 26: on one accepted reversible split document, physical setup (A) and the Core
/// `CreateMirroredHalf` ReuseBoard operation (B) must agree on everything the pair reflects.
/// The differences that remain are pinned exactly, so any new or vanished difference fails
/// with its JSON pointer. Ticket 32 supplies labels, resolving the six relabelling differences
/// recorded in ticket 26's Outcome; the four other differences stay pinned.
#[test]
fn physical_setup_and_reuse_board_agree_on_a_reversible_split_pair() {
    let start = reversible_split_start();
    let a = serde_json::to_value(physical_setup_split(&start)).unwrap();
    let b = serde_json::to_value(reuse_board_half(&start)).unwrap();

    // Must be equal, and present (the comparison is not vacuous).
    for pointer in [
        "/hardware/instances/0/name",
        "/hardware/instances/0/half",
        "/hardware/instances/0/role",
        "/hardware/instances/1/name",
        "/hardware/instances/1/half",
        "/hardware/instances/1/role",
        "/hardware/pairs/0/id",
        "/hardware/pairs/0/members",
        "/hardware/pairs/0/mapping",
        "/hardware/pairs/0/constructionSource",
        "/hardware/pairs/0/construction",
        "/hardware/pairs/0/features",
        "/hardware/instances/1/flipped",
        "/hardware/instances/1/controllerPartId",
        "/hardware/instances/0/constructionOwnership",
        "/hardware/instances/1/constructionOwnership",
        "/hardware/instances/0/mechanical",
        "/hardware/instances/1/mechanical",
    ] {
        let left = a.pointer(pointer);
        assert!(left.is_some(), "A has no {pointer}");
        assert_eq!(left, b.pointer(pointer), "{pointer} differs");
    }
    let features = a.pointer("/hardware/pairs/0/features").unwrap();
    assert_eq!(
        features.as_array().map(Vec::len),
        Some(4),
        "the shared mount, opening and both closure mounts are linked: {features}"
    );
    let closure_parts = |doc: &Value| {
        doc["parts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|part| part["id"].as_str().unwrap().starts_with("case-closure/"))
            .map(|part| part["id"].clone())
            .collect::<Vec<_>>()
    };
    assert!(
        !closure_parts(&a).is_empty(),
        "the generated case-closure parts exist on A"
    );
    assert_eq!(
        closure_parts(&a),
        closure_parts(&b),
        "the generated case-closure parts are the same"
    );

    // Every other difference, pinned exactly: (JSON pointer, A side, B side).
    let mut rows = Vec::new();
    json_differences("", Some(&a), Some(&b), &mut rows);
    let observed: Vec<(String, String, String)> = rows
        .iter()
        .map(|(path, left, right)| (path.clone(), show(left), show(right)))
        .collect();
    for (path, left, right) in &observed {
        eprintln!("EQUIVALENCE ROW {path} | A={left} | B={right}");
    }
    let expected: Vec<(String, String, String)> = vec![];
    assert_eq!(observed, expected, "the differences changed");
}
