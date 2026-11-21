//! Complete vendor STEP assets with retained source and derived preview data.
mod assembly;
mod cadmpeg;
mod cuboid;
mod scene;
mod units;
mod wireframe;
pub use units::precision::{StepPrecision, StepUncertainty};
pub use wireframe::{StepLineOccurrence, StepLinePresentation};

use crate::MeshBuffers;
use crate::layered::brep::{Point3, Vector3};
use cgmath::{InnerSpace, SquareMatrix, Transform};
use std::collections::{HashMap, HashSet};
use step_p21::{
    ast::{EntityInstance, Exchange, Name, Parameter, Record},
    parser,
};

type Matrix4 = cgmath::Matrix4<f64>;

const MAX_STEP_BYTES: usize = 32 * 1024 * 1024;
const TOLERANCE_MM: f64 = 0.1;
/// Imported vendor STEP is preview/alignment geometry: deviations up to
/// `max(PREVIEW_DECLARED_MULTIPLIER * declared, PREVIEW_FLOOR_MM)` are accepted and recorded.
const PREVIEW_FLOOR_MM: f64 = 0.02;
const PREVIEW_DECLARED_MULTIPLIER: f64 = 2.0;

/// Definition geometry is always the immutable original source support.
#[derive(Clone, Debug, PartialEq)]
pub struct StepSourceFace {
    pub solid_id: u64,
    pub shell_index: usize,
    pub face_id: u64,
    pub support_id: u64,
    pub support_kind: String,
    /// RGBA components of the effective face or body color in the source.
    pub color: Option<[f32; 4]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepFaceOccurrence {
    pub source_face: usize,
    pub occurrence: usize,
    pub representation_id: u64,
    pub source_uncertainty_mm: Option<f64>,
    pub repair_displacement_mm: f64,
}

/// Options controlling vendor STEP import and optional on-demand mesh refinement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StepImportOptions {
    pub preview_repair_budget_mm: Option<f64>,
    pub target_deflection_mm: Option<f64>,
}

fn repair_allowance(budget: Option<f64>, uncertainty: Option<f64>, placement: Matrix4) -> f64 {
    let scale = placement
        .x
        .truncate()
        .magnitude()
        .max(placement.y.truncate().magnitude())
        .max(placement.z.truncate().magnitude());
    budget.unwrap_or_else(|| {
        (PREVIEW_DECLARED_MULTIPLIER * uncertainty.unwrap_or(0.0) * scale).max(PREVIEW_FLOOR_MM)
    })
}

/// Requested preview settings are not a certified analysis error bound.
#[derive(Clone, Debug, PartialEq)]
pub struct StepAccuracy {
    pub preview_deflection_mm: f64,
    pub preview_angular_rad: f64,
    pub analysis_error_bound_mm: Option<f64>,
    pub bounds_from_source_geometry: bool,
    pub mesh_closed_oriented: bool,
    /// Preview floor applied to source attachment bounds, in millimetres.
    pub preview_floor_mm: f64,
    /// Factor applied to the declared precision before the floor, so the effective
    /// attachment bound is `max(preview_declared_multiplier * declared, preview_floor_mm)`.
    pub preview_declared_multiplier: f64,
    /// Explicit per-file preview repair budget in millimetres, when supplied.
    pub selected_repair_budget_mm: Option<f64>,
    /// Unknown until the complete geometric repair path measures actual movement.
    pub max_repair_displacement_mm: Option<f64>,
    /// True when a measured attachment discrepancy or sampled support residual exceeded
    /// the declared precision, so a check passed only because of the preview allowance.
    /// Direction and endpoint-trim relaxations are not observable per check and are not
    /// reflected here.
    pub relaxed_beyond_declared_precision: bool,
    /// Boundary curves whose support incidence was accepted by sampling because exact
    /// certification was unsupported or inconclusive (preview geometry, not certified).
    pub sampled_support_checks: usize,
    /// Largest sampled boundary-to-support distance among those curves, in millimetres.
    pub max_sampled_support_residual_mm: Option<f64>,
    /// Faces the ordinary mesher left without triangles that were meshed from their shared
    /// boundary samples instead. Preview geometry within the preview deflection; the
    /// boundary vertices are the shared edge samples.
    pub fallback_meshed_faces: usize,
    /// Fallback triangles kept in the exact f64 analysis geometry whose f32 render projection
    /// collapses or reverses (a face finer than the f32 render grid at its placement). The render
    /// buffer carries the projected points, so these triangles draw as nothing or as a sliver of
    /// at most one render-grid step; the analysis buffers keep the full source coverage.
    pub render_degraded_fallback_triangles: usize,
    /// STEP triangles lost within the f32 coordinate rounding envelope.
    pub discarded_display_triangles: usize,
}

/// Owns immutable source definitions, transforms, bytes and source-tagged preview data.
/// Completeness gates are separate from topology and analysis qualification.
pub struct ImportedStep {
    original_bytes: Vec<u8>,
    analysis_options: StepImportOptions,
    _original_source: Exchange,
    solid_count: usize,
    occurrences: Vec<(u64, Matrix4, u64)>,
    bounds: [[f64; 3]; 2],
    mesh: MeshBuffers,
    preview_positions_f64: Vec<[f64; 3]>,
    source_faces: Vec<StepSourceFace>,
    face_occurrences: Vec<StepFaceOccurrence>,
    triangle_faces: Vec<usize>,
    precision: Vec<StepPrecision>,
    accuracy: StepAccuracy,
    wireframe: wireframe::Wireframe,
}

impl ImportedStep {
    /// The options this analysis mesh was imported with.
    pub(crate) fn analysis_options(&self) -> StepImportOptions {
        self.analysis_options
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.original_bytes
    }
    pub fn bounds(&self) -> [[f64; 3]; 2] {
        self.bounds
    }
    pub fn solid_count(&self) -> usize {
        self.solid_count
    }
    pub fn occurrence_count(&self) -> usize {
        self.occurrences.len()
    }
    /// Surface triangles only; non-surface curves require line-capable delivery.
    pub fn mesh(&self) -> &MeshBuffers {
        &self.mesh
    }
    pub fn preview_positions_f64(&self) -> &[[f64; 3]] {
        &self.preview_positions_f64
    }
    pub fn source_faces(&self) -> &[StepSourceFace] {
        &self.source_faces
    }
    pub fn face_occurrences(&self) -> &[StepFaceOccurrence] {
        &self.face_occurrences
    }
    /// One face-occurrence index per triangle; definition geometry stays shared.
    pub fn triangle_faces(&self) -> &[usize] {
        &self.triangle_faces
    }
    pub fn precision(&self) -> &[StepPrecision] {
        &self.precision
    }
    pub fn accuracy(&self) -> &StepAccuracy {
        &self.accuracy
    }

    /// Retained finite source lines, separate from the surface-only preview mesh.
    pub fn line_presentation(&self) -> StepLinePresentation<'_> {
        self.wireframe.presentation()
    }

    /// Post-import invariant suite. Returns Ok(()) if all cheap invariants hold,
    /// or a list of failure reasons. These are deterministic checks that catch
    /// incomplete or corrupt imports without per-route accounting.
    pub fn validate_invariants(&self) -> Result<(), Vec<String>> {
        let mut failures = Vec::new();

        // 1. Mesh is nonempty.
        if self.mesh.positions.is_empty() {
            failures.push("mesh has no positions".to_string());
        }

        // 2. No NaN or Inf vertices.
        for (i, v) in self.mesh.positions.iter().enumerate() {
            if !v.is_finite() {
                if failures.len() >= 10 {
                    failures.push("... (truncated)".to_string());
                    break;
                }
                failures.push(format!("mesh position {i} is not finite: {v}"));
            }
        }

        // 3. Bounds are finite.
        for (axis, range) in self.bounds.iter().enumerate() {
            for (i, v) in range.iter().enumerate() {
                if !v.is_finite() {
                    failures.push(format!("bounds[{axis}][{i}] is not finite: {v}"));
                }
            }
        }

        // 4. Source faces nonempty.
        if self.source_faces.is_empty() {
            failures.push("no source faces retained".to_string());
        }

        // 5. Triangle-face consistency: one face index per triangle.
        let triangle_count = self.mesh.positions.len() / 9;
        if !self.triangle_faces.is_empty() && self.triangle_faces.len() != triangle_count {
            failures.push(format!(
                "triangle_faces count ({}) != triangle count ({})",
                self.triangle_faces.len(),
                triangle_count
            ));
        }

        // 6. Bounds non-degenerate (min < max on at least one axis).
        let mut degenerate = true;
        for axis in 0..3 {
            if self.bounds[1][axis] > self.bounds[0][axis] {
                degenerate = false;
                break;
            }
        }
        if degenerate && !self.mesh.positions.is_empty() {
            failures.push("bounds are degenerate (zero volume)".to_string());
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }
}

pub fn mesh_imported_step(model: &ImportedStep) -> Result<MeshBuffers, String> {
    if !model.wireframe.is_empty() {
        return Err("STEP source curves require line-capable presentation".into());
    }
    Ok(MeshBuffers {
        positions: model.mesh.positions.clone(),
        normals: model.mesh.normals.clone(),
    })
}

pub fn import_step(bytes: Vec<u8>) -> Result<ImportedStep, String> {
    import_step_with_options(bytes, StepImportOptions::default())
}

pub fn import_step_with_options(
    bytes: Vec<u8>,
    options: StepImportOptions,
) -> Result<ImportedStep, String> {
    import(bytes, options).map_err(|error| format!("STEP import failed: {error}"))
}

fn import(bytes: Vec<u8>, options: StepImportOptions) -> Result<ImportedStep, String> {
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err("invalid file size".into());
    }
    if let Some(budget) = options.preview_repair_budget_mm
        && (!budget.is_finite() || budget <= 0.0)
    {
        return Err("previewRepairBudgetMm must be finite and positive".into());
    }
    let deflection_mm = match options.target_deflection_mm {
        Some(target) => {
            if !target.is_finite() || target <= 0.0 {
                return Err("targetDeflectionMm must be finite and positive".into());
            }
            target.min(TOLERANCE_MM)
        }
        None => TOLERANCE_MM,
    };
    let exchange = parser::parse(&units::decode_step_bytes(&bytes))
        .map_err(|error| format!("source AST: {error}"))?;
    if exchange.data.len() != 1 {
        return Err("STEP input requires exactly one DATA section".into());
    }
    let precision = units::precision::resolve(&exchange)?;
    let representation_units = units::length_factors(&exchange)?;
    let shape_items = units::representation_items(&exchange)?;
    cadmpeg::build(
        bytes,
        exchange,
        shape_items,
        representation_units,
        precision,
        options,
        deflection_mm,
    )
}

/// A fallback triangle carries source coverage only when its f64 cross product has at least one
/// representably nonzero component. Checking components (rather than the squared magnitude)
/// prevents a representably nonzero cross product of ~1e-200 from being treated as zero.
fn fallback_triangle_has_representable_area(points: [Point3; 3]) -> bool {
    let cross = (points[1] - points[0]).cross(points[2] - points[0]);
    cross.x != 0.0 || cross.y != 0.0 || cross.z != 0.0
}

/// Appends the f32 projection of a positive-area fallback triangle whose render orientation does
/// not survive the projection. The normal is the exact f64 normal, so the render buffer remains
/// the f32 image of the analysis geometry. Ordinary faces keep the strict refusal.
fn push_render_degraded_triangle(
    buffers: &mut MeshBuffers,
    points: [Point3; 3],
) -> Result<(), String> {
    let normal = (points[1] - points[0]).cross(points[2] - points[0]);
    let length = normal.x.hypot(normal.y).hypot(normal.z);
    if !length.is_finite() || length == 0.0 {
        return Err("fallback triangle lost its f64 area".into());
    }
    let normal = Point3::new(normal.x / length, normal.y / length, normal.z / length);
    if !normal.x.is_finite() || !normal.y.is_finite() || !normal.z.is_finite() {
        return Err("fallback triangle lost its f64 area".into());
    }
    for point in points {
        crate::mesh::append_point(&mut buffers.positions, point)
            .map_err(|_| "fallback triangle is outside the render range".to_string())?;
        crate::mesh::append_point(&mut buffers.normals, normal)
            .map_err(|_| "fallback triangle is outside the render range".to_string())?;
    }
    Ok(())
}

type SourceRecords<'a> = HashMap<u64, Vec<&'a Record>>;

fn source_records(exchange: &Exchange) -> SourceRecords<'_> {
    exchange.data[0]
        .entities
        .iter()
        .map(|entity| match entity {
            EntityInstance::Simple { id, record } => (*id, vec![record]),
            EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.iter().collect()),
        })
        .collect()
}

fn simple_record<'a>(records: &SourceRecords<'a>, id: u64) -> Result<&'a Record, String> {
    let records = records
        .get(&id)
        .ok_or_else(|| format!("missing representation item #{id}"))?;
    match records.as_slice() {
        [record] => Ok(record),
        _ => Err(format!("unsupported complex representation item #{id}")),
    }
}

fn named_record<'a>(
    records: &SourceRecords<'a>,
    id: u64,
    name: &str,
) -> Result<&'a Record, String> {
    let record = simple_record(records, id)?;
    if record.name != name {
        return Err(format!("entity #{id} is {}, expected {name}", record.name));
    }
    Ok(record)
}

fn parameters(record: &Record, count: usize) -> Result<&[Parameter], String> {
    match &record.parameter {
        Parameter::List(p) if p.len() == count => Ok(p),
        _ => Err(format!("malformed {} parameters", record.name)),
    }
}

fn reference(parameter: &Parameter) -> Result<u64, String> {
    match parameter {
        Parameter::Ref(Name::Entity(id)) => Ok(*id),
        _ => Err("expected STEP entity reference".into()),
    }
}

// Admission limits per import, shared across all roots and assembly paths.
// A traversal step is one representation visit, item visit, assembly path or edge.
// Occurrences count both solids and curve sets, before geometry is materialized.
const MAX_OCCURRENCE_TRAVERSAL_STEPS: usize = 16_384;
const MAX_STEP_OCCURRENCES: usize = 1_024;

#[derive(Default)]
struct OccurrenceBudget {
    traversal_steps: usize,
    occurrences: usize,
    seen_mappings: HashSet<u64>,
}

impl OccurrenceBudget {
    fn traverse(&mut self) -> Result<(), String> {
        if self.traversal_steps == MAX_OCCURRENCE_TRAVERSAL_STEPS {
            return Err(format!(
                "STEP traversal limit exceeded ({MAX_OCCURRENCE_TRAVERSAL_STEPS} steps)"
            ));
        }
        self.traversal_steps += 1;
        Ok(())
    }

    fn emit(&mut self) -> Result<(), String> {
        if self.occurrences == MAX_STEP_OCCURRENCES {
            return Err(format!(
                "STEP occurrence limit exceeded ({MAX_STEP_OCCURRENCES} occurrences)"
            ));
        }
        self.occurrences += 1;
        Ok(())
    }
}

fn closed_oriented_mesh(triangles: &[[Point3; 3]]) -> bool {
    // Preserve the imported STEP preview's existing 1e7 rounded coordinate policy.
    let point = |p: Point3| {
        (
            (p.x * 1e7).round() as i64,
            (p.y * 1e7).round() as i64,
            (p.z * 1e7).round() as i64,
        )
    };
    crate::mesh::closed_oriented_edges(triangles, |p| Some(point(p)))
}

fn validate_assembly_completeness(
    declared_occurrences: usize,
    decoded_edges: usize,
    traversed_edges: usize,
) -> Result<(), String> {
    if declared_occurrences != decoded_edges {
        return Err(format!(
            "incomplete assembly import: {declared_occurrences} NEXT_ASSEMBLY_USAGE_OCCURRENCE records but {decoded_edges} decoded graph edges"
        ));
    }
    if decoded_edges != traversed_edges {
        return Err(format!(
            "incomplete assembly import: {decoded_edges} decoded graph edges but only {traversed_edges} occur in assembly paths"
        ));
    }
    Ok(())
}

fn source_occurrence_ids(exchange: &Exchange) -> HashSet<u64> {
    exchange
        .data
        .iter()
        .flat_map(|section| &section.entities)
        .filter_map(|entity| match entity {
            EntityInstance::Simple { id, record } => {
                (record.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE").then_some(*id)
            }
            EntityInstance::Complex { id, subsuper } => subsuper
                .0
                .iter()
                .any(|record: &Record| record.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE")
                .then_some(*id),
        })
        .collect()
}

fn validate_occurrence_identity(
    source_ids: &HashSet<u64>,
    pds_ids_by_nauo: &HashMap<u64, Vec<u64>>,
    cdsr_links_by_pds: &HashMap<u64, usize>,
) -> Result<(), String> {
    for id in source_ids {
        let pds_ids = pds_ids_by_nauo
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_default();
        if pds_ids.len() != 1 {
            return Err(format!(
                "incomplete assembly import: NEXT_ASSEMBLY_USAGE_OCCURRENCE #{id} has {} PRODUCT_DEFINITION_SHAPE definition links; exactly one is supported",
                pds_ids.len()
            ));
        }
        let cdsr_links = cdsr_links_by_pds.get(&pds_ids[0]).copied().unwrap_or(0);
        if cdsr_links != 1 {
            return Err(format!(
                "incomplete assembly import: occurrence PRODUCT_DEFINITION_SHAPE #{} has {cdsr_links} CONTEXT_DEPENDENT_SHAPE_REPRESENTATION links; exactly one is supported",
                pds_ids[0]
            ));
        }
    }
    if pds_ids_by_nauo.keys().any(|id| !source_ids.contains(id)) {
        return Err(
            "incomplete assembly import: typed PDS links reference a non-source NAUO".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod numerical_underflow_tests {
    use super::*;

    #[test]
    fn tiny_cross_product_retains_real_fallback_append() {
        let mut buffers = crate::mesh::MeshBuffers::default();
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1e-100, 0.0, 0.0);
        let c = Point3::new(0.0, 1e-100, 0.0);
        // Real production append path: must succeed despite the squared magnitude
        // underflowing to zero, and must emit a finite unit normal along +Z.
        push_render_degraded_triangle(&mut buffers, [a, b, c])
            .expect("representably nonzero fallback triangle must not be refused");
        assert_eq!(buffers.positions.len(), 9);
        assert_eq!(buffers.normals.len(), 9);
        for value in buffers.positions.iter().chain(&buffers.normals) {
            assert!(value.is_finite(), "all emitted coordinates must be finite");
        }
        // Positions are the f32 casts of the (subnormal) inputs.
        for (index, coordinate) in [a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z]
            .iter()
            .enumerate()
        {
            assert_eq!(buffers.positions[index], *coordinate as f32);
        }
        // Unit normal points along +Z.
        for vertex in 0..3 {
            assert!((buffers.normals[vertex * 3] - 0.0).abs() < 1e-6);
            assert!((buffers.normals[vertex * 3 + 1] - 0.0).abs() < 1e-6);
            assert!((buffers.normals[vertex * 3 + 2] - 1.0).abs() < 1e-6);
        }
        // The predicate remains a useful filter guard.
        assert!(fallback_triangle_has_representable_area([a, b, c]));
    }

    #[test]
    fn genuinely_degenerate_triangle_is_refused() {
        let mut buffers = crate::mesh::MeshBuffers::default();
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(2.0, 0.0, 0.0);
        assert!(!fallback_triangle_has_representable_area([a, b, c]));
        assert!(push_render_degraded_triangle(&mut buffers, [a, b, c]).is_err());
    }
}

#[cfg(test)]
mod invariant_tests;

#[cfg(test)]
mod surface_preview_tests;
