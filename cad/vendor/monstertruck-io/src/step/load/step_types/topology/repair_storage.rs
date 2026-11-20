//! Behavioral controls for the IO witness owner.
use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::load::step_geometry::Plane;

    fn spare<T: Copy>(points: &[T]) -> Vec<T> {
        let mut values = Vec::with_capacity(128);
        values.extend_from_slice(points);
        values
    }
    fn line() -> Curve3D {
        Curve3D::Line(Line(Point3::origin(), Point3::new(1.0, 0.0, 0.0)))
    }
    fn plane() -> Surface {
        Surface::ElementarySurface(ElementarySurface::Plane(Plane::new(
            Point3::origin(),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        )))
    }
    fn spline() -> Curve3D {
        Curve3D::BsplineCurve(BsplineCurve::new(
            KnotVector::from(spare(&[0.0, 0.0, 1.0, 1.0])),
            spare(&[Point3::origin(), Point3::new(1.0, 0.0, 0.0)]),
        ))
    }

    #[test]
    fn repair_movement_sink_concrete_snapshot_variants_preflight_all_owned_layers() {
        let uv = [
            Curve2D::Line(Line(Point2::origin(), Point2::new(1.0, 0.0))),
            Curve2D::Polyline(PolylineCurve(spare(&[
                Point2::origin(),
                Point2::new(1.0, 0.0),
            ]))),
            Curve2D::Conic(Conic2D::Ellipse(Processor::new(TrimmedCurve::new(
                UnitCircle::<Point2>::new(),
                (0.0, 1.0),
            )))),
            Curve2D::BsplineCurve(BsplineCurve::new(
                KnotVector::bezier_knot(1),
                spare(&[Point2::origin(), Point2::new(1.0, 0.0)]),
            )),
            Curve2D::NurbsCurve(NurbsCurve::new(BsplineCurve::new(
                KnotVector::bezier_knot(1),
                spare(&[Vector3::new(0.0, 0.0, 1.0), Vector3::new(1.0, 0.0, 1.0)]),
            ))),
        ];
        let mut cases = vec![
            line(),
            Curve3D::Conic(Conic3D::Ellipse(Processor::new(TrimmedCurve::new(
                UnitCircle::<Point3>::new(),
                (0.0, 1.0),
            )))),
            Curve3D::Polyline(PolylineCurve(spare(&[
                Point3::origin(),
                Point3::new(1.0, 0.0, 0.0),
            ]))),
            spline(),
            Curve3D::NurbsCurve(NurbsCurve::new(BsplineCurve::new(
                KnotVector::bezier_knot(1),
                spare(&[
                    Vector4::new(0.0, 0.0, 0.0, 1.0),
                    Vector4::new(1.0, 0.0, 0.0, 1.0),
                ]),
            ))),
        ];
        for curve in &uv {
            cases.push(Curve3D::ParameterCurve(StepParameterCurve::new(
                Box::new(curve.clone()),
                Box::new(plane()),
            )));
        }
        let nets = [
            Surface::BsplineSurface(BsplineSurface::new(
                (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
                vec![
                    spare(&[Point3::origin(), Point3::new(0.0, 1.0, 0.0)]),
                    spare(&[Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)]),
                ],
            )),
            Surface::NurbsSurface(NurbsSurface::new(BsplineSurface::new(
                (KnotVector::bezier_knot(1), KnotVector::bezier_knot(1)),
                vec![
                    spare(&[
                        Vector4::new(0.0, 0.0, 0.0, 1.0),
                        Vector4::new(0.0, 1.0, 0.0, 1.0),
                    ]),
                    spare(&[
                        Vector4::new(1.0, 0.0, 0.0, 1.0),
                        Vector4::new(1.0, 1.0, 0.0, 1.0),
                    ]),
                ],
            ))),
            Surface::SweepSurface(SweepSurface::ExtrusionSurface(
                ExtrusionSurface::by_extrusion(spline(), Vector3::unit_z()),
            )),
            Surface::SweepSurface(SweepSurface::RevolutionSurface(Processor::with_transform(
                RevolutionSurface::by_revolution(
                    spline(),
                    Point3::origin(),
                    Vector3::new(1.0, 1.0, 1.0),
                ),
                Matrix4::from_scale(2.0),
            ))),
        ];
        for surface in nets {
            cases.push(Curve3D::ParameterCurve(StepParameterCurve::new(
                Box::new(uv[4].clone()),
                Box::new(surface.clone()),
            )));
            let mut associations = Vec::with_capacity(128);
            associations.push(SurfaceCurveAssociatedGeometry::ParameterCurve(
                StepParameterCurve::new(Box::new(uv[3].clone()), Box::new(surface.clone())),
            ));
            associations.push(SurfaceCurveAssociatedGeometry::Surface(Box::new(
                surface.clone(),
            )));
            cases.push(Curve3D::SurfaceCurve(SurfaceCurve3D::new(
                SurfaceCurveKind::SurfaceCurve,
                Box::new(spline()),
                associations,
                SurfaceCurveRepresentation::Curve3D,
            )));
            cases.push(Curve3D::IntersectionCurve(IntersectionCurve::new(
                Box::new(surface),
                Box::new(plane()),
                Box::new(spline()),
            )));
        }
        for (index, curve) in cases.iter().enumerate() {
            let budget = std::rc::Rc::new(std::cell::Cell::new(0));
            let mut trace = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
            assert_eq!(
                trace.begin(curve).is_some(),
                index < 2,
                "only the inline line/conic snapshots need no heap allocation: {index}"
            );
            assert_eq!(trace.failed, index >= 2);
            budget.set(65536);
            let mut trace = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
            let before = trace.begin(curve).unwrap();
            assert_eq!(
                &before, curve,
                "all metadata must survive verbatim: {index}"
            );
            trace.finish(
                "native-domain-relabel-v1",
                Some(before),
                (0.0, 1.0),
                curve,
                (0.0, 1.0),
            );
            assert_eq!(trace.changes.len(), 1);
            assert_eq!(&trace.changes[0].after, curve);
            let remaining = budget.get();
            trace.surface_rollback(0);
            assert!(trace.changes.is_empty());
            assert!(trace.changes.capacity() > 0);
            assert_eq!(
                budget.get(),
                remaining,
                "rollback retains its conservative charge"
            );
        }
    }

    #[test]
    fn repair_movement_sink_retained_shift_keeps_actual_spare_capacity_and_failure() {
        let mut knots = Vec::with_capacity(1024);
        knots.extend([0.0, 0.0, 1.0, 1.0]);
        let shifted = Curve2D::BsplineCurve(BsplineCurve::new(
            KnotVector::from(knots),
            vec![Point2::origin(), Point2::new(1.0, 0.0)],
        ));
        let budget = std::rc::Rc::new(std::cell::Cell::new(4096));
        let mut trace = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
        assert!(
            trace.surface_shifted(shifted).is_none(),
            "a move retains the large existing knot buffer"
        );
        assert!(trace.failed);
        assert_eq!(
            budget.get(),
            4096,
            "refuse retention before changing ownership"
        );
        trace.surface_rollback(0);
        assert!(trace.failed);
        assert!(
            trace.begin(&line()).is_none(),
            "rollback cannot reopen failed capture"
        );
    }

    #[test]
    fn repair_movement_sink_depth_and_node_limits_precede_nested_clone() {
        let mut curve = line();
        for _ in 0..16 {
            curve = Curve3D::ParameterCurve(StepParameterCurve::new(
                Box::new(Curve2D::Line(Line(Point2::origin(), Point2::new(1.0, 0.0)))),
                Box::new(Surface::SweepSurface(SweepSurface::ExtrusionSurface(
                    ExtrusionSurface::by_extrusion(curve, Vector3::unit_z()),
                ))),
            ));
        }
        let budget = std::rc::Rc::new(std::cell::Cell::new(16 * 1024 * 1024));
        let mut trace = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
        assert!(trace.begin(&curve).is_some());
        let deeper = Curve3D::ParameterCurve(StepParameterCurve::new(
            Box::new(Curve2D::Line(Line(Point2::origin(), Point2::new(1.0, 0.0)))),
            Box::new(Surface::SweepSurface(SweepSurface::ExtrusionSurface(
                ExtrusionSurface::by_extrusion(curve, Vector3::unit_z()),
            ))),
        ));
        assert!(trace.begin(&deeper).is_none());
        assert!(trace.failed);
        for entries in [8190, 8191] {
            let curve = Curve3D::SurfaceCurve(SurfaceCurve3D::new(
                SurfaceCurveKind::SurfaceCurve,
                Box::new(line()),
                (0..entries)
                    .map(|_| SurfaceCurveAssociatedGeometry::Surface(Box::new(plane())))
                    .collect(),
                SurfaceCurveRepresentation::Curve3D,
            ));
            let mut trace = FiniteCurveTrace::new(std::rc::Rc::clone(&budget));
            let remaining = budget.get();
            assert_eq!(trace.begin(&curve).is_some(), entries == 8190);
            if entries == 8191 {
                assert_eq!(budget.get(), remaining);
            }
        }
    }

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
        let Curve3D::ParameterCurve(copy) = &snapshot else {
            unreachable!()
        };
        let Surface::BsplineSurface(copy) = copy.surface().as_ref() else {
            unreachable!()
        };
        assert_eq!(copy.control_points().capacity(), 2);
        assert!(copy.control_points().iter().all(|row| row.capacity() == 2));
        assert!(!trace.failed);
        assert!(budget.get() > 0);
    }
}
