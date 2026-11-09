//! Qualification adapter for source-bound STEP meshing.
//!
//! Geometry and all meshing tolerances use the existing millimetre-normalized
//! representation. Closure is re-proved against each exact normalized surface.
//! This returns a meshing attempt, not whole-shell or assembly qualification.
use monstertruck_io::step::load::convert::{SourceBoundStepShell, StepCompressedShell};
use monstertruck_meshing::{
    prelude::*,
    tessellation::{TessellationOptions, robust_cshell_triangulation_with_closed_axis_metadata},
};
use monstertruck_modeling::{Point3, PolylineCurve as GeometryPolyline};
use monstertruck_topology::compress::CompressedShell;

// The full gate also uses these shared parsing and placement helpers.
#[allow(dead_code)]
#[path = "unit_normalization.rs"]
mod unit_normalization;

pub type StepMesh = CompressedShell<Point3, GeometryPolyline<Point3>, Option<PolygonMesh>>;

type ClosedChartMetadata = Vec<(usize, usize, (f64, f64), Vec<usize>)>;

#[derive(Debug)]
pub struct SourceCertifiedStepMesh {
    pub mesh: StepMesh,
    pub source_face_ids: Vec<u64>,
    /// Source FACE_SURFACE IDs whose closed-chart route was selected.
    pub certified_faces: Vec<u64>,
}

/// Normalizes a private clone using the checked importer's existing unit rules.
/// Certificates are bound to those actual normalized surfaces before meshing.
/// `mm_per_unit` is the factor resolved for this solid's representation; all
/// spatial options are millimetres. The caller applies occurrence placement once.
pub fn triangulate_source_bound_shell(
    source: &SourceBoundStepShell,
    mm_per_unit: f64,
    options: TessellationOptions,
) -> Result<SourceCertifiedStepMesh, String> {
    if !mm_per_unit.is_finite() || mm_per_unit <= 0.0 {
        return Err("source representation unit must be finite and positive".into());
    }
    if !options.tolerance.is_finite()
        || options.tolerance <= 0.0
        || options
            .edge_angular_tolerance
            .is_some_and(|a| !a.is_finite() || a <= 0.0)
    {
        return Err("meshing tolerances must be finite and positive".into());
    }
    let mut normalized = source.shell().clone();
    unit_normalization::normalize_shell_to_mm(&mut normalized, mm_per_unit);
    let finite = |p: &Point3| p.x.is_finite() && p.y.is_finite() && p.z.is_finite();
    if !normalized.vertices.iter().all(finite) {
        return Err("unit normalization produced nonfinite source vertices".into());
    }
    // Raw tuple metadata exists only here, after exact binding and re-certification.
    let metadata = closed_chart_metadata(source, &normalized, mm_per_unit)?;
    let mesh =
        robust_cshell_triangulation_with_closed_axis_metadata(&normalized, options, &metadata);
    if !mesh.vertices.iter().all(finite)
        || !mesh.edges.iter().all(|e| e.curve.iter().all(finite))
        || !mesh.faces.iter().all(|f| {
            f.surface
                .as_ref()
                .is_none_or(|m| m.positions().iter().all(finite))
        })
    {
        return Err("meshing produced nonfinite normalized coordinates".into());
    }
    Ok(SourceCertifiedStepMesh {
        mesh,
        source_face_ids: source.source_face_ids().to_vec(),
        certified_faces: metadata
            .iter()
            .map(|(index, _, _, _)| source.source_face_ids()[*index])
            .collect(),
    })
}

// Metadata never crosses the checked adapter's public interface.
fn closed_chart_metadata(
    source: &SourceBoundStepShell,
    normalized: &StepCompressedShell,
    mm_per_unit: f64,
) -> Result<ClosedChartMetadata, String> {
    if normalized.faces.len() != source.source_face_ids().len() {
        return Err("unit normalization changed source face mapping".into());
    }
    let mut metadata = Vec::new();
    for (index, face) in normalized.faces.iter().enumerate() {
        if face.boundaries.len() != 1 {
            continue;
        }
        let axis = match source.certified_closed_axes()[index] {
            Some([true, false]) => 0,
            Some([false, true]) => 1,
            _ => continue,
        };
        let rebound = source
            .certified_closed_axes_after_uniform_scale(index, &face.surface, mm_per_unit)
            .ok_or("normalized surface does not match its source-bound uniform scale")?;
        if !rebound[axis] {
            return Err("closed-axis equality did not survive unit normalization".into());
        }
        let range = face.surface.try_range_tuple();
        let domain =
            [range.0, range.1][axis].ok_or("certified closed axis has no finite domain")?;
        let mut uses = std::collections::BTreeMap::<usize, Vec<bool>>::new();
        for edge in face.boundaries.iter().flatten() {
            uses.entry(edge.index).or_default().push(edge.orientation);
        }
        let seams = uses
            .into_iter()
            .filter_map(|(edge, orientations)| {
                (orientations.len() == 2 && orientations[0] != orientations[1]).then_some(edge)
            })
            .collect::<Vec<_>>();
        if !seams.is_empty() {
            metadata.push((index, axis, domain, seams));
        }
    }
    Ok(metadata)
}

#[cfg(test)]
mod tests;
