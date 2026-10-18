//! Controlled consumer reports from the shared seam; no opaque handle or geometry proof is minted.
use crate::model_delivery::*;
use boardstudio_web_host::{cad_jobs::*, cad_qualification::*};
use std::rc::Rc;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap()
}

fn subjects(report: &QueryQualification) -> [CadCollisionSubject; 2] {
    let data = fixture();
    let activation = &data["qualificationV2"]["activation"];
    std::array::from_fn(|i| {
        let reference = [
            &report.binding.left_analysis,
            &report.binding.right_analysis,
        ][i];
        let descriptor = [&report.binding.left_mesh, &report.binding.right_mesh][i];
        let row = [
            report.binding.left_row_transform_f64,
            report.binding.right_row_transform_f64,
        ][i];
        let id = [
            &report.binding.left_model_id,
            &report.binding.right_model_id,
        ][i]
            .clone();
        let name = if matches!(reference.source, Source::VendorStep { .. }) {
            "vendor"
        } else if matches!(&reference.source, Source::GeneratedBody { body_id, .. } if body_id == "contract-r1-drilled-plate")
        {
            if descriptor.settings.target_deflection_mm.is_some() {
                "cylinderRefined"
            } else {
                "cylinderDefault"
            }
        } else {
            "plane"
        };
        let source: SourceQualification =
            serde_json::from_value(activation["sourceReports"][name].clone()).unwrap();
        let bound = match source {
            SourceQualification::Bounded { total_bound_mm, .. } => Some(total_bound_mm),
            _ => None,
        };
        match &reference.source {
            Source::GeneratedBody {
                body_id,
                revision,
                geometry_identity,
            } => {
                let mut payload = data["directPayload"].clone();
                payload["geometryIdentity"] = geometry_identity.clone().into();
                let mut asset: CadGeneratedAnalysis = serde_json::from_value(payload).unwrap();
                asset.body_id = body_id.clone();
                asset.revision = *revision;
                asset.analysis_ref = Some(reference.clone());
                asset.analysis_mesh = Some(descriptor.clone());
                asset.deflection_mm = descriptor.settings.selected_deflection_mm;
                asset.source_qualification = Some(source);
                asset.analysis_error_bound_mm = bound;
                CadCollisionSubject::GeneratedCaseBody {
                    id,
                    name: body_id.clone(),
                    row_transform_f64: row,
                    analysis: Some(Rc::new(asset)),
                }
            }
            Source::VendorStep {
                source_sha256,
                import_options,
            } => {
                let mut asset = CadVendorStep {
                    source_sha256: source_sha256.clone(),
                    import_options: CadStepImportOptions {
                        preview_repair_budget_mm: import_options.preview_repair_budget_mm,
                        target_deflection_mm: import_options.target_deflection_mm,
                    },
                    analysis_ref: Some(reference.clone()),
                    analysis_mesh: Some(descriptor.clone()),
                    source_qualification: Some(source),
                    surface_positions_mm_f64: [0., 0., 1.35, 1., 0., 1.35, 0., 1., 1.35].repeat(3),
                    triangle_face_occurrence_indices: vec![0; 3],
                    source_faces: vec![CadStepSourceFace {
                        support_kind: "PLANE".into(),
                        ..Default::default()
                    }],
                    face_occurrences: vec![CadStepFaceOccurrence::default()],
                    ..Default::default()
                };
                asset.accuracy.analysis_error_bound_mm = bound;
                CadCollisionSubject::VendorModel {
                    id,
                    reference: "controlled vendor".into(),
                    row_transform_f64: row,
                    vendor_step: Some(Rc::new(asset)),
                }
            }
        }
    })
}

fn assess(report: QueryQualification) -> CadCollisionResult {
    let [left, right] = subjects(&report);
    let meshes = CadCollisionMeshPair {
        left_positions_mm_f64: left.mesh().unwrap().0.to_vec(),
        left_triangle_face_occurrence_indices: left.mesh().unwrap().1.to_vec(),
        right_positions_mm_f64: right.mesh().unwrap().0.to_vec(),
        right_triangle_face_occurrence_indices: right.mesh().unwrap().1.to_vec(),
    };
    let validated = validate_query_qualification(
        report.clone(),
        &report.binding,
        &report.limits,
        &meshes,
        Some([
            left.source_qualification().unwrap(),
            right.source_qualification().unwrap(),
        ]),
    )
    .unwrap();
    let mut observation: CadCollisionObservation = serde_json::from_value(
        fixture()["qualificationV2"]["activation"]["rawObservationExample"].clone(),
    )
    .unwrap();
    if let CadCollisionObservation::SurfaceSeparated { closest_points, .. } = &mut observation {
        for (provenance, subject) in [
            (&mut closest_points.triangles.left, &left),
            (&mut closest_points.triangles.right, &right),
        ] {
            provenance.face_occurrence_index =
                subject.mesh().unwrap().1[provenance.triangle_index as usize];
        }
    }
    if matches!(
        report.mesh_distance,
        MeshDistance::Bounded {
            lower_mm: 0.,
            upper_mm: 0.,
            ..
        }
    ) && let CadCollisionObservation::SurfaceSeparated { closest_points, .. } = observation
    {
        observation = CadCollisionObservation::SurfaceIntersecting {
            intersection_triangles: closest_points.triangles,
            contact_witness: None,
        };
    }
    assess_subject_collision_observation(
        &left,
        &right,
        report.binding.clearance_threshold_mm,
        report.binding.requested_accuracy_mm,
        Some((&validated, &report.binding)),
        observation,
    )
}

#[test]
fn qualified_default_plane_and_mixed_bands_keep_accuracy_separate_from_threshold() {
    let data = fixture();
    let activation = &data["qualificationV2"]["activation"];
    let plane: QueryQualification =
        serde_json::from_value(activation["generatedPlaneQuery"].clone()).unwrap();
    let result = assess(plane.clone());
    assert_eq!(
        result.assessment,
        CadCollisionAssessment::SupportedBand {
            lower_mm: 1.,
            upper_mm: 1.,
            relation: CadCollisionRelation::Clear
        }
    );
    let mut mixed: QueryQualification =
        serde_json::from_value(activation["mixedVendorGeneratedQuery"].clone()).unwrap();
    assert!(matches!(
        assess(mixed.clone()).assessment,
        CadCollisionAssessment::SupportedBand {
            relation: CadCollisionRelation::Clear,
            ..
        }
    ));
    mixed.binding.requested_accuracy_mm = Some(0.01);
    assert!(matches!(
        assess(mixed).assessment,
        CadCollisionAssessment::Unknown {
            refinement_requested: true,
            ..
        }
    ));
    let mut threshold = plane;
    threshold.binding.clearance_threshold_mm = 1.;
    assert!(matches!(
        assess(threshold.clone()).assessment,
        CadCollisionAssessment::SupportedBand {
            relation: CadCollisionRelation::Clear,
            ..
        }
    ));
    threshold.binding.clearance_threshold_mm = 1.0000000000000002;
    assert!(matches!(
        assess(threshold).assessment,
        CadCollisionAssessment::SupportedBand {
            relation: CadCollisionRelation::Violation,
            ..
        }
    ));
}

#[test]
fn unknown_contact_retains_scalar_diagnostic_and_unknown_containment_only_blocks_clear() {
    let data = fixture();
    let mut report: QueryQualification = serde_json::from_value(
        data["qualificationV2"]["activation"]["generatedPlaneQuery"].clone(),
    )
    .unwrap();
    report.contact_region = ContactRegion::Unknown {
        reasons: vec!["contact-budget".into()],
    };
    let result = assess(report.clone());
    assert_eq!(result.source_interval_mm, Some([1., 1.]));
    assert!(matches!(
        result.assessment,
        CadCollisionAssessment::Unknown { .. }
    ));
    report.contact_region = serde_json::from_value(
        data["qualificationV2"]["activation"]["generatedPlaneQuery"]["contactRegion"].clone(),
    )
    .unwrap();
    report.containment = Containment::Unknown {
        reasons: vec!["global-correspondence-unavailable".into()],
    };
    assert!(matches!(
        assess(report.clone()).assessment,
        CadCollisionAssessment::SupportedBand {
            relation: CadCollisionRelation::Uncertain,
            ..
        }
    ));
    report.binding.clearance_threshold_mm = 2.;
    assert!(matches!(
        assess(report.clone()).assessment,
        CadCollisionAssessment::SupportedBand {
            relation: CadCollisionRelation::Violation,
            ..
        }
    ));
    report.binding.clearance_threshold_mm = 0.;
    report.binding.right_row_transform_f64[14] = 2.;
    if let ContactRegion::Boxes { pairs, .. } = &mut report.contact_region {
        pairs[0].right_mm[0][2] = 2.;
        pairs[0].right_mm[1][2] = 4.;
    }
    report.mesh_distance = MeshDistance::Bounded {
        lower_mm: 0.,
        upper_mm: 0.,
        witness_triangles: [2, 0],
    };
    assert!(
        matches!(
            assess(report).assessment,
            CadCollisionAssessment::SupportedBand {
                relation: CadCollisionRelation::Uncertain,
                ..
            }
        ),
        "unsigned zero does not prove penetration"
    );
}

#[test]
fn shared_cylinder_budget_controls_do_not_treat_selected_deflection_as_a_certificate() {
    let data = fixture();
    let activation = &data["qualificationV2"]["activation"];
    for case in activation["cylinderBudgetCases"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .take(2)
    {
        let name = case["sourceReport"].as_str().unwrap();
        let source: SourceQualification =
            serde_json::from_value(activation["sourceReports"][name].clone()).unwrap();
        let SourceQualification::Bounded {
            direct_bound_mm,
            source_precision_mm,
            total_bound_mm,
            mesh_identity,
            ..
        } = source
        else {
            panic!("shared budget source report")
        };
        let mut report: QueryQualification =
            serde_json::from_value(activation["generatedPlaneQuery"].clone()).unwrap();
        report.binding.left_analysis =
            serde_json::from_value(activation["workerReferences"][name].clone()).unwrap();
        report.binding.left_mesh =
            serde_json::from_value(activation["descriptors"][name].clone()).unwrap();
        report.binding.left_mesh_identity = mesh_identity.clone();
        report.binding.requested_accuracy_mm = case["requestedAccuracyMm"].as_f64();
        report.source_bounds[0] = WorldSourceBound::Bounded {
            mesh_identity,
            method: "scaled-local-plus-row-rounding".into(),
            norm_upper: 1.,
            direct_bound_mm,
            source_precision_mm,
            transform_rounding_mm: 0.,
            total_bound_mm,
        };
        if let ContactRegion::Boxes {
            source_inflation_mm,
            pairs,
        } = &mut report.contact_region
        {
            source_inflation_mm[0] = total_bound_mm;
            for axis in 0..3 {
                pairs[0].left_mm[0][axis] -= total_bound_mm;
                pairs[0].left_mm[1][axis] += total_bound_mm;
            }
        }
        if let Containment::RuledOut {
            evidence: ContainmentExclusion::Slab { left_upper_mm, .. },
        } = &mut report.containment
        {
            *left_upper_mm += total_bound_mm;
        }
        let result = assess(report);
        if case["expectedAssessment"] == "supported-band" {
            assert!(matches!(
                result.assessment,
                CadCollisionAssessment::SupportedBand {
                    relation: CadCollisionRelation::Clear,
                    ..
                }
            ));
        } else {
            assert!(matches!(
                result.assessment,
                CadCollisionAssessment::Unknown {
                    refinement_requested: true,
                    ..
                }
            ));
        }
    }
    // The b03 cap recipe is private. Only its independently authored budget
    // oracle is consumed here; it is never made into a PreparedCaseIR or handle.
    let adversarial = activation["cylinderBudgetCases"]["cases"][3]["minimumDirectBoundMm"]
        .as_f64()
        .unwrap();
    assert!(adversarial > 0.48);
    assert!(pair_error_budget(1., 1., adversarial, 0.) > 0.01);
    let unknown: SourceQualification = serde_json::from_value(
        activation["cylinderBudgetCases"]["cases"][2]["sourceQualification"].clone(),
    )
    .unwrap();
    assert!(
        matches!(unknown, SourceQualification::Unknown { .. }),
        "a finer target has no source certificate"
    );
}
