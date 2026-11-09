//! Enclose stored geometry, not samples; an inconclusive enclosure is a refusal.
use super::*;
use monstertruck_modeling::Vector4;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug)]
struct I {
    lo: f64,
    hi: f64,
}
impl I {
    fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn abs(self) -> f64 {
        if self.finite() {
            self.lo.abs().max(self.hi.abs())
        } else {
            f64::INFINITY
        }
    }
    fn finite(self) -> bool {
        self.lo.is_finite() && self.hi.is_finite() && self.lo <= self.hi
    }
}
impl Add for I {
    type Output = Self;
    fn add(self, r: Self) -> Self {
        Self {
            lo: (self.lo + r.lo).next_down(),
            hi: (self.hi + r.hi).next_up(),
        }
    }
}
impl Neg for I {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}
impl Sub for I {
    type Output = Self;
    fn sub(self, r: Self) -> Self {
        self + -r
    }
}
impl Mul for I {
    type Output = Self;
    fn mul(self, r: Self) -> Self {
        let a = [
            self.lo * r.lo,
            self.lo * r.hi,
            self.hi * r.lo,
            self.hi * r.hi,
        ];
        if a.iter().any(|v| v.is_nan()) {
            return Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            };
        }
        Self {
            lo: a.into_iter().fold(f64::INFINITY, f64::min).next_down(),
            hi: a.into_iter().fold(f64::NEG_INFINITY, f64::max).next_up(),
        }
    }
}
impl Div for I {
    type Output = Self;
    fn div(self, r: Self) -> Self {
        if r.lo <= 0.0 && r.hi >= 0.0 {
            return Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            };
        }
        self * Self {
            lo: (1.0 / r.hi).next_down(),
            hi: (1.0 / r.lo).next_up(),
        }
    }
}
const ZERO: I = I { lo: 0.0, hi: 0.0 };
const ONE: I = I { lo: 1.0, hi: 1.0 };
fn dot(a: [I; 3], b: [I; 3]) -> I {
    (0..3).fold(ZERO, |s, i| s + a[i] * b[i])
}
fn cross(a: [I; 3], b: [I; 3]) -> [I; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn point(p: Point3) -> [I; 3] {
    [I::point(p.x), I::point(p.y), I::point(p.z)]
}
fn vector(v: Vector3) -> [I; 3] {
    [I::point(v.x), I::point(v.y), I::point(v.z)]
}
fn sub(a: [I; 3], b: [I; 3]) -> [I; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

struct Reference {
    radius: I,
    half: I,
    depth: I,
    height: I,
    sin: I,
    cos: I,
}
impl Reference {
    fn new(p: Profile) -> Result<Self, String> {
        let x = I::point(p.tilt.to_radians());
        if !x.finite() || x.abs() > 0.25 {
            return Err("uncertified rotation angle".into());
        }
        // Taylor remainder < 1e-40 for |angle|<=.25 after sixteen terms.
        let xx = x * x;
        let (mut sin, mut cos, mut st, mut ct) = (x, ONE, x, ONE);
        for k in 1..=16 {
            st = -st * xx / I::point((2 * k * (2 * k + 1)) as f64);
            ct = -ct * xx / I::point(((2 * k - 1) * 2 * k) as f64);
            sin = sin + st;
            cos = cos + ct;
        }
        let remainder = I {
            lo: -1e-40,
            hi: 1e-40,
        };
        let half = I::point(p.top_d) / I::point(2.0);
        let depth = I::point(p.dish_depth);
        Ok(Self {
            radius: (half * half + depth * depth) / (I::point(2.0) * depth),
            half,
            depth,
            height: I::point(p.height),
            sin: sin + remainder,
            cos: cos + remainder,
        })
    }
    // Exact requested inverse pose applied with outward rounding to stored data.
    fn frame(&self, h: [I; 4]) -> [I; 4] {
        let dz = h[2] - self.height * h[3];
        [
            h[0],
            self.cos * h[1] + self.sin * dz,
            -self.sin * h[1] + self.cos * dz,
            h[3],
        ]
    }
}

pub(super) fn certify_source_scale(p: Profile, radius: f64) -> Result<(), String> {
    let r = Reference::new(p)?.radius;
    let uncertainty = (r - I::point(radius)).abs();
    if !r.finite() || r.lo <= 0.0 || !radius.is_finite() || uncertainty > BOOLEAN_TOLERANCE / 16.0 {
        return Err(format!(
            "uncertifiable requested primitive at tolerance {BOOLEAN_TOLERANCE}: radius uncertainty={uncertainty}"
        ));
    }
    Ok(())
}

// Interval Boehm insertion retains the complete stored B-spline, including its rounding uncertainty.
fn bezier_spans(knots: &[f64], controls: Vec<[I; 4]>) -> Result<Vec<Vec<[I; 4]>>, String> {
    let degree = knots
        .len()
        .checked_sub(controls.len() + 1)
        .ok_or("invalid spline degree")?;
    if degree == 0
        || degree > 3
        || controls.len() <= degree
        || !knots.iter().all(|v| v.is_finite())
        || knots.windows(2).any(|k| k[0] > k[1])
        || knots[0] >= knots[knots.len() - 1]
        || controls.iter().flatten().any(|v| !v.finite())
    {
        return Err("unsupported certificate spline degree".into());
    }
    let mut knots = knots.to_vec();
    let mut c = controls;
    if knots[..=degree].iter().any(|t| *t != knots[0])
        || knots[knots.len() - degree - 1..]
            .iter()
            .any(|t| *t != knots[knots.len() - 1])
    {
        return Err("certificate requires clamped spline".into());
    }
    let mut interior = knots[degree + 1..knots.len() - degree - 1].to_vec();
    interior.dedup();
    for t in interior {
        if t <= knots[0]
            || t >= knots[knots.len() - 1]
            || knots.iter().filter(|v| **v == t).count() > degree
        {
            return Err("unsupported spline multiplicity".into());
        }
        while knots.iter().filter(|v| **v == t).count() < degree {
            let k = knots
                .iter()
                .rposition(|v| *v <= t)
                .ok_or("missing spline span")?;
            let multiplicity = knots.iter().filter(|v| **v == t).count();
            let mut q = vec![[ZERO; 4]; c.len() + 1];
            q[..=k - degree].copy_from_slice(&c[..=k - degree]);
            q[k - multiplicity + 1..].copy_from_slice(&c[k - multiplicity..]);
            for i in k - degree + 1..=k - multiplicity {
                let alpha = (I::point(t) - I::point(knots[i]))
                    / (I::point(knots[i + degree]) - I::point(knots[i]));
                q[i] = std::array::from_fn(|j| alpha * c[i][j] + (ONE - alpha) * c[i - 1][j]);
            }
            knots.insert(k + 1, t);
            c = q;
        }
    }
    if !(c.len() - 1).is_multiple_of(degree) {
        return Err("invalid Bezier extraction".into());
    }
    Ok((0..c.len() - 1)
        .step_by(degree)
        .map(|i| c[i..=i + degree].to_vec())
        .collect())
}
fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |v, i| v * (n - i) as f64 / (i + 1) as f64)
}

// The implicit residual's Bernstein coefficients bound every rational patch point.
fn rational_error(
    reference: &Reference,
    net: &[Vec<[I; 4]>],
    spherical: bool,
) -> Result<f64, String> {
    let (n, m) = (net.len() - 1, net[0].len() - 1);
    let c: Vec<Vec<_>> = net
        .iter()
        .map(|row| row.iter().map(|v| reference.frame(*v)).collect())
        .collect();
    let min_weight = c
        .iter()
        .flatten()
        .map(|v| v[3].lo)
        .fold(f64::INFINITY, f64::min);
    if min_weight <= 0.0 {
        return Err("nonpositive rational weight cannot be certified".into());
    }
    let mut max_error = 0.0_f64;
    for a in 0..=2 * n {
        for b in 0..=2 * m {
            let mut coefficient = ZERO;
            for i in 0..=n {
                for j in 0..=m {
                    let (Some(k), Some(l)) = (a.checked_sub(i), b.checked_sub(j)) else {
                        continue;
                    };
                    if k > n || l > m {
                        continue;
                    }
                    let weight = I::point(choose(n, i)) * I::point(choose(n, k))
                        / I::point(choose(2 * n, a))
                        * I::point(choose(m, j))
                        * I::point(choose(m, l))
                        / I::point(choose(2 * m, b));
                    let (p, q) = (c[i][j], c[k][l]);
                    let mut residual = p[1] * q[1] + p[2] * q[2]
                        - (reference.radius - reference.depth) * (p[2] * q[3] + p[3] * q[2])
                        - reference.half * reference.half * p[3] * q[3];
                    if spherical {
                        residual = residual + p[0] * q[0];
                    }
                    coefficient = coefficient + weight * residual;
                }
            }
            max_error = max_error.max(coefficient.abs());
        }
    }
    // |d-R|=|d²-R²|/(d+R) <= |residual|/R.
    let bound =
        (I::point(max_error) / (I::point(min_weight) * I::point(min_weight) * reference.radius)).hi;
    if !bound.is_finite() {
        return Err("non-finite rational certificate".into());
    }
    Ok(bound)
}
fn h(v: &Vector4) -> [I; 4] {
    [I::point(v.x), I::point(v.y), I::point(v.z), I::point(v.w)]
}

fn sphere_error(
    reference: &Reference,
    sphere: &Processor<Sphere, monstertruck_modeling::Matrix4>,
) -> f64 {
    let t = sphere.transform();
    let center = sphere.entity().center();
    let hc = [
        I::point(center.x),
        I::point(center.y),
        I::point(center.z),
        ONE,
    ];
    let world: [I; 4] =
        std::array::from_fn(|j| (0..4).fold(ZERO, |sum, i| sum + I::point(t[i][j]) * hc[i]));
    let f = reference.frame(world);
    let center_error = I::point(f[0].abs())
        + I::point(f[1].abs())
        + I::point((f[2] - (reference.radius - reference.depth)).abs());
    let axes: Vec<[I; 3]> = (0..3)
        .map(|i| {
            let a = reference.frame([
                I::point(t[i][0]),
                I::point(t[i][1]),
                I::point(t[i][2]),
                ZERO,
            ]);
            [a[0], a[1], a[2]]
        })
        .collect();
    let mut gram = ZERO;
    for i in 0..3 {
        for j in 0..3 {
            let identity = if i == j { ONE } else { ZERO };
            gram = gram + I::point((dot(axes[i], axes[j]) - identity).abs());
        }
    }
    if gram.hi >= 1.0 {
        return f64::INFINITY;
    }
    (center_error
        + I::point((I::point(sphere.entity().radius()) - reference.radius).abs())
        + I::point(sphere.entity().radius()) * gram)
        .hi
}

// No normalized normal is needed: the determinant sign certifies a strict halfspace.
fn closure_separates(
    p: Profile,
    reference: &Reference,
    plane: &monstertruck_geometry::prelude::Plane,
) -> bool {
    let origin = point(plane.origin());
    let normal = cross(vector(plane.axis_u()), vector(plane.axis_v()));
    let direction = dot(normal, [ZERO, -reference.sin, reference.cos]);
    if direction.lo <= 0.0 {
        return false;
    }
    [
        sampled_wire(p.width, p.depth, 0.0, 0.0),
        sampled_wire(p.top_w, p.top_d, p.height, p.tilt),
    ]
    .iter()
    .flat_map(|w| w.edge_iter())
    .all(|e| dot(normal, sub(point(e.front().point()), origin)).hi < 0.0)
}

pub(super) fn certify_cutter(
    p: Profile,
    cutter: &Solid,
    needs_closure: bool,
) -> Result<(), String> {
    let reference = Reference::new(p)?;
    let mut largest = 0.0_f64;
    let mut closure = false;
    for shell in cutter.boundaries() {
        for edge in shell.edge_iter() {
            if let Curve::NurbsCurve(c) = edge.curve() {
                for span in
                    bezier_spans(c.knot_vector(), c.control_points().iter().map(h).collect())?
                {
                    largest = largest.max(rational_error(
                        &reference,
                        &span.into_iter().map(|v| vec![v]).collect::<Vec<_>>(),
                        p.spherical,
                    )?);
                }
            }
        }
        for face in shell.iter() {
            match face.surface() {
                Surface::Plane(plane) => {
                    if closure_separates(p, &reference, &plane) {
                        closure = true;
                    }
                }
                Surface::SphericalSurface(s) => {
                    largest = largest.max(sphere_error(&reference, &s));
                }
                Surface::NurbsSurface(s) => {
                    // Cylinder extrusion is degree one in V; extract every U span independently.
                    let controls = s.control_points();
                    if controls.is_empty() || controls[0].len() != 2 || s.degrees() != (2, 1) {
                        return Err("unsupported curved surface certificate".into());
                    }
                    let columns: Vec<_> = (0..2)
                        .map(|j| {
                            bezier_spans(
                                s.knot_vector_u(),
                                controls.iter().map(|r| h(&r[j])).collect(),
                            )
                        })
                        .collect::<Result<_, _>>()?;
                    for (left, right) in columns[0].iter().zip(&columns[1]) {
                        let net: Vec<_> =
                            left.iter().zip(right).map(|(a, b)| vec![*a, *b]).collect();
                        largest = largest.max(rational_error(&reference, &net, p.spherical)?);
                        if !p.spherical {
                            for row in &net {
                                for (j, v) in row.iter().enumerate() {
                                    let expected =
                                        I::point(if j == 0 { -p.width } else { p.width });
                                    largest = largest.max((v[0] / v[3] - expected).abs());
                                }
                            }
                        }
                    }
                }
                _ => return Err("unsupported dish surface certificate".into()),
            }
        }
    }
    if needs_closure && !closure {
        return Err(
            "actual transformed closing plane does not certify outer-hull separation".into(),
        );
    }
    if !largest.is_finite() || largest > BOOLEAN_TOLERANCE {
        return Err(format!(
            "primitive/arc/transform error bound {largest} exceeds unchanged tolerance {BOOLEAN_TOLERANCE}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outward_arithmetic_and_bezier_extraction() {
        assert!((I::point(1.0) + I::point(f64::EPSILON / 2.0)).lo <= 1.0);
        let controls = [0.0, 1.0, 0.0, 1.0].map(|x| [I::point(x), ZERO, ZERO, ONE]);
        let spans = bezier_spans(&[0., 0., 0., 0.5, 1., 1., 1.], controls.to_vec()).unwrap();
        for (span, expected) in spans.iter().zip([[0.0, 1.0, 0.5], [0.5, 0.0, 1.0]]) {
            for (p, x) in span.iter().zip(expected) {
                assert!(p[0].lo <= x && p[0].hi >= x);
                assert!(p[3].lo <= 1.0 && p[3].hi >= 1.0);
            }
        }
        assert_eq!(spans.len(), 2);
        let reference = Reference::new(profile("dsa").unwrap()).unwrap();
        assert!(reference.sin.lo <= 0.0 && reference.sin.hi >= 0.0);
        assert!(reference.cos.lo <= 1.0 && reference.cos.hi >= 1.0);
    }
    #[test]
    fn bernstein_bounds_accept_circle_and_reject_control_error() {
        let p = Profile {
            top_d: 2.0,
            dish_depth: 1.0,
            height: 0.0,
            ..profile("dsa").unwrap()
        };
        let reference = Reference::new(p).unwrap();
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let mut arc = vec![
            vec![[ONE, ZERO, ZERO, ONE]],
            vec![[I::point(w), I::point(w), ZERO, I::point(w)]],
            vec![[ZERO, ONE, ZERO, ONE]],
        ];
        assert!(rational_error(&reference, &arc, true).unwrap() < 1e-13);
        arc[1][0][0] = arc[1][0][0] + I::point(0.01);
        assert!(rational_error(&reference, &arc, true).unwrap() > 0.001);
    }
}
