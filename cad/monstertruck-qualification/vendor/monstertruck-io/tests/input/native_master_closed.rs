include!("native_master_closed_refusal.rs");
use monstertruck_io::step::load::step_geometry::{
    BoundedCurve, MetricSpace, ParametricCurve, Point3,
};

#[test]
fn valid_closed_curve3d_master_still_keeps_the_full_circle() {
    let text = MALFORMED_CLOSED_MASTER.replace(".PCURVE_S1.", ".CURVE_3D.");
    let table = Table::from_step(text.trim()).unwrap();
    let edge = table.edge_curve[&8].clone().into_owned(&table).unwrap();
    let curve = edge.parse_curve3d().unwrap();
    let range = curve.range_tuple();
    assert!((range.1 - range.0 - std::f64::consts::TAU).abs() < 1e-12);
    for i in 0..=32 {
        let t = range.0 + (range.1 - range.0) * i as f64 / 32.;
        let p = curve.evaluate(t);
        assert!(p.z.abs() < 1e-12);
        assert!((p.x * p.x + p.y * p.y - 1.).abs() < 1e-12);
    }
    assert!(
        table
            .to_source_bound_shell_checked(&table.shell[&13])
            .is_ok()
    );
}

#[test]
fn a_valid_closed_pcurve_master_cannot_be_replaced_by_its_approximate_leader() {
    let text=MALFORMED_CLOSED_MASTER
        .replace("#3=CARTESIAN_POINT('',(0.,0.,0.));","#3=CARTESIAN_POINT('',(0.,0.,0.00005));")
        .replace("#6=PLANE('',#4);","#6=PLANE('',#24);")
        .replace("#7=SURFACE_CURVE('',#5,(#6),.PCURVE_S1.);","#7=SURFACE_CURVE('',#5,(#31),.PCURVE_S1.);")
        .replace("LENGTH_MEASURE(0.01)","LENGTH_MEASURE(0.00001)")
        .replace("ENDSEC;END-ISO-10303-21;","#24=AXIS2_PLACEMENT_3D('',#25,$,$);#25=CARTESIAN_POINT('',(0.,0.,0.));#26=CARTESIAN_POINT('',(0.,0.));#27=AXIS2_PLACEMENT_2D('',#26,$);#28=CIRCLE('',#27,1.);#29=DEFINITIONAL_REPRESENTATION('',(#28),#30);#30=(GEOMETRIC_REPRESENTATION_CONTEXT(2)PARAMETRIC_REPRESENTATION_CONTEXT()REPRESENTATION_CONTEXT('','2D'));#31=PCURVE('',#6,#29);ENDSEC;END-ISO-10303-21;");
    let table = Table::from_step(text.trim()).unwrap();
    table.entity_report.require_empty().unwrap();
    let raw = table.surface_curve[&7].clone().into_owned(&table).unwrap();
    let actual = Curve3D::try_from(&raw).unwrap();
    let edge = table.edge_curve[&8].clone().into_owned(&table).unwrap();
    let curve = edge.parse_curve3d().unwrap();
    let range = curve.range_tuple();
    assert!((range.1 - range.0 - std::f64::consts::TAU).abs() < 1e-12);
    for i in 0..=32 {
        let t = range.0 + (range.1 - range.0) * i as f64 / 32.;
        let p = curve.evaluate(t);
        assert!(
            p.distance(actual.evaluate(t)) < 1e-12,
            "closed edge must use selected Pc master, t={t}: {p:?}"
        );
        assert!(p.z.abs() < 1e-12);
    }
    assert!(curve.front().distance(Point3::new(1., 0., 0.)) < 1e-12);
    assert!(
        table
            .to_source_bound_shell_checked(&table.shell[&13])
            .is_ok(),
        "a valid exact selected master is admissible under tight bound despite its approximate alternate"
    );
}
