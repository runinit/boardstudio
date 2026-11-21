//! Tessellate the validated layer boundaries in model coordinates.
use crate::profile::{CircleEdge, closed_oriented_shell_triangles};
use monstertruck_meshing::rexport_polymesh::{
    Faces, PolygonMesh, StandardAttributes, StandardVertex,
};
use monstertruck_modeling::{Curve, Edge, Face, Point3, Shell, Surface, Vector2};
use monstertruck_topology::{EdgeId, FaceId};
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};
use std::collections::{HashMap, hash_map::Entry};

type Result<T> = std::result::Result<T, String>;
const MAX_ARC_STEPS: usize = 4096;

pub(super) struct WallStrip {
    pub lower: Edge,
    pub upper: Edge,
    pub axis: Option<[f64; 2]>,
    pub tapered: bool,
}

#[derive(Default)]
pub(crate) struct LayeredMesh {
    // Missing wall strips identify caps. A merged native wall owns all of its strips.
    pub(super) walls: HashMap<FaceId<Surface>, Vec<WallStrip>>,
    pub(super) arcs: HashMap<EdgeId<Curve>, CircleEdge>,
}

impl LayeredMesh {
    pub(super) fn extend(&mut self, other: Self) {
        self.walls.extend(other.walls);
        self.arcs.extend(other.arcs);
    }

    pub(crate) fn cache_complexity(&self) -> usize {
        self.walls.values().fold(self.arcs.len(), |total, strips| {
            total.saturating_add(strips.len().saturating_mul(2))
        })
    }

    pub(crate) fn tessellate(
        &self,
        shell: &Shell,
        deflection: f64,
        angular: f64,
    ) -> Result<Vec<PolygonMesh>> {
        let mut effective_radii = HashMap::<_, f64>::new();
        for face in shell.face_iter() {
            for strip in self.walls.get(&face.id()).into_iter().flatten() {
                if let Some(axis) = strip.axis {
                    let radius = effective_radii.entry(axis.map(coordinate_key)).or_default();
                    // Reserve half the chord budget on tapered walls to retain the
                    // public analysis fixture's clipped-frustum volume accuracy.
                    let factor = if strip.tapered { 2. } else { 1. };
                    for edge in [&strip.lower, &strip.upper] {
                        let arc = self.arcs.get(&edge.id()).ok_or("Missing wall arc")?;
                        *radius = radius.max(arc.radius * factor);
                    }
                }
            }
        }

        // Sample the absolute edge once, so every use has exactly the same vertices.
        // Concentric strips share the tightest sampling budget on their axis,
        // keeping cones and adjoining cylinders on the same angular partition.
        let mut samples = EdgeSamples::default();
        for face in shell.face_iter() {
            for strip in self.walls.get(&face.id()).into_iter().flatten() {
                for edge in [&strip.lower, &strip.upper] {
                    let radius = strip
                        .axis
                        .map(|axis| effective_radii[&axis.map(coordinate_key)]);
                    samples.insert(edge, self.arcs.get(&edge.id()), radius, deflection, angular)?;
                }
            }
        }

        let faces = shell
            .face_iter()
            .map(|face| match self.walls.get(&face.id()) {
                Some(strips) => walls(strips, &samples),
                None => cap(face, &samples),
            })
            .collect::<Result<Vec<_>>>()?;
        if faces
            .iter()
            .any(|face| face.faces().triangle_iter().next().is_none())
        {
            return Err("Layered tessellation left a source face empty".into());
        }
        let triangles = faces
            .iter()
            .flat_map(|face| {
                face.faces()
                    .triangle_iter()
                    .map(|tri| tri.map(|v| face.positions()[v.pos]))
            })
            .collect::<Vec<_>>();
        if !closed_oriented_shell_triangles(&triangles) {
            return Err("Layered tessellation has an open or inconsistently wound shell".into());
        }
        for triangle in triangles {
            crate::mesh::triangle_normal(triangle)?;
        }
        Ok(faces)
    }
}

fn coordinate_key(value: f64) -> u64 {
    if value == 0. { 0 } else { value.to_bits() }
}

#[derive(Default)]
struct EdgeSamples(HashMap<EdgeId<Curve>, Vec<Point3>>);

impl EdgeSamples {
    fn insert(
        &mut self,
        edge: &Edge,
        arc: Option<&CircleEdge>,
        axis_radius: Option<f64>,
        deflection: f64,
        angular: f64,
    ) -> Result<()> {
        let Entry::Vacant(entry) = self.0.entry(edge.id()) else {
            return Ok(());
        };
        let first = edge.absolute_front().point();
        let last = edge.absolute_back().point();
        let mut points = vec![first];
        if let Some(arc) = arc {
            let radius = axis_radius.ok_or("Missing circular wall axis")?;
            // Sagitta = 2 r sin²(angle / 4). This form avoids cancellation for small angles.
            let chord_span = 4. * (0.5 * (deflection / radius)).sqrt().min(1.).asin();
            let span = angular.min(chord_span);
            let steps = (arc.sweep.abs() / span).ceil().max(1.) as usize;
            // Leave one chord of headroom on deflection-limited arcs: the sealed
            // cylinder proof also accounts for cap trims, beyond the chord sagitta.
            let steps = steps.saturating_add(usize::from(chord_span < angular));
            if steps > MAX_ARC_STEPS {
                return Err("Layered arc exceeds the sampling budget".into());
            }
            let start = (first.y - arc.center.y).atan2(first.x - arc.center.x);
            for step in 1..steps {
                let angle = start + arc.sweep * (step as f64 / steps as f64);
                let (sin, cos) = angle.sin_cos();
                points.push(Point3::new(
                    arc.center.x + arc.radius * cos,
                    arc.center.y + arc.radius * sin,
                    first.z,
                ));
            }
        }
        points.push(last);
        entry.insert(points);
        Ok(())
    }

    fn directed(&self, edge: &Edge) -> Result<impl ExactSizeIterator<Item = Point3> + '_> {
        let Some(points) = self.0.get(&edge.id()) else {
            return Err("Missing shared cap or wall edge".into());
        };
        let forward = edge.orientation();
        let len = points.len();
        Ok((0..len).map(move |i| points[if forward { i } else { len - i - 1 }]))
    }
}

fn polygon(
    positions: Vec<Point3>,
    uv_coords: Vec<Vector2>,
    triangles: impl IntoIterator<Item = [usize; 3]>,
) -> PolygonMesh {
    let faces = Faces::from_iter(triangles.into_iter().map(|tri| {
        tri.map(|pos| StandardVertex {
            pos,
            uv: Some(pos),
            nor: None,
        })
    }));
    PolygonMesh::new(
        StandardAttributes {
            positions,
            uv_coords,
            ..Default::default()
        },
        faces,
    )
}

fn walls(strips: &[WallStrip], samples: &EdgeSamples) -> Result<PolygonMesh> {
    let mut positions = Vec::new();
    let mut uv = Vec::new();
    let mut triangles = Vec::new();
    for strip in strips {
        let lower = samples.directed(&strip.lower)?;
        let upper = samples.directed(&strip.upper)?;
        let count = lower.len();
        if count != upper.len() {
            return Err("Layered wall has mismatched arc partitions".into());
        }
        let base = positions.len();
        for (index, (low, high)) in lower.zip(upper).enumerate() {
            positions.extend([low, high]);
            // Each strip has its own chart, including at cylindrical seams.
            let t = index as f64 / (count - 1) as f64;
            uv.extend([Vector2::new(t, low.z), Vector2::new(t, high.z)]);
        }
        for i in (base..base + 2 * (count - 1)).step_by(2) {
            triangles.extend([[i, i + 2, i + 3], [i, i + 3, i + 1]]);
        }
    }
    Ok(polygon(positions, uv, triangles))
}

fn cap(face: &Face, samples: &EdgeSamples) -> Result<PolygonMesh> {
    let Surface::Plane(plane) = face.oriented_surface() else {
        return Err("Layered cap has unsupported surface".into());
    };
    let up = plane.normal().z > 0.;
    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();
    let mut positions = Vec::new();
    let mut rings = Vec::new();
    for wire in face.boundaries() {
        let mut ring = Vec::new();
        for edge in wire.edge_iter() {
            let points = samples.directed(edge)?;
            // The last point is the first point of the next oriented edge.
            let count = points.len();
            for point in points.take(count - 1) {
                let handle = cdt
                    .insert(Point2::new(point.x, point.y))
                    .map_err(|err| format!("Invalid layered cap vertex: {err:?}"))?;
                if handle.index() != positions.len() {
                    return Err("Layered cap boundary touches or repeats a vertex".into());
                }
                positions.push(point);
                ring.push(handle);
            }
        }
        if ring.len() < 3 {
            return Err("Layered cap has fewer than three boundary vertices".into());
        }
        rings.push(ring);
    }
    // Constrain after inserting all points. A split or crossing would change the
    // boundary discretization that this cap must share with the wall.
    for ring in rings {
        for (a, b) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            if cdt.try_add_constraint(*a, *b).len() != 1 {
                return Err("Layered cap constraints intersect or split a wall edge".into());
            }
        }
    }

    // Flood from the exterior. Crossing a ring toggles material, including holes,
    // without computing a centroid that can round onto a boundary.
    let mut adjacent = vec![Vec::new(); cdt.num_all_faces()];
    for edge in cdt.directed_edges() {
        adjacent[edge.face().index()].push((edge.rev().face().index(), edge.is_constraint_edge()));
    }
    let exterior = cdt.outer_face().index();
    let mut material = vec![None; adjacent.len()];
    material[exterior] = Some(false);
    let mut pending = vec![(exterior, false)];
    while let Some((face, inside)) = pending.pop() {
        for &(neighbor, boundary) in &adjacent[face] {
            let next = inside ^ boundary;
            match material[neighbor] {
                None => {
                    material[neighbor] = Some(next);
                    pending.push((neighbor, next));
                }
                Some(previous) if previous != next => {
                    return Err("Layered cap has inconsistent boundary parity".into());
                }
                Some(_) => (),
            }
        }
    }

    let uv = positions.iter().map(|p| Vector2::new(p.x, p.y)).collect();
    let triangles = cdt
        .inner_faces()
        .filter(|face| material[face.index()] == Some(true))
        .map(|face| {
            let [a, b, c] = face.vertices().map(|v| v.index());
            if up { [a, b, c] } else { [a, c, b] }
        });
    Ok(polygon(positions, uv, triangles))
}
