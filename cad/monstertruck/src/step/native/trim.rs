//! Mesh the authored trim loops using the samples shared by adjacent faces.
use super::{geometry::Curve, *};
use cgmath::Point2;
use spade::{ConstrainedDelaunayTriangulation, Point2 as ChartPoint, Triangulation};

type Chart = ConstrainedDelaunayTriangulation<ChartPoint<f64>>;
type Samples = HashMap<u64, Vec<Point3>>;
type Triangles = Vec<[Point3; 3]>;

impl Face {
    pub(super) fn mesh(
        &self,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
    ) -> Result<Triangles, String> {
        match self.surface.radius {
            None => self.mesh_plane(edges, samples, epsilon),
            Some(radius) => self.mesh_cylinder(radius, edges, samples, epsilon),
        }
    }

    fn mesh_plane(
        &self,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
    ) -> Result<Triangles, String> {
        let mut points = Vec::new();
        let mut rings = Vec::new();
        for ring in &self.loops {
            let start = points.len();
            for &(id, forward) in &ring.edges {
                let edge = &edges[&id];
                let residual = match edge.curve {
                    Curve::Line { .. } => edge
                        .points
                        .into_iter()
                        .map(|p| self.surface.local(p).z.abs())
                        .fold(0.0, f64::max),
                    Curve::Circle { frame, radius, .. } => {
                        let center = Point3::new(frame.w.x, frame.w.y, frame.w.z);
                        let n = self.surface.frame.z.truncate();
                        self.surface.local(center).z.abs()
                            + radius * frame.x.truncate().dot(n).hypot(frame.y.truncate().dot(n))
                    }
                };
                if residual > epsilon {
                    return Err("source boundary is off its PLANE".into());
                }
                let span = &samples[&id];
                points.extend(
                    (0..span.len() - 1).map(|i| span[if forward { i } else { span.len() - 1 - i }]),
                );
            }
            rings.push(start..points.len());
        }
        if points.len() > 4096 {
            return Err("native planar trim sample limit".into());
        }
        // Dropping the dominant normal axis avoids introducing artificial
        // slivers when rotating collinear authored boundary points into a chart.
        let normal = self.surface.frame.z.truncate();
        let axis = (0..3)
            .max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
            .unwrap();
        let (mut u, mut v) = ((axis + 1) % 3, (axis + 2) % 3);
        if normal[axis] < 0.0 {
            std::mem::swap(&mut u, &mut v);
        }
        let uv: Vec<_> = points.iter().map(|p| Point2::new(p[u], p[v])).collect();
        let mut cdt = Chart::new();
        let mut handles = Vec::new();
        for (i, p) in uv.iter().enumerate() {
            let handle = cdt
                .insert(ChartPoint::new(p.x, p.y))
                .map_err(|error| error.to_string())?;
            if handle.index() != i {
                return Err("planar trim rings touch or repeat a point".into());
            }
            handles.push(handle);
        }
        let sign = if self.same_sense { 1.0 } else { -1.0 };
        let mut authored = HashSet::new();
        let mut outers = 0;
        for (ring, definition) in rings.iter().zip(&self.loops) {
            if ring.len() < 3 {
                return Err("degenerate planar trim ring".into());
            }
            let mut area = 0.0;
            let origin = uv[ring.start];
            for (a, b) in ring_edges(ring.clone()) {
                let p = uv[a] - origin;
                let q = uv[b] - origin;
                area += p.x * q.y - p.y * q.x;
                let added = cdt.try_add_constraint(handles[a], handles[b]);
                if added.len() != 1 || !authored.insert((a, b)) {
                    return Err("planar trim crosses or splits another boundary".into());
                }
            }
            if !area.is_finite() || area == 0.0 || (definition.explicit_outer && area * sign < 0.0)
            {
                return Err("invalid planar trim orientation".into());
            }
            outers += usize::from(area * sign > 0.0);
        }
        if outers != 1 {
            return Err("planar trim does not have one outward-oriented outer ring".into());
        }

        let winding = trim_winding(&cdt, &authored, if self.same_sense { 1 } else { -1 })?;
        let mut triangles = Vec::new();
        let mut boundary = HashSet::new();
        for face in cdt.inner_faces() {
            if winding[face.fix().index()] == Some(0) {
                continue;
            }
            let mut indices = face.vertices().map(|v| v.fix().index());
            if !self.same_sense {
                indices.swap(1, 2);
            }
            let [a, b, c] = indices;
            for (a, b) in [(a, b), (b, c), (c, a)] {
                if !boundary.remove(&(b, a)) && !boundary.insert((a, b)) {
                    return Err("duplicate oriented planar triangle edge".into());
                }
            }
            triangles.push(indices.map(|i| points[i]));
        }
        if triangles.is_empty() || boundary != authored {
            return Err("planar mesh did not retain all authored trims".into());
        }
        Ok(triangles)
    }

    fn mesh_cylinder(
        &self,
        radius: f64,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
    ) -> Result<Triangles, String> {
        let [ring] = self.loops.as_slice() else {
            return Err("native cylinder requires one strip boundary".into());
        };
        if ring.edges.len() != 4 {
            return Err("native cylinder requires four strip edges".into());
        }
        let start = ring
            .edges
            .iter()
            .position(|(id, _)| matches!(edges[id].curve, Curve::Circle { .. }))
            .ok_or("native cylinder lacks a circular trim")?;
        let [(a, fa), (b, _), (c, fc), (d, _)] = [0, 1, 2, 3].map(|i| ring.edges[(start + i) % 4]);
        if !matches!(
            (&edges[&b].curve, &edges[&d].curve),
            (Curve::Line { .. }, Curve::Line { .. })
        ) {
            return Err("native cylinder edges do not alternate circles and generators".into());
        }
        let axis = self.surface.frame.z.truncate();
        for edge in [&edges[&a], &edges[&c]] {
            let Curve::Circle {
                frame,
                radius: edge_radius,
                ..
            } = edge.curve
            else {
                return Err("native cylinder lacks its second circular trim".into());
            };
            let center = self
                .surface
                .local(Point3::new(frame.w.x, frame.w.y, frame.w.z));
            let radial_error = center.x.hypot(center.y)
                + (radius - edge_radius).abs()
                + radius * frame.z.truncate().cross(axis).magnitude();
            if radial_error > epsilon {
                return Err("source circular boundary is off its CYLINDRICAL_SURFACE".into());
            }
        }
        let a = &samples[&a];
        let c = &samples[&c];
        if a.len() != c.len() {
            return Err("native cylinder arcs lack matching subdivisions".into());
        }
        let mut strips = Vec::new();
        for i in 0..a.len() {
            let p = a[if fa { i } else { a.len() - 1 - i }];
            let q = c[if fc { c.len() - 1 - i } else { i }];
            let delta = q - p;
            if delta.cross(axis).magnitude() > epsilon || delta.dot(axis).abs() <= epsilon {
                return Err("native cylinder trims do not define a constant-height strip".into());
            }
            strips.push([p, q]);
        }
        let mut triangles = Vec::new();
        for window in strips.windows(2) {
            let [[a, d], [b, c]] = [window[0], window[1]];
            let radial = self.surface.local(a);
            let outward = self.surface.frame.x.truncate() * radial.x
                + self.surface.frame.y.truncate() * radial.y;
            let orientation = (b - a).cross(d - a).dot(outward);
            if orientation == 0.0 || (orientation > 0.0) != self.same_sense {
                return Err("native cylinder trim contradicts surface sense".into());
            }
            triangles.extend([[a, b, d], [b, c, d]]);
        }
        Ok(triangles)
    }
}

fn ring_edges(ring: std::ops::Range<usize>) -> impl Iterator<Item = (usize, usize)> {
    ring.clone()
        .zip(ring.clone().skip(1).chain(std::iter::once(ring.start)))
}

// Seed the exterior at zero and cross each directed constraint topologically.
// Classifying a floating-point centroid can lose the side of a very thin triangle.
fn trim_winding(
    cdt: &Chart,
    authored: &HashSet<(usize, usize)>,
    inside: i8,
) -> Result<Vec<Option<i8>>, String> {
    let mut neighbors = vec![Vec::new(); cdt.num_all_faces()];
    for edge in cdt.undirected_edges() {
        let edge = edge.as_directed();
        let a = edge.from().fix().index();
        let b = edge.to().fix().index();
        let delta = i8::from(authored.contains(&(a, b))) - i8::from(authored.contains(&(b, a)));
        let left = edge.face().fix().index();
        let right = edge.rev().face().fix().index();
        neighbors[right].push((left, delta));
        neighbors[left].push((right, -delta));
    }
    let outer = cdt.outer_face().fix().index();
    let mut winding = vec![None; neighbors.len()];
    winding[outer] = Some(0);
    let mut pending = vec![(outer, 0)];
    while let Some((face, value)) = pending.pop() {
        for &(next, delta) in &neighbors[face] {
            let next_value = value + delta;
            if (next_value != 0 && next_value != inside)
                || winding[next].is_some_and(|previous| previous != next_value)
            {
                return Err("planar trim has intersecting or nested boundaries".into());
            }
            if winding[next].is_none() {
                winding[next] = Some(next_value);
                pending.push((next, next_value));
            }
        }
    }
    if winding.iter().any(Option::is_none) {
        return Err("incomplete planar triangulation".into());
    }
    Ok(winding)
}
