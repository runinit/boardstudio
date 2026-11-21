use super::{List, Ref, StepResult, Writer};
use crate::profile::CircleEdge;
use monstertruck_io::step::load::step_geometry::TryIntoHomogeneousBsplineSurface;
use monstertruck_modeling::{
    Curve, Decomposition, Homogeneous, InnerSpace, Line, Matrix4, NurbsSurface, Origin, Point3,
    Processor, RevolutionSurface, Surface, Tolerance, Transform, Transformed, Vector3,
};
use std::fmt::{self, Display};

impl Writer {
    pub(super) fn point(&mut self, p: Point3) -> Ref {
        let values = List(&[Real(p.x), Real(p.y), Real(p.z)]);
        self.entity(format_args!("CARTESIAN_POINT('', {values})"))
    }

    fn direction(&mut self, v: Vector3) -> Ref {
        let values = List(&[Real(v.x), Real(v.y), Real(v.z)]);
        self.entity(format_args!("DIRECTION('', {values})"))
    }

    fn placement(&mut self, origin: Point3, normal: Vector3, reference: Vector3) -> Ref {
        let origin = self.point(origin);
        let normal = self.direction(normal);
        let reference = self.direction(reference);
        self.entity(format_args!(
            "AXIS2_PLACEMENT_3D('', {origin}, {normal}, {reference})"
        ))
    }

    pub(super) fn circle(&mut self, circle: &CircleEdge, start: Point3) -> Ref {
        // STEP trims the circle by the EDGE_CURVE vertices in its positive
        // parameter direction. The signed axis preserves which arc they bound.
        let placement = self.placement(
            circle.center,
            circle.axis_z * Vector3::unit_z(),
            (start - circle.center).normalize(),
        );
        self.entity(format_args!(
            "CIRCLE('', {placement}, {})",
            Real(circle.radius)
        ))
    }

    pub(super) fn curve(&mut self, curve: &Curve) -> StepResult<Ref> {
        Ok(match curve {
            Curve::Line(Line(a, b)) => {
                let delta = b - a;
                let magnitude = delta.magnitude();
                let origin = self.point(*a);
                let axis = self.direction(delta / magnitude);
                let vector = self.entity(format_args!("VECTOR('', {axis}, {})", Real(magnitude)));
                self.entity(format_args!("LINE('', {origin}, {vector})"))
            }
            Curve::BsplineCurve(curve) => {
                let controls = curve
                    .control_points()
                    .iter()
                    .map(|&p| self.point(p))
                    .collect::<Vec<_>>();
                let (knots, multiplicities) = curve.knot_vector().to_single_multi();
                let knots = reals(&knots);
                self.entity(format_args!(
                    "B_SPLINE_CURVE_WITH_KNOTS('', {}, {}, .UNSPECIFIED., .U., .U., {}, {}, .UNSPECIFIED.)",
                    curve.degree(), List(&controls), List(&multiplicities), List(&knots),
                ))
            }
            Curve::NurbsCurve(curve) => {
                let controls = curve
                    .control_points()
                    .iter()
                    .map(|p| self.point(p.to_point()))
                    .collect::<Vec<_>>();
                let weights = curve
                    .control_points()
                    .iter()
                    .map(|p| Real(p.weight()))
                    .collect::<Vec<_>>();
                let (knots, multiplicities) = curve.knot_vector().to_single_multi();
                let knots = reals(&knots);
                self.entity(format_args!(
                    "(BOUNDED_CURVE()
B_SPLINE_CURVE({}, {}, .UNSPECIFIED., .U., .U.)
B_SPLINE_CURVE_WITH_KNOTS({}, {}, .UNSPECIFIED.)
CURVE()
GEOMETRIC_REPRESENTATION_ITEM()
RATIONAL_B_SPLINE_CURVE({})
REPRESENTATION_ITEM(''))",
                    curve.degree(),
                    List(&controls),
                    List(&multiplicities),
                    List(&knots),
                    List(&weights),
                ))
            }
            _ => return Err("unsupported exact edge curve"),
        })
    }

    /// Returns the support and whether its STEP parameterization has the source normal.
    pub(super) fn surface(&mut self, surface: &Surface) -> StepResult<(Ref, bool)> {
        let support = match surface {
            Surface::Plane(plane) => {
                let axis =
                    self.placement(plane.origin(), plane.normal(), plane.axis_u().normalize());
                self.entity(format_args!("PLANE('', {axis})"))
            }
            Surface::RevolutionSurface(revolution) => return self.revolution(revolution),
            Surface::BsplineSurface(surface) => {
                let controls = surface
                    .control_points()
                    .iter()
                    .map(|row| row.iter().map(|&p| self.point(p)).collect::<Vec<_>>())
                    .collect::<Vec<_>>();
                self.entity(format_args!(
                    "B_SPLINE_SURFACE_WITH_KNOTS('', {}, {}, {}, .UNSPECIFIED., .U., .U., .U., {}, .UNSPECIFIED.)",
                    surface.udegree(), surface.vdegree(), rows(&controls), surface_knots(
                        surface.knot_vector_u().to_single_multi(),
                        surface.knot_vector_v().to_single_multi(),
                    ),
                ))
            }
            Surface::NurbsSurface(surface) => self.nurbs_surface(surface),
            _ => return Err("unsupported exact face support"),
        };
        Ok((support, true))
    }

    fn revolution(
        &mut self,
        surface: &Processor<RevolutionSurface<Curve>, Matrix4>,
    ) -> StepResult<(Ref, bool)> {
        let revolution = surface.entity();
        let transform = surface.transform();
        let axis = revolution.axis().normalize();
        if let Curve::Line(Line(a, b)) = revolution.entity_curve()
            && (*b - *a).normalize().cross(axis).so_small()
            && let Some((_, scale, _)) = transform.iwasawa_decomposition()
            && scale[0][0].near(&scale[1][1])
            && scale[1][1].near(&scale[2][2])
        {
            let origin = revolution.origin();
            let height = (*a - origin).dot(axis);
            let foot = origin + height * axis;
            let radial = (*a - origin) - height * axis;
            if !radial.so_small() {
                let radial = transform.transform_vector(radial);
                let world_axis = transform.transform_vector(axis).normalize();
                let placement = self.placement(
                    transform.transform_point(foot),
                    world_axis,
                    radial.normalize(),
                );
                let support = self.entity(format_args!(
                    "CYLINDRICAL_SURFACE('', {placement}, {})",
                    Real(radial.magnitude())
                ));
                // Compare the actual source normal, including processor reversal
                // and handedness, to the outward STEP cylinder normal.
                use monstertruck_modeling::ParametricSurface3D;
                let normal = surface.normal(0.5, 0.0);
                return Ok((support, normal.dot(radial) > 0.0));
            }
        }

        // Generated countersinks retain the exact directed rational net used
        // by the independent source and trim certificate.
        if let Some(net) = surface.try_into_homogeneous_bspline_surface() {
            return Ok((self.nurbs_surface(&NurbsSurface::new(net)), true));
        }

        let (rotation, scale, _) = transform
            .iwasawa_decomposition()
            .ok_or("singular revolution transform")?;
        if !scale[0][0].near(&scale[1][1]) || !scale[1][1].near(&scale[2][2]) {
            return Err("nonuniform revolution transform");
        }
        let curve = self.curve(&revolution.entity_curve().transformed(*transform))?;
        let origin = self.point(transform.transform_point(revolution.origin()));
        let axis = self.direction(rotation.transform_vector(axis));
        let placement = self.entity(format_args!("AXIS1_PLACEMENT('', {origin}, {axis})"));
        let support = self.entity(format_args!(
            "SURFACE_OF_REVOLUTION('', {curve}, {placement})"
        ));
        Ok((support, !surface.orientation()))
    }

    fn nurbs_surface(&mut self, surface: &NurbsSurface<monstertruck_modeling::Vector4>) -> Ref {
        let controls = surface
            .control_points()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|p| self.point(p.to_point()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let weights = surface
            .control_points()
            .iter()
            .map(|row| row.iter().map(|p| Real(p.weight())).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        self.entity(format_args!(
            "(BOUNDED_SURFACE()
B_SPLINE_SURFACE({}, {}, {}, .UNSPECIFIED., .U., .U., .U.)
B_SPLINE_SURFACE_WITH_KNOTS({}, .UNSPECIFIED.)
GEOMETRIC_REPRESENTATION_ITEM()
RATIONAL_B_SPLINE_SURFACE({})
REPRESENTATION_ITEM('')
SURFACE())",
            surface.udegree(),
            surface.vdegree(),
            rows(&controls),
            surface_knots(
                surface.knot_vector_u().to_single_multi(),
                surface.knot_vector_v().to_single_multi(),
            ),
            rows(&weights),
        ))
    }
}

fn rows<T: Display>(rows: &[Vec<T>]) -> impl Display + '_ {
    struct Rows<'a, T>(&'a [Vec<T>]);
    impl<T: Display> Display for Rows<'_, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let lists = self.0.iter().map(|row| List(row)).collect::<Vec<_>>();
            List(&lists).fmt(f)
        }
    }
    Rows(rows)
}

fn reals(values: &[f64]) -> Vec<Real> {
    values.iter().copied().map(Real).collect()
}

fn surface_knots((u, um): (Vec<f64>, Vec<usize>), (v, vm): (Vec<f64>, Vec<usize>)) -> String {
    format!(
        "{}, {}, {}, {}",
        List(&um),
        List(&vm),
        List(&reals(&u)),
        List(&reals(&v))
    )
}

#[derive(Clone, Copy)]
struct Real(f64);

impl Display for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.0;
        let real = format!("{value:?}");
        if let Some((mantissa, exponent)) = real.split_once('e') {
            let decimal = if mantissa.contains('.') { "" } else { "." };
            write!(f, "{mantissa}{decimal}E{exponent}")
        } else {
            f.write_str(&real)
        }
    }
}
