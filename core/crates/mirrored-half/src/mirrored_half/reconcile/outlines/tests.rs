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

fn reopen(document: &mut crate::model::ProjectDoc) -> Result<Vec<String>, String> {
    let (draft, _) = super::super::prepare_open(document,&[])?;
    let ids = super::super::changed_document_ids(document,&draft,&[])?;
    *document = draft;
    Ok(ids)
}

fn outline_fixture() -> crate::model::ProjectDoc {
    let mut document = fixed_geometry_fixture().expect("valid pair fixture");
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
    document
}

fn outline_assembly_fixture() -> crate::model::ProjectDoc {
    let mut document =
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
    document
}

include!("tests/projection.rs");
include!("tests/removal.rs");
