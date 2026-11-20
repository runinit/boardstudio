use super::*;
use monstertruck_core::cgmath_extend_traits::control_point::ControlPoint;

pub(super) fn trim_edge_curve_recorded(
    curve: Curve3D,
    p: Point3,
    q: Point3,
    accuracy: f64,
    repair: Option<&mut FiniteCurveTrace>,
) -> Result<Curve3D, StepConvertingError> {
    trim_edge_curve_impl(curve, p, q, accuracy, None, repair)
}

pub(super) fn trim_edge_curve_in_native_frame(
    source: &BoundedCurveAny,
    p: Point3,
    q: Point3,
    accuracy: f64,
    same_sense: bool,
) -> Result<Curve3D, StepConvertingError> {
    trim_edge_curve_impl(
        source.try_into()?,
        p,
        q,
        accuracy,
        Some((source, same_sense)),
        None,
    )
}

pub(super) fn trim_edge_curve_in_native_frame_recorded(
    source: &BoundedCurveAny,
    p: Point3,
    q: Point3,
    accuracy: f64,
    same_sense: bool,
    repair: &mut FiniteCurveTrace,
) -> Result<Curve3D, StepConvertingError> {
    trim_edge_curve_impl(
        source.try_into()?,
        p,
        q,
        accuracy,
        Some((source, same_sense)),
        Some(repair),
    )
}

fn trim_edge_curve_impl(
    curve: Curve3D,
    p: Point3,
    q: Point3,
    accuracy: f64,
    native_frame: Option<(&BoundedCurveAny, bool)>,
    repair: Option<&mut FiniteCurveTrace>,
) -> Result<Curve3D, StepConvertingError> {
    if !accuracy.is_finite()
        || accuracy <= 0.0
        || !accuracy.powi(2).is_finite()
        || accuracy.powi(2) == 0.0
    {
        return Err("invalid source connectivity accuracy".into());
    }
    let (knots, degree, controls) = match &curve {
        Curve3D::BsplineCurve(spline) => (
            spline.knot_vector(),
            spline.degree(),
            spline.control_points().clone(),
        ),
        Curve3D::NurbsCurve(spline) => {
            if spline
                .control_points()
                .iter()
                .any(|point| !point.w.is_finite() || point.w <= 0.0)
            {
                return Err("B-spline edge weights must be finite and positive".into());
            }
            (
                spline.knot_vector(),
                spline.degree(),
                spline
                    .control_points()
                    .iter()
                    .map(|point| {
                        Point3::new(point.x / point.w, point.y / point.w, point.z / point.w)
                    })
                    .collect(),
            )
        }
        _ => return Ok(curve),
    };
    if controls
        .iter()
        .chain([&p, &q])
        .any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite())
    {
        return Err("B-spline edge controls and endpoints must be finite".into());
    }
    let (a, b) = curve.range_tuple();
    if degree == 0
        || degree >= controls.len()
        || !a.is_finite()
        || !b.is_finite()
        || !(b - a).is_finite()
        || a >= b
        || knots.iter().any(|knot| !knot.is_finite())
        || knots.windows(2).any(|pair| pair[0] > pair[1])
        || knots
            .windows(degree + 2)
            .any(|run| run[0] == run[degree + 1])
    {
        return Err("invalid B-spline edge knot domain".into());
    }
    let matches = |t, point| curve.subs(t).distance2(point) <= accuracy * accuracy;
    // The source domain already identifies the intended full interval, including closed curves.
    if matches(a, p) && matches(b, q) {
        if let Some(repair) = repair {
            let before = repair.begin(&curve);
            repair.finish("native-full-interval-v1", before, (a, b), &curve, (a, b));
        }
        return Ok(curve);
    }
    if knots[..=degree].iter().any(|knot| *knot != a)
        || knots[controls.len()..].iter().any(|knot| *knot != b)
        || knots[degree + 1..controls.len()]
            .windows(degree + 1)
            .any(|run| run[0] == run[degree])
    {
        return Err("partial B-spline edges require a continuous clamped knot domain".into());
    }
    let start = curve.subs(a);
    let end = curve.subs(b);
    // A strictly ordered control projection is injective for positive-weight B-splines.
    let direction = [
        end - start,
        Vector3::unit_x(),
        Vector3::unit_y(),
        Vector3::unit_z(),
    ]
    .into_iter()
    .find(|direction| {
        direction.magnitude2().is_finite()
            && controls.windows(2).all(|pair| {
                let advance = (pair[1] - pair[0]).dot(*direction);
                advance.is_finite() && advance > 0.0
            })
    });
    let join_interval = direction.is_none();
    let (u, v) = if let Some(direction) = direction {
        let inverse = |point: Point3| -> Result<f64, StepConvertingError> {
            if matches(a, point) {
                return Ok(a);
            }
            if matches(b, point) {
                return Ok(b);
            }
            let coordinate = (point - start).dot(direction);
            if !coordinate.is_finite()
                || coordinate <= 0.0
                || coordinate >= (end - start).dot(direction)
            {
                return Err("B-spline edge vertex is outside its bounded curve".into());
            }
            let (mut lo, mut hi) = (a, b);
            for _ in 0..80 {
                let mid = lo + (hi - lo) * 0.5;
                if mid == lo || mid == hi {
                    break;
                }
                if (curve.subs(mid) - start).dot(direction) < coordinate {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let parameter = if curve.subs(lo).distance2(point) <= curve.subs(hi).distance2(point) {
                lo
            } else {
                hi
            };
            if !matches(parameter, point) {
                return Err("B-spline edge vertex is not on its bounded curve".into());
            }
            Ok(parameter)
        };
        (inverse(p)?, inverse(q)?)
    } else {
        // Only original polynomial coefficients support the exact Bezier join proof.
        // Relabeled narrow domains and rational controls retain the existing refusal.
        match (&curve, native_frame) {
            (Curve3D::BsplineCurve(_), Some((source, _))) => {
                if !original_source_metadata_matches(source, knots, degree, &controls) {
                    return Err("partial B-spline join interval requires consistent original source metadata".into());
                }
                bezier_join_interval(knots, degree, &controls, p, q, accuracy)
            }
            _ => None,
        }
        .ok_or("partial B-spline edge has no certified unique endpoint inverse")?
    };
    if u >= v {
        return Err("B-spline edge vertices contradict the geometry sense".into());
    }
    // Every actual mutation retains its immediate finite-domain pair. Selection
    // authority is separate: these recorded inverse results are not a proof.
    let trimmed = match &curve {
        Curve3D::BsplineCurve(spline) => Curve3D::BsplineCurve(trim_spline(
            spline,
            (a, b),
            (u, v),
            repair,
            Curve3D::BsplineCurve,
        )?),
        Curve3D::NurbsCurve(spline) => Curve3D::NurbsCurve(NurbsCurve::new(trim_spline(
            spline.non_rationalized(),
            (a, b),
            (u, v),
            repair,
            |spline| Curve3D::NurbsCurve(NurbsCurve::new(spline)),
        )?)),
        _ => unreachable!(),
    };
    if [trimmed.front().distance2(p), trimmed.back().distance2(q)]
        .iter()
        .any(|distance| !distance.is_finite() || *distance > accuracy * accuracy)
    {
        return Err("trimmed B-spline endpoints do not match EDGE_CURVE vertices".into());
    }
    // Only the additional join path promises original native endpoint labels.
    // Preflight the SAME caller inversion; preserve all legacy full/monotone paths.
    if join_interval
        && native_frame.is_some_and(|(_, same_sense)| !same_sense)
        && trimmed.inverse().range_tuple() != (u, v)
    {
        return Err(
            "partial B-spline EDGE_CURVE inversion cannot preserve native endpoint labels".into(),
        );
    }
    Ok(trimmed)
}

// Conversion can infer degree or zip unequal lists; new source-join admission
// must authenticate the written basis (new_join_admission_requires_consistent_original_source_metadata).
fn original_source_metadata_matches(
    source: &BoundedCurveAny,
    knots: &KnotVector,
    degree: usize,
    controls: &[Point3],
) -> bool {
    let BoundedCurveAny::BsplineCurve(source) = source else {
        return false;
    };
    let BsplineCurveAny::NonRationalBsplineCurve(source) = source.as_ref() else {
        return false;
    };
    let (written_degree, written_controls) = match source.as_ref() {
        NonRationalBsplineCurve::BsplineCurveWithKnots(source) => {
            if source.knots.len() < 2
                || source.knots.len() != source.knot_multiplicities.len()
                || source.knots.iter().any(|knot| !knot.is_finite())
                || source.knots.windows(2).any(|pair| pair[0] >= pair[1])
                || checked_knot_multiplicity_total(&source.knot_multiplicities).ok()
                    != Some(knots.len())
            {
                return false;
            }
            let mut actual = knots.iter();
            for (knot, multiplicity) in source.knots.iter().zip(&source.knot_multiplicities) {
                let Ok(multiplicity) = usize::try_from(*multiplicity) else {
                    return false;
                };
                if (0..multiplicity).any(|_| actual.next() != Some(knot)) {
                    return false;
                }
            }
            (source.degree, &source.control_points_list)
        }
        NonRationalBsplineCurve::BezierCurve(source) => {
            (source.degree, &source.control_points_list)
        }
        NonRationalBsplineCurve::QuasiUniformCurve(source) => {
            (source.degree, &source.control_points_list)
        }
        NonRationalBsplineCurve::UniformCurve(source) => {
            (source.degree, &source.control_points_list)
        }
    };
    usize::try_from(written_degree).ok() == Some(degree)
        && written_controls.len() == controls.len()
        && written_controls
            .iter()
            .flat_map(|point| &point.coordinates)
            .all(|coordinate| coordinate.is_finite())
}

// Source join anchors can have several finite preimages (solid32377-source.step).
// A supporting coordinate excludes every open Bezier span; retain ALL join aliases
// until geometry order proves exactly one interval, without extending periodically.
fn bezier_join_interval(
    knots: &KnotVector,
    degree: usize,
    controls: &[Point3],
    p: Point3,
    q: Point3,
    accuracy: f64,
) -> Option<(f64, f64)> {
    if !(controls.len() - 1).is_multiple_of(degree) {
        return None;
    }
    let spans = (controls.len() - 1) / degree;
    let mut joins = vec![knots[degree]];
    for run in knots[degree + 1..controls.len()].chunks(degree) {
        if run.len() != degree
            || run.iter().any(|knot| *knot != run[0])
            || run[0] <= *joins.last()?
        {
            return None;
        }
        joins.push(run[0]);
    }
    joins.push(knots[controls.len()]);
    if joins.len() != spans + 1 || joins[spans] <= joins[spans - 1] {
        return None;
    }
    let extrema: [(f64, f64); 3] = std::array::from_fn(|axis| {
        controls
            .iter()
            .fold((controls[0][axis], controls[0][axis]), |(lo, hi), c| {
                (lo.min(c[axis]), hi.max(c[axis]))
            })
    });
    let supports: [[bool; 2]; 3] = std::array::from_fn(|axis| {
        [extrema[axis].0, extrema[axis].1].map(|value| {
            (0..spans).all(|span| {
                controls[span * degree..=span * degree + degree]
                    .iter()
                    .any(|c| c[axis] != value)
            })
        })
    });
    let preimages = |point: Point3| -> Option<Vec<f64>> {
        let mut anchor = None;
        for candidate in controls.iter().step_by(degree) {
            if !((0..3).all(|axis| {
                // Coordinatewise signs prove (P-A).(C-A) <= 0 without rounded dots.
                (point[axis] <= candidate[axis] || extrema[axis].1 <= candidate[axis])
                    && (point[axis] >= candidate[axis] || extrema[axis].0 >= candidate[axis])
            }) && (0..3).any(|axis| {
                (candidate[axis] == extrema[axis].0 && supports[axis][0])
                    || (candidate[axis] == extrema[axis].1 && supports[axis][1])
            }) && candidate.distance2(point) <= accuracy * accuracy)
            {
                continue;
            }
            if anchor.is_some_and(|anchor| anchor != *candidate) {
                return None;
            }
            anchor = Some(*candidate);
        }
        let anchor = anchor?;
        Some(
            controls
                .iter()
                .step_by(degree)
                .zip(&joins)
                .filter_map(|(c, t)| (*c == anchor).then_some(*t))
                .collect(),
        )
    };
    let (starts, ends) = (preimages(p)?, preimages(q)?);
    let (mut result, mut first_end) = (None, 0);
    for u in starts {
        while first_end < ends.len() && ends[first_end] <= u {
            first_end += 1;
        }
        if first_end == ends.len() {
            break;
        }
        if result.is_some() || ends.len() - first_end != 1 {
            return None;
        }
        result = Some((u, ends[first_end]));
    }
    // Existing restriction normalizes shorter intervals. This proof promises native labels.
    result.filter(|(u, v)| v - u > TOLERANCE)
}

fn trim_spline<P: ControlPoint<f64> + Tolerance>(
    spline: &BsplineCurve<P>,
    (a, b): (f64, f64),
    (u, v): (f64, f64),
    mut trace: Option<&mut FiniteCurveTrace>,
    wrap: impl Fn(BsplineCurve<P>) -> Curve3D,
) -> Result<BsplineCurve<P>, StepConvertingError> {
    let mut trimmed = spline.clone();
    // Charge owned capacities before retaining a carrier clone.
    let mut pending = begin_spline_change(&mut trace, &wrap, &trimmed);
    if u > a {
        trimmed = cut_exact(&mut trimmed, u);
        finish_spline_change(
            &mut trace,
            &wrap,
            "native-cut-front-v1",
            pending.take(),
            (u, v),
            &trimmed,
            (u, v),
        );
        pending = begin_spline_change(&mut trace, &wrap, &trimmed);
    }
    if v < b {
        cut_exact(&mut trimmed, v);
        finish_spline_change(
            &mut trace,
            &wrap,
            "native-cut-back-v1",
            pending.take(),
            (u, v),
            &trimmed,
            (u, v),
        );
        pending = begin_spline_change(&mut trace, &wrap, &trimmed);
    }
    if trimmed.knot_vector().range_length() <= TOLERANCE {
        let (knots, controls) = trimmed.destruct();
        let (first, span) = (knots[0], knots.range_length());
        let knots = knots
            .iter()
            .map(|knot| (knot - first) / span)
            .collect::<Vec<_>>();
        trimmed = BsplineCurve::try_new(KnotVector::from(knots), controls)?;
        finish_spline_change(
            &mut trace,
            &wrap,
            "native-domain-relabel-v1",
            pending.take(),
            (u, v),
            &trimmed,
            (0.0, 1.0),
        );
    }
    if trimmed
        .control_points()
        .iter()
        .any(|point| (0..P::DIM).any(|axis| !point[axis].is_finite()))
    {
        return Err("B-spline edge trim produced non-finite controls".into());
    }
    Ok(trimmed)
}

fn begin_spline_change<P>(
    trace: &mut Option<&mut FiniteCurveTrace>,
    wrap: &impl Fn(BsplineCurve<P>) -> Curve3D,
    spline: &BsplineCurve<P>,
) -> Option<Curve3D>
where
    P: Copy,
{
    trace
        .as_deref_mut()
        .and_then(|trace| trace.begin_spline(spline).map(wrap))
}
fn finish_spline_change<P>(
    trace: &mut Option<&mut FiniteCurveTrace>,
    wrap: &impl Fn(BsplineCurve<P>) -> Curve3D,
    stage: &'static str,
    before: Option<Curve3D>,
    before_domain: (f64, f64),
    after: &BsplineCurve<P>,
    after_domain: (f64, f64),
) where
    P: Copy,
{
    let Some(trace) = trace.as_deref_mut() else {
        return;
    };
    let Some(before) = before else {
        return;
    };
    let Some(after) = trace.begin_spline(after) else {
        return;
    };
    trace.push(FiniteCurveRepair {
        stage,
        before,
        before_domain,
        after: wrap(after),
        after_domain,
        native_line: None,
        raw_spline: None,
        seam: None,
    });
}

// Generic Cut snaps spatial TOLERANCE onto knot parameters; STEP interval cuts must be exact.
fn cut_exact<P: ControlPoint<f64>>(spline: &mut BsplineCurve<P>, t: f64) -> BsplineCurve<P> {
    let degree = spline.degree();
    let (source_knots, mut controls) = spline.clone().destruct();
    let mut knots = source_knots.to_vec();
    let mut index = knots.partition_point(|knot| *knot <= t) - 1;
    let mut multiplicity = knots.iter().filter(|knot| **knot == t).count();
    while multiplicity <= degree {
        let mut inserted = controls.clone();
        inserted.push(controls[controls.len() - 1]);
        for i in index - multiplicity..controls.len() {
            inserted[i + 1] = controls[i];
        }
        for i in index - degree + 1..=index - multiplicity {
            let alpha = (t - knots[i]) / (knots[i + degree] - knots[i]);
            inserted[i] = controls[i - 1] + (controls[i] - controls[i - 1]) * alpha;
        }
        knots.insert(index + 1, t);
        controls = inserted;
        index += 1;
        multiplicity += 1;
    }
    let split = index - degree;
    // Exact knot insertion preserves the validated domain and control-count invariants.
    *spline = BsplineCurve::new_unchecked(
        KnotVector::from(knots[..=index].to_vec()),
        controls[..split].to_vec(),
    );
    BsplineCurve::new_unchecked(
        KnotVector::from(knots[split..].to_vec()),
        controls[split..].to_vec(),
    )
}
