//! CAD collision subjects, findings and assessment of collision observations.
use super::*;
use boardstudio_model::model::Side;

pub const TIGHT_COLLISION_ACCURACY_MM: f64 = 0.01;
pub const IDENTITY_ROW_TRANSFORM_F64: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

#[derive(Clone, Debug, PartialEq)]
pub enum CadCollisionSubject {
    VendorModel {
        id: String,
        reference: String,
        row_transform_f64: [f64; 16],
        vendor_step: Option<Rc<boardstudio_web_host::cad_jobs::CadVendorStep>>,
    },
    GeneratedCaseBody {
        id: String,
        name: String,
        row_transform_f64: [f64; 16],
        analysis: Option<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>>,
    },
}

impl CadCollisionSubject {
    pub fn source_qualification(
        &self,
    ) -> Option<&boardstudio_web_host::cad_qualification::SourceQualification> {
        match self {
            Self::VendorModel {
                vendor_step: Some(asset),
                ..
            } => asset.source_qualification.as_ref(),
            Self::GeneratedCaseBody {
                analysis: Some(asset),
                ..
            } => asset.source_qualification.as_ref(),
            _ => None,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::VendorModel { id, .. } | Self::GeneratedCaseBody { id, .. } => id,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::VendorModel { reference, .. } => reference,
            Self::GeneratedCaseBody { name, .. } => name,
        }
    }

    pub fn operand(&self) -> Result<boardstudio_web_host::cad_jobs::CadCollisionOperand, String> {
        use boardstudio_web_host::cad_jobs::{CadCollisionOperand, CadStepImportOptions};
        match self {
            Self::VendorModel {
                id,
                row_transform_f64,
                vendor_step: Some(asset),
                ..
            } => Ok(CadCollisionOperand {
                analysis_ref: asset.analysis_ref.clone(),
                analysis_mesh: asset.analysis_mesh.clone(),
                model_id: id.clone(),
                source_sha256: Some(asset.source_sha256.clone()),
                generated_geometry_identity: None,
                import_options: asset.import_options,
                row_transform_f64: *row_transform_f64,
            }),
            Self::GeneratedCaseBody {
                id,
                row_transform_f64,
                analysis: Some(asset),
                ..
            } => Ok(CadCollisionOperand {
                analysis_ref: asset.analysis_ref.clone(),
                analysis_mesh: asset.analysis_mesh.clone(),
                model_id: id.clone(),
                source_sha256: None,
                generated_geometry_identity: Some(asset.geometry_identity.clone()),
                import_options: CadStepImportOptions::default(),
                row_transform_f64: *row_transform_f64,
            }),
            _ => Err(format!("{} has no source analysis mesh", self.label())),
        }
    }

    pub fn mesh(&self) -> Option<(&[f64], &[u32])> {
        match self {
            Self::VendorModel {
                vendor_step: Some(asset),
                ..
            } => Some((
                &asset.surface_positions_mm_f64,
                &asset.triangle_face_occurrence_indices,
            )),
            Self::GeneratedCaseBody {
                analysis: Some(asset),
                ..
            } => Some((
                &asset.surface_positions_mm_f64,
                &asset.triangle_face_occurrence_indices,
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CadCollisionPairFinding {
    pub left_id: String,
    pub left_label: String,
    pub right_id: String,
    pub right_label: String,
    pub result: CadCollisionResult,
}

impl CadCollisionPairFinding {
    pub fn qualification_details(&self) -> Vec<String> {
        use boardstudio_web_host::cad_qualification::{
            ContactRegion, Containment, ContainmentExclusion, WorldSourceBound,
        };
        let mut details = Vec::new();
        if let Some([lower, upper]) = self.result.source_interval_mm {
            details.push(format!("Source surface distance: [{lower}, {upper}] mm. This interval remains diagnostic when the finding is uncertain."));
        }
        if let Some(report) = &self.result.query_qualification {
            for (side, bound) in ["Left", "Right"].into_iter().zip(&report.source_bounds) {
                match bound {
                    WorldSourceBound::Bounded { total_bound_mm, source_precision_mm, .. } => details.push(format!("{side} source allowance: {total_bound_mm} mm, including declared source precision {source_precision_mm} mm once.")),
                    WorldSourceBound::Unknown { reasons } => details.push(format!("{side} source allowance unknown: {}", reasons.join("; "))),
                }
            }
            match &report.contact_region {
                ContactRegion::Boxes { pairs, .. } => {
                    details.push(format!(
                        "Contact region: {} conservative box pairs.",
                        pairs.len()
                    ));
                    for pair in pairs {
                        details.push(format!(
                            "Left box {:?} to {:?} mm; right box {:?} to {:?} mm.",
                            pair.left_mm[0], pair.left_mm[1], pair.right_mm[0], pair.right_mm[1]
                        ));
                    }
                }
                ContactRegion::Unknown { reasons } => {
                    details.push(format!("Contact region unknown: {}", reasons.join("; ")))
                }
            }
            match &report.containment {
                Containment::Unknown { reasons } => {
                    details.push(format!("Containment unknown: {}", reasons.join("; ")))
                }
                Containment::RuledOut {
                    evidence: ContainmentExclusion::Slab { axis, .. },
                } => details.push(format!(
                    "Containment ruled out by separated source slabs on axis {axis}."
                )),
                Containment::RuledOut { .. } => {
                    details.push("Containment ruled out by qualified global correspondence.".into())
                }
                Containment::ContainedLeft { .. } | Containment::ContainedRight { .. } => details
                    .push(
                        "Material overlap established; penetration depth remains unqualified."
                            .into(),
                    ),
            }
        }
        if self.result.observation.is_some() {
            details.push("The raw mesh witness does not certify a unique source contact point, normal or penetration depth.".into());
        }
        details
    }

    pub fn provenance_details(&self) -> [String; 2] {
        let describe = |id: &str, label: &str, provenance: &Option<CadSubjectFaceProvenance>| {
            let identity = match provenance {
                Some(CadSubjectFaceProvenance::Vendor(face)) => format!(
                    "STEP representation #{}; source solid #{}; shell {}; solid occurrence {}; face occurrence {}; source face #{}; support #{} ({}); triangle {}",
                    face.representation_id,
                    face.solid_id,
                    face.shell_index,
                    face.solid_occurrence_index,
                    face.face_occurrence_index,
                    face.face_id,
                    face.support_id,
                    face.support_kind,
                    face.triangle_index
                ),
                Some(CadSubjectFaceProvenance::Generated {
                    body_id,
                    triangle_index,
                    face_occurrence_index,
                    solid_occurrence_index,
                    source,
                }) => format!(
                    "generated body {body_id}; source solid index {}; shell index {}; solid occurrence {solid_occurrence_index}; face occurrence {face_occurrence_index}; source face index {}; support {}; triangle {triangle_index}",
                    source.solid_index, source.shell_index, source.face_index, source.support_kind
                ),
                None => "source face/occurrence identity unresolved".to_owned(),
            };
            format!("{label} [{id}]: {identity}")
        };
        [
            describe(
                &self.left_id,
                &self.left_label,
                &self.result.left_provenance,
            ),
            describe(
                &self.right_id,
                &self.right_label,
                &self.result.right_provenance,
            ),
        ]
    }

    pub fn summary(&self, clearance_threshold_mm: f64) -> String {
        use boardstudio_web_host::cad_jobs::CadCollisionObservation;

        let pair_label = format!(
            "{} [{}] ↔ {} [{}]",
            self.left_label, self.left_id, self.right_label, self.right_id
        );
        let witness = match &self.result.observation {
            Some(CadCollisionObservation::SurfaceIntersecting {
                contact_witness: Some(witness),
                ..
            }) => format!(
                "surface intersection near ({:.2}, {:.2}, {:.2}) mm",
                witness.point1_mm[0], witness.point1_mm[1], witness.point1_mm[2]
            ),
            Some(CadCollisionObservation::SurfaceIntersecting { .. }) => {
                "surface intersection observed".to_owned()
            }
            Some(CadCollisionObservation::SurfaceSeparated {
                surface_distance_mm,
                closest_points,
            }) => format!(
                "surface separation {:.3} mm between ({:.2}, {:.2}, {:.2}) and ({:.2}, {:.2}, {:.2}) mm",
                surface_distance_mm,
                closest_points.point1_mm[0],
                closest_points.point1_mm[1],
                closest_points.point1_mm[2],
                closest_points.point2_mm[0],
                closest_points.point2_mm[1],
                closest_points.point2_mm[2]
            ),
            Some(CadCollisionObservation::Uncertain { reason, .. }) => {
                format!("uncertain observation ({reason})")
            }
            None => "no mesh observation".to_owned(),
        };
        match &self.result.assessment {
            CadCollisionAssessment::SupportedBand {
                lower_mm,
                upper_mm,
                relation,
            } => {
                let status = match relation {
                    CadCollisionRelation::Violation => "Violation",
                    CadCollisionRelation::Clear => "Clear",
                    CadCollisionRelation::Uncertain => "Uncertain",
                };
                format!(
                    "{pair_label}: {status} [{lower_mm:.3}, {upper_mm:.3}] mm vs {clearance_threshold_mm:.3} mm ({witness})"
                )
            }
            CadCollisionAssessment::Unknown {
                reasons,
                refinement_requested,
            } => {
                let refine = if *refinement_requested {
                    "; requested accuracy remains unqualified"
                } else {
                    ""
                };
                format!(
                    "{pair_label}: Uncertain ({witness}; {}{refine})",
                    reasons.join(", ")
                )
            }
        }
    }
}

pub fn physical_pcb_model_row_transform_f64(
    model: &PcbModel,
    board_thickness: f64,
    reference: Option<&BoardReference>,
    has_mechanical_stack: bool,
) -> Result<[f64; 16], String> {
    let mul = |a: [f64; 16], b: [f64; 16]| -> [f64; 16] {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = a[row] * b[col * 4]
                    + a[4 + row] * b[col * 4 + 1]
                    + a[8 + row] * b[col * 4 + 2]
                    + a[12 + row] * b[col * 4 + 3];
            }
        }
        out
    };
    let translate = |x: f64, y: f64, z: f64| -> [f64; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
        ]
    };
    let scale = |x: f64, y: f64, z: f64| -> [f64; 16] {
        [
            x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };
    let rot_x = |rad: f64| -> [f64; 16] {
        let (s, c) = rad.sin_cos();
        [
            1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };
    let rot_y = |rad: f64| -> [f64; 16] {
        let (s, c) = rad.sin_cos();
        [
            c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };
    let rot_z = |rad: f64| -> [f64; 16] {
        let (s, c) = rad.sin_cos();
        [
            c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };

    let base_z_delta = -if has_mechanical_stack {
        board_thickness
    } else {
        0.0
    };
    let reference_transform = reference
        .filter(|item| item.enabled)
        .map(|item| {
            mul(
                translate(item.pose.at.x, item.pose.at.y, item.elevation),
                rot_z(item.pose.rotation.to_radians()),
            )
        })
        .unwrap_or(IDENTITY_ROW_TRANSFORM_F64);
    let pcb_transform = mul(reference_transform, translate(0.0, 0.0, base_z_delta));

    let back = model.side == Side::Back;
    let side = if back {
        mul(rot_y(std::f64::consts::PI), rot_z(std::f64::consts::PI))
    } else {
        IDENTITY_ROW_TRANSFORM_F64
    };
    let model_transform = [
        translate(
            model.pose.at.x,
            model.pose.at.y,
            if back { 0.0 } else { board_thickness },
        ),
        rot_z(model.pose.rotation.to_radians()),
        side,
        translate(model.offset.x, model.offset.y, model.offset.z),
        rot_z(-model.rotation.z.to_radians()),
        rot_y(-model.rotation.y.to_radians()),
        rot_x(-model.rotation.x.to_radians()),
        scale(model.scale.x, model.scale.y, model.scale.z),
    ]
    .into_iter()
    .fold(IDENTITY_ROW_TRANSFORM_F64, mul);

    let matrix = mul(pcb_transform, model_transform);
    boardstudio_web_host::cad_jobs::validate_affine_row_transform(&matrix)
        .map_err(|error| error.to_string())?;
    Ok(matrix)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadCollisionRelation {
    Clear,
    Violation,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CadCollisionAssessment {
    SupportedBand {
        lower_mm: f64,
        upper_mm: f64,
        relation: CadCollisionRelation,
    },
    Unknown {
        reasons: Vec<String>,
        refinement_requested: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CadResolvedFaceProvenance {
    pub triangle_index: u32,
    pub face_occurrence_index: u32,
    pub solid_occurrence_index: u32,
    pub representation_id: String,
    pub solid_id: String,
    pub shell_index: u32,
    pub face_id: String,
    pub support_id: String,
    pub support_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CadSubjectFaceProvenance {
    Vendor(CadResolvedFaceProvenance),
    Generated {
        body_id: String,
        triangle_index: u32,
        face_occurrence_index: u32,
        solid_occurrence_index: u32,
        source: boardstudio_web_host::cad_jobs::CadGeneratedSourceFace,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CadCollisionResult {
    pub source_interval_mm: Option<[f64; 2]>,
    pub query_qualification: Option<boardstudio_web_host::cad_qualification::QueryQualification>,
    pub observation: Option<boardstudio_web_host::cad_jobs::CadCollisionObservation>,
    pub left_provenance: Option<CadSubjectFaceProvenance>,
    pub right_provenance: Option<CadSubjectFaceProvenance>,
    pub assessment: CadCollisionAssessment,
}

pub fn resolve_face_provenance(
    asset: &boardstudio_web_host::cad_jobs::CadVendorStep,
    provenance: &boardstudio_web_host::cad_jobs::CadTriangleProvenance,
) -> Option<CadResolvedFaceProvenance> {
    let expected = *asset
        .triangle_face_occurrence_indices
        .get(provenance.triangle_index as usize)?;
    if expected != provenance.face_occurrence_index {
        return None;
    }
    let occurrence = asset
        .face_occurrences
        .get(provenance.face_occurrence_index as usize)?;
    let source_face = asset
        .source_faces
        .get(occurrence.source_face_index as usize)?;
    Some(CadResolvedFaceProvenance {
        triangle_index: provenance.triangle_index,
        face_occurrence_index: provenance.face_occurrence_index,
        solid_occurrence_index: occurrence.solid_occurrence_index,
        representation_id: occurrence.representation_id.clone(),
        solid_id: source_face.solid_id.clone(),
        shell_index: source_face.shell_index,
        face_id: source_face.face_id.clone(),
        support_id: source_face.support_id.clone(),
        support_kind: source_face.support_kind.clone(),
    })
}

pub fn resolve_subject_face_provenance(
    subject: &CadCollisionSubject,
    provenance: &boardstudio_web_host::cad_jobs::CadTriangleProvenance,
) -> Option<CadSubjectFaceProvenance> {
    match subject {
        CadCollisionSubject::VendorModel { vendor_step, .. } => {
            resolve_face_provenance(vendor_step.as_deref()?, provenance)
                .map(CadSubjectFaceProvenance::Vendor)
        }
        CadCollisionSubject::GeneratedCaseBody { id, analysis, .. } => {
            let asset = analysis.as_ref()?;
            if *asset
                .triangle_face_occurrence_indices
                .get(provenance.triangle_index as usize)?
                != provenance.face_occurrence_index
            {
                return None;
            }
            let occurrence = asset
                .face_occurrences
                .get(provenance.face_occurrence_index as usize)?;
            Some(CadSubjectFaceProvenance::Generated {
                body_id: id.clone(),
                triangle_index: provenance.triangle_index,
                face_occurrence_index: provenance.face_occurrence_index,
                solid_occurrence_index: occurrence.solid_occurrence_index,
                source: asset
                    .source_faces
                    .get(occurrence.source_face_index as usize)?
                    .clone(),
            })
        }
    }
}

pub fn assess_subject_collision_observation(
    left: &CadCollisionSubject,
    right: &CadCollisionSubject,
    clearance_threshold_mm: f64,
    requested_accuracy_mm: Option<f64>,
    qualification: Option<(
        &boardstudio_web_host::cad_qualification::ValidatedQueryQualification,
        &boardstudio_web_host::cad_qualification::Binding,
    )>,
    observation: boardstudio_web_host::cad_jobs::CadCollisionObservation,
) -> CadCollisionResult {
    use boardstudio_web_host::cad_jobs::CadCollisionObservation;

    let pair = match &observation {
        CadCollisionObservation::SurfaceIntersecting {
            intersection_triangles,
            ..
        } => Some(intersection_triangles),
        CadCollisionObservation::SurfaceSeparated { closest_points, .. } => {
            Some(&closest_points.triangles)
        }
        CadCollisionObservation::Uncertain { .. } => None,
    };
    let left_provenance = pair.and_then(|p| resolve_subject_face_provenance(left, &p.left));
    let right_provenance = pair.and_then(|p| resolve_subject_face_provenance(right, &p.right));

    let mut reasons = Vec::new();
    if pair.is_some() && (left_provenance.is_none() || right_provenance.is_none()) {
        reasons.push("witness triangle provenance could not be resolved to source faces".into());
    }
    if let CadCollisionObservation::Uncertain { reason, .. } = &observation {
        reasons.push(format!("raw mesh observation is uncertain: {reason}"));
    }
    use boardstudio_web_host::cad_qualification::{
        ContactRegion, Containment, MeshDistance, WorldSourceBound, add_down, add_up,
    };
    let mut source_interval_mm = None;
    let mut report = None;
    let mut half_width = None;
    if let Some((validated, expected)) = qualification {
        let candidate = validated.report();
        let operands_match =
            left.operand()
                .ok()
                .zip(right.operand().ok())
                .is_some_and(|(left, right)| {
                    left.model_id == expected.left_model_id
                        && right.model_id == expected.right_model_id
                        && left.analysis_ref.as_ref() == Some(&expected.left_analysis)
                        && right.analysis_ref.as_ref() == Some(&expected.right_analysis)
                        && left.analysis_mesh.as_ref() == Some(&expected.left_mesh)
                        && right.analysis_mesh.as_ref() == Some(&expected.right_mesh)
                        && left.row_transform_f64.map(f64::to_bits)
                            == expected.left_row_transform_f64.map(f64::to_bits)
                        && right.row_transform_f64.map(f64::to_bits)
                            == expected.right_row_transform_f64.map(f64::to_bits)
                });
        if serde_json::to_vec(&candidate.binding).ok() != serde_json::to_vec(expected).ok()
            || expected.clearance_threshold_mm.to_bits() != clearance_threshold_mm.to_bits()
            || expected.requested_accuracy_mm.map(f64::to_bits)
                != requested_accuracy_mm.map(f64::to_bits)
            || !operands_match
        {
            reasons.push("validated qualifier belongs to another assessment request".into());
        } else {
            report = Some(candidate.clone());
            match (&candidate.mesh_distance, &candidate.source_bounds) {
                (
                    MeshDistance::Bounded {
                        lower_mm, upper_mm, ..
                    },
                    [
                        WorldSourceBound::Bounded {
                            total_bound_mm: left,
                            ..
                        },
                        WorldSourceBound::Bounded {
                            total_bound_mm: right,
                            ..
                        },
                    ],
                ) => {
                    let lower = add_down(add_down(*lower_mm, -*left), -*right).max(0.0);
                    let upper = add_up(add_up(*upper_mm, *left), *right);
                    let budget = boardstudio_web_host::cad_qualification::pair_error_budget(
                        *lower_mm, *upper_mm, *left, *right,
                    );
                    if lower.is_finite() && upper.is_finite() && budget.is_finite() {
                        source_interval_mm = Some([lower, upper]);
                        half_width = Some(budget);
                    } else {
                        reasons.push("outward source interval overflow".into());
                    }
                }
                _ => reasons.push("source or numerical query qualification is unknown".into()),
            }
            if let MeshDistance::Unknown { reasons: why } = &candidate.mesh_distance {
                reasons.extend(why.iter().cloned());
            }
            for bound in &candidate.source_bounds {
                if let WorldSourceBound::Unknown { reasons: why } = bound {
                    reasons.extend(why.iter().cloned());
                }
            }
            if let ContactRegion::Unknown { reasons: why } = &candidate.contact_region {
                reasons.push("contact location is unqualified".into());
                reasons.extend(why.iter().cloned());
            }
        }
    } else {
        reasons.push("source/query qualification is unavailable".into());
    }
    if let (Some(requested), Some(budget)) = (requested_accuracy_mm, half_width)
        && budget > requested
    {
        reasons.push(format!(
            "pair error budget {budget} mm exceeds requested accuracy {requested} mm"
        ));
    }
    let assessment = if reasons.is_empty() {
        if let Some([lower_mm, upper_mm]) = source_interval_mm {
            let containment_ruled_out = report
                .as_ref()
                .is_some_and(|r| matches!(r.containment, Containment::RuledOut { .. }));
            let relation = if upper_mm < clearance_threshold_mm {
                CadCollisionRelation::Violation
            } else if lower_mm >= clearance_threshold_mm && containment_ruled_out {
                CadCollisionRelation::Clear
            } else {
                CadCollisionRelation::Uncertain
            };
            CadCollisionAssessment::SupportedBand {
                lower_mm,
                upper_mm,
                relation,
            }
        } else {
            CadCollisionAssessment::Unknown {
                reasons: vec!["source interval is unavailable".into()],
                refinement_requested: requested_accuracy_mm.is_some(),
            }
        }
    } else {
        CadCollisionAssessment::Unknown {
            reasons,
            refinement_requested: requested_accuracy_mm.is_some(),
        }
    };
    CadCollisionResult {
        observation: Some(observation),
        left_provenance,
        right_provenance,
        assessment,
        source_interval_mm,
        query_qualification: report,
    }
}
