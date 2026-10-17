//! Classic (0.3.2) boolean pipeline -- the self-contained default backend.
//! BoardStudio modifies material classification to include oriented inner boundaries.
//!
//! This subtree grafts the proven upstream boolean assembly from the published
//! 0.3.2 crate. It is compiled only when the marching SSI is the active backend
//! (default features); a build with an external SSI backend never touches it
//! and runs the current `integrate` pipeline unchanged. The public entry points in
//! `transversal::integrate` cfg-dispatch `and`/`or`/`difference`/
//! `symmetric_difference` here.
//!
//! Graft-boundary adaptations vs. the 0.3.2 source (all documented inline):
//! - The Alternative intersection-curve arm is a raw
//!   `IntersectionCurve<PolylineCurve<Point3>, S, S>` (0.3.2 wrapped it with
//!   parameter polylines the pipeline never consumed).
//! - `altshell_to_shell` converts that arm back to `C` via
//!   `SurfaceCurve::with_boundaries(..).into()` (the conversion the current
//!   `ShapeOpsCurve` bound provides), not `IntersectionCurve::new(..).into()`.
//! - Failures map onto the current `ShapeOpsError` (`EmptyOutputShell` for a
//!   `None` pipeline result, `InvalidOutputShell` for a rejected solid), since
//!   the current error enum has no single `Internal` variant.

mod divide_face;
mod faces_classification;
mod intersection_curve;
mod loops_store;

use super::integrate::{ShapeOpsCurve, ShapeOpsError, ShapeOpsSurface};
use crate::alternative::Alternative;
use monstertruck_geometry::prelude::*;
use monstertruck_meshing::prelude::*;
use monstertruck_topology::*;
use std::iter;

type ClassicResult<T> = std::result::Result<T, ShapeOpsError>;

type AltCurve<C, S> = Alternative<C, IntersectionCurve<PolylineCurve<Point3>, S, S>>;
type AltCurveShell<C, S> = Shell<Point3, AltCurve<C, S>, S>;

fn bspline_from_intersection_points(mut points: Vec<Point3>) -> Option<BsplineCurve<Point3>> {
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite())
    {
        return None;
    }
    points.dedup_by(|lhs, rhs| *lhs == *rhs);
    if points.len() < 2 {
        return None;
    }
    let denominator = (points.len() - 1) as f64;
    let knot_vec = KnotVector::from(
        iter::once(0.0)
            .chain((0..points.len()).map(|index| index as f64 / denominator))
            .chain(iter::once(1.0))
            .collect::<Vec<_>>(),
    );
    Some(BsplineCurve::new(knot_vec, points))
}

/// Convert an Alternative-curve shell back to a target-curve shell, approximating
/// each intersection-curve arm with a polyline B-spline wrapped in a
/// `SurfaceCurve` (uses the `From<SurfaceCurve<BsplineCurve<Point3>, ..>>`
/// conversion the current `ShapeOpsCurve` bound guarantees).
fn altshell_to_shell<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    altshell: &AltCurveShell<C, S>,
) -> Option<Shell<Point3, C, S>> {
    altshell.try_mapped(
        |p| Some(*p),
        |c| match c {
            Alternative::FirstType(c) => Some(c.clone()),
            Alternative::SecondType(ic) => {
                let surface0 = ic.surface0().clone();
                let surface1 = ic.surface1().clone();
                let bspline = bspline_from_intersection_points(ic.leader().as_slice().to_vec())?;
                let boundary0: Option<ParameterCurve<BoundaryCurve2D, S>> = None;
                let boundary1: Option<ParameterCurve<BoundaryCurve2D, S>> = None;
                Some(
                    SurfaceCurve::with_boundaries(
                        surface0, surface1, bspline, boundary0, boundary1,
                    )
                    .into(),
                )
            }
        },
        |s| Some(s.clone()),
    )
}

// Shift the closed mesh before the signed-volume orientation test to avoid
// cancellation when the authored solid is far from the origin.
fn inverted_mesh(poly: &PolygonMesh) -> Option<bool> {
    let origin = poly.positions().first()?.to_vec();
    let mut local = poly.clone();
    for point in local.positions_mut() {
        *point -= origin;
    }
    let volume = local.volume();
    (volume.is_finite() && volume != 0.0).then_some(volume < 0.0)
}

/// Split two oriented shell sets into `[and_shell, or_shell]`: build
/// intersection loops, divide faces, classify each divided face as `and`/`or`,
/// then ray-cast the remaining `unknown` faces against the other shell.
fn process_one_pair_of_shells<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    shell0: &Shell<Point3, C, S>,
    shell1: &Shell<Point3, C, S>,
    tol: f64,
) -> Option<[Shell<Point3, C, S>; 2]> {
    if tol <= 0.0 {
        return None;
    }
    let poly_shell0 = shell0.triangulation(tol);
    let poly_shell1 = shell1.triangulation(tol);
    if poly_shell0
        .iter()
        .chain(&poly_shell1)
        .any(|face| face.surface().is_none())
    {
        return None;
    }
    let poly0 = poly_shell0.to_polygon();
    let poly1 = poly_shell1.to_polygon();
    let inverted0 = inverted_mesh(&poly0)?;
    let inverted1 = inverted_mesh(&poly1)?;
    let altshell0: AltCurveShell<C, S> =
        shell0.mapped(|x| *x, |c| Alternative::FirstType(c.clone()), Clone::clone);
    let altshell1: AltCurveShell<C, S> =
        shell1.mapped(|x| *x, |c| Alternative::FirstType(c.clone()), Clone::clone);
    let loops_store::LoopsStoreQuadruple {
        geom_loops_store0: loops_store0,
        geom_loops_store1: loops_store1,
        ..
    } = loops_store::create_loops_stores(&altshell0, &poly_shell0, &altshell1, &poly_shell1)?;
    let mut cls0 = divide_face::divide_faces(&altshell0, &loops_store0, tol)?;
    cls0.integrate_by_component();
    let mut cls1 = divide_face::divide_faces(&altshell1, &loops_store1, tol)?;
    cls1.integrate_by_component();
    let [mut and0, mut or0, unknown0] = cls0.and_or_unknown();
    unknown0.into_iter().try_for_each(|face| {
        let pt = face.boundaries()[0].vertex_iter().next().unwrap().point();
        let dir = hash::take_one_unit(pt);
        let count = poly1.signed_crossing_faces(pt, dir);
        // Inverted operands include winding zero and exclude winding minus one.
        if count >= isize::from(!inverted1) {
            and0.push(face);
        } else {
            or0.push(face);
        }
        Some(())
    })?;
    let [mut and1, mut or1, unknown1] = cls1.and_or_unknown();
    unknown1.into_iter().try_for_each(|face| {
        let pt = face.boundaries()[0].vertex_iter().next().unwrap().point();
        let dir = hash::take_one_unit(pt);
        let count = poly0.signed_crossing_faces(pt, dir);
        if count >= isize::from(!inverted0) {
            and1.push(face);
        } else {
            or1.push(face);
        }
        Some(())
    })?;
    and0.append(&mut and1);
    or0.append(&mut or1);
    Some([altshell_to_shell(&and0)?, altshell_to_shell(&or0)?])
}

fn finalize<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    operation: &'static str,
    shell: Shell<Point3, C, S>,
) -> ClassicResult<Solid<Point3, C, S>> {
    if shell.is_empty() {
        return Err(ShapeOpsError::EmptySet { operation });
    }
    let boundaries = shell.connected_components();
    Solid::try_new(boundaries)
        .map_err(|source| ShapeOpsError::InvalidOutputShell { operation, source })
}

/// AND operation between two solids (classic backend).
pub(crate) fn and<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ClassicResult<Solid<Point3, C, S>> {
    let operation = "and";
    let shell0 = solid0.boundaries().iter().flatten().cloned().collect();
    let shell1 = solid1.boundaries().iter().flatten().cloned().collect();
    let [and_shell, _] = process_one_pair_of_shells(&shell0, &shell1, tol)
        .ok_or(ShapeOpsError::EmptyOutputShell { operation })?;
    finalize(operation, and_shell)
}

/// OR operation between two solids (classic backend).
pub(crate) fn or<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ClassicResult<Solid<Point3, C, S>> {
    let operation = "or";
    let shell0 = solid0.boundaries().iter().flatten().cloned().collect();
    let shell1 = solid1.boundaries().iter().flatten().cloned().collect();
    let [_, or_shell] = process_one_pair_of_shells(&shell0, &shell1, tol)
        .ok_or(ShapeOpsError::EmptyOutputShell { operation })?;
    finalize(operation, or_shell)
}

/// Difference: the region inside `solid0` but outside `solid1` (classic backend).
pub(crate) fn difference<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ClassicResult<Solid<Point3, C, S>> {
    let mut neg = solid1.clone();
    neg.not();
    and(solid0, &neg, tol)
}

/// Symmetric difference (XOR): the region inside exactly one solid (classic backend).
pub(crate) fn symmetric_difference<C: ShapeOpsCurve<S>, S: ShapeOpsSurface>(
    solid0: &Solid<Point3, C, S>,
    solid1: &Solid<Point3, C, S>,
    tol: f64,
) -> ClassicResult<Solid<Point3, C, S>> {
    let d0 = difference(solid0, solid1, tol)?;
    let d1 = difference(solid1, solid0, tol)?;
    or(&d0, &d1, tol)
}

#[cfg(test)]
mod intersection_conversion_tests {
    use super::*;

    #[test]
    fn preserves_exact_distinct_sub_tolerance_endpoints_in_the_bspline() {
        let p0 = Point3::new(0.0, 0.0, 0.0);
        let p1 = Point3::new(4.115242653313798e-7, 0.0, 0.0);
        let curve = bspline_from_intersection_points(vec![p0, p0, p1, p1])
            .expect("distinct endpoints must survive conversion even below geometry tolerance");
        let (t0, t1) = curve.range_tuple();
        assert_eq!(curve.evaluate(t0), p0);
        assert_eq!(curve.evaluate(t1), p1);
        assert_eq!(curve.evaluate((t0 + t1) / 2.0), p0.midpoint(p1));
    }

    #[test]
    fn rejects_all_identical_points() {
        let p = Point3::new(1.0, 2.0, 3.0);
        assert!(bspline_from_intersection_points(vec![p, p, p]).is_none());
    }

    #[test]
    fn rejects_non_finite_samples() {
        let p0 = Point3::new(0.0, 0.0, 0.0);
        let p1 = Point3::new(f64::NAN, 0.0, 0.0);
        assert!(bspline_from_intersection_points(vec![p0, p1]).is_none());
    }
}
