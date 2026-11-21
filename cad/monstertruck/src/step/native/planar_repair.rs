//! Reconcile shared vertices with their fixed planes before rebuilding straight trims.
use super::{geometry::Curve, *};
use std::collections::BTreeMap;

pub(super) struct RepairFrame {
    pub epsilon: f64,
    pub allowance_mm: f64,
    pub mm_per_unit: f64,
    pub placement: Matrix4,
}

pub(super) struct SharedRepair {
    pub edges: HashMap<u64, Edge>,
    pub movement_mm: HashMap<u64, f64>,
}

impl Solid {
    pub(super) fn repair_planar_trims(
        &self,
        records: &SourceRecords<'_>,
        bytes: &[u8],
        frame: RepairFrame,
        bounds: &mut [[f64; 3]; 2],
    ) -> Result<Option<SharedRepair>, String> {
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
                    .flat_map(|(id, _)| {
                        let edge = &self.edges[id];
                        edge.vertices
                            .into_iter()
                            .zip(edge.points)
                            .map(|(id, point)| (id, point, face.surface))
                    })
            })
        };
        let mut affected = BTreeMap::new();
        for (id, point, plane) in incidences() {
            if plane.local(point).z.abs() > frame.epsilon {
                affected.entry(id).or_insert_with(|| (point, Vec::new()));
            }
        }
        if affected.is_empty() {
            return Ok(None);
        }
        for (id, _, plane) in incidences() {
            if let Some((_, planes)) = affected.get_mut(&id) {
                planes.push(plane);
            }
        }
        let points = affected
            .into_iter()
            .map(|(id, (point, planes))| {
                intersect_planes(point, &planes, frame.epsilon)
                    .map(|point| (id, point))
                    .ok_or_else(|| {
                        format!("native vertex #{id}: incompatible incident PLANE supports")
                    })
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        let inputs = self
            .edges
            .iter()
            .filter(|(_, edge)| edge.vertices.iter().any(|id| points.contains_key(id)))
            .map(|(&id, _)| line_input(records, id).map(|input| (id, input)))
            .collect::<Result<HashMap<_, _>, _>>()?;
        let source = repair::finite_line::capture_native(bytes, &inputs)
            .ok_or("shared trim repair requires original LINE coefficients")?;
        let world = frame.placement * Matrix4::from_scale(frame.mm_per_unit);
        let mut movement_mm = HashMap::with_capacity(inputs.len());
        let mut edges = HashMap::with_capacity(self.edges.len());
        for (&id, edge) in &self.edges {
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
            if let Some(line) = source.get(&id) {
                for (vertex, point) in repaired.vertices.into_iter().zip(&mut repaired.points) {
                    if let Some(replacement) = points.get(&vertex) {
                        *point = *replacement;
                    }
                }
                let (movement, projected) = line
                    .shared_repair_bound(
                        repaired.points,
                        frame.mm_per_unit,
                        frame.placement,
                        frame.allowance_mm,
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
                // The caller includes the rebuilt edges. Keep the original
                // vertex chord and projected source interval in the envelope too.
                for point in edge.points.into_iter().chain(projected) {
                    let point = world.transform_point(point);
                    check_coordinate(point)?;
                    super::super::bounds::push(bounds, point)?;
                }
            }
            edges.insert(id, repaired);
        }
        Ok(Some(SharedRepair { edges, movement_mm }))
    }
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
