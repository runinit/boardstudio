//! Same-invocation seam pieces; never caller-constructed certificates.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SeamCopy {
    shifted: Curve2D,
    join: f64,
}
impl FiniteCurveRepair {
    pub(in crate::step::load) fn surface_stage_valid(&self) -> bool {
        let stage = self.stage;
        if !stage.starts_with("same-master-planar-pcurve-") {
            return true;
        }
        let (Curve3D::SurfaceCurve(a), Curve3D::SurfaceCurve(b)) = (&self.before, &self.after)
        else {
            return false;
        };
        if !a.planar_spline_master()
            || !b.planar_spline_master()
            || a.master_representation() != b.master_representation()
        {
            return false;
        }
        match stage {
            "same-master-planar-pcurve-seam-extension-v1" => {
                let Some(copy) = &self.seam else {
                    return false;
                };
                let slot = match a.master_representation() {
                    SurfaceCurveRepresentation::ParameterCurve0 => 0,
                    SurfaceCurveRepresentation::ParameterCurve1 => 1,
                    _ => return false,
                };
                let SurfaceCurveAssociatedGeometry::ParameterCurve(master) =
                    &a.associated_geometry()[slot]
                else {
                    return false;
                };
                let (s, e) = master.curve().range_tuple();
                let span = e - s;
                let matches = match (master.curve().as_ref(), &copy.shifted) {
                    (Curve2D::BsplineCurve(a), Curve2D::BsplineCurve(b)) => {
                        a.control_points() == b.control_points()
                            && shifted_knots(a.knot_vector(), b.knot_vector(), span)
                    }
                    (Curve2D::NurbsCurve(a), Curve2D::NurbsCurve(b)) => {
                        a.control_points() == b.control_points()
                            && shifted_knots(a.knot_vector(), b.knot_vector(), span)
                    }
                    _ => false,
                };
                matches && copy.join == s+span && self.before_domain == (s,e)
                    && self.after_domain == (s,e+span) && a.active_trim().is_none() && b.active_trim().is_none()
                    && a.kind() == b.kind() && a.leader() == b.leader()
                    && a.associated_geometry().len() == b.associated_geometry().len()
                    && a.associated_geometry().iter().zip(b.associated_geometry()).enumerate().all(|(i,(a,b))| {
                        if i != slot { return a == b; }
                        matches!((a,b), (SurfaceCurveAssociatedGeometry::ParameterCurve(a),SurfaceCurveAssociatedGeometry::ParameterCurve(b)) if a.surface() == b.surface() && b.curve().range_tuple() == self.after_domain)
                    })
            }
            "same-master-planar-pcurve-trim-v1" => {
                self.seam.is_none()
                    && self.before_domain == self.after_domain
                    && b.active_trim() == Some(self.after_domain)
                    && a.kind() == b.kind()
                    && a.leader() == b.leader()
                    && a.associated_geometry() == b.associated_geometry()
            }
            "same-master-planar-pcurve-sense-inversion-v1" => {
                self.seam.is_none()
                    && a.inverse() == *b
                    && self.before_domain == a.range_tuple()
                    && self.after_domain == (b.range_tuple().1, b.range_tuple().0)
            }
            _ => false,
        }
    }
    pub(in crate::step::load) fn surface_restricts(&self, previous: (f64, f64)) -> bool {
        self.stage == "same-master-planar-pcurve-trim-v1"
            && self.surface_stage_valid()
            && previous.0 <= self.before_domain.0
            && self.before_domain.0 < self.before_domain.1
            && self.before_domain.1 <= previous.1
    }
}
fn shifted_knots(a: &KnotVector, b: &KnotVector, span: f64) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a + span == *b)
}
impl FiniteCurveTrace {
    pub(in crate::step::load) fn surface_mark(&self) -> usize {
        self.changes.len()
    }
    pub(in crate::step::load) fn surface_rollback(&mut self, mark: usize) {
        self.changes.truncate(mark);
        // Retained charge and failure are monotone even when a candidate fails.
    }
    pub(in crate::step::load) fn surface_begin(
        &mut self,
        curve: &SurfaceCurve3D,
    ) -> Option<Curve3D> {
        // Account before wrapping/cloning nested source curves/surfaces.
        let cost = repair_storage::surface_curve(curve);
        self.charge(cost)
            .then(|| Curve3D::SurfaceCurve(curve.clone()))
    }
    pub(in crate::step::load) fn surface_finish(
        &mut self,
        before: Option<Curve3D>,
        domain: (f64, f64),
        after: &SurfaceCurve3D,
    ) {
        let Some(after) = self.surface_begin(after) else {
            return;
        };
        // Move the charged AFTER rather than making another nested clone.
        self.surface_push(
            "same-master-planar-pcurve-trim-v1",
            before,
            domain,
            after,
            domain,
            None,
        );
    }
    pub(in crate::step::load) fn surface_shifted(&mut self, curve: Curve2D) -> Option<Curve2D> {
        self.charge(repair_storage::curve2(&curve)).then_some(curve)
    }
    pub(in crate::step::load) fn surface_seam(
        &mut self,
        before: Option<Curve3D>,
        domain: (f64, f64),
        after: &SurfaceCurve3D,
        shifted: Option<Curve2D>,
    ) {
        let Some(shifted) = shifted else {
            self.failed = true;
            return;
        };
        let Some(after) = self.surface_begin(after) else {
            return;
        };
        let span = domain.1 - domain.0;
        self.surface_push(
            "same-master-planar-pcurve-seam-extension-v1",
            before,
            domain,
            after,
            (domain.0, domain.1 + span),
            Some(SeamCopy {
                shifted,
                join: domain.0 + span,
            }),
        );
    }
    fn surface_push(
        &mut self,
        stage: &'static str,
        before: Option<Curve3D>,
        before_domain: (f64, f64),
        after: Curve3D,
        after_domain: (f64, f64),
        seam: Option<SeamCopy>,
    ) {
        let Some(before) = before else {
            return;
        };
        if !self.charge(Some(2 * std::mem::size_of::<FiniteCurveRepair>()))
            || self.changes.try_reserve_exact(1).is_err()
        {
            self.failed = true;
            return;
        }
        self.changes.push(FiniteCurveRepair {
            stage,
            before,
            before_domain,
            after,
            after_domain,
            native_line: None,
            seam,
        });
    }
    pub(in crate::step::load) fn surface_inversion(
        &mut self,
        before: Option<Curve3D>,
        domain: (f64, f64),
        after: &SurfaceCurve3D,
    ) {
        let range = after.range_tuple();
        let Some(after) = self.surface_begin(after) else {
            return;
        };
        self.surface_push(
            "same-master-planar-pcurve-sense-inversion-v1",
            before,
            domain,
            after,
            (range.1, range.0),
            None,
        );
    }
}
