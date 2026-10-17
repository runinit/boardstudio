//! Complete vendor STEP assets with retained source and derived preview data.
mod bounds;
mod scene;
mod tessellation;
mod units;
mod wireframe;
pub use units::precision::{StepPrecision, StepUncertainty};
pub use wireframe::{StepLineOccurrence, StepLinePresentation};

use crate::MeshBuffers;
use monstertruck_io::step::load::step_p21::{
    ast::{EntityInstance, Exchange, Name, Parameter, Record},
    parser,
};
use monstertruck_io::step::load::{convert::*, *};
use monstertruck_meshing::prelude::*;
use monstertruck_modeling::{InnerSpace, Matrix4, Point3, SquareMatrix, Transform};
use std::collections::{HashMap, HashSet};

const MAX_STEP_BYTES: usize = 32 * 1024 * 1024;
const TOLERANCE_MM: f64 = 0.1;
/// Imported vendor STEP is preview/alignment geometry: deviations up to
/// `max(PREVIEW_DECLARED_MULTIPLIER * declared, PREVIEW_FLOOR_MM)` are accepted and recorded.
const PREVIEW_FLOOR_MM: f64 = 0.02;
const PREVIEW_DECLARED_MULTIPLIER: f64 = 2.0;

struct SourceSolid {
    source: HashMap<u64, SourceBoundStepSolid>,
    mm_per_unit: f64,
}

/// Definition geometry is always the immutable original source support.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSourceFace {
    pub solid_id: u64,
    pub shell_index: usize,
    pub face_id: u64,
    pub support_id: u64,
    pub support_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepFaceOccurrence {
    pub source_face: usize,
    pub occurrence: usize,
    pub representation_id: u64,
    pub source_uncertainty_mm: Option<f64>,
    pub repair_displacement_mm: f64,
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
}

/// Owns immutable source definitions, transforms, bytes and source-tagged preview data.
/// Completeness gates are separate from topology and analysis qualification.
pub struct ImportedStep {
    original_bytes: Vec<u8>,
    _original_source: Exchange,
    solids: HashMap<u64, SourceSolid>,
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
    pub fn original_bytes(&self) -> &[u8] {
        &self.original_bytes
    }
    pub fn bounds(&self) -> [[f64; 3]; 2] {
        self.bounds
    }
    pub fn solid_count(&self) -> usize {
        self.solids.len()
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
    import(bytes).map_err(|error| format!("STEP import failed: {error}"))
}

fn import(bytes: Vec<u8>) -> Result<ImportedStep, String> {
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err("invalid file size".into());
    }
    let text = units::decode_step_bytes(&bytes);
    let original_source = parser::parse(&text).map_err(|error| format!("source AST: {error}"))?;
    let precision = units::precision::resolve(&original_source)?;
    // The reviewed contextual producer currently supports simple declarations.
    // Other authenticated metadata remains diagnostic; strict construction must
    // bypass discovery entirely rather than borrowing an unrelated owner.
    let simple_uncertainties: HashSet<_> = original_source
        .data
        .iter()
        .flat_map(|data| &data.entities)
        .filter_map(|instance| match instance {
            EntityInstance::Simple { id, record }
                if record.name == "UNCERTAINTY_MEASURE_WITH_UNIT" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    let source_occurrence_ids = source_occurrence_ids(&text)?;
    let representation_units = units::resolve_mm_per_unit_by_representation(&text)?;
    let shape_items = units::shape_representation_items(&text)?;
    let exchange = units::normalize_conical_angles(&text)?;
    let mut table = Table::from_data_section(&exchange.data[0]);
    table.set_preview_tolerance(PreviewTolerance {
        floor_mm: PREVIEW_FLOOR_MM,
        declared_multiplier: PREVIEW_DECLARED_MULTIPLIER,
    });
    table
        .entity_report
        .require_empty()
        .map_err(|e| e.to_string())?;
    validate_pds_definition_targets(&table)?;
    let mut ids: Vec<_> = table.manifold_solid_brep.keys().copied().collect();
    ids.sort_unstable();
    let solid_units = units::solid_representation_units(&ids, &shape_items, &representation_units)?;
    let scene = scene::Scene::new(&exchange, &shape_items, &representation_units, &ids);
    scene.validate()?;
    let all_occurrences = occurrences(
        &table,
        &scene,
        &representation_units,
        &source_occurrence_ids,
    )?;
    let (occurrences, curve_occurrences): (Vec<_>, Vec<_>) = all_occurrences
        .into_iter()
        .partition(|(id, _, _)| table.manifold_solid_brep.contains_key(id));
    if occurrences.is_empty()
        || ids
            .iter()
            .any(|id| !occurrences.iter().any(|(placed, _, _)| placed == id))
    {
        return Err("empty or unreferenced exact geometry".into());
    }
    let mut solids = HashMap::<u64, SourceSolid>::new();
    for (id, _, representation) in &occurrences {
        let solid = solids.entry(*id).or_insert_with(|| SourceSolid {
            source: HashMap::new(),
            mm_per_unit: solid_units[id],
        });
        if !solid.source.contains_key(representation) {
            let owner = precision
                .iter()
                .find(|p| p.representation_id == *representation)
                .ok_or("source representation precision missing")?;
            let contextual = owner.geometric_dimension == Some(3)
                && matches!(owner.declarations.as_slice(), [declaration] if simple_uncertainties.contains(&declaration.entity_id));
            let source = if contextual {
                table.to_source_bound_solid_in_representation_checked(
                    &table.manifold_solid_brep[id],
                    *representation,
                )
            } else {
                table.to_source_bound_solid_without_connectivity_checked(
                    &table.manifold_solid_brep[id],
                )
            }
            .map_err(|error| format!("solid #{id} representation #{representation}: {error}"))?;
            if source.shells().is_empty() {
                return Err(format!("solid #{id} has no shells"));
            }
            solid.source.insert(*representation, source);
        }
    }
    let source_faces = source_face_catalogue(&original_source, &solids)?;
    let wireframe = wireframe::Wireframe::build(
        &original_source,
        &table,
        &curve_occurrences,
        &representation_units,
        &precision,
    )?;
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let mut bounds_from_source_geometry = true;
    let mut mesh_closed_oriented = true;
    let mut relaxed_beyond_declared_precision = false;
    let mut sampled_support_checks = 0;
    let mut fallback_meshed_faces = 0;
    let mut max_sampled_support_residual_mm: Option<f64> = None;
    let mut preview_positions_f64 = Vec::new();
    let mut face_occurrences = Vec::new();
    let mut triangle_faces = Vec::new();
    let mut mesh = MeshBuffers {
        positions: Vec::new(),
        normals: Vec::new(),
    };
    for (occurrence, (id, placement, representation)) in occurrences.iter().enumerate() {
        units::validate_placement_matrix(*placement)?;
        let solid = &solids[id];
        let mut signed_volume = 0.0;
        for (shell_index, source) in solid.source[representation].shells().iter().enumerate() {
            require_complete_source_shell(source)?;
            let owner = precision
                .iter()
                .find(|p| p.representation_id == *representation)
                .ok_or("source support representation precision missing")?;
            let support = bounds::validate_support(source, solid.mm_per_unit, *placement, owner)?;
            relaxed_beyond_declared_precision |= support.relaxed_beyond_declared_precision;
            sampled_support_checks += support.sampled_checks;
            max_sampled_support_residual_mm = max_sampled_support_residual_mm
                .into_iter()
                .chain(support.max_sampled_residual_mm)
                .reduce(f64::max);
            match bounds::shell_bounds(source, solid.mm_per_unit, *placement) {
                Ok(exact_bounds) => {
                    for axis in 0..3 {
                        bounds[0][axis] = bounds[0][axis].min(exact_bounds[0][axis]);
                        bounds[1][axis] = bounds[1][axis].max(exact_bounds[1][axis]);
                    }
                    if exact_bounds
                        .iter()
                        .flatten()
                        .any(|v| !(*v as f32).is_finite() || (v * 1e7).abs() >= i64::MAX as f64)
                    {
                        return Err("unsupported mapped mesh coordinate range".into());
                    }
                }
                Err(_) => bounds_from_source_geometry = false,
            }
            let placement_scale = placement
                .x
                .truncate()
                .magnitude()
                .max(placement.y.truncate().magnitude())
                .max(placement.z.truncate().magnitude());
            let (triangulation, fallback_faces) =
                tessellation::triangulate_source_bound_shell_report(
                    source,
                    solid.mm_per_unit,
                    monstertruck_meshing::tessellation::TessellationOptions {
                        tolerance: TOLERANCE_MM / placement_scale,
                        edge_angular_tolerance: Some(0.5),
                        preview_fallback: true,
                        ..Default::default()
                    },
                )?;
            fallback_meshed_faces += fallback_faces.len();
            let mut triangles = Vec::new();
            if triangulation.faces.is_empty() {
                return Err(format!("solid #{id} has an empty shell"));
            }
            if triangulation.faces.len() != source.source_face_ids().len() {
                return Err("meshed source face count mismatch".into());
            }
            for (face_index, face) in triangulation.faces.iter().enumerate() {
                let before = triangles.len();
                append_face_triangles(
                    face.surface.as_ref(),
                    face.orientation,
                    placement,
                    &mut triangles,
                )
                .ok_or_else(|| format!("solid #{id} has an unmeshed face"))?;
                let fallback = fallback_faces.contains(&face_index);
                if fallback {
                    // A face finer than the f32 render grid (a sliver narrower than the
                    // preview deflection) keeps its catalogue entry but renders nothing.
                    let mut tail = triangles.split_off(before);
                    tail.retain(|tri| {
                        crate::mesh::push_triangle(&mut MeshBuffers::default(), *tri).is_ok()
                    });
                    triangles.extend(tail);
                }
                if before == triangles.len() && !fallback {
                    return Err(format!("solid #{id} has an empty triangle face"));
                }
                let source_face = source_faces
                    .iter()
                    .position(|f| {
                        f.solid_id == *id
                            && f.shell_index == shell_index
                            && f.face_id == source.source_face_ids()[face_index]
                    })
                    .ok_or("source face catalogue mismatch")?;
                let uncertainty = precision
                    .iter()
                    .find(|p| p.representation_id == *representation)
                    .and_then(StepPrecision::uncertainty_mm)
                    .map(|value| value * placement_scale);
                if uncertainty.is_some_and(|value| !value.is_finite()) {
                    return Err("nonfinite placed source precision".into());
                }
                let tag = face_occurrences.len();
                face_occurrences.push(StepFaceOccurrence {
                    source_face,
                    occurrence,
                    representation_id: *representation,
                    source_uncertainty_mm: uncertainty,
                    repair_displacement_mm: 0.,
                });
                triangle_faces.extend(std::iter::repeat_n(tag, triangles.len() - before));
            }
            // Check in world millimetres, matching the qualified import policy.
            if triangles.iter().flatten().any(|p| {
                [p.x, p.y, p.z].iter().any(|v| {
                    !v.is_finite() || !(*v as f32).is_finite() || (v * 1e7).abs() >= i64::MAX as f64
                })
            }) {
                return Err("nonfinite or unsupported mesh coordinate range".into());
            }
            mesh_closed_oriented &= closed_oriented_mesh(&triangles);
            // A shell-local origin avoids cancellation under large occurrence translations.
            let reference = triangles[0][0];
            for [a, b, c] in triangles {
                signed_volume += (a - reference).dot((b - reference).cross(c - reference)) / 6.0;
                crate::mesh::push_triangle(&mut mesh, [a, b, c])?;
                for p in [a, b, c] {
                    preview_positions_f64.push([p.x, p.y, p.z]);
                }
            }
        }
        mesh_closed_oriented &= signed_volume.is_finite() && signed_volume > 0.;
    }
    if !bounds_from_source_geometry {
        bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for point in &preview_positions_f64 {
            for axis in 0..3 {
                // Enclose both f64 preview and the actual f32 render projection.
                bounds[0][axis] = bounds[0][axis]
                    .min(point[axis])
                    .min(f64::from(point[axis] as f32));
                bounds[1][axis] = bounds[1][axis]
                    .max(point[axis])
                    .max(f64::from(point[axis] as f32));
            }
        }
    }
    wireframe.include_bounds(&mut bounds);
    if bounds.iter().flatten().any(|v| !v.is_finite())
        || (0..3).any(|i| bounds[0][i] > bounds[1][i])
    {
        return Err("invalid bounds".into());
    }
    Ok(ImportedStep {
        original_bytes: bytes,
        _original_source: original_source,
        solids,
        occurrences,
        bounds,
        mesh,
        preview_positions_f64,
        source_faces,
        face_occurrences,
        triangle_faces,
        precision,
        accuracy: StepAccuracy {
            preview_deflection_mm: TOLERANCE_MM,
            preview_angular_rad: 0.5,
            analysis_error_bound_mm: None,
            bounds_from_source_geometry,
            mesh_closed_oriented,
            preview_floor_mm: PREVIEW_FLOOR_MM,
            preview_declared_multiplier: PREVIEW_DECLARED_MULTIPLIER,
            relaxed_beyond_declared_precision,
            sampled_support_checks,
            max_sampled_support_residual_mm,
            fallback_meshed_faces,
        },
        wireframe,
    })
}

fn require_complete_source_shell(source: &SourceBoundStepShell) -> Result<(), String> {
    let shell = source.shell();
    let mut uses = vec![0usize; shell.edges.len()];
    if shell.faces.is_empty()
        || source
            .source_face_ids()
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len()
            != shell.faces.len()
    {
        return Err("empty or repeated source shell face".into());
    }
    for (face_index, face) in shell.faces.iter().enumerate() {
        if face.boundaries.is_empty() || face.boundaries.iter().any(Vec::is_empty) {
            return Err("source face has an empty boundary".into());
        }
        for boundary in &face.boundaries {
            let mut first_start = None;
            let mut previous_end = None;
            for edge_use in boundary {
                let edge = shell
                    .edges
                    .get(edge_use.index)
                    .ok_or("source boundary references a missing edge")?;
                let (start, end) = if edge_use.orientation {
                    edge.vertices
                } else {
                    (edge.vertices.1, edge.vertices.0)
                };
                if start >= shell.vertices.len() || end >= shell.vertices.len() {
                    return Err("source boundary references a missing vertex".into());
                }
                if previous_end.is_some_and(|previous| previous != start) {
                    return Err(format!(
                        "source face #{} has a disconnected or unordered boundary",
                        source.source_face_ids()[face_index]
                    ));
                }
                first_start.get_or_insert(start);
                previous_end = Some(end);
                uses[edge_use.index] += 1;
            }
            if previous_end != first_start {
                return Err(format!(
                    "source face #{} has an open boundary",
                    source.source_face_ids()[face_index]
                ));
            }
        }
    }
    if uses.is_empty() || uses.iter().any(|count| *count != 2) {
        return Err("incomplete or nonmanifold source shell edge uses".into());
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

fn source_face_catalogue(
    exchange: &Exchange,
    solids: &HashMap<u64, SourceSolid>,
) -> Result<Vec<StepSourceFace>, String> {
    let records = source_records(exchange);
    let mut ids: Vec<_> = solids.keys().copied().collect();
    ids.sort_unstable();
    let mut catalogue = Vec::new();
    for solid_id in ids {
        // Definition IDs only: actual construction/meshing uses its explicit owner.
        let owner = solids[&solid_id]
            .source
            .keys()
            .min()
            .ok_or("source definition has no owner")?;
        for (shell_index, shell) in solids[&solid_id].source[owner].shells().iter().enumerate() {
            for face_id in shell.source_face_ids() {
                let parts = records
                    .get(face_id)
                    .ok_or("original source face is missing")?;
                let face = parts
                    .iter()
                    .find(|record| matches!(record.name.as_str(), "ADVANCED_FACE" | "FACE_SURFACE"))
                    .ok_or("original source face has no support record")?;
                let Parameter::List(params) = &face.parameter else {
                    return Err("malformed original source face".into());
                };
                let index = if parts.len() == 1 { 2 } else { 0 };
                let Some(Parameter::Ref(Name::Entity(support_id))) = params.get(index) else {
                    return Err("original source face lacks support identity".into());
                };
                let support = records
                    .get(support_id)
                    .ok_or("original source support is missing")?;
                catalogue.push(StepSourceFace {
                    solid_id,
                    shell_index,
                    face_id: *face_id,
                    support_id: *support_id,
                    support_kind: support
                        .iter()
                        .map(|record| record.name.as_str())
                        .collect::<Vec<_>>()
                        .join("+"),
                });
            }
        }
    }
    Ok(catalogue)
}

fn occurrences(
    table: &Table,
    scene: &scene::Scene<'_>,
    representation_units: &HashMap<u64, f64>,
    source_ids: &HashSet<u64>,
) -> Result<Vec<(u64, Matrix4, u64)>, String> {
    let assembly = table.step_assy().map_err(|e| format!("assembly: {e}"))?;
    let standalone = assembly.is_empty()
        && table.product_definition_shape.is_empty()
        && table.next_assembly_usage_occurrence.is_empty();
    if assembly.is_empty() && !standalone {
        return Err("assembly graph absent for a non-standalone STEP model".into());
    }
    validate_occurrence_identity(
        source_ids,
        &table
            .next_assembly_usage_occurrence
            .keys()
            .copied()
            .collect(),
        &occurrence_product_definition_shape_ids(table),
        &occurrence_context_dependent_shape_links(table),
    )?;
    let mut instances = Vec::new();
    let mut traversed = HashSet::new();
    let mut seen_mappings = HashSet::new();
    if standalone {
        for root in scene.roots()? {
            instances.extend(scene.instances(root, Matrix4::identity(), &mut seen_mappings)?);
        }
    } else {
        for top in assembly.top_nodes() {
            for path in assembly.paths_iter(top.index()) {
                let nodes = path.nodes();
                if nodes.len() != path.edges().len() + 1 {
                    return Err("assembly path node/edge sequence is inconsistent".into());
                }
                let mut transform = Matrix4::identity();
                for (index, edge) in path.edges().iter().enumerate() {
                    traversed.insert(std::ptr::from_ref(edge.entity()) as usize);
                    let factor = |index: usize| -> Result<f64, String> {
                        let representation = table
                            .shape_representation_of_node(nodes[index].entity())
                            .ok_or("assembly node lacks a shape representation")?;
                        representation_units
                            .get(&representation)
                            .copied()
                            .ok_or_else(|| {
                                format!("representation #{representation} lacks a resolved unit")
                            })
                    };
                    transform = transform
                        * units::normalized_placement_mm(
                            edge.matrix(),
                            factor(index + 1)?,
                            factor(index)?,
                        )?;
                }
                let representation = table
                    .shape_representation_of_node(path.terminal_node().entity())
                    .ok_or("assembly node lacks a shape representation")?;
                instances.extend(scene.instances(representation, transform, &mut seen_mappings)?);
            }
        }
    }
    let edges = assembly
        .node_indices()
        .map(|index| assembly.node(index).edges().count())
        .sum();
    validate_assembly_completeness(source_ids.len(), edges, traversed.len())?;
    scene.require_complete_mappings(&seen_mappings)?;
    Ok(instances)
}

fn closed_oriented_mesh(triangles: &[[Point3; 3]]) -> bool {
    let point = |p: Point3| {
        (
            (p.x * 1e7).round() as i64,
            (p.y * 1e7).round() as i64,
            (p.z * 1e7).round() as i64,
        )
    };
    let mut counts = HashMap::new();
    for tri in triangles {
        for i in 0..3 {
            let (a, b) = (point(tri[i]), point(tri[(i + 1) % 3]));
            if a == b {
                return false;
            }
            let (key, direction) = if a < b { ((a, b), 1i64) } else { ((b, a), -1) };
            let (count, orientation) = counts.entry(key).or_insert((0, 0));
            *count += 1;
            *orientation += direction;
        }
    }
    !counts.is_empty()
        && counts
            .values()
            .all(|&(count, orientation)| count == 2 && orientation == 0)
}

fn append_face_triangles(
    surface: Option<&PolygonMesh>,
    orientation: bool,
    matrix: &Matrix4,
    triangles: &mut Vec<[Point3; 3]>,
) -> Option<usize> {
    let mesh = surface?;
    let initial_count = triangles.len();
    let positions = mesh.positions();
    for polygon in mesh.face_iter() {
        for k in 1..polygon.len().saturating_sub(1) {
            let mut tri =
                [polygon[0].pos, polygon[k].pos, polygon[k + 1].pos].map(|i| positions[i]);
            // A mirrored occurrence reverses the boundary's handedness.
            if !orientation ^ (matrix.determinant() < 0.) {
                tri.swap(1, 2);
            }
            triangles.push(tri.map(|p| matrix.transform_point(p)));
        }
    }
    Some(triangles.len() - initial_count)
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

fn validate_pds_definition_targets(table: &Table) -> Result<(), String> {
    for (&pds_id, pds) in &table.product_definition_shape {
        if let monstertruck_io::step::load::step_p21::tables::PlaceHolder::Ref(
            monstertruck_io::step::load::step_p21::ast::Name::Entity(definition_id),
        ) = &pds.definition
        {
            let target_exists = table.product_definition.contains_key(definition_id)
                || table
                    .next_assembly_usage_occurrence
                    .contains_key(definition_id)
                || table.product_definition_shape.contains_key(definition_id);
            if !target_exists {
                return Err(format!(
                    "incomplete assembly import: PRODUCT_DEFINITION_SHAPE #{pds_id} references missing definition entity #{definition_id}"
                ));
            }
        }
    }
    Ok(())
}

fn source_occurrence_ids(text: &str) -> Result<HashSet<u64>, String> {
    let exchange = parser::parse(text).map_err(|error| format!("STEP AST: {error}"))?;
    Ok(exchange
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
        .collect())
}

fn occurrence_product_definition_shape_ids(table: &Table) -> HashMap<u64, Vec<u64>> {
    let mut links = HashMap::<u64, Vec<u64>>::new();
    for (&pds_id, pds) in &table.product_definition_shape {
        match &pds.definition {
            monstertruck_io::step::load::step_p21::tables::PlaceHolder::Ref(
                monstertruck_io::step::load::step_p21::ast::Name::Entity(id),
            ) if table.next_assembly_usage_occurrence.contains_key(id) => {
                links.entry(*id).or_default().push(pds_id);
            }
            _ => {}
        }
    }
    links
}

fn occurrence_context_dependent_shape_links(table: &Table) -> HashMap<u64, usize> {
    let mut links = HashMap::new();
    for cdsr in table.context_dependent_shape_representation.values() {
        if let monstertruck_io::step::load::step_p21::tables::PlaceHolder::Ref(
            monstertruck_io::step::load::step_p21::ast::Name::Entity(pds_id),
        ) = &cdsr.represented_product_relation
        {
            *links.entry(*pds_id).or_default() += 1;
        }
    }
    links
}

fn validate_occurrence_identity(
    source_ids: &HashSet<u64>,
    typed_ids: &HashSet<u64>,
    pds_ids_by_nauo: &HashMap<u64, Vec<u64>>,
    cdsr_links_by_pds: &HashMap<u64, usize>,
) -> Result<(), String> {
    if source_ids != typed_ids {
        let source_missing: Vec<_> = source_ids.difference(typed_ids).copied().collect();
        let table_extra: Vec<_> = typed_ids.difference(source_ids).copied().collect();
        return Err(format!(
            "incomplete assembly import: source/typed NAUO entity IDs differ (missing typed={source_missing:?}, extra typed={table_extra:?})"
        ));
    }
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
