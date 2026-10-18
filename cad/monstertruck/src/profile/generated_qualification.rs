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

fn source_face_boundaries<'a>(
    model: &'a ExactModel,
    face: &GeneratedSourceFace,
) -> Option<Vec<Wire>> {
    let solid = model.solids.get(face.solid_index)?;
    let shell = solid.boundaries().get(face.shell_index)?;
    let source_face = shell.face_iter().nth(face.face_index)?;
    Some(source_face.boundaries())
}

fn wire_vertices(wire: &Wire) -> Vec<Point3> {
    let mut points = Vec::new();
    for edge in wire.edge_iter() {
        let (start, _) = edge.ends();
        let point = start.point();
        if points.last().copied() != Some(point) {
            points.push(point);
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points
}

fn plane_loop_area(normal: Vector3, points: &[Point3]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let origin = points[0];
    let mut sum = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        let cross = (points[index] - origin).cross(points[next] - origin);
        sum += cross.dot(normal);
    }
    sum / 2.0
}

fn source_plane_area(
    model: &ExactModel,
    face: &GeneratedSourceFace,
    normal: Vector3,
) -> Option<f64> {
    let boundaries = source_face_boundaries(model, face)?;
    let mut area = 0.0;
    for wire in &boundaries {
        area += plane_loop_area(normal, &wire_vertices(wire));
    }
    Some(area.abs())
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
    // Native census: count exact retained source faces independently of the
    // analysis payload's source face list.
    let mut native_face_count = 0usize;
    for solid in &model.solids {
        for shell in solid.boundaries() {
            native_face_count += shell.face_iter().count();
        }
    }
    if native_face_count != analysis.source_faces.len() {
        reasons.push("incomplete native source face census".into());
    }
    if analysis
        .source_faces
        .iter()
        .any(|face| face.support_kind == "REVOLUTION_SURFACE")
    {
        reasons.push("revolution support UV/trim/numeric proof unavailable".into());
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
                // Every final vertex must lie exactly on the retained source plane.
                for vertex in &vertices {
                    let point = Point3::new(vertex[0], vertex[1], vertex[2]);
                    let distance = (point - origin).dot(normal);
                    if !distance.is_finite() {
                        local_reasons.push("non-finite plane residual".into());
                        break;
                    }
                    if distance != 0.0 {
                        local_reasons.push("final vertex off source plane".into());
                        break;
                    }
                    let residual = distance.abs();
                    if residual > direct {
                        direct = residual;
                    }
                }
                if local_reasons.iter().any(|r| {
                    r == "final vertex off source plane" || r == "non-finite plane residual"
                }) {
                    // Skip area check once plane correspondence is already invalid.
                } else {
                    let source_area = match source_plane_area(model, face, normal) {
                        Some(value) if value.is_finite() && value > 0.0 => value,
                        _ => {
                            local_reasons.push("invalid source plane area".into());
                            0.0
                        }
                    };
                    // Final triangle area on the same plane.
                    let mut mesh_area = 0.0;
                    for triangle in vertices.as_chunks::<3>().0 {
                        if triangle.len() < 3 {
                            continue;
                        }
                        let a = Point3::new(triangle[0][0], triangle[0][1], triangle[0][2]);
                        let b = Point3::new(triangle[1][0], triangle[1][1], triangle[1][2]);
                        let c = Point3::new(triangle[2][0], triangle[2][1], triangle[2][2]);
                        let cross = (b - a).cross(c - a);
                        mesh_area += cross.dot(normal) / 2.0;
                    }
                    mesh_area = mesh_area.abs();
                    if source_area == 0.0 || mesh_area != source_area {
                        local_reasons.push("incomplete plane trim coverage".into());
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
    fn shrunk_plane_triangles_are_unknown() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        // Shrink every triangle toward its own centroid. Vertices stay on the
        // source planes, but the covered area no longer matches the source face.
        for triangle in 0..(analysis.positions_mm_f64.len() / 9) {
            let base = triangle * 9;
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            for corner in 0..3 {
                cx += analysis.positions_mm_f64[base + corner * 3];
                cy += analysis.positions_mm_f64[base + corner * 3 + 1];
                cz += analysis.positions_mm_f64[base + corner * 3 + 2];
            }
            cx /= 3.0;
            cy /= 3.0;
            cz /= 3.0;
            for corner in 0..3 {
                let index = base + corner * 3;
                analysis.positions_mm_f64[index] = (analysis.positions_mm_f64[index] + cx) * 0.5;
                analysis.positions_mm_f64[index + 1] =
                    (analysis.positions_mm_f64[index + 1] + cy) * 0.5;
                analysis.positions_mm_f64[index + 2] =
                    (analysis.positions_mm_f64[index + 2] + cz) * 0.5;
            }
        }
        let result = qualify_generated(&model, &analysis);
        match result {
            GeneratedQualification::Unknown { reasons } => {
                assert!(
                    reasons
                        .iter()
                        .any(|reason| reason.contains("trim coverage"))
                );
            }
            other => panic!("expected unknown, got {other:?}"),
        }
    }

    #[test]
    fn removed_source_face_is_unknown_by_native_census() {
        let model = plate_model();
        let mut analysis = mesh_profile_for_analysis(&model, None).expect("mesh");
        let removed = analysis.source_faces.len() - 1;
        analysis.source_faces.pop();
        let mut kept_tags = Vec::new();
        for tag in &analysis.triangle_face_occurrence_indices {
            if *tag == removed {
                continue;
            }
            let adjusted = if *tag > removed { *tag - 1 } else { *tag };
            kept_tags.push(adjusted);
        }
        analysis.triangle_face_occurrence_indices = kept_tags;
        // Keep positions length consistent by dropping removed face's triangles.
        // (Positions length no longer matches tags; census/shape checks reject.)
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
    fn pcb_hole_is_unknown_until_uv_trim_proof_exists() {
        let input: boardstudio_contracts::PreparedCaseIR = serde_json::from_str(include_str!(
            "../../tests/fixtures/pcb-source/circle-tracer.json"
        ))
        .expect("pcb fixture");
        let entry = crate::build_case(&input).expect("build");
        let analysis = mesh_profile_for_analysis(&entry, Some(0.01)).expect("mesh");
        let result = qualify_generated(&entry, &analysis);
        match result {
            GeneratedQualification::Unknown { reasons } => {
                assert!(
                    reasons
                        .iter()
                        .any(|reason| reason.contains("revolution support"))
                );
            }
            other => panic!("expected unknown, got {other:?}"),
        }
    }
}
