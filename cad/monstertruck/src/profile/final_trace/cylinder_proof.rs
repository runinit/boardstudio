//! Private whole-surface proof for axis-aligned cylindrical extrusions and caps.
//!
//! See PROOF.md beside this file. All predicates and error accounting use exact
//! rationals of the retained f64 geometry. No mesher tolerance is a proof input.
use super::super::generated_qualification::GeneratedQualification;
use super::*;
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
    mesh_rings: Vec<Vec<P>>,
}

fn curve_ring(wire: &Wire, center: &[Q; 2], radius: &Q, z: &Q) -> Proof<(Q, i32)> {
    let mut endpoints = Vec::new();
    let mut error = n(0);
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
        if control.iter().any(|p| p[3] <= n(0) || p[2] != z * &p[3]) {
            return Err("nonplanar or nonpositive circular trim");
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
    Ok((error, angular_sign))
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
        mesh_rings: boundary(tris)?,
    };
    if circle.mesh_rings.len() != 2 {
        return Err("cylinder seam or missing patch");
    }
    let mut levels = BTreeSet::new();
    for wire in &face.boundaries {
        let level = point(wire.front_vertex().ok_or("empty circular trim")?.point())?[2].clone();
        // Line is an affine carrier: actual trim parameters may extend beyond
        // its nominal seed segment after the native face-merging operation.
        if !levels.insert(level.clone()) {
            return Err("cylinder source trim height");
        }
        circle.source_error = circle
            .source_error
            .clone()
            .max(curve_ring(wire, &circle.center, &circle.radius, &level)?.0);
        circle.source_edges.push(wire_ids(wire));
    }
    circle.z = [
        levels.first().ok_or("missing lower source trim")?.clone(),
        levels.last().ok_or("missing upper source trim")?.clone(),
    ];
    levels.clear();
    for ring in &circle.mesh_rings {
        if ring.iter().any(|p| p[2] != ring[0][2])
            || !circle.z.contains(&ring[0][2])
            || !levels.insert(ring[0][2].clone())
        {
            return Err("incomplete cylinder end rings");
        }
        turn(ring, &circle.center)?;
    }
    let mut error = n(0);
    let mut orientation = 0;
    for tri in tris {
        if tri.iter().any(|p| p[2] < circle.z[0] || p[2] > circle.z[1]) {
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

fn serialized_vector(v: &serde_json::Value, count: usize) -> Proof<Vec<Q>> {
    (0..count)
        .map(|i| {
            q(v.get(["x", "y", "z", "w"][i])
                .or_else(|| v.get(i))
                .and_then(serde_json::Value::as_f64)
                .ok_or("unreadable retained plane data")?)
        })
        .collect()
}
fn plane_frame(plane: &monstertruck_modeling::Plane) -> Proof<(usize, Q, P)> {
    // Read the retained affine coefficients, not rounded origin/normalized-normal
    // accessors. Serde is already part of this pinned native geometry's API.
    let value = serde_json::to_value(plane).map_err(|_| "invalid retained plane")?;
    let columns = (0..4)
        .map(|i| {
            serialized_vector(
                value["transform"]
                    .get(["x", "y", "z", "w"][i])
                    .or_else(|| value["transform"].get(i))
                    .ok_or("plane transform schema")?,
                4,
            )
        })
        .collect::<Proof<Vec<_>>>()?;
    if (0..3).any(|i| columns[i][3] != n(0)) || columns[3][3] != n(1) {
        return Err("nonaffine plane");
    }
    let points = ["o", "p", "q"].map(|key| serialized_vector(&value[key], 3));
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

fn rectangle_wire(wire: &Wire, axis: usize, level: &Q) -> Proof<Vec<P>> {
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
        if a == b
            || a != point(edge.front().point())?
            || b != point(edge.back().point())?
            || a[axis] != *level
            || b[axis] != *level
        {
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
    Ok(points)
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
    for tri in tris {
        if tri.iter().any(|p| p[axis] != level) {
            return Err("nonplanar final triangle");
        }
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
            let (trim_error, source_sign) = curve_ring(wire, &c.center, &c.radius, &level)?;
            source_error = source_error.max(trim_error);
            if source_sign != if index == 0 { tri_sign } else { -tri_sign } {
                return Err("source cap circular material orientation");
            }
            let expected = c
                .mesh_rings
                .iter()
                .find(|r| r[0][2] == level)
                .ok_or("missing cap/cylinder join")?;
            let pos = rings
                .iter()
                .position(|r| edge_set(r) == edge_set(expected))
                .ok_or("cap/cylinder boundary mismatch")?;
            (pos, CapRing::Circular(ci))
        } else {
            let source = rectangle_wire(wire, axis, &level)?;
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
                .position(|r| rectangle_matches(&source, r).is_ok())
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
    let band = mesh_error + source_error;
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
            || serde_json::to_value(&face.surface).map_err(|_| "source serialization")?
                != serde_json::to_value(native.surface()).map_err(|_| "source serialization")?
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
    for (face, tris) in trace.faces.iter().zip(&groups) {
        if matches!(face.surface, Surface::RevolutionSurface(_)) {
            let (circle, error) = cylinder(face, tris)?;
            source_error = source_error.max(circle.source_error.clone());
            mesh_error = mesh_error.max(error);
            circles.push(circle);
        }
    }
    if circles.is_empty() {
        return Err("no supported cylinder");
    }
    let mut band = &mesh_error + &source_error;
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
fn bound(model: &ExactModel, mesh: &GeneratedAnalysisMesh, trace: &AnalysisTrace) -> f64 {
    match qualify(model, mesh, trace) {
        GeneratedQualification::Bounded {
            direct_bound_mm,
            source_precision_mm,
            total_bound_mm,
            ..
        } => {
            assert_eq!(source_precision_mm, 0.);
            assert_eq!(direct_bound_mm, total_bound_mm);
            direct_bound_mm
        }
        other => panic!("expected complete symmetric generated proof, got {other:?}"),
    }
}

#[test]
fn drilled_plate_has_a_complete_symmetric_source_bound() {
    let mut input = prepared_plate();
    input.body.thickness = 2.;
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let bound = bound(&model, &mesh, &trace);
    assert!(bound > 0. && bound < 0.2, "{bound}");
}

fn drilled_input(radius: f64, thickness: f64) -> boardstudio_contracts::PreparedCaseIR {
    let mut input = prepared_plate();
    input.body.thickness = thickness;
    input.regions[0].outer = [[0., 0.], [20., 0.], [20., 10.], [0., 10.]]
        .map(|[x, y]| Vec2 { x, y })
        .to_vec();
    input.regions[0].mounts[0].at = Vec2 { x: 10., y: 5. };
    input.regions[0].mounts[0].hole_diameter = 2. * radius;
    input
}

#[test]
fn cylinder_proof_authored_radius_height_and_actual_refinement_controls() {
    for (radius, height) in [(0.5, 2.), (1., 2.), (1., 4.)] {
        let model = crate::build_case(&drilled_input(radius, height)).unwrap();
        let (coarse, trace) = checked_trace(&model, None);
        let eb = bound(&model, &coarse, &trace);
        let (fine, trace) = checked_trace(&model, Some(0.01));
        let ef = bound(&model, &fine, &trace);
        eprintln!("authored R={radius} H={height}: coarse={eb:.17} refined={ef:.17}");
        assert!(ef < eb && ef < 0.04 && eb < 0.4);
        assert!(fine.positions_mm_f64.len() > coarse.positions_mm_f64.len());
        assert_eq!(fine.bounds, [[0., 0., 0.], [20., 10., height]]);
        // Independent chord-midpoint observation only checks the certificate's
        // lower sanity band; the certificate itself never samples an error.
        let mut witnessed: f64 = 0.;
        for tri in fine.positions_mm_f64.as_chunks::<9>().0 {
            let a = [tri[0] - 10., tri[1] - 5.];
            let b = [tri[3] - 10., tri[4] - 5.];
            if (a[0].hypot(a[1]) - radius).abs() < 1e-12
                && (b[0].hypot(b[1]) - radius).abs() < 1e-12
            {
                witnessed =
                    witnessed.max((radius - ((a[0] + b[0]) / 2.).hypot((a[1] + b[1]) / 2.)).abs());
            }
        }
        assert!(witnessed > 0. && ef >= witnessed);
        assert_eq!(fine.analysis_error_bound_mm, None);
    }
}

#[test]
fn cylinder_proof_authored_circle_pcb_with_real_drill_is_bounded() {
    use boardstudio_contracts::{Mount, MountKind};
    let mut input: boardstudio_contracts::PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    input.body.mounts = Some(vec![Mount {
        id: "authored-drill".into(),
        kind: MountKind::Hole,
        at: Vec2 { x: 10., y: 8. },
        hole_diameter: 1.,
        boss_diameter: None,
        height: None,
    }]);
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    assert_eq!(
        trace
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
            .count(),
        2
    );
    assert!(bound(&model, &mesh, &trace) < 0.04);
}

#[test]
fn cylinder_proof_native_boss_caps_and_drilled_wall_are_bounded() {
    use boardstudio_contracts::{CaseKind, MountKind};
    let mut input = prepared_plate();
    input.body.kind = CaseKind::Tray;
    input.body.thickness = 2.;
    input.body.wall_height = Some(4.);
    input.regions[0].cavities = vec![
        [[2., 2.], [18., 2.], [18., 14.], [2., 14.]]
            .map(|[x, y]| Vec2 { x, y })
            .to_vec(),
    ];
    let mount = &mut input.regions[0].mounts[0];
    mount.kind = MountKind::Boss;
    mount.hole_diameter = 1.;
    mount.boss_diameter = Some(4.);
    mount.height = Some(3.);
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    assert!(bound(&model, &mesh, &trace) < 0.04);
}

#[test]
fn cylinder_proof_clearance_is_independent_of_requested_deflection() {
    let mut input = drilled_input(1., 2.);
    input.regions[0].mounts[0].at.x = 1.03125;
    let model = crate::build_case(&input).unwrap();
    let (coarse, trace) = checked_trace(&model, None);
    assert!(matches!(
        qualify(&model, &coarse, &trace),
        GeneratedQualification::Unknown { .. }
    ));
    let (fine, trace) = checked_trace(&model, Some(0.01));
    assert!(bound(&model, &fine, &trace) < 0.03125);
}

fn unknown(model: &ExactModel, mesh: &GeneratedAnalysisMesh, trace: &AnalysisTrace) {
    match qualify(model, mesh, trace) {
        GeneratedQualification::Unknown { reasons } => assert!(!reasons.is_empty()),
        other => panic!("expected Unknown, got {other:?}"),
    }
}

// Mutation-only helper: keep malformed delivered buffers/trace consistently
// bound, so coverage tests must fail the proof rather than a stale-data check.
fn mutate_face_triangle(
    mesh: &mut GeneratedAnalysisMesh,
    trace: &mut AnalysisTrace,
    face_index: usize,
    duplicate: bool,
) {
    use monstertruck_meshing::rexport_polymesh::Faces;
    let poly = &mut trace.faces[face_index].polygon;
    let mut triangles = poly.faces().triangle_iter().collect::<Vec<_>>();
    let ti = trace
        .triangles
        .iter()
        .position(|t| t.occurrence_index == face_index)
        .unwrap();
    if duplicate {
        let copy = triangles[0];
        triangles.push(copy);
        let old = &trace.triangles[ti];
        let new = FinalTriangle {
            occurrence_index: face_index,
            vertices: old.vertices,
            uv: old.uv,
            positions_mm_f64: old.positions_mm_f64,
            rendered_positions_f32: old.rendered_positions_f32,
        };
        mesh.positions_mm_f64.extend(new.positions_mm_f64);
        mesh.mesh.positions.extend(new.rendered_positions_f32);
        mesh.mesh.normals.extend_from_within(ti * 9..ti * 9 + 9);
        mesh.triangle_face_occurrence_indices.push(face_index);
        trace.triangles.push(new);
    } else {
        triangles.remove(0);
        trace.triangles.remove(ti);
        mesh.positions_mm_f64.drain(ti * 9..ti * 9 + 9);
        mesh.mesh.positions.drain(ti * 9..ti * 9 + 9);
        mesh.mesh.normals.drain(ti * 9..ti * 9 + 9);
        mesh.triangle_face_occurrence_indices.remove(ti);
    }
    *poly = PolygonMesh::new(poly.attributes().clone(), Faces::from_iter(triangles));
}

#[test]
fn cylinder_proof_missing_duplicate_and_seam_sliver_fail_after_binding() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    for curved in [false, true] {
        for duplicate in [false, true] {
            let (mut mesh, mut trace) = checked_trace(&model, None);
            let fi = trace
                .faces
                .iter()
                .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)) == curved)
                .unwrap();
            mutate_face_triangle(&mut mesh, &mut trace, fi, duplicate);
            assert!(bind(&model, &mesh, &trace).is_ok());
            unknown(&model, &mesh, &trace);
        }
    }
}

#[test]
fn cylinder_proof_missing_uv_face_and_occurrence_are_unknown() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    for mutation in 0..5 {
        let (mut mesh, mut trace) = checked_trace(&model, None);
        match mutation {
            0 => {
                trace.triangles[0].uv = Err(UvUnknown::Missing);
            }
            1 => {
                trace.faces.pop();
            }
            2 => {
                mesh.source_faces.pop();
            }
            3 => {
                mesh.triangle_face_occurrence_indices[0] = usize::MAX;
            }
            _ => {
                trace.faces[0]
                    .polygon
                    .uv_coords_mut()
                    .fill(monstertruck_modeling::Vector2::new(0., 0.));
            }
        }
        unknown(&model, &mesh, &trace);
    }
}

#[test]
fn cylinder_proof_rejects_bad_span_retrace_and_normalization() {
    let r = n(1);
    let center = [n(0), n(0)];
    let p = |x, y| [n(x), n(y), n(0)];
    assert!(turn(&[p(1, 0), p(-1, 1), p(-1, -1)], &center).is_err());
    assert!(turn(&[p(1, 0), p(0, 1), p(1, 0), p(0, -1)], &center).is_err());
    assert!(triangle_radial_error(&[p(1, 0), p(-1, 1), p(-1, -1)], &center, &r).is_err());
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (_, mut trace) = checked_trace(&model, None);
    let fi = trace
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let tris = bind(
        &model,
        &mesh_profile_for_analysis(&model, None).unwrap(),
        &trace,
    )
    .unwrap();
    let Surface::RevolutionSurface(s) = &trace.faces[fi].surface else {
        unreachable!()
    };
    let mut value = serde_json::to_value(s).unwrap();
    value["entity"]["revolution"]["axis"]["z"] = serde_json::json!(2.);
    let bad = serde_json::from_value(value).unwrap();
    trace.faces[fi].surface = Surface::RevolutionSurface(bad);
    assert!(cylinder(&trace.faces[fi], &tris[fi]).is_err());
}

#[test]
fn cylinder_proof_nonplanar_trim_and_cone_remain_unknown() {
    use boardstudio_contracts::CaseFeature;
    let model = crate::build_case(&drilled_input(1., 1.6)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    unknown(&model, &mesh, &trace);
    let mut input = drilled_input(1., 4.);
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![CaseFeature::ConicalSeat {
        id: "cone".into(),
        at: Vec2 { x: 10., y: 5. },
        z: 2.,
        height: 2.,
        diameter: 4.,
        end_diameter: 2.,
    }]);
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    unknown(&model, &mesh, &trace);
}

#[test]
fn cylinder_proof_source_translation_and_large_coordinate_controls() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let e = bound(&model, &mesh, &trace);
    let translated = ExactModel {
        solids: model
            .solids
            .iter()
            .map(|s| builder::translated(s, Vector3::new(32., 16., 8.)))
            .collect(),
    };
    let (mesh, trace) = checked_trace(&translated, None);
    let moved = bound(&translated, &mesh, &trace);
    assert!((moved - e).abs() < 1e-12); // test observation only; each proof bounds its own final rounding.
    assert!(q(1e20).is_err());
    let mut huge = mesh;
    huge.positions_mm_f64[0] = 1e20;
    unknown(&translated, &huge, &trace);
}

#[test]
fn cylinder_proof_source_coefficients_have_a_continuous_numeric_contribution() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let fi = trace
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let groups = bind(&model, &mesh, &trace).unwrap();
    let (original, _) = cylinder(&trace.faces[fi], &groups[fi]).unwrap();
    assert_eq!(original.source_error, n(0));
    let edge = trace.faces[fi].boundaries[0]
        .edge_iter()
        .next()
        .unwrap()
        .clone();
    let Curve::NurbsCurve(mut curve) = edge.curve() else {
        unreachable!()
    };
    curve.control_point_mut(1).x = curve.control_point(1).x.next_up();
    edge.set_curve(Curve::NurbsCurve(curve));
    assert!(bind(&model, &mesh, &trace).is_ok());
    let (changed, _) = cylinder(&trace.faces[fi], &groups[fi]).unwrap();
    assert!(changed.source_error > n(0));
    assert!(prove(&model, &mesh, &trace).unwrap() > original.source_error);
    let Curve::NurbsCurve(mut curve) = edge.curve() else {
        unreachable!()
    };
    curve.control_point_mut(1).z += 0.25;
    edge.set_curve(Curve::NurbsCurve(curve));
    unknown(&model, &mesh, &trace);
}

#[test]
fn cylinder_proof_consistently_bound_cap_sliver_and_real_open_seam_are_unknown() {
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mut mesh, mut trace) = checked_trace(&model, None);
    let face = 0;
    let vertex = trace
        .triangles
        .iter()
        .find(|t| t.occurrence_index == face)
        .unwrap()
        .vertices[0]
        .pos;
    trace.faces[face].polygon.positions_mut()[vertex].x += 1. / 1024.;
    for (i, tri) in trace
        .triangles
        .iter_mut()
        .enumerate()
        .filter(|(_, t)| t.occurrence_index == face)
    {
        for (j, v) in tri.vertices.iter().enumerate() {
            let p = trace.faces[face].polygon.positions()[v.pos];
            for (axis, value) in [p.x, p.y, p.z].into_iter().enumerate() {
                tri.positions_mm_f64[j * 3 + axis] = value;
                tri.rendered_positions_f32[j * 3 + axis] = value as f32;
                mesh.positions_mm_f64[i * 9 + j * 3 + axis] = value;
                mesh.mesh.positions[i * 9 + j * 3 + axis] = value as f32;
            }
        }
    }
    assert!(bind(&model, &mesh, &trace).is_ok());
    unknown(&model, &mesh, &trace);
    let model = crate::build_case(&drilled_input(2., 4.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Unknown { reasons } => {
            assert!(reasons.iter().any(|r| r.contains("seam")), "{reasons:?}")
        }
        other => panic!("unproven real seam must stay Unknown: {other:?}"),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test::wasm_bindgen_test]
fn generated_cylinder_proof_actual_wasm() {
    drilled_plate_has_a_complete_symmetric_source_bound();
    cylinder_proof_authored_radius_height_and_actual_refinement_controls();
    cylinder_proof_authored_circle_pcb_with_real_drill_is_bounded();
    cylinder_proof_native_boss_caps_and_drilled_wall_are_bounded();
    cylinder_proof_clearance_is_independent_of_requested_deflection();
    cylinder_proof_missing_duplicate_and_seam_sliver_fail_after_binding();
    cylinder_proof_missing_uv_face_and_occurrence_are_unknown();
    cylinder_proof_rejects_bad_span_retrace_and_normalization();
    cylinder_proof_nonplanar_trim_and_cone_remain_unknown();
    cylinder_proof_source_translation_and_large_coordinate_controls();
    cylinder_proof_source_coefficients_have_a_continuous_numeric_contribution();
    cylinder_proof_consistently_bound_cap_sliver_and_real_open_seam_are_unknown();
    cylinder_proof_discontinuous_trim_knots_are_unknown();
    p1_asymmetric_shared_trim_cannot_publish_a_smaller_surface_bound();
    p1_raw_orientation_preserves_locus_when_reflected_knots_collide();
    p1_cap_trim_error_is_returned_without_a_cylinder_error_allowance();
    p1_cap_trim_error_participates_in_nesting_separation();
}

#[test]
fn cylinder_proof_discontinuous_trim_knots_are_unknown() {
    use monstertruck_modeling::{BsplineCurve, NurbsCurve};
    let model = crate::build_case(&drilled_input(1., 2.)).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let face = trace
        .faces
        .iter()
        .find(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let edge = face.boundaries[0].edge_iter().next().unwrap();
    let Curve::NurbsCurve(curve) = edge.curve() else {
        unreachable!()
    };
    let mut knots = curve.knot_vector().iter().copied().collect::<Vec<_>>();
    knots.insert(4, 0.5);
    knots.insert(4, 0.5);
    let mut controls = curve.control_points().clone();
    controls.insert(3, controls[3]);
    controls.insert(3, controls[3]);
    edge.set_curve(Curve::NurbsCurve(NurbsCurve::new(BsplineCurve::new(
        knots.into(),
        controls,
    ))));
    assert!(bind(&model, &mesh, &trace).is_ok());
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Unknown { reasons } => {
            assert!(reasons[0].contains("trim knots"), "{reasons:?}")
        }
        other => panic!("discontinuous domain must refuse: {other:?}"),
    }
}

// Adversarial source mutation only: retain the real emitted fine mesh, and
// replace one actual shared native trim used backward on its cylinder and
// forward on its cap. The retained source, not a newly meshed replacement, is
// what a source/final certificate must bound.
struct AsymmetricTrim {
    model: ExactModel,
    mesh: GeneratedAnalysisMesh,
    trace: AnalysisTrace,
    side: (usize, usize),
    cap: (usize, usize),
    witness: P,
}

fn asymmetric_shared_trim() -> AsymmetricTrim {
    asymmetric_shared_trim_with_left(0.)
}

fn asymmetric_shared_trim_with_left(left: f64) -> AsymmetricTrim {
    use monstertruck_modeling::{BsplineCurve, NurbsCurve, Vector4};
    let mut input = drilled_input(1., 2.);
    input.regions[0].outer[0].x = left;
    input.regions[0].outer[3].x = left;
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, Some(0.01));
    let ci = trace
        .faces
        .iter()
        .position(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
        .unwrap();
    let (wi, edge) = trace.faces[ci]
        .boundaries
        .iter()
        .enumerate()
        .find_map(|(wi, wire)| wire.edge_iter().find(|e| !e.orientation()).map(|e| (wi, e)))
        .unwrap();
    let (cap, cap_wire) = trace
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.surface, Surface::Plane(_)))
        .find_map(|(fi, f)| {
            f.boundaries.iter().enumerate().find_map(|(wi, w)| {
                w.edge_iter()
                    .any(|e| e.id() == edge.id() && e.orientation())
                    .then_some((fi, wi))
            })
        })
        .unwrap();
    let Curve::NurbsCurve(raw) = edge.curve() else {
        unreachable!()
    };
    let start = edge.absolute_front().point();
    let sx = (start.x - 10.).signum();
    let control = raw.control_point(1);
    let sy = (control.y / control.w - 5.).signum();
    let z = start.z;
    let w = 0.5_f64.sqrt();
    let controls = [
        (sx, 0., 1.),
        (sx * w, sy * w, w),
        (0., sy, 1.),
        (-sx * w, sy * w, w),
        (-sx, 0., 1.),
    ]
    .map(|(x, y, weight)| Vector4::new(10. * weight + x, 5. * weight + y, z * weight, weight))
    .to_vec();
    let curve = NurbsCurve::new(BsplineCurve::new(
        vec![0., 0., 0., 0.00008, 0.00015, 1e12, 1e12, 1e12].into(),
        controls,
    ));
    edge.set_curve(Curve::NurbsCurve(curve));
    let witness = [
        q(10.).unwrap() + q(sx).unwrap() * n(47) / n(64),
        q(5.).unwrap() + q(sy).unwrap() * n(47) / n(64),
        q(z).unwrap(),
    ];
    assert!(
        bind(&model, &mesh, &trace).is_ok(),
        "shared source mutation must retain binding"
    );
    AsymmetricTrim {
        model,
        mesh,
        trace,
        side: (ci, wi),
        cap: (cap, cap_wire),
        witness,
    }
}

// Positive homogeneous control hulls provide an independent continuous oracle:
// each dyadic subcurve and its endpoint chord stay outside the witness ball.
// Their homotopy therefore preserves winding about that witness.
fn trim_distance_and_winding(wire: &Wire, witness: &P, rho: &Q) -> i32 {
    let mut polygon = Vec::new();
    for edge in wire.edge_iter() {
        let Curve::NurbsCurve(raw) = edge.curve() else {
            unreachable!()
        };
        let knots = raw
            .knot_vector()
            .iter()
            .map(|v| q(*v).unwrap())
            .collect::<Vec<_>>();
        let control = raw
            .control_points()
            .iter()
            .map(|p| {
                [
                    q(p.x).unwrap(),
                    q(p.y).unwrap(),
                    q(p.z).unwrap(),
                    q(p.w).unwrap(),
                ]
            })
            .collect::<Vec<_>>();
        let mut points = Vec::new();
        for span in knots.windows(2).filter(|p| p[0] < p[1]) {
            for i in 0..64 {
                let a = &span[0] + (&span[1] - &span[0]) * n(i) / n(64);
                let b = &span[0] + (&span[1] - &span[0]) * n(i + 1) / n(64);
                let left = spline_value(&knots, &control, &a);
                let right = spline_value(&knots, &control, &b);
                let mid = spline_value(&knots, &control, &((&a + &b) / n(2)));
                let middle: [Q; 4] =
                    std::array::from_fn(|k| n(2) * &mid[k] - (&left[k] + &right[k]) / n(2));
                let hull = [left, middle, right].map(|p| {
                    assert!(p[3] > n(0));
                    std::array::from_fn::<_, 3, _>(|k| &p[k] / &p[3])
                });
                let distance2: Q = (0..3)
                    .map(|k| {
                        let lo = hull.iter().map(|p| &p[k]).min().unwrap();
                        let hi = hull.iter().map(|p| &p[k]).max().unwrap();
                        let d = if witness[k] < *lo {
                            lo - &witness[k]
                        } else if witness[k] > *hi {
                            &witness[k] - hi
                        } else {
                            n(0)
                        };
                        &d * &d
                    })
                    .sum();
                assert!(
                    distance2 > rho * rho,
                    "entire raw source subcurve must avoid witness ball"
                );
                if points.is_empty() {
                    points.push(hull[0].clone());
                }
                points.push(hull[2].clone());
            }
        }
        if !edge.orientation() {
            points.reverse();
        }
        if let Some(last) = polygon.last() {
            assert_eq!(last, &points[0]);
        }
        polygon.extend(points.into_iter().skip(usize::from(!polygon.is_empty())));
    }
    assert_eq!(polygon.first(), polygon.last());
    polygon.pop();
    let center = [witness[0].clone(), witness[1].clone()];
    let mut winding = 0;
    for (a, b) in edges(&polygon) {
        let a = centered(a, &center);
        let b = centered(b, &center);
        let d = det(&a, &b);
        if a[1] <= n(0) && b[1] > n(0) && d > n(0) {
            winding += 1;
        }
        if b[1] <= n(0) && a[1] > n(0) && d < n(0) {
            winding -= 1;
        }
    }
    winding
}

#[test]
fn p1_asymmetric_shared_trim_cannot_publish_a_smaller_surface_bound() {
    let AsymmetricTrim {
        model,
        mesh,
        trace,
        side: (ci, wi),
        cap: (cap, cap_wire),
        witness: p,
    } = asymmetric_shared_trim();
    let groups = bind(&model, &mesh, &trace).unwrap();
    assert_eq!(
        trace
            .faces
            .iter()
            .filter(|f| matches!(f.surface, Surface::RevolutionSurface(_)))
            .count(),
        1
    );
    let (circle, _) = cylinder(&trace.faces[ci], &groups[ci]).unwrap();
    let cap_error = curve_ring(
        &trace.faces[cap].boundaries[cap_wire],
        &circle.center,
        &circle.radius,
        &p[2],
    )
    .unwrap()
    .0;
    let rho = n(1) / n(32);
    assert!(cap_error > n(1) / n(4));
    assert!(cap_wire > 0, "witness winding is in a source hole");
    for (fi, face) in trace.faces.iter().enumerate() {
        if fi != cap
            && let Surface::Plane(plane) = &face.surface
        {
            let (axis, level, _) = plane_frame(plane).unwrap();
            assert!(
                (&p[axis] - level).abs() > rho,
                "every other source plane is also farther than 1/32"
            );
        }
    }
    assert_eq!(
        trim_distance_and_winding(&trace.faces[cap].boundaries[cap_wire], &p, &rho).abs(),
        1
    );
    let radial = centered(&p, &circle.center);
    assert!(
        dot2(&radial, &radial) > (&circle.radius + &rho) * (&circle.radius + &rho),
        "source cylinder is also farther than 1/32"
    );
    assert!(
        groups[cap].iter().any(|tri| {
            let signs = edges(tri)
                .map(|(a, b)| det(&sub(b, a), &sub(&p, a)))
                .collect::<Vec<_>>();
            tri.iter().all(|v| v[2] == p[2])
                && (signs.iter().all(|d| d >= &n(0)) || signs.iter().all(|d| d <= &n(0)))
        }),
        "witness lies on an actual final cap triangle"
    );
    assert!(trace.faces[cap].boundaries[cap_wire].edge_iter().any(|e| {
        e.orientation()
            && trace.faces[ci].boundaries[wi]
                .edge_iter()
                .any(|other| !other.orientation() && other.id() == e.id())
    }));
    match qualify(&model, &mesh, &trace) {
        GeneratedQualification::Bounded {
            direct_bound_mm, ..
        } => {
            eprintln!(
                "P1 corrected bound={direct_bound_mm:.17}; certified final/source distance >1/32"
            );
            assert!(
                q(direct_bound_mm).unwrap() >= rho,
                "P1: reported {direct_bound_mm}, but final cap witness is >1/32 from the source surface"
            );
        }
        GeneratedQualification::Unknown { .. } => (),
    }
}

#[test]
fn p1_raw_orientation_preserves_locus_when_reflected_knots_collide() {
    let case = asymmetric_shared_trim();
    let wire = &case.trace.faces[case.cap.0].boundaries[case.cap.1];
    let edge = wire
        .edge_iter()
        .find(|e| {
            e.orientation()
                && case.trace.faces[case.side.0].boundaries[case.side.1]
                    .edge_iter()
                    .any(|other| other.id() == e.id() && !other.orientation())
        })
        .unwrap();
    let Curve::NurbsCurve(raw) = edge.curve() else {
        unreachable!()
    };
    assert_ne!(raw.knot(3), raw.knot(4));
    let end = q(raw.knot(7)).unwrap();
    assert_ne!(
        &end - q(raw.knot(3)).unwrap(),
        &end - q(raw.knot(4)).unwrap()
    );
    // Exhibit the library's lossy f64 inversion only as a regression control.
    let mut rounded = raw;
    rounded.invert();
    assert_eq!(rounded.knot(3).to_bits(), rounded.knot(4).to_bits());
    let reversed: Wire = wire.edge_iter().rev().map(|edge| edge.inverse()).collect();
    let forward = curve_ring(wire, &[n(10), n(5)], &n(1), &case.witness[2]).unwrap();
    let backward = curve_ring(&reversed, &[n(10), n(5)], &n(1), &case.witness[2]).unwrap();
    assert!(forward.0 > n(1) / n(4));
    assert_eq!(
        forward.0, backward.0,
        "raw source enclosure must be orientation invariant"
    );
    assert_eq!(forward.1, -backward.1);
}

#[test]
fn p1_cap_trim_error_is_returned_without_a_cylinder_error_allowance() {
    let case = asymmetric_shared_trim();
    let groups = bind(&case.model, &case.mesh, &case.trace).unwrap();
    let (circle, mesh_error) =
        cylinder(&case.trace.faces[case.side.0], &groups[case.side.0]).unwrap();
    let cap = &case.trace.faces[case.cap.0];
    let error = curve_ring(
        &cap.boundaries[case.cap.1],
        &circle.center,
        &circle.radius,
        &case.witness[2],
    )
    .unwrap()
    .0;
    // Authenticate the cap independently, with no allowance inherited from a
    // cylinder pass. Its own raw trim error must still enter the returned band.
    let cap_band = planar(cap, &groups[case.cap.0], &[circle], &mesh_error, &n(0)).unwrap();
    assert_eq!(cap_band, mesh_error + error);
    assert!(cap_band > n(1) / n(4));
}

#[test]
fn p1_cap_trim_error_participates_in_nesting_separation() {
    let case = asymmetric_shared_trim_with_left(8.75);
    let groups = bind(&case.model, &case.mesh, &case.trace).unwrap();
    let (circle, mesh_error) =
        cylinder(&case.trace.faces[case.side.0], &groups[case.side.0]).unwrap();
    let cap = &case.trace.faces[case.cap.0];
    assert!(mesh_error < n(1) / n(4));
    assert_eq!(
        planar(cap, &groups[case.cap.0], &[circle], &mesh_error, &n(0)),
        Err("trim band reaches exterior"),
        "cap-only error must enlarge the domain separation band"
    );
}
