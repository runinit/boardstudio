//! A single unwrapped chart for regular cone trims, or a fan for a circular rim and tip.
use super::{
    geometry::{Curve, circle_step},
    trim::{Samples, Triangles, triangulate_trim},
    *,
};
use cgmath::Point2;
use std::f64::consts::TAU;

const MAX_TRIM_SAMPLES: usize = 4096;
const ANGLE_ROUNDOFF: f64 = 1e-12;

struct Cone {
    surface: Surface,
    radius: f64,
    slope: f64,
}

#[derive(Clone, Copy)]
struct Sample {
    uv: Point2<f64>,
    point: Point3,
}

struct Ring {
    samples: Vec<Sample>,
    angles: [f64; 2],
    generators: Vec<(u64, f64)>,
    max_radius: f64,
    area: f64,
}

impl Face {
    pub(super) fn mesh_cone(
        &self,
        radius: f64,
        slope: f64,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        deflection: f64,
    ) -> Result<Triangles, String> {
        let cone = Cone {
            surface: self.surface,
            radius,
            slope,
        };
        if let [ring] = self.loops.as_slice()
            && let Some(mesh) = cone.apex_fan(ring, edges, samples, epsilon, self.same_sense)?
        {
            return Ok(mesh);
        }
        let rings = self
            .loops
            .iter()
            .map(|ring| cone.chart_ring(ring, edges, samples, epsilon))
            .collect::<Result<Vec<_>, _>>()?;
        let mut outers = rings
            .iter()
            .filter(|ring| (ring.area > 0.0) == self.same_sense);
        let outer = outers
            .next()
            .ok_or("native cone lacks an outer chart ring")?;
        if outers.next().is_some() {
            return Err("native cone requires one outer chart ring".into());
        }
        let [lo, hi] = outer.angles;
        let tolerance = (epsilon / outer.max_radius).min(1e-8);
        if hi - lo > TAU + tolerance {
            return Err("native conical trim spans more than one period".into());
        }
        if hi - lo >= TAU - tolerance {
            let seam = |limit: f64| {
                outer
                    .generators
                    .iter()
                    .filter_map(|&(id, u)| ((u - limit).abs() <= tolerance).then_some(id))
                    .collect::<HashSet<_>>()
            };
            let lower = seam(lo);
            if lower.is_empty() || lower != seam(hi) {
                return Err("native full-period cone requires shared seam generators".into());
            }
        }
        if rings.iter().map(|r| r.samples.len()).sum::<usize>() > MAX_TRIM_SAMPLES {
            return Err("native conical trim sample limit".into());
        }
        let mut vertices = Vec::new();
        let mut ranges = Vec::new();
        for ring in &rings {
            let shift = if std::ptr::eq(ring, outer) {
                0.0
            } else {
                // The hole must have a unique lift strictly inside the outer
                // chart. Its winding and containment are then checked by CDT.
                let shift = ((lo - ring.angles[0]) / TAU).ceil() * TAU;
                if ring.angles[0] + shift <= lo || ring.angles[1] + shift >= hi {
                    return Err("native conical hole crosses the chart boundary".into());
                }
                shift
            };
            let start = vertices.len();
            vertices.extend(ring.samples.iter().map(|sample| Sample {
                uv: Point2::new(sample.uv.x + shift, sample.uv.y),
                point: sample.point,
            }));
            let points = &vertices[start..];
            if points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .any(|(a, b)| cone.angular_excess(*a, *b, deflection) > ANGLE_ROUNDOFF)
            {
                return Err("native conical trim needs more shared edge samples".into());
            }
            ranges.push(start..vertices.len());
        }
        let uv: Vec<_> = vertices
            .iter()
            .map(|p| Point2::new(p.uv.x, p.uv.y / outer.max_radius))
            .collect();
        let indices = triangulate_trim(&uv, &ranges, &self.loops, self.same_sense)?;
        cone.refine(
            indices
                .into_iter()
                .map(|triangle| triangle.map(|i| vertices[i]))
                .collect(),
            deflection,
            self.same_sense,
        )
    }
}

impl Cone {
    fn radius_at(&self, height: f64) -> f64 {
        self.radius + height * self.slope
    }

    fn point(&self, uv: Point2<f64>) -> Point3 {
        let (sin, cos) = uv.x.sin_cos();
        let radius = self.radius_at(uv.y);
        self.surface
            .frame
            .transform_point(Point3::new(radius * cos, radius * sin, uv.y))
    }

    fn apex_fan(
        &self,
        ring: &Boundary,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
        same_sense: bool,
    ) -> Result<Option<Triangles>, String> {
        if ring.edges.len() != 3 {
            return Ok(None);
        }
        let Some(start) = ring
            .edges
            .iter()
            .position(|(id, _)| matches!(edges[id].curve, Curve::Circle { .. }))
        else {
            return Ok(None);
        };
        let [(rim, forward), (seam, _), (other, _)] =
            [0, 1, 2].map(|i| ring.edges[(start + i) % ring.edges.len()]);
        let circle = &edges[&rim];
        if seam != other || circle.vertices[0] != circle.vertices[1] {
            return Ok(None);
        }
        let edge = &edges[&seam];
        if !matches!(edge.curve, Curve::Line { .. }) {
            return Ok(None);
        }
        let apex_index = usize::from(edge.vertices[0] == circle.vertices[0]);
        let apex = edge.points[apex_index];
        let local = self.surface.local(apex);
        let residual = local.x.hypot(local.y).hypot(self.radius_at(local.z));
        if !residual.is_finite() || residual > epsilon {
            return Ok(None);
        }
        let sweep = self.circular_sweep(circle, epsilon)?;
        let sweep = if forward { sweep } else { -sweep };
        let rim = &samples[&rim];
        let count = rim.len() - 1;
        let at = |i| rim[if forward { i } else { count - i }];
        let local = self.surface.local(at(0));
        if self.radius_at(local.z) <= epsilon {
            return Err("native conical apex fan lacks a positive-radius rim".into());
        }
        let start_angle = local.y.atan2(local.x);
        let mut triangles = Vec::with_capacity(count);
        // From a single full circular rim to the apex, the cone and chord
        // fan contract linearly with height. The shared rim bounds deflection.
        for i in 0..count {
            let triangle = [at(i), at(i + 1), apex];
            let mut angles = [0, 1].map(|j| {
                self.sample_angle(
                    triangle[j],
                    start_angle + sweep * (i + j) as f64 / count as f64,
                )
            });
            self.check_orientation(triangle, &mut angles, same_sense)?;
            triangles.push(triangle);
        }
        Ok(Some(triangles))
    }

    fn chart_ring(
        &self,
        ring: &Boundary,
        edges: &HashMap<u64, Edge>,
        samples: &Samples,
        epsilon: f64,
    ) -> Result<Ring, String> {
        let &(first_id, forward) = ring.edges.first().ok_or("empty native conical ring")?;
        let first = &samples[&first_id];
        let start = self.surface.local(if forward {
            first[0]
        } else {
            first[first.len() - 1]
        });
        let start_angle = start.y.atan2(start.x);
        let mut u = start_angle;
        let mut vertices = Vec::new();
        let mut generators = Vec::new();
        let mut max_radius: f64 = 0.0;
        let mut height = start.z;
        for &(id, forward) in &ring.edges {
            let span = &samples[&id];
            let at = |i| span[if forward { i } else { span.len() - 1 - i }];
            let end_height = self.surface.local(at(span.len() - 1)).z;
            let sweep = match edges[&id].curve {
                Curve::Line { .. } => {
                    generators.push((id, u));
                    0.0
                }
                Curve::Circle { .. } => {
                    let sweep = self.circular_sweep(&edges[&id], epsilon)?;
                    if forward { sweep } else { -sweep }
                }
                Curve::Ellipse { .. } => return Err("unsupported native conical trim curve".into()),
            };
            let next_height = if sweep == 0.0 { end_height } else { height };
            let count = span.len() - 1;
            if vertices.len() + count > MAX_TRIM_SAMPLES {
                return Err("native conical trim sample limit".into());
            }
            // Include the end in the support check even when the following
            // edge has a different chart direction.
            for i in 0..=count {
                let uv = if i == count {
                    Point2::new(u + sweep, next_height)
                } else {
                    let fraction = i as f64 / count as f64;
                    Point2::new(
                        u + sweep * fraction,
                        height + (next_height - height) * fraction,
                    )
                };
                let radius = self.radius_at(uv.y);
                if !radius.is_finite() || radius <= epsilon {
                    return Err("native conical trim reaches the apex or negative nappe".into());
                }
                let point = at(i);
                let residual = (point - self.point(uv)).magnitude();
                if !residual.is_finite() || residual > epsilon {
                    return Err("source boundary is off its conical chart".into());
                }
                max_radius = max_radius.max(radius);
                if i < count {
                    vertices.push(Sample { uv, point });
                }
            }
            u += sweep;
            height = next_height;
        }
        if (u - start_angle).abs() * max_radius > epsilon || (height - start.z).abs() > epsilon {
            return Err("native conical trim does not close in an unwrapped chart".into());
        }
        let origin = vertices[0].uv;
        let mut angles = [origin.x, origin.x];
        let mut area = 0.0;
        for (i, sample) in vertices.iter().enumerate() {
            angles[0] = angles[0].min(sample.uv.x);
            angles[1] = angles[1].max(sample.uv.x);
            let a = sample.uv - origin;
            let b = vertices[(i + 1) % vertices.len()].uv - origin;
            area += a.x * b.y - a.y * b.x;
        }
        Ok(Ring {
            samples: vertices,
            angles,
            generators,
            max_radius,
            area,
        })
    }

    fn circular_sweep(&self, edge: &Edge, epsilon: f64) -> Result<f64, String> {
        let Curve::Circle {
            frame,
            radius,
            forward,
        } = edge.curve
        else {
            return Err("missing native conical circle".into());
        };
        let center = self.surface.local(Point3::from_vec(frame.w.truncate()));
        let axis = self.surface.frame.z.truncate();
        let normal = frame.z.truncate();
        // Bound attachment over the entire carrier, including between samples.
        let residual = center.x.hypot(center.y)
            + (self.radius_at(center.z) - radius).abs()
            + radius * normal.cross(axis).magnitude() * (1.0 + self.slope);
        if !residual.is_finite() || residual > epsilon {
            return Err("source circular boundary is off its CONICAL_SURFACE".into());
        }
        let (_, sweep) = edge.interval(frame, radius, forward, epsilon)?;
        Ok(sweep * normal.dot(axis).signum())
    }

    fn refine(
        &self,
        mut pending: Vec<[Sample; 3]>,
        deflection: f64,
        same_sense: bool,
    ) -> Result<Triangles, String> {
        let mut triangles = Vec::new();
        while let Some(triangle) = pending.pop() {
            let excess = [0, 1, 2]
                .map(|i| self.angular_excess(triangle[i], triangle[(i + 1) % 3], deflection));
            let split = (0..3)
                .max_by(|&a, &b| excess[a].total_cmp(&excess[b]))
                .unwrap();
            // Intersect the three angular intervals centered on the vertices.
            // Projecting on a common direction loses at most the deflection at
            // each vertex. The cone radius and the projection are both affine
            // on the chord triangle, so this bounds its full radial deficit.
            if excess[split] <= ANGLE_ROUNDOFF {
                let mut angles = triangle.map(|p| self.sample_angle(p.point, p.uv.x));
                let triangle = triangle.map(|sample| sample.point);
                self.check_orientation(triangle, &mut angles, same_sense)?;
                triangles.push(triangle);
                continue;
            }
            if pending.len() + triangles.len() >= 32768 {
                return Err("native conical trim refinement limit".into());
            }
            let [a, b, c] = [0, 1, 2].map(|i| triangle[(split + i) % 3]);
            // Canonical averaging makes both incident triangles split at the
            // same point. Authored circles and generators already fit.
            let uv = Point2::new((a.uv.x + b.uv.x) * 0.5, (a.uv.y + b.uv.y) * 0.5);
            let mid = Sample {
                uv,
                point: self.point(uv),
            };
            pending.extend([[a, mid, c], [mid, b, c]]);
        }
        Ok(triangles)
    }

    fn angular_excess(&self, a: Sample, b: Sample, deflection: f64) -> f64 {
        let half_step = |p: Sample| circle_step(self.radius_at(p.uv.y), deflection) * 0.5;
        (a.uv.x - b.uv.x).abs() - (half_step(a) + half_step(b))
    }

    fn sample_angle(&self, point: Point3, near: f64) -> f64 {
        let local = self.surface.local(point);
        let angle = local.y.atan2(local.x);
        angle + ((near - angle) / TAU).round() * TAU
    }

    fn check_orientation(
        &self,
        [a, b, c]: [Point3; 3],
        angles: &mut [f64],
        same_sense: bool,
    ) -> Result<(), String> {
        let normal = self.surface.local_direction((b - a).cross(c - a))
            * if same_sense { 1.0 } else { -1.0 };
        angles.sort_by(f64::total_cmp);
        let (lo, hi) = (angles[0], angles[angles.len() - 1]);
        if hi - lo > ANGULAR_TOLERANCE + ANGLE_ROUNDOFF {
            return Err("native conical triangle exceeds angular tolerance".into());
        }
        // The small angular span keeps the chord points' azimuths between
        // these limits, apart from the singular tip. Check the minimum normal dot product.
        let opposite = normal.y.atan2(normal.x) + std::f64::consts::PI;
        let minimum = lo + (opposite - lo).rem_euclid(TAU);
        for u in [lo, hi, minimum.min(hi)] {
            let alignment = normal.x * u.cos() + normal.y * u.sin() - normal.z * self.slope;
            if !alignment.is_finite() || alignment <= 0.0 {
                return Err("native conical chart folds when lifted to its support".into());
            }
        }
        Ok(())
    }
}
