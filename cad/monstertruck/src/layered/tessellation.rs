//! Tessellate the validated layer boundaries in model coordinates.
use super::brep::{EdgeUse, Face, Point3, Strip, Support, Topology};
use crate::profile::closed_oriented_shell_triangles;
type Vector2 = cgmath::Vector2<f64>;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet, hash_map::Entry},
};

type Result<T> = std::result::Result<T, String>;
const MAX_ARC_STEPS: usize = 4096;

pub(crate) struct FaceMesh {
    pub positions: Vec<Point3>,
    pub uv: Vec<Vector2>,
    pub triangles: Vec<[usize; 3]>,
}

impl FaceMesh {
    pub(crate) fn triangle_points(&self) -> impl Iterator<Item = [Point3; 3]> + '_ {
        self.triangles
            .iter()
            .map(|triangle| triangle.map(|i| self.positions[i]))
    }
}

pub(crate) fn tessellate(
    topology: &Topology,
    faces: &[Face],
    deflection: f64,
    angular: f64,
) -> Result<Vec<FaceMesh>> {
    let cap_levels: HashSet<_> = faces
        .iter()
        .filter(|face| face.strips.is_empty())
        .filter_map(|face| match face.support {
            Support::Plane { origin, .. } => Some(coordinate_key(origin.z)),
            Support::Round { .. } => None,
        })
        .collect();
    let strips: Vec<_> = faces
        .iter()
        .map(|face| render_strips(topology, face, &cap_levels))
        .collect();
    let mut effective_radii = HashMap::<_, f64>::new();
    for (face, strips) in faces.iter().zip(&strips) {
        if let Support::Round {
            center,
            lower,
            upper,
            ..
        } = face.support
        {
            let radius = effective_radii
                .entry(center.map(coordinate_key))
                .or_default();
            // A tapered patch uses half the chord budget to retain the frustum
            // volume accuracy of generated case analysis.
            let factor = if lower[1] == upper[1] { 1. } else { 2. };
            for strip in strips.iter() {
                for edge in [strip.lower, strip.upper] {
                    let circle = topology.edges[edge.index]
                        .circle
                        .ok_or("Missing wall arc")?;
                    *radius = radius.max(circle.radius * factor);
                }
            }
        }
    }

    // Sample each edge once, with one budget on each round wall axis. This
    // keeps the common boundaries of strips, cones and cylinders identical.
    let mut samples = EdgeSamples::default();
    for (face, strips) in faces.iter().zip(&strips) {
        let radius = match face.support {
            Support::Round { center, .. } => Some(effective_radii[&center.map(coordinate_key)]),
            Support::Plane { .. } => None,
        };
        for strip in strips.iter() {
            for edge in [strip.lower, strip.upper] {
                samples.insert(topology, edge, radius, deflection, angular)?;
            }
        }
    }
    let faces = faces
        .iter()
        .zip(&strips)
        .map(|(face, strips)| {
            if strips.is_empty() {
                cap(face, &samples)
            } else {
                walls(strips, &samples)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    if faces.iter().any(|face| face.triangles.is_empty()) {
        return Err("Layered tessellation left a source face empty".into());
    }
    let triangles = faces
        .iter()
        .flat_map(FaceMesh::triangle_points)
        .collect::<Vec<_>>();
    if !closed_oriented_shell_triangles(&triangles) {
        return Err("Layered tessellation has an open or inconsistently wound shell".into());
    }
    Ok(faces)
}

// Adjacent strips on the same Z-invariant support can share a triangle
// across an internal layer break. Omit breaks that would otherwise create
// zero-height render triangles. Native rings and their exact Z stay intact.
fn render_strips<'a>(
    topology: &Topology,
    face: &'a Face,
    cap_levels: &HashSet<u64>,
) -> Cow<'a, [Strip]> {
    let strips = &face.strips;
    let height = |edge: EdgeUse| topology.vertices[edge.vertices(topology)[0]].z;
    let thin = |strip: &Strip| height(strip.lower) as f32 == height(strip.upper) as f32;
    if matches!(face.support, Support::Round { lower, upper, .. } if lower[1] != upper[1])
        || !strips.iter().any(thin)
    {
        return Cow::Borrowed(strips);
    }

    let mut strips = strips.clone();
    let mut index = 0;
    while index < strips.len() {
        let current = strips[index];
        if !thin(&current) {
            index += 1;
            continue;
        }
        let above = strips.iter().position(|next| next.lower == current.upper);
        let below = strips.iter().position(|next| next.upper == current.lower);
        // Keep the shared cap elevation when either neighboring strip is available.
        let prefer_below = cap_levels.contains(&coordinate_key(height(current.upper)));
        if let Some(above) = above.filter(|_| !prefer_below || below.is_none()) {
            let upper = strips.remove(above).upper;
            index -= usize::from(above < index);
            strips[index].upper = upper;
        } else if let Some(below) = below {
            let lower = strips.remove(below).lower;
            index -= usize::from(below < index);
            strips[index].lower = lower;
        } else {
            index += 1;
        }
    }
    Cow::Owned(strips)
}

fn coordinate_key(value: f64) -> u64 {
    if value == 0. { 0 } else { value.to_bits() }
}

#[derive(Default)]
struct EdgeSamples(HashMap<usize, Vec<Point3>>);

impl EdgeSamples {
    fn insert(
        &mut self,
        topology: &Topology,
        edge: EdgeUse,
        axis_radius: Option<f64>,
        deflection: f64,
        angular: f64,
    ) -> Result<()> {
        let Entry::Vacant(entry) = self.0.entry(edge.index) else {
            return Ok(());
        };
        let edge = topology.edges[edge.index];
        let [first, last] = edge.vertices.map(|index| topology.vertices[index]);
        let mut points = vec![first];
        if let Some(arc) = edge.circle {
            let radius = axis_radius.ok_or("Missing circular wall axis")?;
            // Sagitta = 2 r sin²(angle / 4). This form avoids cancellation for small angles.
            let chord_span = 4. * (0.5 * (deflection / radius)).sqrt().min(1.).asin();
            let span = angular.min(chord_span);
            let steps = (arc.sweep.abs() / span).ceil() as usize;
            // Keep the refined cylinder inside the qualified-analysis source budget.
            let steps = steps.saturating_add(usize::from(chord_span < angular));
            // An arc and its chord can be a two-edge cap. Reserve an interior
            // arc sample so that such a cap always has a nonzero area.
            let steps = steps.max(2);
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

    fn directed(&self, edge: EdgeUse) -> Result<impl ExactSizeIterator<Item = Point3> + '_> {
        let Some(points) = self.0.get(&edge.index) else {
            return Err("Missing shared cap or wall edge".into());
        };
        let forward = edge.forward;
        let len = points.len();
        Ok((0..len).map(move |i| points[if forward { i } else { len - i - 1 }]))
    }
}

fn walls(strips: &[Strip], samples: &EdgeSamples) -> Result<FaceMesh> {
    let mut positions = Vec::new();
    let mut uv = Vec::new();
    let mut triangles = Vec::new();
    for strip in strips {
        let lower = samples.directed(strip.lower)?;
        let upper = samples.directed(strip.upper)?;
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
    Ok(FaceMesh {
        positions,
        uv,
        triangles,
    })
}

fn cap(face: &Face, samples: &EdgeSamples) -> Result<FaceMesh> {
    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();
    let mut positions = Vec::new();
    let mut rings = Vec::new();
    for wire in &face.rings {
        let mut ring = Vec::new();
        for &edge in wire {
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
        .map(|triangle| {
            let [a, b, c] = triangle.vertices().map(|v| v.index());
            if face.forward { [a, b, c] } else { [a, c, b] }
        })
        .collect();
    Ok(FaceMesh {
        positions,
        uv,
        triangles,
    })
}
