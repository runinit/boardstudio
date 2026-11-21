use crate::layered::{
    brep::{self, Point3, Support},
    tessellation,
};
use crate::{MeshBuffers, mesh::push_triangle};
use cgmath::InnerSpace;

const PROFILE_MESH_TOLERANCE_MM: f64 = 0.1;
pub(crate) const PROFILE_EDGE_ANGULAR_TOLERANCE_RAD: f64 = 0.5;

/// Opaque exact solids created from a prepared case profile.
pub struct ExactModel {
    pub(crate) regions: Vec<brep::Region>,
}

impl ExactModel {
    pub(crate) fn empty() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    pub(crate) fn from_region(region: brep::Region) -> Self {
        let mut model = Self::empty();
        model.regions = vec![region];
        model
    }

    pub(crate) fn shells(&self) -> impl Iterator<Item = (&brep::Topology, &[brep::Face])> {
        self.regions.iter().flat_map(|region| {
            region
                .shells
                .iter()
                .map(|shell| (&region.topology, shell.as_slice()))
        })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.shells().next().is_none()
    }

    /// Returns physical-solid and conservative face/edge-occurrence cache costs.
    pub fn cache_complexity(&self) -> (usize, usize) {
        let mut topology = 0usize;
        let mut solids = 0usize;
        for region in &self.regions {
            let mut boundary_uses = 0usize;
            let mut strip_uses = 0usize;
            for face in region.shells.iter().flatten() {
                boundary_uses = boundary_uses.saturating_add(1);
                for ring in &face.rings {
                    boundary_uses = boundary_uses.saturating_add(ring.len());
                }
                strip_uses = strip_uses.saturating_add(face.strips.len().saturating_mul(2));
            }
            solids += region.shells.len();
            topology = topology.saturating_add(
                boundary_uses.max(region.topology.edges.len().saturating_add(strip_uses)),
            );
        }
        (solids, topology)
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
    let mut result = MeshBuffers::default();
    for (topology, faces) in model.shells() {
        let meshes = tessellation::tessellate(
            topology,
            faces,
            PROFILE_MESH_TOLERANCE_MM,
            PROFILE_EDGE_ANGULAR_TOLERANCE_RAD,
        )?;
        for points in meshes
            .iter()
            .flat_map(tessellation::FaceMesh::triangle_points)
        {
            push_triangle(&mut result, points)?;
        }
    }
    if result.positions.is_empty() {
        return Err("Generated case produced an empty mesh".into());
    }
    Ok(result)
}

/// Tessellates exact generated case/PCB solids into f64 analysis buffers with per-face
/// provenance. Requested tolerances do not certify source-to-mesh error.
pub fn mesh_profile_for_analysis(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
) -> Result<GeneratedAnalysisMesh, String> {
    emit_analysis(model, target_deflection_mm)
}

struct EmittedFace<T> {
    support_kind: &'static str,
    triangles: T,
}

fn emit_analysis(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
) -> Result<GeneratedAnalysisMesh, String> {
    let deflection_mm = match target_deflection_mm {
        Some(target) => {
            if !target.is_finite() || target <= 0.0 {
                return Err("targetDeflectionMm must be finite and positive".into());
            }
            target.min(PROFILE_MESH_TOLERANCE_MM)
        }
        None => PROFILE_MESH_TOLERANCE_MM,
    };

    let mut mesh = MeshBuffers::default();
    let mut positions_mm_f64 = Vec::new();
    let mut source_faces = Vec::new();
    let mut triangle_face_occurrence_indices = Vec::new();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let mut mesh_closed_oriented = true;

    type Triangles = tessellation::FaceMesh;
    let mut emit_shell = |solid_index,
                          shell_index,
                          faces: Vec<EmittedFace<Triangles>>|
     -> Result<(bool, f64), String> {
        let mut shell_triangles = Vec::new();
        for (face_index, face) in faces.into_iter().enumerate() {
            let EmittedFace {
                triangles,
                support_kind,
            } = face;
            let occurrence_index = source_faces.len();
            source_faces.push(GeneratedSourceFace {
                solid_index,
                shell_index,
                face_index,
                support_kind: support_kind.into(),
            });
            let count_before = triangle_face_occurrence_indices.len();
            let mut emit_triangle = |points: [Point3; 3]| -> Result<(), String> {
                push_triangle(&mut mesh, points)?;
                shell_triangles.push(points);
                for point in points {
                    for (axis, coord) in [point.x, point.y, point.z].into_iter().enumerate() {
                        if !coord.is_finite() {
                            return Err(
                                "Generated case produced a non-finite analysis coordinate".into()
                            );
                        }
                        bounds[0][axis] = bounds[0][axis].min(coord);
                        bounds[1][axis] = bounds[1][axis].max(coord);
                        positions_mm_f64.push(coord);
                    }
                }
                triangle_face_occurrence_indices.push(occurrence_index);
                Ok(())
            };
            for points in triangles.triangle_points() {
                emit_triangle(points)?;
            }
            if triangle_face_occurrence_indices.len() == count_before {
                return Err(format!(
                    "Generated case face {face_index} produced no triangles"
                ));
            }
        }
        let closed = closed_oriented_shell_triangles(&shell_triangles);
        let mut signed_volume = 0.;
        if let Some(first) = shell_triangles.first() {
            let reference = first[0];
            for [a, b, c] in shell_triangles {
                signed_volume += (a - reference).dot((b - reference).cross(c - reference)) / 6.;
            }
        }
        Ok((closed, signed_volume))
    };

    for (solid_index, (topology, shell)) in model.shells().enumerate() {
        let generated = tessellation::tessellate(
            topology,
            shell,
            deflection_mm,
            PROFILE_EDGE_ANGULAR_TOLERANCE_RAD,
        )?;
        let faces = shell
            .iter()
            .zip(generated)
            .map(|(face, mesh)| EmittedFace {
                support_kind: match face.support {
                    Support::Plane { .. } => "PLANE",
                    Support::Round { .. } => "REVOLUTION_SURFACE",
                },
                triangles: mesh,
            })
            .collect();
        let (closed, volume) = emit_shell(solid_index, 0, faces)?;
        mesh_closed_oriented &= closed;
        // Native shells already have exact oriented incidence; still check
        // positive volume on the emitted triangles.
        if !volume.is_finite() || volume <= 0. {
            return Err("Generated case has nonpositive mesh volume".into());
        }
    }
    if positions_mm_f64.is_empty()
        || bounds.iter().flatten().any(|v| !v.is_finite())
        || (0..3).any(|axis| bounds[0][axis] > bounds[1][axis])
    {
        return Err("Generated case produced an empty or invalid analysis mesh".into());
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

pub(crate) fn closed_oriented_shell_triangles(triangles: &[[Point3; 3]]) -> bool {
    let point = |p: Point3| {
        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
            return None;
        }
        // Exact coordinate identity, with signed zero canonicalized; never weld a gap.
        Some([p.x, p.y, p.z].map(|v| if v == 0.0 { 0 } else { v.to_bits() }))
    };
    crate::mesh::closed_oriented_edges(triangles, point)
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
