use super::*;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};

type R3 = [BigRational; 3];
fn vector(v: Vector3) -> Option<R3> {
    Some([
        BigRational::from_float(v.x)?,
        BigRational::from_float(v.y)?,
        BigRational::from_float(v.z)?,
    ])
}
fn cross(a: &R3, b: &R3) -> R3 {
    [
        &a[1] * &b[2] - &a[2] * &b[1],
        &a[2] * &b[0] - &a[0] * &b[2],
        &a[0] * &b[1] - &a[1] * &b[0],
    ]
}
fn dot(a: &R3, b: &R3) -> BigRational {
    &a[0] * &b[0] + &a[1] * &b[1] + &a[2] * &b[2]
}

/// An exact affine-plane equation constructed from finite authored coefficients.
///
/// Arithmetic preserves construction provenance through affine transforms; it
/// does not infer coincidence from sampled normals or a spatial tolerance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffinePlaneCertificate {
    normal: R3,
    offset: BigRational,
}
impl AffinePlaneCertificate {
    /// Constructs the plane through three finite, non-collinear authored points.
    pub fn from_points(origin: Point3, one: Point3, another: Point3) -> Option<Self> {
        let o = vector(origin.to_vec())?;
        let p = vector(one.to_vec())?;
        let q = vector(another.to_vec())?;
        let u = std::array::from_fn(|i| &p[i] - &o[i]);
        let v = std::array::from_fn(|i| &q[i] - &o[i]);
        let normal = cross(&u, &v);
        if normal.iter().all(Zero::is_zero) {
            return None;
        }
        let offset = -dot(&normal, &o);
        Some(Self { normal, offset })
    }
    /// Transports the equation by a finite nonsingular affine matrix (column-major).
    pub fn transformed(&self, matrix: [[f64; 4]; 4]) -> Option<Self> {
        if matrix[0][3] != 0.0 || matrix[1][3] != 0.0 || matrix[2][3] != 0.0 || matrix[3][3] != 1.0
        {
            return None;
        }
        let columns: Vec<R3> = matrix
            .iter()
            .map(|c| vector(Vector3::new(c[0], c[1], c[2])))
            .collect::<Option<_>>()?;
        let cofactors = [
            cross(&columns[1], &columns[2]),
            cross(&columns[2], &columns[0]),
            cross(&columns[0], &columns[1]),
        ];
        let determinant = dot(&columns[0], &cofactors[0]);
        if determinant.is_zero() {
            return None;
        }
        let normal =
            std::array::from_fn(|i| (0..3).map(|j| &self.normal[j] * &cofactors[j][i]).sum());
        let offset = determinant * &self.offset - dot(&normal, &columns[3]);
        Some(Self { normal, offset })
    }
    /// Whether both equations denote the same plane, independently of orientation.
    pub fn coincides(&self, other: &Self) -> bool {
        let i = self
            .normal
            .iter()
            .position(|x| !x.is_zero())
            .expect("nonzero certified normal");
        (0..3).all(|j| &self.normal[j] * &other.normal[i] == &other.normal[j] * &self.normal[i])
            && &self.offset * &other.normal[i] == &other.offset * &self.normal[i]
    }
    /// A finite unoriented unit normal; its sign does not supply material orientation.
    /// Returns `None` when conversion to the working floating range fails.
    pub fn normal(&self) -> Option<Vector3> {
        let scale = self
            .normal
            .iter()
            .map(|n| {
                if n < &BigRational::zero() {
                    -n
                } else {
                    n.clone()
                }
            })
            .max()?;
        let x = (&self.normal[0] / &scale).to_f64()?;
        let y = (&self.normal[1] / &scale).to_f64()?;
        let z = (&self.normal[2] / &scale).to_f64()?;
        Some(Vector3::new(x, y, z).normalize())
    }
    /// Orthogonally projects finite homogeneous coordinates, preserving their weight.
    /// Zero weight projects a vector; no division by the weight is performed.
    pub fn project_homogeneous(&self, value: Vector4) -> Option<Vector4> {
        let xyz = vector(value.truncate())?;
        let weight = BigRational::from_float(value.w)?;
        let distance =
            (dot(&self.normal, &xyz) + &self.offset * weight) / dot(&self.normal, &self.normal);
        let projected: Vec<f64> = (0..3)
            .map(|i| (&xyz[i] - &self.normal[i] * &distance).to_f64())
            .collect::<Option<_>>()?;
        projected
            .iter()
            .all(|x| x.is_finite())
            .then(|| Vector4::new(projected[0], projected[1], projected[2], value.w))
    }
    /// Orthogonally projects a finite point using the exact plane coefficients.
    pub fn project_point(&self, point: Point3) -> Option<Point3> {
        self.project_homogeneous(point.to_homogeneous())
            .map(|value| Point3::new(value.x, value.y, value.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn xy(z: f64) -> AffinePlaneCertificate {
        AffinePlaneCertificate::from_points(
            Point3::new(0.0, 0.0, z),
            Point3::new(1.0, 0.0, z),
            Point3::new(0.0, 1.0, z),
        )
        .unwrap()
    }
    #[test]
    fn exact_equations_distinguish_subnormal_separation_and_degenerate_inputs() {
        assert!(!xy(0.0).coincides(&xy(f64::from_bits(1))));
        assert!(
            AffinePlaneCertificate::from_points(
                Point3::origin(),
                Point3::origin(),
                Point3::origin()
            )
            .is_none()
        );
        assert!(
            AffinePlaneCertificate::from_points(
                Point3::new(f64::INFINITY, 0.0, 0.0),
                Point3::origin(),
                Point3::new(1.0, 0.0, 0.0)
            )
            .is_none()
        );
    }
    #[test]
    fn authored_covectors_preserve_coincidence_and_projection_after_shear() {
        let a = xy(2.0);
        let b = AffinePlaneCertificate::from_points(
            Point3::new(3.0, 4.0, 2.0),
            Point3::new(3.0, 8.0, 2.0),
            Point3::new(5.0, 4.0, 2.0),
        )
        .unwrap();
        let matrix = [
            [1.1, 0.2, 0.3, 0.0],
            [0.4, 1.7, 0.6, 0.0],
            [0.7, 0.8, 2.3, 0.0],
            [3.1, 5.2, -7.3, 1.0],
        ];
        let a = a.transformed(matrix).unwrap();
        assert!(a.coincides(&b.transformed(matrix).unwrap()));
        let point = Point3::new(1.2, -3.4, 5.6);
        let projected = a.project_point(point).unwrap();
        assert!((a.project_point(projected).unwrap() - projected).magnitude() < 1e-12);
        let normal = a.normal().unwrap();
        assert!((point - projected).cross(normal).magnitude() < 1e-12);
    }
    #[test]
    fn homogeneous_projection_preserves_weights_and_projects_vectors() {
        let plane = xy(2.0);
        assert_eq!(
            plane.project_homogeneous(Vector4::new(3.0, 4.0, 9.0, 2.0)),
            Some(Vector4::new(3.0, 4.0, 4.0, 2.0))
        );
        assert_eq!(
            plane.project_homogeneous(Vector4::new(3.0, 4.0, 9.0, 0.0)),
            Some(Vector4::new(3.0, 4.0, 0.0, 0.0))
        );
        assert_eq!(
            plane.project_homogeneous(Vector4::new(3.0, 4.0, 9.0, -2.0)),
            Some(Vector4::new(3.0, 4.0, -4.0, -2.0))
        );
        let value = Vector4::new(3.0, 4.0, 9.0, 2.0);
        let point = Point3::from_homogeneous(value);
        assert_eq!(
            Point3::from_homogeneous(plane.project_homogeneous(value).unwrap()),
            plane.project_point(point).unwrap()
        );
        assert!(
            plane
                .project_homogeneous(Vector4::new(f64::INFINITY, 0.0, 0.0, 1.0))
                .is_none()
        );
        assert!(
            plane
                .project_homogeneous(Vector4::new(0.0, 0.0, 0.0, f64::NAN))
                .is_none()
        );
    }
    #[test]
    fn transport_refuses_singular_projective_and_nonfinite_matrices() {
        let matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut bad = matrix;
        bad[0][0] = 0.0;
        assert!(xy(0.0).transformed(bad).is_none());
        let mut bad = matrix;
        bad[0][3] = 0.01;
        assert!(xy(0.0).transformed(bad).is_none());
        let mut bad = matrix;
        bad[3][0] = f64::NAN;
        assert!(xy(0.0).transformed(bad).is_none());
    }
}
