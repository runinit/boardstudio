use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

pub(super) type Point = [Q; 3];
pub(super) type Triangle = [Point; 3];
pub(super) type FPoint = [f64; 3];
pub(super) type FTriangle = [FPoint; 3];
pub(super) type Check<T> = Result<T, &'static str>;

pub(super) fn decimal(s: &str) -> Check<Q> {
    if s.len() > 80 {
        return Err("decimal limit");
    }
    let (mantissa, exponent) = s.split_once(['e', 'E']).unwrap_or((s, "0"));
    let exponent: i32 = exponent.parse().map_err(|_| "decimal exponent")?;
    if !(-64..=64).contains(&exponent) {
        return Err("decimal exponent limit");
    }
    let fractional = mantissa.split_once('.').map_or(0, |(_, x)| x.len() as i32);
    let digits = mantissa.replace('.', "");
    if !digits
        .trim_start_matches(['-', '+'])
        .bytes()
        .all(|c| c.is_ascii_digit())
    {
        return Err("decimal syntax");
    }
    let power = exponent - fractional;
    let fraction = if power >= 0 {
        format!("{}{}", digits, "0".repeat(power as usize))
    } else {
        format!("{digits}/1{}", "0".repeat((-power) as usize))
    };
    fraction.parse().map_err(|_| "decimal syntax")
}

pub(super) fn point(p: FPoint) -> Check<Point> {
    let mut out = std::array::from_fn(|_| Q::zero());
    for (a, b) in out.iter_mut().zip(p) {
        if !b.is_finite() {
            return Err("nonfinite coordinate");
        }
        if b != 0.0 && !(2f64.powi(-128)..=2f64.powi(128)).contains(&b.abs()) {
            return Err("exponent cap");
        }
        *a = Q::from_float(b).ok_or("nonfinite coordinate")?;
    }
    Ok(out)
}
pub(super) fn sub(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|i| &a[i] - &b[i])
}
pub(super) fn add(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|i| &a[i] + &b[i])
}
pub(super) fn mul(a: &Point, s: &Q) -> Point {
    std::array::from_fn(|i| &a[i] * s)
}
pub(super) fn dot(a: &Point, b: &Point) -> Q {
    (0..3).map(|i| &a[i] * &b[i]).sum()
}
pub(super) fn cross(a: &Point, b: &Point) -> Point {
    [
        &a[1] * &b[2] - &a[2] * &b[1],
        &a[2] * &b[0] - &a[0] * &b[2],
        &a[0] * &b[1] - &a[1] * &b[0],
    ]
}
pub(super) fn norm2(a: &Point) -> Q {
    dot(a, a)
}
pub(super) fn normal(t: &Triangle) -> Point {
    cross(&sub(&t[1], &t[0]), &sub(&t[2], &t[0]))
}
pub(super) fn orient(a: &Point, b: &Point, c: &Point, axis: usize) -> Q {
    cross(&sub(b, a), &sub(c, a))[axis].clone()
}
pub(super) fn edges(t: &Triangle) -> impl Iterator<Item = (&Point, &Point)> {
    (0..3).map(|i| (&t[i], &t[(i + 1) % 3]))
}
pub(super) fn in_triangle(p: &Point, t: &Triangle, n: &Point) -> bool {
    edges(t).all(|(a, b)| dot(&cross(&sub(b, a), &sub(p, a)), n) >= Q::zero())
}
pub(super) fn closest_segment(p: &Point, a: &Point, b: &Point) -> Point {
    let d = sub(b, a);
    let n = norm2(&d);
    if n.is_zero() {
        return a.clone();
    }
    let u = (dot(&sub(p, a), &d) / n).max(Q::zero()).min(Q::one());
    add(a, &mul(&d, &u))
}

fn coplanar_hit(p: &Point, q: &Point, t: &Triangle, n: &Point) -> bool {
    if in_triangle(p, t, n) || in_triangle(q, t, n) {
        return true;
    }
    let axis = (0..3).max_by_key(|&i| n[i].abs()).unwrap();
    edges(t).any(|(a, b)| {
        let d = sub(q, p);
        let e = sub(b, a);
        let w = sub(a, p);
        let denominator = cross(&d, &e)[axis].clone();
        if denominator.is_zero() {
            return cross(&w, &d)[axis].is_zero()
                && (closest_segment(p, a, b) == *p
                    || closest_segment(q, a, b) == *q
                    || closest_segment(a, p, q) == *a
                    || closest_segment(b, p, q) == *b);
        }
        let u = &cross(&w, &e)[axis] / &denominator;
        let v = &cross(&w, &d)[axis] / &denominator;
        u >= Q::zero() && u <= Q::one() && v >= Q::zero() && v <= Q::one()
    })
}
fn segment_hit(p: &Point, q: &Point, t: &Triangle, n: &Point) -> bool {
    let a = dot(&sub(p, &t[0]), n);
    let b = dot(&sub(q, &t[0]), n);
    if a.is_zero() && b.is_zero() {
        return coplanar_hit(p, q, t, n);
    }
    if (a.is_positive() && b.is_positive()) || (a.is_negative() && b.is_negative()) {
        return false;
    }
    let u = &a / (&a - &b);
    let h = add(p, &mul(&sub(q, p), &u));
    in_triangle(&h, t, n)
}
// After intersection exclusion, a closest pair on two compact triangles lies on
// vertex/face or edge/edge features; parallel minima include one of these pairs.
pub(super) fn distance2(a: &Triangle, b: &Triangle) -> Check<Q> {
    let na = normal(a);
    let nb = normal(b);
    if norm2(&na).is_zero() || norm2(&nb).is_zero() {
        return Err("degenerate triangle");
    }
    if edges(a).any(|(p, q)| segment_hit(p, q, b, &nb))
        || edges(b).any(|(p, q)| segment_hit(p, q, a, &na))
    {
        return Ok(Q::zero());
    }
    let mut best = norm2(&sub(&a[0], &b[0]));
    for (points, face, n) in [(a, b, &nb), (b, a, &na)] {
        for p in points {
            let u = dot(&sub(p, &face[0]), n) / norm2(n);
            let h = sub(p, &mul(n, &u));
            if in_triangle(&h, face, n) {
                best = best.min(norm2(&sub(p, &h)));
            }
            for (x, y) in edges(face) {
                best = best.min(norm2(&sub(p, &closest_segment(p, x, y))));
            }
        }
    }
    for (p, q) in edges(a) {
        for (r, s) in edges(b) {
            let u = sub(q, p);
            let v = sub(s, r);
            let w = sub(p, r);
            let aa = norm2(&u);
            let bb = dot(&u, &v);
            let cc = norm2(&v);
            let dd = dot(&u, &w);
            let ee = dot(&v, &w);
            let det = &aa * &cc - &bb * &bb;
            if !det.is_zero() {
                let x = (&bb * &ee - &cc * &dd) / &det;
                let y = (&aa * &ee - &bb * &dd) / det;
                if x >= Q::zero() && x <= Q::one() && y >= Q::zero() && y <= Q::one() {
                    best = best.min(norm2(&sub(&add(p, &mul(&u, &x)), &add(r, &mul(&v, &y)))));
                }
            }
        }
    }
    Ok(best)
}

// Binary search the ordered positive f64 values; endpoint squares are compared exactly.
pub(super) fn sqrt_bounds(q: &Q) -> Check<[f64; 2]> {
    if q < &Q::zero() {
        return Err("negative squared distance");
    }
    let mut lo = 0u64;
    let mut hi = f64::MAX.to_bits();
    let square = |bits| {
        let x = Q::from_float(f64::from_bits(bits)).unwrap();
        &x * &x
    };
    if square(hi) < *q {
        return Err("distance range");
    }
    while lo < hi {
        let m = lo + (hi - lo).div_ceil(2);
        if square(m) <= *q {
            lo = m;
        } else {
            hi = m - 1;
        }
    }
    let lower = f64::from_bits(lo);
    Ok([
        lower,
        if square(lo) == *q {
            lower
        } else {
            lower.next_up()
        },
    ])
}
pub(super) fn upper(q: &Q) -> Check<f64> {
    let magnitude = q.abs();
    let mut lo = 0u64;
    let mut hi = f64::MAX.to_bits();
    let value = |bits| Q::from_float(f64::from_bits(bits)).unwrap();
    if value(hi) < magnitude {
        return Err("rational range");
    }
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if value(mid) <= magnitude {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let floor = f64::from_bits(lo);
    Ok(if q.is_negative() {
        -floor
    } else if value(lo) == magnitude {
        floor
    } else {
        floor.next_up()
    })
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Interval {
    pub lo: f64,
    pub hi: f64,
}
impl Interval {
    pub fn point(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn bound(lo: f64, hi: f64) -> Self {
        if !lo.is_finite() || !hi.is_finite() {
            Self {
                lo: f64::NEG_INFINITY,
                hi: f64::INFINITY,
            }
        } else {
            Self {
                lo: lo.next_down(),
                hi: hi.next_up(),
            }
        }
    }
    pub fn add(self, b: Self) -> Self {
        Self::bound(self.lo + b.lo, self.hi + b.hi)
    }
    pub fn sub(self, b: Self) -> Self {
        Self::bound(self.lo - b.hi, self.hi - b.lo)
    }
    pub fn mul(self, b: Self) -> Self {
        let x = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        Self::bound(
            x.into_iter().fold(f64::INFINITY, f64::min),
            x.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    pub fn div_positive(self, b: Self) -> Self {
        if b.lo <= 0.0 {
            return Self {
                lo: 0.0,
                hi: f64::INFINITY,
            };
        }
        self.mul(Self::bound(1.0 / b.hi, 1.0 / b.lo))
    }
}
