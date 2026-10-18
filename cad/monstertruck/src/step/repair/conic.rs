//! Rigorous end residual of an emitted elliptic arc against the original vertices.
//!
//! The emitted arc is `M * (cos t, sin t, 0, 1)` over its stored trim `(u, v)`.
//! Both `M` and the parameters are exact binary64 rationals; cosine and sine are
//! enclosed by a truncated Taylor series on a fixed rational grid, so the residual
//! is a true upper bound. Binary64 decoding and constructor rounding of `M` from
//! the source record stay a separate numerical term.
use super::*;
use monstertruck_io::step::load::step_geometry::Ellipse;
use monstertruck_modeling::{Matrix4, Point3};
use num_rational::BigRational as Q;
use num_traits::{One, Signed};

const GRID_BITS: u32 = 128;
const TERMS: u32 = 90;
/// Largest |t| whose series tail is far below the margin with `TERMS` terms.
const MAX_PARAMETER: f64 = 16.0;

/// Exact power of two as a rational (binary64 represents these exactly).
fn pow2(exponent: i32) -> Q {
    Q::from_float(2.0_f64.powi(exponent)).expect("finite power of two")
}
fn grid(value: Q) -> Q {
    let unit = pow2(GRID_BITS as i32);
    (value * &unit).floor() / unit
}

/// `[lo, hi]` enclosing `(cos t, sin t)`.
///
/// Terms shrink monotonically once `2k > |t|`; stopping when both are below
/// `2^-110` bounds the alternating tail by the same amount.
fn sin_cos(t: &Q) -> ([Q; 2], [Q; 2]) {
    let square = t * t;
    let mut cosine = Q::one();
    let mut sine = t.clone();
    let (mut c_term, mut s_term) = (Q::one(), t.clone());
    let stop = pow2(-110);
    let floor_k = t
        .abs()
        .to_integer()
        .to_u32_digits()
        .1
        .first()
        .copied()
        .unwrap_or(0);
    for k in 1..=TERMS {
        let (a, b) = (i64::from(2 * k - 1), i64::from(2 * k));
        c_term = grid(-(&c_term * &square) / Q::from_integer((a * b).into()));
        s_term = grid(-(&s_term * &square) / Q::from_integer((b * (b + 1)).into()));
        cosine += &c_term;
        sine += &s_term;
        if k > floor_k + 1 && c_term.abs() < stop && s_term.abs() < stop {
            break;
        }
    }
    // Tail and rounding stay below 2^-105 in total.
    let margin = pow2(-100);
    (
        [&cosine - &margin, &cosine + &margin],
        [&sine - &margin, &sine + &margin],
    )
}

fn scaled(interval: &[Q; 2], factor: &Q) -> [Q; 2] {
    let (a, b) = (&interval[0] * factor, &interval[1] * factor);
    if a <= b { [a, b] } else { [b, a] }
}

fn end_delta(matrix: &Matrix4, parameter: f64, vertex: &Point3) -> Result<[f64; 3], &'static str> {
    if !parameter.is_finite() || parameter.abs() > MAX_PARAMETER {
        return Err("conic arc parameter is outside the enclosed range");
    }
    // Arcs of one asset repeat a handful of parameters; the enclosure is pure.
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<u64, ([Q; 2], [Q; 2])>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    let (cos, sin) = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 4096 {
            cache.clear();
        }
        cache
            .entry(parameter.to_bits())
            .or_insert_with(|| sin_cos(&Q::from_float(parameter).expect("finite parameter")))
            .clone()
    });
    let mut delta = [0.0; 3];
    for axis in 0..3 {
        let entry = |col: usize| Q::from_float(matrix[col][axis]).ok_or("nonfinite conic matrix");
        let (x, y) = (scaled(&cos, &entry(0)?), scaled(&sin, &entry(1)?));
        let shift =
            entry(3)? - Q::from_float([vertex.x, vertex.y, vertex.z][axis]).unwrap_or_default();
        let (lo, hi) = (&x[0] + &y[0] + &shift, &x[1] + &y[1] + &shift);
        let worst = lo.abs().max(hi.abs());
        delta[axis] = sqrt_upper(&(&worst * &worst), "conic end residual range")?;
    }
    Ok(delta)
}

/// Largest world distance between the arc ends `(M e(u), M e(v))` and the original
/// native-frame vertices. `expected` must be exactly the constructor result for the
/// original record, which binds the emitted support to that record.
pub(super) fn end_residual(
    ellipse: &Ellipse<Point3, Matrix4>,
    expected: &Matrix4,
    ends: &[Point3; 2],
    factor: f64,
    placement: Matrix4,
) -> Result<f64, &'static str> {
    if ellipse.transform() != expected {
        return Err("emitted conic support is not the source record's constructor result");
    }
    let (u, v) = ellipse.entity().range();
    let mut worst = 0.0_f64;
    for (parameter, vertex) in [(u, &ends[0]), (v, &ends[1])] {
        let delta = end_delta(ellipse.transform(), parameter, vertex)?;
        worst = worst.max(super::placed_translation_bound(delta, factor, placement)?);
    }
    Ok(worst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taylor_enclosure_contains_known_values_and_is_tight() {
        for (t, cos, sin) in [
            (0.0_f64, 1.0_f64, 0.0_f64),
            (1.0, 1.0_f64.cos(), 1.0_f64.sin()),
            (-2.5, (-2.5_f64).cos(), (-2.5_f64).sin()),
            (6.283185307179586, 1.0, -2.4492935982947064e-16),
            (9.42477796076938, -1.0, 3.6739403974420594e-16),
        ] {
            let (c, s) = sin_cos(&Q::from_float(t).unwrap());
            for (enclosure, reference) in [(&c, cos), (&s, sin)] {
                let reference = Q::from_float(reference).unwrap();
                let slack = Q::new(1.into(), 1_000_000_000_000_i64.into());
                assert!(enclosure[0] <= &reference + &slack && &reference - &slack <= enclosure[1]);
                assert!(&enclosure[1] - &enclosure[0] < pow2(-90));
            }
        }
    }

    fn arc(matrix: Matrix4, range: (f64, f64)) -> Ellipse<Point3, Matrix4> {
        use monstertruck_modeling::{Processor, TrimmedCurve, UnitCircle};
        Processor::with_transform(TrimmedCurve::new(UnitCircle::new(), range), matrix)
    }

    #[test]
    fn partial_arc_ends_bind_to_their_own_vertices_and_the_source_constructor_result() {
        let frame = Matrix4::from_scale(2.0);
        let quarter = arc(frame, (0.0, std::f64::consts::FRAC_PI_2));
        let (start, end) = (Point3::new(2.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0));
        let world = Matrix4::from_scale(1.0);
        // cos(pi/2 as binary64) = 6.1e-17, so the far end misses by 1.2e-16.
        let r = end_residual(&quarter, &frame, &[start, end], 1.0, world).unwrap();
        assert!((1.2e-16..1e-15).contains(&r), "{r}");
        // Swapped vertices are the other arc direction: the ends miss by 2.8.
        let swapped = end_residual(&quarter, &frame, &[end, start], 1.0, world).unwrap();
        assert!(swapped > 2.8, "{swapped}");
        // A transform that is not the constructor result of the record is refused.
        let other = Matrix4::from_scale(2.0 + f64::EPSILON * 4.0);
        assert!(end_residual(&quarter, &other, &[start, end], 1.0, world).is_err());
    }

    #[test]
    fn identity_matrix_residual_is_the_distance_to_the_unit_circle_point() {
        let id = Matrix4::from_scale(1.0);
        let delta = end_delta(&id, 0.0, &Point3::new(1.0, 0.25, 0.0)).unwrap();
        assert!(delta[0] < 1e-25 && (0.25..=0.25 * (1.0 + 1e-15)).contains(&delta[1]));
        assert!(end_delta(&id, 17.0, &Point3::new(1.0, 0.0, 0.0)).is_err());
    }
}
