//! Sealed evidence over one actual final emission. Reports and descriptors are
//! read-only values; only preparation can create an evidence owner.
mod types;
use crate::{ExactModel, GeneratedAnalysisMesh, ImportedStep, StepFaceOccurrence, StepSourceFace};
use std::sync::atomic::{AtomicU64, Ordering};
pub use types::*;

pub(crate) type SourceBoundResult = Result<[f64; 3], Vec<String>>;

static NEXT_MESH: AtomicU64 = AtomicU64::new(1);

pub struct PreparedGeneratedFaceOccurrence {
    pub source_face_index: usize,
    pub solid_occurrence_index: usize,
}
pub struct PreparedGeneratedView<'a> {
    pub emission: &'a GeneratedAnalysisMesh,
    pub face_occurrences: &'a [PreparedGeneratedFaceOccurrence],
}
pub struct PreparedVendorView<'a> {
    pub positions_mm_f64: &'a [[f64; 3]],
    pub triangle_face_occurrence_indices: &'a [usize],
    pub source_faces: &'a [StepSourceFace],
    pub face_occurrences: &'a [StepFaceOccurrence],
}
pub enum PreparedAnalysisView<'a> {
    Generated(PreparedGeneratedView<'a>),
    Vendor(PreparedVendorView<'a>),
}
enum Emission {
    Generated {
        mesh: GeneratedAnalysisMesh,
        occurrences: Vec<PreparedGeneratedFaceOccurrence>,
    },
    Vendor {
        positions: Vec<[f64; 3]>,
        tags: Vec<usize>,
        faces: Vec<StepSourceFace>,
        occurrences: Vec<StepFaceOccurrence>,
    },
}
/// Owns immutable final arrays, catalogues, settings and the result of source proof.
/// No caller-authored report can create this object.
pub struct PreparedAnalysisMesh {
    emission: Emission,
    descriptor: AnalysisMesh,
    source: SourceQualification,
}
impl PreparedAnalysisMesh {
    pub fn export_view(&self) -> PreparedAnalysisView<'_> {
        match &self.emission {
            Emission::Generated { mesh, occurrences } => {
                PreparedAnalysisView::Generated(PreparedGeneratedView {
                    emission: mesh,
                    face_occurrences: occurrences,
                })
            }
            Emission::Vendor {
                positions,
                tags,
                faces,
                occurrences,
            } => PreparedAnalysisView::Vendor(PreparedVendorView {
                positions_mm_f64: positions,
                triangle_face_occurrence_indices: tags,
                source_faces: faces,
                face_occurrences: occurrences,
            }),
        }
    }
    pub fn analysis_mesh(&self) -> &AnalysisMesh {
        &self.descriptor
    }
    pub fn source_qualification(&self) -> &SourceQualification {
        &self.source
    }
    pub fn retained_storage_bytes(&self) -> Option<usize> {
        fn vector<T>(v: &Vec<T>) -> Option<usize> {
            v.capacity().checked_mul(std::mem::size_of::<T>())
        }
        let mut bytes = std::mem::size_of::<Self>();
        let mut add = |n: usize| -> Option<()> {
            bytes = bytes.checked_add(n)?;
            Some(())
        };
        match &self.emission {
            Emission::Generated { mesh, occurrences } => {
                add(vector(&mesh.mesh.positions)?)?;
                add(vector(&mesh.mesh.normals)?)?;
                add(vector(&mesh.positions_mm_f64)?)?;
                add(vector(&mesh.triangle_face_occurrence_indices)?)?;
                add(vector(&mesh.source_faces)?)?;
                add(vector(occurrences)?)?;
                for face in &mesh.source_faces {
                    add(face.support_kind.capacity())?;
                }
            }
            Emission::Vendor {
                positions,
                tags,
                faces,
                occurrences,
            } => {
                add(vector(positions)?)?;
                add(vector(tags)?)?;
                add(vector(faces)?)?;
                add(vector(occurrences)?)?;
                for face in faces {
                    add(face.support_kind.capacity())?;
                }
            }
        }
        for text in [
            &self.descriptor.version,
            &self.descriptor.mesh_identity,
            &self.descriptor.source_proof,
        ] {
            add(text.capacity())?;
        }
        match &self.source {
            SourceQualification::Unknown { reasons } => {
                add(vector(reasons)?)?;
                for text in reasons {
                    add(text.capacity())?;
                }
            }
            SourceQualification::Bounded {
                class,
                mesh_identity,
                accounting,
                ..
            } => {
                for text in [class, mesh_identity, accounting] {
                    add(text.capacity())?;
                }
            }
        }
        // Exact arithmetic/AST/trace allocations are temporary and dropped before sealing.
        Some(bytes)
    }
}
fn descriptor(settings: MeshSettings, generated: bool) -> Result<AnalysisMesh, String> {
    let token = NEXT_MESH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
        .map_err(|_| "analysis identity exhausted")?;
    Ok(AnalysisMesh {
        version: "final-f64-v1".into(),
        mesh_identity: format!("final-f64-v1/{token}"),
        source_proof: if generated {
            "analytic-generated-v1"
        } else {
            "planar-final-v2"
        }
        .into(),
        settings,
        source_limits: if generated {
            SourceLimits::generated()
        } else {
            SourceLimits::planar()
        },
    })
}
fn source_report(
    bound: Result<[f64; 3], Vec<String>>,
    descriptor: &AnalysisMesh,
    generated: bool,
) -> SourceQualification {
    match bound {
        Ok([direct, precision, total]) => SourceQualification::Bounded {
            class: if generated {
                "analytic-generated"
            } else {
                "complete-native-line-plane"
            }
            .into(),
            version: if generated { 1 } else { 2 },
            direct_bound_mm: direct,
            source_precision_mm: precision,
            total_bound_mm: total,
            mesh_identity: descriptor.mesh_identity.clone(),
            accounting: "direct-includes-conversion-repair-trims-evaluation".into(),
        },
        Err(reasons) => SourceQualification::Unknown { reasons },
    }
}
pub fn prepare_generated_analysis(
    model: &ExactModel,
    target: Option<f64>,
) -> Result<PreparedAnalysisMesh, String> {
    let (mesh, bound) = crate::profile::prepare_qualified_emission(model, target)?;
    let descriptor = descriptor(
        MeshSettings {
            target_deflection_mm: target,
            selected_deflection_mm: mesh.deflection_mm,
            angular_rad: mesh.angular_rad,
            preview_repair_budget_mm: None,
        },
        true,
    )?;
    let source = source_report(bound, &descriptor, true);
    let occurrences = mesh
        .source_faces
        .iter()
        .enumerate()
        .map(|(index, face)| PreparedGeneratedFaceOccurrence {
            source_face_index: index,
            solid_occurrence_index: face.solid_index,
        })
        .collect();
    Ok(PreparedAnalysisMesh {
        emission: Emission::Generated { mesh, occurrences },
        descriptor,
        source,
    })
}
pub fn prepare_vendor_analysis(model: &ImportedStep) -> Result<PreparedAnalysisMesh, String> {
    let settings = crate::step::analysis_import_settings(model);
    let descriptor = descriptor(settings, false)?;
    let bound = crate::collision_qualification::asset_bound(model);
    let source = source_report(bound, &descriptor, false);
    let emission = Emission::Vendor {
        positions: model.preview_positions_f64().to_vec(),
        tags: model.triangle_faces().to_vec(),
        faces: model.source_faces().to_vec(),
        occurrences: model.face_occurrences().to_vec(),
    };
    Ok(PreparedAnalysisMesh {
        emission,
        descriptor,
        source,
    })
}
pub fn verify_analysis_pair(
    left: &PreparedAnalysisMesh,
    right: &PreparedAnalysisMesh,
    rows: [[f64; 16]; 2],
    limits: Limits,
) -> QueryEvidence {
    crate::collision_qualification::prepared_pair([left, right], rows, limits)
}

#[cfg(test)]
mod tests {
    use super::*;
    const IDENTITY: [f64; 16] = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap()
    }
    fn plate() -> PreparedAnalysisMesh {
        let body = serde_json::from_value(fixture()["preparedBody"].clone()).unwrap();
        prepare_generated_analysis(&crate::build_case(&body).unwrap(), None).unwrap()
    }
    fn assert_cap_refusal(owner: &PreparedAnalysisMesh, limits: Limits, reason: &str) {
        let mut raised = IDENTITY;
        raised[14] = 3.;
        let report = verify_analysis_pair(owner, owner, [IDENTITY, raised], limits);
        assert!(report.world_positions().iter().all(|view| !view.is_empty()));
        assert!(
            matches!(&report.mesh_distance, MeshDistance::Unknown { .. }),
            "{:?}",
            report.mesh_distance
        );
        assert!(
            report
                .source_bounds
                .iter()
                .all(|bound| matches!(bound, WorldSourceBound::Unknown { .. })),
            "{:?}",
            report.source_bounds
        );
        assert!(
            matches!(&report.containment, Containment::Unknown { .. }),
            "{:?}",
            report.containment
        );
        assert!(
            matches!(&report.mesh_distance, MeshDistance::Unknown { reasons } if reasons == &[reason])
        );
        assert!(report.source_bounds.iter().all(
            |bound| matches!(bound, WorldSourceBound::Unknown { reasons } if reasons == &[reason])
        ));
        assert!(
            matches!(&report.contact_region, ContactRegion::Unknown { reasons } if reasons == &[reason])
        );
        assert!(
            matches!(&report.containment, Containment::Unknown { reasons } if reasons == &[reason])
        );
        assert_eq!(report.query_counters(), [0, 0]);
    }
    #[test]
    fn cap_preflight_refuses_triangle_budget_before_exact_work() {
        let limits = Limits {
            max_triangles: 1,
            ..Limits::default()
        };
        assert_cap_refusal(&plate(), limits, "triangle cap");
        let vendor = crate::import_step(
            include_bytes!("../tests/fixtures/touching-unit-boxes.step").to_vec(),
        )
        .unwrap();
        let unsupported = prepare_vendor_analysis(&vendor).unwrap();
        assert!(matches!(
            unsupported.source_qualification(),
            SourceQualification::Unknown { .. }
        ));
        assert_cap_refusal(&unsupported, limits, "triangle cap");
    }
    #[test]
    fn cap_preflight_refuses_exponent_budget_before_exact_work() {
        assert_cap_refusal(
            &plate(),
            Limits {
                maximum_exponent: 0,
                ..Limits::default()
            },
            "exponent cap",
        );
    }

    #[test]
    fn real_preparations_bind_shared_activation_plate_vendor_and_mixed_pairs() {
        let plate = plate();
        assert_eq!(plate.source_qualification().total_bound_mm(), Some(0.));
        let PreparedAnalysisView::Generated(view) = plate.export_view() else {
            unreachable!()
        };
        assert_eq!(
            view.face_occurrences.len(),
            view.emission.source_faces.len()
        );
        assert!(plate.retained_storage_bytes().unwrap() > view.emission.positions_mm_f64.len() * 8);
        let mut raised = IDENTITY;
        raised[14] = 3.;
        let pair = verify_analysis_pair(&plate, &plate, [IDENTITY, raised], Limits::default());
        match pair.mesh_distance {
            MeshDistance::Bounded {
                lower_mm, upper_mm, ..
            } => assert!(lower_mm <= 1. && upper_mm >= 1. && upper_mm - lower_mm <= 1e-10),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            pair.containment,
            Containment::Slab { axis: 2, .. }
        ));
        assert!(matches!(pair.contact_region, ContactRegion::Boxes { .. }));
        let vendor = crate::import_step(
            include_bytes!(
                "../../../ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step"
            )
            .to_vec(),
        )
        .unwrap();
        let prepared = prepare_vendor_analysis(&vendor).unwrap();
        let PreparedAnalysisView::Vendor(view) = prepared.export_view() else {
            unreachable!()
        };
        assert_eq!(view.source_faces.len(), 30);
        assert_eq!(
            view.positions_mm_f64
                .iter()
                .flatten()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            vendor
                .preview_positions_f64()
                .iter()
                .flatten()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(view.face_occurrences, vendor.face_occurrences());
        assert_eq!(prepared.analysis_mesh().settings.target_deflection_mm, None);
        let Some(total) = prepared.source_qualification().total_bound_mm() else {
            panic!("{:?}", prepared.source_qualification())
        };
        assert!((0.01..0.01000001).contains(&total));
        let delta =
            &fixture()["qualificationV2"]["sourcePlacement"]["privateSuppliedWorldTranslationMm"];
        let mut translated = IDENTITY;
        for i in 0..3 {
            translated[12 + i] = delta[i].as_f64().unwrap();
        }
        let pair = verify_analysis_pair(
            &prepared,
            &prepared,
            [translated, IDENTITY],
            Limits::default(),
        );
        assert!(
            matches!(pair.mesh_distance, MeshDistance::Bounded { .. }),
            "{:?}",
            pair.mesh_distance
        );
        assert!(
            matches!(pair.containment, Containment::Slab { .. }),
            "{:?}",
            pair.containment
        );
        let limits = Limits {
            max_exact_pairs: 1,
            ..Limits::default()
        };
        let refused = verify_analysis_pair(&prepared, &prepared, [translated, IDENTITY], limits);
        assert!(
            matches!(refused.mesh_distance, MeshDistance::Unknown { .. }),
            "{:?}",
            refused.mesh_distance
        );
        let mixed = verify_analysis_pair(&prepared, &plate, [IDENTITY, raised], Limits::default());
        assert!(matches!(
            mixed.source_bounds,
            [
                WorldSourceBound::Bounded { .. },
                WorldSourceBound::Bounded { .. }
            ]
        ));
    }
}
