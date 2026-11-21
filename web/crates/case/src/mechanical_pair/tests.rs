use super::*;
use boardstudio_model::model::{
    Board, BoardContours, Contour, GasketPlacement, HardwareConfiguration, HardwareTopology,
    HardwareTransport, MechanicalConstructionOwnership, MechanicalFeatureValue,
    MechanicalGasketAnchor, MechanicalGasketLayout, MechanicalMirrorPair, Mount, MountKind,
    PhysicalBoardInstance, Vec2,
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
fn mounting_hole_template() -> PartDefinition {
    serde_json::from_value(json!({
        "id": "generator:ceoloide/mounting_hole_npth", "name": "mounting hole npth",
        "kind": "custom",
        "pads": [{
            "id": "pad-0", "number": "", "at": { "x": 0, "y": 0 },
            "size": { "x": 2.2, "y": 2.2 }, "shape": "circle", "drill": 2.2,
            "plated": false, "rotation": 0
        }],
        "courtyard": [
            { "x": -1.1, "y": -1.1 }, { "x": 1.1, "y": -1.1 },
            { "x": 1.1, "y": 1.1 }, { "x": -1.1, "y": 1.1 }
        ],
        "envelopeSource": { "courtyard": "generated" },
        "envelopeNotice": "No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator": { "source": "ceoloide/mounting_hole_npth", "version": "bundled-1", "parameters": {} }
    }))
    .expect("the mounting-hole fixture parses")
}

/// Two pair-free halves whose shared closure mount generates closure clearance.
fn linkable_halves_with_closure_holes() -> ProjectDoc {
    let mut doc = linkable_halves();
    let closure = Mount {
        id: "closure-1".into(),
        at: Vec2 { x: 1.0, y: 2.0 },
        kind: MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    };
    for instance in &mut doc.hardware.as_mut().unwrap().instances {
        let configuration = instance.mechanical.as_mut().unwrap();
        configuration.closure_mounts = Some(vec![closure.clone()]);
    }
    doc
}

/// A pair of gasket-mounted halves whose shared support is resolved on the author's
/// contours, mirroring Core's own gasket fixture.
fn gasket_pair_document() -> (ProjectDoc, Vec<BoardContours>) {
    let mut document = ProjectDoc::empty("gasket-pair", "Gasket pair");
    for board_id in ["wide", "narrow"] {
        document.boards.push(Board {
            id: board_id.into(),
            name: board_id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
    }
    let gasket = |board_id: &str| {
        let mut configuration = configuration(board_id);
        configuration.mount = boardstudio_model::model::MechanicalMount::Gasket;
        configuration.gasket_layout =
            Some(boardstudio_core_mechanical::mechanical::default_gasket_layout());
        configuration
    };
    let instance = |id: &str, board_id: &str| PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(gasket(board_id)),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    };
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![instance("left", "wide"), instance("right", "narrow")],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    let rectangle = |half_width: f64, half_height: f64| Contour {
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
    };
    let accepted = vec![
        BoardContours {
            board_id: "wide".into(),
            contours: vec![rectangle(200.0, 150.0)],
        },
        BoardContours {
            board_id: "narrow".into(),
            contours: vec![rectangle(10.0, 5.0)],
        },
    ];
    (document, accepted)
}

/// The author's own resolved support, placed on both halves as the reviewed anchor.
fn resolved_author_support(
    document: &ProjectDoc,
    accepted: &[BoardContours],
) -> boardstudio_model::model::MechanicalGasketSupport {
    let mut author_document = document.clone();
    author_document.physical_instance_id = Some("left".into());
    author_document.mechanical = Some(
        document
            .hardware
            .as_ref()
            .expect("hardware")
            .instances
            .iter()
            .find(|instance| instance.id == "left")
            .expect("the author")
            .mechanical
            .clone()
            .expect("the author's configuration"),
    );
    let contours = accepted
        .iter()
        .find(|entry| entry.board_id == "wide")
        .expect("the wide board")
        .contours
        .clone();
    let assembly = boardstudio_core_mechanical::mechanical::resolve(&author_document, &contours);
    assert!(
        !assembly.generation_blocked,
        "the author's assembly resolves: {:?}",
        assembly.diagnostics
    );
    assembly
        .gasket_supports
        .first()
        .cloned()
        .expect("the fixture resolves a gasket support")
}

#[test]
fn a_link_review_describes_resolved_gasket_geometry_not_the_authored_fraction() {
    let (mut document, accepted) = gasket_pair_document();
    let author = resolved_author_support(&document, &accepted);
    let anchor = MechanicalGasketAnchor {
        length: None,
        width: None,
        placement: Some(GasketPlacement::User),
        id: author.id.clone(),
        region_id: author.region_id.clone(),
        outline_key: author.outline_key.clone(),
        anchor: author.anchor,
        unlinked: false,
    };
    for instance in document
        .hardware
        .as_mut()
        .expect("hardware")
        .instances
        .iter_mut()
    {
        instance
            .mechanical
            .as_mut()
            .expect("configuration")
            .gasket_layout
            .as_mut()
            .expect("gasket layout")
            .supports = vec![anchor.clone()];
    }
    let change = MechanicalChange::LinkMirrorPair {
        pair_id: "pair-left-right".into(),
        members: ["left".into(), "right".into()],
        mapping: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        construction_source: "left".into(),
        correspondences: vec![MechanicalFeatureCorrespondence {
            kind: MechanicalFeatureKind::GasketSupport,
            first_feature_id: anchor.id.clone(),
            second_feature_id: anchor.id.clone(),
            source_instance_id: Some("left".into()),
        }],
    };

    let preview = link_preview(
        &document,
        &accepted,
        &change,
        Some(&mounting_hole_template()),
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    )
    .expect("the reviewed link projects");

    let rows = |instance_id: &str| {
        preview
            .members
            .iter()
            .find(|member| member.instance_id == instance_id)
            .expect("the member preview")
            .features
            .clone()
    };
    // The reviewed row states numbers, so compare the placement numerically: the
    // author's own resolution and the linked projection may differ in the last bits.
    fn placement(row: &str) -> (f64, f64) {
        let inner = row
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(inner, _)| inner)
            .unwrap_or_else(|| panic!("a resolved support row states its placement: {row}"));
        let (x, y) = inner
            .split_once(", ")
            .unwrap_or_else(|| panic!("a placement has two numbers: {inner}"));
        (
            x.trim().parse().expect("x is a number"),
            y.trim().parse().expect("y is a number"),
        )
    }
    let close = |actual: (f64, f64), expected: Vec2| {
        (actual.0 - expected.x).abs() < 0.5 && (actual.1 - expected.y).abs() < 0.5
    };
    let author_row = rows("left")
        .into_iter()
        .find(|row| row.starts_with("Gasket support"))
        .expect("the author's support row");
    assert!(
        close(placement(&author_row), author.at),
        "the review states the resolved placement {:?}, got {author_row}",
        author.at
    );
    assert!(
        author_row.contains("· 80.0 × 3.0 mm"),
        "the review states the resolved size, got {author_row}"
    );
    assert!(
        !author_row.contains("along its region"),
        "the authored fraction is not the reviewed geometry, got {author_row}"
    );

    // The other half shows the exact reflection of the author's resolved support,
    // and the board that cannot receive it reports the located fit error instead.
    let expected = boardstudio_core_mechanical::mechanical::features::mirror_support(
        Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        &author,
    );
    let mirrored_row = rows("right")
        .into_iter()
        .find(|row| row.starts_with("Gasket support"))
        .expect("the mirrored support row");
    assert!(
        close(placement(&mirrored_row), expected.at),
        "the review states the reflected placement {:?}, got {mirrored_row}",
        expected.at
    );
    assert!(
        mirrored_row.contains("does not fit"),
        "a placed reflection that cannot fit its own outline says so, got {mirrored_row}"
    );
    let _ = &document;
}

#[test]
fn a_link_that_needs_generated_holes_shows_its_complete_result_once_the_template_is_known() {
    let doc = linkable_halves_with_closure_holes();

    // Without the normalized template the projection cannot produce the generated
    // clearance, so the review must say so instead of showing a partial result.
    let without = pair_link_options(
        &doc,
        &[],
        "left",
        None,
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    );
    assert_eq!(without.len(), 1, "the candidate is still reviewable");
    assert!(
        without[0].preview.is_none(),
        "a projection that must create generated holes is unavailable without its template"
    );
    assert!(
        without[0]
            .preview_note
            .as_deref()
            .is_some_and(|note| !note.is_empty()),
        "the review explains why the complete result is unavailable"
    );

    // With the template the same candidate shows its complete proposed result.
    let template = mounting_hole_template();
    let with_template = pair_link_options(
        &doc,
        &[],
        "left",
        Some(&template),
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    );
    assert_eq!(with_template.len(), 1);
    let preview = with_template[0]
        .preview
        .as_ref()
        .expect("the template makes the complete result available before confirmation");
    assert_eq!(preview.members.len(), 2);
    assert!(
        preview.members.iter().all(|member| member
            .features
            .iter()
            .any(|feature| feature.contains("Closure mount"))),
        "both halves show the linked closure mount, got {:?}",
        preview.members
    );
    assert_eq!(
        with_template[0].preview_note, None,
        "no unavailable note once the complete result is projected"
    );
}

#[test]
fn a_reviewed_link_projects_both_halves_through_core_without_touching_accepted_state() {
    let doc = linkable_halves();
    let change = MechanicalChange::LinkMirrorPair {
        pair_id: "pair/left".into(),
        members: ["left".into(), "right".into()],
        mapping: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        construction_source: "left".into(),
        correspondences: vec![MechanicalFeatureCorrespondence {
            kind: MechanicalFeatureKind::Mount,
            first_feature_id: "mount-1".into(),
            second_feature_id: "mount-1".into(),
            source_instance_id: None,
        }],
    };
    let accepted = doc.clone();

    let preview = link_preview(
        &doc,
        &[],
        &change,
        None,
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    )
    .expect("the reviewed link projects through the disposable engine");

    assert_eq!(
        doc, accepted,
        "a read-only projection never mutates or stores the accepted document"
    );
    assert_eq!(preview.members.len(), 2);
    assert_eq!(preview.members[0].instance_id, "left");
    assert_eq!(preview.members[1].instance_id, "right");
    assert!(
        preview.members[0]
            .features
            .iter()
            .any(|feature| feature == "Mount at (2.0, 3.0)"),
        "the authoring half keeps its authored geometry, got {:?}",
        preview.members[0].features
    );
    assert!(
        preview.members[1]
            .features
            .iter()
            .any(|feature| feature == "Mount at (-2.0, 3.0)"),
        "the mirrored half shows the reflected counterpart, not its pre-link geometry, got {:?}",
        preview.members[1].features
    );
    assert!(
        !preview.members[1]
            .features
            .iter()
            .any(|feature| feature == "Mount at (5.0, 5.0)"),
        "the pre-link geometry is not part of the proposed result"
    );
    assert!(
        preview
            .members
            .iter()
            .all(|member| member.findings.is_empty()),
        "a clean link projects no findings"
    );
}

#[test]
fn the_reviewed_choices_become_the_link_payload_the_review_shows() {
    let doc = linkable_halves();
    let option = pair_link_options(
        &doc,
        &[],
        "left",
        None,
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    )
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
    let options = pair_link_options(
        &doc,
        &[],
        "left",
        None,
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    );
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
    assert!(
        pair_link_options(
            &doc,
            &[],
            "missing",
            None,
            boardstudio_web_test_runtime::engine_core_services().open_scratch
        )
        .is_empty()
    );
}

#[test]
fn link_options_need_an_explicit_map_for_translated_frames_and_are_absent_when_paired() {
    let mut doc = independent_halves();
    doc.hardware.as_mut().unwrap().instances[1].flipped = false;
    let options = pair_link_options(
        &doc,
        &[],
        "left",
        None,
        boardstudio_web_test_runtime::engine_core_services().open_scratch,
    );
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
        pair_link_options(
            &paired,
            &[],
            "left",
            None,
            boardstudio_web_test_runtime::engine_core_services().open_scratch
        )
        .is_empty(),
        "a paired half is linked through its pair actions instead"
    );
    assert!(
        pair_link_options(
            &paired,
            &[],
            "right",
            None,
            boardstudio_web_test_runtime::engine_core_services().open_scratch
        )
        .is_empty()
    );
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

fn correspondences()
-> Vec<boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence> {
    vec![
        boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence {
            kind: MechanicalFeatureKind::Mount,
            first_feature_id: "mount-1-left".into(),
            second_feature_id: "mount-1-right".into(),
            source_instance_id: None,
        },
    ]
}

fn link(doc: &ProjectDoc, mapping: Pose2) -> MechanicalChange {
    let _ = doc;
    MechanicalChange::LinkMirrorPair {
        pair_id: "pair-new".into(),
        members: ["left".into(), "right".into()],
        mapping,
        construction_source: "left".into(),
        correspondences: correspondences(),
    }
}

#[test]
fn the_link_fingerprint_covers_the_accepted_inputs_and_ignores_the_reviewed_choices() {
    let mut doc = document();
    doc.hardware.as_mut().unwrap().pairs.clear();
    let change = link(
        &doc,
        Pose2 {
            at: Vec2 { x: 30.0, y: -5.0 },
            rotation: 180.0,
        },
    );
    let reviewed = reviewed_fingerprint(&doc, &[], &change).expect("the reviewed inputs exist");

    let mut renamed = doc.clone();
    renamed.name = "Renamed".into();
    renamed.revision += 3;
    renamed.parameters.insert("unrelated".into(), json!("keep"));
    assert_eq!(
        reviewed_fingerprint(&renamed, &[], &change).unwrap(),
        reviewed,
        "unrelated project edits and a revision change do not retire a reviewed link"
    );

    // The map, the construction source and the source choices are the reviewed
    // payload: changing them is not document staleness.
    let other_choices = MechanicalChange::LinkMirrorPair {
        pair_id: "pair-new".into(),
        members: ["left".into(), "right".into()],
        mapping: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        construction_source: "right".into(),
        correspondences: correspondences(),
    };
    assert_eq!(
        reviewed_fingerprint(&doc, &[], &other_choices).unwrap(),
        reviewed,
        "the reviewed map and construction source are the user's own choices"
    );

    // A declared-transform change alters the map a review derives, so it retires.
    let mut reoriented = doc.clone();
    reoriented.hardware.as_mut().unwrap().instances[1].flipped = true;
    assert_ne!(
        reviewed_fingerprint(&reoriented, &[], &change).unwrap(),
        reviewed,
        "changing a half's declared transform retires the review"
    );

    // Moving the geometry a match consolidates retires it too.
    let mut moved = doc.clone();
    moved.hardware.as_mut().unwrap().instances[0]
        .mechanical
        .as_mut()
        .unwrap()
        .mounts = vec![mount("mount-1-left", 8.0, 1.0)];
    assert_ne!(
        reviewed_fingerprint(&moved, &[], &change).unwrap(),
        reviewed,
        "moving a matched feature retires the review"
    );

    let mut already_paired = doc.clone();
    let mut stored = document().hardware.unwrap().pairs;
    stored[0].id = "pair-new".into();
    already_paired.hardware.as_mut().unwrap().pairs = stored;
    assert!(
        reviewed_fingerprint(&already_paired, &[], &change).is_err(),
        "an existing pair id makes the reviewed link ineligible"
    );
}

#[test]
fn a_gasket_anchor_rejoin_previews_the_source_geometry_it_would_take() {
    let doc = document();
    let projection = pair_projection(&doc, &[], "left").expect("the half is paired");
    let support = projection
        .features
        .iter()
        .find(|row| row.kind == MechanicalFeatureKind::GasketSupport)
        .expect("the anchor row exists");
    assert_eq!(support.state, MechanicalFeatureState::Omitted);
    assert_eq!(
        support.preview.as_deref(),
        Some("Gasket support at 0.25 along its region"),
        "an omitted anchor previews the source half's visible anchor"
    );

    // An omitted source half has no geometry to rejoin with, so no preview.
    let right = pair_projection(&doc, &[], "right").expect("the other half is paired too");
    let omitted_source = right
        .features
        .iter()
        .find(|row| row.kind == MechanicalFeatureKind::GasketSupport)
        .expect("the anchor row exists");
    assert_eq!(omitted_source.preview, None);
}
