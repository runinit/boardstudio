//! Conservative owned-clone costs, with bounded recursion before allocation.
use super::*;

pub(super) fn curve(curve: &Curve3D) -> Option<usize> {
    Cost { nodes: 0, bytes: 0 }.curve(curve, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_movement_sink_nested_snapshot_does_not_retain_preview_spare_capacity() {
        let mut rows = Vec::with_capacity(256);
        for x in [0.0, 1.0] {
            let mut row = Vec::with_capacity(256);
            row.extend([Point3::new(x, 0.0, 0.0), Point3::new(x, 1.0, 0.0)]);
            rows.push(row);
        }
        let surface = Surface::BsplineSurface(BsplineSurface::new(
            (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
            rows,
        ));
        let curve = Curve3D::ParameterCurve(StepParameterCurve::new(
            Box::new(Curve2D::BsplineCurve(BsplineCurve::new(
                KnotVector::bezier_knot(1),
                vec![Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)],
            ))),
            Box::new(surface),
        ));
        let budget = std::rc::Rc::new(std::cell::Cell::new(4096));
        let mut trace = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
        let snapshot = trace.begin(&curve).expect(
            "the small actual nested snapshot fits even when the preview net has large spare capacity",
        );
        assert_eq!(snapshot, curve);
        let Curve3D::ParameterCurve(copy) = &snapshot else { unreachable!() };
        let Surface::BsplineSurface(copy) = copy.surface().as_ref() else { unreachable!() };
        assert_eq!(copy.control_points().capacity(), 2);
        assert!(copy.control_points().iter().all(|row| row.capacity() == 2));
        assert!(!trace.failed);
        assert!(budget.get() > 0);
    }
}
pub(super) fn curve2(curve: &Curve2D) -> Option<usize> {
    let mut cost = Cost { nodes: 0, bytes: 0 };
    cost.curve2(curve)?;
    Some(cost.bytes)
}
pub(super) fn surface_curve(curve: &SurfaceCurve3D) -> Option<usize> {
    let mut cost = Cost { nodes: 0, bytes: 0 };
    cost.node::<Curve3D>(0)?;
    cost.surface_curve(curve, 0)?;
    Some(cost.bytes)
}
struct Cost {
    nodes: usize,
    bytes: usize,
}
impl Cost {
    fn add(&mut self, bytes: usize) -> Option<()> {
        self.bytes = self.bytes.checked_add(bytes)?;
        Some(())
    }
    fn node<T>(&mut self, depth: usize) -> Option<()> {
        self.nodes += 1;
        if depth > 32 || self.nodes > 8192 {
            return None;
        }
        self.add(2 * std::mem::size_of::<T>())
    }
    fn curve(&mut self, curve: &Curve3D, depth: usize) -> Option<usize> {
        self.node::<Curve3D>(depth)?;
        match curve {
            Curve3D::Line(_) | Curve3D::Conic(_) => {}
            Curve3D::Polyline(c) => {
                self.add(c.len().checked_mul(2 * std::mem::size_of::<Point3>())?)?
            }
            Curve3D::BsplineCurve(c) => self.add(spline_storage(c)?)?,
            Curve3D::NurbsCurve(c) => self.add(spline_storage(c.non_rationalized())?)?,
            Curve3D::ParameterCurve(c) => self.pcurve(c, depth + 1)?,
            Curve3D::SurfaceCurve(c) => {
                self.surface_curve(c, depth)?;
            }
            Curve3D::IntersectionCurve(c) => {
                self.curve(c.leader(), depth + 1)?;
                self.surface(c.surface0(), depth + 1)?;
                self.surface(c.surface1(), depth + 1)?;
            }
        }
        Some(self.bytes)
    }
    fn surface_curve(&mut self, c: &SurfaceCurve3D, depth: usize) -> Option<()> {
        self.curve(c.leader(), depth + 1)?;
        self.add(
            c.associated_geometry()
                .len()
                .checked_mul(2 * std::mem::size_of::<SurfaceCurveAssociatedGeometry>())?,
        )?;
        for entry in c.associated_geometry() {
            match entry {
                SurfaceCurveAssociatedGeometry::ParameterCurve(c) => self.pcurve(c, depth + 1)?,
                SurfaceCurveAssociatedGeometry::Surface(s) => self.surface(s, depth + 1)?,
            }
        }
        Some(())
    }
    fn curve2(&mut self, curve: &Curve2D) -> Option<()> {
        self.node::<Curve2D>(0)?;
        match curve {
            Curve2D::Line(_) | Curve2D::Conic(_) => {}
            Curve2D::Polyline(c) => {
                self.add(c.len().checked_mul(2 * std::mem::size_of::<Point2>())?)?
            }
            Curve2D::BsplineCurve(c) => self.add(spline_storage(c)?)?,
            Curve2D::NurbsCurve(c) => self.add(spline_storage(c.non_rationalized())?)?,
        }
        Some(())
    }
    fn pcurve(&mut self, c: &StepParameterCurve, depth: usize) -> Option<()> {
        self.curve2(c.curve())?;
        self.surface(c.surface(), depth)
    }
    fn surface(&mut self, surface: &Surface, depth: usize) -> Option<()> {
        self.node::<Surface>(depth)?;
        match surface {
            Surface::ElementarySurface(_) => {}
            Surface::SweepSurface(SweepSurface::ExtrusionSurface(s)) => {
                self.curve(s.entity_curve(), depth + 1)?;
            }
            Surface::SweepSurface(SweepSurface::RevolutionSurface(s)) => {
                self.curve(s.entity().entity_curve(), depth + 1)?;
            }
            Surface::BsplineSurface(s) => self.net(s.control_points(), s.knot_vectors())?,
            Surface::NurbsSurface(s) => self.net(s.control_points(), s.knot_vectors())?,
        }
        Some(())
    }
    fn net<P>(&mut self, controls: &Vec<Vec<P>>, knots: &(KnotVector, KnotVector)) -> Option<()> {
        if controls.len() > 8192 {
            return None;
        }
        self.add(
            controls
                .capacity()
                .checked_mul(2 * std::mem::size_of::<Vec<P>>())?,
        )?;
        for row in controls {
            self.add(row.capacity().checked_mul(2 * std::mem::size_of::<P>())?)?;
        }
        self.add(
            knots
                .0
                .len()
                .checked_add(knots.1.len())?
                .checked_mul(2 * std::mem::size_of::<f64>())?,
        )
    }
}
