//! Source-coordinate bounds, independent of display tessellation.
mod support;
use monstertruck_io::step::load::{
    convert::SourceBoundStepShell,
    report::{ConnectivityIncidence, ScaledConnectivityAttachment},
    step_geometry::{
        Conic3D, Curve2D, Curve3D, ElementarySurface, Surface, SurfaceCurve3D,
        SurfaceCurveAssociatedGeometry, SurfaceCurveRepresentation, SweepSurface,
    },
};
use monstertruck_modeling::{
    BoundedCurve, EuclideanSpace, InnerSpace, Matrix4, ParametricCurve, Plane, Point3, TOLERANCE,
    Transform, Vector4,
};

const ERROR_MM: f64 = 0.001;
type Bounds = [[f64; 3]; 2];

/// Outcome of support validation, recorded in `StepAccuracy`.
#[derive(Default)]
pub(super) struct SupportReport {
    /// A measured attachment discrepancy or a sampled residual exceeded the declared
    /// precision (it passed only under the preview allowance).
    pub relaxed_beyond_declared_precision: bool,
    /// Boundary curves accepted by sampling after exact certification failed.
    pub sampled_checks: usize,
    pub max_sampled_residual_mm: Option<f64>,
}

/// Preserve support mismatch refusal independently of optional tight bounds.
/// Exact certification comes first. When it is unsupported or inconclusive, preview
/// geometry is accepted if sampled residuals stay within the effective allowance.
pub(super) fn validate_support(
    source: &SourceBoundStepShell,
    factor: f64,
    placement: Matrix4,
    owner: &super::StepPrecision,
) -> Result<SupportReport, String> {
    let mut report = SupportReport::default();
    let shell = source.shell();
    let mut normalized = shell.clone();
    super::units::normalize_shell_to_mm(&mut normalized, factor)?;
    let attachments = source
        .connectivity_after_uniform_scale(&normalized, factor)
        .ok_or("source connectivity did not survive authenticated unit normalization")?;
    for attachment in &attachments {
        let accuracy = attachment.source().accuracy();
        let [declaration] = owner.declarations.as_slice() else {
            return Err("source connectivity has no unique owning declaration".into());
        };
        if owner.geometric_dimension != Some(3)
            || accuracy.representation_id() != owner.representation_id
            || accuracy.context_id() != owner.context_id
            || accuracy.measure_id() != declaration.entity_id
            || accuracy.unit_id() != declaration.length_unit_id
            || attachment.declared_bound_mm() != declaration.millimetres
        {
            return Err(
                "source connectivity belongs to a different representation or declaration".into(),
            );
        }
    }
    let scale = placement
        .x
        .truncate()
        .magnitude()
        .max(placement.y.truncate().magnitude())
        .max(placement.z.truncate().magnitude());
    for (face_index, face) in shell.faces.iter().enumerate() {
        for edge in face.boundaries.iter().flatten() {
            let curve = &shell
                .edges
                .get(edge.index)
                .ok_or("missing source support edge")?
                .curve;
            let (allowance, declared) = support_allowance(
                &attachments,
                source.source_face_ids()[face_index],
                face_index,
                edge.index,
                scale,
            )?;
            let exact =
                support::certify_with_allowance(curve, &face.surface, factor, placement, allowance)
                    .and_then(|()| match &face.surface {
                        Surface::ElementarySurface(ElementarySurface::Plane(plane)) => {
                            certify_plane_support_with_allowance(
                                curve,
                                plane,
                                factor * scale,
                                allowance,
                            )
                        }
                        _ => Ok(()),
                    });
            let Err(exact_error) = exact else {
                continue;
            };
            let sampled = match &face.surface {
                Surface::ElementarySurface(ElementarySurface::Plane(plane)) => {
                    sampled_plane_residual_mm(curve, plane, factor * scale)?
                }
                surface => support::sampled_residual_mm(curve, surface, factor, placement)?
                    .ok_or(exact_error.clone())?,
            };
            if !sampled.is_finite() || sampled > allowance {
                return Err(format!(
                    "{exact_error}; sampled support residual {sampled:.6} mm exceeds allowance {allowance:.6} mm"
                ));
            }
            report.sampled_checks += 1;
            report.relaxed_beyond_declared_precision |= sampled > declared;
            report.max_sampled_residual_mm =
                Some(report.max_sampled_residual_mm.map_or(sampled, |m| m.max(sampled)));
        }
    }
    report.relaxed_beyond_declared_precision |= attachments
        .iter()
        .any(|attachment| attachment.discrepancy_mm() > attachment.declared_bound_mm());
    Ok(report)
}

/// Maximum sampled signed distance (mm) of the curve from the plane.
fn sampled_plane_residual_mm(
    curve: &Curve3D,
    plane: &Plane,
    mm_per_source_unit: f64,
) -> Result<f64, String> {
    let normal = plane.normal() * mm_per_source_unit;
    let origin = plane.origin();
    sampled_max(curve, |point| normal.dot(point - origin).abs())
}

/// Preview-only: evaluate a curve at its endpoints and evenly spaced parameters (at
/// least 16 per knot span, 64 per curve) and return the maximum of `residual`.
fn sampled_max(curve: &Curve3D, residual: impl Fn(Point3) -> f64) -> Result<f64, String> {
    let (low, high) = curve.range_tuple();
    if !low.is_finite() || !high.is_finite() || low > high {
        return Err("sampled support check needs a finite curve interval".into());
    }
    let mut breaks = vec![low, high];
    if let Some(knots) = spline_knots(curve) {
        breaks.extend(knots.into_iter().filter(|u| *u > low && *u < high));
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
    }
    let per_span = (64 / (breaks.len() - 1).max(1)).max(16);
    let mut worst = 0f64;
    for pair in breaks.windows(2).chain((breaks.len() == 1).then_some(&breaks[..])) {
        for i in 0..=per_span {
            let t = pair[0] + (pair[pair.len() - 1] - pair[0]) * i as f64 / per_span as f64;
            let point = curve.evaluate(t);
            if ![point.x, point.y, point.z].into_iter().all(f64::is_finite) {
                return Err("sampled support check hit a nonfinite curve point".into());
            }
            worst = worst.max(residual(point));
        }
    }
    Ok(worst)
}

fn spline_knots(curve: &Curve3D) -> Option<Vec<f64>> {
    match curve {
        Curve3D::BsplineCurve(c) => Some(c.knot_vector().to_vec()),
        Curve3D::NurbsCurve(c) => Some(c.knot_vector().to_vec()),
        Curve3D::SurfaceCurve(c)
            if c.active_trim().is_none()
                && c.master_representation() == SurfaceCurveRepresentation::Curve3D =>
        {
            spline_knots(c.leader())
        }
        Curve3D::Polyline(line) => Some((0..line.len()).map(|i| i as f64).collect()),
        _ => None,
    }
}

// The direct endpoint witnesses authenticate the local allowance only. The
// separate analytic/control-hull checks above still cover the whole curve.
fn support_allowance(
    attachments: &[ScaledConnectivityAttachment],
    face_id: u64,
    face_index: usize,
    edge_index: usize,
    placement_scale: f64,
) -> Result<(f64, f64), String> {
    let mut ends = [None::<(f64, f64)>; 2];
    for attachment in attachments {
        let source = attachment.source();
        if source.incidence() != ConnectivityIncidence::EdgeEndpointOnFace
            || source.face_index() != Some(face_index)
            || source.edge_index() != edge_index
        {
            continue;
        }
        if source.face_id() != Some(face_id) {
            return Err("source connectivity face identity mismatch".into());
        }
        let end = usize::from(source.is_back_endpoint());
        let (bound, declared) = (attachment.bound_mm(), attachment.declared_bound_mm());
        ends[end] = Some(ends[end].map_or((bound, declared), |(b, d)| {
            (b.min(bound), d.min(declared))
        }));
    }
    match ends {
        [None, None] => Ok((TOLERANCE, TOLERANCE)),
        [Some(a), Some(b)] => {
            let bound = a.0.min(b.0) * placement_scale;
            let declared = a.1.min(b.1) * placement_scale;
            if !bound.is_finite() || bound <= 0. {
                return Err("invalid placed source support allowance".into());
            }
            Ok((bound, declared))
        }
        _ => Err("source support allowance lacks both owned endpoint witnesses".into()),
    }
}

pub(super) fn shell_bounds(
    source: &SourceBoundStepShell,
    mm_per_unit: f64,
    placement: Matrix4,
) -> Result<Bounds, String> {
    let world = placement * Matrix4::from_scale(mm_per_unit);
    let shell = source.shell();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for face in &shell.faces {
        // Along a straight ruling every world coordinate is monotone or constant.
        // A compact trimmed face therefore attains its extrema on its own boundary.
        if face.boundaries.is_empty() || face.boundaries.iter().any(Vec::is_empty) {
            return Err("exact bounds require complete finite face boundaries".into());
        }
        let mut face_bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for edge in face.boundaries.iter().flatten() {
            let curve = &shell
                .edges
                .get(edge.index)
                .ok_or("exact bounds reference a missing edge")?
                .curve;
            support::certify(curve, &face.surface, mm_per_unit, placement)?;
            if let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &face.surface {
                let placement_scale = placement
                    .x
                    .truncate()
                    .magnitude()
                    .max(placement.y.truncate().magnitude())
                    .max(placement.z.truncate().magnitude());
                certify_plane_support(curve, plane, mm_per_unit * placement_scale)?;
            }
            curve_bounds(curve, world, &mut face_bounds)?;
        }
        if !has_straight_rulings(&face.surface) {
            let outer = surface_support_box(&face.surface, world)
                .ok_or("exact bounds unsupported for a non-ruled STEP face")?;
            // Boundary extrema are certified source witnesses with at most ERROR_MM
            // enclosure slack. Accept a support box only when all six gaps close.
            if !(0..3).all(|axis| {
                face_bounds[0][axis] + ERROR_MM - outer[0][axis] <= 2. * ERROR_MM
                    && outer[1][axis] - face_bounds[1][axis] + ERROR_MM <= 2. * ERROR_MM
            }) {
                return Err("exact bounds unsupported: non-ruled STEP face needs interior-extrema certification".into());
            }
            face_bounds = outer;
        }
        push(&mut bounds, Point3::from(face_bounds[0]))?;
        push(&mut bounds, Point3::from(face_bounds[1]))?;
    }
    if bounds.iter().flatten().any(|v| !v.is_finite()) {
        return Err("exact source bounds are empty or nonfinite".into());
    }
    Ok(bounds)
}

fn certify_plane_support(
    curve: &Curve3D,
    plane: &Plane,
    mm_per_source_unit: f64,
) -> Result<(), String> {
    certify_plane_support_with_allowance(curve, plane, mm_per_source_unit, TOLERANCE)
}

fn certify_plane_support_with_allowance(
    curve: &Curve3D,
    plane: &Plane,
    mm_per_source_unit: f64,
    allowance_mm: f64,
) -> Result<(), String> {
    let normal = plane.normal() * mm_per_source_unit;
    // Project the entire native curve onto signed plane distance. Its source
    // conic/control-hull enclosure, not sampled vertices, certifies incidence.
    let projection = Matrix4::from_cols(
        Vector4::new(0., 0., normal.x, 0.),
        Vector4::new(0., 0., normal.y, 0.),
        Vector4::new(0., 0., normal.z, 0.),
        Vector4::new(0., 0., -normal.dot(plane.origin().to_vec()), 1.),
    );
    let mut distance = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    curve_bounds(curve, projection, &mut distance)?;
    if distance[0][2] < -allowance_mm || distance[1][2] > allowance_mm {
        return Err("source boundary has uncertified plane support incidence".into());
    }
    Ok(())
}

fn has_straight_rulings(surface: &Surface) -> bool {
    match surface {
        Surface::ElementarySurface(
            ElementarySurface::Plane(_)
            | ElementarySurface::CylindricalSurface(_)
            | ElementarySurface::ConicalSurface(_),
        )
        | Surface::SweepSurface(SweepSurface::ExtrusionSurface(_)) => true,
        Surface::BsplineSurface(surface) => {
            let net = surface.control_points();
            if net.is_empty() || net[0].is_empty() {
                return false;
            }
            surface.is_clamped()
                && ((surface.udegree() == 1 && net.len() == 2)
                    || (surface.vdegree() == 1 && net[0].len() == 2))
        }
        Surface::NurbsSurface(surface) => {
            let net = surface.control_points();
            if net.is_empty() || net[0].is_empty() {
                return false;
            }
            surface.non_rationalized().is_clamped()
                && net.iter().flatten().all(|p| p.w > 0. && p.w.is_finite())
                && ((surface.udegree() == 1 && net.len() == 2)
                    || (surface.vdegree() == 1 && net[0].len() == 2))
        }
        _ => false,
    }
}

fn surface_support_box(surface: &Surface, world: Matrix4) -> Option<Bounds> {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    match surface {
        Surface::BsplineSurface(surface) if surface.is_clamped() => {
            for p in surface.control_points().iter().flatten() {
                push(&mut bounds, world.transform_point(*p)).ok()?;
            }
        }
        Surface::NurbsSurface(surface) if surface.non_rationalized().is_clamped() => {
            for p in surface.control_points().iter().flatten() {
                if !p.w.is_finite() || p.w <= 0. {
                    return None;
                }
                let p = world * p;
                push(&mut bounds, Point3::new(p.x / p.w, p.y / p.w, p.z / p.w)).ok()?;
            }
        }
        Surface::ElementarySurface(ElementarySurface::Sphere(surface)) => {
            let matrix = world * surface.transform();
            let sphere = &surface.entity().0;
            let center = matrix.transform_point(sphere.center());
            for axis in 0..3 {
                let radius = sphere.radius()
                    * (matrix.x[axis].powi(2) + matrix.y[axis].powi(2) + matrix.z[axis].powi(2))
                        .sqrt();
                bounds[0][axis] = center[axis] - radius;
                bounds[1][axis] = center[axis] + radius;
            }
        }
        Surface::ElementarySurface(ElementarySurface::ToroidalSurface(surface)) => {
            let matrix = world * surface.transform();
            let torus = surface.entity();
            let center = matrix.transform_point(torus.center());
            for axis in 0..3 {
                let radial = matrix.x[axis].hypot(matrix.y[axis]);
                let radius = torus.large_radius().abs() * radial
                    + torus.small_radius().abs() * radial.hypot(matrix.z[axis]);
                bounds[0][axis] = center[axis] - radius;
                bounds[1][axis] = center[axis] + radius;
            }
        }
        _ => return None,
    }
    bounds
        .iter()
        .flatten()
        .all(|v| v.is_finite())
        .then_some(bounds)
}

fn push(bounds: &mut Bounds, point: Point3) -> Result<(), String> {
    for (axis, value) in [point.x, point.y, point.z].into_iter().enumerate() {
        if !value.is_finite() {
            return Err("nonfinite exact source bound".into());
        }
        bounds[0][axis] = bounds[0][axis].min(value);
        bounds[1][axis] = bounds[1][axis].max(value);
    }
    Ok(())
}

fn curve_bounds(curve: &Curve3D, world: Matrix4, bounds: &mut Bounds) -> Result<(), String> {
    match curve {
        Curve3D::Line(line) => {
            push(bounds, world.transform_point(line.0))?;
            push(bounds, world.transform_point(line.1))
        }
        Curve3D::Polyline(line) => {
            for p in line.iter() {
                push(bounds, world.transform_point(*p))?;
            }
            Ok(())
        }
        Curve3D::Conic(curve) => conic_bounds(curve, world, bounds),
        Curve3D::BsplineCurve(curve) => {
            let controls = curve
                .control_points()
                .iter()
                .map(|p| world * p.to_homogeneous())
                .collect();
            spline_bounds(
                curve.knot_vector().to_vec(),
                controls,
                curve.degree(),
                bounds,
            )
        }
        Curve3D::NurbsCurve(curve) => {
            let controls = curve.control_points().iter().map(|p| world * p).collect();
            spline_bounds(
                curve.knot_vector().to_vec(),
                controls,
                curve.degree(),
                bounds,
            )
        }
        Curve3D::SurfaceCurve(curve) => {
            if curve.active_trim().is_none()
                && curve.master_representation() == SurfaceCurveRepresentation::Curve3D
            {
                return curve_bounds(curve.leader(), world, bounds);
            }
            for point in selected_affine_endpoints(curve)? {
                push(bounds, world.transform_point(point))?;
            }
            Ok(())
        }
        _ => Err("exact bounds unsupported for this STEP trim curve".into()),
    }
}

// A native LINE, or a UV LINE on an affine Plane, is affine in its unchanged
// scalar labels. Its active endpoints therefore enclose every point, including
// after world placement or signed support-distance projection. Select the actual
// ordered association slot; a preceding Surface is not a PCurve ordinal.
fn selected_affine_endpoints(curve: &SurfaceCurve3D) -> Result<[Point3; 2], String> {
    let slot = match curve.master_representation() {
        SurfaceCurveRepresentation::Curve3D => None,
        SurfaceCurveRepresentation::ParameterCurve0 => Some(0),
        SurfaceCurveRepresentation::ParameterCurve1 => Some(1),
    };
    let affine = match slot {
        None => matches!(curve.leader(), Curve3D::Line(_)),
        Some(slot) => match curve.associated_geometry().get(slot) {
            Some(SurfaceCurveAssociatedGeometry::ParameterCurve(parameter_curve)) => {
                matches!(parameter_curve.curve().as_ref(), Curve2D::Line(_))
                    && matches!(
                        parameter_curve.surface().as_ref(),
                        Surface::ElementarySurface(ElementarySurface::Plane(_))
                    )
            }
            _ => false,
        },
    };
    if !affine {
        return Err("exact active-curve bounds unsupported for this selected STEP master".into());
    }
    let (a, b) = curve.range_tuple();
    if !a.is_finite() || !b.is_finite() || a >= b {
        return Err("nonfinite or empty selected STEP active interval".into());
    }
    let points = [curve.subs(a), curve.subs(b)];
    if points
        .iter()
        .any(|p| ![p.x, p.y, p.z].into_iter().all(f64::is_finite))
    {
        return Err("nonfinite selected STEP active endpoint".into());
    }
    Ok(points)
}

fn conic_bounds(curve: &Conic3D, world: Matrix4, bounds: &mut Bounds) -> Result<(), String> {
    let (range, matrix) = match curve {
        Conic3D::Ellipse(c) => (c.entity().range(), world * c.transform()),
        Conic3D::Hyperbola(c) => (c.entity().range(), world * c.transform()),
        Conic3D::Parabola(c) => (c.entity().range(), world * c.transform()),
    };
    let (low, high) = (range.0.min(range.1), range.0.max(range.1));
    if !low.is_finite() || !high.is_finite() {
        return Err("nonfinite exact conic trim".into());
    }
    let mut parameters = vec![low, high];
    for axis in 0..3 {
        let (a, b) = (matrix.x[axis], matrix.y[axis]);
        match curve {
            Conic3D::Ellipse(_) => {
                let phase = b.atan2(a);
                let first =
                    phase + ((low - phase) / std::f64::consts::PI).ceil() * std::f64::consts::PI;
                for t in [first, first + std::f64::consts::PI] {
                    if t >= low && t <= high {
                        parameters.push(t);
                    }
                }
            }
            Conic3D::Hyperbola(_) if a != 0. && (-b / a).abs() < 1. => {
                let t = (-b / a).atanh();
                if t >= low && t <= high {
                    parameters.push(t);
                }
            }
            Conic3D::Parabola(_) if a != 0. => {
                let t = -b / a;
                if t >= low && t <= high {
                    parameters.push(t);
                }
            }
            _ => {}
        }
    }
    for t in parameters {
        let point = match curve {
            Conic3D::Ellipse(_) => Point3::new(t.cos(), t.sin(), 0.),
            Conic3D::Hyperbola(_) => Point3::new(t.cosh(), t.sinh(), 0.),
            Conic3D::Parabola(_) => Point3::new(t * t, 2. * t, 0.),
        };
        push(bounds, matrix.transform_point(point))?;
    }
    Ok(())
}

fn spline_bounds(
    knots: Vec<f64>,
    controls: Vec<Vector4>,
    degree: usize,
    bounds: &mut Bounds,
) -> Result<(), String> {
    spline_beziers(knots, controls, degree, &mut |piece| {
        bezier_bounds(piece, bounds, 0)
    })
}

fn spline_beziers(
    mut knots: Vec<f64>,
    mut controls: Vec<Vector4>,
    degree: usize,
    visit: &mut impl FnMut(Vec<Vector4>) -> Result<(), String>,
) -> Result<(), String> {
    if controls.is_empty()
        || knots.len() != controls.len() + degree + 1
        || knots.iter().any(|v| !v.is_finite())
        || knots.windows(2).any(|w| w[0] > w[1])
        || controls.iter().any(|p| p.w <= 0. || !p.w.is_finite())
    {
        return Err("invalid exact spline bound inputs".into());
    }
    let (low, high) = (knots[0], knots[knots.len() - 1]);
    if low >= high
        || knots.iter().take(degree + 1).any(|u| *u != low)
        || knots.iter().rev().take(degree + 1).any(|u| *u != high)
    {
        return Err("exact spline bounds require a clamped finite source edge".into());
    }
    let mut interior: Vec<_> = knots
        .iter()
        .copied()
        .filter(|u| *u > low && *u < high)
        .collect();
    interior.dedup();
    // Match the IO bounded-trim insertion: exact knot equality and division,
    // since tolerance-snapping the general geometry helper can erase narrow spans.
    for t in interior {
        let mut multiplicity = knots.iter().filter(|u| **u == t).count();
        while multiplicity < degree {
            let index = knots.partition_point(|u| *u <= t) - 1;
            let mut inserted = controls.clone();
            inserted.push(controls[controls.len() - 1]);
            inserted[index - multiplicity + 1..].copy_from_slice(&controls[index - multiplicity..]);
            for i in index - degree + 1..=index - multiplicity {
                let alpha = (t - knots[i]) / (knots[i + degree] - knots[i]);
                inserted[i] = controls[i - 1] + (controls[i] - controls[i - 1]) * alpha;
            }
            knots.insert(index + 1, t);
            controls = inserted;
            multiplicity += 1;
        }
    }
    for span in degree..controls.len() {
        if knots[span] < knots[span + 1] {
            visit(controls[span - degree..=span].to_vec())?;
        }
    }
    Ok(())
}

fn bezier_bounds(controls: Vec<Vector4>, bounds: &mut Bounds, depth: usize) -> Result<(), String> {
    let project = |p: Vector4| Point3::new(p.x / p.w, p.y / p.w, p.z / p.w);
    let mut hull = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for p in &controls {
        push(&mut hull, project(*p))?;
    }
    let mut endpoints = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    push(&mut endpoints, project(controls[0]))?;
    push(&mut endpoints, project(controls[controls.len() - 1]))?;
    if (0..3).all(|axis| {
        endpoints[0][axis] - hull[0][axis] <= ERROR_MM
            && hull[1][axis] - endpoints[1][axis] <= ERROR_MM
    }) {
        push(bounds, Point3::from(hull[0]))?;
        return push(bounds, Point3::from(hull[1]));
    }
    if depth >= 32 {
        return Err("exact spline bounds did not converge".into());
    }
    let mut work = controls;
    let mut left = vec![work[0]];
    let mut right = vec![work[work.len() - 1]];
    for count in (1..work.len()).rev() {
        for i in 0..count {
            work[i] = (work[i] + work[i + 1]) * 0.5;
        }
        left.push(work[0]);
        right.push(work[count - 1]);
    }
    right.reverse();
    bezier_bounds(left, bounds, depth + 1)?;
    bezier_bounds(right, bounds, depth + 1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nonaffine_selected_active_curve_cannot_use_endpoint_bounds_or_an_alternate_leader() {
        use monstertruck_io::step::load::step_geometry::{
            Curve2D, StepParameterCurve, SurfaceCurveKind,
        };
        use monstertruck_modeling::{BsplineCurve, KnotVector, Line, Point2};
        let curve = SurfaceCurve3D::new(
            SurfaceCurveKind::SurfaceCurve,
            Box::new(Curve3D::Line(Line(
                Point3::new(0., 0., 0.),
                Point3::new(2., 0., 0.),
            ))),
            vec![SurfaceCurveAssociatedGeometry::ParameterCurve(
                StepParameterCurve::new(
                    Box::new(Curve2D::BsplineCurve(BsplineCurve::new(
                        KnotVector::from(vec![0., 0., 0., 1., 1., 1.]),
                        vec![
                            Point2::new(0., 0.),
                            Point2::new(1., 8.),
                            Point2::new(2., 0.),
                        ],
                    ))),
                    Box::new(Surface::ElementarySurface(ElementarySurface::Plane(
                        Plane::xy(),
                    ))),
                ),
            )],
            SurfaceCurveRepresentation::ParameterCurve0,
        )
        .try_with_active_trim(Some((0., 1.)))
        .unwrap();
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        assert!(
            curve_bounds(
                &Curve3D::SurfaceCurve(curve),
                Matrix4::from_scale(1.),
                &mut bounds
            )
            .is_err()
        );
    }

    #[test]
    fn selected_plane_line_bounds_use_active_labels_and_ordered_association_slot() {
        use monstertruck_io::step::load::step_geometry::{
            Curve2D, StepParameterCurve, SurfaceCurve3D, SurfaceCurveAssociatedGeometry,
            SurfaceCurveKind,
        };
        use monstertruck_modeling::{Line, Point2};
        let surface = Surface::ElementarySurface(ElementarySurface::Plane(Plane::xy()));
        let curve = SurfaceCurve3D::new(
            SurfaceCurveKind::SurfaceCurve,
            Box::new(Curve3D::Line(Line(
                Point3::new(100., 100., 100.),
                Point3::new(101., 100., 100.),
            ))),
            vec![
                SurfaceCurveAssociatedGeometry::Surface(Box::new(surface.clone())),
                SurfaceCurveAssociatedGeometry::ParameterCurve(StepParameterCurve::new(
                    Box::new(Curve2D::Line(Line(
                        Point2::new(2., 3.),
                        Point2::new(4., 6.),
                    ))),
                    Box::new(surface),
                )),
            ],
            SurfaceCurveRepresentation::ParameterCurve1,
        )
        .try_with_active_trim(Some((2., 10.)))
        .unwrap();
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        curve_bounds(
            &Curve3D::SurfaceCurve(curve),
            Matrix4::from_scale(2.),
            &mut bounds,
        )
        .unwrap();
        assert_eq!(bounds, [[12., 18., 0.], [44., 66., 0.]]);
    }

    use super::*;
    use monstertruck_modeling::{Invertible, Processor, SquareMatrix, TrimmedCurve, UnitCircle};

    #[test]
    fn partial_circle_keeps_only_included_extrema_under_reversal() {
        let arc = Processor::<_, Matrix4>::new(TrimmedCurve::new(
            UnitCircle::<Point3>::new(),
            (
                std::f64::consts::FRAC_PI_4,
                3. * std::f64::consts::FRAC_PI_4,
            ),
        ));
        for arc in [arc, arc.inverse()] {
            let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
            conic_bounds(&Conic3D::Ellipse(arc), Matrix4::identity(), &mut bounds).unwrap();
            let h = 0.5f64.sqrt();
            for (actual, expected) in bounds.into_iter().flatten().zip([-h, h, 0., h, 1., 0.]) {
                assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
            }
        }
    }

    #[test]
    fn quadratic_bezier_interior_peak_and_narrow_spans_have_a_source_gap_bound() {
        for width in [1., 1e-14] {
            // A narrow first knot span rises to y=1, then a second falls to0.
            // Exact insertion must not tolerance-snap away that span.
            let knots = vec![0., 0., 0., width, width, 2., 2., 2.];
            let controls = vec![
                Vector4::new(0., 0., 0., 1.),
                Vector4::new(0.5, 2., 0., 1.),
                Vector4::new(1., 0., 0., 1.),
                Vector4::new(1.5, -2., 0., 1.),
                Vector4::new(2., 0., 0., 1.),
            ];
            let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
            spline_bounds(knots, controls, 2, &mut bounds).unwrap();
            assert!((bounds[0][0] - 0.).abs() < 1e-12 && (bounds[1][0] - 2.).abs() < 1e-12);
            assert!(bounds[0][1] <= -1. && bounds[0][1] >= -1. - ERROR_MM);
            assert!(bounds[1][1] >= 1. && bounds[1][1] <= 1. + ERROR_MM);
        }
    }

    #[test]
    fn exact_knot_insertion_keeps_a_narrow_single_knot_peak() {
        for width in [0.25, 1e-14] {
            let knots = vec![0., 0., 0., width, 1., 1., 1.];
            let controls = vec![
                Vector4::new(0., 0., 0., 1.),
                Vector4::new(1., 2., 0., 1.),
                Vector4::new(2., 0., 0., 1.),
                Vector4::new(3., 0., 0., 1.),
            ];
            let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
            spline_bounds(knots, controls, 2, &mut bounds).unwrap();
            // On the first span y=2t/w*(2-t/w-t), whose peak is 2/(1+w).
            let maximum = 2. / (1. + width);
            assert!(bounds[1][1] >= maximum - 1e-12 && bounds[1][1] <= maximum + ERROR_MM);
            assert!((bounds[0][0] - 0.).abs() < 1e-12 && (bounds[1][0] - 3.).abs() < 1e-12);
        }
    }

    #[test]
    fn plane_membership_encloses_the_whole_curve_not_three_samples() {
        use monstertruck_modeling::{BsplineCurve, KnotVector};
        for width in [1., 1e-14] {
            // The exact finite knot domain is valid even below the general constructor's tolerance.
            let curve = Curve3D::BsplineCurve(BsplineCurve::new_unchecked(
                KnotVector::from(vec![0., 0., 0., 0., width, width, width, width]),
                vec![
                    Point3::new(0., 0., 0.),
                    Point3::new(1., 0., 0.01),
                    Point3::new(2., 0., -0.01),
                    Point3::new(3., 0., 0.),
                ],
            ));
            assert!(certify_plane_support(&curve, &Plane::xy(), 1.).is_err());
        }
    }

    #[test]
    fn nonpositive_rational_weights_cannot_certify_a_curve_hull() {
        for weight in [0., -1., f64::NAN] {
            let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
            assert!(
                spline_bounds(
                    vec![0., 0., 1., 1.],
                    vec![
                        Vector4::new(0., 0., 0., 1.),
                        Vector4::new(1., 1., 0., weight)
                    ],
                    1,
                    &mut bounds
                )
                .is_err()
            );
        }
    }
}
