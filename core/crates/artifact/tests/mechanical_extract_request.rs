//! The artifact request path for mechanical extraction.
//!
//! It lives here, one layer above `boardstudio-core-kicad-source`, so that crate's own tests do
//! not depend on the artifact/engine graph.

use boardstudio_core_artifact::artifact;
use boardstudio_core_artifact::model::{
    ArtifactReply, ArtifactRequest, MechanicalGeometryKind as GeometryKind, MechanicalPurpose,
    MechanicalPurposeMapping as PurposeMapping,
};

#[test]
fn artifact_request_returns_selectable_source_geometry_and_mapped_cutouts() {
    let source = include_str!("../../../tests/fixtures/mechanical/SW_MX_1u.kicad_mod");
    let request = ArtifactRequest::ExtractMechanical {
        id: "extract-test".into(),
        source: source.into(),
        mappings: vec![
            PurposeMapping {
                source_id: None,
                kind: Some(GeometryKind::Line),
                layer: Some("Eco2.User".into()),
                purpose: MechanicalPurpose::PlateCutout,
            },
            PurposeMapping {
                source_id: None,
                kind: Some(GeometryKind::Arc),
                layer: Some("Eco2.User".into()),
                purpose: MechanicalPurpose::PlateCutout,
            },
        ],
        max_deviation_mm: 0.01,
    };
    let json = serde_json::to_string(&request).unwrap();
    let response: ArtifactReply = serde_json::from_str(&artifact::request(&json)).unwrap();
    let ArtifactReply::ExtractMechanical { id, result } = response else {
        panic!("expected mechanical extraction reply");
    };
    assert_eq!(id, "extract-test");
    assert_eq!(result.geometry.source_name.as_deref(), Some("SW_MX_1u"));
    assert!(!result.geometry.primitives.is_empty());
    assert_eq!(result.plate_cutouts.len(), 1);
    assert!(result.clearance_envelopes.is_empty());
    assert_eq!(result.source_geometry.text, source);
    assert_eq!(result.source_geometry.source_ids.len(), 8);
    assert!(result.pcb_holes.is_empty());
}
