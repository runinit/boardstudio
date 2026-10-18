//! Topology entities: vertices, edges, loops, faces, shells and solids.
//!
//! Also carries the `*Holder` inherent impls that resolve a topological
//! entity's references against the entity table while loading.

use super::*;

mod bounded_trim;
mod repair_storage;
mod seam_capture;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::step::load) struct FiniteCurveRepair {
    pub(in crate::step::load) stage: &'static str,
    pub(in crate::step::load) before: Curve3D,
    pub(in crate::step::load) before_domain: (f64, f64),
    pub(in crate::step::load) after: Curve3D,
    pub(in crate::step::load) after_domain: (f64, f64),
    pub(in crate::step::load) native_line: Option<NativeLineProjection>,
    seam: Option<seam_capture::SeamCopy>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::step::load) struct NativeLineProjection {
    point: [f64; 3],
    direction: [f64; 3],
    magnitude: f64,
    same_sense: bool,
    native: geom::Line<Point3>,
}
impl NativeLineProjection {
    fn capture(
        source: &super::curve::Line,
        native: geom::Line<Point3>,
        same_sense: bool,
    ) -> Option<Self> {
        Some(Self {
            point: source.pnt.coordinates.as_slice().try_into().ok()?,
            direction: source
                .dir
                .orientation
                .direction_ratios
                .as_slice()
                .try_into()
                .ok()?,
            magnitude: source.dir.magnitude,
            same_sense,
            native,
        })
    }
    pub(in crate::step::load) fn matches_original(&self, table: &Table, edge: &EdgeCurveHolder) -> bool {
        fn resolve<'a, T>(value: &'a PlaceHolder<T>, map: &'a std::collections::HashMap<u64, T>) -> Option<&'a T> {
            match value {
                PlaceHolder::Ref(Name::Entity(id)) => map.get(id),
                PlaceHolder::Owned(value) => Some(value),
                _ => None,
            }
        }
        let source = match &edge.edge_geometry {
            PlaceHolder::Ref(Name::Entity(id)) => table.line.get(id),
            PlaceHolder::Owned(CurveAnyHolder::Line(line)) => Some(line.as_ref()),
            _ => None,
        };
        let Some((point, vector, direction)) = source.and_then(|source| {
            let point = resolve(&source.pnt, &table.cartesian_point)?;
            let vector = resolve(&source.dir, &table.vector)?;
            Some((point, vector, resolve(&vector.orientation, &table.direction)?))
        }) else { return false };
        point.coordinates.as_slice() == self.point
            && direction.direction_ratios.as_slice() == self.direction
            && vector.magnitude == self.magnitude
            && edge.same_sense == self.same_sense
            && self.native.0 == Point3::new(self.point[0], self.point[1], self.point[2])
    }
}

#[derive(Debug)]
pub(in crate::step::load) struct FiniteCurveTrace {
    pub(in crate::step::load) changes: Vec<FiniteCurveRepair>,
    pub(in crate::step::load) failed: bool,
    budget: std::rc::Rc<std::cell::Cell<usize>>,
}
impl FiniteCurveTrace {
    pub(in crate::step::load) fn new(budget: std::rc::Rc<std::cell::Cell<usize>>) -> Self {
        Self {
            changes: Vec::new(),
            failed: false,
            budget,
        }
    }
    fn charge(&mut self, bytes: Option<usize>) -> bool {
        let Some(bytes) = bytes else {
            self.failed = true;
            return false;
        };
        if self.failed || bytes > self.budget.get() || self.changes.len() >= 1_000_000 {
            self.failed = true;
            return false;
        }
        self.budget.set(self.budget.get() - bytes);
        true
    }
    fn begin(&mut self, curve: &Curve3D) -> Option<Curve3D> {
        self.charge(curve_storage(curve)).then(|| curve.clone())
    }
    fn finish(
        &mut self,
        stage: &'static str,
        before: Option<Curve3D>,
        before_domain: (f64, f64),
        after: &Curve3D,
        after_domain: (f64, f64),
    ) {
        let Some(before) = before else {
            return;
        };
        let bytes = curve_storage(after)
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<FiniteCurveRepair>()));
        if !self.charge(bytes) || self.changes.try_reserve_exact(1).is_err() {
            self.failed = true;
            return;
        }
        self.changes.push(FiniteCurveRepair {
            stage,
            before,
            before_domain,
            after: after.clone(),
            after_domain,
            native_line: None,
            seam: None,
        });
    }
}
pub(in crate::step::load) fn curve_storage(curve: &Curve3D) -> Option<usize> {
    repair_storage::curve(curve)
}
fn spline_storage<P>(curve: &BsplineCurve<P>) -> Option<usize> {
    curve
        .control_points()
        .capacity()
        .checked_mul(2 * std::mem::size_of::<P>())?
        .checked_add(
            curve
                .knot_vector()
                .len()
                .checked_mul(2 * std::mem::size_of::<f64>())?,
        )
}

/// `vertex_point`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vertex_point)]
#[holder(generate_deserialize)]
pub struct VertexPoint {
    pub label: String,
    #[holder(use_place_holder)]
    pub vertex_geometry: CartesianPoint,
}

/// `edge`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum EdgeAny {
    #[holder(use_place_holder)]
    EdgeCurve(EdgeCurve),
    #[holder(use_place_holder)]
    OrientedEdge(OrientedEdge),
}

/// `edge_curve`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = edge_curve)]
#[holder(generate_deserialize)]
pub struct EdgeCurve {
    pub label: String,
    #[holder(use_place_holder)]
    pub edge_start: VertexPoint,
    #[holder(use_place_holder)]
    pub edge_end: VertexPoint,
    #[holder(use_place_holder)]
    pub edge_geometry: CurveAny,
    pub same_sense: bool,
}

impl EdgeCurve {
    pub fn parse_curve2d(&self) -> Result<Curve2D, StepConvertingError> {
        let p = Point2::from(&self.edge_start.vertex_geometry);
        let q = Point2::from(&self.edge_end.vertex_geometry);
        let (p, q) = match self.same_sense {
            true => (p, q),
            false => (q, p),
        };
        Self::sub_parse_2d(&self.edge_geometry, p, q, self.same_sense)
    }
    fn sub_parse_2d(
        curve: &CurveAny,
        p: Point2,
        q: Point2,
        same_sense: bool,
    ) -> Result<Curve2D, StepConvertingError> {
        let mut curve = match curve {
            CurveAny::Line(line) => {
                let line = geom::Line::<Point2>::from(line.as_ref());
                let p = line.projection(p);
                let q = line.projection(q);
                Curve2D::Line(Line(p, q))
            }
            CurveAny::BoundedCurve(b) => b.as_ref().try_into()?,
            CurveAny::Conic(curve) => match curve.as_ref() {
                Conic::Circle(circle) => {
                    let mat =
                        Matrix3::try_from(&circle.position)? * Matrix3::from_scale(circle.radius);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                    );
                    if v <= u + TOLERANCE {
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point2>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve2D::Conic(Conic2D::Ellipse(ellipse))
                }
                Conic::Ellipse(ellipse) => {
                    let mat = Matrix3::try_from(&ellipse.position)?
                        * Matrix3::from_nonuniform_scale(ellipse.semi_axis_1, ellipse.semi_axis_2);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                        UnitCircle::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                    );
                    if v <= u + TOLERANCE {
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point2>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve2D::Conic(Conic2D::Ellipse(ellipse))
                }
                Conic::Hyperbola(hyperbola) => {
                    let mat = Matrix3::try_from(&hyperbola.position)?
                        * Matrix3::from_nonuniform_scale(
                            hyperbola.semi_axis,
                            hyperbola.semi_imag_axis,
                        );
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Hyperbola".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on hyperbola".to_string())?,
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on hyparbola".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point2>::new(), (u, v));
                    let mut hyperbola = Processor::new(unit);
                    hyperbola.transform_by(mat);
                    Curve2D::Conic(Conic2D::Hyperbola(hyperbola))
                }
                Conic::Parabola(parabola) => {
                    let mat = Matrix3::try_from(&parabola.position)?
                        * Matrix3::from_scale(parabola.focal_dist);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Parabola".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                        UnitHyperbola::<Point2>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point2>::new(), (u, v));
                    let mut parabola = Processor::new(unit);
                    parabola.transform_by(mat);
                    Curve2D::Conic(Conic2D::Hyperbola(parabola))
                }
            },
            CurveAny::Pcurve(_) => return Err("Pcurves cannot be parsed to 2D curves.".into()),
            CurveAny::SurfaceCurve(_) => {
                return Err("Surface curves cannot be parsed to 2D curves.".into());
            }
        };
        if !same_sense {
            curve.invert();
        }
        Ok(curve)
    }
    pub fn parse_curve3d(&self) -> Result<Curve3D, StepConvertingError> {
        self.parse_curve3d_with_bound(None, false)
    }
    /// Like [`Self::parse_curve3d`] but with preview-mode relaxations: a selected
    /// native LINE direction need not be bit-exactly retained.
    pub(crate) fn parse_curve3d_preview(&self) -> Result<Curve3D, StepConvertingError> {
        self.parse_curve3d_with_bound(None, true)
    }
    /// `bound` is the effective (possibly preview-floored) attachment bound.
    pub(crate) fn parse_curve3d_with_connectivity(
        &self,
        bound: f64,
        preview: bool,
    ) -> Result<Curve3D, StepConvertingError> {
        self.parse_curve3d_with_connectivity_recorded(bound, preview, None)
    }
    fn parse_curve3d_with_connectivity_recorded(
        &self,
        bound: f64,
        preview: bool,
        mut repair: Option<&mut FiniteCurveTrace>,
    ) -> Result<Curve3D, StepConvertingError> {
        if !bound.is_finite() || bound <= 0.0 || !bound.powi(2).is_finite() || bound.powi(2) == 0.0
        {
            return Err("invalid source connectivity bound".into());
        }
        let p = Point3::from(&self.edge_start.vertex_geometry);
        let q = Point3::from(&self.edge_end.vertex_geometry);
        // Preserve an already resolved interval before considering wider source attachments.
        let strict = matches!(&self.edge_geometry, CurveAny::BoundedCurve(_))
            .then(|| {
                self.parse_curve3d_with_bound_recorded(None, preview, repair.as_deref_mut())
                    .ok()
            })
            .flatten()
            .filter(|curve| {
                curve.front().distance(p) <= bound && curve.back().distance(q) <= bound
            });
        let curve = match strict {
            Some(curve) => curve,
            None => {
                // Captures from a rejected candidate cannot authenticate the
                // selected route. Keep conservative resource failures/charges.
                if let Some(trace) = repair.as_deref_mut() {
                    trace.changes.clear();
                }
                self.parse_curve3d_with_bound_recorded(Some(bound), preview, repair)?
            }
        };
        if [curve.front().distance(p), curve.back().distance(q)]
            .into_iter()
            .any(|d| !d.is_finite() || d > bound)
        {
            return Err("EDGE_CURVE attachment exceeds its source connectivity bound".into());
        }
        Ok(curve)
    }
    fn parse_curve3d_with_bound(
        &self,
        bound: Option<f64>,
        preview: bool,
    ) -> Result<Curve3D, StepConvertingError> {
        self.parse_curve3d_with_bound_recorded(bound, preview, None)
    }

    fn parse_curve3d_with_bound_recorded(
        &self,
        bound: Option<f64>,
        preview: bool,
        repair: Option<&mut FiniteCurveTrace>,
    ) -> Result<Curve3D, StepConvertingError> {
        let p = Point3::from(&self.edge_start.vertex_geometry);
        let q = Point3::from(&self.edge_end.vertex_geometry);
        let (p, q) = match self.same_sense {
            true => (p, q),
            false => (q, p),
        };
        if let Some(mut curve) = try_normalized_edge_bspline(&self.edge_geometry, p, q, bound)? {
            if !self.same_sense {
                curve.invert();
            }
            return Ok(curve);
        }
        let curve = match (&self.edge_geometry, repair) {
            (CurveAny::BoundedCurve(source), Some(capture)) => {
                let mut curve = bounded_trim::trim_edge_curve_in_native_frame_recorded(
                    source,
                    p,
                    q,
                    bound.unwrap_or(TOLERANCE),
                    self.same_sense,
                    capture,
                )?;
                if !self.same_sense {
                    let before_domain = curve.range_tuple();
                    let before = capture.begin(&curve);
                    curve.invert();
                    let (a, b) = curve.range_tuple();
                    capture.finish(
                        "native-sense-inversion-v1",
                        before,
                        before_domain,
                        &curve,
                        (b, a),
                    );
                }
                curve
            }
            (_, capture) => Self::sub_parse_curve3d(
                &self.edge_geometry,
                p,
                q,
                self.same_sense,
                bound,
                preview,
                capture,
            )?,
        };
        Ok(curve)
    }

    pub(in crate::step::load) fn parse_curve3d_for_repair(
        &self,
        bound: Option<f64>,
        preview: bool,
        budget: std::rc::Rc<std::cell::Cell<usize>>,
    ) -> Result<(Curve3D, FiniteCurveTrace), StepConvertingError> {
        let mut capture = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
        let curve = match bound {
            None => self.parse_curve3d_with_bound_recorded(None, preview, Some(&mut capture))?,
            Some(bound) => {
                self.parse_curve3d_with_connectivity_recorded(bound, preview, Some(&mut capture))?
            }
        };
        Ok((curve, capture))
    }
    fn sub_parse_curve3d(
        curve: &CurveAny,
        p: Point3,
        q: Point3,
        same_sense: bool,
        bound: Option<f64>,
        preview: bool,
        mut repair: Option<&mut FiniteCurveTrace>,
    ) -> Result<Curve3D, StepConvertingError> {
        let mut curve = match curve {
            CurveAny::Line(raw) => {
                if bound.is_some() {
                    let source: geom::Line<Point3> = raw.as_ref().into();
                    let direction = source.1 - source.0;
                    let denominator = direction.magnitude2();
                    if !denominator.is_finite() || denominator <= 0.0 {
                        return Err("invalid source line direction".into());
                    }
                    // This BEFORE is the finite attachment carrier, NOT the
                    // native STEP support or an inferred native scalar domain.
                    let pending = repair.as_deref_mut().and_then(|capture| {
                        let native = NativeLineProjection::capture(raw, source, same_sense)?;
                        let before = capture.begin(&Curve3D::Line(Line(p, q)))?;
                        Some((before, native))
                    });
                    let projected = Curve3D::Line(Line(source.projection(p), source.projection(q)));
                    if let (Some(capture), Some((before, native))) =
                        (repair.as_deref_mut(), pending)
                    {
                        let count = capture.changes.len();
                        capture.finish(
                            "native-line-attachment-projection-v1",
                            Some(before),
                            (0., 1.),
                            &projected,
                            (0., 1.),
                        );
                        if capture.changes.len() > count {
                            capture.changes.last_mut().unwrap().native_line = Some(native);
                        }
                    }
                    projected
                } else {
                    Curve3D::Line(Line(p, q))
                }
            }
            CurveAny::BoundedCurve(b) => bounded_trim::trim_edge_curve_in_native_frame(
                b.as_ref(),
                p,
                q,
                bound.unwrap_or(TOLERANCE),
                same_sense,
            )?,
            CurveAny::Conic(curve) => match curve.as_ref() {
                Conic::Circle(circle) => {
                    let mat =
                        Matrix4::try_from(&circle.position)? * Matrix4::from_scale(circle.radius);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {p:?}"))?,
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {q:?}"))?,
                    );
                    if v <= u + TOLERANCE {
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point3>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve3D::Conic(Conic3D::Ellipse(ellipse))
                }
                Conic::Ellipse(ellipse) => {
                    let mat = Matrix4::try_from(&ellipse.position)?
                        * Matrix4::from_nonuniform_scale(
                            ellipse.semi_axis_1,
                            ellipse.semi_axis_2,
                            f64::min(ellipse.semi_axis_1, ellipse.semi_axis_2),
                        );
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {p:?}"))?,
                        UnitCircle::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| format!("the point is not on circle: {q:?}"))?,
                    );
                    if v <= u + TOLERANCE {
                        v += 2.0 * PI;
                    }
                    let circle = TrimmedCurve::new(UnitCircle::<Point3>::new(), (u, v));
                    let mut ellipse = Processor::new(circle);
                    ellipse.transform_by(mat);
                    Curve3D::Conic(Conic3D::Ellipse(ellipse))
                }
                Conic::Hyperbola(hyperbola) => {
                    let mat = Matrix4::try_from(&hyperbola.position)?
                        * Matrix4::from_nonuniform_scale(
                            hyperbola.semi_axis,
                            hyperbola.semi_imag_axis,
                            f64::min(hyperbola.semi_axis, hyperbola.semi_imag_axis),
                        );
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Circle".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, mut v) = (
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on circle".to_string())?,
                    );
                    if v <= u + TOLERANCE {
                        v += 2.0 * PI;
                    }
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point3>::new(), (u, v));
                    let mut hyperbola = Processor::new(unit);
                    hyperbola.transform_by(mat);
                    Curve3D::Conic(Conic3D::Hyperbola(hyperbola))
                }
                Conic::Parabola(parabola) => {
                    let mat = Matrix4::try_from(&parabola.position)?
                        * Matrix4::from_scale(parabola.focal_dist);
                    let inv_mat = mat
                        .invert()
                        .ok_or_else(|| "Failed to convert Parabola".to_string())?;
                    let (p, q) = (inv_mat.transform_point(p), inv_mat.transform_point(q));
                    let (u, v) = (
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(p, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                        UnitHyperbola::<Point3>::new()
                            .search_nearest_parameter(q, None, 0)
                            .ok_or_else(|| "the point is not on parabola".to_string())?,
                    );
                    let unit = TrimmedCurve::new(UnitHyperbola::<Point3>::new(), (u, v));
                    let mut parabola = Processor::new(unit);
                    parabola.transform_by(mat);
                    Curve3D::Conic(Conic3D::Hyperbola(parabola))
                }
            },
            CurveAny::Pcurve(c) => {
                let surface: Surface = (&c.basis_surface).try_into()?;
                let u = surface
                    .search_nearest_parameter(p, None, 100)
                    .ok_or_else(|| "the point is not on surface".to_string())?;
                let v = surface
                    .search_nearest_parameter(q, None, 100)
                    .ok_or_else(|| "the point is not on surface".to_string())?;
                let curve2d = c
                    .reference_to_curve
                    .representation_item
                    .first()
                    .ok_or("no representation item")?;
                let curve2d = Self::sub_parse_2d(
                    curve2d,
                    Point2::new(u.0, u.1),
                    Point2::new(v.0, v.1),
                    true,
                )?;
                Curve3D::ParameterCurve(geom::ParameterCurve::new(
                    Box::new(curve2d),
                    Box::new(surface),
                ))
            }
            CurveAny::SurfaceCurve(c) => {
                c.validate_master()?;
                let Curve3D::SurfaceCurve(source) = Curve3D::try_from_surface_curve(c, preview)?
                else {
                    return Err("declared STEP surface curve did not retain its master".into());
                };
                let trimmed = if repair.is_some() {
                    source.try_trim_to_points_with_repair(
                        p,
                        q,
                        bound.unwrap_or(TOLERANCE),
                        preview,
                        repair.as_deref_mut(),
                    )
                } else {
                    source.try_trim_to_points(p, q, bound.unwrap_or(TOLERANCE), preview)
                };
                Curve3D::SurfaceCurve(
                    trimmed
                        .ok_or("edge endpoints do not resolve a finite selected-master interval")?,
                )
            }
        };
        if !same_sense {
            let pending = repair
                .as_deref_mut()
                .filter(|_| matches!(curve, Curve3D::Line(_)) && bound.is_some())
                .and_then(|capture| capture.begin(&curve));
            let surface_before = repair.as_deref_mut().and_then(|trace| match &curve {
                Curve3D::SurfaceCurve(c)
                    if c.planar_spline_master() && !trace.changes.is_empty() =>
                {
                    Some((trace.surface_begin(c), c.range_tuple()))
                }
                _ => None,
            });
            curve.invert();
            if let (Some(trace), Some((before, domain)), Curve3D::SurfaceCurve(after)) =
                (repair.as_deref_mut(), surface_before, &curve)
            {
                trace.surface_inversion(before, domain, after);
            }
            if let Some(capture) = repair {
                capture.finish(
                    "native-sense-inversion-v1",
                    pending,
                    (0., 1.),
                    &curve,
                    (1., 0.),
                );
            }
        }
        Ok(curve)
    }
}

/// Normalizes only a directly-used, clamped, non-rational B-spline edge curve
/// whose full parameter span is too small for the generic constructor guard.
/// The `p`/`q` arguments are the endpoint order already adjusted by
/// `EdgeCurve::parse_curve3d`; the supplied order is the native geometry order
/// even when the outer edge sense is reversed (the caller inverts afterward).
/// Other curve contexts fall back to the normal parser.
fn try_normalized_edge_bspline(
    curve: &CurveAny,
    p: Point3,
    q: Point3,
    bound: Option<f64>,
) -> Result<Option<Curve3D>, StepConvertingError> {
    let CurveAny::BoundedCurve(bounded) = curve else {
        return Ok(None);
    };
    let BoundedCurveAny::BsplineCurve(bspline) = bounded.as_ref() else {
        return Ok(None);
    };
    let BsplineCurveAny::NonRationalBsplineCurve(bspline) = bspline.as_ref() else {
        return Ok(None);
    };
    let NonRationalBsplineCurve::BsplineCurveWithKnots(source) = bspline.as_ref() else {
        return Ok(None);
    };

    let Some((&first, &last)) = source.knots.first().zip(source.knots.last()) else {
        return Err("empty B-spline knot list".into());
    };
    if source.knots.iter().any(|knot| !knot.is_finite()) || !first.is_finite() || !last.is_finite()
    {
        return Err("non-finite B-spline knot value".into());
    }
    let span = last - first;
    if !span.is_finite() || span <= 0.0 {
        return Err("B-spline edge knot domain must have a finite positive span".into());
    }
    if span > TOLERANCE {
        return Ok(None);
    }

    let degree = usize::try_from(source.degree).map_err(|_| "negative B-spline degree")?;
    let control_count = source.control_points_list.len();
    if degree == 0 || degree >= control_count {
        return Err(
            "B-spline degree must be positive and less than the control-point count".into(),
        );
    }
    if source.knots.len() != source.knot_multiplicities.len() {
        return Err("B-spline knot and multiplicity list lengths differ".into());
    }
    if source.knots.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("B-spline knot values must be strictly increasing".into());
    }
    let maximum_multiplicity = degree.checked_add(1).ok_or("B-spline degree overflow")?;
    match i64::try_from(maximum_multiplicity) {
        Ok(maximum_multiplicity)
            if source
                .knot_multiplicities
                .iter()
                .any(|multiplicity| *multiplicity > maximum_multiplicity) =>
        {
            return Err("B-spline knot multiplicity exceeds degree plus one".into());
        }
        _ => {}
    }
    let total_multiplicity = checked_knot_multiplicity_total(&source.knot_multiplicities)?;
    let expected_knot_count = degree
        .checked_add(control_count)
        .and_then(|count| count.checked_add(1))
        .ok_or("B-spline degree/control-point knot-count overflow")?;
    if total_multiplicity != expected_knot_count {
        return Err(format!(
            "B-spline multiplicity total {total_multiplicity} does not match expected knot count {expected_knot_count}"
        ).into());
    }
    let finite_point =
        |point: Point3| point.x.is_finite() && point.y.is_finite() && point.z.is_finite();
    if !finite_point(p)
        || !finite_point(q)
        || source
            .control_points_list
            .iter()
            .flat_map(|point| &point.coordinates)
            .any(|coordinate| !coordinate.is_finite())
    {
        return Err("B-spline edge controls and endpoints must be finite".into());
    }

    // Restrict this path to clamped full-domain splines. Reparameterizing an
    // unclamped active interval requires a broader treatment of its outside
    // knots and is deliberately left to the ordinary conversion path.
    let end_multiplicity = source
        .degree
        .checked_add(1)
        .ok_or("B-spline degree overflow")?;
    if source.knot_multiplicities.len() < 2
        || source.knot_multiplicities[0] < end_multiplicity
        || source.knot_multiplicities[source.knot_multiplicities.len() - 1] < end_multiplicity
    {
        return Ok(None);
    }

    let mut normalized = source.clone();
    normalized.knots = source
        .knots
        .iter()
        .map(|knot| (knot - first) / span)
        .collect();
    if normalized.knots.iter().any(|knot| !knot.is_finite()) {
        return Err("non-finite normalized B-spline knot value".into());
    }
    let normalized: BsplineCurve<Point3> = (&normalized).try_into()?;
    bounded_trim::trim_edge_curve(
        Curve3D::BsplineCurve(normalized),
        p,
        q,
        bound.unwrap_or(TOLERANCE),
    )
    .map(Some)
}

fn checked_knot_multiplicity_total(multiplicities: &[i64]) -> Result<usize, StepConvertingError> {
    multiplicities
        .iter()
        .try_fold(0usize, |total, multiplicity| {
            let multiplicity = usize::try_from(*multiplicity)
                .map_err(|_| "negative B-spline knot multiplicity")?;
            if multiplicity == 0 {
                return Err("B-spline knot multiplicities must be positive".into());
            }
            total
                .checked_add(multiplicity)
                .ok_or_else(|| "B-spline knot multiplicity total overflow".into())
        })
}

/// `oriented_edge`
///
/// `oriented_edge` has duplicated information.
/// These are not included here because they are essentially omitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_edge)]
#[holder(generate_deserialize)]
pub struct OrientedEdge {
    pub label: String,
    #[holder(use_place_holder)]
    pub edge_element: EdgeCurve,
    pub orientation: bool,
}

impl OrientedEdgeHolder {
    pub(crate) fn edge_element_holder(&self, table: &Table) -> Option<EdgeCurveHolder> {
        match &self.edge_element {
            PlaceHolder::Owned(holder) => Some(holder.clone()),
            PlaceHolder::Ref(Name::Entity(idx)) => table.edge_curve.get(idx).cloned(),
            _ => None,
        }
    }
    pub(crate) fn edge_element_idx(&self) -> Option<u64> {
        if let PlaceHolder::Ref(Name::Entity(idx)) = self.edge_element {
            Some(idx)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod edge_normalization_tests {
    use super::*;

    const START: f64 = 3.0;
    const SPAN: f64 = 8.7e-8;

    fn points() -> Vec<CartesianPoint> {
        [
            [0.0, 0.0, 0.0],
            [1.0, 2.0, 0.5],
            [2.0, -1.0, 1.5],
            [3.0, 0.0, 0.0],
        ]
        .into_iter()
        .map(|coordinates| CartesianPoint {
            label: String::new(),
            coordinates: coordinates.to_vec(),
        })
        .collect()
    }

    fn curve(knots: Vec<f64>) -> CurveAny {
        let spline = BsplineCurveWithKnots {
            label: String::new(),
            degree: 3,
            control_points_list: points(),
            curve_form: BsplineCurveForm::Unspecified,
            closed_curve: Logical::False,
            self_intersect: Logical::False,
            knot_multiplicities: vec![4, 4],
            knots,
            knot_spec: KnotType::Unspecified,
        };
        CurveAny::BoundedCurve(Box::new(BoundedCurveAny::BsplineCurve(Box::new(
            BsplineCurveAny::NonRationalBsplineCurve(Box::new(
                NonRationalBsplineCurve::BsplineCurveWithKnots(spline),
            )),
        ))))
    }

    fn small_curve() -> CurveAny {
        curve(vec![START, START + SPAN])
    }

    fn endpoints() -> (Point3, Point3) {
        (Point3::new(0.0, 0.0, 0.0), Point3::new(3.0, 0.0, 0.0))
    }

    #[test]
    fn normalized_edge_spline_preserves_points_and_derivative_chain_rule() {
        let direct = small_curve();
        let (p, q) = endpoints();
        let Some(Curve3D::BsplineCurve(normalized)) =
            try_normalized_edge_bspline(&direct, p, q, None).unwrap()
        else {
            panic!("the direct small-span edge spline should normalize");
        };
        let source_knots = KnotVector::from(vec![
            START,
            START,
            START,
            START,
            START + SPAN,
            START + SPAN,
            START + SPAN,
            START + SPAN,
        ]);
        let source_controls = points().iter().map(Point3::from).collect();
        // This unchecked instance is safe in this fixture: the STEP knot and
        // multiplicity data above satisfy the B-spline constructor invariants;
        // it lets the test compare the original parameter domain that the
        // ordinary checked constructor rejects solely for its small span.
        let source = BsplineCurve::new_unchecked(source_knots, source_controls);
        for fraction in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let t = START + fraction * SPAN;
            let actual = normalized.evaluate(fraction);
            let expected = source.evaluate(t);
            assert!(
                actual.distance2(expected) < 1.0e-12,
                "point at {fraction}: {actual:?} vs {expected:?}"
            );
            let actual_d1 = normalized.derivative(fraction);
            let expected_d1 = source.derivative(t) * SPAN;
            assert!(
                (actual_d1 - expected_d1).magnitude() < 1.0e-6,
                "derivative at {fraction}: {actual_d1:?} vs {expected_d1:?}"
            );
        }
        assert!(normalized.evaluate(0.0).distance2(p) < 1.0e-20);
        assert!(normalized.evaluate(1.0).distance2(q) < 1.0e-20);
    }

    #[test]
    fn exact_zero_and_nonfinite_edge_domains_are_rejected() {
        let (p, q) = endpoints();
        let zero = curve(vec![START, START]);
        assert!(try_normalized_edge_bspline(&zero, p, q, None).is_err());
        let nonfinite = curve(vec![START, f64::NAN]);
        assert!(try_normalized_edge_bspline(&nonfinite, p, q, None).is_err());
    }

    #[test]
    fn normalized_edge_spline_rejects_endpoint_mismatch() {
        let (p, mut q) = endpoints();
        q.x += 0.01;
        assert!(try_normalized_edge_bspline(&small_curve(), p, q, None).is_err());
    }

    #[test]
    fn reversed_edge_sense_inverts_the_normalized_curve_after_endpoint_check() {
        let Some(Curve3D::BsplineCurve(mut normalized)) = try_normalized_edge_bspline(
            &small_curve(),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            None,
        )
        .unwrap() else {
            panic!("the direct small-span edge spline should normalize");
        };
        // `EdgeCurve::sub_parse_curve3d` applies this same invert after the
        // helper returns when `same_sense` is false.
        normalized.invert();
        assert!(
            normalized
                .evaluate(0.0)
                .distance2(Point3::new(3.0, 0.0, 0.0))
                < 1.0e-20
        );
        assert!(
            normalized
                .evaluate(1.0)
                .distance2(Point3::new(0.0, 0.0, 0.0))
                < 1.0e-20
        );
    }

    #[test]
    fn non_edge_curve_variants_stay_on_the_baseline_conversion_path() {
        let (p, q) = endpoints();
        let line = CurveAny::Line(Box::default());
        assert!(
            try_normalized_edge_bspline(&line, p, q, None)
                .unwrap()
                .is_none()
        );
        // The ordinary shared converter remains unchanged and still reports
        // the original small-domain construction failure.
        assert!(Curve3D::try_from(&small_curve()).is_err());
    }

    #[test]
    fn surface_curve_wrappers_do_not_enter_the_edge_normalizer() {
        let wrapper = CurveAny::SurfaceCurve(Box::new(SurfaceCurve {
            label: String::new(),
            curve_3d: small_curve(),
            associated_geometry: Vec::new(),
            master_representation: PreferredSurfaceCurveRepresentation::Curve3D,
            kind: SurfaceCurveEntityKind::Surface,
        }));
        let (p, q) = endpoints();
        assert!(
            try_normalized_edge_bspline(&wrapper, p, q, None)
                .unwrap()
                .is_none()
        );
        assert!(EdgeCurve::sub_parse_curve3d(&wrapper, p, q, true, None, false, None).is_err());
    }

    fn spline_mut(curve: &mut CurveAny) -> &mut BsplineCurveWithKnots {
        let CurveAny::BoundedCurve(bounded) = curve else {
            panic!("expected bounded curve");
        };
        let BoundedCurveAny::BsplineCurve(bspline) = bounded.as_mut() else {
            panic!("expected B-spline");
        };
        let BsplineCurveAny::NonRationalBsplineCurve(bspline) = bspline.as_mut() else {
            panic!("expected non-rational B-spline");
        };
        let NonRationalBsplineCurve::BsplineCurveWithKnots(spline) = bspline.as_mut() else {
            panic!("expected B-spline with knots");
        };
        spline
    }

    #[test]
    fn malformed_knot_multiplicity_lengths_are_rejected() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).knot_multiplicities.push(1);
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn zero_knot_multiplicity_is_rejected() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).knot_multiplicities = vec![0, 4];
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn negative_knot_multiplicity_is_rejected() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).knot_multiplicities = vec![-1, 4];
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn knot_multiplicity_total_must_match_degree_and_control_count() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).knot_multiplicities = vec![4, 3];
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn duplicate_knot_values_are_rejected() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        let spline = spline_mut(&mut value);
        let extra_control = spline.control_points_list[3].clone();
        spline
            .control_points_list
            .extend((0..2).map(|_| extra_control.clone()));
        spline.knots = vec![START, START + SPAN / 2.0, START + SPAN / 2.0, START + SPAN];
        spline.knot_multiplicities = vec![4, 1, 1, 4];
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn knot_multiplicity_cannot_exceed_degree_plus_one() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        let spline = spline_mut(&mut value);
        let extra_control = spline.control_points_list[3].clone();
        spline
            .control_points_list
            .extend((0..5).map(|_| extra_control.clone()));
        spline.knots = vec![START, START + SPAN / 2.0, START + SPAN];
        // The total is otherwise consistent: 4 + 5 + 4 == degree 3 + 9 controls + 1.
        spline.knot_multiplicities = vec![4, 5, 4];
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn zero_spline_degree_is_rejected() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).degree = 0;
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn spline_degree_must_be_less_than_control_count() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).degree = 4;
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn nonfinite_control_points_are_rejected() {
        let (p, q) = endpoints();
        let mut value = small_curve();
        spline_mut(&mut value).control_points_list[1].coordinates[1] = f64::NAN;
        assert!(try_normalized_edge_bspline(&value, p, q, None).is_err());
    }

    #[test]
    fn nonfinite_edge_endpoints_are_rejected() {
        let (p, q) = endpoints();
        let mut nonfinite_start = p;
        nonfinite_start.x = f64::NAN;
        let mut nonfinite_end = q;
        nonfinite_end.z = f64::INFINITY;
        assert!(try_normalized_edge_bspline(&small_curve(), nonfinite_start, q, None).is_err());
        assert!(try_normalized_edge_bspline(&small_curve(), p, nonfinite_end, None).is_err());
    }

    #[test]
    fn multiplicity_total_overflow_is_rejected() {
        assert!(checked_knot_multiplicity_total(&[i64::MAX, i64::MAX, 2]).is_err());
    }
}

/// `edge_loop`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = edge_loop)]
#[holder(generate_deserialize)]
pub struct EdgeLoop {
    pub label: String,
    #[holder(use_place_holder)]
    pub edge_list: Vec<EdgeAny>,
}

/// `vertex_loop`
///
/// A `loop` whose entire extent is ONE vertex -- the degenerate boundary
/// exporters emit at a parameterisation singularity: a cone apex, a sphere pole,
/// or a stand-in for the (boundary-less) closed torus.
///
/// It has an arm here purely so the record is ATTRIBUTABLE. Before spec 011 T7
/// it fell into [`Table::dummy`](crate::step::load::Table::dummy), so
/// `FaceBoundHolder::bound_holder` looked the id up in `Table::edge_loop`, got
/// `None`, and the wire was discarded by a `filter_map` **with no error and no
/// message anywhere** -- the only truly silent drop the spec 011 Phase 0 census
/// found. It bit four in-repo fixtures, `boxy-with-surfacetex.step` losing 10 of
/// its 160 wires.
///
/// It is still not REPRESENTED in a `CompressedShell`: a compressed wire is a
/// sequence of edge uses and a point boundary has none, so the loop is reported
/// as [`LossReason::DegenerateVertexLoop`](crate::step::load::report::LossReason::DegenerateVertexLoop)
/// and dropped. Read that variant's docs for the measurement behind the choice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = vertex_loop)]
#[holder(generate_deserialize)]
pub struct VertexLoop {
    pub label: String,
    #[holder(use_place_holder)]
    pub loop_vertex: VertexPoint,
}

/// `face_bound`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = face_bound)]
#[holder(generate_deserialize)]
/// `FACE_OUTER_BOUNDS` is also parsed to this struct.
pub struct FaceBound {
    pub label: String,
    // For now, we are going with the policy of accepting nothing but edgeloop.
    #[holder(use_place_holder)]
    pub bound: EdgeLoop,
    pub orientation: bool,
}

/// What a `FACE_BOUND`'s `bound` reference turns out to point at.
///
/// The declared type of `FaceBound::bound` is `EdgeLoop`, but STEP's `loop` has
/// three subtypes and the reference is untyped until it is resolved. Naming the
/// three outcomes is what lets the loader distinguish a LEGITIMATE degeneracy
/// (a vertex loop) from a genuine loss (a bound that is simply not there).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BoundResolution {
    /// An `EDGE_LOOP`, inline or resolved from the table.
    EdgeLoop(EdgeLoopHolder),
    /// A `VERTEX_LOOP`, with its entity id.
    VertexLoop(u64),
    /// Neither: no record with that id, or a `loop` subtype with no arm. Carries
    /// the id when the reference had one.
    Unresolved(Option<u64>),
}

impl FaceBoundHolder {
    pub(crate) fn resolve_bound(&self, table: &Table) -> BoundResolution {
        match &self.bound {
            PlaceHolder::Owned(holder) => BoundResolution::EdgeLoop(holder.clone()),
            PlaceHolder::Ref(Name::Entity(idx)) => table
                .edge_loop
                .get(idx)
                .cloned()
                .map(BoundResolution::EdgeLoop)
                .unwrap_or_else(|| {
                    if table.vertex_loop.contains_key(idx) {
                        BoundResolution::VertexLoop(*idx)
                    } else {
                        BoundResolution::Unresolved(Some(*idx))
                    }
                }),
            _ => BoundResolution::Unresolved(None),
        }
    }

    pub(crate) fn bound_holder(&self, table: &Table) -> Option<EdgeLoopHolder> {
        match self.resolve_bound(table) {
            BoundResolution::EdgeLoop(holder) => Some(holder),
            _ => None,
        }
    }
}

/// `face`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum FaceAny {
    #[holder(use_place_holder)]
    FaceSurface(FaceSurface),
    #[holder(use_place_holder)]
    OrientedFace(OrientedFace),
}

/// `face_surface`
///
/// `advanced_face` is also parsed to this struct.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = face_surface)]
#[holder(generate_deserialize)]
pub struct FaceSurface {
    pub label: String,
    #[holder(use_place_holder)]
    pub bounds: Vec<FaceBound>,
    #[holder(use_place_holder)]
    pub face_geometry: SurfaceAny,
    pub same_sense: bool,
}

impl FaceSurfaceHolder {
    /// Every bound the face LISTS, paired with the entity id the face referenced.
    ///
    /// The id is carried alongside so a bound that fails to resolve can still be
    /// reported by name -- without it, a lost wire is anonymous and a human has
    /// nothing to go and look at.
    pub(crate) fn bounds_holder<'a>(
        &'a self,
        table: &'a Table,
    ) -> Vec<(Option<u64>, Option<FaceBoundHolder>)> {
        self.bounds
            .iter()
            .map(|bound| match bound {
                PlaceHolder::Owned(bound) => (None, Some(bound.clone())),
                PlaceHolder::Ref(Name::Entity(idx)) => {
                    (Some(*idx), table.face_bound.get(idx).cloned())
                }
                _ => (None, None),
            })
            .collect()
    }
}

/// `oriented_face`
///
/// `oriented_face` has duplicated information.
/// These are not included here because they are essentially omitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_face)]
#[holder(generate_deserialize)]
pub struct OrientedFace {
    pub label: String,
    #[holder(use_place_holder)]
    pub face_element: FaceSurface,
    pub orientation: bool,
}

impl OrientedFaceHolder {
    pub(crate) fn face_element_holder(&self, table: &Table) -> Option<FaceSurfaceHolder> {
        match &self.face_element {
            PlaceHolder::Ref(Name::Entity(idx)) => table.face_surface.get(idx).cloned(),
            PlaceHolder::Owned(x) => Some(x.clone()),
            _ => None,
        }
    }
}

/// `shell`
///
/// Includes `open_shell` and `closed_shell`.
/// Since these differences are only informal propositions, the data structure does not distinguish between the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shell)]
#[holder(generate_deserialize)]
pub struct Shell {
    pub label: String,
    #[holder(use_place_holder)]
    pub cfs_faces: Vec<FaceAny>,
}

impl ShellHolder {
    pub(crate) fn cfs_faces_holder<'a>(
        &'a self,
        table: &'a Table,
    ) -> impl Iterator<Item = Option<FaceAnyHolder>> + 'a {
        self.cfs_faces.iter().map(|face| match face {
            PlaceHolder::Owned(holder) => Some(holder.clone()),
            PlaceHolder::Ref(Name::Entity(idx)) => table
                .oriented_face
                .get(idx)
                .cloned()
                .map(FaceAnyHolder::OrientedFace)
                .or_else(|| {
                    table
                        .face_surface
                        .get(idx)
                        .cloned()
                        .map(FaceAnyHolder::FaceSurface)
                }),
            _ => None,
        })
    }
}

/// `oriented_shell`
///
/// Includes `oriented_open_shell` and `oriented_closed_shell`.
/// Since these differences are only informal propositions, the data structure does not distinguish between the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = oriented_shell)]
#[holder(generate_deserialize)]
pub struct OrientedShell {
    pub label: String,
    #[holder(use_place_holder)]
    pub shell_element: Shell,
    pub orientation: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(generate_deserialize)]
pub enum ShellAny {
    #[holder(use_place_holder)]
    Shell(Shell),
    #[holder(use_place_holder)]
    OrientedShell(OrientedShell),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = shell_based_surface_model)]
#[holder(generate_deserialize)]
pub struct ShellBasedSurfaceModel {
    pub label: String,
    #[holder(use_place_holder)]
    pub sbsm_boundary: Vec<ShellAny>,
}

/// Also serves as `brep_with_voids`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Holder)]
#[holder(table = Table)]
#[holder(field = manifold_solid_brep)]
#[holder(generate_deserialize)]
pub struct ManifoldSolidBrep {
    pub label: String,
    #[holder(use_place_holder)]
    pub outer: ShellAny,
    #[holder(use_place_holder)]
    pub voids: Vec<OrientedShell>,
}
