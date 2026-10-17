use monstertruck_geometry::prelude as truck;
use monstertruck_io::step::{load::*, save::*};
use monstertruck_mesh::PolylineCurve;
use std::{
    f64::consts::PI,
    fmt::{Debug, Display},
    str::FromStr,
};
use step_p21::{
    ast::DataSection,
    tables::{EntityTable, Holder},
};
use truck::*;

fn step_roundtrip<Truck, StepHolder>(t: &Truck) -> StepHolder::Owned
where
    StepHolder: Holder<Table = Table>,
    for<'a> StepDisplay<&'a Truck>: Display,
    Table: EntityTable<StepHolder>, {
    let step_display = StepDisplay::new(t, 1);
    let step = format!("DATA;\n{step_display}ENDSEC;");
    println!("{step}");
    let data_section = DataSection::from_str(&step).unwrap();
    let table = Table::from_data_section(&data_section);
    EntityTable::get_owned(&table, 1).unwrap()
}

fn oitest<Truck, StepHolder>(t: Truck)
where
    StepHolder: Holder<Table = Table>,
    Truck: for<'a> From<&'a StepHolder::Owned> + Debug + PartialEq,
    for<'a> StepDisplay<&'a Truck>: Display,
    Table: EntityTable<StepHolder>, {
    let got = Truck::from(&step_roundtrip::<Truck, StepHolder>(&t));
    assert_eq!(t, got);
}

fn oitest_tryfrom<Truck, StepHolder>(t: Truck)
where
    StepHolder: Holder<Table = Table>,
    Truck: for<'a> TryFrom<&'a StepHolder::Owned> + Debug + PartialEq,
    for<'a> <Truck as TryFrom<&'a StepHolder::Owned>>::Error: Debug,
    for<'a> StepDisplay<&'a Truck>: Display,
    Table: EntityTable<StepHolder>, {
    let got = Truck::try_from(&step_roundtrip::<Truck, StepHolder>(&t)).unwrap();
    assert_eq!(t, got);
}

fn itest_tryfrom<Truck, StepHolder>(t: Truck, step: &str)
where
    Truck: for<'a> TryFrom<&'a StepHolder::Owned> + Debug + PartialEq,
    for<'a> <Truck as TryFrom<&'a StepHolder::Owned>>::Error: Debug,
    StepHolder: Holder<Table = Table>,
    Table: EntityTable<StepHolder>, {
    let data_section = DataSection::from_str(step).unwrap();
    let table = Table::from_data_section(&data_section);
    let step_data: StepHolder::Owned = EntityTable::get_owned(&table, 1).unwrap();
    let got = Truck::try_from(&step_data).unwrap();
    assert_eq!(t, got);
}

#[test]
fn oi() {
    oitest::<Point2, CartesianPointHolder>(Point2::new(1.0, 2.0));
    oitest::<Point3, CartesianPointHolder>(Point3::new(1.0, 2.0, 3.0));
    oitest::<Vector2, VectorHolder>(Vector2::new(1.0, 2.0));
    oitest::<Vector3, VectorHolder>(Vector3::new(1.0, 2.0, 3.0));
    oitest::<truck::Line<Point2>, LineHolder>(truck::Line(
        Point2::new(0.0, 1.0),
        Point2::new(2.0, 3.0),
    ));
    oitest::<PolylineCurve<Point2>, PolylineHolder>(PolylineCurve(vec![
        Point2::origin(),
        Point2::new(1.0, 2.0),
        Point2::new(3.0, 4.0),
    ]));
    oitest_tryfrom::<BsplineCurve<Point2>, BsplineCurveWithKnotsHolder>(BsplineCurve::new(
        KnotVector::uniform_knot(3, 4),
        vec![
            Point2::new(1.0, 2.0),
            Point2::new(3.0, 4.0),
            Point2::new(5.0, 6.0),
            Point2::new(7.0, 8.0),
            Point2::new(9.0, 10.0),
            Point2::new(11.0, 12.0),
            Point2::new(13.0, 14.0),
            Point2::new(15.0, 16.0),
        ],
    ));
    itest_tryfrom::<BsplineCurve<Point2>, BezierCurveHolder>(
        BsplineCurve::new(
            KnotVector::bezier_knot(2),
            vec![
                Point2::new(0.0, 1.0),
                Point2::new(2.0, 3.0),
                Point2::new(4.0, 5.0),
            ],
        ),
        "DATA; #1 = BEZIER_CURVE('', 2, (#2, #3, #4), .UNSPECIFIED., .U., .U.);
#2 = CARTESIAN_POINT('', (0.0, 1.0)); #3 = CARTESIAN_POINT('', (2.0, 3.0));
#4 = CARTESIAN_POINT('', (4.0, 5.0)); ENDSEC;",
    );
    itest_tryfrom::<BsplineCurve<Point2>, QuasiUniformCurveHolder>(
        BsplineCurve::new(
            KnotVector::from(vec![0.0, 0.0, 0.0, 1.0, 2.0, 2.0, 2.0]),
            vec![
                Point2::new(0.0, 1.0),
                Point2::new(2.0, 3.0),
                Point2::new(4.0, 5.0),
                Point2::new(6.0, 7.0),
            ],
        ),
        "DATA; #1 = QUASI_UNIFORM_CURVE('', 2, (#2, #3, #4, #5), .UNSPECIFIED., .U., .U.);
#2 = CARTESIAN_POINT('', (0.0, 1.0)); #3 = CARTESIAN_POINT('', (2.0, 3.0));
#4 = CARTESIAN_POINT('', (4.0, 5.0)); #5 = CARTESIAN_POINT('', (6.0, 7.0)); ENDSEC;",
    );
    oitest_tryfrom::<NurbsCurve<Vector3>, RationalBsplineCurveHolder>(NurbsCurve::new(
        BsplineCurve::new(
            KnotVector::bezier_knot(3),
            vec![
                Vector3::new(0.0, 1.0, 2.0),
                Vector3::new(3.0, 4.0, 5.0),
                Vector3::new(6.0, 7.0, 8.0),
            ],
        ),
    ));
    oitest_tryfrom::<step_geometry::Ellipse<Point2, Matrix3>, CircleHolder>(
        Processor::new(TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI))).transformed(
            Matrix3::from_cols(
                Vector3::new(0.0, 3.0, 0.0),
                Vector3::new(-3.0, 0.0, 0.0),
                Vector3::new(1.0, 2.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Ellipse<Point2, Matrix3>, EllipseHolder>(
        Processor::new(TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI))).transformed(
            Matrix3::from_cols(
                Vector3::new(0.0, 3.0, 0.0),
                Vector3::new(-8.0, 0.0, 0.0),
                Vector3::new(1.0, 2.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Ellipse<Point3, Matrix4>, CircleHolder>(
        Processor::new(TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI))).transformed(
            Matrix4::from_cols(
                Vector4::new(0.0, 3.0, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 3.0, 0.0),
                Vector4::new(3.0, 0.0, 0.0, 0.0),
                Vector4::new(1.0, 2.0, 3.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Ellipse<Point3, Matrix4>, EllipseHolder>(
        Processor::new(TrimmedCurve::new(UnitCircle::new(), (0.0, 2.0 * PI))).transformed(
            Matrix4::from_cols(
                Vector4::new(0.0, 3.0, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 8.0, 0.0),
                Vector4::new(3.0, 0.0, 0.0, 0.0),
                Vector4::new(1.0, 2.0, 3.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Hyperbola<Point2, Matrix3>, HyperbolaHolder>(
        Processor::new(TrimmedCurve::new(UnitHyperbola::new(), (-1.0, 1.0))).transformed(
            Matrix3::from_cols(
                Vector3::new(0.0, 3.0, 0.0),
                Vector3::new(-8.0, 0.0, 0.0),
                Vector3::new(1.0, 2.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Hyperbola<Point3, Matrix4>, HyperbolaHolder>(
        Processor::new(TrimmedCurve::new(UnitHyperbola::new(), (-1.0, 1.0))).transformed(
            Matrix4::from_cols(
                Vector4::new(0.0, 3.0, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 8.0, 0.0),
                Vector4::new(3.0, 0.0, 0.0, 0.0),
                Vector4::new(1.0, 2.0, 3.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Parabola<Point2, Matrix3>, ParabolaHolder>(
        Processor::new(TrimmedCurve::new(UnitParabola::new(), (-1.0, 1.0))).transformed(
            Matrix3::from_cols(
                Vector3::new(0.0, 2.0, 0.0),
                Vector3::new(-2.0, 0.0, 0.0),
                Vector3::new(1.0, 2.0, 1.0),
            ),
        ),
    );
    oitest_tryfrom::<step_geometry::Parabola<Point3, Matrix4>, ParabolaHolder>(
        Processor::new(TrimmedCurve::new(UnitParabola::new(), (-1.0, 1.0))).transformed(
            Matrix4::from_cols(
                Vector4::new(0.0, 3.0, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 3.0, 0.0),
                Vector4::new(3.0, 0.0, 0.0, 0.0),
                Vector4::new(1.0, 2.0, 3.0, 1.0),
            ),
        ),
    );
    let plane = truck::Plane::new(
        Point3::new(1.0, 2.0, 3.0),
        // The ISO regulations require that the coordinate axes of the Plane must be vertical;
        // on the truck side, the axes do not have to be vertical,
        // so the results will not necessarily match.
        Point3::new(2.0, 2.0, 3.0),
        Point3::new(1.0, 3.0, 3.0),
    );
    let got = truck::Plane::from(&step_roundtrip::<_, PlaneHolder>(&plane));
    // Retained transforms may differ while the complete affine parameterization is identical.
    assert_eq!(
        (plane.origin(), plane.axis_u(), plane.axis_v()),
        (got.origin(), got.axis_u(), got.axis_v()),
    );
    oitest::<Processor<step_geometry::Sphere, Matrix4>, SphericalSurfaceHolder>(
        Processor::new(step_geometry::Sphere(truck::Sphere::new(
            Point3::new(0.0, 0.0, 0.0),
            15.0,
        )))
        .transformed(Matrix4::from_cols(
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(-1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(1.0, 2.0, 3.0, 1.0),
        )),
    );
    // `TryFrom` since spec 011 T1: the STEP -> geometry torus conversion refuses
    // the degenerate self-intersecting regime instead of panicking in
    // `Torus::new`. A ring torus (15 > 9) round-trips exactly as before.
    oitest_tryfrom::<Processor<Torus, Matrix4>, ToroidalSurfaceHolder>(
        Processor::new(Torus::new(Point3::new(0.0, 0.0, 0.0), 15.0, 9.0)).transformed(
            Matrix4::from_cols(
                Vector4::new(0.0, 1.0, 0.0, 0.0),
                Vector4::new(-1.0, 0.0, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 1.0, 0.0),
                Vector4::new(1.0, 2.0, 3.0, 1.0),
            ),
        ),
    );
    let pcurve = ParameterCurve::new(
        Box::new(step_geometry::Curve2D::Line(Line(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 1.0),
        ))),
        Box::new(step_geometry::Surface::ElementarySurface(
            step_geometry::ElementarySurface::Plane(truck::Plane::new(
                Point3::new(1.0, 2.0, 3.0),
                Point3::new(1.0, 2.0, 4.0),
                Point3::new(1.0, 1.0, 3.0),
            )),
        )),
    );
    let got = step_geometry::StepParameterCurve::try_from(
        &step_roundtrip::<_, PcurveHolder>(&pcurve),
    ).unwrap();
    assert_eq!(pcurve.curve(), got.curve());
    assert!(matches!(
        got.surface().as_ref(),
        step_geometry::Surface::ElementarySurface(step_geometry::ElementarySurface::Plane(_)),
    ));
    // Three affine basis points determine the plane's complete UV mapping.
    for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
        assert_eq!(pcurve.surface().subs(u, v), got.surface().subs(u, v));
    }
    oitest_tryfrom::<BsplineSurface<Point3>, BsplineSurfaceWithKnotsHolder>(BsplineSurface::new(
        (KnotVector::bezier_knot(3), KnotVector::bezier_knot(2)),
        vec![
            vec![
                Point3::new(0.0, 1.0, 2.0),
                Point3::new(3.0, 4.0, 5.0),
                Point3::new(6.0, 7.0, 8.0),
            ],
            vec![
                Point3::new(0.0, 1.0, 2.0),
                Point3::new(3.0, 4.0, 5.0),
                Point3::new(6.0, 7.0, 8.0),
            ],
            vec![
                Point3::new(0.0, 1.0, 2.0),
                Point3::new(3.0, 4.0, 5.0),
                Point3::new(6.0, 7.0, 8.0),
            ],
            vec![
                Point3::new(0.0, 1.0, 2.0),
                Point3::new(3.0, 4.0, 5.0),
                Point3::new(6.0, 7.0, 8.0),
            ],
        ],
    ));
}
