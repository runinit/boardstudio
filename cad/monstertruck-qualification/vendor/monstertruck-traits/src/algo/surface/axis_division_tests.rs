use super::*;

#[derive(Clone, Copy)]
struct UCurved;
#[derive(Clone, Copy)]
struct VCurved;
#[derive(Clone, Copy)]
struct BothCurved;
#[derive(Clone, Copy)]
struct UVSaddle;
#[derive(Clone, Copy)]
struct CancellationSurface;

macro_rules! curved_surface {
    ($type:ty, $u:ident, $v:ident, $curve:expr, $du:expr, $dv:expr, $duu:expr, $dvv:expr) => {
        impl ParametricSurface for $type {
            type Point = Point3;
            type Vector = Vector3;
            fn evaluate(&self, $u: f64, $v: f64) -> Point3 {
                Point3::new($u, $v, ($curve)($u, $v))
            }
            fn derivative_u(&self, u: f64, _: f64) -> Vector3 {
                Vector3::new(1.0, 0.0, ($du)(u, 0.0))
            }
            fn derivative_v(&self, _: f64, v: f64) -> Vector3 {
                Vector3::new(0.0, 1.0, ($dv)(0.0, v))
            }
            fn derivative_uu(&self, u: f64, _: f64) -> Vector3 {
                Vector3::new(0.0, 0.0, ($duu)(u, 0.0))
            }
            fn derivative_uv(&self, _: f64, _: f64) -> Vector3 {
                Vector3::zero()
            }
            fn derivative_vv(&self, _: f64, v: f64) -> Vector3 {
                Vector3::new(0.0, 0.0, ($dvv)(0.0, v))
            }
            fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
                match (m, n) {
                    (0, 0) => self.evaluate(u, v).to_vec(),
                    (1, 0) => self.derivative_u(u, v),
                    (0, 1) => self.derivative_v(u, v),
                    (2, 0) => self.derivative_uu(u, v),
                    (1, 1) => self.derivative_uv(u, v),
                    (0, 2) => self.derivative_vv(u, v),
                    _ => Vector3::zero(),
                }
            }
        }
    };
}

curved_surface!(UCurved, u, v, |u: f64, _: f64| u.sin(), |u: f64, _: f64| u.cos(), |_: f64, _: f64| 0.0, |u: f64, _: f64| -u.sin(), |_: f64, _: f64| 0.0);
curved_surface!(VCurved, u, v, |_: f64, v: f64| v.sin(), |_: f64, _: f64| 0.0, |_: f64, v: f64| v.cos(), |_: f64, _: f64| 0.0, |_: f64, v: f64| -v.sin());
curved_surface!(
    BothCurved,
    u,
    v,
    |u: f64, v: f64| u.sin() + v.sin(),
    |u: f64, _: f64| u.cos(),
    |_: f64, v: f64| v.cos(),
    |u: f64, _: f64| -u.sin(),
    |_: f64, v: f64| -v.sin()
);

fn max_bilinear_residual<S>(surface: &S, udiv: &[f64], vdiv: &[f64]) -> f64
where
    S: ParametricSurface<Point = Point3>,
{
    let mut max_residual: f64 = 0.0;
    for u in udiv.windows(2) {
        for v in vdiv.windows(2) {
            let p00 = surface.evaluate(u[0], v[0]);
            let p01 = surface.evaluate(u[0], v[1]);
            let p10 = surface.evaluate(u[1], v[0]);
            let p11 = surface.evaluate(u[1], v[1]);
            for i in 0..=16 {
                for j in 0..=16 {
                    let p = i as f64 / 16.0;
                    let q = j as f64 / 16.0;
                    let interpolated = Point3::from_vec(
                        p00.to_vec() * (1.0 - p) * (1.0 - q)
                            + p01.to_vec() * (1.0 - p) * q
                            + p10.to_vec() * p * (1.0 - q)
                            + p11.to_vec() * p * q,
                    );
                    let actual = surface.evaluate(
                        u[0] * (1.0 - p) + u[1] * p,
                        v[0] * (1.0 - q) + v[1] * q,
                    );
                    max_residual = max_residual.max(actual.distance(interpolated));
                }
            }
        }
    }
    max_residual
}

#[test]
fn u_curvature_refines_only_u_for_a_u_linear_v_surface() {
    let (udiv, vdiv) = parameter_division(&UCurved, ((0.0, 1.0), (0.0, 1000.0)), 0.01);
    assert!(udiv.len() > 2);
    assert_eq!(vdiv.len(), 2, "linear v must retain only its endpoints");
    assert_eq!((vdiv[0], vdiv[1]), (0.0, 1000.0));
    assert!(max_bilinear_residual(&UCurved, &udiv, &vdiv) <= 0.01);
}

#[test]
fn v_curvature_refines_only_v_for_a_v_linear_u_surface() {
    let (udiv, vdiv) = parameter_division(&VCurved, ((0.0, 1000.0), (0.0, 1.0)), 0.01);
    assert_eq!(udiv.len(), 2, "linear u must retain only its endpoints");
    assert_eq!((udiv[0], udiv[1]), (0.0, 1000.0));
    assert!(vdiv.len() > 2);
    assert!(max_bilinear_residual(&VCurved, &udiv, &vdiv) <= 0.01);
}

#[test]
fn two_curved_axes_refine_and_meet_dense_interpolation_residual() {
    let tol = 0.01;
    let (udiv, vdiv) = parameter_division(&BothCurved, ((0.0, 1.0), (0.0, 1.0)), tol);
    assert!(udiv.len() > 2);
    assert!(vdiv.len() > 2);
    assert!(max_bilinear_residual(&BothCurved, &udiv, &vdiv) <= tol);
}

impl ParametricSurface for CancellationSurface {
    type Point = Point3;
    type Vector = Vector3;
    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        Point3::new(u, v, u * u * (v - 0.5))
    }
    fn derivative_u(&self, u: f64, v: f64) -> Vector3 {
        Vector3::new(1.0, 0.0, 2.0 * u * (v - 0.5))
    }
    fn derivative_v(&self, u: f64, _: f64) -> Vector3 {
        Vector3::new(0.0, 1.0, u * u)
    }
    fn derivative_uu(&self, _: f64, v: f64) -> Vector3 {
        Vector3::new(0.0, 0.0, 2.0 * (v - 0.5))
    }
    fn derivative_uv(&self, u: f64, _: f64) -> Vector3 {
        Vector3::new(0.0, 0.0, 2.0 * u)
    }
    fn derivative_vv(&self, _: f64, _: f64) -> Vector3 {
        Vector3::zero()
    }
    fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
        match (m, n) {
            (0, 0) => self.evaluate(u, v).to_vec(),
            (1, 0) => self.derivative_u(u, v),
            (0, 1) => self.derivative_v(u, v),
            (2, 0) => self.derivative_uu(u, v),
            (1, 1) => self.derivative_uv(u, v),
            (0, 2) => self.derivative_vv(u, v),
            _ => Vector3::zero(),
        }
    }
}

#[test]
fn inconclusive_directional_residuals_refine_both_axes() {
    let tol = 0.01;
    let (udiv, vdiv) = parameter_division(&CancellationSurface, ((0.0, 1.0), (0.0, 1.0)), tol);
    assert!(udiv.len() > 2);
    assert!(vdiv.len() > 2);
    assert!(max_bilinear_residual(&CancellationSurface, &udiv, &vdiv) <= tol);
}

impl ParametricSurface for UVSaddle {
    type Point = Point3;
    type Vector = Vector3;
    fn evaluate(&self, u: f64, v: f64) -> Point3 {
        Point3::new(u, v, u * v)
    }
    fn derivative_u(&self, _: f64, v: f64) -> Vector3 {
        Vector3::new(1.0, 0.0, v)
    }
    fn derivative_v(&self, u: f64, _: f64) -> Vector3 {
        Vector3::new(0.0, 1.0, u)
    }
    fn derivative_uu(&self, _: f64, _: f64) -> Vector3 {
        Vector3::zero()
    }
    fn derivative_uv(&self, _: f64, _: f64) -> Vector3 {
        Vector3::new(0.0, 0.0, 1.0)
    }
    fn derivative_vv(&self, _: f64, _: f64) -> Vector3 {
        Vector3::zero()
    }
    fn derivative_mn(&self, m: usize, n: usize, u: f64, v: f64) -> Vector3 {
        match (m, n) {
            (0, 0) => self.evaluate(u, v).to_vec(),
            (1, 0) => self.derivative_u(u, v),
            (0, 1) => self.derivative_v(u, v),
            (1, 1) => self.derivative_uv(u, v),
            _ => Vector3::zero(),
        }
    }
}

#[test]
fn bilinear_saddle_keeps_the_existing_bilinear_interpolation_contract() {
    let (udiv, vdiv) = parameter_division(&UVSaddle, ((0.0, 1.0), (0.0, 1.0)), 0.01);
    assert_eq!(udiv, vec![0.0, 1.0]);
    assert_eq!(vdiv, vec![0.0, 1.0]);
}
