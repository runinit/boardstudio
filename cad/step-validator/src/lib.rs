//! Test-only STEP reimport oracle.
//!
//! Reads STEP with Monstertruck's own parser and B-rep kernel, so it shares
//! neither a reader nor a geometry kernel with the production Cadrum/OCCT path.
//! Deliberately not used in production or WASM builds.
//!
//! The report answers the questions the retired libcascade harness answered
//! (read success, solid count, volume, bounds) plus an explicit B-rep
//! validity verdict: every shell must extract as a topologically closed,
//! oriented shell, the tessellation must be watertight, the enclosed volume
//! must be positive and nothing may be silently dropped while converting.
//!
//! Monstertruck's loader ignores declared units, so a minimal declared
//! length-unit scan lives here and its factor scales every reported length.

use monstertruck_assembly::assy::*;
use monstertruck_io::step::load::{convert::*, *};
use monstertruck_meshing::prelude::*;
use monstertruck_topology::compress::CompressedShell;
use std::collections::HashMap;

/// Absolute chord tolerance (model units, before unit scaling) used for tessellation.
pub const DEFAULT_CHORD_TOLERANCE: f64 = 0.0005;

#[derive(Debug, Clone)]
pub struct StepReport {
    /// Every check below passed. `problems` explains a false verdict.
    pub valid: bool,
    /// Solid instances, counted once per placement (assembly instancing included).
    pub solid_count: usize,
    pub shell_count: usize,
    /// Enclosed volume in mm³ (signed sum of shell volumes; negative means inside-out).
    pub volume: f64,
    /// Bounds in mm, taken from the tessellation of the placed shells.
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// Millimetres per declared length unit that scaled the file.
    pub length_unit_mm: f64,
    /// Chord tolerance (mm) of the tessellation that produced the volume and bounds.
    pub chord_tolerance_mm: f64,
    /// Edges shared by more than two face uses (balanced, so still closed and oriented).
    pub non_manifold_edges: usize,
    /// Number of tessellation edges not paired with an opposite half-edge.
    pub open_mesh_edges: usize,
    /// Entities the loader could not convert, as `category: reason` strings.
    pub lost: Vec<String>,
    pub problems: Vec<String>,
}

#[derive(Debug)]
pub enum StepError {
    /// The bytes are not a parseable STEP exchange structure.
    Syntax(String),
    /// Declared units could not be understood.
    Units(String),
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(message) => write!(f, "STEP syntax error: {message}"),
            Self::Units(message) => write!(f, "STEP unit error: {message}"),
        }
    }
}

impl std::error::Error for StepError {}

pub fn inspect(bytes: &[u8]) -> Result<StepReport, StepError> {
    inspect_with(bytes, DEFAULT_CHORD_TOLERANCE)
}

type Matrix = monstertruck_core::cgmath64::Matrix4;
type Point = monstertruck_core::cgmath64::Point3;

struct Placed {
    triangles: Vec<[Point; 3]>,
}

/// Monstertruck's tessellation occasionally leaves a handful of seam edges unmatched at one
/// particular tolerance (observed: the TrackPoint vendor file at 0.0005 mm, clean at 0.002 and
/// 0.0001). Retry a few nearby tolerances before calling a mesh open, and report the one used.
pub fn inspect_with(bytes: &[u8], chord_tolerance: f64) -> Result<StepReport, StepError> {
    let mut report = inspect_once(bytes, chord_tolerance)?;
    for factor in [0.8, 0.6, 0.4, 0.25] {
        if report.open_mesh_edges == 0 {
            break;
        }
        report = inspect_once(bytes, chord_tolerance * factor)?;
    }
    Ok(report)
}

fn inspect_once(bytes: &[u8], chord_tolerance: f64) -> Result<StepReport, StepError> {
    let text = String::from_utf8_lossy(bytes);
    let unit = declared_length_unit_mm(&text)?;
    let table =
        Table::from_step_bytes(bytes).map_err(|error| StepError::Syntax(error.to_string()))?;
    let mut problems = Vec::new();
    let mut lost = Vec::new();

    let entity_report = &table.entity_report;
    if let Err(error) = entity_report.require_empty() {
        lost.push(format!("entities: {error}"));
    }

    // One tessellated solid per MANIFOLD_SOLID_BREP, then one placed copy per
    // assembly instance (or a single identity copy for a bare file).
    let mut shell_count = 0usize;
    let mut non_manifold_edges = 0usize;
    let mut meshed: HashMap<u64, Vec<Vec<[Point; 3]>>> = HashMap::new();
    let mut ids: Vec<u64> = table.manifold_solid_brep.keys().copied().collect();
    ids.sort_unstable();
    for id in &ids {
        let holder = &table.manifold_solid_brep[id];
        let (solid, reports) = match table.to_compressed_solid_reported(holder) {
            Ok(value) => value,
            Err(error) => {
                problems.push(format!("solid #{id} did not convert: {error}"));
                continue;
            }
        };
        for report in &reports {
            if let Err(error) = report.clone().require_lossless() {
                lost.push(format!("solid #{id}: {error}"));
            }
        }
        let mut shells = Vec::new();
        for (index, boundary) in solid.boundaries.iter().enumerate() {
            shell_count += 1;
            let (messages, non_manifold) = half_edge_problems(boundary);
            non_manifold_edges += non_manifold;
            for message in messages {
                problems.push(format!("solid #{id} shell {index}: {message}"));
            }
            let meshed_shell = boundary.robust_triangulation((chord_tolerance / unit).max(2.0e-6));
            let mut triangles = Vec::new();
            for face in &meshed_shell.faces {
                let Some(mesh) = face.surface.as_ref() else {
                    problems.push(format!(
                        "solid #{id} shell {index} has a face that did not tessellate"
                    ));
                    continue;
                };
                let positions = mesh.positions();
                for polygon in mesh.face_iter() {
                    for k in 1..polygon.len().saturating_sub(1) {
                        let mut triangle = [
                            positions[polygon[0].pos],
                            positions[polygon[k].pos],
                            positions[polygon[k + 1].pos],
                        ];
                        if !face.orientation {
                            triangle.swap(1, 2);
                        }
                        triangles.push(triangle);
                    }
                }
            }
            shells.push(triangles);
        }
        meshed.insert(*id, shells);
    }

    // Placement: walk the assembly like Monstertruck's own step-to-mesh example.
    let mut instances: Vec<(u64, Matrix)> = Vec::new();
    if let Ok(assembly) = table.step_assy() {
        let placed = assembly.par_map(
            |ProductEntity { shape, .. }: &ProductEntity| NodeEntity {
                shape: shape.clone(),
                attrs: (),
            },
            |EdgeEntity { matrix, .. }: &AssembleEntity| EdgeEntity {
                matrix: Matrix::try_from(matrix).unwrap_or_else(|_| {
                    <Matrix as monstertruck_core::cgmath64::SquareMatrix>::identity()
                }),
                attrs: (),
            },
        );
        for top in placed.top_nodes() {
            for path in placed.paths_iter(top.index()) {
                let matrix = path.matrix();
                for index in path.terminal_node().shape() {
                    if meshed.contains_key(index) {
                        instances.push((*index, matrix));
                    }
                }
            }
        }
    }
    if instances.is_empty() {
        instances = ids
            .iter()
            .filter(|id| meshed.contains_key(id))
            .map(|id| {
                (
                    *id,
                    <Matrix as monstertruck_core::cgmath64::SquareMatrix>::identity(),
                )
            })
            .collect();
    }
    let placed_ids: std::collections::HashSet<u64> = instances.iter().map(|(id, _)| *id).collect();
    for id in &ids {
        if meshed.contains_key(id) && !placed_ids.contains(id) {
            problems.push(format!(
                "solid #{id} is not reachable from the assembly graph"
            ));
        }
    }

    let mut volume = 0.0;
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut open_mesh_edges = 0usize;
    let mut solid_count = 0usize;
    for (id, matrix) in &instances {
        solid_count += 1;
        let mut solid_volume = 0.0;
        for shell in &meshed[id] {
            let placed = place(shell, matrix, unit);
            solid_volume += signed_volume(&placed.triangles);
            open_mesh_edges += unpaired_edges(&placed.triangles);
            for triangle in &placed.triangles {
                for p in triangle {
                    for (axis, v) in [p.x, p.y, p.z].into_iter().enumerate() {
                        min[axis] = min[axis].min(v);
                        max[axis] = max[axis].max(v);
                    }
                }
            }
        }
        if solid_volume <= 0.0 {
            problems.push(format!(
                "solid #{id} has non-positive volume {solid_volume}"
            ));
        }
        volume += solid_volume;
    }
    if solid_count == 0 {
        problems.push("no solids".to_string());
        min = [0.0; 3];
        max = [0.0; 3];
    }
    if open_mesh_edges > 0 {
        problems.push(format!(
            "{open_mesh_edges} tessellation edges are not shared by two oppositely oriented faces"
        ));
    }
    if !lost.is_empty() {
        problems.push("conversion dropped entities".to_string());
    }
    Ok(StepReport {
        valid: problems.is_empty(),
        solid_count,
        shell_count,
        volume,
        min,
        max,
        length_unit_mm: unit,
        chord_tolerance_mm: (chord_tolerance / unit).max(2.0e-6) * unit,
        non_manifold_edges,
        open_mesh_edges,
        lost,
        problems,
    })
}

/// B-rep closed-and-oriented check on the converted shell: every edge must be
/// used exactly twice, once in each direction once face orientation is applied.
/// Done on the compressed form because Monstertruck's own `Shell` refuses edges
/// whose two ends are the same vertex (full circles), which valid STEP contains.
fn half_edge_problems<C, S>(shell: &CompressedShell<Point, C, S>) -> (Vec<String>, usize) {
    let mut uses = vec![(0usize, 0usize); shell.edges.len()];
    let mut problems = Vec::new();
    for face in &shell.faces {
        for boundary in &face.boundaries {
            for edge in boundary {
                let Some(entry) = uses.get_mut(edge.index) else {
                    problems.push(format!("face references missing edge {}", edge.index));
                    continue;
                };
                if edge.orientation == face.orientation {
                    entry.0 += 1;
                } else {
                    entry.1 += 1;
                }
            }
        }
    }
    if std::env::var_os("STEP_VALIDATOR_DEBUG").is_some() {
        for (index, (forward, backward)) in uses.iter().enumerate() {
            if (*forward, *backward) != (1, 1) {
                eprintln!(
                    "edge {index}: forward {forward} backward {backward} vertices {:?}",
                    shell.edges[index].vertices
                );
            }
        }
    }
    let bad = uses
        .iter()
        .filter(|(forward, backward)| forward != backward)
        .count();
    let unused = uses.iter().filter(|u| **u == (0, 0)).count();
    if bad > 0 {
        problems.push(format!("{bad} edges are not used exactly once in each direction (shell is open, non-manifold or inconsistently oriented)"));
    }
    if unused > 0 {
        problems.push(format!("{unused} edges belong to no face"));
    }
    let non_manifold = uses
        .iter()
        .filter(|(forward, backward)| forward == backward && *forward > 1)
        .count();
    (problems, non_manifold)
}

fn place(triangles: &[[Point; 3]], matrix: &Matrix, unit: f64) -> Placed {
    use monstertruck_core::cgmath64::Transform;
    Placed {
        triangles: triangles
            .iter()
            .map(|t| {
                [0, 1, 2].map(|i| {
                    let p = matrix.transform_point(t[i]);
                    Point::new(p.x * unit, p.y * unit, p.z * unit)
                })
            })
            .collect(),
    }
}

fn signed_volume(triangles: &[[Point; 3]]) -> f64 {
    triangles
        .iter()
        .map(|[a, b, c]| {
            (a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x)
                + a.z * (b.x * c.y - b.y * c.x))
                / 6.0
        })
        .sum()
}

/// Counts half-edges lacking an opposite twin. Vertices are matched by position
/// snapped to 1e-7 mm, because adjacent faces share the same edge polyline.
fn unpaired_edges(triangles: &[[Point; 3]]) -> usize {
    let key = |p: &Point| {
        let q = |v: f64| (v * 1.0e7).round() as i64;
        (q(p.x), q(p.y), q(p.z))
    };
    let mut balance: HashMap<((i64, i64, i64), (i64, i64, i64)), i64> = HashMap::new();
    for t in triangles {
        for i in 0..3 {
            let (a, b) = (key(&t[i]), key(&t[(i + 1) % 3]));
            if a == b {
                continue;
            }
            if a < b {
                *balance.entry((a, b)).or_default() += 1;
            } else {
                *balance.entry((b, a)).or_default() -= 1;
            }
        }
    }
    balance.values().filter(|count| **count != 0).count()
}

/// Millimetres per declared length unit. Handles `SI_UNIT` (any prefix, metre)
/// and `CONVERSION_BASED_UNIT` whose measure is itself an `SI_UNIT`-based length.
/// A file declaring several different length units is rejected rather than guessed.
pub fn declared_length_unit_mm(text: &str) -> Result<f64, StepError> {
    // Whitespace is insignificant outside strings, and exporters disagree on it.
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let entities: HashMap<u64, &str> = compact
        .split(';')
        .filter_map(|statement| {
            let statement = statement.trim_start();
            let rest = statement.strip_prefix('#')?;
            let (id, body) = rest.split_once('=')?;
            Some((id.trim().parse().ok()?, body.trim()))
        })
        .collect();
    // Only units a representation context actually assigns count; exporters
    // routinely declare unused extras (e.g. a bare metre beside millimetres).
    let mut assigned: Vec<u64> = Vec::new();
    for body in entities
        .values()
        .filter(|b| b.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT("))
    {
        let list = body
            .split("GLOBAL_UNIT_ASSIGNED_CONTEXT(")
            .nth(1)
            .unwrap_or("");
        let list = list.split("))").next().unwrap_or("");
        assigned.extend(list.split(',').filter_map(|item| {
            item.trim_matches(|c| c == '(' || c == ')' || c == '#')
                .parse::<u64>()
                .ok()
        }));
    }
    let mut factors = Vec::new();
    for (id, body) in &entities {
        if body.contains("LENGTH_UNIT") && (assigned.is_empty() || assigned.contains(id)) {
            factors.push(unit_factor(body, &entities, 0)?);
        }
    }
    match factors.as_slice() {
        [] => Ok(1.0),
        [only] => Ok(*only),
        many if many.windows(2).all(|w| (w[0] - w[1]).abs() < 1e-12) => Ok(many[0]),
        many => Err(StepError::Units(format!(
            "conflicting length units {many:?}"
        ))),
    }
}

fn unit_factor(body: &str, entities: &HashMap<u64, &str>, depth: usize) -> Result<f64, StepError> {
    if depth > 4 {
        return Err(StepError::Units("unit chain too deep".into()));
    }
    if let Some(si) = body.find("SI_UNIT(") {
        let args = &body[si + "SI_UNIT(".len()..];
        let args = args.split(')').next().unwrap_or("");
        let (prefix, name) = args
            .split_once(',')
            .ok_or_else(|| StepError::Units("malformed SI_UNIT".into()))?;
        if name.trim() != ".METRE." {
            return Err(StepError::Units(format!("unsupported length unit {name}")));
        }
        return Ok(match prefix.trim() {
            ".MILLI." => 1.0,
            ".CENTI." => 10.0,
            ".DECI." => 100.0,
            "$" | "*" => 1000.0,
            ".KILO." => 1.0e6,
            ".MICRO." => 1.0e-3,
            other => return Err(StepError::Units(format!("unsupported SI prefix {other}"))),
        });
    }
    if let Some(cb) = body.find("CONVERSION_BASED_UNIT(") {
        let args = &body[cb + "CONVERSION_BASED_UNIT(".len()..];
        let reference = args
            .split('#')
            .nth(1)
            .and_then(|tail| tail.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|id| id.parse::<u64>().ok())
            .ok_or_else(|| StepError::Units("CONVERSION_BASED_UNIT without a measure".into()))?;
        let measure = entities
            .get(&reference)
            .ok_or_else(|| StepError::Units("dangling conversion measure".into()))?;
        let value: f64 = measure
            .split("LENGTH_MEASURE(")
            .nth(1)
            .and_then(|tail| tail.split(')').next())
            .and_then(|number| number.trim().parse().ok())
            .ok_or_else(|| StepError::Units("conversion measure is not a length".into()))?;
        let target = measure
            .rsplit('#')
            .next()
            .and_then(|tail| tail.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|id| id.parse::<u64>().ok())
            .and_then(|id| entities.get(&id))
            .ok_or_else(|| StepError::Units("conversion measure without a unit".into()))?;
        return Ok(value * unit_factor(target, entities, depth + 1)?);
    }
    Err(StepError::Units(format!(
        "unsupported length unit declaration: {body}"
    )))
}
