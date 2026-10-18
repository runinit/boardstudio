//! Controlled consumer reports from the shared seam; no opaque handle or geometry proof is minted.
use crate::model_delivery::*;
use boardstudio_web_host::{cad_jobs::*, cad_qualification::*};
use std::rc::Rc;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json")).unwrap()
}

fn subjects(report: &QueryQualification) -> [CadCollisionSubject; 2] {
    let data = fixture();
    let activation = &data["qualificationV2"]["activation"];
    std::array::from_fn(|i| {
        let reference = [&report.binding.left_analysis, &report.binding.right_analysis][i];
        let descriptor = [&report.binding.left_mesh, &report.binding.right_mesh][i];
        let row = [report.binding.left_row_transform_f64, report.binding.right_row_transform_f64][i];
        let id = [&report.binding.left_model_id, &report.binding.right_model_id][i].clone();
        let name = if matches!(reference.source, Source::VendorStep { .. }) { "vendor" } else { "plane" };
        let source: SourceQualification = serde_json::from_value(activation["sourceReports"][name].clone()).unwrap();
        let bound = match source { SourceQualification::Bounded { total_bound_mm, .. } => Some(total_bound_mm), _ => None };
        match &reference.source {
            Source::GeneratedBody { body_id, revision, geometry_identity } => {
                let mut payload = data["directPayload"].clone();
                payload["geometryIdentity"] = geometry_identity.clone().into();
                let mut asset: CadGeneratedAnalysis = serde_json::from_value(payload).unwrap();
                asset.body_id = body_id.clone(); asset.revision = *revision;
                asset.analysis_ref = Some(reference.clone()); asset.analysis_mesh = Some(descriptor.clone());
                asset.source_qualification = Some(source); asset.analysis_error_bound_mm = bound;
                CadCollisionSubject::GeneratedCaseBody { id, name: body_id.clone(), row_transform_f64: row, analysis: Some(Rc::new(asset)) }
            }
            Source::VendorStep { source_sha256, import_options } => {
                let mut asset = CadVendorStep { source_sha256: source_sha256.clone(),
                    import_options: CadStepImportOptions { preview_repair_budget_mm: import_options.preview_repair_budget_mm, target_deflection_mm: import_options.target_deflection_mm },
                    analysis_ref: Some(reference.clone()), analysis_mesh: Some(descriptor.clone()), source_qualification: Some(source),
                    surface_positions_mm_f64: vec![0.; 27], triangle_face_occurrence_indices: vec![0; 3],
                    source_faces: vec![CadStepSourceFace { support_kind: "PLANE".into(), ..Default::default() }],
                    face_occurrences: vec![CadStepFaceOccurrence::default()], ..Default::default() };
                asset.accuracy.analysis_error_bound_mm = bound;
                CadCollisionSubject::VendorModel { id, reference: "controlled vendor".into(), row_transform_f64: row, vendor_step: Some(Rc::new(asset)) }
            }
        }
    })
}

fn assess(report: QueryQualification) -> CadCollisionResult {
    let [left, right] = subjects(&report);
    let meshes = CadCollisionMeshPair { left_positions_mm_f64: left.mesh().unwrap().0.to_vec(),
        left_triangle_face_occurrence_indices: left.mesh().unwrap().1.to_vec(),
        right_positions_mm_f64: right.mesh().unwrap().0.to_vec(), right_triangle_face_occurrence_indices: right.mesh().unwrap().1.to_vec() };
    let validated = validate_query_qualification(report.clone(), &report.binding, &report.limits, &meshes,
        Some([left.source_qualification().unwrap(), right.source_qualification().unwrap()])).unwrap();
    let observation: CadCollisionObservation = serde_json::from_value(fixture()["qualificationV2"]["activation"]["rawObservationExample"].clone()).unwrap();
    assess_subject_collision_observation(&left, &right, report.binding.clearance_threshold_mm,
        report.binding.requested_accuracy_mm, Some((&validated, &report.binding)), observation)
}

#[test]
fn qualified_default_plane_and_mixed_bands_keep_accuracy_separate_from_threshold() {
    let data = fixture();
    let activation = &data["qualificationV2"]["activation"];
    let plane: QueryQualification = serde_json::from_value(activation["generatedPlaneQuery"].clone()).unwrap();
    let result = assess(plane.clone());
    assert_eq!(result.assessment, CadCollisionAssessment::SupportedBand { lower_mm: 1., upper_mm: 1., relation: CadCollisionRelation::Clear });
    let mut mixed: QueryQualification = serde_json::from_value(activation["mixedVendorGeneratedQuery"].clone()).unwrap();
    assert!(matches!(assess(mixed.clone()).assessment, CadCollisionAssessment::SupportedBand { relation: CadCollisionRelation::Clear, .. }));
    mixed.binding.requested_accuracy_mm = Some(0.01);
    assert!(matches!(assess(mixed).assessment, CadCollisionAssessment::Unknown { refinement_requested: true, .. }));
    let mut threshold = plane;
    threshold.binding.clearance_threshold_mm = 1.;
    assert!(matches!(assess(threshold.clone()).assessment, CadCollisionAssessment::SupportedBand { relation: CadCollisionRelation::Clear, .. }));
    threshold.binding.clearance_threshold_mm = 1.0000000000000002;
    assert!(matches!(assess(threshold).assessment, CadCollisionAssessment::SupportedBand { relation: CadCollisionRelation::Violation, .. }));
}

#[test]
fn unknown_contact_retains_scalar_diagnostic_and_unknown_containment_only_blocks_clear() {
    let data = fixture();
    let mut report: QueryQualification = serde_json::from_value(data["qualificationV2"]["activation"]["generatedPlaneQuery"].clone()).unwrap();
    report.contact_region = ContactRegion::Unknown { reasons: vec!["contact-budget".into()] };
    let result = assess(report.clone());
    assert_eq!(result.source_interval_mm, Some([1., 1.]));
    assert!(matches!(result.assessment, CadCollisionAssessment::Unknown { .. }));
    report.contact_region = serde_json::from_value(data["qualificationV2"]["activation"]["generatedPlaneQuery"]["contactRegion"].clone()).unwrap();
    report.containment = Containment::Unknown { reasons: vec!["global-correspondence-unavailable".into()] };
    assert!(matches!(assess(report.clone()).assessment, CadCollisionAssessment::SupportedBand { relation: CadCollisionRelation::Uncertain, .. }));
    report.binding.clearance_threshold_mm = 2.;
    assert!(matches!(assess(report.clone()).assessment, CadCollisionAssessment::SupportedBand { relation: CadCollisionRelation::Violation, .. }));
    report.binding.clearance_threshold_mm = 0.;
    report.mesh_distance = MeshDistance::Bounded { lower_mm: 0., upper_mm: 0., witness_triangles: [2, 0] };
    assert!(matches!(assess(report).assessment, CadCollisionAssessment::SupportedBand { relation: CadCollisionRelation::Uncertain, .. }), "unsigned zero does not prove penetration");
}
