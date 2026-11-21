//! Private whole-surface proof for axis-aligned cylindrical extrusions and caps.
//!
//! See PROOF.md beside this file. All predicates and error accounting use exact
//! rationals of the retained f64 geometry. No mesher tolerance is a proof input.
use super::super::generated_qualification::GeneratedQualification;
use super::*;
use monstertruck_modeling::{Curve, Point3, Vector3};
use num_rational::BigRational as Q;
use num_traits::{Signed, ToPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet, HashSet};

type P = [Q; 3];
type Edge = (P, P);
type Proof<T> = Result<T, &'static str>;
fn n(v: i64) -> Q {
    Q::from_integer(v.into())
}
fn q(v: f64) -> Proof<Q> {
    if !v.is_finite() || v.abs() > 1e12 || (v != 0. && v.abs() < 1e-100) {
        return Err("numeric domain limit");
    }
    Q::from_float(v).ok_or("nonfinite source/final coordinate")
}
fn point(p: Point3) -> Proof<P> {
    Ok([q(p.x)?, q(p.y)?, q(p.z)?])
}
fn sub(a: &P, b: &P) -> P {
    std::array::from_fn(|i| &a[i] - &b[i])
}
fn cross(a: &P, b: &P) -> P {
    [
        &a[1] * &b[2] - &a[2] * &b[1],
        &a[2] * &b[0] - &a[0] * &b[2],
        &a[0] * &b[1] - &a[1] * &b[0],
    ]
}
fn det(a: &P, b: &P) -> Q {
    &a[0] * &b[1] - &a[1] * &b[0]
}
fn dot2(a: &P, b: &P) -> Q {
    &a[0] * &b[0] + &a[1] * &b[1]
}
fn centered(p: &P, c: &[Q; 2]) -> P {
    [&p[0] - &c[0], &p[1] - &c[1], n(0)]
}
fn edges(ring: &[P]) -> impl Iterator<Item = (&P, &P)> {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
}
fn edge_key(a: &P, b: &P) -> Edge {
    if a < b {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    }
}
fn edge_set(ring: &[P]) -> BTreeSet<Edge> {
    edges(ring).map(|(a, b)| edge_key(a, b)).collect()
}

// Oriented triangle chains, not an area comparison: internal edges cancel once;
// remaining degree-two cycles are the complete boundary, including every hole.
fn boundary(triangles: &[[P; 3]]) -> Proof<Vec<Vec<P>>> {
    let mut uses: BTreeMap<Edge, (usize, i32)> = BTreeMap::new();
    for tri in triangles {
        for (a, b) in edges(tri) {
            if a == b {
                return Err("zero edge");
            }
            let entry = uses.entry(edge_key(a, b)).or_default();
            entry.0 += 1;
            entry.1 += if a < b { 1 } else { -1 };
        }
    }
    let mut outgoing = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for ((a, b), (count, sign)) in uses {
        match (count, sign) {
            (2, 0) => (),
            (1, 1) | (1, -1) => {
                let (a, b) = if sign == 1 { (a, b) } else { (b, a) };
                if outgoing.insert(a, b.clone()).is_some() || !incoming.insert(b) {
                    return Err("branched boundary");
                }
            }
            _ => return Err("duplicate or inconsistent triangle chain"),
        }
    }
    if outgoing.keys().cloned().collect::<BTreeSet<_>>() != incoming {
        return Err("open boundary");
    }
    let mut rings = Vec::new();
    while let Some(start) = outgoing.keys().next().cloned() {
        let mut ring = vec![start.clone()];
        let mut at = outgoing.remove(&start).ok_or("boundary start")?;
        while at != start {
            let next = outgoing.remove(&at).ok_or("boundary cycle")?;
            ring.push(at);
            at = next;
        }
        if ring.len() < 3 {
            return Err("sliver boundary");
        }
        rings.push(ring);
    }
    if rings.is_empty() {
        return Err("missing face boundary");
    }
    Ok(rings)
}

// A single angular turn is authenticated without atan2, pi, normalization or UV
// modulo arithmetic. Positive minor steps and winding one prohibit retracing.
fn turn(ring: &[P], center: &[Q; 2]) -> Proof<i32> {
    let radial = ring.iter().map(|p| centered(p, center)).collect::<Vec<_>>();
    let mut winding = 0;
    let mut sign = 0;
    for (a, b) in edges(&radial) {
        let d = det(a, b);
        if d.is_zero() || dot2(a, b) < n(0) {
            return Err("unproven angular seam/span");
        }
        let s = if d > n(0) { 1 } else { -1 };
        if sign != 0 && sign != s {
            return Err("angular retrace");
        }
        sign = s;
        if a[1] <= n(0) && b[1] > n(0) && d > n(0) {
            winding += 1;
        }
        if b[1] <= n(0) && a[1] > n(0) && d < n(0) {
            winding -= 1;
        }
    }
    if winding != sign {
        return Err("incomplete angular turn");
    }
    Ok(sign)
}
fn segment_radius2(a: &P, b: &P) -> Q {
    let d = sub(b, a);
    let length = dot2(&d, &d);
    if length.is_zero() {
        return dot2(a, a);
    }
    let t = (-dot2(a, &d) / length).max(n(0)).min(n(1));
    let p = std::array::from_fn(|i| &a[i] + &t * &d[i]);
    dot2(&p, &p)
}
fn triangle_radial_error(tri: &[P; 3], center: &[Q; 2], radius: &Q) -> Proof<Q> {
    let p = tri.each_ref().map(|v| centered(v, center));
    let signs = edges(&p).map(|(a, b)| det(a, b)).collect::<Vec<_>>();
    if signs.iter().all(|v| v >= &n(0)) || signs.iter().all(|v| v <= &n(0)) {
        return Err("triangle reaches cylinder axis");
    }
    let lower = edges(&p)
        .map(|(a, b)| segment_radius2(a, b))
        .min()
        .ok_or("no radial edges")?;
    let upper = p
        .iter()
        .map(|v| dot2(v, v))
        .max()
        .ok_or("no radial vertices")?;
    let r2 = radius * radius;
    // |sqrt(s)-r| = |s-r^2|/(sqrt(s)+r) <= |s-r^2|/r.
    Ok((&lower - &r2).abs().max((&upper - &r2).abs()) / radius)
}

fn choose(n: usize, k: usize) -> i64 {
    (0..k).fold(1, |v, i| v * (n - i) as i64 / (i + 1) as i64)
}
fn product(a: &[Q], b: &[Q]) -> Vec<Q> {
    let (m, d) = (a.len() - 1, b.len() - 1);
    (0..=m + d)
        .map(|k| {
            (0..=m)
                .filter(|&i| k >= i && k - i <= d)
                .map(|i| {
                    &a[i] * &b[k - i] * n(choose(m, i) * choose(d, k - i)) / n(choose(m + d, k))
                })
                .sum()
        })
        .collect()
}
// Exact homogeneous Cox-de Boor evaluation; only quadratic clamped native
// curves are accepted. Three exact values extract each polynomial Bezier span.
fn spline_value(knots: &[Q], control: &[[Q; 4]], t: &Q) -> [Q; 4] {
    let last = &knots[knots.len() - 1];
    let mut basis = (0..knots.len() - 1)
        .map(|i| {
            n(i64::from(
                (&knots[i] <= t && t < &knots[i + 1])
                    || (t == last && &knots[i] < t && &knots[i + 1] == t),
            ))
        })
        .collect::<Vec<_>>();
    for rank in 1..=2 {
        basis = (0..basis.len() - 1)
            .map(|i| {
                let left = &knots[i + rank] - &knots[i];
                let right = &knots[i + rank + 1] - &knots[i + 1];
                let a = if left.is_zero() {
                    n(0)
                } else {
                    (t - &knots[i]) * &basis[i] / left
                };
                let b = if right.is_zero() {
                    n(0)
                } else {
                    (&knots[i + rank + 1] - t) * &basis[i + 1] / right
                };
                a + b
            })
            .collect();
    }
    std::array::from_fn(|axis| control.iter().zip(&basis).map(|(p, b)| &p[axis] * b).sum())
}

#[derive(Clone)]
struct Circle {
    center: [Q; 2],
    radius: Q,
    z: [Q; 2],
    source_error: Q,
    source_edges: Vec<HashSet<monstertruck_modeling::EdgeId>>,
    source_levels: Vec<Q>,
    mesh_rings: Vec<Vec<P>>,
    mesh_levels: Vec<Q>,
    mesh_height: Q,
}

struct TrimEnclosure {
    radial: Q,
    height: Q,
    sign: i32,
}
impl TrimEnclosure {
    fn total(&self) -> Q {
        &self.radial + &self.height
    }
}

fn curve_ring(wire: &Wire, center: &[Q; 2], radius: &Q, z: &Q) -> Proof<TrimEnclosure> {
    let mut endpoints = Vec::new();
    let mut error = n(0);
    let mut height = n(0);
    let mut angular_sign = 0;
    for edge in wire.edge_iter() {
        // Native inversion reflects knots in f64 and can change the locus.
        // Extract the retained raw geometry before applying any orientation.
        let Curve::NurbsCurve(curve) = edge.curve() else {
            return Err("unsupported circular trim curve");
        };
        if curve.degree() != 2
            || !curve.is_clamped()
            || !(3..=64).contains(&curve.control_points().len())
        {
            return Err("unsupported circular spline domain");
        }
        let knots = curve
            .knot_vector()
            .iter()
            .copied()
            .map(q)
            .collect::<Proof<Vec<_>>>()?;
        if knots.windows(2).any(|w| w[0] > w[1]) {
            return Err("unordered curve domain");
        }
        let first_knot = &knots[0];
        let last_knot = &knots[knots.len() - 1];
        if knots.iter().take_while(|k| *k == first_knot).count() != 3
            || knots.iter().rev().take_while(|k| *k == last_knot).count() != 3
            || knots
                .windows(3)
                .any(|w| w[0] == w[2] && &w[0] != first_knot && &w[0] != last_knot)
        {
            return Err("discontinuous or unsupported trim knots");
        }
        let control = curve
            .control_points()
            .iter()
            .map(|p| Ok([q(p.x)?, q(p.y)?, q(p.z)?, q(p.w)?]))
            .collect::<Proof<Vec<_>>>()?;
        if control.iter().any(|p| p[3] <= n(0)) {
            return Err("nonpositive circular trim denominator");
        }
        let first = spline_value(&knots, &control, &knots[0]);
        let last = spline_value(&knots, &control, &knots[knots.len() - 1]);
        let dehom = |p: &[Q; 4]| -> [Q; 3] { std::array::from_fn(|i| &p[i] / &p[3]) };
        if dehom(&first) != point(edge.absolute_front().point())?
            || dehom(&last) != point(edge.absolute_back().point())?
        {
            return Err("trim endpoint mismatch");
        }
        let mut spans = Vec::new();
        for pair in knots.windows(2).filter(|w| w[0] < w[1]) {
            let a = spline_value(&knots, &control, &pair[0]);
            let c = spline_value(&knots, &control, &pair[1]);
            let mid = spline_value(&knots, &control, &((&pair[0] + &pair[1]) / n(2)));
            let b: [Q; 4] = std::array::from_fn(|i| n(2) * &mid[i] - (&a[i] + &c[i]) / n(2));
            spans.push([a, b, c]);
        }
        if !edge.orientation() {
            spans.reverse();
            for span in &mut spans {
                span.reverse();
            }
        }
        for span in spans {
            if span.iter().any(|p| p[3] <= n(0)) {
                return Err("nonpositive Bezier weight");
            }
            let x = span
                .iter()
                .map(|p| &p[0] - &center[0] * &p[3])
                .collect::<Vec<_>>();
            let y = span
                .iter()
                .map(|p| &p[1] - &center[1] * &p[3])
                .collect::<Vec<_>>();
            let w = span.iter().map(|p| p[3].clone()).collect::<Vec<_>>();
            let radial: [P; 3] = std::array::from_fn(|i| [x[i].clone(), y[i].clone(), n(0)]);
            let d = det(&radial[0], &radial[2]);
            if d.is_zero()
                || dot2(&radial[0], &radial[2]) < n(0)
                || det(&radial[0], &radial[1]) * &d < n(0)
                || det(&radial[1], &radial[2]) * &d < n(0)
            {
                return Err("source trim leaves authenticated minor sector");
            }
            let dx = x
                .windows(2)
                .map(|v| n(2) * (&v[1] - &v[0]))
                .collect::<Vec<_>>();
            let dy = y
                .windows(2)
                .map(|v| n(2) * (&v[1] - &v[0]))
                .collect::<Vec<_>>();
            let angular = product(&x, &dy)
                .iter()
                .zip(product(&y, &dx))
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>();
            let sign = if angular.iter().all(|v| v > &n(0)) {
                1
            } else if angular.iter().all(|v| v < &n(0)) {
                -1
            } else {
                return Err("unproven continuous trim turn");
            };
            if angular_sign != 0 && angular_sign != sign {
                return Err("source trim angular retrace");
            }
            angular_sign = sign;
            let xx = product(&x, &x);
            let yy = product(&y, &y);
            let ww = product(&w, &w);
            let residual = xx
                .iter()
                .zip(yy)
                .zip(ww)
                .map(|((a, b), c)| (a + b - radius * radius * c).abs())
                .max()
                .ok_or("empty radial polynomial")?;
            let minw = w.iter().min().ok_or("empty weights")?;
            error = error.max(residual / (minw * minw * radius));
            // |Z/W-z| <= max |Z_i-z W_i| / min W_i on the entire span.
            height = height.max(
                span.iter()
                    .map(|p| (&p[2] - z * &p[3]).abs())
                    .max()
                    .ok_or("empty height polynomial")?
                    / minw,
            );
            let p = dehom(&span[0]);
            if endpoints.last().is_some_and(|previous| previous != &p) {
                return Err("discontinuous source trim span");
            }
            if endpoints.is_empty() {
                endpoints.push(p);
            }
            endpoints.push(dehom(&span[2]));
        }
    }
    if endpoints.first() != endpoints.last() {
        return Err("open source circle");
    }
    endpoints.pop();
    endpoints.dedup();
    if turn(&endpoints, center)? != angular_sign {
        return Err("source circle winding");
    }
    Ok(TrimEnclosure {
        radial: error,
        height,
        sign: angular_sign,
    })
}

fn wire_ids(wire: &Wire) -> HashSet<monstertruck_modeling::EdgeId> {
    wire.edge_iter().map(|e| e.id()).collect()
}
fn cylinder(face: &RetainedFace, tris: &[[P; 3]]) -> Proof<(Circle, Q)> {
    let Surface::RevolutionSurface(s) = &face.surface else {
        return Err("not cylindrical");
    };
    let entity = s.entity();
    if entity.origin() != Point3::origin() || entity.axis() != Vector3::unit_z() || !s.orientation()
    {
        return Err("unsupported revolution normalization");
    }
    let Curve::Line(line) = entity.entity_curve() else {
        return Err("unsupported revolution generator");
    };
    if line.0.x != line.1.x || line.0.y != 0. || line.1.y != 0. || line.0.x <= 0. {
        return Err("cone or noncanonical cylinder generator");
    }
    let t = s.transform();
    for col in 0..4 {
        for row in 0..4 {
            if col == 3 && row < 3 {
                continue;
            }
            if t[col][row] != if col == row { 1. } else { 0. } {
                return Err("unsupported source affine transform");
            }
        }
    }
    let center = [q(t.w.x)?, q(t.w.y)?];
    let radius = q(line.0.x)?;
    let za = q(line.0.z)? + q(t.w.z)?;
    let zb = q(line.1.z)? + q(t.w.z)?;
    let z = [za.clone().min(zb.clone()), za.max(zb)];
    if z[0] >= z[1] || face.boundaries.len() != 2 {
        return Err("incomplete cylindrical trim domain");
    }
    let mut circle = Circle {
        center,
        radius,
        z,
        source_error: n(0),
        source_edges: vec![],
        source_levels: vec![],
        mesh_rings: boundary(tris)?,
        mesh_levels: vec![],
        mesh_height: n(0),
    };
    if circle.mesh_rings.len() != 2 {
        return Err("cylinder seam or missing patch");
    }
    let mut levels = BTreeSet::new();
    let mut source_height = n(0);
    let mut source_signs = BTreeSet::new();
    for wire in &face.boundaries {
        let level = point(wire.front_vertex().ok_or("empty circular trim")?.point())?[2].clone();
        // Line is an affine carrier: actual trim parameters may extend beyond
        // its nominal seed segment after the native face-merging operation.
        if !levels.insert(level.clone()) {
            return Err("cylinder source trim height");
        }
        let trim = curve_ring(wire, &circle.center, &circle.radius, &level)?;
        circle.source_error = circle.source_error.clone().max(trim.total());
        source_height = source_height.max(trim.height);
        source_signs.insert(trim.sign);
        circle.source_edges.push(wire_ids(wire));
        circle.source_levels.push(level);
    }
    circle.z = [
        levels.first().ok_or("missing lower source trim")?.clone(),
        levels.last().ok_or("missing upper source trim")?.clone(),
    ];
    let gap = &circle.z[1] - &circle.z[0];
    if n(2) * source_height >= gap || source_signs.len() != 2 {
        return Err("source cylinder height graphs are not separated/oriented");
    }
    levels.clear();
    for ring in &circle.mesh_rings {
        let midpoint = (&circle.z[0] + &circle.z[1]) / n(2);
        let level = if ring.iter().all(|p| p[2] < midpoint) {
            &circle.z[0]
        } else if ring.iter().all(|p| p[2] > midpoint) {
            &circle.z[1]
        } else {
            return Err("final cylinder height graphs are not separated");
        };
        if !levels.insert(level.clone()) {
            return Err("incomplete cylinder end rings");
        }
        let height = ring.iter().map(|p| (&p[2] - level).abs()).max().unwrap();
        if n(2) * &height >= gap {
            return Err("final cylinder height enclosure exceeds half interval");
        }
        circle.mesh_height = circle.mesh_height.clone().max(height);
        circle.mesh_levels.push(level.clone());
        turn(ring, &circle.center)?;
    }
    let mut error = n(0);
    let mut orientation = 0;
    for tri in tris {
        if tri.iter().any(|p| {
            p[2] < &circle.z[0] - &circle.mesh_height || p[2] > &circle.z[1] + &circle.mesh_height
        }) {
            return Err("cylinder height overshoot");
        }
        let normal = cross(&sub(&tri[1], &tri[0]), &sub(&tri[2], &tri[0]));
        for p in tri {
            let j =
                &normal[0] * (&p[0] - &circle.center[0]) + &normal[1] * (&p[1] - &circle.center[1]);
            if j.is_zero() {
                return Err("singular radial projection");
            }
            let sign = if j > n(0) { 1 } else { -1 };
            if orientation != 0 && orientation != sign {
                return Err("folded radial projection");
            }
            orientation = sign;
        }
        error = error.max(triangle_radial_error(tri, &circle.center, &circle.radius)?);
    }
    // The two boundary cycles must induce opposite cylinder orientations.
    if turn(&circle.mesh_rings[0], &circle.center)? == turn(&circle.mesh_rings[1], &circle.center)?
    {
        return Err("cylinder boundary orientation");
    }
    Ok((circle, error))
}

fn plane_frame(plane: &monstertruck_modeling::Plane) -> Proof<(usize, Q, P)> {
    let coefficients = super::retained::plane_coefficients(plane)?;
    let columns = coefficients[3..]
        .iter()
        .map(|v| v.iter().copied().map(q).collect::<Proof<Vec<_>>>())
        .collect::<Proof<Vec<_>>>()?;
    if (0..3).any(|i| columns[i][3] != n(0)) || columns[3][3] != n(1) {
        return Err("nonaffine plane");
    }
    let points = coefficients[..3]
        .iter()
        .map(|v| v[..3].iter().copied().map(q).collect::<Proof<Vec<_>>>());
    let mut transformed = Vec::new();
    for p in points {
        let p = p?;
        transformed.push(std::array::from_fn(|i| {
            &columns[3][i] + (0..3).map(|j| &columns[j][i] * &p[j]).sum::<Q>()
        }));
    }
    let normal = cross(
        &sub(&transformed[1], &transformed[0]),
        &sub(&transformed[2], &transformed[0]),
    );
    let nonzero = (0..3).filter(|&i| !normal[i].is_zero()).collect::<Vec<_>>();
    if nonzero.len() != 1 {
        return Err("unsupported or singular plane support");
    }
    let axis = nonzero[0];
    Ok((axis, transformed[0][axis].clone(), normal))
}

fn rectangle_wire(wire: &Wire, axis: usize, level: &Q) -> Proof<(Vec<P>, Q)> {
    if wire.len() < 4 || wire.len() > 128 {
        return Err("unsupported noncircular trim");
    }
    let mut points = Vec::new();
    for edge in wire.edge_iter() {
        let Curve::Line(line) = edge.curve() else {
            return Err("unsupported planar trim");
        };
        let (front, back) = if edge.orientation() {
            (line.0, line.1)
        } else {
            (line.1, line.0)
        };
        let a = point(front)?;
        let b = point(back)?;
        if a == b || a != point(edge.front().point())? || b != point(edge.back().point())? {
            return Err("line trim/support mismatch");
        }
        if points.last().is_some_and(|p| p != &a) {
            return Err("unjoined rectangle trim");
        }
        if points.is_empty() {
            points.push(a);
        }
        points.push(b);
    }
    if points.first() != points.last() {
        return Err("open rectangle");
    }
    points.pop();
    // A retained line's normal coordinate is affine. Its endpoint maximum
    // encloses the entire source-to-plane discrepancy, including placement.
    let height = points
        .iter()
        .map(|p| (&p[axis] - level).abs())
        .max()
        .unwrap();
    let mut points = points
        .iter()
        .map(|p| project(p, axis, level))
        .collect::<Vec<_>>();
    loop {
        let count = points.len();
        let removable = (0..count).find(|&i| {
            let a = &points[(i + count - 1) % count];
            let b = &points[i];
            let c = &points[(i + 1) % count];
            cross(&sub(b, a), &sub(c, b)).iter().all(Q::is_zero)
                && (0..3).all(|k| {
                    b[k] >= a[k].clone().min(c[k].clone()) && b[k] <= a[k].clone().max(c[k].clone())
                })
        });
        if count <= 4 || removable.is_none() {
            break;
        }
        points.remove(removable.unwrap());
    }
    if points.len() != 4 {
        return Err("unsupported subdivided rectangle trim");
    }
    let axes = (0..3).filter(|&i| i != axis).collect::<Vec<_>>();
    let coords = axes
        .iter()
        .map(|&i| points.iter().map(|p| p[i].clone()).collect::<BTreeSet<_>>())
        .collect::<Vec<_>>();
    if coords.iter().any(|s| s.len() != 2) || points.iter().collect::<BTreeSet<_>>().len() != 4 {
        return Err("nonrectangular source trim");
    }
    for (a, b) in edges(&points) {
        if (0..3).filter(|&i| a[i] != b[i]).count() != 1 {
            return Err("diagonal rectangle trim");
        }
    }
    Ok((points, height))
}
fn rectangle_matches(source: &[P], actual: &[P]) -> Proof<()> {
    let mut intervals = vec![Vec::<(Q, Q)>::new(); 4];
    for (a, b) in edges(actual) {
        let mut matched = false;
        for (i, (s, e)) in edges(source).enumerate() {
            let axis = (0..3)
                .find(|&k| s[k] != e[k])
                .ok_or("rectangle zero side")?;
            if (0..3).any(|k| k != axis && (a[k] != s[k] || b[k] != s[k])) {
                continue;
            }
            let ta = (&a[axis] - &s[axis]) / (&e[axis] - &s[axis]);
            let tb = (&b[axis] - &s[axis]) / (&e[axis] - &s[axis]);
            if ta == tb || ta < n(0) || tb < n(0) || ta > n(1) || tb > n(1) {
                continue;
            }
            intervals[i].push((ta.clone().min(tb.clone()), ta.max(tb)));
            matched = true;
            break;
        }
        if !matched {
            return Err("rectangle boundary mismatch");
        }
    }
    for mut list in intervals {
        list.sort();
        let mut at = n(0);
        for (lo, hi) in list {
            if lo != at {
                return Err("rectangle gap/duplicate");
            }
            at = hi;
        }
        if at != n(1) {
            return Err("missing rectangle side");
        }
    }
    Ok(())
}
fn ring_area(ring: &[P], axis: usize) -> Q {
    let axes = (0..3).filter(|&i| i != axis).collect::<Vec<_>>();
    edges(ring)
        .map(|(a, b)| &a[axes[0]] * &b[axes[1]] - &a[axes[1]] * &b[axes[0]])
        .sum()
}

enum CapRing {
    Rectangle(Vec<P>),
    Circular(usize),
}
fn project(p: &P, axis: usize, level: &Q) -> P {
    let mut out = p.clone();
    out[axis] = level.clone();
    out
}
fn planar(
    face: &RetainedFace,
    tris: &[[P; 3]],
    circles: &[Circle],
    mesh_error: &Q,
    cylinder_source_error: &Q,
) -> Proof<Q> {
    let Surface::Plane(plane) = &face.surface else {
        return Err("unsupported support");
    };
    let (axis, level, mut normal) = plane_frame(plane)?;
    if !face.mesh_orientation {
        normal = normal.map(|v| -v);
    }
    let mut tri_sign = 0;
    let mut mesh_height = n(0);
    for tri in tris {
        mesh_height = mesh_height.max(tri.iter().map(|p| (&p[axis] - &level).abs()).max().unwrap());
        let nn = cross(&sub(&tri[1], &tri[0]), &sub(&tri[2], &tri[0]));
        if &nn[axis] * &normal[axis] <= n(0) {
            return Err("plane triangle orientation/degeneracy");
        }
        let area = ring_area(tri, axis);
        let sign = if area > n(0) { 1 } else { -1 };
        if tri_sign != 0 && sign != tri_sign {
            return Err("inconsistent cap orientation");
        }
        tri_sign = sign;
    }
    let mut rings = boundary(tris)?;
    if rings.len() != face.boundaries.len() {
        return Err("incomplete cap holes/domain");
    }
    // The same delivered triangles project affinely; positivity above excludes
    // collapsed/folded projections. Authenticate their complete projected chain.
    let projected = tris
        .iter()
        .map(|tri| tri.each_ref().map(|p| project(p, axis, &level)))
        .collect::<Vec<_>>();
    let projected_rings = boundary(&projected)?;
    let expected_rings = rings
        .iter()
        .map(|ring| {
            edge_set(
                &ring
                    .iter()
                    .map(|p| project(p, axis, &level))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeSet<_>>();
    if projected_rings
        .iter()
        .map(|r| edge_set(r))
        .collect::<BTreeSet<_>>()
        != expected_rings
    {
        return Err("projection changed cap boundary census");
    }
    let mut specs = Vec::new();
    let mut source_error = cylinder_source_error.clone();
    for (index, wire) in face.boundaries.iter().enumerate() {
        let ids = wire_ids(wire);
        let circle = circles.iter().position(|c| c.source_edges.contains(&ids));
        let (position, spec) = if let Some(ci) = circle {
            if axis != 2 {
                return Err("nonhorizontal circular cap");
            }
            let c = &circles[ci];
            let source_end = c
                .source_edges
                .iter()
                .position(|edges| edges == &ids)
                .unwrap();
            let trim = curve_ring(wire, &c.center, &c.radius, &level)?;
            source_error = source_error.max(trim.total());
            if trim.sign != if index == 0 { tri_sign } else { -tri_sign } {
                return Err("source cap circular material orientation");
            }
            let expected = c
                .mesh_rings
                .iter()
                .zip(&c.mesh_levels)
                .find(|(_, z)| **z == c.source_levels[source_end])
                .map(|(ring, _)| ring)
                .ok_or("missing cap/cylinder join")?;
            let pos = rings
                .iter()
                .position(|r| edge_set(r) == edge_set(expected))
                .ok_or("cap/cylinder boundary mismatch")?;
            (pos, CapRing::Circular(ci))
        } else {
            let (source, height) = rectangle_wire(wire, axis, &level)?;
            source_error = source_error.max(height);
            let source_sign = if ring_area(&source, axis) > n(0) {
                1
            } else {
                -1
            };
            if source_sign != if index == 0 { tri_sign } else { -tri_sign } {
                return Err("source cap rectangular material orientation");
            }
            let pos = rings
                .iter()
                .position(|r| {
                    rectangle_matches(
                        &source,
                        &r.iter()
                            .map(|p| project(p, axis, &level))
                            .collect::<Vec<_>>(),
                    )
                    .is_ok()
                })
                .ok_or("uncovered rectangular cap")?;
            (pos, CapRing::Rectangle(source))
        };
        let ring = rings.remove(position);
        let sign = if ring_area(&ring, axis) > n(0) { 1 } else { -1 };
        if sign != if index == 0 { tri_sign } else { -tri_sign } {
            return Err("cap material orientation");
        }
        specs.push(spec);
    }
    if !rings.is_empty() {
        return Err("extra cap boundary");
    }
    // Every cap authenticates its own raw trims. Carrier identity does not
    // authorize discarding their error, even if the cylinder was checked first.
    let band = mesh_error + source_error + mesh_height;
    for spec in &specs {
        if let CapRing::Circular(ci) = spec
            && band >= circles[*ci].radius
        {
            return Err("cap radial band reaches axis");
        }
    }
    // Disjoint enclosing bands keep each radial boundary deformation local.
    let axes = (0..3).filter(|&i| i != axis).collect::<Vec<_>>();
    let boxes = specs
        .iter()
        .map(|spec| -> Proof<[[Q; 2]; 2]> {
            match spec {
                CapRing::Rectangle(points) => Ok(std::array::from_fn(|a| {
                    [
                        points.iter().map(|p| p[axes[a]].clone()).min().unwrap(),
                        points.iter().map(|p| p[axes[a]].clone()).max().unwrap(),
                    ]
                })),
                CapRing::Circular(ci) => {
                    let c = &circles[*ci];
                    let r = &c.radius + &band;
                    Ok(std::array::from_fn(|a| {
                        [&c.center[a] - &r, &c.center[a] + &r]
                    }))
                }
            }
        })
        .collect::<Proof<Vec<_>>>()?;
    for i in 1..specs.len() {
        match &specs[0] {
            CapRing::Rectangle(_) => {
                if (0..2)
                    .any(|a| boxes[i][a][0] <= boxes[0][a][0] || boxes[i][a][1] >= boxes[0][a][1])
                {
                    return Err("trim band reaches exterior");
                }
            }
            CapRing::Circular(ci) => {
                let c = &circles[*ci];
                let inside = &c.radius - &band;
                if inside <= n(0) {
                    return Err("outer trim band reaches axis");
                }
                for x in &boxes[i][0] {
                    for y in &boxes[i][1] {
                        let d = [x - &c.center[0], y - &c.center[1], n(0)];
                        if dot2(&d, &d) >= &inside * &inside {
                            return Err("nested trim bands overlap");
                        }
                    }
                }
            }
        }
        for j in 1..i {
            if !(0..2).any(|a| boxes[i][a][1] < boxes[j][a][0] || boxes[j][a][1] < boxes[i][a][0]) {
                return Err("hole trim bands overlap");
            }
        }
    }
    Ok(band)
}

fn bind(
    model: &ExactModel,
    mesh: &GeneratedAnalysisMesh,
    trace: &AnalysisTrace,
) -> Proof<Vec<Vec<[P; 3]>>> {
    if model.solids.len() != 1 || model.solids[0].boundaries().len() != 1 {
        return Err("unsupported solid/shell census");
    }
    let shell = &model.solids[0].boundaries()[0];
    if trace.faces.len() != shell.len()
        || trace.faces.len() != mesh.source_faces.len()
        || trace.faces.is_empty()
        || trace.triangles.len() > 10_000
        || trace.triangles.len() * 9 != mesh.positions_mm_f64.len()
        || mesh.mesh.positions.len() != mesh.positions_mm_f64.len()
        || mesh.mesh.normals.len() != mesh.positions_mm_f64.len()
        || trace.triangles.len() != mesh.triangle_face_occurrence_indices.len()
    {
        return Err("incomplete final census");
    }
    let mut groups = vec![Vec::new(); trace.faces.len()];
    let mut next = vec![0; trace.faces.len()];
    for (i, tri) in trace.triangles.iter().enumerate() {
        let tag = tri.occurrence_index;
        let face = trace.faces.get(tag).ok_or("invalid occurrence")?;
        let expected = face
            .polygon
            .faces()
            .triangle_iter()
            .nth(next[tag])
            .ok_or("extra final triangle")?;
        next[tag] += 1;
        if tri.vertices != expected
            || tri.uv != triangle_uv(&face.polygon, expected)
            || tri.uv.is_err()
            || mesh.triangle_face_occurrence_indices[i] != tag
        {
            return Err("missing/stale UV or triangle provenance");
        }
        let mut points = Vec::new();
        for (j, v) in expected.iter().enumerate() {
            let point = point(face.polygon.positions()[v.pos])?;
            for (a, coordinate) in point.iter().enumerate() {
                if q(tri.positions_mm_f64[j * 3 + a])? != *coordinate
                    || tri.positions_mm_f64[j * 3 + a].to_bits()
                        != mesh.positions_mm_f64[i * 9 + j * 3 + a].to_bits()
                {
                    return Err("final coordinate binding mismatch");
                }
                if tri.rendered_positions_f32[j * 3 + a].to_bits()
                    != mesh.mesh.positions[i * 9 + j * 3 + a].to_bits()
                    || tri.rendered_positions_f32[j * 3 + a].to_bits()
                        != (tri.positions_mm_f64[j * 3 + a] as f32).to_bits()
                {
                    return Err("final render/filter binding mismatch");
                }
            }
            points.push(point);
        }
        groups[tag].push(points.try_into().map_err(|_| "triangle arity")?);
    }
    for (i, native) in shell.face_iter().enumerate() {
        let face = &trace.faces[i];
        let source = &face.source;
        if (source.solid_index, source.shell_index, source.face_index) != (0, 0, i)
            || source != &mesh.source_faces[i]
            || !super::retained::same_surface(&face.surface, &native.surface())?
            || face.boundaries != native.boundaries()
            || face.mesh_orientation != native.orientation()
            || next[i] != face.polygon.faces().triangle_iter().count()
            || groups[i].is_empty()
        {
            return Err("native support/trim census binding mismatch");
        }
        let kind = match &face.surface {
            Surface::Plane(_) => "PLANE",
            Surface::RevolutionSurface(_) => "REVOLUTION_SURFACE",
            _ => return Err("unsupported native support"),
        };
        if source.support_kind != kind {
            return Err("support kind mismatch");
        }
    }
    Ok(groups)
}
fn prove(model: &ExactModel, mesh: &GeneratedAnalysisMesh, trace: &AnalysisTrace) -> Proof<Q> {
    let groups = bind(model, mesh, trace)?;
    let mut circles = Vec::new();
    let mut mesh_error = n(0);
    let mut source_error = n(0);
    let mut mesh_height = n(0);
    for (face, tris) in trace.faces.iter().zip(&groups) {
        if matches!(face.surface, Surface::RevolutionSurface(_)) {
            let (circle, error) = cylinder(face, tris)?;
            source_error = source_error.max(circle.source_error.clone());
            mesh_height = mesh_height.max(circle.mesh_height.clone());
            mesh_error = mesh_error.max(error);
            circles.push(circle);
        }
    }
    if circles.is_empty() {
        return Err("no supported cylinder");
    }
    let mut band = &mesh_error + &source_error + mesh_height;
    for c in &circles {
        if band >= c.radius {
            return Err("radial band reaches axis");
        }
    }
    for (face, tris) in trace.faces.iter().zip(&groups) {
        if matches!(face.surface, Surface::Plane(_)) {
            band = band.max(planar(face, tris, &circles, &mesh_error, &source_error)?);
        }
    }
    Ok(band)
}
fn qualify(
    model: &ExactModel,
    mesh: &GeneratedAnalysisMesh,
    trace: &AnalysisTrace,
) -> GeneratedQualification {
    let result = prove(model, mesh, trace).and_then(|bound| {
        let mut out = bound.to_f64().ok_or("bound conversion overflow")?;
        while q(out)? < bound {
            out = out.next_up();
        }
        Ok(out)
    });
    match result {
        Ok(bound) => GeneratedQualification::Bounded {
            version: 1,
            class: "analytic-generated",
            direct_bound_mm: bound,
            source_precision_mm: 0.,
            total_bound_mm: bound,
        },
        Err(reason) => GeneratedQualification::Unknown {
            reasons: vec![reason.into()],
        },
    }
}
pub(super) fn qualified_bound(
    model: &ExactModel,
    mesh: &GeneratedAnalysisMesh,
    trace: &AnalysisTrace,
) -> GeneratedQualification {
    qualify(model, mesh, trace)
}

#[cfg(test)]
mod tests;
