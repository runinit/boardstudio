//! Requested layouts of the concrete IO witness clones, not allocator/process peaks.
//!
//! Rust 1.98.0 is pinned by rust-toolchain.toml. Its alloc::vec::Vec::clone
//! calls alloc::slice::to_vec_in: both the Clone and Copy branches request
//! with_capacity_in(len), then initialize slots without growing. Box::clone
//! requests one pointee. Every concrete carrier here is Copy, a derived Clone,
//! or a wrapper around those Vec/Box clones (geometry/nurbs and decorators).
//! In particular RevolutionSurface's derived Clone preserves its stored axis;
//! reconstructing it with by_revolution would normalize that axis again.
//! Preflight walks those exact requests; reconciliation walks owned capacities.
use super::*;
use std::cell::Cell;
use std::mem::size_of;

pub(in crate::step::load) fn curve(value: &Curve3D, budget: &Cell<usize>) -> Option<Curve3D> {
    snapshot(value, budget, |value, retained| {
        let mut cost = Cost::new(retained);
        cost.curve(value, 0)?;
        Some(cost.bytes)
    })
}

pub(in crate::step::load) fn surface_curve(
    value: &SurfaceCurve3D,
    budget: &Cell<usize>,
) -> Option<SurfaceCurve3D> {
    snapshot(value, budget, |value, retained| {
        let mut cost = Cost::new(retained);
        cost.node(0)?;
        cost.surface_curve(value, 0)?;
        Some(cost.bytes)
    })
}

pub(in crate::step::load) fn spline<P: Copy>(
    value: &BsplineCurve<P>,
    budget: &Cell<usize>,
) -> Option<BsplineCurve<P>> {
    snapshot(value, budget, |value, retained| {
        let mut cost = Cost::new(retained);
        cost.spline(value)?;
        Some(cost.bytes)
    })
}

pub(in crate::step::load) fn retain_curve2(value: &Curve2D, budget: &Cell<usize>) -> bool {
    let mut cost = Cost::new(true);
    cost.curve2(value, 0).is_some() && reserve(Some(cost.bytes), budget)
}

fn snapshot<T: Clone>(
    value: &T,
    budget: &Cell<usize>,
    storage: impl Fn(&T, bool) -> Option<usize>,
) -> Option<T> {
    let requested = storage(value, false)?;
    if !reserve(Some(requested), budget) {
        return None;
    }
    let copy = value.clone();
    (storage(&copy, true)? <= requested).then_some(copy)
}

pub(in crate::step::load) fn inverse_surface_curve(
    value: &SurfaceCurve3D,
    budget: &Cell<usize>,
) -> Option<SurfaceCurve3D> {
    // Full preflight bounds recursion before the scratch walk or any clone.
    let mut requested = Cost::new(false);
    requested.node(0)?;
    requested.surface_curve(value, 0)?;
    let scratch = surface_inverse_scratch(value)?;
    if !reserve(requested.bytes.checked_add(scratch), budget) {
        return None;
    }
    let mut copy = value.clone();
    let mut retained = Cost::new(true);
    retained.node(0)?;
    retained.surface_curve(&copy, 0)?;
    if retained.bytes > requested.bytes {
        return None;
    }
    copy.invert();
    Some(copy)
}

// KnotVector::invert clones its Vec, then drops it before returning. All other
// concrete inversions are in-place: lines/conics/polylines, spline controls,
// ParameterCurve's UV curve, IntersectionCurve's leader, and SurfaceCurve's
// leader followed by each PCurve (not its backing surface). Thus scratch is
// the maximum sequential knot request, not a sum or a collection growth factor.
// Its ephemeral capacity is not observable here; the pinned requested layout
// is audited, while the inverse carrier's owned capacities are reconciled above.
fn surface_inverse_scratch(value: &SurfaceCurve3D) -> Option<usize> {
    let mut peak = curve_inverse_scratch(value.leader())?;
    for entry in value.associated_geometry() {
        if let SurfaceCurveAssociatedGeometry::ParameterCurve(curve) = entry {
            peak = peak.max(curve2_inverse_scratch(curve.curve())?);
        }
    }
    Some(peak)
}
fn curve_inverse_scratch(value: &Curve3D) -> Option<usize> {
    match value {
        Curve3D::Line(_) | Curve3D::Conic(_) | Curve3D::Polyline(_) => Some(0),
        Curve3D::BsplineCurve(c) => c.knot_vector().len().checked_mul(size_of::<f64>()),
        Curve3D::NurbsCurve(c) => c.knot_vector().len().checked_mul(size_of::<f64>()),
        Curve3D::ParameterCurve(c) => curve2_inverse_scratch(c.curve()),
        Curve3D::SurfaceCurve(c) => surface_inverse_scratch(c),
        Curve3D::IntersectionCurve(c) => curve_inverse_scratch(c.leader()),
    }
}
fn curve2_inverse_scratch(value: &Curve2D) -> Option<usize> {
    match value {
        Curve2D::Line(_) | Curve2D::Conic(_) | Curve2D::Polyline(_) => Some(0),
        Curve2D::BsplineCurve(c) => c.knot_vector().len().checked_mul(size_of::<f64>()),
        Curve2D::NurbsCurve(c) => c.knot_vector().len().checked_mul(size_of::<f64>()),
    }
}

fn reserve(bytes: Option<usize>, budget: &Cell<usize>) -> bool {
    let Some(remaining) = bytes.and_then(|bytes| budget.get().checked_sub(bytes)) else {
        return false;
    };
    budget.set(remaining);
    true
}

struct Cost {
    retained: bool,
    nodes: usize,
    bytes: usize,
}
impl Cost {
    const fn new(retained: bool) -> Self {
        Self {
            retained,
            nodes: 0,
            bytes: 0,
        }
    }
    fn add(&mut self, bytes: usize) -> Option<()> {
        self.bytes = self.bytes.checked_add(bytes)?;
        Some(())
    }
    fn node(&mut self, depth: usize) -> Option<()> {
        self.nodes = self.nodes.checked_add(1)?;
        (depth <= 32 && self.nodes <= 8192).then_some(())
    }
    fn vector<T>(&mut self, values: &Vec<T>) -> Option<()> {
        let count = if self.retained {
            values.capacity()
        } else {
            values.len()
        };
        self.add(count.checked_mul(size_of::<T>())?)
    }
    fn curve(&mut self, curve: &Curve3D, depth: usize) -> Option<()> {
        self.node(depth)?;
        match curve {
            Curve3D::Line(_) | Curve3D::Conic(_) => {}
            Curve3D::Polyline(c) => self.vector(&c.0)?,
            Curve3D::BsplineCurve(c) => self.spline(c)?,
            Curve3D::NurbsCurve(c) => self.spline(c.non_rationalized())?,
            Curve3D::ParameterCurve(c) => self.pcurve(c, depth + 1)?,
            Curve3D::SurfaceCurve(c) => self.surface_curve(c, depth)?,
            Curve3D::IntersectionCurve(c) => {
                self.add(size_of::<Curve3D>() + 2 * size_of::<Surface>())?;
                self.curve(c.leader(), depth + 1)?;
                self.surface(c.surface0(), depth + 1)?;
                self.surface(c.surface1(), depth + 1)?;
            }
        }
        Some(())
    }
    fn curve2(&mut self, curve: &Curve2D, depth: usize) -> Option<()> {
        self.node(depth)?;
        match curve {
            Curve2D::Line(_) | Curve2D::Conic(_) => {}
            Curve2D::Polyline(c) => self.vector(&c.0)?,
            Curve2D::BsplineCurve(c) => self.spline(c)?,
            Curve2D::NurbsCurve(c) => self.spline(c.non_rationalized())?,
        }
        Some(())
    }
    fn pcurve(&mut self, curve: &StepParameterCurve, depth: usize) -> Option<()> {
        self.add(size_of::<Curve2D>() + size_of::<Surface>())?;
        self.curve2(curve.curve(), depth)?;
        self.surface(curve.surface(), depth)
    }
    fn surface_curve(&mut self, curve: &SurfaceCurve3D, depth: usize) -> Option<()> {
        let SurfaceCurve3D {
            kind: _,
            leader,
            associated_geometry,
            master_representation: _,
            active_trim: _,
        } = curve;
        self.add(size_of::<Curve3D>())?;
        self.curve(leader, depth + 1)?;
        self.vector(associated_geometry)?;
        for entry in associated_geometry {
            match entry {
                SurfaceCurveAssociatedGeometry::ParameterCurve(c) => self.pcurve(c, depth + 1)?,
                SurfaceCurveAssociatedGeometry::Surface(s) => {
                    self.add(size_of::<Surface>())?;
                    self.surface(s, depth + 1)?;
                }
            }
        }
        Some(())
    }
    fn surface(&mut self, surface: &Surface, depth: usize) -> Option<()> {
        self.node(depth)?;
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
    fn spline<P>(&mut self, curve: &BsplineCurve<P>) -> Option<()> {
        self.vector(curve.control_points())?;
        self.vector(curve.knot_vector())
    }
    fn net<P>(&mut self, controls: &Vec<Vec<P>>, knots: &(KnotVector, KnotVector)) -> Option<()> {
        if controls.len() > 8192 {
            return None;
        }
        self.vector(controls)?;
        for row in controls {
            self.vector(row)?;
        }
        self.vector(&knots.0)?;
        self.vector(&knots.1)
    }
}
