use crate::{MeshBuffers, mesh::mesh_buffers, qualification::SourceBoundResult};
use boardstudio_contracts::Vec2;
use monstertruck_meshing::tessellation::{
    TessellationOptions, shell_to_polygon_strict_with, triangulation_with,
};
use monstertruck_modeling::{Curve, Invertible, Point3, Solid, Surface, Vector3, Wire, builder};

const PROFILE_MESH_TOLERANCE_MM: f64 = 0.1;
const PROFILE_EDGE_ANGULAR_TOLERANCE_RAD: f64 = 0.5;

/// Opaque exact solids created from a prepared case profile.
pub struct ExactModel {
    pub(crate) solids: Vec<Solid>,
}

impl ExactModel {
    /// Returns physical-solid and conservative face/edge-occurrence cache costs.
    pub fn cache_complexity(&self) -> (usize, usize) {
        let mut topology = 0usize;
        for solid in &self.solids {
            for face in solid.face_iter() {
                topology = topology
                    .saturating_add(1)
                    .saturating_add(face.edge_iter().count());
            }
        }
        (self.solids.len(), topology)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedSourceFace {
    pub solid_index: usize,
    pub shell_index: usize,
    pub face_index: usize,
    pub support_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedAnalysisMesh {
    pub mesh: MeshBuffers,
    pub positions_mm_f64: Vec<f64>,
    pub source_faces: Vec<GeneratedSourceFace>,
    pub triangle_face_occurrence_indices: Vec<usize>,
    pub bounds: [[f64; 3]; 2],
    pub analysis_error_bound_mm: Option<f64>,
    pub mesh_closed_oriented: bool,
    pub deflection_mm: f64,
    pub angular_rad: f64,
}

/// Tessellates exact profile solids into complete, oriented triangle buffers.
pub fn mesh_profile(model: &ExactModel) -> Result<MeshBuffers, String> {
    let mut meshes = Vec::with_capacity(model.solids.len());
    for solid in &model.solids {
        for shell in solid.boundaries() {
            meshes.push(
                shell_to_polygon_strict_with(
                    shell,
                    TessellationOptions {
                        tolerance: PROFILE_MESH_TOLERANCE_MM,
                        edge_angular_tolerance: Some(PROFILE_EDGE_ANGULAR_TOLERANCE_RAD),
                        ..Default::default()
                    },
                )
                .map_err(|error| format!("Monstertruck could not mesh case profile: {error:?}"))?,
            );
        }
    }
    mesh_buffers(meshes)
}

/// Tessellates exact generated case/PCB solids into f64 analysis buffers with per-face
/// provenance. Requested tolerances do not certify source-to-mesh error.
pub fn mesh_profile_for_analysis(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
) -> Result<GeneratedAnalysisMesh, String> {
    mesh_profile_for_analysis_impl(model, target_deflection_mm, None)
}

fn mesh_profile_for_analysis_impl(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
    mut trace: Option<&mut final_trace::AnalysisTrace>,
) -> Result<GeneratedAnalysisMesh, String> {
    if model.solids.is_empty() {
        return Err("Case analysis mesh requires at least one exact solid".into());
    }
    let deflection_mm = match target_deflection_mm {
        Some(target) => {
            if !target.is_finite() || target <= 0.0 {
                return Err("targetDeflectionMm must be finite and positive".into());
            }
            target.min(PROFILE_MESH_TOLERANCE_MM)
        }
        None => PROFILE_MESH_TOLERANCE_MM,
    };
    let options = TessellationOptions {
        tolerance: deflection_mm,
        edge_angular_tolerance: Some(PROFILE_EDGE_ANGULAR_TOLERANCE_RAD),
        ..Default::default()
    };

    let mut mesh = MeshBuffers::default();
    let mut positions_mm_f64 = Vec::new();
    let mut source_faces = Vec::new();
    let mut triangle_face_occurrence_indices = Vec::new();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let mut mesh_closed_oriented = true;

    for (solid_index, solid) in model.solids.iter().enumerate() {
        let mut signed_volume = 0.0;
        for (shell_index, shell) in solid.boundaries().iter().enumerate() {
            // Validate whole-shell strict tessellation first so edge/metric failures refuse.
            let _ = shell_to_polygon_strict_with(shell, options)
                .map_err(|error| format!("Monstertruck could not mesh case profile: {error:?}"))?;
            let meshed = triangulation_with(shell, options);
            if meshed.face_iter().count() != shell.face_iter().count() {
                return Err("Generated analysis dropped a source face".into());
            }
            let mut shell_triangles = Vec::new();
            for ((face_index, face), meshed_face) in
                shell.face_iter().enumerate().zip(meshed.face_iter())
            {
                let support_kind = match face.surface() {
                    Surface::Plane(_) => "PLANE",
                    Surface::RevolutionSurface(_) => "REVOLUTION_SURFACE",
                    _ => "SURFACE",
                };
                let mut poly = meshed_face.surface().ok_or_else(|| {
                    format!("Monstertruck dropped generated case face {face_index}")
                })?;
                if !meshed_face.orientation() {
                    poly.invert();
                }
                let occurrence_index = source_faces.len();
                source_faces.push(GeneratedSourceFace {
                    solid_index,
                    shell_index,
                    face_index,
                    support_kind: support_kind.into(),
                });
                if let Some(trace) = trace.as_deref_mut() {
                    trace.record_face(
                        &source_faces[occurrence_index],
                        face,
                        meshed_face.orientation(),
                        &poly,
                    );
                }
                let mut face_triangles = 0usize;
                for triangle in poly.faces().triangle_iter() {
                    let points = triangle.map(|v| poly.positions()[v.pos]);
                    crate::mesh::push_triangle(&mut mesh, points)?;
                    shell_triangles.push(points);
                    for point in points {
                        for (axis, coord) in [point.x, point.y, point.z].into_iter().enumerate() {
                            if !coord.is_finite() {
                                return Err(
                                    "Monstertruck produced a non-finite analysis coordinate".into(),
                                );
                            }
                            bounds[0][axis] = bounds[0][axis].min(coord);
                            bounds[1][axis] = bounds[1][axis].max(coord);
                            positions_mm_f64.push(coord);
                        }
                    }
                    triangle_face_occurrence_indices.push(occurrence_index);
                    // Observe only the accepted, oriented fan triangle, using the final
                    // f64 coordinates and the f32 conversion that push_triangle emitted.
                    if let Some(trace) = trace.as_deref_mut() {
                        trace.record_triangle(
                            occurrence_index,
                            &poly,
                            triangle,
                            &positions_mm_f64[positions_mm_f64.len() - 9..],
                            &mesh.positions[mesh.positions.len() - 9..],
                        );
                    }
                    face_triangles += 1;
                }
                if face_triangles == 0 {
                    return Err(format!(
                        "Generated case face {face_index} produced no triangles"
                    ));
                }
            }
            mesh_closed_oriented &= closed_oriented_shell_triangles(&shell_triangles);
            if let Some(first) = shell_triangles.first() {
                let reference = first[0];
                use monstertruck_modeling::InnerSpace;
                for [a, b, c] in shell_triangles {
                    signed_volume +=
                        (a - reference).dot((b - reference).cross(c - reference)) / 6.0;
                }
            }
        }
        mesh_closed_oriented &= signed_volume.is_finite() && signed_volume > 0.0;
    }
    if positions_mm_f64.is_empty()
        || bounds.iter().flatten().any(|v| !v.is_finite())
        || (0..3).any(|axis| bounds[0][axis] > bounds[1][axis])
    {
        return Err("Monstertruck produced an empty or invalid analysis mesh".into());
    }
    Ok(GeneratedAnalysisMesh {
        mesh,
        positions_mm_f64,
        source_faces,
        triangle_face_occurrence_indices,
        bounds,
        analysis_error_bound_mm: None,
        mesh_closed_oriented,
        deflection_mm,
        angular_rad: PROFILE_EDGE_ANGULAR_TOLERANCE_RAD,
    })
}

fn closed_oriented_shell_triangles(triangles: &[[Point3; 3]]) -> bool {
    let point = |p: Point3| {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return None;
        }
        // Exact coordinate identity, with signed zero canonicalized; never weld a gap.
        Some([p.x, p.y, p.z].map(|v| if v == 0.0 { 0 } else { v.to_bits() }))
    };
    crate::mesh::closed_oriented_edges(triangles, point)
}

mod final_trace;

mod generated_qualification;

// Owner-side wrapper keeps the native AST and retained trace private.
pub(crate) fn prepare_qualified_emission(
    model: &ExactModel,
    target: Option<f64>,
) -> Result<(GeneratedAnalysisMesh, SourceBoundResult), String> {
    let mut trace = final_trace::AnalysisTrace::default();
    let mesh = mesh_profile_for_analysis_impl(model, target, Some(&mut trace))?;
    let within_profile = mesh.triangle_face_occurrence_indices.len() <= 10_000
        && mesh
            .positions_mm_f64
            .iter()
            .all(|x| x.is_finite() && (*x == 0. || (1e-100..=1e12).contains(&x.abs())));
    let result = if within_profile {
        let planar = generated_qualification::qualify_generated(model, &mesh);
        if matches!(
            planar,
            generated_qualification::GeneratedQualification::Unknown { .. }
        ) {
            final_trace::qualified_bound(model, &mesh, &trace)
        } else {
            planar
        }
    } else {
        generated_qualification::GeneratedQualification::Unknown {
            reasons: vec!["generated source profile cap".into()],
        }
    };
    let bound = match result {
        generated_qualification::GeneratedQualification::Bounded {
            direct_bound_mm,
            source_precision_mm,
            total_bound_mm,
            ..
        } => Ok([direct_bound_mm, source_precision_mm, total_bound_mm]),
        generated_qualification::GeneratedQualification::Unknown { reasons } => Err(reasons),
    };
    Ok((mesh, bound))
}

#[cfg(test)]
mod analysis_tests {
    use super::*;

    #[test]
    fn generated_topology_does_not_weld_an_open_submicron_gap() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        let mut triangles = [[a, c, b], [a, b, d], [a, d, c], [b, c, d]];
        assert!(closed_oriented_shell_triangles(&triangles));
        triangles[0][0].x = 1e-9;
        assert!(
            !closed_oriented_shell_triangles(&triangles),
            "topology metadata must not silently weld a gap"
        );
    }
}
