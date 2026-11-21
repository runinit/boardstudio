//! Product-structure occurrences read directly from the original Part 21 records.
//!
//! Each PRODUCT_DEFINITION with a PRODUCT_DEFINITION_SHAPE is a node, each
//! NEXT_ASSEMBLY_USAGE_OCCURRENCE an edge placed by its context-dependent
//! ITEM_DEFINED_TRANSFORMATION. Every path prefix from a top node is visited in
//! source-id order, so the occurrence order is deterministic.
use super::*;
use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3};

const MIN_DIRECTION_SINE: f64 = 1e-12;

struct Node {
    representation: u64,
    edges: Vec<usize>,
    parents: usize,
}

struct Edge {
    child: usize,
    /// ITEM_DEFINED_TRANSFORMATION frames: (transform_item_1, transform_item_2).
    frames: (u64, u64),
}

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

fn entities_named<'a>(records: &'a SourceRecords<'_>, name: &str) -> Vec<(u64, &'a Record)> {
    let mut found: Vec<_> = records
        .iter()
        .filter_map(|(id, parts)| {
            parts
                .iter()
                .find(|record| record.name == name)
                .map(|record| (*id, *record))
        })
        .collect();
    found.sort_unstable_by_key(|(id, _)| *id);
    found
}

fn has_record(records: &SourceRecords<'_>, id: u64, name: &str) -> bool {
    records
        .get(&id)
        .is_some_and(|parts| parts.iter().any(|record| record.name == name))
}

fn list(parameter: &Parameter) -> Option<&[Parameter]> {
    match parameter {
        Parameter::List(values) => Some(values),
        _ => None,
    }
}

fn param(record: &Record, index: usize) -> Result<&Parameter, String> {
    list(&record.parameter)
        .and_then(|values| values.get(index))
        .ok_or_else(|| format!("malformed {} parameters", record.name))
}

/// Solids are the MANIFOLD_SOLID_BREP instances, in source-id order.
pub(super) fn solids(records: &SourceRecords<'_>) -> Vec<u64> {
    entities_named(records, "MANIFOLD_SOLID_BREP")
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

pub(super) fn curve_sets(records: &SourceRecords<'_>) -> Vec<u64> {
    entities_named(records, "GEOMETRIC_CURVE_SET")
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

pub(super) fn validate_pds_definition_targets(records: &SourceRecords<'_>) -> Result<(), String> {
    for (pds_id, pds) in entities_named(records, "PRODUCT_DEFINITION_SHAPE") {
        if let Ok(definition_id) = reference(param(pds, 2)?) {
            let target_exists = [
                "PRODUCT_DEFINITION",
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
                "PRODUCT_DEFINITION_SHAPE",
            ]
            .iter()
            .any(|name| has_record(records, definition_id, name));
            if !target_exists {
                return Err(format!(
                    "incomplete assembly import: PRODUCT_DEFINITION_SHAPE #{pds_id} references missing definition entity #{definition_id}"
                ));
            }
        }
    }
    Ok(())
}

fn graph(records: &SourceRecords<'_>, items: &HashMap<u64, Vec<u64>>) -> Result<Graph, String> {
    let definitions = entities_named(records, "SHAPE_DEFINITION_REPRESENTATION");
    let contexts = entities_named(records, "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION");
    let mut nodes = Vec::new();
    let mut node_of = HashMap::new();
    let mut usages = Vec::new();
    for (pds_id, pds) in entities_named(records, "PRODUCT_DEFINITION_SHAPE") {
        let definition = reference(param(pds, 2)?)
            .map_err(|_| "assembly: failed to reference `product_definition_shape.definition`")?;
        if has_record(records, definition, "PRODUCT_DEFINITION") {
            let representation = definitions
                .iter()
                .find(|(_, sdr)| {
                    param(sdr, 0)
                        .ok()
                        .and_then(|p| reference(p).ok())
                        .is_some_and(|id| id == pds_id)
                })
                .ok_or("assembly: failed to find `shape_definition_representation` corresp. to `product_definition_shape`")
                .and_then(|(_, sdr)| {
                    param(sdr, 1)
                        .ok()
                        .and_then(|p| reference(p).ok())
                        .ok_or("assembly: failed to reference `shape_definition_representation.used_representation`")
                })?;
            if !items.contains_key(&representation) {
                return Err("assembly: failed to reference `shape_representation`".into());
            }
            node_of.insert(definition, nodes.len());
            nodes.push(Node {
                representation,
                edges: Vec::new(),
                parents: 0,
            });
        } else if let Some(usage) = records.get(&definition).and_then(|parts| {
            parts
                .iter()
                .find(|record| record.name == "NEXT_ASSEMBLY_USAGE_OCCURRENCE")
        }) {
            let parent = reference(param(usage, 3)?)
                .map_err(|_| "assembly: failed to reference the parent node")?;
            let child = reference(param(usage, 4)?)
                .map_err(|_| "assembly: failed to reference the child node")?;
            let relation = contexts
                .iter()
                .find(|(_, cdsr)| {
                    param(cdsr, 1)
                        .ok()
                        .and_then(|p| reference(p).ok())
                        .is_some_and(|id| id == pds_id)
                })
                .ok_or("assembly: missing context-dependent shape representation")
                .and_then(|(_, cdsr)| {
                    param(cdsr, 0).ok().and_then(|p| reference(p).ok()).ok_or(
                        "assembly: failed to reference `context_dependent_shape_representation.representation_relation`",
                    )
                })?;
            let transformation = records
                .get(&relation)
                .and_then(|parts| {
                    parts.iter().find(|record| {
                        record.name == "REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION"
                    })
                })
                .ok_or("assembly: failed to reference `shape_representation_relationship`")?;
            let operator = reference(param(transformation, 0)?)?;
            let idt = named_record(records, operator, "ITEM_DEFINED_TRANSFORMATION")
                .map_err(|error| format!("assembly: {error}"))?;
            let frames = (reference(param(idt, 2)?)?, reference(param(idt, 3)?)?);
            usages.push((parent, child, frames));
        }
    }
    let mut edges = Vec::new();
    for (parent, child, frames) in usages {
        let (Some(&from), Some(&to)) = (node_of.get(&parent), node_of.get(&child)) else {
            return Err("assembly: failed to reference `product_definition_shape`.".into());
        };
        nodes[from].edges.push(edges.len());
        nodes[to].parents += 1;
        edges.push(Edge { child: to, frames });
    }
    let mut graph = Graph { nodes, edges };
    graph.require_acyclic()?;
    Ok(graph)
}

impl Graph {
    fn require_acyclic(&mut self) -> Result<(), String> {
        // 0 unvisited, 1 on the current path, 2 finished.
        let mut state = vec![0u8; self.nodes.len()];
        for start in 0..self.nodes.len() {
            if state[start] != 0 {
                continue;
            }
            let mut stack = vec![(start, 0usize)];
            state[start] = 1;
            while let Some((node, next)) = stack.last_mut() {
                if let Some(&edge) = self.nodes[*node].edges.get(*next) {
                    *next += 1;
                    let child = self.edges[edge].child;
                    match state[child] {
                        0 => {
                            state[child] = 1;
                            stack.push((child, 0));
                        }
                        1 => return Err("assembly: maybe the assembly graph has a cycle.".into()),
                        _ => {}
                    }
                } else {
                    state[*node] = 2;
                    stack.pop();
                }
            }
        }
        Ok(())
    }
}

fn vector(records: &SourceRecords<'_>, id: u64, kind: &str) -> Result<Vector3<f64>, String> {
    let record = named_record(records, id, kind)?;
    let values = list(param(record, 1)?).ok_or_else(|| format!("{kind} #{id} lacks values"))?;
    let [x, y, z] = values else {
        return Err(format!("{kind} #{id} is not three-dimensional"));
    };
    let real = |p: &Parameter| match p {
        Parameter::Real(v) => Ok(*v),
        Parameter::Integer(v) => Ok(*v as f64),
        _ => Err(format!("{kind} #{id} has a non-numeric value")),
    };
    Ok(Vector3::new(real(x)?, real(y)?, real(z)?))
}

fn normalize_direction(direction: Vector3<f64>) -> Vector3<f64> {
    let scale = direction
        .x
        .abs()
        .max(direction.y.abs())
        .max(direction.z.abs());
    if direction.x.is_finite() && direction.y.is_finite() && direction.z.is_finite() && scale > 0.0
    {
        (direction / scale).normalize()
    } else {
        direction
    }
}

/// The AXIS2_PLACEMENT_3D frame with its origin in millimetres. Directions are
/// checked as dimensionless sines after scale-safe normalization.
fn frame(records: &SourceRecords<'_>, id: u64, mm_per_unit: f64) -> Result<Matrix4<f64>, String> {
    let record = named_record(records, id, "AXIS2_PLACEMENT_3D")?;
    let origin = vector(records, reference(param(record, 1)?)?, "CARTESIAN_POINT")?;
    let direction = |index: usize| -> Result<Option<Vector3<f64>>, String> {
        match param(record, index)? {
            Parameter::NotProvided => Ok(None),
            p => {
                let d = vector(records, reference(p)?, "DIRECTION")?;
                if !d.x.is_finite()
                    || !d.y.is_finite()
                    || !d.z.is_finite()
                    || d == Vector3::new(0.0, 0.0, 0.0)
                {
                    return Err("axis placement has a zero or non-finite direction".into());
                }
                Ok(Some(normalize_direction(d)))
            }
        }
    };
    let z = direction(2)?.unwrap_or_else(Vector3::unit_z);
    let reference_direction = direction(3)?;
    if let Some(x) = reference_direction
        && z.cross(x).magnitude() <= MIN_DIRECTION_SINE
    {
        return Err("axis and reference directions are parallel or near-parallel".into());
    }
    // STEP defaults ref_direction to +X, except when axis is exactly ±X.
    let fallback = if z == Vector3::unit_x() || z == -Vector3::unit_x() {
        Vector3::unit_y()
    } else {
        Vector3::unit_x()
    };
    let x = reference_direction.unwrap_or(fallback);
    let projected = x - x.dot(z) * z;
    let x = if projected.magnitude2() <= MIN_DIRECTION_SINE * MIN_DIRECTION_SINE {
        (fallback - fallback.dot(z) * z).normalize()
    } else {
        projected.normalize()
    };
    let origin = origin * mm_per_unit;
    Ok(Matrix4::from_cols(
        x.extend(0.0),
        z.cross(x).extend(0.0),
        z.extend(0.0),
        origin.extend(1.0),
    ))
}

/// Child-to-parent placement in millimetres: each frame origin is converted
/// with its own representation's length factor; directions are unitless.
fn placement(
    records: &SourceRecords<'_>,
    frames: (u64, u64),
    child_mm_per_unit: f64,
    parent_mm_per_unit: f64,
) -> Result<Matrix4<f64>, String> {
    if ![child_mm_per_unit, parent_mm_per_unit]
        .iter()
        .all(|f| f.is_finite() && *f > 0.0)
    {
        return Err("normalized placement: invalid representation length factor".into());
    }
    let child = frame(records, frames.0, child_mm_per_unit)
        .map_err(|error| format!("normalized placement: {error}"))?;
    let parent = frame(records, frames.1, parent_mm_per_unit)
        .map_err(|error| format!("normalized placement: {error}"))?;
    let inverse = child
        .invert()
        .ok_or("normalized placement: failed to invert normalized child placement frame")?;
    let matrix = parent * inverse;
    units::validate_placement_matrix(matrix)
        .map_err(|error| format!("normalized placement: {error}"))?;
    Ok(matrix)
}

fn occurrence_pds_links(records: &SourceRecords<'_>) -> Result<HashMap<u64, Vec<u64>>, String> {
    let mut links = HashMap::<u64, Vec<u64>>::new();
    for (pds_id, pds) in entities_named(records, "PRODUCT_DEFINITION_SHAPE") {
        if let Ok(id) = reference(param(pds, 2)?)
            && has_record(records, id, "NEXT_ASSEMBLY_USAGE_OCCURRENCE")
        {
            links.entry(id).or_default().push(pds_id);
        }
    }
    Ok(links)
}

fn context_links(records: &SourceRecords<'_>) -> HashMap<u64, usize> {
    let mut links = HashMap::new();
    for (_, cdsr) in entities_named(records, "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION") {
        if let Some(pds_id) = param(cdsr, 1).ok().and_then(|p| reference(p).ok()) {
            *links.entry(pds_id).or_default() += 1;
        }
    }
    links
}

pub(super) fn occurrences(
    records: &SourceRecords<'_>,
    scene: &scene::Scene<'_>,
    items: &HashMap<u64, Vec<u64>>,
    representation_units: &HashMap<u64, f64>,
    source_ids: &HashSet<u64>,
) -> Result<Vec<(u64, Matrix4<f64>, u64)>, String> {
    let graph = graph(records, items)?;
    let standalone = graph.nodes.is_empty()
        && entities_named(records, "PRODUCT_DEFINITION_SHAPE").is_empty()
        && source_ids.is_empty();
    if graph.nodes.is_empty() && !standalone {
        return Err("assembly graph absent for a non-standalone STEP model".into());
    }
    validate_occurrence_identity(
        source_ids,
        &occurrence_pds_links(records)?,
        &context_links(records),
    )?;
    let mut instances = Vec::new();
    let mut traversed = HashSet::new();
    let mut budget = OccurrenceBudget::default();
    if standalone {
        for root in scene.roots()? {
            instances.extend(scene.instances(root, Matrix4::identity(), &mut budget)?);
        }
    } else {
        let factor = |node: usize| -> Result<f64, String> {
            let representation = graph.nodes[node].representation;
            representation_units
                .get(&representation)
                .copied()
                .ok_or_else(|| format!("representation #{representation} lacks a resolved unit"))
        };
        for top in (0..graph.nodes.len()).filter(|node| graph.nodes[*node].parents == 0) {
            // Pre-order walk over every path prefix: (node, placement, path length).
            let mut stack = vec![(top, Matrix4::identity(), 0usize)];
            while let Some((node, transform, depth)) = stack.pop() {
                budget.traverse()?;
                for _ in 0..depth {
                    budget.traverse()?;
                }
                instances.extend(scene.instances(
                    graph.nodes[node].representation,
                    transform,
                    &mut budget,
                )?);
                for &edge in graph.nodes[node].edges.iter().rev() {
                    traversed.insert(edge);
                    let child = graph.edges[edge].child;
                    let next = transform
                        * placement(
                            records,
                            graph.edges[edge].frames,
                            factor(child)?,
                            factor(node)?,
                        )?;
                    stack.push((child, next, depth + 1));
                }
            }
        }
    }
    validate_assembly_completeness(source_ids.len(), graph.edges.len(), traversed.len())?;
    scene.require_complete_mappings(&budget.seen_mappings)?;
    Ok(instances)
}
