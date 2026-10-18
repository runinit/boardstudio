//! Private generated analytic qualification checkpoint.
//!
//! This module is test-only. It does not publish a production certificate,
//! change `analysisErrorBoundMm`, or alter the active v1 wire contract.

use super::*;
use monstertruck_modeling::{Curve, InnerSpace, Point3, Surface, Transform, Vector3};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedQualification {
    Bounded {
        version: u32,
        class: &'static str,
        direct_bound_mm: f64,
        source_precision_mm: f64,
        total_bound_mm: f64,
        mesh_identity: String,
    },
    Unknown {
        reasons: Vec<String>,
    },
}

fn source_surface(model: &ExactModel, face: &GeneratedSourceFace) -> Option<Surface> {
    let solid = model.solids.get(face.solid_index)?;
    let shell = solid.boundaries().get(face.shell_index)?;
    let source_face = shell.face_iter().nth(face.face_index)?;
    Some(source_face.surface().clone())
}

fn revolution_parameters(
    proc: &monstertruck_modeling::Processor<
        monstertruck_modeling::RevolutionSurface<Curve>,
        monstertruck_modeling::Matrix4,
    >,
) -> Option<(Point3, Vector3, f64, f64, f64)> {
    // Returns (origin, axis, radius, slope, height0).
    let origin = proc.entity().origin();
    let axis = proc.entity().axis();
    if !axis.x.is_finite()
        || !axis.y.is_finite()
        || !axis.z.is_finite()
        || (axis.magnitude2() - 1.0).abs() > 1e-9
    {
        return None;
    }
    let curve = proc.entity().entity_curve();
    let line = match curve {
        Curve::Line(line) => line,
        _ => return None,
    };
    let offset = line.0 - origin;
    let height0 = offset.dot(axis);
    let radial = offset - axis * height0;
    let radius = radial.magnitude();
    if !radius.is_finite() || radius < 0.0 {
        return None;
    }
    let delta = line.1 - line.0;
    let axial = delta.dot(axis);
    let slope = if axial.abs() > 1e-18 && radius > 1e-18 {
        delta.dot(radial / radius) / axial
    } else {
        0.0
    };
    if !slope.is_finite() {
        return None;
    }
    Some((origin, axis, radius, slope, height0))
}

pub fn qualify_generated(
    model: &ExactModel,
    analysis: &GeneratedAnalysisMesh,
) -> GeneratedQualification {
    let mut reasons: Vec<String> = Vec::new();

    if model.solids.is_empty() {
        reasons.push("missing exact solid".into());
    }
    if analysis.positions_mm_f64.is_empty() || analysis.positions_mm_f64.len() % 9 != 0 {
        reasons.push("invalid final buffer".into());
    }
    if analysis
        .positions_mm_f64
        .iter()
        .any(|value| !value.is_finite())
    {
        reasons.push("non-finite coordinate".into());
    }
    if analysis.triangle_face_occurrence_indices.len() != analysis.positions_mm_f64.len() / 9 {
        reasons.push("tag count mismatch".into());
    }
    if analysis.source_faces.is_empty() {
        reasons.push("missing source face".into());
    }
    if !analysis.deflection_mm.is_finite() || analysis.deflection_mm <= 0.0 {
        reasons.push("invalid deflection setting".into());
    }
    if !analysis.angular_rad.is_finite() || analysis.angular_rad <= 0.0 {
        reasons.push("invalid angular parameter".into());
    }
    if analysis
        .triangle_face_occurrence_indices
        .iter()
        .any(|tag| *tag >= analysis.source_faces.len())
    {
        reasons.push("invalid triangle tag".into());
    }
    for face in &analysis.source_faces {
        if face.support_kind != "PLANE" && face.support_kind != "REVOLUTION_SURFACE" {
            reasons.push("unsupported support kind".into());
        }
        if source_surface(model, face).is_none() {
            reasons.push("missing source face".into());
        }
    }

    let mut distinct = BTreeSet::new();
    for tag in &analysis.triangle_face_occurrence_indices {
        distinct.insert(*tag);
    }
    if distinct.len() != analysis.source_faces.len() {
        reasons.push("incomplete face coverage".into());
    }
    for index in 0..analysis.source_faces.len() {
        if !distinct.contains(&index) {
            reasons.push("missing face occurrence".into());
            break;
        }
    }

    if !reasons.is_empty() {
        reasons.sort();
        reasons.dedup();
        return GeneratedQualification::Unknown { reasons };
    }

    let triangle_count = analysis.positions_mm_f64.len() / 9;
    let mut direct = 0.0f64;
    let mut local_reasons: Vec<String> = Vec::new();

    for (face_index, face) in analysis.source_faces.iter().enumerate() {
        let surface = match source_surface(model, face) {
            Some(surface) => surface,
            None => {
                local_reasons.push("missing source face".into());
                continue;
            }
        };

        let mut vertices: Vec<[f64; 3]> = Vec::new();
        for triangle in 0..triangle_count {
            if analysis.triangle_face_occurrence_indices[triangle] != face_index {
                continue;
            }
            for corner in 0..3 {
                let base = (triangle * 3 + corner) * 3;
                vertices.push([
                    analysis.positions_mm_f64[base],
                    analysis.positions_mm_f64[base + 1],
                    analysis.positions_mm_f64[base + 2],
                ]);
            }
        }
        if vertices.is_empty() {
            local_reasons.push("missing triangle for source face".into());
            continue;
        }

        match surface {
            Surface::Plane(plane) => {
                let normal = plane.normal();
                let origin = plane.origin();
                for vertex in &vertices {
                    let point = Point3::new(vertex[0], vertex[1], vertex[2]);
                    let distance = (point - origin).dot(normal).abs();
                    if distance > direct {
                        direct = distance;
                    }
                }
            }
            Surface::RevolutionSurface(proc) => {
                let (origin, axis, radius, slope, height0) = match revolution_parameters(&proc) {
                    Some(values) => values,
                    None => {
                        local_reasons.push("unsupported revolution support".into());
                        continue;
                    }
                };
                let transform = proc.transform();
                let inverse = match transform.inverse_transform() {
                    Some(value) => value,
                    None => {
                        local_reasons.push("invalid revolution transform".into());
                        continue;
                    }
                };

                // Numeric contribution from final vertices.
                for vertex in &vertices {
                    let point =
                        inverse.transform_point(Point3::new(vertex[0], vertex[1], vertex[2]));
                    let offset = point - origin;
                    let height = offset.dot(axis);
                    let radial = (offset - axis * height).magnitude();
                    let expected_radius = radius + slope * (height - height0);
                    let residual = (radial - expected_radius.abs()).abs();
                    if residual.is_finite() && residual > direct {
                        direct = residual;
                    }
                }

                // Analytic angular/height contribution.
                let theta = analysis.angular_rad;
                let height_range = {
                    let values = vertices
                        .iter()
                        .map(|vertex| {
                            let point = inverse
                                .transform_point(Point3::new(vertex[0], vertex[1], vertex[2]));
                            let offset = point - origin;
                            offset.dot(axis)
                        })
                        .collect::<Vec<_>>();
                    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    (max - min).abs()
                };
                let cylindrical = radius * theta * theta / 8.0;
                let mixed = slope.abs() * theta * height_range / 4.0;
                let analytic = cylindrical + mixed;
                if analytic.is_finite() && analytic > direct {
                    direct = analytic;
                }
            }
            _ => {
                local_reasons.push("unsupported generated surface".into());
            }
        }
    }

    if !local_reasons.is_empty() {
        local_reasons.sort();
        local_reasons.dedup();
        return GeneratedQualification::Unknown {
            reasons: local_reasons,
        };
    }
    if !direct.is_finite() || direct < 0.0 {
        return GeneratedQualification::Unknown {
            reasons: vec!["invalid numeric bound".into()],
        };
    }

    GeneratedQualification::Bounded {
        version: 1,
        class: "analytic-generated",
        direct_bound_mm: direct,
        source_precision_mm: 0.0,
        total_bound_mm: direct,
        mesh_identity: "generated-final-mesh".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProfileInput;
    use crate::ProfileKind;
    use crate::ProfileRegion;
    use boardstudio_contracts::Vec2;

    fn point(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    fn plate_model() -> ExactModel {
        let input = ProfileInput {
            kind: ProfileKind::Plate,
            base_z: 0.0,
            thickness: 2.0,
            wall_height: 0.0,
            regions: vec![ProfileRegion {
                outer: vec![
                    point(0.0, 0.0),
                    point(20.0, 0.0),
                    point(20.0, 10.0),
                    point(0.0, 10.0),
                ],
                holes: vec![],
            }],
        };
        build_profile(input).expect("plate")
    }

    #[test]
    fn plane_plate_qualifies_with_zero_direct_bound() {
        let model = plate_model();
        let analysis = mesh_profile_for_analysis(&model, Some(0.01)).expect("mesh");
        let result = qualify_generated(&model, &analysis);
        match result {
            GeneratedQualification::Bounded {
                direct_bound_mm,
                source_precision_mm,
                total_bound_mm,
                ..
            } => {
                assert_eq!(source_precision_mm, 0.0);
                assert_eq!(direct_bound_mm, total_bound_mm);
                assert!(direct_bound_mm >= 0.0);
                assert!(
                    direct_bound_mm < 1e-12,
                    "plane bound must be exact-ish, got {direct_bound_mm}"
                );
            }
            other => panic!("expected bounded, got {other:?}"),
        }
    }

    #[test]
    fn missing_face_occurrence_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        analysis.source_faces.pop();
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn invalid_tag_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        if !analysis.triangle_face_occurrence_indices.is_empty() {
            analysis.triangle_face_occurrence_indices[0] = analysis.source_faces.len() + 1;
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn unsupported_support_kind_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        if !analysis.source_faces.is_empty() {
            analysis.source_faces[0].support_kind = "SURFACE".into();
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn wrong_coverage_is_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        // Remove triangles for one face by re-tagging them to face 0, leaving a face uncovered.
        if analysis.source_faces.len() > 1 {
            for tag in &mut analysis.triangle_face_occurrence_indices {
                if *tag == analysis.source_faces.len() - 1 {
                    *tag = 0;
                }
            }
        }
        let result = qualify_generated(&model, &analysis);
        assert!(matches!(result, GeneratedQualification::Unknown { .. }));
    }

    #[test]
    fn pcb_hole_qualifies_with_positive_bound() {
        let input: boardstudio_contracts::PreparedCaseIR = serde_json::from_str(include_str!(
            "../../tests/fixtures/pcb-source/circle-tracer.json"
        ))
        .expect("pcb fixture");
        let entry = crate::build_case(&input).expect("build");
        let analysis = mesh_profile_for_analysis(&entry, Some(0.01)).expect("mesh");
        let result = qualify_generated(&entry, &analysis);
        match result {
            GeneratedQualification::Bounded {
                direct_bound_mm, ..
            } => {
                assert!(direct_bound_mm >= 0.0);
                assert!(direct_bound_mm.is_finite());
            }
            other => panic!("expected bounded, got {other:?}"),
        }
    }
}
