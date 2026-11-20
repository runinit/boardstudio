//! Curve behaviour on the STEP curve enums: evaluation, division, cutting,
//! inversion, endpoint snapping, parameter search and transformation.

use super::*;
macro_rules! master_curve {
    ($curve:expr,$method:ident($($argument:expr),* $(,)?)) => {
        match $curve.master_parameter_curve() {
            Some(curve)=>curve.$method($($argument),*),
            None=>$curve.leader().$method($($argument),*),
        }
    };
}

impl ParametricCurve for SurfaceCurve3D {
    type Point = Point3;
    type Vector = Vector3;

    fn evaluate(&self, t: f64) -> Self::Point {
        master_curve!(self, evaluate(t))
    }

    fn derivative(&self, t: f64) -> Self::Vector {
        master_curve!(self, derivative(t))
    }

    fn derivative_2(&self, t: f64) -> Self::Vector {
        master_curve!(self, derivative_2(t))
    }

    fn derivative_n(&self, n: usize, t: f64) -> Self::Vector {
        master_curve!(self, derivative_n(n, t))
    }

    fn parameter_range(&self) -> ParameterRange {
        self.active_trim.map_or_else(
            || master_curve!(self, parameter_range()),
            |(a, b)| (std::ops::Bound::Included(a), std::ops::Bound::Included(b)),
        )
    }

    fn period(&self) -> Option<f64> {
        master_curve!(self, period())
    }
}

impl BoundedCurve for SurfaceCurve3D {}

impl ParameterDivision1D for SurfaceCurve3D {
    type Point = Point3;

    fn try_parameter_division(
        &self,
        range: (f64, f64),
        tol: f64,
    ) -> Option<(Vec<f64>, Vec<Self::Point>)> {
        if ![range.0, range.1].into_iter().all(f64::is_finite)
            || range.0 > range.1
            || self
                .active_trim
                .is_some_and(|(a, b)| range.0 < a || range.1 > b)
        {
            return None;
        }
        master_curve!(self, try_parameter_division(range, tol))
    }

    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        self.try_parameter_division(range, tol)
            .expect("STEP selected-master division exceeds its view")
    }
}

use crate::step::load::step_types::FiniteCurveTrace;

impl SurfaceCurve3D {
    /// Trims the selected master to the vertices. In preview mode, when that fails, the
    /// other representations of the same STEP `surface_curve` (the 3D curve, then each
    /// PCURVE) are tried in turn and the first whose endpoints are within `allowance` of
    /// both vertices is used. The result then names that representation as its master.
    pub(in crate::step::load) fn try_trim_to_points(
        &self,
        front: Point3,
        back: Point3,
        allowance: f64,
        preview: bool,
    ) -> Option<Self> {
        self.try_trim_to_points_with_repair(front, back, allowance, preview, None)
    }

    pub(in crate::step::load) fn try_trim_to_points_with_repair(
        &self,
        front: Point3,
        back: Point3,
        allowance: f64,
        preview: bool,
        mut trace: Option<&mut FiniteCurveTrace>,
    ) -> Option<Self> {
        // Only the unchanged chosen master belongs to this qualified class.
        let mark = trace.as_ref().map(|trace| trace.surface_mark());
        let selected =
            self.try_trim_selected_master(front, back, allowance, preview, trace.as_deref_mut());
        if selected.is_none()
            && let (Some(trace), Some(mark)) = (trace, mark)
        {
            trace.surface_rollback(mark);
        }
        if selected.is_some() || !preview {
            return selected;
        }
        [
            SurfaceCurveRepresentation::Curve3D,
            SurfaceCurveRepresentation::ParameterCurve0,
            SurfaceCurveRepresentation::ParameterCurve1,
        ]
        .into_iter()
        .filter(|&representation| representation != self.master_representation)
        .find_map(|representation| {
            let mut alternate = Self::try_new(
                self.kind,
                self.leader.clone(),
                self.associated_geometry.clone(),
                representation,
            )?;
            alternate.active_trim = self.active_trim;
            alternate.try_trim_selected_master(front, back, allowance, preview, None)
        })
    }

    fn try_trim_selected_master(
        &self,
        front: Point3,
        back: Point3,
        allowance: f64,
        preview: bool,
        trace: Option<&mut FiniteCurveTrace>,
    ) -> Option<Self> {
        let mut trace = trace.filter(|_| self.planar_spline_master() || self.planar_line_master());
        if !allowance.is_finite() || allowance <= 0.0 {
            return None;
        }
        let (a, b) = self.try_range_tuple()?;
        let domain = self.master_scalar_domain().ok()?;
        let matches = |t: f64, p: Point3| {
            let d = self.evaluate(t).distance2(p);
            t.is_finite()
                && domain.is_none_or(|(s, e)| s <= t && t <= e)
                && d.is_finite()
                && d <= allowance * allowance
        };
        // Authored finite endpoints already determine this full source extent.
        if a < b && domain == Some((a, b)) && matches(a, front) && matches(b, back) {
            return Some(self.clone());
        }
        let locate = |p| {
            let exact = self.search_parameter(p, None, 100);
            let t = exact.or_else(|| self.search_nearest_parameter(p, None, 100))?;
            if matches(t, p) {
                Some(t)
            } else if (preview || domain.is_some()) && matches(a, p) {
                Some(a)
            } else if (preview || domain.is_some()) && matches(b, p) {
                Some(b)
            } else {
                None
            }
        };
        let u = locate(front)?;
        let mut v = locate(back)?;
        if front == back {
            if self.regular_native_circle() {
                v = u + self.period()?;
            } else if !preview {
                // Unproved closed native families are refused, never replaced by an alternate.
                return None;
            } else if let Some(period) = self.period().or_else(|| self.preview_scalar_period()) {
                v = u + period;
            } else if let Some((s, e)) =
                domain.filter(|&(s, e)| matches(s, front) && matches(e, back))
            {
                // The finite master itself returns to its start within the allowance.
                return self.install_trim(s, e, trace);
            } else {
                return None;
            }
        } else if u > v {
            match self.period() {
                Some(period) => v += period,
                None if !preview => return None,
                None => match self.preview_scalar_period() {
                    Some(period) => v += period,
                    None => match domain.map(|(_, e)| e).filter(|&e| matches(e, back)) {
                        // A finite master that wraps onto its own start ends at its domain end.
                        Some(e) => v = e,
                        // Otherwise the interval crosses the seam of a geometrically closed
                        // master; a contiguous copy of the master is trimmed instead.
                        None => {
                            let (extended, period) =
                                self.preview_seam_extended(allowance, trace.as_deref_mut())?;
                            return extended.trim_checked(u, v + period, back, allowance, trace);
                        }
                    },
                },
            }
        }
        if v <= u || !matches(v, back) {
            return None;
        }
        self.install_trim(u, v, trace)
    }

    pub(in crate::step::load) fn planar_spline_master(&self) -> bool {
        self.master_parameter_curve().is_some_and(|master| {
            matches!(
                master.curve().as_ref(),
                Curve2D::BsplineCurve(_) | Curve2D::NurbsCurve(_)
            ) && matches!(
                master.surface().as_ref(),
                Surface::ElementarySurface(ElementarySurface::Plane(_))
            )
        })
    }

    /// A selected UV LINE master on a plane: injective, so its trim selection is unique.
    pub(in crate::step::load) fn planar_line_master(&self) -> bool {
        self.master_parameter_curve().is_some_and(|master| {
            matches!(master.curve().as_ref(), Curve2D::Line(_))
                && matches!(
                    master.surface().as_ref(),
                    Surface::ElementarySurface(ElementarySurface::Plane(_))
                )
        })
    }

    fn install_trim(
        &self,
        u: f64,
        v: f64,
        mut trace: Option<&mut FiniteCurveTrace>,
    ) -> Option<Self> {
        // Selection remains Unsupported; this captures the actual finite view
        // immediately before installation, not an inferred earlier source trim.
        let before = trace
            .as_deref_mut()
            .and_then(|trace| trace.surface_begin(self));
        let after = self.clone().try_with_active_trim(Some((u, v)))?;
        if let Some(trace) = trace {
            trace.surface_finish(before, (u, v), &after);
        }
        Some(after)
    }

    /// Trims to `(u, v)` when the point at `v` lies within `allowance` of `back`.
    fn trim_checked(
        self,
        u: f64,
        v: f64,
        back: Point3,
        allowance: f64,
        trace: Option<&mut FiniteCurveTrace>,
    ) -> Option<Self> {
        (v > u && self.evaluate(v).distance(back) <= allowance)
            .then(|| self.install_trim(u, v, trace))
            .flatten()
    }

    /// A seam-crossing interval of a finite, geometrically closed master (its two domain
    /// ends coincide within `allowance`, but it reports no period) cannot be a single
    /// interval of that master. The master spline is joined to a copy of itself with
    /// knots shifted by one domain span, so the domain becomes `(s, e + (e - s))` and
    /// every point is still the authored curve (`C(t)` for `t > e` is `C(t - (e - s))`).
    /// Returns that copy and the span. No control point is moved: the join must be
    /// within the spline concatenation tolerance, otherwise there is no copy.
    fn preview_seam_extended(
        &self,
        allowance: f64,
        mut trace: Option<&mut FiniteCurveTrace>,
    ) -> Option<(Self, f64)> {
        let (s, e) = self.master_scalar_domain().ok()??;
        let span = e - s;
        if !(span.is_finite() && span > 0.0)
            || self.evaluate(s).distance(self.evaluate(e)) > allowance
        {
            return None;
        }
        let before = trace
            .as_deref_mut()
            .and_then(|trace| trace.surface_begin(self));
        let mut shifted_piece = None;
        macro_rules! extend {
            ($curve:expr) => {{
                let mut shifted = $curve.clone();
                shifted.knot_translate(span);
                $curve.try_concat(&shifted).ok()?
            }};
            ($curve:expr, $wrap:expr) => {{
                let mut shifted = $curve.clone();
                shifted.knot_translate(span);
                let joined = $curve.try_concat(&shifted).ok()?;
                if let Some(trace) = trace.as_deref_mut() {
                    shifted_piece = trace.surface_shifted($wrap(shifted));
                }
                joined
            }};
        }
        let mut extended = self.clone();
        match self.master_parameter_curve() {
            Some(master) => {
                let curve = match master.curve().as_ref() {
                    Curve2D::NurbsCurve(c) => Curve2D::NurbsCurve(extend!(c, Curve2D::NurbsCurve)),
                    Curve2D::BsplineCurve(c) => {
                        Curve2D::BsplineCurve(extend!(c, Curve2D::BsplineCurve))
                    }
                    _ => return None,
                };
                let slot = match self.master_representation() {
                    SurfaceCurveRepresentation::ParameterCurve0 => 0,
                    _ => 1,
                };
                extended.associated_geometry[slot] = SurfaceCurveAssociatedGeometry::ParameterCurve(
                    StepParameterCurve::new(Box::new(curve), master.surface().clone()),
                );
            }
            None => {
                *extended.leader = match self.leader() {
                    Curve3D::NurbsCurve(c) => Curve3D::NurbsCurve(extend!(c)),
                    Curve3D::BsplineCurve(c) => Curve3D::BsplineCurve(extend!(c)),
                    _ => return None,
                };
            }
        }
        extended.active_trim = None;
        if let Some(trace) = trace {
            trace.surface_seam(before, (s, e), &extended, shifted_piece);
        }
        Some((extended, span))
    }

    /// Scalar period of a master whose own `period()` is unreported: an unbounded
    /// straight parameter curve running along a periodic axis of its backing surface
    /// repeats its 3D positions after one surface period of travel.
    fn preview_scalar_period(&self) -> Option<f64> {
        let master = self.master_parameter_curve()?;
        let Curve2D::Line(line) = master.curve().as_ref() else {
            return None;
        };
        let (dx, dy) = ((line.1.x - line.0.x).abs(), (line.1.y - line.0.y).abs());
        let surface = master.surface();
        let axis = |travel: f64, other: f64, period: Option<f64>| {
            (travel > 0.0 && other <= 1.0e-9 * travel)
                .then_some(period)
                .flatten()
                .map(|period| period / travel)
                .filter(|t| t.is_finite() && *t > 0.0)
        };
        axis(dx, dy, surface.period_u()).or_else(|| axis(dy, dx, surface.period_v()))
    }

    fn regular_native_circle(&self) -> bool {
        let placed = |m: Matrix4| {
            AffinePlaneCertificate::from_points(
                Point3::origin(),
                Point3::new(1., 0., 0.),
                Point3::new(0., 1., 0.),
            )
            .and_then(|p| p.transformed(*m.as_ref()))
            .is_some()
        };
        match self.master_parameter_curve() {
            None => {
                matches!(self.leader(), Curve3D::Conic(Conic3D::Ellipse(c)) if placed(*c.transform()))
            }
            Some(c) => {
                let Surface::ElementarySurface(ElementarySurface::Plane(p)) = c.surface().as_ref()
                else {
                    return false;
                };
                let Curve2D::Conic(Conic2D::Ellipse(c)) = c.curve().as_ref() else {
                    return false;
                };
                let m = c.transform();
                p.affine_plane_certificate().is_some()
                    && m.x.z == 0.0
                    && m.y.z == 0.0
                    && m.z.z == 1.0
                    && placed(Matrix4::from_cols(
                        m.x.truncate().extend(0.).extend(0.),
                        m.y.truncate().extend(0.).extend(0.),
                        Vector4::new(0., 0., 1., 0.),
                        m.z.truncate().extend(0.).extend(1.),
                    ))
            }
        }
    }
}

impl Cut for SurfaceCurve3D {
    fn cut(&mut self, t: f64) -> Self {
        let (a, b) = self.range_tuple();
        assert!(
            t.is_finite() && a <= t && t <= b,
            "STEP selected-master cut exceeds its view"
        );
        let right = self
            .clone()
            .try_with_active_trim(Some((t, b)))
            .expect("invalid STEP selected-master cut");
        self.active_trim = Some((a, t));
        right
    }
}

impl SnapCurveEndpoints for SurfaceCurve3D {
    fn snap_endpoints(&mut self, _front: Point3, _back: Point3) {
        // Attachment certificates do not authorize rewriting authored geometry.
    }
}

impl SnapCurveEndpoints for Curve3D {
    fn snap_endpoints(&mut self, front: Point3, back: Point3) {
        match self {
            Curve3D::Polyline(curve) => curve.snap_endpoints(front, back),
            Curve3D::SurfaceCurve(curve) => curve.snap_endpoints(front, back),
            Curve3D::IntersectionCurve(curve) => curve.snap_endpoints(front, back),
            Curve3D::Line(_)
            | Curve3D::Conic(_)
            | Curve3D::BsplineCurve(_)
            | Curve3D::ParameterCurve(_)
            | Curve3D::NurbsCurve(_) => {}
        }
    }
}

impl Invertible for SurfaceCurveAssociatedGeometry {
    fn invert(&mut self) {
        if let SurfaceCurveAssociatedGeometry::ParameterCurve(curve) = self {
            curve.invert();
        }
    }
}

impl Invertible for SurfaceCurve3D {
    fn invert(&mut self) {
        let full = self.master_reversal_range();
        let reflect = |t| {
            if t == full.0 {
                full.1
            } else if t == full.1 {
                full.0
            } else if t - full.0 <= full.1 - t {
                full.1 - (t - full.0)
            } else {
                full.0 + (full.1 - t)
            }
        };
        self.active_trim = self.active_trim.map(|(a, b)| (reflect(b), reflect(a)));
        self.leader_mut().invert();
        self.associated_geometry
            .iter_mut()
            .for_each(Invertible::invert);
    }
}

impl SearchParameter<CurveParameter> for SurfaceCurve3D {
    type Point = Point3;

    fn search_parameter<H: Into<SearchParameterHint1D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let t = master_curve!(self, search_parameter(point, hint, trials))?;
        self.active_trim
            .is_none_or(|(a, b)| a <= t && t <= b)
            .then_some(t)
    }
}

impl SearchNearestParameter<CurveParameter> for SurfaceCurve3D {
    type Point = Point3;

    fn search_nearest_parameter<H: Into<SearchParameterHint1D>>(
        &self,
        point: Self::Point,
        hint: H,
        trials: usize,
    ) -> Option<f64> {
        let t = master_curve!(self, search_nearest_parameter(point, hint, trials))?;
        if let Some((a, b)) = self.active_trim
            && (t < a || t > b)
        {
            return None;
        }
        Some(t)
    }
}

impl Transformed<Matrix4> for SurfaceCurveAssociatedGeometry {
    fn transform_by(&mut self, trans: Matrix4) {
        match self {
            SurfaceCurveAssociatedGeometry::ParameterCurve(curve) => curve.transform_by(trans),
            SurfaceCurveAssociatedGeometry::Surface(surface) => surface.transform_by(trans),
        }
    }
}

impl Transformed<Matrix4> for SurfaceCurve3D {
    fn transform_by(&mut self, trans: Matrix4) {
        self.leader_mut().transform_by(trans);
        self.associated_geometry
            .iter_mut()
            .for_each(|entry| entry.transform_by(trans));
    }
}

impl ParameterDivision1D for Curve3D {
    type Point = Point3;

    fn try_parameter_division(
        &self,
        range: (f64, f64),
        tol: f64,
    ) -> Option<(Vec<f64>, Vec<Self::Point>)> {
        let debug_profile = env::var("MT_PROFILE_CURVE_DIVISION").is_ok();
        // Only consult the clock when actually profiling -- `Instant::now()`
        // panics on `wasm32-unknown-unknown` ("time not implemented"), so
        // an unconditional call here breaks browser STEP loading.
        let started = debug_profile.then(Instant::now);
        let result = match self {
            Curve3D::Line(curve) => curve.try_parameter_division(range, tol),
            Curve3D::Polyline(curve) => curve.try_parameter_division(range, tol),
            Curve3D::Conic(curve) => curve.try_parameter_division(range, tol),
            Curve3D::BsplineCurve(curve) => curve.try_parameter_division(range, tol),
            Curve3D::ParameterCurve(curve) => curve.try_parameter_division(range, tol),
            Curve3D::SurfaceCurve(curve) => curve.try_parameter_division(range, tol),
            Curve3D::IntersectionCurve(curve) => curve.leader().try_parameter_division(range, tol),
            Curve3D::NurbsCurve(curve) => curve.try_parameter_division(range, tol),
        };
        if let Some(started) = started {
            let kind = match self {
                Curve3D::Line(_) => "Line",
                Curve3D::Polyline(_) => "Polyline",
                Curve3D::Conic(_) => "Conic",
                Curve3D::BsplineCurve(_) => "BsplineCurve",
                Curve3D::ParameterCurve(_) => "StepParameterCurve",
                Curve3D::SurfaceCurve(_) => "SurfaceCurve",
                Curve3D::IntersectionCurve(_) => "IntersectionCurve",
                Curve3D::NurbsCurve(_) => "NurbsCurve",
            };
            eprintln!(
                "trace bool curve_division kind={} points={} tol={} elapsed_ms={:.3}",
                kind,
                result.as_ref().map_or(0, |(_, points)| points.len()),
                tol,
                started.elapsed().as_secs_f64() * 1000.0,
            );
        }
        result
    }

    fn parameter_division(&self, range: (f64, f64), tol: f64) -> (Vec<f64>, Vec<Self::Point>) {
        let debug_profile = env::var("MT_PROFILE_CURVE_DIVISION").is_ok();
        // Same wasm-safety guard as `try_parameter_division` above.
        let started = debug_profile.then(Instant::now);
        let result = match self {
            Curve3D::Line(curve) => curve.parameter_division(range, tol),
            Curve3D::Polyline(curve) => curve.parameter_division(range, tol),
            Curve3D::Conic(curve) => curve.parameter_division(range, tol),
            Curve3D::BsplineCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::ParameterCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::SurfaceCurve(curve) => curve.parameter_division(range, tol),
            Curve3D::IntersectionCurve(curve) => curve.leader().parameter_division(range, tol),
            Curve3D::NurbsCurve(curve) => curve.parameter_division(range, tol),
        };
        if let Some(started) = started {
            let kind = match self {
                Curve3D::Line(_) => "Line",
                Curve3D::Polyline(_) => "Polyline",
                Curve3D::Conic(_) => "Conic",
                Curve3D::BsplineCurve(_) => "BsplineCurve",
                Curve3D::ParameterCurve(_) => "StepParameterCurve",
                Curve3D::SurfaceCurve(_) => "SurfaceCurve",
                Curve3D::IntersectionCurve(_) => "IntersectionCurve",
                Curve3D::NurbsCurve(_) => "NurbsCurve",
            };
            eprintln!(
                "trace bool curve_division kind={} points={} tol={} elapsed_ms={:.3}",
                kind,
                result.1.len(),
                tol,
                started.elapsed().as_secs_f64() * 1000.0,
            );
        }
        result
    }
}

impl ToSameGeometry<Curve3D> for SurfaceCurve3D {
    fn to_same_geometry(&self) -> Curve3D {
        Curve3D::SurfaceCurve(self.clone())
    }
}

impl From<IntersectionCurve<BsplineCurve<Point3>, Surface, Surface>> for Curve3D {
    fn from(ic: IntersectionCurve<BsplineCurve<Point3>, Surface, Surface>) -> Self {
        let (surface0, surface1, leader) = ic.destruct();
        Curve3D::IntersectionCurve(IntersectionCurve::new(
            Box::new(surface0),
            Box::new(surface1),
            Box::new(Curve3D::BsplineCurve(leader)),
        ))
    }
}
