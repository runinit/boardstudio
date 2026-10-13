use super::{Projection, reconcile as reconcile_candidate};
use super::{fixed_geometry_fixture, fixed_membership_assembly_fixture, pair_state_bytes};
use crate::model::{
    BoardOutline, MirroredCorrespondence, MirroredItemKind, Operation, OutlineBridge,
    OutlineConnection, OutlineControlPoint, OutlineFeature, OutlineOwnership, OutlineProvenance,
    OutlineSnapshot, OutlineVersion, ProtectedOutlineGap, Vec2,
};

fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn control(x: f64, y: f64, part_id: Option<&str>) -> OutlineControlPoint {
    OutlineControlPoint {
        at: point(x, y),
        part_id: part_id.map(str::to_owned),
    }
}

fn source_snapshot() -> OutlineSnapshot {
    OutlineSnapshot {
        features: vec![OutlineFeature::Polygon {
            anchor_part_id: Some("matrix/source-matrix/r0c1".into()),
            id: "source-board/feature".into(),
            points: vec![point(1.0, 2.0), point(3.0, 4.0), point(5.0, 6.0)],
            operation: Operation::Subtract,
        }],
        settings: crate::model::OutlineSettings {
            corners: crate::model::CornerStyle::Fillet,
            size: 2.75,
            bridge_width: 8.25,
            repair: Some(crate::model::OutlineRepairSettings {
                enabled: true,
                maximum_gap_span: 13.5,
                minimum_connection_width: 2.25,
                edge_clearance: 0.4,
                keep_gaps: vec![ProtectedOutlineGap {
                    id: "source-board/setting-gap".into(),
                    points: vec![control(2.0, 3.0, Some("matrix/source-matrix/r0c1"))],
                }],
            }),
        },
        expected_regions: 3,
        bridges: vec![OutlineBridge {
            id: "source-board/bridge".into(),
            width: 4.25,
            points: vec![point(7.0, 8.0), point(9.0, 10.0)],
            part_ids: vec!["matrix/source-matrix/r0c1".into()],
            matrix_ids: vec!["source-matrix".into()],
            authored: true,
        }],
        protected_gaps: vec![ProtectedOutlineGap {
            id: "source-board/snapshot-gap".into(),
            points: vec![
                control(11.0, 12.0, None),
                control(13.0, 14.0, Some("matrix/source-matrix/r0c1")),
            ],
        }],
    }
}

fn version(id: &str, ownership: OutlineOwnership, geometry: OutlineSnapshot) -> OutlineVersion {
    OutlineVersion {
        id: id.into(),
        name: format!("version {id}"),
        source: OutlineProvenance {
            revision: 17,
            version_id: Some("source-board/version/linked".into()),
        },
        geometry,
        ownership,
    }
}

#[derive(Debug, PartialEq)]
struct ProjectionState {
    source_board_id: String,
    mirror_board_id: String,
    matrix: Option<(String, String)>,
    mapping: crate::model::Pose2,
    parts: Vec<(String, String)>,
    independent_part_transitions: Vec<super::super::IndependentPartTransition>,
    hosted_transitions: Vec<super::super::HostedTransition>,
    applied_independent_additions: std::collections::BTreeSet<(String, String, String, String)>,
    applied_hosted_operations: std::collections::BTreeSet<(String, String, String)>,
    applied_hosted_circuit_additions: std::collections::BTreeMap<
        (String, String, String),
        super::super::HostedCircuitAddProvenance,
    >,
    hosted_removal_baselines:
        std::collections::BTreeMap<(String, String, String), crate::model::ProjectDoc>,
    hosted_omission_baselines:
        std::collections::BTreeMap<(String, String, String), crate::model::ProjectDoc>,
    outline_removal_baselines:
        std::collections::BTreeMap<(String, String, String), crate::model::ProjectDoc>,
    locally_protected_mirror_parts: std::collections::BTreeSet<String>,
    final_owners: std::collections::BTreeMap<String, String>,
}

fn projection_state(projection: &Projection) -> ProjectionState {
    ProjectionState {
        source_board_id: projection.source_board_id.clone(),
        mirror_board_id: projection.mirror_board_id.clone(),
        matrix: projection.matrix.as_ref().map(|matrix| {
            (
                matrix.source_matrix_id.clone(),
                matrix.mirror_matrix_id.clone(),
            )
        }),
        mapping: projection.mapping,
        parts: projection
            .parts
            .iter()
            .map(|part| (part.source_id.clone(), part.mirror_id.clone()))
            .collect(),
        independent_part_transitions: projection.independent_part_transitions.clone(),
        hosted_transitions: projection.hosted_transitions.clone(),
        applied_independent_additions: projection.applied_independent_additions.borrow().clone(),
        applied_hosted_operations: projection.applied_hosted_operations.borrow().clone(),
        applied_hosted_circuit_additions: projection
            .applied_hosted_circuit_additions
            .borrow()
            .clone(),
        hosted_removal_baselines: projection.hosted_removal_baselines.borrow().clone(),
        hosted_omission_baselines: projection.hosted_omission_baselines.borrow().clone(),
        outline_removal_baselines: projection.outline_removal_baselines.borrow().clone(),
        locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
        final_owners: projection.final_owners.clone(),
    }
}

fn outline_fixture() -> (crate::model::ProjectDoc, Projection) {
    let (mut document, projection) = fixed_geometry_fixture().expect("valid pair fixture");
    let source_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .expect("source board");
    source_board.outline_ids.push("source-board/feature".into());
    document.outline.push(OutlineFeature::Polygon {
        anchor_part_id: Some("matrix/source-matrix/r0c1".into()),
        id: "source-board/feature".into(),
        points: vec![point(1.0, 2.0), point(3.0, 4.0), point(5.0, 6.0)],
        operation: Operation::Subtract,
    });

    let mirror_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .expect("mirror board");
    mirror_board.outline_ids.push("mirror-board/feature".into());
    document.outline.push(OutlineFeature::Polygon {
        anchor_part_id: Some("matrix/mirror-matrix/r0c1".into()),
        id: "mirror-board/feature".into(),
        points: vec![point(99.0, 98.0), point(97.0, 96.0), point(95.0, 94.0)],
        operation: Operation::Add,
    });

    let source_geometry = source_snapshot();
    let mirror_geometry = OutlineSnapshot {
        features: vec![],
        settings: Default::default(),
        expected_regions: 1,
        bridges: vec![],
        protected_gaps: vec![],
    };
    document.board_outlines = vec![
        BoardOutline {
            board_id: "source-board".into(),
            active_version_id: Some("source-board/version/linked".into()),
            versions: vec![
                version(
                    "source-board/version/linked",
                    OutlineOwnership::Linked,
                    source_geometry.clone(),
                ),
                version(
                    "source-board/version/fixed",
                    OutlineOwnership::Fixed,
                    source_geometry,
                ),
            ],
            generated_last_valid: None,
            retired_version_ids: vec![],
        },
        BoardOutline {
            board_id: "mirror-board".into(),
            active_version_id: Some("mirror-board/version/linked".into()),
            versions: vec![
                version(
                    "mirror-board/version/linked",
                    OutlineOwnership::Linked,
                    mirror_geometry.clone(),
                ),
                version(
                    "mirror-board/version/fixed",
                    OutlineOwnership::Fixed,
                    mirror_geometry,
                ),
                version(
                    "mirror-board/local-version",
                    OutlineOwnership::Fixed,
                    OutlineSnapshot {
                        features: vec![],
                        settings: Default::default(),
                        expected_regions: 9,
                        bridges: vec![],
                        protected_gaps: vec![],
                    },
                ),
            ],
            generated_last_valid: Some(OutlineSnapshot {
                features: vec![],
                settings: Default::default(),
                expected_regions: 99,
                bridges: vec![],
                protected_gaps: vec![],
            }),
            retired_version_ids: vec!["mirror-board/retired-version".into()],
        },
    ];
    for version in &mut document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
    {
        version.source.version_id = Some(version.id.clone());
    }
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Outline,
            source_id: "source-board/feature".into(),
            mirror_id: "mirror-board/feature".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::OutlineVersion,
            source_id: "source-board/version/linked".into(),
            mirror_id: "mirror-board/version/linked".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::OutlineVersion,
            source_id: "source-board/version/fixed".into(),
            mirror_id: "mirror-board/version/fixed".into(),
        },
    ]);
    (document, projection)
}

fn outline_assembly_fixture() -> (crate::model::ProjectDoc, Projection) {
    let (mut document, projection) =
        fixed_membership_assembly_fixture(90.0).expect("valid linked assembly fixture");
    document.assemblies.push(
        serde_json::from_value(serde_json::json!({
            "id": "shared-assembly",
            "name": "Shared assembly",
            "members": [{
                "id": "shared-assembly/diode",
                "definitionId": "diode",
                "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
                "side": "front",
                "models": []
            }]
        }))
        .expect("nonempty shared assembly definition"),
    );
    let geometry = OutlineSnapshot {
        features: vec![],
        settings: Default::default(),
        expected_regions: 2,
        bridges: vec![OutlineBridge {
            id: "source-board/assembly-bridge".into(),
            width: 1.5,
            points: vec![point(3.0, 5.0), point(7.0, 11.0)],
            part_ids: vec!["matrix/source-matrix/r0c1/diode".into()],
            matrix_ids: vec!["source-matrix".into()],
            authored: true,
        }],
        protected_gaps: vec![],
    };
    document.board_outlines = vec![
        BoardOutline {
            board_id: "source-board".into(),
            active_version_id: Some("source-board/version/assembly".into()),
            versions: vec![version(
                "source-board/version/assembly",
                OutlineOwnership::Linked,
                geometry,
            )],
            generated_last_valid: None,
            retired_version_ids: vec![],
        },
        BoardOutline {
            board_id: "mirror-board".into(),
            active_version_id: Some("mirror-board/version/assembly".into()),
            versions: vec![version(
                "mirror-board/version/assembly",
                OutlineOwnership::Linked,
                OutlineSnapshot {
                    features: vec![],
                    settings: Default::default(),
                    expected_regions: 1,
                    bridges: vec![],
                    protected_gaps: vec![],
                },
            )],
            generated_last_valid: None,
            retired_version_ids: vec![],
        },
    ];
    let source_feature = OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: "source-board/assembly-outline".into(),
        center: point(1.0, 2.0),
        size: point(3.0, 4.0),
        radius: 0.0,
        operation: Operation::Subtract,
    };
    let mirror_feature = OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: "mirror-board/assembly-outline".into(),
        center: point(0.0, 0.0),
        size: point(1.0, 1.0),
        radius: 0.0,
        operation: Operation::Add,
    };
    document.outline.extend([source_feature, mirror_feature]);
    for (board_id, outline_id) in [
        ("source-board", "source-board/assembly-outline"),
        ("mirror-board", "mirror-board/assembly-outline"),
    ] {
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .outline_ids
            .push(outline_id.into());
    }
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Outline,
            source_id: "source-board/assembly-outline".into(),
            mirror_id: "mirror-board/assembly-outline".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::OutlineVersion,
            source_id: "source-board/version/assembly".into(),
            mirror_id: "mirror-board/version/assembly".into(),
        },
    ]);
    (document, projection)
}

#[test]
fn linked_outline_and_fixed_and_linked_versions_project_literal_anchored_payload() {
    let (mut document, projection) = outline_fixture();
    document.assemblies.push(
        serde_json::from_value(serde_json::json!({
            "id": "shared-assembly",
            "name": "Shared assembly",
            "members": []
        }))
        .unwrap(),
    );
    let assemblies_before = document.assemblies.clone();
    let pair_before = pair_state_bytes(&document);
    let changed = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid outline projection");

    let target = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/feature")
        .expect("mapped outline remains under its stable identity");
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        operation,
        ..
    } = target
    else {
        panic!("the target feature keeps its source variant")
    };
    assert_eq!(anchor_part_id.as_deref(), Some("matrix/mirror-matrix/r0c1"));
    assert_eq!(
        points,
        &vec![point(-5.0, 6.0), point(-3.0, 4.0), point(-1.0, 2.0)]
    );
    assert_eq!(*operation, Operation::Subtract);

    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .expect("target board outline");
    assert_eq!(
        target_outline.active_version_id.as_deref(),
        Some("mirror-board/version/linked")
    );
    assert_eq!(target_outline.generated_last_valid, None);
    assert_eq!(
        target_outline.retired_version_ids,
        ["mirror-board/retired-version"]
    );
    assert_eq!(target_outline.versions.len(), 3);
    let linked = target_outline
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/linked")
        .unwrap();
    let fixed = target_outline
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/fixed")
        .unwrap();
    assert_eq!(linked.ownership, OutlineOwnership::Linked);
    assert_eq!(fixed.ownership, OutlineOwnership::Fixed);
    assert_eq!(linked.source.revision, 17);
    assert_eq!(
        linked.source.version_id.as_deref(),
        Some("mirror-board/version/linked")
    );
    assert_eq!(linked.geometry.expected_regions, 3);
    assert_eq!(linked.geometry.features[0].id(), "mirror-board/feature");
    assert_eq!(
        linked.geometry.bridges[0].id,
        "mirror-board/source-board/bridge"
    );
    assert_eq!(linked.geometry.bridges[0].matrix_ids, ["mirror-matrix"]);
    assert_eq!(
        linked.geometry.bridges[0].points,
        vec![point(92.0, -37.0), point(90.0, -39.0)]
    );
    assert_eq!(
        linked.geometry.protected_gaps[0].id,
        "mirror-board/source-board/snapshot-gap"
    );
    assert_eq!(
        linked.geometry.protected_gaps[0].points,
        vec![
            control(88.0, -41.0, None),
            control(-13.0, 14.0, Some("matrix/mirror-matrix/r0c1")),
        ]
    );
    assert_eq!(
        linked.geometry.settings.repair.as_ref().unwrap().keep_gaps[0].id,
        "mirror-board/source-board/setting-gap"
    );
    assert!(changed.iter().any(|id| id == "mirror-board/feature"));
    assert_eq!(pair_state_bytes(&document), pair_before);
    assert_eq!(
        document.assemblies, assemblies_before,
        "shared definitions are reused"
    );

    let first_output = document.clone();
    let repeat = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("updated repeat");
    assert!(repeat.is_empty());
    assert_eq!(document, first_output);
}

#[test]
fn a_cross_matrix_anchor_uses_its_own_matrix_frame_rule() {
    let (mut document, projection) = outline_fixture();
    let source_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    let mirror_matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    let source_thumb_id = "source-thumb";
    let mirror_thumb_id = "mirror-thumb";
    let mut source_thumb = source_matrix.clone();
    source_thumb.id = source_thumb_id.into();
    source_thumb.name = Some("Source thumb".into());
    source_thumb.origin.x += 40.0;
    source_thumb.part_ids = source_matrix
        .part_ids
        .iter()
        .map(|id| id.replacen("matrix/source-matrix/", "matrix/source-thumb/", 1))
        .collect();
    let mut mirror_thumb = mirror_matrix.clone();
    mirror_thumb.id = mirror_thumb_id.into();
    mirror_thumb.name = Some("Mirror thumb".into());
    mirror_thumb.origin = crate::mechanical::pair::forward(projection.mapping, source_thumb.origin);
    mirror_thumb.part_ids = mirror_matrix
        .part_ids
        .iter()
        .map(|id| id.replacen("matrix/mirror-matrix/", "matrix/mirror-thumb/", 1))
        .collect();

    for (old_id, new_id) in source_matrix.part_ids.iter().zip(&source_thumb.part_ids) {
        let mut part = document.parts.iter().find(|part| part.id == *old_id).unwrap().clone();
        part.id.clone_from(new_id);
        part.reference = format!("THUMB-{}", part.reference);
        document.parts.push(part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push(new_id.clone());
    }
    for (old_id, new_id) in mirror_matrix.part_ids.iter().zip(&mirror_thumb.part_ids) {
        let mut part = document.parts.iter().find(|part| part.id == *old_id).unwrap().clone();
        part.id.clone_from(new_id);
        part.reference = format!("THUMB-{}", part.reference);
        document.parts.push(part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(new_id.clone());
    }
    document.matrices.extend([source_thumb, mirror_thumb]);
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: source_thumb_id.into(),
        mirror_id: mirror_thumb_id.into(),
    });
    for (source_id, mirror_id) in source_matrix.part_ids.iter().zip(&mirror_matrix.part_ids) {
        halves.correspondences.push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_id.replacen("source-matrix", "source-thumb", 1),
            mirror_id: mirror_id.replacen("mirror-matrix", "mirror-thumb", 1),
        });
    }

    let source_feature = document
        .outline
        .iter_mut()
        .find(|feature| feature.id() == "source-board/feature")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = source_feature
    else {
        panic!("the source feature remains a polygon")
    };
    *anchor_part_id = Some("matrix/source-thumb/r0c0".into());
    *points = vec![point(2.0, 3.0), point(4.0, 5.0), point(6.0, 7.0)];
    let source_linked_version = document
        .board_outlines
        .iter_mut()
        .find(|state| state.board_id == "source-board")
        .unwrap()
        .versions
        .iter_mut()
        .find(|version| version.id == "source-board/version/linked")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = &mut source_linked_version.geometry.features[0]
    else {
        panic!("the source version feature remains a polygon")
    };
    *anchor_part_id = Some("matrix/source-thumb/r0c0".into());
    *points = vec![point(2.0, 3.0), point(4.0, 5.0), point(6.0, 7.0)];

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("the active base matrix and linked thumb matrix both remain valid");
    let mapped = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/feature")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = mapped
    else {
        panic!("the projected feature remains a polygon")
    };
    assert_eq!(anchor_part_id.as_deref(), Some("matrix/mirror-thumb/r0c0"));
    assert_eq!(
        points,
        &vec![point(-6.0, 7.0), point(-4.0, 5.0), point(-2.0, 3.0)]
    );
    let mirror_linked_version = document
        .board_outlines
        .iter()
        .find(|state| state.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/linked")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = &mirror_linked_version.geometry.features[0]
    else {
        panic!("the mirrored version feature remains a polygon")
    };
    assert_eq!(anchor_part_id.as_deref(), Some("matrix/mirror-thumb/r0c0"));
    assert_eq!(
        points,
        &vec![point(-6.0, 7.0), point(-4.0, 5.0), point(-2.0, 3.0)]
    );
}

#[test]
fn outline_projection_never_copies_generated_case_closure_members() {
    let (mut document, projection) = outline_fixture();
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    let envelope = OutlineFeature::PartEnvelope {
        connections: vec![OutlineConnection {
            id: "closure-connection".into(),
            width: 1.25,
            points: vec![control(0.0, 0.0, None), control(2.0, 0.0, None)],
        }],
        settings: Default::default(),
        id: "source-board/envelope".into(),
        part_ids: vec![
            "matrix/source-matrix/r0c1".into(),
            "case-closure/generated-mount".into(),
        ],
        margin: 1.75,
        operation: Operation::Add,
    };
    source.versions[0].geometry.features.push(envelope);
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "source-board/envelope".into(),
        part_ids: vec![
            "matrix/source-matrix/r0c1".into(),
            "case-closure/generated-mount".into(),
        ],
        margin: 1.75,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .push("source-board/envelope".into());
    let mirror_board = document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    mirror_board
        .outline_ids
        .push("mirror-board/envelope".into());
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "mirror-board/envelope".into(),
        part_ids: vec!["matrix/mirror-matrix/r0c1".into()],
        margin: 0.0,
        operation: Operation::Add,
    });
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Outline,
        source_id: "source-board/envelope".into(),
        mirror_id: "mirror-board/envelope".into(),
    });

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid source outline");
    let mirrored = &document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions[0]
        .geometry
        .features;
    let envelope = mirrored
        .iter()
        .find_map(|feature| match feature {
            OutlineFeature::PartEnvelope { id, part_ids, .. } if id == "mirror-board/envelope" => {
                Some(part_ids)
            }
            _ => None,
        })
        .expect("mapped source envelope");
    assert_eq!(envelope, &["matrix/mirror-matrix/r0c1"]);
    assert!(!envelope.iter().any(|id| id.starts_with("case-closure/")));
}

#[test]
fn outline_exception_is_targeted_and_unrelated_local_version_survives_projection() {
    let (mut document, projection) = outline_fixture();
    let before_local = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/local-version")
        .unwrap()
        .clone();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions.push(crate::model::MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::OutlineVersion,
        removed_copper: None,
        mirror_id: "mirror-board/local-version".into(),
        state: crate::model::MirroredExceptionState::Local,
    });
    halves.exceptions.push(crate::model::MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Outline,
        removed_copper: None,
        mirror_id: "mirror-board/omitted-outline".into(),
        state: crate::model::MirroredExceptionState::Omitted,
    });

    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("mirror-board/omitted-outline".into());
    let before_fixed_local = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/local-version")
        .unwrap()
        .clone();
    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid outline exceptions");
    let target = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target
            .versions
            .iter()
            .find(|version| version.id == "mirror-board/local-version")
            .unwrap(),
        &before_fixed_local,
        "Local keeps the complete authored target payload"
    );
    assert!(
        !document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/omitted-outline"),
        "Omitted stays absent"
    );
    assert!(
        document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .contains(&"mirror-board/omitted-outline".to_owned()),
        "the exception identity remains registered for durable pair validity"
    );
    assert_eq!(
        target
            .versions
            .iter()
            .find(|version| version.id == "mirror-board/local-version")
            .unwrap(),
        &before_local,
        "unrelated target-owned choices remain unchanged"
    );
    let linked_feature = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/feature")
        .expect("other linked outline still projects");
    let OutlineFeature::Polygon { points, .. } = linked_feature else {
        panic!("linked target feature keeps its source variant")
    };
    assert_eq!(
        points,
        &vec![point(-5.0, 6.0), point(-3.0, 4.0), point(-1.0, 2.0)]
    );
    let first_output = document.clone();
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("updated exception repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}

#[test]
fn axis_map_projects_world_geometry_without_changing_established_identities() {
    let (mut document, mut projection) = outline_fixture();
    let mapping = crate::model::Pose2 {
        at: point(100.0, -30.0),
        rotation: 0.0,
    };
    document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
    projection.mapping = mapping;
    document.outline.push(OutlineFeature::Rect {
        rotation: Some(25.0),
        anchor_part_id: None,
        id: "source-board/axis-rect".into(),
        center: point(3.0, 4.0),
        size: point(8.0, 5.0),
        radius: 0.75,
        operation: Operation::Subtract,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .push("source-board/axis-rect".into());
    document.outline.push(OutlineFeature::Rect {
        rotation: Some(0.0),
        anchor_part_id: None,
        id: "mirror-board/axis-rect".into(),
        center: point(-9.0, -8.0),
        size: point(1.0, 1.0),
        radius: 0.0,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("mirror-board/axis-rect".into());
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Outline,
            source_id: "source-board/axis-rect".into(),
            mirror_id: "mirror-board/axis-rect".into(),
        });

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid axis-mapped outline");
    let OutlineFeature::Rect {
        rotation,
        center,
        size,
        radius,
        operation,
        ..
    } = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/axis-rect")
        .unwrap()
    else {
        panic!("the projected world rectangle keeps its variant")
    };
    assert_eq!(*rotation, Some(-25.0));
    assert_eq!(*center, point(97.0, -26.0));
    assert_eq!(*size, point(8.0, 5.0));
    assert_eq!(*radius, 0.75);
    assert_eq!(*operation, Operation::Subtract);
}

#[test]
fn independent_part_anchor_and_protected_gap_use_full_heading_coordinates() {
    let (mut document, projection) = outline_fixture();
    let mut source_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/source-matrix/r0c1")
        .unwrap()
        .clone();
    source_part.id = "source-board/independent-part".into();
    source_part.reference = "SRC-INDEPENDENT".into();
    let mut mirror_part = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c1")
        .unwrap()
        .clone();
    mirror_part.id = "mirror-board/independent-part".into();
    mirror_part.reference = "MIR-INDEPENDENT".into();
    document.parts.extend([source_part, mirror_part]);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .part_ids
        .push("source-board/independent-part".into());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("mirror-board/independent-part".into());
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: "source-board/independent-part".into(),
        mirror_id: "mirror-board/independent-part".into(),
    });
    halves.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Outline,
        source_id: "source-board/independent-anchor".into(),
        mirror_id: "mirror-board/independent-anchor".into(),
    });
    let source_outline = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source_outline.versions[0]
        .geometry
        .protected_gaps
        .push(ProtectedOutlineGap {
            id: "source-board/independent-gap".into(),
            points: vec![control(7.0, -11.0, Some("source-board/independent-part"))],
        });
    document.outline.push(OutlineFeature::Polygon {
        anchor_part_id: Some("source-board/independent-part".into()),
        id: "source-board/independent-anchor".into(),
        points: vec![point(2.0, 4.0), point(6.0, 8.0), point(10.0, 12.0)],
        operation: Operation::Subtract,
    });
    document.outline.push(OutlineFeature::Polygon {
        anchor_part_id: Some("mirror-board/independent-part".into()),
        id: "mirror-board/independent-anchor".into(),
        points: vec![point(0.0, 0.0); 3],
        operation: Operation::Add,
    });
    for (board_id, feature_id) in [
        ("source-board", "source-board/independent-anchor"),
        ("mirror-board", "mirror-board/independent-anchor"),
    ] {
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .outline_ids
            .push(feature_id.into());
    }

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid independent-part outline and protected-gap anchors");
    let independent_anchor = document
        .outline
        .iter()
        .find(|feature| feature.id() == "mirror-board/independent-anchor")
        .unwrap();
    let OutlineFeature::Polygon {
        anchor_part_id,
        points,
        ..
    } = independent_anchor
    else {
        panic!("the mapped independent anchor remains a polygon")
    };
    assert_eq!(
        anchor_part_id.as_deref(),
        Some("mirror-board/independent-part")
    );
    assert_eq!(
        points,
        &vec![point(10.0, -12.0), point(6.0, -8.0), point(2.0, -4.0)]
    );
    let linked = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .iter()
        .find(|version| version.id == "mirror-board/version/linked")
        .unwrap();
    assert_eq!(
        linked.geometry.protected_gaps[1].points,
        [control(7.0, 11.0, Some("mirror-board/independent-part"))]
    );
    assert_eq!(
        linked.geometry.protected_gaps[0].points[1],
        control(-13.0, 14.0, Some("matrix/mirror-matrix/r0c1")),
        "the key anchor keeps its separate -x,y reflection rule"
    );
}

#[test]
fn changed_source_outline_bridge_keeps_assembly_references_consistent() {
    let (mut document, projection) = outline_assembly_fixture();
    crate::mechanical::halves::validate_document(&document)
        .expect("the matrix assembly and outline references begin valid");
    let assemblies_before = document.assemblies.clone();
    let pair_before = pair_state_bytes(&document);

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("initial source assembly reference projects");
    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target_outline.versions[0].geometry.bridges[0].part_ids,
        ["matrix/mirror-matrix/r0c1/diode"]
    );
    assert_eq!(
        target_outline.versions[0].geometry.bridges[0].matrix_ids,
        ["mirror-matrix"]
    );

    let source_outline = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source_outline.versions[0].geometry.bridges[0].part_ids =
        vec!["matrix/source-matrix/r0c1/fallback".into()];
    source_outline.versions[0].geometry.bridges[0].width = 2.25;
    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("changed source assembly reference projects");

    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target_outline.versions[0].geometry.bridges[0].part_ids,
        ["matrix/mirror-matrix/r0c1/fallback"]
    );
    assert_eq!(target_outline.versions[0].geometry.bridges[0].width, 2.25);
    assert_eq!(
        document.assemblies, assemblies_before,
        "the referenced document-scoped assembly definition remains shared"
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    crate::mechanical::halves::validate_document(&document)
        .expect("mapped bridge references remain valid after the source edit");

    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("changed assembly reference repeat")
        .is_empty());
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn removing_source_version_prunes_only_its_mapped_target_and_retains_other_choices() {
    let (mut document, projection) = outline_fixture();
    let pair_before = pair_state_bytes(&document);
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/version/fixed".into(),
            "mirror-board/version/fixed".into(),
        ),
        document.clone(),
    );
    let private_context_before = projection.outline_removal_baselines.borrow().clone();
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source
        .versions
        .retain(|version| version.id != "source-board/version/fixed");
    source.active_version_id = Some("source-board/version/linked".into());

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid source version removal");
    let target = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(
        target.active_version_id.as_deref(),
        Some("mirror-board/version/linked")
    );
    assert!(
        target
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/version/linked")
    );
    assert!(
        !target
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/version/fixed")
    );
    assert!(
        target
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/local-version")
    );
    assert_eq!(
        target.retired_version_ids,
        ["mirror-board/retired-version", "mirror-board/version/fixed"]
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("updated version-removal repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(
        *projection.outline_removal_baselines.borrow(),
        private_context_before
    );
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn outline_removal_refuses_a_foreign_unpaired_target_atomically() {
    let (mut document, projection) = outline_fixture();
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .versions
        .push(version(
            "mirror-board/foreign-version",
            OutlineOwnership::Fixed,
            OutlineSnapshot {
                features: vec![],
                settings: Default::default(),
                expected_regions: 123,
                bridges: vec![],
                protected_gaps: vec![],
            },
        ));
    let baseline = document.clone();
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/version/fixed".into(),
            "mirror-board/foreign-version".into(),
        ),
        baseline,
    );
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap()
        .versions
        .retain(|version| version.id != "source-board/version/fixed");
    let post_source = document.clone();
    let post_source_context = projection_state(&projection);
    let error = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect_err("a valid but unrelated target version is not owned by this removal");
    assert!(
        error.contains("lacks one exact baseline correspondence"),
        "{error}"
    );
    assert_eq!(
        document, post_source,
        "refusal is atomic across the whole document"
    );
    assert_eq!(projection_state(&projection), post_source_context);
}

#[test]
fn removing_source_outline_retires_only_its_exact_target_and_updates_versions() {
    let (mut document, projection) = outline_fixture();
    let pair_before = pair_state_bytes(&document);
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/feature".into(),
            "mirror-board/feature".into(),
        ),
        document.clone(),
    );
    document
        .outline
        .retain(|feature| feature.id() != "source-board/feature");
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .retain(|id| id != "source-board/feature");
    for outline in document
        .board_outlines
        .iter_mut()
        .filter(|outline| outline.board_id == "source-board")
    {
        for version in &mut outline.versions {
            version
                .geometry
                .features
                .retain(|feature| feature.id() != "source-board/feature");
        }
    }

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid source outline retirement");
    assert!(
        !document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/feature")
    );
    assert!(
        !document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .contains(&"mirror-board/feature".to_owned())
    );
    let target_outline = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert!(target_outline.versions.iter().all(|version| {
        !version
            .geometry
            .features
            .iter()
            .any(|feature| feature.id() == "mirror-board/feature")
    }));
    assert_eq!(
        target_outline.retired_version_ids,
        ["mirror-board/retired-version"]
    );
    assert_eq!(pair_state_bytes(&document), pair_before);

    let first_output = document.clone();
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("updated outline-retirement repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
}

#[test]
fn source_outline_version_without_correspondence_is_refused_atomically() {
    let (mut document, projection) = outline_fixture();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.retain(|association| {
        !(association.kind == MirroredItemKind::OutlineVersion
            && association.source_id == "source-board/version/fixed")
    });
    let before = document.clone();
    let error = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect_err("source version work without a selected target must not silently no-op");
    assert!(error.contains("Source outline version \"source-board/version/fixed\" has no selected mirror correspondence"), "{error}");
    assert_eq!(
        document, before,
        "the refusal leaves the complete document untouched"
    );
}

#[test]
fn outline_only_removal_cleans_the_exact_target_feature_and_board_reference() {
    let (mut document, projection) = outline_fixture();
    document.board_outlines.clear();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves
        .correspondences
        .retain(|association| association.kind != MirroredItemKind::OutlineVersion);
    document.outline.push(OutlineFeature::Rect {
        rotation: None,
        anchor_part_id: None,
        id: "mirror-board/unrelated-outline".into(),
        center: point(3.0, 4.0),
        size: point(7.0, 9.0),
        radius: 0.0,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("mirror-board/unrelated-outline".into());
    crate::mechanical::halves::validate_document(&document)
        .expect("the pre-source outline-only baseline is valid");

    let pair_before = pair_state_bytes(&document);
    projection.outline_removal_baselines.borrow_mut().insert(
        (
            "pair-source-mirror".into(),
            "source-board/feature".into(),
            "mirror-board/feature".into(),
        ),
        document.clone(),
    );
    document
        .outline
        .retain(|feature| feature.id() != "source-board/feature");
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .outline_ids
        .retain(|id| id != "source-board/feature");

    let changed = reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("valid authenticated outline-only removal");
    assert!(
        !document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/feature")
    );
    let mirror_board = document
        .boards
        .iter()
        .find(|board| board.id == "mirror-board")
        .unwrap();
    assert!(
        !mirror_board
            .outline_ids
            .contains(&"mirror-board/feature".to_owned())
    );
    assert!(
        mirror_board
            .outline_ids
            .contains(&"mirror-board/unrelated-outline".to_owned())
    );
    assert!(
        document
            .outline
            .iter()
            .any(|feature| feature.id() == "mirror-board/unrelated-outline")
    );
    assert!(changed.contains(&"mirror-board/feature".to_owned()));
    assert_eq!(pair_state_bytes(&document), pair_before);

    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("outline-only removal repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn clearing_source_active_version_deselects_only_the_linked_target_version() {
    let (mut document, projection) = outline_fixture();
    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("initial linked selection projects");
    let source = document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap();
    source.active_version_id = None;
    let pair_before = pair_state_bytes(&document);

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("clearing the source selection projects generated selection");
    let mirror = document
        .board_outlines
        .iter()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap();
    assert_eq!(mirror.active_version_id, None);
    assert!(
        mirror
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/version/fixed")
    );
    assert!(
        mirror
            .versions
            .iter()
            .any(|version| version.id == "mirror-board/local-version")
    );
    assert_eq!(pair_state_bytes(&document), pair_before);

    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(
        reconcile_candidate(&mut document, "pair-source-mirror", &projection)
            .expect("generated selection repeat")
            .is_empty()
    );
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}

#[test]
fn clearing_source_active_version_preserves_a_local_target_selection() {
    let (mut document, projection) = outline_fixture();
    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.exceptions.push(crate::model::MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::OutlineVersion,
        removed_copper: None,
        mirror_id: "mirror-board/local-version".into(),
        state: crate::model::MirroredExceptionState::Local,
    });
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "source-board")
        .unwrap()
        .active_version_id = None;
    document
        .board_outlines
        .iter_mut()
        .find(|outline| outline.board_id == "mirror-board")
        .unwrap()
        .active_version_id = Some("mirror-board/local-version".into());
    crate::mechanical::halves::validate_document(&document)
        .expect("the independent local target selection is valid");
    let pair_before = pair_state_bytes(&document);

    reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("source generated selection does not overwrite a local target choice");
    assert_eq!(
        document
            .board_outlines
            .iter()
            .find(|outline| outline.board_id == "mirror-board")
            .unwrap()
            .active_version_id
            .as_deref(),
        Some("mirror-board/local-version")
    );
    assert_eq!(pair_state_bytes(&document), pair_before);
    let first_output = document.clone();
    let first_context = projection_state(&projection);
    assert!(reconcile_candidate(&mut document, "pair-source-mirror", &projection)
        .expect("local selection repeat")
        .is_empty());
    assert_eq!(document, first_output);
    assert_eq!(projection_state(&projection), first_context);
}
