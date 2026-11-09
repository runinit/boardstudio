use monstertruck_io::step::load::{
    LossReason, Table,
    step_geometry::{Curve3D, re_exports::*},
    step_p21::tables::IntoOwned,
};

fn split_source() -> &'static str {
    "ISO-10303-21; HEADER; FILE_SCHEMA(('AUTOMOTIVE_DESIGN')); ENDSEC; DATA;
        #1=CARTESIAN_POINT('',(0.,0.,0.));
        #2=CARTESIAN_POINT('',(1.,2.,0.));
        #3=CARTESIAN_POINT('',(2.,0.,0.));
        #4=B_SPLINE_CURVE_WITH_KNOTS('',2,(#1,#2,#3),.UNSPECIFIED.,.F.,.F.,(3,3),(0.,1.),.UNSPECIFIED.);
        #5=CARTESIAN_POINT('',(1.,1.,0.));
        #10=VERTEX_POINT('',#1);
        #11=VERTEX_POINT('',#5);
        #12=VERTEX_POINT('',#3);
        #20=EDGE_CURVE('',#10,#11,#4,.T.);
        #21=EDGE_CURVE('',#11,#12,#4,.T.);
        ENDSEC; END-ISO-10303-21;"
}

fn split_edges() -> Table {
    Table::from_step(split_source()).unwrap()
}

fn edge(table: &Table, id: u64) -> Curve3D {
    table.edge_curve[&id]
        .clone()
        .into_owned(table)
        .unwrap()
        .parse_curve3d()
        .unwrap()
}

#[test]
fn bounded_edge_trim_retains_two_distinct_source_intervals() {
    let table = split_edges();
    let first = edge(&table, 20);
    let second = edge(&table, 21);
    assert!(first.back().distance(Point3::new(1.0, 1.0, 0.0)) < 1e-10);
    assert!(second.front().distance(Point3::new(1.0, 1.0, 0.0)) < 1e-10);
    assert!(first.front().distance(Point3::new(0.0, 0.0, 0.0)) < 1e-10);
    assert!(second.back().distance(Point3::new(2.0, 0.0, 0.0)) < 1e-10);
    let (a, b) = first.range_tuple();
    assert!(
        first
            .subs((a + b) / 2.0)
            .distance(Point3::new(0.5, 0.75, 0.0))
            < 1e-10
    );
}

fn rational_source() -> String {
    split_source().replace(
        "#4=B_SPLINE_CURVE_WITH_KNOTS('',2,(#1,#2,#3),.UNSPECIFIED.,.F.,.F.,(3,3),(0.,1.),.UNSPECIFIED.);",
        "#4=(BOUNDED_CURVE() B_SPLINE_CURVE(2,(#1,#2,#3),.UNSPECIFIED.,.F.,.F.)
        B_SPLINE_CURVE_WITH_KNOTS((3,3),(0.,1.),.UNSPECIFIED.) CURVE()
        GEOMETRIC_REPRESENTATION_ITEM() RATIONAL_B_SPLINE_CURVE((1.,2.,1.)) REPRESENTATION_ITEM(''));",
    ).replace("(1.,1.,0.)", "(1.,1.333333333333333333,0.)")
}

#[test]
fn bounded_edge_trim_preserves_rational_geometry() {
    let table = Table::from_step(&rational_source()).unwrap();
    let first = edge(&table, 20);
    let second = edge(&table, 21);
    assert!(first.back().distance(Point3::new(1.0, 4.0 / 3.0, 0.0)) < 1e-10);
    assert!(second.front().distance(Point3::new(1.0, 4.0 / 3.0, 0.0)) < 1e-10);
    let (a, b) = first.range_tuple();
    assert!(
        first
            .subs((a + b) / 2.0)
            .distance(Point3::new(7.0 / 11.0, 12.0 / 11.0, 0.0))
            < 1e-10
    );
}

#[test]
fn bounded_edge_trim_retains_narrow_source_domain_intervals() {
    let table = Table::from_step(&split_source().replace("(0.,1.)", "(3.,3.00000008)")).unwrap();
    let first = edge(&table, 20);
    assert!(first.back().distance(Point3::new(1.0, 1.0, 0.0)) < 1e-8);
    let (a, b) = first.range_tuple();
    assert!(
        first
            .subs((a + b) / 2.0)
            .distance(Point3::new(0.5, 0.75, 0.0))
            < 1e-8
    );
}

#[test]
fn bounded_edge_trim_preserves_full_curve_and_geometry_sense() {
    let table = Table::from_step(&split_source().replace(
        "#20=EDGE_CURVE('',#10,#11,#4,.T.);",
        "#20=EDGE_CURVE('',#10,#12,#4,.T.);",
    ))
    .unwrap();
    let raw = table.edge_curve[&20].clone().into_owned(&table).unwrap();
    assert_eq!(
        raw.parse_curve3d().unwrap(),
        Curve3D::try_from(&raw.edge_geometry).unwrap()
    );
    let table = Table::from_step(&split_source().replace(
        "#20=EDGE_CURVE('',#10,#11,#4,.T.);",
        "#20=EDGE_CURVE('',#11,#10,#4,.F.);",
    ))
    .unwrap();
    let reversed = edge(&table, 20);
    assert!(reversed.front().distance(Point3::new(1., 1., 0.)) < 1e-10);
    assert!(reversed.back().distance(Point3::new(0., 0., 0.)) < 1e-10);
}

#[test]
fn bounded_edge_trim_refuses_ambiguous_off_curve_and_wrong_sense_vertices() {
    let cases = [
        split_source().replace("(1.,1.,0.)", "(1.,1.,0.01)"),
        split_source()
            .replace("(2.,0.,0.)", "(0.,0.,0.)")
            .replace("(1.,1.,0.)", "(0.375,0.75,0.)"),
        split_source().replace(
            "#20=EDGE_CURVE('',#10,#11,#4,.T.);",
            "#20=EDGE_CURVE('',#10,#11,#4,.F.);",
        ),
    ];
    for source in cases {
        let table = Table::from_step(&source).unwrap();
        let raw = table.edge_curve[&20].clone().into_owned(&table).unwrap();
        assert!(raw.parse_curve3d().is_err());
    }
}

fn shell_source() -> String {
    split_source().replace(
        "ENDSEC; END-ISO-10303-21;",
        "
        #30=EDGE_LOOP('',(#20,#21));
        #31=FACE_BOUND('',#30,.T.);
        #32=AXIS2_PLACEMENT_3D('',#1,$,$);
        #33=PLANE('',#32);
        #34=ADVANCED_FACE('',(#31),#33,.T.);
        #35=OPEN_SHELL('',(#34));
        ENDSEC; END-ISO-10303-21;",
    )
}

#[test]
fn bounded_edge_trim_refusal_cannot_be_accepted_as_a_lossless_shell() {
    let table = Table::from_step(&shell_source().replace("(1.,1.,0.)", "(1.,1.,0.01)")).unwrap();
    let (_, report) = table
        .to_compressed_shell_reported(&table.shell[&35])
        .unwrap();
    assert!(report.lost_for(LossReason::EdgeCurveRefused) > 0);
    assert!(report.require_lossless().is_err());
}

#[test]
fn bounded_edge_trim_preserves_distinct_source_edges_in_shell_conversion() {
    let table = Table::from_step(&shell_source()).unwrap();
    let (shell, report) = table
        .to_compressed_shell_reported(&table.shell[&35])
        .unwrap();
    report.require_lossless().unwrap();
    assert_eq!(shell.edges.len(), 2);
    assert_eq!(shell.vertices.len(), 3);
    assert_eq!(shell.edges[0].vertices.1, shell.edges[1].vertices.0);
    assert_ne!(shell.edges[0].vertices.0, shell.edges[1].vertices.1);
}

#[test]
fn bounded_edge_trim_does_not_snap_nearby_knots_onto_the_interval() {
    let source = split_source()
        .replace("(1.,2.,0.)", "(0.5,1.,0.)")
        .replace("(#1,#2,#3)", "(#1,#2,#6,#7,#3)")
        .replace("(3,3),(0.,1.)", "(3,2,3),(0.,0.5,1.)")
        .replace("(1.,1.,0.)", "(1.00000002,0.9999999999999996,0.)")
        .replace("#10=VERTEX_POINT", "#6=CARTESIAN_POINT('',(1.,1.,0.)); #7=CARTESIAN_POINT('',(1.5,1.,0.)); #10=VERTEX_POINT");
    let table = Table::from_step(&source).unwrap();
    let first = edge(&table, 20);
    let second = edge(&table, 21);
    let endpoint = Point3::new(1.00000002, 0.9999999999999996, 0.);
    assert!(first.back().distance(endpoint) < 1e-13);
    assert!(second.front().distance(endpoint) < 1e-13);
}

#[test]
fn bounded_edge_trim_keeps_a_full_closed_curve() {
    let source = split_source().replace("(2.,0.,0.)", "(0.,0.,0.)").replace(
        "#20=EDGE_CURVE('',#10,#11,#4,.T.);",
        "#20=EDGE_CURVE('',#10,#12,#4,.T.);",
    );
    let table = Table::from_step(&source).unwrap();
    let raw = table.edge_curve[&20].clone().into_owned(&table).unwrap();
    assert_eq!(
        raw.parse_curve3d().unwrap(),
        Curve3D::try_from(&raw.edge_geometry).unwrap()
    );
}

#[test]
fn bounded_edge_trim_refuses_nonpositive_and_nonfinite_rational_weights() {
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let table = Table::from_step(&rational_source()).unwrap();
        let mut raw = table.edge_curve[&20].clone().into_owned(&table).unwrap();
        raw.edge_end = table.edge_curve[&21]
            .clone()
            .into_owned(&table)
            .unwrap()
            .edge_end;
        use monstertruck_io::step::load::{BoundedCurveAny, BsplineCurveAny, CurveAny};
        let CurveAny::BoundedCurve(bounded) = &mut raw.edge_geometry else {
            unreachable!()
        };
        let BoundedCurveAny::BsplineCurve(spline) = bounded.as_mut() else {
            unreachable!()
        };
        let BsplineCurveAny::RationalBsplineCurve(spline) = spline.as_mut() else {
            unreachable!()
        };
        spline.weights_data[1] = weight;
        assert!(raw.parse_curve3d().is_err(), "weight={weight}");
    }
}

#[test]
fn bounded_edge_trim_supports_a_narrow_partial_interval_without_panicking() {
    let source = split_source()
        .replace("#10=VERTEX_POINT", "#6=CARTESIAN_POINT('',(1.00000002,0.9999999999999996,0.)); #13=VERTEX_POINT('',#6); #10=VERTEX_POINT")
        .replace("#20=EDGE_CURVE('',#10,#11,#4,.T.);", "#20=EDGE_CURVE('',#11,#13,#4,.T.);");
    let table = Table::from_step(&source).unwrap();
    let curve = edge(&table, 20);
    assert!(curve.front().distance(Point3::new(1., 1., 0.)) < 1e-13);
    assert!(
        curve
            .back()
            .distance(Point3::new(1.00000002, 0.9999999999999996, 0.))
            < 1e-13
    );
    assert!(
        curve
            .subs(0.5)
            .distance(Point3::new(1.00000001, 0.9999999999999999, 0.))
            < 1e-13
    );
}
