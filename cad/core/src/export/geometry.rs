use super::{List, Ref, Writer};
use crate::layered::brep::{Point3, Vector3};
use cgmath::InnerSpace;
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

    pub(super) fn placement(&mut self, origin: Point3, normal: Vector3, reference: Vector3) -> Ref {
        let origin = self.point(origin);
        let normal = self.direction(normal);
        let reference = self.direction(reference);
        self.entity(format_args!(
            "AXIS2_PLACEMENT_3D('', {origin}, {normal}, {reference})"
        ))
    }

    pub(super) fn circle(
        &mut self,
        center: Point3,
        radius: f64,
        axis_z: f64,
        start: Point3,
    ) -> Ref {
        // EDGE_CURVE vertices trim the positive parameter direction of the circle.
        let placement = self.placement(
            center,
            axis_z * Vector3::unit_z(),
            (start - center).normalize(),
        );
        self.entity(format_args!("CIRCLE('', {placement}, {})", Real(radius)))
    }

    pub(super) fn line(&mut self, start: Point3, end: Point3) -> Ref {
        let delta = end - start;
        let magnitude = delta.magnitude();
        let origin = self.point(start);
        let axis = self.direction(delta / magnitude);
        let vector = self.entity(format_args!("VECTOR('', {axis}, {})", Real(magnitude)));
        self.entity(format_args!("LINE('', {origin}, {vector})"))
    }
}

#[derive(Clone, Copy)]
pub(super) struct Real(pub(super) f64);

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
