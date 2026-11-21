//! Reconcile shared vertices with their fixed planes before rebuilding straight trims.
use super::{geometry::Curve, *};
use repair::finite_line::{self, ExactPlacement, NativeSources};
use std::collections::BTreeMap;

pub(super) struct OccurrenceGeometry {
    pub world: Matrix4,
    pub epsilon: f64,
    pub world_epsilon: f64,
    pub allowance_mm: f64,
    pub uncertainty_mm: Option<f64>,
}

pub(super) struct SharedRepair {
    pub edges: HashMap<u64, Edge>,
    pub movement_mm: HashMap<u64, f64>,
}

struct Plan {
    points: HashMap<u64, Point3>,
    endpoints: HashMap<u64, (Point3, HashSet<u64>)>,
    edges: HashSet<u64>,
}

pub(super) struct Repairs {
    plans: HashMap<u64, Plan>,
    source: NativeSources,
    placements: HashMap<usize, ExactPlacement>,
}

impl Repairs {
    pub(super) fn prepare(
        solids: &HashMap<u64, Solid>,
        epsilon: &BTreeMap<u64, f64>,
        faces: &[StepSourceFace],
        records: &SourceRecords<'_>,
        bytes: &[u8],
        occurrences: &[scene::TracedInstance],
    ) -> Result<Option<Self>, String> {
        let mut plans = HashMap::new();
        let mut moved = HashMap::new();
        let mut lines = HashMap::new();
        let mut planes = HashMap::new();
        for (&solid, &epsilon) in epsilon {
            let Some(plan) = solids[&solid].plan_repair(epsilon, faces)? else {
                continue;
            };
            for &vertex in plan.points.keys() {
                if moved.insert(vertex, solid).is_some() {
                    return Err(format!(
                        "native repair vertex #{vertex} is shared between solids"
                    ));
                }
            }
            // Every authored trim can set an extremum of the repaired solid.
            for &edge in solids[&solid].edges.keys() {
                lines.insert(edge, line_input(records, edge)?);
            }
            for support in plan.endpoints.values().flat_map(|(_, supports)| supports) {
                if let std::collections::hash_map::Entry::Vacant(entry) = planes.entry(*support) {
                    entry.insert(plane_input(records, *support)?);
                }
            }
            plans.insert(solid, plan);
        }
        if plans.is_empty() {
            return Ok(None);
        }
        for (&solid, definition) in solids {
            for vertex in definition.edges.values().flat_map(|edge| edge.vertices) {
                if moved.get(&vertex).is_some_and(|owner| *owner != solid) {
                    return Err(format!(
                        "native repair vertex #{vertex} is shared between solids"
                    ));
                }
            }
        }
        let mut placement_inputs = placement::Inputs::default();
        let mut placed = Vec::new();
        for (index, &((solid, _, representation), ref path)) in occurrences.iter().enumerate() {
            if plans.contains_key(&solid) {
                let path = path
                    .as_ref()
                    .ok_or("missing authored shared trim placement")?;
                placement_inputs
                    .add(records, representation, path)
                    .ok_or("unsupported exact shared trim placement or length unit")?;
                placed.push((index, representation, path));
            }
        }
        let source = finite_line::capture_native(bytes, &lines, &planes, &placement_inputs.values)
            .ok_or("shared trim repair requires original LINE, PLANE and placement coefficients")?;
        let mut steps = HashMap::new();
        let placements = placed
            .into_iter()
            .map(|(index, representation, path)| {
                placement_inputs
                    .resolve(&source, representation, path, &mut steps)
                    .map(|placement| (index, placement))
                    .ok_or("unsupported exact shared trim placement axes or scale")
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(Self {
            plans,
            source,
            placements,
        }))
    }

    pub(super) fn apply(
        &self,
        id: u64,
        solid: &Solid,
        occurrence: usize,
        frame: &OccurrenceGeometry,
        bounds: &mut [[f64; 3]; 2],
    ) -> Result<Option<SharedRepair>, String> {
        let Some(plan) = self.plans.get(&id) else {
            return Ok(None);
        };
        let exact = &self.placements[&occurrence];
        for (&vertex, &(point, ref planes)) in &plan.endpoints {
            let emitted = frame.world.transform_point(point);
            for &support in planes {
                if self.source.planes[&support].contains(
                    point,
                    emitted,
                    exact,
                    frame.epsilon,
                    frame.world_epsilon,
                ) != Some(true)
                {
                    return Err(format!(
                        "native vertex #{vertex}: emitted trim does not meet original PLANE #{support}"
                    ));
                }
            }
        }
        let mut movement_mm = HashMap::with_capacity(plan.edges.len());
        let mut edges = HashMap::with_capacity(solid.edges.len());
        for (&id, edge) in &solid.edges {
            let Curve::Line {
                origin,
                direction,
                forward,
            } = edge.curve
            else {
                unreachable!("only plain LINE edges are eligible for planar repair");
            };
            let mut repaired = Edge {
                vertices: edge.vertices,
                points: edge.points,
                curve: Curve::Line {
                    origin,
                    direction,
                    forward,
                },
                carrier: None,
                finite_ellipse: None,
            };
            if plan.edges.contains(&id) {
                repaired.points = edge.vertices.map(|vertex| plan.endpoints[&vertex].0);
                let movement = self.source.lines[&id]
                    .shared_repair_bound(
                        repaired.points,
                        frame.world,
                        exact,
                        frame.allowance_mm,
                        bounds,
                    )
                    .map_err(|error| format!("native edge #{id} shared trim repair: {error}"))?;
                let [a, b] = repaired.points;
                let chord = if forward { b - a } else { a - b };
                if !chord.magnitude2().is_finite() || chord.magnitude2() <= 0.0 {
                    return Err(format!("native edge #{id}: degenerate repaired LINE"));
                }
                repaired.curve = Curve::Line {
                    origin: a,
                    direction: chord.normalize(),
                    forward,
                };
                movement_mm.insert(id, movement);
            } else {
                self.source.lines[&id]
                    .include_placed_bounds(exact, bounds)
                    .map_err(|error| format!("native edge #{id}: {error}"))?;
            }
            edges.insert(id, repaired);
        }
        for point in *bounds {
            check_coordinate(Point3::from(point))?;
        }
        Ok(Some(SharedRepair { edges, movement_mm }))
    }
}

impl Solid {
    fn plan_repair(&self, epsilon: f64, faces: &[StepSourceFace]) -> Result<Option<Plan>, String> {
        // Curved supports and parameter curves need their own finite-domain
        // correspondence. Keep them under the strict native attachment checks.
        if self
            .faces
            .iter()
            .any(|f| !matches!(f.surface.kind, SurfaceKind::Plane))
            || self
                .edges
                .values()
                .any(|e| !matches!(e.curve, Curve::Line { .. }) || e.carrier.is_some())
        {
            return Ok(None);
        }
        let incidences = || {
            self.faces.iter().flat_map(|face| {
                face.loops
                    .iter()
                    .flat_map(|ring| &ring.edges)
                    .flat_map(move |(id, _)| {
                        let edge = &self.edges[id];
                        edge.vertices
                            .into_iter()
                            .zip(edge.points)
                            .map(move |(id, point)| (id, point, face))
                    })
            })
        };
        let mut affected = BTreeMap::new();
        for (id, point, face) in incidences() {
            if face.surface.local(point).z.abs() > epsilon {
                affected.entry(id).or_insert_with(|| (point, Vec::new()));
            }
        }
        if affected.is_empty() {
            return Ok(None);
        }
        for (id, _, face) in incidences() {
            if let Some((_, planes)) = affected.get_mut(&id) {
                planes.push(face.surface);
            }
        }
        let points = affected
            .into_iter()
            .map(|(id, (point, planes))| {
                intersect_planes(point, &planes, epsilon)
                    .map(|point| (id, point))
                    .ok_or_else(|| {
                        format!("native vertex #{id}: incompatible incident PLANE supports")
                    })
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let mut edges = HashSet::new();
        let mut endpoints = HashMap::new();
        for (&id, edge) in &self.edges {
            if edge.vertices.iter().any(|id| points.contains_key(id)) {
                edges.insert(id);
                for (vertex, point) in edge.vertices.into_iter().zip(edge.points) {
                    endpoints.entry(vertex).or_insert_with(|| {
                        (
                            points.get(&vertex).copied().unwrap_or(point),
                            HashSet::new(),
                        )
                    });
                }
            }
        }
        for (vertex, _, face) in incidences() {
            if let Some((_, planes)) = endpoints.get_mut(&vertex) {
                planes.insert(faces[face.source].support_id);
            }
        }
        Ok(Some(Plan {
            points,
            endpoints,
            edges,
        }))
    }
}

fn plane_input(records: &SourceRecords<'_>, id: u64) -> Result<[u64; 2], String> {
    let plane = named_parameters(records, id, "PLANE", 2)?;
    let frame = named_parameters(records, reference(&plane[1])?, "AXIS2_PLACEMENT_3D", 4)?;
    let origin = reference(&frame[1])?;
    let normal = reference(&frame[2])
        .map_err(|_| format!("native repair requires explicit axis for PLANE #{id}"))?;
    named_record(records, origin, "CARTESIAN_POINT")?;
    named_record(records, normal, "DIRECTION")?;
    Ok([origin, normal])
}

fn intersect_planes(point: Point3, planes: &[Surface], epsilon: f64) -> Option<Point3> {
    let mut basis = Vec::<Vector3<f64>>::with_capacity(3);
    let mut delta = Vector3::new(0.0, 0.0, 0.0);
    // Solve in a point-relative frame for the smallest displacement in the span
    // of independent normals. Dependent constraints still have to pass below.
    for plane in planes {
        let normal = plane.frame.z.truncate();
        let independent = basis.iter().fold(normal, |n, &axis| n - axis * n.dot(axis));
        let magnitude = independent.magnitude();
        if magnitude > 1e-8 {
            let axis = independent / magnitude;
            delta += axis * ((-plane.local(point).z - normal.dot(delta)) / magnitude);
            basis.push(axis);
            if basis.len() == 3 {
                break;
            }
        }
    }
    let candidate = point + delta;
    planes
        .iter()
        .all(|plane| plane.local(candidate).z.abs() <= epsilon)
        .then_some(candidate)
}

fn line_input(records: &SourceRecords<'_>, id: u64) -> Result<([u64; 4], bool), String> {
    let edge = named_parameters(records, id, "EDGE_CURVE", 5)?;
    let line = named_parameters(records, reference(&edge[3])?, "LINE", 3)?;
    let vector = named_parameters(records, reference(&line[2])?, "VECTOR", 3)?;
    let vertex_point = |index: usize| {
        let vertex = named_parameters(records, reference(&edge[index])?, "VERTEX_POINT", 2)?;
        reference(&vertex[1])
    };
    Ok((
        [
            reference(&line[1])?,
            reference(&vector[1])?,
            vertex_point(1)?,
            vertex_point(2)?,
        ],
        flag(&edge[4])?,
    ))
}
