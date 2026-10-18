//! Conservative owned-clone costs, with bounded recursion before allocation.
use super::*;

pub(super) fn curve(curve: &Curve3D) -> Option<usize> {
    Cost { nodes: 0, bytes: 0 }.curve(curve, 0)
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
struct Cost { nodes: usize, bytes: usize }
impl Cost {
    fn add(&mut self, bytes: usize) -> Option<()> {
        self.bytes = self.bytes.checked_add(bytes)?;
        Some(())
    }
    fn node<T>(&mut self, depth: usize) -> Option<()> {
        self.nodes += 1;
        if depth > 32 || self.nodes > 8192 { return None; }
        self.add(2 * std::mem::size_of::<T>())
    }
    fn curve(&mut self, curve: &Curve3D, depth: usize) -> Option<usize> {
        self.node::<Curve3D>(depth)?;
        match curve {
            Curve3D::Line(_) | Curve3D::Conic(_) => {},
            Curve3D::Polyline(c) => self.add(c.len().checked_mul(2 * std::mem::size_of::<Point3>())?)?,
            Curve3D::BsplineCurve(c) => self.add(spline_storage(c)?)?,
            Curve3D::NurbsCurve(c) => self.add(spline_storage(c.non_rationalized())?)?,
            Curve3D::ParameterCurve(c) => self.pcurve(c, depth + 1)?,
            Curve3D::SurfaceCurve(c) => {
                self.surface_curve(c, depth)?;
            },
            Curve3D::IntersectionCurve(c) => {
                self.curve(c.leader(), depth + 1)?;
                self.surface(c.surface0(), depth + 1)?;
                self.surface(c.surface1(), depth + 1)?;
            },
        }
        Some(self.bytes)
    }
    fn surface_curve(&mut self, c: &SurfaceCurve3D, depth: usize) -> Option<()> {
        self.curve(c.leader(), depth + 1)?;
        self.add(c.associated_geometry().len().checked_mul(2 * std::mem::size_of::<SurfaceCurveAssociatedGeometry>())?)?;
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
            Curve2D::Line(_) | Curve2D::Conic(_) => {},
            Curve2D::Polyline(c) => self.add(c.len().checked_mul(2 * std::mem::size_of::<Point2>())?)?,
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
            Surface::ElementarySurface(_) => {},
            Surface::SweepSurface(SweepSurface::ExtrusionSurface(s)) => { self.curve(s.entity_curve(), depth + 1)?; },
            Surface::SweepSurface(SweepSurface::RevolutionSurface(s)) => { self.curve(s.entity().entity_curve(), depth + 1)?; },
            Surface::BsplineSurface(s) => self.net(s.control_points(), s.knot_vectors())?,
            Surface::NurbsSurface(s) => self.net(s.control_points(), s.knot_vectors())?,
        }
        Some(())
    }
    fn net<P>(&mut self, controls: &Vec<Vec<P>>, knots: &(KnotVector, KnotVector)) -> Option<()> {
        self.add(controls.capacity().checked_mul(2 * std::mem::size_of::<Vec<P>>())?)?;
        for row in controls {
            self.add(row.capacity().checked_mul(2 * std::mem::size_of::<P>())?)?;
        }
        self.add(knots.0.len().checked_add(knots.1.len())?.checked_mul(2 * std::mem::size_of::<f64>())?)
    }
}
