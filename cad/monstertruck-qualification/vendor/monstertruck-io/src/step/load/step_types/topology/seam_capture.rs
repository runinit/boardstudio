//! Same-invocation seam pieces; never caller-constructed certificates.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SeamCopy {
    shifted: Curve2D,
    join: f64,
}
impl FiniteCurveRepair {
    pub(in crate::step::load) fn surface_stage_valid(
        &self,
        budget: &std::cell::Cell<usize>,
    ) -> bool {
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
                    && step_geometry::repair_storage::inverts_to(a, b, budget)
                    && self.before_domain == a.range_tuple()
                    && self.after_domain == (b.range_tuple().1, b.range_tuple().0)
            }
            _ => false,
        }
    }
    pub(in crate::step::load) fn surface_restricts(&self, previous: (f64, f64)) -> bool {
        self.stage == "same-master-planar-pcurve-trim-v1"
            && self.surface_stage_valid(&std::cell::Cell::new(0))
            && previous.0 <= self.before_domain.0
            && self.before_domain.0 < self.before_domain.1
            && self.before_domain.1 <= previous.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use step_p21::tables::IntoOwned;

    #[test]
    fn repair_movement_sink_inverse_validation_requires_its_own_workspace() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../monstertruck/tests/fixtures/repair-planar-seam.step"
        ));
        let sense = "#8=EDGE_CURVE('',#2,#32,#7,.T.);";
        assert_eq!(source.matches(sense).count(), 1);
        let table =
            Table::from_step(&source.replace(sense, "#8=EDGE_CURVE('',#2,#32,#7,.F.);")).unwrap();
        let edge = table.edge_curve[&8].clone().into_owned(&table).unwrap();
        let budget = std::rc::Rc::new(std::cell::Cell::new(256 * 1024 * 1024));
        let (output, trace) = edge
            .parse_curve3d_for_repair(Some(0.02), true, budget)
            .unwrap();
        assert_eq!(
            output,
            edge.parse_curve3d_with_connectivity(0.02, true).unwrap()
        );
        let change = trace
            .changes
            .iter()
            .find(|change| change.stage == "same-master-planar-pcurve-sense-inversion-v1")
            .unwrap();
        assert_eq!(change.after, output);
        let workspace = std::cell::Cell::new(0);
        assert!(
            !change.surface_stage_valid(&workspace),
            "actual inverse comparison cannot borrow the retained snapshots' allocation charge"
        );
        assert_eq!(workspace.get(), 0);
        workspace.set(65536);
        assert!(change.surface_stage_valid(&workspace));
        assert_eq!(workspace.get(), 65536, "generous allowance is restored");
        // The reservation is released, so find the exact requirement by search.
        let (mut low, mut high) = (0, 65536);
        while low < high {
            let mid = (low + high) / 2;
            workspace.set(mid);
            if change.surface_stage_valid(&workspace) {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let needed = low;
        assert!(needed > 0);
        let clone_budget = std::rc::Rc::new(std::cell::Cell::new(65536));
        let mut comparison = FiniteCurveTrace::new(std::rc::Rc::clone(&clone_budget));
        let Curve3D::SurfaceCurve(before) = &change.before else {
            unreachable!()
        };
        comparison.surface_begin(before).unwrap();
        assert!(
            needed > 65536 - clone_budget.get(),
            "inverse needs knot scratch in addition to the same actual carrier clone"
        );
        workspace.set(needed - 1);
        assert!(!change.surface_stage_valid(&workspace));
        assert_eq!(workspace.get(), needed - 1, "refuse before cloning");
        workspace.set(needed);
        assert!(change.surface_stage_valid(&workspace));
        assert_eq!(
            workspace.get(),
            needed,
            "temporary inverse carrier and scratch are released after comparison"
        );
        for _ in 0..8 {
            assert!(change.surface_stage_valid(&workspace));
        }
        assert_eq!(workspace.get(), needed);

        // A mismatching comparison also drops its workspace and stays invalid.
        let mut mismatch = change.clone();
        mismatch.after = change.before.clone();
        assert!(!mismatch.surface_stage_valid(&workspace));
        assert_eq!(workspace.get(), needed);
    }

    #[test]
    fn repair_movement_sink_actual_rejected_seam_keeps_prior_records_charges_and_failures() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../monstertruck/tests/fixtures/repair-planar-seam.step"
        ));
        let table = Table::from_step(source).unwrap();
        let edge = table.edge_curve[&8].clone().into_owned(&table).unwrap();
        let unclamped = source.replace(
            "((3,2,2,3),(0.,1.,2.,3.),.UNSPECIFIED.)",
            "((1,1,1,2,2,1,1,1),(-1.,-0.00000001,0.,1.,2.,3.,3.00000001,4.),.UNSPECIFIED.)",
        );
        assert_ne!(unclamped, source);
        let rejected_table = Table::from_step(&unclamped).unwrap();
        let rejected = rejected_table.edge_curve[&8]
            .clone()
            .into_owned(&rejected_table)
            .unwrap();
        let CurveAny::SurfaceCurve(raw) = &rejected.edge_geometry else {
            unreachable!()
        };
        let Curve3D::SurfaceCurve(raw) = Curve3D::try_from_surface_curve(raw, true).unwrap() else {
            unreachable!()
        };
        let front = Point3::from(&rejected.edge_start.vertex_geometry);
        let back = Point3::from(&rejected.edge_end.vertex_geometry);
        let expected = raw.try_trim_to_points(front, back, 0.02, true).unwrap();
        assert_ne!(
            raw.master_representation(),
            expected.master_representation()
        );
        for remaining in [0, 65536] {
            let budget = std::rc::Rc::new(std::cell::Cell::new(65536));
            let (_, mut trace) = edge
                .parse_curve3d_for_repair(Some(0.02), true, std::rc::Rc::clone(&budget))
                .unwrap();
            assert!(
                trace.changes.iter().any(|change| change.seam.is_some()),
                "the real accepted path retains its shifted copy"
            );
            let prior = trace.changes.clone();
            let capacity = trace.changes.capacity();
            budget.set(remaining);
            let actual = raw
                .try_trim_to_points_with_repair(front, back, 0.02, true, Some(&mut trace))
                .unwrap();
            assert_eq!(actual, expected);
            assert_eq!(
                trace.changes, prior,
                "rejected candidate cannot remove earlier captures"
            );
            assert!(trace.changes.capacity() >= capacity);
            assert_eq!(trace.failed, remaining == 0);
            if remaining > 0 {
                assert!(
                    budget.get() < remaining,
                    "rejected snapshots remain charged"
                );
            }
            trace.surface_rollback(trace.surface_mark());
            assert_eq!(trace.failed, remaining == 0);
        }
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
        if !self.charge(Some(0)) {
            return None;
        }
        let copy = step_geometry::repair_storage::surface_curve(curve, &self.budget);
        self.failed |= copy.is_none();
        copy.map(Curve3D::SurfaceCurve)
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
        // This moves an existing preview allocation; retain its actual spare
        // capacity while snapshots and the joined preview output coexist.
        if !self.charge(Some(0)) {
            return None;
        }
        if !step_geometry::repair_storage::retain_curve2(&curve, &self.budget) {
            self.failed = true;
            return None;
        }
        Some(curve)
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
        self.push(FiniteCurveRepair {
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
