//! Source-only line/arc containment; classification never changes an authored curve.
use super::*;

// Match the exact PCB consumer's XY equality budget; this permits refusal, not repair.
const POSITION_EPS_MM: f64 = 1e-10;

#[derive(Clone, Copy, Debug)]
struct ArcSeg {
    start: Vec2,
    end: Vec2,
    center: Vec2,
    radius: f64,
    start_angle: f64,
    sweep: f64,
}

#[derive(Clone, Copy, Debug)]
enum Seg {
    Line { start: Vec2, end: Vec2 },
    Arc(ArcSeg),
}

impl Seg {
    fn start(&self) -> Vec2 {
        match *self {
            Self::Line { start, .. } => start,
            Self::Arc(arc) => arc.start,
        }
    }

    fn end(&self) -> Vec2 {
        match *self {
            Self::Line { end, .. } => end,
            Self::Arc(arc) => arc.end,
        }
    }

    fn midpoint(&self) -> Vec2 {
        match *self {
            Self::Line { start, end } => Vec2 {
                x: 0.5 * (start.x + end.x),
                y: 0.5 * (start.y + end.y),
            },
            Self::Arc(arc) => arc.point_at(0.5),
        }
    }

    fn point_at(&self, t: f64) -> Vec2 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Self::Line { start, end } => Vec2 {
                x: start.x + (end.x - start.x) * t,
                y: start.y + (end.y - start.y) * t,
            },
            Self::Arc(arc) => arc.point_at(t),
        }
    }

    fn tangent_at_start(&self) -> Vec2 {
        match *self {
            Self::Line { start, end } => unit(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }),
            Self::Arc(arc) => {
                let sign = arc.sweep.signum();
                Vec2 {
                    x: -arc.start_angle.sin() * sign,
                    y: arc.start_angle.cos() * sign,
                }
            }
        }
    }

    fn tangent_at_end(&self) -> Vec2 {
        match *self {
            Self::Line { start, end } => unit(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }),
            Self::Arc(arc) => {
                let end_angle = arc.start_angle + arc.sweep;
                let sign = arc.sweep.signum();
                Vec2 {
                    x: -end_angle.sin() * sign,
                    y: end_angle.cos() * sign,
                }
            }
        }
    }

    fn aabb(&self) -> (Vec2, Vec2) {
        let mut min = Vec2 {
            x: self.start().x.min(self.end().x),
            y: self.start().y.min(self.end().y),
        };
        let mut max = Vec2 {
            x: self.start().x.max(self.end().x),
            y: self.start().y.max(self.end().y),
        };
        if let Self::Arc(arc) = *self {
            for cardinal in [
                0.0,
                std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
                3.0 * std::f64::consts::FRAC_PI_2,
            ] {
                if arc.contains_angle(cardinal) {
                    let p = Vec2 {
                        x: arc.center.x + arc.radius * cardinal.cos(),
                        y: arc.center.y + arc.radius * cardinal.sin(),
                    };
                    min.x = min.x.min(p.x);
                    min.y = min.y.min(p.y);
                    max.x = max.x.max(p.x);
                    max.y = max.y.max(p.y);
                }
            }
        }
        (min, max)
    }
}

impl ArcSeg {
    fn point_at(&self, t: f64) -> Vec2 {
        if t <= 0.0 {
            return self.start;
        }
        if t >= 1.0 {
            return self.end;
        }
        let angle = self.start_angle + self.sweep * t;
        Vec2 {
            x: self.center.x + self.radius * angle.cos(),
            y: self.center.y + self.radius * angle.sin(),
        }
    }

    fn contains_angle(&self, angle: f64) -> bool {
        angle_in_sweep(angle, self.start_angle, self.sweep)
    }

    fn clamp_angle(&self, angle: f64) -> Vec2 {
        if self.contains_angle(angle) {
            Vec2 {
                x: self.center.x + self.radius * angle.cos(),
                y: self.center.y + self.radius * angle.sin(),
            }
        } else {
            let p = Vec2 {
                x: self.center.x + self.radius * angle.cos(),
                y: self.center.y + self.radius * angle.sin(),
            };
            if dist(p, self.start) <= dist(p, self.end) {
                self.start
            } else {
                self.end
            }
        }
    }
}

struct Ring {
    segments: Vec<Seg>,
    min: Vec2,
    max: Vec2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Relation {
    FirstInsideSecond,
    SecondInsideFirst,
    Disjoint,
    IntersectOrTouch,
}

pub fn assign(contours: &mut [CasePcbSourceContour]) -> Result<(), ArtifactError> {
    // Keep the existing ModuleBoard contour budget at the source boundary too.
    if contours.len() > 1000 {
        return Err(validation("PCB source exceeds the 1000-contour limit"));
    }
    let rings = contours.iter().map(view).collect::<Result<Vec<_>, _>>()?;
    let mut parents = vec![Vec::new(); rings.len()];
    for i in 0..rings.len() {
        for j in i + 1..rings.len() {
            match relate(&rings[i], &rings[j]) {
                Relation::FirstInsideSecond => parents[i].push(j),
                Relation::SecondInsideFirst => parents[j].push(i),
                Relation::Disjoint => {}
                Relation::IntersectOrTouch => {
                    return Err(validation(format!(
                        "PCB source contours '{}' and '{}' intersect, touch or have ambiguous ownership",
                        contours[i].id, contours[j].id
                    )));
                }
            }
        }
    }
    for i in 0..contours.len() {
        if parents[i].len() % 2 == 0 {
            contours[i].outer_id = None;
            continue;
        }
        let owners: Vec<_> = parents[i]
            .iter()
            .filter(|&&parent| parents[parent].len() + 1 == parents[i].len())
            .copied()
            .collect();
        if owners.len() != 1 {
            return Err(validation(format!(
                "PCB source contour '{}' has ambiguous material ownership",
                contours[i].id
            )));
        }
        contours[i].outer_id = Some(contours[owners[0]].id.clone());
    }
    Ok(())
}

fn view(contour: &CasePcbSourceContour) -> Result<Ring, ArtifactError> {
    let mut vertices: Vec<(Vec2, f64)> = Vec::new();
    for primitive in &contour.primitives {
        match &primitive.geometry {
            MechanicalShape::Line { start, .. } => vertices.push((*start, 0.0)),
            MechanicalShape::Circle { center, end, .. } if contour.primitives.len() == 1 => {
                vertices.push((*end, 1.0));
                vertices.push((
                    Vec2 {
                        x: center.x - (end.x - center.x),
                        y: center.y - (end.y - center.y),
                    },
                    1.0,
                ));
            }
            MechanicalShape::Arc { start, .. } => {
                let support = primitive.curve_support.ok_or_else(|| {
                    validation(format!(
                        "PCB arc '{}' has no authored support",
                        primitive.source_id
                    ))
                })?;
                let count = (support.sweep_radians.abs() / std::f64::consts::PI).ceil() as usize;
                let span = support.sweep_radians / count as f64;
                let bulge = if span.abs() == std::f64::consts::PI {
                    span.signum()
                } else {
                    (span / 4.0).tan()
                };
                for index in 0..count {
                    let point = if index == 0 {
                        *start
                    } else {
                        support_point(
                            &support,
                            support.start_direction.y.atan2(support.start_direction.x)
                                + span * index as f64,
                        )
                    };
                    vertices.push((point, bulge));
                }
            }
            _ => {
                return Err(validation(format!(
                    "PCB contour '{}' is not normalized",
                    contour.id
                )));
            }
        }
    }
    if vertices.len() < 2 {
        return Err(validation(format!(
            "PCB source contour '{}' is degenerate, self-crossing or self-touching",
            contour.id
        )));
    }
    let mut segments = Vec::with_capacity(vertices.len());
    let mut min = Vec2 {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };
    let mut max = Vec2 {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };
    for i in 0..vertices.len() {
        let (start, bulge) = vertices[i];
        let (end, _) = vertices[(i + 1) % vertices.len()];
        if dist(start, end) <= POSITION_EPS_MM {
            return Err(validation(format!(
                "PCB source contour '{}' is degenerate, self-crossing or self-touching",
                contour.id
            )));
        }
        let seg = if bulge == 0.0 {
            Seg::Line { start, end }
        } else {
            let Some(arc) = arc_from_bulge(start, end, bulge) else {
                return Err(validation(format!(
                    "PCB source contour '{}' is degenerate, self-crossing or self-touching",
                    contour.id
                )));
            };
            Seg::Arc(arc)
        };
        let (smin, smax) = seg.aabb();
        min.x = min.x.min(smin.x);
        min.y = min.y.min(smin.y);
        max.x = max.x.max(smax.x);
        max.y = max.y.max(smax.y);
        segments.push(seg);
    }
    let area = signed_area(&segments);
    if !area.is_finite() || area.abs() <= 1e-12 || has_self_intersection(&segments) {
        return Err(validation(format!(
            "PCB source contour '{}' is degenerate, self-crossing or self-touching",
            contour.id
        )));
    }
    Ok(Ring { segments, min, max })
}

fn arc_from_bulge(start: Vec2, end: Vec2, bulge: f64) -> Option<ArcSeg> {
    let chord_dx = end.x - start.x;
    let chord_dy = end.y - start.y;
    let chord = chord_dx.hypot(chord_dy);
    if !chord.is_finite() || chord <= POSITION_EPS_MM || !bulge.is_finite() {
        return None;
    }
    let sweep = 4.0 * bulge.atan();
    let radius = chord * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
    if !radius.is_finite() || radius <= POSITION_EPS_MM {
        return None;
    }
    let mid_x = 0.5 * (start.x + end.x);
    let mid_y = 0.5 * (start.y + end.y);
    let cot_quarter = (1.0 - bulge * bulge) / (2.0 * bulge);
    let center = Vec2 {
        x: mid_x - 0.5 * chord_dy * cot_quarter,
        y: mid_y + 0.5 * chord_dx * cot_quarter,
    };
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    Some(ArcSeg {
        start,
        end,
        center,
        radius,
        start_angle,
        sweep,
    })
}

fn signed_area(segments: &[Seg]) -> f64 {
    let mut area2 = 0.0;
    for seg in segments {
        let a = seg.start();
        let b = seg.end();
        area2 += a.x * b.y - a.y * b.x;
        if let Seg::Arc(arc) = *seg {
            area2 += arc.radius * arc.radius * (arc.sweep - arc.sweep.sin());
        }
    }
    0.5 * area2
}

fn has_self_intersection(segments: &[Seg]) -> bool {
    let n = segments.len();
    for i in 0..n {
        for j in i + 1..n {
            let adjacent = j == i + 1 || (i == 0 && j + 1 == n);
            if adjacent {
                if adjacent_segments_conflict(&segments[i], &segments[j], i == 0 && j + 1 == n) {
                    return true;
                }
            } else if segments_min_dist(&segments[i], &segments[j]) <= POSITION_EPS_MM {
                return true;
            }
        }
    }
    false
}

fn adjacent_segments_conflict(first: &Seg, second: &Seg, wrap: bool) -> bool {
    let (incoming, outgoing) = if wrap {
        (second, first)
    } else {
        (first, second)
    };
    let joint = incoming.end();
    // Check if the two segments reversed back along the same tangent at the joint
    let t_in = incoming.tangent_at_end();
    let t_out = outgoing.tangent_at_start();
    let reverse_dot = (-t_in.x) * t_out.x + (-t_in.y) * t_out.y;
    if reverse_dot > 1.0 - 1e-12 {
        match (*incoming, *outgoing) {
            (Seg::Line { .. }, Seg::Line { .. }) => return true,
            (Seg::Arc(a), Seg::Arc(b))
                if dist(a.center, b.center) <= POSITION_EPS_MM
                    && (a.radius - b.radius).abs() <= POSITION_EPS_MM
                    && a.sweep.signum() != b.sweep.signum() =>
            {
                return true;
            }
            _ => {}
        }
    }
    // Check if they touch or cross anywhere away from `joint`
    // In a 2-segment closed loop, both endpoints are shared; check interior points and curve intersections
    let mut interior_points = Vec::new();
    for &t in &[0.2, 0.5, 0.8] {
        let p = incoming.point_at(t);
        if dist(p, incoming.start()) > POSITION_EPS_MM * 10.0
            && dist(p, incoming.end()) > POSITION_EPS_MM * 10.0
        {
            interior_points.push((p, outgoing));
        }
        let q = outgoing.point_at(t);
        if dist(q, outgoing.start()) > POSITION_EPS_MM * 10.0
            && dist(q, outgoing.end()) > POSITION_EPS_MM * 10.0
        {
            interior_points.push((q, incoming));
        }
    }
    for (p, other) in interior_points {
        if point_to_seg_dist(p, other) <= POSITION_EPS_MM {
            return true;
        }
    }
    for pt in exact_intersection_candidates(incoming, outgoing) {
        let is_joint = dist(pt, joint) <= 1e-7
            || (dist(incoming.start(), outgoing.end()) <= POSITION_EPS_MM
                && dist(pt, incoming.start()) <= 1e-7);
        if !is_joint
            && point_to_seg_dist(pt, incoming) <= POSITION_EPS_MM
            && point_to_seg_dist(pt, outgoing) <= POSITION_EPS_MM
        {
            return true;
        }
    }
    false
}

fn relate(a: &Ring, b: &Ring) -> Relation {
    if a.max.x + POSITION_EPS_MM < b.min.x
        || b.max.x + POSITION_EPS_MM < a.min.x
        || a.max.y + POSITION_EPS_MM < b.min.y
        || b.max.y + POSITION_EPS_MM < a.min.y
    {
        return Relation::Disjoint;
    }
    for sa in &a.segments {
        for sb in &b.segments {
            if segments_min_dist(sa, sb) <= POSITION_EPS_MM {
                return Relation::IntersectOrTouch;
            }
        }
    }
    let a_in_b = ring_contains_point(b, a.segments[0].midpoint());
    let b_in_a = ring_contains_point(a, b.segments[0].midpoint());
    match (a_in_b, b_in_a) {
        (true, false) => Relation::FirstInsideSecond,
        (false, true) => Relation::SecondInsideFirst,
        (false, false) => Relation::Disjoint,
        (true, true) => Relation::IntersectOrTouch,
    }
}

fn ring_contains_point(ring: &Ring, p: Vec2) -> bool {
    let mut total_angle = 0.0;
    for seg in &ring.segments {
        match *seg {
            Seg::Line { start, end } => {
                total_angle += winding_step(p, start, end);
            }
            Seg::Arc(arc) => {
                // Subdivide the arc so the chord sagitta is strictly less than half the distance from p to the arc.
                let d_min = point_to_arc_dist(p, &arc).max(POSITION_EPS_MM);
                let steps = ((arc.sweep.abs() * (arc.radius / d_min).sqrt()).ceil() as usize)
                    .clamp(16, 4096);
                let mut prev = arc.start;
                for k in 1..=steps {
                    let next = if k == steps {
                        arc.end
                    } else {
                        arc.point_at(k as f64 / steps as f64)
                    };
                    total_angle += winding_step(p, prev, next);
                    prev = next;
                }
            }
        }
    }
    (total_angle / std::f64::consts::TAU).abs() > 0.5
}

fn winding_step(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let ux = a.x - p.x;
    let uy = a.y - p.y;
    let vx = b.x - p.x;
    let vy = b.y - p.y;
    (ux * vy - uy * vx).atan2(ux * vx + uy * vy)
}

fn segments_min_dist(a: &Seg, b: &Seg) -> f64 {
    let (amin, amax) = a.aabb();
    let (bmin, bmax) = b.aabb();
    let dx = (bmin.x - amax.x).max(amin.x - bmax.x).max(0.0);
    let dy = (bmin.y - amax.y).max(amin.y - bmax.y).max(0.0);
    if dx.hypot(dy) > POSITION_EPS_MM * 100.0 {
        return dx.hypot(dy);
    }

    let mut best = f64::INFINITY;
    for p in [a.start(), a.end(), a.midpoint()] {
        best = best.min(point_to_seg_dist(p, b));
    }
    for q in [b.start(), b.end(), b.midpoint()] {
        best = best.min(point_to_seg_dist(q, a));
    }
    for candidate in exact_intersection_candidates(a, b) {
        let d = point_to_seg_dist(candidate, a).max(point_to_seg_dist(candidate, b));
        best = best.min(d);
    }
    match (*a, *b) {
        (Seg::Line { start, end }, Seg::Arc(arc)) | (Seg::Arc(arc), Seg::Line { start, end }) => {
            let proj = closest_point_on_line(arc.center, start, end);
            best = best.min(point_to_arc_dist(proj, &arc));
            let angle = (proj.y - arc.center.y).atan2(proj.x - arc.center.x);
            for ang in [angle, angle + std::f64::consts::PI] {
                let p_arc = arc.clamp_angle(ang);
                best = best.min(point_to_line_dist(p_arc, start, end));
            }
        }
        (Seg::Arc(a1), Seg::Arc(a2)) => {
            let d_centers = dist(a1.center, a2.center);
            if d_centers <= POSITION_EPS_MM && (a1.radius - a2.radius).abs() <= POSITION_EPS_MM {
                // Concentric equal-radius arcs: check if angular intervals overlap
                for ang in [
                    a1.start_angle,
                    a1.start_angle + a1.sweep,
                    a1.start_angle + 0.5 * a1.sweep,
                    a2.start_angle,
                    a2.start_angle + a2.sweep,
                    a2.start_angle + 0.5 * a2.sweep,
                ] {
                    if a1.contains_angle(ang) && a2.contains_angle(ang) {
                        return 0.0;
                    }
                }
            } else if d_centers > 0.0 {
                let dir12 = (a2.center.y - a1.center.y).atan2(a2.center.x - a1.center.x);
                for ang in [dir12, dir12 + std::f64::consts::PI] {
                    let p1 = a1.clamp_angle(ang);
                    best = best.min(point_to_arc_dist(p1, &a2));
                    let p2 = a2.clamp_angle(ang);
                    best = best.min(point_to_arc_dist(p2, &a1));
                }
            }
        }
        (Seg::Line { .. }, Seg::Line { .. }) => {}
    }
    best
}

fn exact_intersection_candidates(a: &Seg, b: &Seg) -> Vec<Vec2> {
    let mut out = Vec::new();
    match (*a, *b) {
        (Seg::Line { start: a0, end: a1 }, Seg::Line { start: b0, end: b1 }) => {
            let ux = a1.x - a0.x;
            let uy = a1.y - a0.y;
            let vx = b1.x - b0.x;
            let vy = b1.y - b0.y;
            let det = ux * vy - uy * vx;
            if det.abs() > 1e-18 {
                let wx = b0.x - a0.x;
                let wy = b0.y - a0.y;
                let t = (wx * vy - wy * vx) / det;
                let u = (wx * uy - wy * ux) / det;
                if (-1e-7..=1.0 + 1e-7).contains(&t) && (-1e-7..=1.0 + 1e-7).contains(&u) {
                    out.push(Vec2 {
                        x: a0.x + t.clamp(0.0, 1.0) * ux,
                        y: a0.y + t.clamp(0.0, 1.0) * uy,
                    });
                }
            }
        }
        (Seg::Line { start, end }, Seg::Arc(arc)) | (Seg::Arc(arc), Seg::Line { start, end }) => {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let len2 = dx * dx + dy * dy;
            if len2 > 0.0 {
                let fx = start.x - arc.center.x;
                let fy = start.y - arc.center.y;
                let b_coef = 2.0 * (fx * dx + fy * dy);
                let c_coef = fx * fx + fy * fy - arc.radius * arc.radius;
                let disc = b_coef * b_coef - 4.0 * len2 * c_coef;
                if disc >= -1e-9 * len2 {
                    let root = disc.max(0.0).sqrt();
                    for s in [-1.0, 1.0] {
                        let t = (-b_coef + s * root) / (2.0 * len2);
                        let p = Vec2 {
                            x: start.x + t.clamp(0.0, 1.0) * dx,
                            y: start.y + t.clamp(0.0, 1.0) * dy,
                        };
                        out.push(p);
                    }
                }
            }
        }
        (Seg::Arc(a1), Seg::Arc(a2)) => {
            let dx = a2.center.x - a1.center.x;
            let dy = a2.center.y - a1.center.y;
            let d = dx.hypot(dy);
            if d > 1e-12
                && d <= a1.radius + a2.radius + 1e-7
                && d + 1e-7 >= (a1.radius - a2.radius).abs()
            {
                let a_dist = (a1.radius * a1.radius - a2.radius * a2.radius + d * d) / (2.0 * d);
                let h2 = a1.radius * a1.radius - a_dist * a_dist;
                let h = h2.max(0.0).sqrt();
                let px = a1.center.x + a_dist * (dx / d);
                let py = a1.center.y + a_dist * (dy / d);
                let rx = -dy * (h / d);
                let ry = dx * (h / d);
                out.push(Vec2 {
                    x: px + rx,
                    y: py + ry,
                });
                out.push(Vec2 {
                    x: px - rx,
                    y: py - ry,
                });
            }
        }
    }
    out
}

fn point_to_seg_dist(p: Vec2, seg: &Seg) -> f64 {
    match *seg {
        Seg::Line { start, end } => point_to_line_dist(p, start, end),
        Seg::Arc(arc) => point_to_arc_dist(p, &arc),
    }
}

fn closest_point_on_line(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len2 = dx * dx + dy * dy;
    if len2 <= 0.0 {
        return a;
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    Vec2 {
        x: a.x + t * dx,
        y: a.y + t * dy,
    }
}

fn point_to_line_dist(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    dist(p, closest_point_on_line(p, a, b))
}

fn point_to_arc_dist(p: Vec2, arc: &ArcSeg) -> f64 {
    let angle = (p.y - arc.center.y).atan2(p.x - arc.center.x);
    if arc.contains_angle(angle) {
        (dist(p, arc.center) - arc.radius).abs()
    } else {
        dist(p, arc.start).min(dist(p, arc.end))
    }
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn unit(v: Vec2) -> Vec2 {
    let len = v.x.hypot(v.y);
    if len > 0.0 {
        Vec2 {
            x: v.x / len,
            y: v.y / len,
        }
    } else {
        Vec2 { x: 1.0, y: 0.0 }
    }
}
