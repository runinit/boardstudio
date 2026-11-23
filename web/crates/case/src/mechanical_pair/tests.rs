use super::*;
use boardstudio_model::model::{
    Board, GasketPlacement, HardwareConfiguration, HardwareTopology, HardwareTransport,
    MechanicalConstructionOwnership, MechanicalFeatureValue, MechanicalGasketAnchor,
    MechanicalGasketLayout, MechanicalMirrorPair, Mount, MountKind, PhysicalBoardInstance, Vec2,
};
use serde_json::json;

fn configuration(board_id: &str) -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": board_id, "method":"printed", "mount":"tray",
        "integratedPlateFrame":false, "bottomStyle":"shell", "middleFrame":false,
        "plateThickness":1.5, "plateFoamThickness":3.0, "pcbThickness":1.6,
        "bottomFoamThickness":2.0, "batteryHeight":0.0, "bottomThickness":3.0,
        "plateToPcb":3.5, "wallThickness":2.0, "clearance":0.3,
        "mounts":[], "closureMounts":[], "profiles":[]
    }))
    .unwrap()
}

fn instance(id: &str, board_id: &str) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "custom-role".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(configuration(board_id)),
        construction_ownership: MechanicalConstructionOwnership::Pair,
        mechanical_identity_reservation: None,
    }
}

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

fn anchor(id: &str, fraction: f64) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        length: None,
        width: None,
        placement: Some(GasketPlacement::Generated),
        id: id.into(),
        region_id: "region-1".into(),
        outline_key: "outline-1".into(),
        anchor: fraction,
        unlinked: false,
    }
}

fn gasket_layout(supports: Vec<MechanicalGasketAnchor>) -> MechanicalGasketLayout {
    MechanicalGasketLayout {
        auto_size: None,
        adhesive_thickness: None,
        minimum_foam_thickness: None,
        preset_id: None,
        material: None,
        length: 80.0,
        width: 3.0,
        thickness: 2.0,
        compression: 0.15,
        supports,
    }
}

fn linked_feature(
    id: &str,
    kind: MechanicalFeatureKind,
    value: MechanicalFeatureValue,
    first_state: MechanicalFeatureState,
    second_state: MechanicalFeatureState,
) -> MechanicalFeature {
    MechanicalFeature {
        id: id.into(),
        kind,
        author: "left".into(),
        value,
        members: vec![
            MechanicalFeatureMember {
                instance_id: "left".into(),
                feature_id: format!("{id}-left"),
                state: first_state,
            },
            MechanicalFeatureMember {
                instance_id: "right".into(),
                feature_id: format!("{id}-right"),
                state: second_state,
            },
        ],
    }
}

/// One pair with a local mount, a linked opening and an omitted gasket support.
fn document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut left = configuration("board");
    left.mounts = vec![mount("mount-1-left", 2.0, 3.0)];
    left.openings = Some(vec![]);
    left.gasket_layout = Some(gasket_layout(vec![anchor("support-1-left", 0.25)]));
    let mut right = configuration("board");
    right.mounts = vec![mount("mount-1-right", -2.0, 3.0)];
    right.gasket_layout = Some(gasket_layout(vec![anchor("support-1-right", 0.25)]));
    let pair = MechanicalMirrorPair {
        id: "pair-1".into(),
        members: ["left".into(), "right".into()],
        mapping: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        construction: features::shared_construction(&left),
        construction_source: None,
        halves: None,
        features: vec![
            linked_feature(
                "mount-1",
                MechanicalFeatureKind::Mount,
                MechanicalFeatureValue::Mount {
                    mount: mount("mount-1-left", 2.0, 3.0),
                },
                MechanicalFeatureState::Local,
                MechanicalFeatureState::Linked,
            ),
            linked_feature(
                "opening-1",
                MechanicalFeatureKind::Opening,
                MechanicalFeatureValue::Opening {
                    opening: boardstudio_model::model::MechanicalOpening {
                        id: "opening-1".into(),
                        vertices: vec![],
                        z: 1.0,
                        height: 2.0,
                        next_vertex_identity: 0,
                    },
                },
                MechanicalFeatureState::Linked,
                MechanicalFeatureState::Linked,
            ),
            linked_feature(
                "support-1",
                MechanicalFeatureKind::GasketSupport,
                MechanicalFeatureValue::GasketSupport {
                    anchor: anchor("support-1-left", 0.25),
                },
                MechanicalFeatureState::Omitted,
                MechanicalFeatureState::Linked,
            ),
        ],
    };
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Split,
        transport: HardwareTransport::None,
        instances: vec![
            PhysicalBoardInstance {
                mechanical: Some(left),
                ..instance("left", "board")
            },
            PhysicalBoardInstance {
                mechanical: Some(right),
                ..instance("right", "board")
            },
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![pair],
    });
    doc
}

#[test]
fn projection_reports_each_linked_feature_and_its_accepted_state() {
    let doc = document();
    let projection = pair_projection(&doc, &[], "left").expect("the half belongs to a pair");
    assert_eq!(projection.pair_id, "pair-1");
    assert_eq!(
        projection.members,
        ["left".to_string(), "right".to_string()]
    );
    let kinds: Vec<_> = projection
        .features
        .iter()
        .map(|row| (row.feature_id.as_str(), row.kind, row.state))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (
                "mount-1-left",
                MechanicalFeatureKind::Mount,
                MechanicalFeatureState::Local
            ),
            (
                "opening-1-left",
                MechanicalFeatureKind::Opening,
                MechanicalFeatureState::Linked
            ),
            (
                "support-1-left",
                MechanicalFeatureKind::GasketSupport,
                MechanicalFeatureState::Omitted
            ),
        ],
        "the states come from the accepted pair store, covering mounts, openings and anchors"
    );
    assert_eq!(projection.features[0].counterpart_instance_id, "right");
    assert_eq!(
        projection.features[0].counterpart_feature_id,
        "mount-1-right"
    );
    assert!(
        projection.features[0].rejoin_fingerprint.is_some(),
        "each row carries the fingerprint of the reviewed rejoin inputs"
    );
    assert_eq!(
        projection.features[0].own_geometry.as_deref(),
        Some("Mount at (2.0, 3.0)"),
        "a local half carries its own geometry for a deliberate own-source rejoin"
    );
    let omitted = projection
        .features
        .iter()
        .find(|row| row.state == MechanicalFeatureState::Omitted)
        .expect("the omitted anchor row");
    assert_eq!(
        omitted.own_geometry, None,
        "an omitted half has no geometry of its own to take"
    );
    assert_eq!(
        projection.features[0].preview.as_deref(),
        Some("Mount at (-2.0, 3.0)"),
        "a rejoin previews the geometry the source half actually shows"
    );
}

#[test]
fn projection_is_absent_for_an_unpaired_or_unknown_half() {
    let mut doc = document();
    doc.hardware.as_mut().unwrap().pairs.clear();
    assert_eq!(pair_projection(&doc, &[], "left"), None);
    let doc = document();
    assert_eq!(pair_projection(&doc, &[], "missing"), None);
    let mut without_hardware = ProjectDoc::empty("doc", "Fixture");
    without_hardware.boards.push(doc.boards[0].clone());
    assert_eq!(pair_projection(&without_hardware, &[], "left"), None);
}

/// Two independent halves: one shared mount id and one half-local id.
fn independent_halves() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut left = configuration("board");
    left.mounts = vec![mount("mount-1", 2.0, 3.0), mount("mount-left", 8.0, 3.0)];
    let mut right = configuration("board");
    right.mounts = vec![mount("mount-1", -2.0, 3.0), mount("mount-right", -8.0, 3.0)];
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Split,
        transport: HardwareTransport::None,
        instances: vec![
            PhysicalBoardInstance {
                mechanical: Some(left),
                flipped: false,
                ..instance("left", "board")
            },
            PhysicalBoardInstance {
                mechanical: Some(right),
                flipped: true,
                ..instance("right", "board")
            },
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    doc
}

/// Two pair-free halves with the same durable mount id at different positions, so a
/// projection that ran is distinguishable from one that returned the current state.
fn linkable_halves() -> ProjectDoc {
    let mut doc = independent_halves();
    doc.hardware.as_mut().unwrap().instances[1]
        .mechanical
        .as_mut()
        .unwrap()
        .mounts[0] = mount("mount-1", 5.0, 5.0);
    doc
}

/// The normalized mounting-hole definition Core validates on a supplied template.

#[test]
fn the_reviewed_choices_become_the_link_payload_the_review_shows() {
    let doc = linkable_halves();
    let option = pair_link_options(&doc, &[], "left", None)
        .into_iter()
        .next()
        .expect("one candidate");
    let mut sources = std::collections::BTreeMap::new();

    // No map, no payload: a translated frame must supply one.
    assert!(
        reviewed_link_change(&option, None, false, &sources).is_some(),
        "a derived map is enough"
    );
    let mut explicit = option.clone();
    explicit.derived_mapping = None;
    assert!(
        reviewed_link_change(&explicit, None, false, &sources).is_none(),
        "an entered map is required before anything is submitted"
    );

    let mapping = Pose2 {
        at: Vec2 { x: 30.0, y: -5.0 },
        rotation: 180.0,
    };
    let change = reviewed_link_change(&explicit, Some(mapping), true, &sources)
        .expect("the entered map makes the payload");
    let MechanicalChange::LinkMirrorPair {
        members,
        mapping: payload_mapping,
        construction_source,
        correspondences,
        ..
    } = change
    else {
        panic!("a reviewed link is a LinkMirrorPair");
    };
    assert_eq!(members, ["left".to_string(), "right".to_string()]);
    assert_eq!(payload_mapping, mapping);
    assert_eq!(
        construction_source, "right",
        "the construction source follows the review's choice"
    );
    assert_eq!(
        correspondences[0].source_instance_id.as_deref(),
        Some("left"),
        "each match starts from the reviewed half"
    );

    // Choosing the other half per feature is the payload the review shows.
    sources.insert("mount-1".to_owned(), "right".to_owned());
    let change =
        reviewed_link_change(&option, None, false, &sources).expect("a derived map is enough");
    let MechanicalChange::LinkMirrorPair {
        construction_source,
        correspondences,
        ..
    } = change
    else {
        panic!("a reviewed link is a LinkMirrorPair");
    };
    assert_eq!(construction_source, "left");
    assert_eq!(
        correspondences[0].source_instance_id.as_deref(),
        Some("right"),
        "the per-feature choice reaches the payload"
    );
}

#[test]
fn link_options_offer_each_other_half_with_its_map_and_durable_matches() {
    let doc = independent_halves();
    let options = pair_link_options(&doc, &[], "left", None);
    assert_eq!(options.len(), 1, "the other half is the one candidate");
    let option = &options[0];
    assert_eq!(option.instance_id, "left");
    assert_eq!(option.other_instance_id, "right");
    assert_eq!(option.pair_id, "pair/left");
    assert_eq!(
        option.derived_mapping,
        Some(Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        }),
        "opposite declared transforms imply the physical reflection"
    );
    assert_eq!(
        option.correspondences.len(),
        1,
        "only the shared durable id matches; the half-local mount stays local"
    );
    assert_eq!(option.matched_features, "Mount mount-1");
    assert!(
        option.reviewed_fingerprint.is_some(),
        "the review carries the fingerprint of the inputs it was produced from"
    );
    assert_eq!(
        option.matched.len(),
        1,
        "each match is reviewable on its own"
    );
    let matched = &option.matched[0];
    assert_eq!(matched.first_feature_id, "mount-1");
    assert_eq!(matched.second_feature_id, "mount-1");
    assert_eq!(
        matched.source_instance_id, "left",
        "the review starts each match from the half being reviewed"
    );
    assert_eq!(
        matched.first_geometry.as_deref(),
        Some("Mount at (2.0, 3.0)")
    );
    assert_eq!(
        matched.second_geometry.as_deref(),
        Some("Mount at (-2.0, 3.0)"),
        "the other half's current geometry is shown for the choice"
    );
    assert_eq!(
        option.correspondences[0].source_instance_id.as_deref(),
        Some("left"),
        "the reviewed payload names a source per feature explicitly"
    );
    assert!(pair_link_options(&doc, &[], "missing", None,).is_empty());
}

#[test]
fn link_options_need_an_explicit_map_for_translated_frames_and_are_absent_when_paired() {
    let mut doc = independent_halves();
    doc.hardware.as_mut().unwrap().instances[1].flipped = false;
    let options = pair_link_options(&doc, &[], "left", None);
    assert_eq!(
        options[0].derived_mapping, None,
        "equal declared transforms need the review to supply the map"
    );
    assert_eq!(
        options[0].correspondences.len(),
        1,
        "the matches are still reviewable without a derived map"
    );

    let mut paired = doc.clone();
    paired.hardware.as_mut().unwrap().pairs = vec![MechanicalMirrorPair {
        id: "pair-1".into(),
        members: ["left".into(), "right".into()],
        mapping: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        construction: features::shared_construction(&configuration("board")),
        construction_source: None,
        halves: None,
        features: vec![],
    }];
    assert!(
        pair_link_options(&paired, &[], "left", None,).is_empty(),
        "a paired half is linked through its pair actions instead"
    );
    assert!(pair_link_options(&paired, &[], "right", None,).is_empty());
}

fn rejoin(instance_id: &str, feature_id: &str, source: &str) -> MechanicalChange {
    MechanicalChange::RejoinFeatureMember {
        feature_id: feature_id.into(),
        instance_id: instance_id.into(),
        source_instance_id: source.into(),
    }
}

#[test]
fn the_rejoin_fingerprint_tolerates_unrelated_edits_and_revision_changes() {
    let doc = document();
    let change = rejoin("left", "mount-1-left", "right");
    let reviewed = reviewed_fingerprint(&doc, &[], &change).expect("the reviewed inputs exist");

    let mut unrelated = doc.clone();
    unrelated.name = "Renamed".into();
    unrelated.revision += 7;
    unrelated
        .parameters
        .insert("unrelated".into(), json!("keep"));
    unrelated.hardware.as_mut().unwrap().instances[0]
        .mechanical
        .as_mut()
        .unwrap()
        .wall_thickness = 2.5;
    assert_eq!(
        reviewed_fingerprint(&unrelated, &[], &change).unwrap(),
        reviewed,
        "unrelated edits and a revision change do not retire the review"
    );

    // Each relevant input does change it.
    let mut moved = doc.clone();
    moved.hardware.as_mut().unwrap().pairs[0].mapping.rotation = 90.0;
    assert_ne!(
        reviewed_fingerprint(&moved, &[], &change).unwrap(),
        reviewed
    );

    let mut authored = doc.clone();
    authored.hardware.as_mut().unwrap().pairs[0].features[0].value =
        MechanicalFeatureValue::Mount {
            mount: mount("mount-1-left", 9.0, 3.0),
        };
    assert_ne!(
        reviewed_fingerprint(&authored, &[], &change).unwrap(),
        reviewed
    );

    let mut membership = doc.clone();
    membership.hardware.as_mut().unwrap().pairs[0].members[1] = "other".into();
    membership.hardware.as_mut().unwrap().pairs[0]
        .features
        .iter_mut()
        .for_each(|feature| feature.members[1].instance_id = "other".into());
    assert_ne!(
        reviewed_fingerprint(&membership, &[], &change).unwrap(),
        reviewed
    );

    let mut state = doc.clone();
    state.hardware.as_mut().unwrap().pairs[0].features[0].members[0].state =
        MechanicalFeatureState::Linked;
    assert_ne!(
        reviewed_fingerprint(&state, &[], &change).unwrap(),
        reviewed
    );

    assert_eq!(
        reviewed_fingerprint(&doc, &[], &rejoin("left", "mount-1-left", "right")).unwrap(),
        reviewed_fingerprint(&doc, &[], &rejoin("left", "mount-1-left", "left")).unwrap(),
        "the reviewed source choice is the user's payload, not accepted-document state"
    );
}

#[test]
fn a_vanished_reviewed_target_has_no_fingerprint() {
    let doc = document();
    assert!(
        reviewed_fingerprint(&doc, &[], &rejoin("left", "missing", "right")).is_err(),
        "a feature that is no longer linked has no reviewed inputs"
    );
    assert!(reviewed_fingerprint(&doc, &[], &rejoin("missing", "mount-1-left", "right")).is_err());
    assert!(reviewed_fingerprint(&doc, &[], &MechanicalChange::Enable).is_err());
}
