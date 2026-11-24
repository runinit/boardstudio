//! Shared internal 3D vector helpers over `[f64; 3]` and `[BigRational; 3]`.
//!
//! These bodies are verbatim copies of the helpers previously duplicated in
//! `step_accuracy`, `collision_qualification::exact`,
//! `collision_qualification::query`, and
//! `qualification::final_trace::cylinder_proof`, so floating-point evaluation
//! order stays bit-for-bit identical. Only helpers with identical argument and
//! return types were consolidated; polynomial coefficient arithmetic,
//! 2D/bulge contour math, and `cgmath` vector code intentionally live elsewhere.

use num_rational::BigRational as Q;

/// Floating-point (`[f64; 3]`, by value) vector math.
pub(crate) mod f64 {
    pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|i| a[i] - b[i])
    }
    pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
}

/// Exact-rational (`[Q; 3]`, by reference) vector math.
pub(crate) mod q {
    use super::Q;

    pub(crate) fn sub(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
        std::array::from_fn(|i| &a[i] - &b[i])
    }
    pub(crate) fn add(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
        std::array::from_fn(|i| &a[i] + &b[i])
    }
    pub(crate) fn mul(a: &[Q; 3], s: &Q) -> [Q; 3] {
        std::array::from_fn(|i| &a[i] * s)
    }
    pub(crate) fn dot(a: &[Q; 3], b: &[Q; 3]) -> Q {
        (0..3).map(|i| &a[i] * &b[i]).sum()
    }
    pub(crate) fn cross(a: &[Q; 3], b: &[Q; 3]) -> [Q; 3] {
        [
            &a[1] * &b[2] - &a[2] * &b[1],
            &a[2] * &b[0] - &a[0] * &b[2],
            &a[0] * &b[1] - &a[1] * &b[0],
        ]
    }
    pub(crate) fn norm2(a: &[Q; 3]) -> Q {
        dot(a, a)
    }
}
