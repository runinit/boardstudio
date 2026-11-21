//! Mesh the authored trim loops using the samples shared by adjacent faces.
use super::{
    geometry::{Curve, circle_step},
    *,
};
use cgmath::Point2;
use spade::{ConstrainedDelaunayTriangulation, Point2 as ChartPoint, Triangulation};

type Chart = ConstrainedDelaunayTriangulation<ChartPoint<f64>>;
pub(super) type Samples = HashMap<u64, Vec<Point3>>;
pub(super) type Triangles = Vec<[Point3; 3]>;

impl Face {
    pub(super) fn mesh(
        &self,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        deflection: f64,
    ) -> Result<Triangles, String> {
        match self.surface.kind {
            SurfaceKind::Plane => self.mesh_plane(edges, samples, epsilon),
            SurfaceKind::Cylinder(radius) => {
                self.mesh_cylinder(radius, edges, samples, epsilon, deflection)
            }
            SurfaceKind::Cone { radius, slope, .. } => {
                self.mesh_cone(radius, slope, edges, samples, epsilon, deflection)
            }
        }
    }

    fn mesh_plane(
        &self,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
    ) -> Result<Triangles, String> {
        if self
            .loops
            .iter()
            .flat_map(|ring| &ring.edges)
            .any(|(id, _)| matches!(edges[id].curve, Curve::Ellipse { .. }))
        {
            self.check_planar_ellipse_trim(edges, epsilon)?;
        }
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
                    Curve::Ellipse {
                        frame,
                        major,
                        minor,
                        ..
                    } => {
                        let center = Point3::new(frame.w.x, frame.w.y, frame.w.z);
                        let n = self.surface.frame.z.truncate();
                        self.surface.local(center).z.abs()
                            + (major * frame.x.truncate().dot(n))
                                .hypot(minor * frame.y.truncate().dot(n))
                    }
                };
                if !residual.is_finite() || residual > epsilon {
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
        let triangles = triangulate_trim(&uv, &rings, &self.loops, self.same_sense)?;
        Ok(triangles
            .into_iter()
            .map(|triangle| triangle.map(|i| points[i]))
            .collect())
    }

    fn mesh_cylinder(
        &self,
        radius: f64,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        deflection: f64,
    ) -> Result<Triangles, String> {
        if let [a, b] = self.loops.as_slice() {
            return self.mesh_cylinder_band(radius, [a, b], edges, samples, epsilon, deflection);
        }
        let [ring] = self.loops.as_slice() else {
            return Err("native cylinder requires one strip boundary".into());
        };
        if ring
            .edges
            .iter()
            .any(|(id, _)| matches!(edges[id].curve, Curve::Ellipse { .. }))
        {
            return self.mesh_oblique_cylinder(radius, ring, edges, samples, epsilon);
        }
        if ring.edges.len() != 4 {
            return self.mesh_cylinder_chart(radius, ring, edges, samples, epsilon, deflection);
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
        for edge in [&edges[&a], &edges[&c]] {
            edge.cylinder_sweep(self.surface, radius, epsilon)?;
        }
        let axis = self.surface.frame.z.truncate();
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

    fn mesh_cylinder_chart(
        &self,
        radius: f64,
        ring: &Boundary,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        deflection: f64,
    ) -> Result<Triangles, String> {
        let is_line = |i: usize| matches!(edges[&ring.edges[i].0].curve, Curve::Line { .. });
        let count = ring.edges.len();
        let start = (0..count)
            .find(|&i| !is_line(i) && is_line((i + count - 1) % count))
            .ok_or("native cylinder requires circular arcs and generators")?;
        let (start_id, start_forward) = ring.edges[start];
        let first_span = &samples[&start_id];
        let first = self.surface.local(if start_forward {
            first_span[0]
        } else {
            *first_span.last().unwrap()
        });
        let origin_angle = first.y.atan2(first.x);
        let mut u = 0.0;
        let mut height = first.z;
        let mut uv = Vec::new();
        let mut points = Vec::new();
        let limit = circle_step(radius, deflection) * radius;
        let mut max_boundary_step = limit;
        for i in 0..count {
            let (id, forward) = ring.edges[(start + i) % count];
            let span = &samples[&id];
            let at = |j| span[if forward { j } else { span.len() - 1 - j }];
            let from = self.surface.local(at(0));
            let to = self.surface.local(at(span.len() - 1));
            if matches!(edges[&id].curve, Curve::Line { .. }) {
                if (to.x - from.x).hypot(to.y - from.y) > epsilon {
                    return Err("native cylinder line is not a generator".into());
                }
                uv.push(Point2::new(u, height));
                points.push(at(0));
                height = to.z;
            } else {
                let sweep = edges[&id].cylinder_sweep(self.surface, radius, epsilon)?;
                let sweep = if forward { sweep } else { -sweep };
                let extent = sweep * radius;
                let segments = span.len() - 1;
                // A circular run is horizontal in the chart, even when authored
                // frames and vertices differ by floating-point attachment noise.
                for j in 0..segments {
                    let x = u + extent * j as f64 / segments as f64;
                    uv.push(Point2::new(x, height));
                    points.push(at(j));
                }
                u += extent;
            }
            if uv.len() > 4096 {
                return Err("native cylindrical trim sample limit".into());
            }
        }
        let (lo, hi) = uv
            .iter()
            .fold((0.0_f64, 0.0_f64), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)));
        if u.abs() > epsilon || hi - lo >= std::f64::consts::TAU * radius - epsilon {
            return Err("native cylindrical trim does not close within a single chart".into());
        }
        let ring_indices = 0..uv.len();
        for (a, b) in ring_edges(ring_indices.clone()) {
            max_boundary_step = max_boundary_step.max((uv[a].x - uv[b].x).abs());
        }
        // Preserve the exact shared 3D trims. The 2D chart may only remove
        // floating-point discrepancies in angle or height from its coordinates.
        let on_support = |p: Point2<f64>| {
            let angle = origin_angle + p.x / radius;
            self.surface.frame.transform_point(Point3::new(
                radius * angle.cos(),
                radius * angle.sin(),
                p.y,
            ))
        };
        if points
            .iter()
            .zip(&uv)
            .any(|(&p, &q)| (p - on_support(q)).magnitude() > 2.0 * epsilon)
        {
            return Err("native cylindrical trim is off its chart".into());
        }
        let indices = triangulate_trim(&uv, &[ring_indices], &self.loops, self.same_sense)?;
        let mut pending: Vec<_> = indices
            .into_iter()
            .map(|triangle| triangle.map(|i| (uv[i], points[i])))
            .collect();
        let mut triangles = Vec::new();
        while let Some(triangle) = pending.pop() {
            let [a, b, c] = triangle;
            let side = [
                (a.0.x - b.0.x).abs(),
                (b.0.x - c.0.x).abs(),
                (c.0.x - a.0.x).abs(),
            ];
            let longest = (0..3).max_by(|&a, &b| side[a].total_cmp(&side[b])).unwrap();
            if side[longest] <= max_boundary_step {
                let lifted = triangle.map(|(_, point)| point);
                self.check_cylinder_triangle(lifted)?;
                triangles.push(lifted);
                continue;
            }
            if pending.len() + triangles.len() >= 32768 {
                return Err("native cylindrical trim refinement limit".into());
            }
            let [a, b, c] = [0, 1, 2].map(|offset| triangle[(longest + offset) % 3]);
            let mid = Point2::new((a.0.x + b.0.x) * 0.5, (a.0.y + b.0.y) * 0.5);
            let mid = (mid, on_support(mid));
            // Adjacent triangles bisect the same long chart edge at the same
            // point. No source boundary segment exceeds the split threshold.
            pending.extend([[a, mid, c], [mid, b, c]]);
        }
        Ok(triangles)
    }

    pub(super) fn check_cylinder_triangle(&self, [a, b, c]: [Point3; 3]) -> Result<(), String> {
        let normal = (b - a).cross(c - a);
        // The radial direction varies affinely on a chord triangle.
        // A consistent sign at the corners rules out a fold under radial projection.
        for point in [a, b, c] {
            let local = self.surface.local(point);
            let radial = self.surface.frame.x.truncate() * local.x
                + self.surface.frame.y.truncate() * local.y;
            let orientation = normal.dot(radial);
            if !orientation.is_finite()
                || orientation == 0.0
                || (orientation > 0.0) != self.same_sense
            {
                return Err("native cylindrical chart folds when lifted to its support".into());
            }
        }
        Ok(())
    }
}

pub(super) fn triangulate_trim(
    uv: &[Point2<f64>],
    rings: &[std::ops::Range<usize>],
    loops: &[Boundary],
    same_sense: bool,
) -> Result<Vec<[usize; 3]>, String> {
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
    let sign = if same_sense { 1.0 } else { -1.0 };
    let mut authored = HashSet::new();
    let mut outers = 0;
    for (ring, definition) in rings.iter().zip(loops) {
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
        if !area.is_finite() || area == 0.0 || (definition.explicit_outer && area * sign < 0.0) {
            return Err("invalid planar trim orientation".into());
        }
        outers += usize::from(area * sign > 0.0);
    }
    if outers != 1 {
        return Err("planar trim does not have one outward-oriented outer ring".into());
    }

    let winding = trim_winding(&cdt, &authored, if same_sense { 1 } else { -1 })?;
    let mut triangles = Vec::new();
    let mut boundary = HashSet::new();
    for face in cdt.inner_faces() {
        if winding[face.fix().index()] == Some(0) {
            continue;
        }
        let mut indices = face.vertices().map(|v| v.fix().index());
        if !same_sense {
            indices.swap(1, 2);
        }
        let [a, b, c] = indices;
        for (a, b) in [(a, b), (b, c), (c, a)] {
            if !boundary.remove(&(b, a)) && !boundary.insert((a, b)) {
                return Err("duplicate oriented planar triangle edge".into());
            }
        }
        triangles.push(indices);
    }
    if triangles.is_empty() || boundary != authored {
        return Err("planar mesh did not retain all authored trims".into());
    }
    Ok(triangles)
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
