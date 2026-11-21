use crate::layered::{
    brep::{self, Point3, Support},
    tessellation,
};
#[cfg(test)]
use crate::mesh::mesh_buffers;
use crate::{MeshBuffers, mesh::push_triangle, qualification::proof_geometry};
use cgmath::InnerSpace;
use monstertruck_meshing::rexport_polymesh::{PolygonMesh, StandardVertex};
#[cfg(test)]
use monstertruck_meshing::tessellation::{
    TessellationOptions, shell_to_polygon_strict_with, triangulation_with,
};
use monstertruck_modeling::{Face, Shell, Solid};
#[cfg(test)]
use monstertruck_modeling::{Invertible, Surface};
use std::borrow::Cow;
#[cfg(test)]
use std::sync::OnceLock;

const PROFILE_MESH_TOLERANCE_MM: f64 = 0.1;
pub(crate) const PROFILE_EDGE_ANGULAR_TOLERANCE_RAD: f64 = 0.5;

/// Opaque exact solids created from a prepared case profile.
pub struct ExactModel {
    pub(crate) regions: Vec<brep::Region>,
    // Test-only access to the external geometry oracle and synthetic proof mutations.
    #[cfg(test)]
    fixture: OnceLock<Vec<Solid>>,
    #[cfg(test)]
    use_fixture: bool,
}

impl ExactModel {
    pub(crate) fn empty() -> Self {
        Self {
            regions: Vec::new(),
            #[cfg(test)]
            fixture: OnceLock::new(),
            #[cfg(test)]
            use_fixture: false,
        }
    }

    pub(crate) fn from_region(region: brep::Region) -> Self {
        Self {
            regions: vec![region],
            ..Self::empty()
        }
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
        #[cfg(test)]
        if let Some(solids) = self.test_override() {
            return solids.is_empty();
        }
        self.shells().next().is_none()
    }

    fn build_proof_solids(&self) -> Result<Vec<Solid>, String> {
        let mut solids = Vec::new();
        for region in &self.regions {
            solids.extend(proof_geometry::solids(region)?);
        }
        Ok(solids)
    }

    pub(crate) fn proof_solids(&self) -> Result<Cow<'_, [Solid]>, String> {
        // Proof mutation fixtures need stable edge identity across separate calls.
        #[cfg(test)]
        {
            Ok(Cow::Borrowed(self.test_solids()))
        }
        #[cfg(not(test))]
        {
            self.build_proof_solids().map(Cow::Owned)
        }
    }

    #[cfg(test)]
    pub(crate) fn test_solids(&self) -> &[Solid] {
        self.fixture
            .get_or_init(|| self.build_proof_solids().expect("proof topology oracle"))
    }

    #[cfg(test)]
    pub(crate) fn from_test_solids(solids: Vec<Solid>) -> Self {
        Self {
            fixture: OnceLock::from(solids),
            use_fixture: true,
            ..Self::empty()
        }
    }

    #[cfg(test)]
    pub(crate) fn test_override(&self) -> Option<&[Solid]> {
        self.use_fixture.then(|| self.test_solids())
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
    #[cfg(test)]
    if let Some(solids) = model.test_override() {
        let meshes = solids
            .iter()
            .flat_map(Solid::boundaries)
            .map(|shell| vendor_preview(shell, mesh_options(PROFILE_MESH_TOLERANCE_MM)))
            .collect::<Result<Vec<_>, _>>()?;
        return mesh_buffers(meshes.into_iter().flatten());
    }

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

#[cfg(test)]
fn vendor_preview(shell: &Shell, options: TessellationOptions) -> Result<Vec<PolygonMesh>, String> {
    shell_to_polygon_strict_with(shell, options)
        .map(|mesh| vec![mesh])
        .map_err(|error| format!("Monstertruck could not mesh case profile: {error:?}"))
}

#[cfg(test)]
fn mesh_options(tolerance: f64) -> TessellationOptions {
    TessellationOptions {
        tolerance,
        edge_angular_tolerance: Some(PROFILE_EDGE_ANGULAR_TOLERANCE_RAD),
        ..Default::default()
    }
}

/// Tessellates exact generated case/PCB solids into f64 analysis buffers with per-face
/// provenance. Requested tolerances do not certify source-to-mesh error.
pub fn mesh_profile_for_analysis(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
) -> Result<GeneratedAnalysisMesh, String> {
    emit_analysis(model, target_deflection_mm, None, &mut ())
}

/// Sees each accepted face and fan triangle of an analysis mesh as it is emitted.
pub(crate) trait EmissionObserver {
    fn face(
        &mut self,
        source: &GeneratedSourceFace,
        face: &Face,
        mesh_orientation: bool,
        polygon: &PolygonMesh,
    );
    fn triangle(
        &mut self,
        occurrence_index: usize,
        polygon: &PolygonMesh,
        vertices: [StandardVertex; 3],
        positions_mm_f64: &[f64],
        rendered_positions_f32: &[f32],
    );
}

/// The no-observer path.
impl EmissionObserver for () {
    fn face(&mut self, _: &GeneratedSourceFace, _: &Face, _: bool, _: &PolygonMesh) {}
    fn triangle(
        &mut self,
        _: usize,
        _: &PolygonMesh,
        _: [StandardVertex; 3],
        _: &[f64],
        _: &[f32],
    ) {
    }
}

#[cfg(test)]
pub(crate) fn mesh_profile_observed(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
    observer: &mut impl EmissionObserver,
) -> Result<GeneratedAnalysisMesh, String> {
    emit_analysis(
        model,
        target_deflection_mm,
        Some(model.test_solids()),
        observer,
    )
}

struct EmittedFace<'a> {
    support_kind: &'static str,
    triangles: FaceTriangles<'a>,
}

enum FaceTriangles<'a> {
    Native(tessellation::FaceMesh),
    Observed {
        face: &'a Face,
        forward: bool,
        polygon: PolygonMesh,
    },
}

pub(crate) fn emit_analysis(
    model: &ExactModel,
    target_deflection_mm: Option<f64>,
    proof_solids: Option<&[Solid]>,
    observer: &mut impl EmissionObserver,
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

    let mut emit_shell =
        |solid_index, shell_index, faces: Vec<EmittedFace<'_>>| -> Result<(bool, f64), String> {
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
                if let FaceTriangles::Observed {
                    face,
                    forward,
                    polygon,
                } = &triangles
                {
                    observer.face(&source_faces[occurrence_index], face, *forward, polygon);
                }
                let count_before = triangle_face_occurrence_indices.len();
                let mut emit_triangle = |points: [Point3; 3],
                                         proof: Option<(&PolygonMesh, [StandardVertex; 3])>|
                 -> Result<(), String> {
                    push_triangle(&mut mesh, points)?;
                    shell_triangles.push(points);
                    for point in points {
                        for (axis, coord) in [point.x, point.y, point.z].into_iter().enumerate() {
                            if !coord.is_finite() {
                                return Err(
                                    "Generated case produced a non-finite analysis coordinate"
                                        .into(),
                                );
                            }
                            bounds[0][axis] = bounds[0][axis].min(coord);
                            bounds[1][axis] = bounds[1][axis].max(coord);
                            positions_mm_f64.push(coord);
                        }
                    }
                    triangle_face_occurrence_indices.push(occurrence_index);
                    // Bind the observation to the accepted f64 and rendered f32 coordinates.
                    if let Some((polygon, triangle)) = proof {
                        observer.triangle(
                            occurrence_index,
                            polygon,
                            triangle,
                            &positions_mm_f64[positions_mm_f64.len() - 9..],
                            &mesh.positions[mesh.positions.len() - 9..],
                        );
                    }
                    Ok(())
                };
                match triangles {
                    FaceTriangles::Native(mesh) => {
                        for points in mesh.triangle_points() {
                            emit_triangle(points, None)?;
                        }
                    }
                    FaceTriangles::Observed { polygon, .. } => {
                        for triangle in polygon.faces().triangle_iter() {
                            let points = triangle.map(|v| polygon.positions()[v.pos]);
                            emit_triangle(points, Some((&polygon, triangle)))?;
                        }
                    }
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

    #[cfg(test)]
    let fixture = model.test_override();
    #[cfg(not(test))]
    let fixture: Option<&[Solid]> = None;
    if let Some(solids) = fixture {
        #[cfg(test)]
        for (solid_index, solid) in solids.iter().enumerate() {
            let mut volume = 0.;
            for (shell_index, shell) in solid.boundaries().iter().enumerate() {
                let polygons = vendor_analysis(shell, mesh_options(deflection_mm))?;
                let faces = shell
                    .face_iter()
                    .zip(polygons)
                    .map(|(face, (polygon, forward))| {
                        let support_kind = match face.surface() {
                            Surface::Plane(_) => "PLANE",
                            Surface::RevolutionSurface(_) => "REVOLUTION_SURFACE",
                            _ => "SURFACE",
                        };
                        EmittedFace {
                            support_kind,
                            triangles: FaceTriangles::Observed {
                                face,
                                forward,
                                polygon,
                            },
                        }
                    })
                    .collect();
                let (closed, signed_volume) = emit_shell(solid_index, shell_index, faces)?;
                mesh_closed_oriented &= closed;
                volume += signed_volume;
            }
            mesh_closed_oriented &= volume.is_finite() && volume > 0.;
        }
        #[cfg(not(test))]
        let _ = solids;
    } else {
        for (solid_index, (topology, shell)) in model.shells().enumerate() {
            let generated = tessellation::tessellate(
                topology,
                shell,
                deflection_mm,
                PROFILE_EDGE_ANGULAR_TOLERANCE_RAD,
            )?;
            let proof_faces = proof_solids
                .and_then(|solids| solids.get(solid_index))
                .and_then(|solid| solid.boundaries().first())
                .into_iter()
                .flat_map(Shell::face_iter)
                .map(Some)
                .chain(std::iter::repeat(None));
            let faces = shell
                .iter()
                .zip(generated)
                .zip(proof_faces)
                .map(|((face, mesh), proof_face)| EmittedFace {
                    support_kind: match face.support {
                        Support::Plane { .. } => "PLANE",
                        Support::Round { .. } => "REVOLUTION_SURFACE",
                    },
                    triangles: match proof_face {
                        Some(proof_face) => FaceTriangles::Observed {
                            face: proof_face,
                            forward: face.forward,
                            polygon: proof_geometry::polygon(mesh),
                        },
                        None => FaceTriangles::Native(mesh),
                    },
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

#[cfg(test)]
fn vendor_analysis(
    shell: &Shell,
    options: TessellationOptions,
) -> Result<Vec<(PolygonMesh, bool)>, String> {
    // Validate whole-shell strict tessellation first so edge/metric failures refuse.
    let _ = vendor_preview(shell, options)?;
    let meshed = triangulation_with(shell, options);
    if meshed.face_iter().count() != shell.face_iter().count() {
        return Err("Generated analysis dropped a source face".into());
    }
    meshed
        .face_iter()
        .enumerate()
        .map(|(index, face)| {
            let mut poly = face
                .surface()
                .ok_or_else(|| format!("Monstertruck dropped generated case face {index}"))?;
            if !face.orientation() {
                poly.invert();
            }
            Ok((poly, face.orientation()))
        })
        .collect()
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
