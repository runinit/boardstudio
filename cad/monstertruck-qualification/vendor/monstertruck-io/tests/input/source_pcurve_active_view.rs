use monstertruck_io::step::load::{Table, step_geometry::*, step_p21::tables::IntoOwned};

fn cube() -> Table {
    Table::from_step(include_str!("fixtures/source-pcurve-cube.step")).unwrap()
}

#[test]
fn selected_native_pcurve_keeps_its_full_source_and_uses_the_actual_edge_interval() {
    let table = cube();
    assert_eq!(table.edge_curve.len(), 12);
    for (&id, raw) in &table.edge_curve {
        let edge = raw.clone().into_owned(&table).unwrap();
        let source = Curve3D::try_from(&edge.edge_geometry).unwrap();
        let curve = edge.parse_curve3d().unwrap();
        assert_eq!(
            curve.range_tuple(),
            (0., 10.),
            "edge #{id}: native LINE sampler is not its topological interval"
        );
        let (Curve3D::SurfaceCurve(source), Curve3D::SurfaceCurve(curve)) = (source, curve) else {
            panic!("retain source associations for edge #{id}")
        };
        assert_eq!(curve.leader(), source.leader());
        assert_eq!(curve.associated_geometry(), source.associated_geometry());
        assert_eq!(
            curve.master_representation(),
            SurfaceCurveRepresentation::ParameterCurve0
        );
        let p = Point3::from(&edge.edge_start.vertex_geometry);
        let q = Point3::from(&edge.edge_end.vertex_geometry);
        for i in 0..=10 {
            let t = i as f64;
            assert_eq!(
                curve.evaluate(t),
                source.evaluate(t),
                "selected physical master edge #{id} at {t}"
            );
            assert!(curve.evaluate(t).distance(p + (q - p) * (t / 10.)) < 1e-12);
            assert!(curve.derivative(t).distance((q - p) / 10.) < 1e-12);
        }
        assert!(
            curve
                .front()
                .distance(Point3::from(&edge.edge_start.vertex_geometry))
                < 1e-12
        );
        assert!(
            curve
                .back()
                .distance(Point3::from(&edge.edge_end.vertex_geometry))
                < 1e-12
        );
    }
}

#[test]
fn actual007_pcurve_cube_keeps_all_six_faces_twelve_edges_and_twenty_four_uses() {
    let table = cube();
    let source = table
        .to_source_bound_shell_checked(&table.shell[&16])
        .expect("actual007 selected-PCurve input must retain its complete source topology");
    let shell = source.shell();
    assert_eq!(shell.faces.len(), 6);
    assert_eq!(shell.edges.len(), 12);
    assert_eq!(shell.vertices.len(), 8);
    assert_eq!(
        shell
            .faces
            .iter()
            .flat_map(|f| &f.boundaries)
            .map(Vec::len)
            .sum::<usize>(),
        24
    );
    assert_eq!(source.source_face_ids().len(), 6);
}

#[test]
fn source_normalization_scales_geometry_without_changing_the_scalar_view() {
    use monstertruck_io::step::load::context::normalization::normalize_shell_to_mm;
    let table = cube();
    let source = table
        .to_source_bound_shell_checked(&table.shell[&16])
        .unwrap();
    for factor in [1., 25.4, 1000.] {
        let mut normalized = source.shell().clone();
        normalize_shell_to_mm(&mut normalized, factor);
        for (before, after) in source.shell().edges.iter().zip(&normalized.edges) {
            let (Curve3D::SurfaceCurve(before), Curve3D::SurfaceCurve(after)) =
                (&before.curve, &after.curve)
            else {
                panic!("retain associations")
            };
            assert_eq!(after.active_trim(), Some((0., 10.)));
            assert_eq!(after.range_tuple(), before.range_tuple());
            for t in [0., 0.25, 3.5, 10.] {
                assert!(after.evaluate(t).distance(before.evaluate(t) * factor) < 1e-10);
            }
        }
        if factor == 1. {
            assert!(
                source
                    .connectivity_after_uniform_scale(&normalized, factor)
                    .is_some()
            );
        }
    }
}

#[test]
fn scalar_views_are_checked_and_cut_without_modifying_the_retained_sources() {
    let table = cube();
    let edge = table
        .edge_curve
        .values()
        .next()
        .unwrap()
        .clone()
        .into_owned(&table)
        .unwrap();
    let Curve3D::SurfaceCurve(source) = Curve3D::try_from(&edge.edge_geometry).unwrap() else {
        panic!("surface curve")
    };
    assert_eq!(source.active_trim(), None);
    let old_json = serde_json::to_value(&source).unwrap();
    assert!(old_json.get("active_trim").is_none());
    assert_eq!(
        serde_json::from_value::<SurfaceCurve3D>(old_json).unwrap(),
        source
    );
    for range in [(f64::NAN, 1.), (0., f64::INFINITY), (2., 1.)] {
        assert!(source.clone().try_with_active_trim(Some(range)).is_none());
    }
    let mut left = source
        .clone()
        .try_with_active_trim(Some((-2., 4.)))
        .unwrap();
    let right = left.cut(1.25);
    assert_eq!(left.range_tuple(), (-2., 1.25));
    assert_eq!(right.range_tuple(), (1.25, 4.));
    for part in [&left, &right] {
        assert_eq!(part.leader(), source.leader());
        assert_eq!(part.associated_geometry(), source.associated_geometry());
        assert_eq!(
            serde_json::from_value::<SurfaceCurve3D>(serde_json::to_value(part).unwrap()).unwrap(),
            *part
        );
    }
    let reversed = right.inverse();
    assert_eq!(reversed.range_tuple(), (-3., -0.25));
    assert_eq!(reversed.front(), right.back());
    assert_eq!(reversed.back(), right.front());
    assert_eq!(reversed.inverse(), right);
    assert!(left.try_parameter_division((-3., 0.), 0.01).is_none());

    let finite = SurfaceCurve3D::new(
        SurfaceCurveKind::SurfaceCurve,
        Box::new(Curve3D::BsplineCurve(BsplineCurve::new(
            KnotVector::from(vec![2., 2., 8., 8.]),
            vec![Point3::origin(), Point3::new(1., 0., 0.)],
        ))),
        vec![],
        SurfaceCurveRepresentation::Curve3D,
    );
    assert!(
        finite
            .clone()
            .try_with_active_trim(Some((1., 4.)))
            .is_none()
    );
    assert!(finite.try_with_active_trim(Some((3., 4.))).is_some());
}

#[test]
fn imported_curve3d_master_is_not_replaced_by_projection_onto_associated_surfaces() {
    let text = "ISO-10303-21;HEADER;FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));ENDSEC;DATA;\
        #1=INTERSECTION_CURVE('',#2,(#6,#10),.CURVE_3D.);\
        #2=LINE('',#3,#4);#3=CARTESIAN_POINT('',(0.,0.,0.00005));\
        #4=VECTOR('',#5,1.);#5=DIRECTION('',(1.,0.,0.));\
        #6=PLANE('',#7);#7=AXIS2_PLACEMENT_3D('',#8,#9,#5);\
        #8=CARTESIAN_POINT('',(0.,0.,0.));#9=DIRECTION('',(0.,0.,1.));\
        #10=PLANE('',#11);#11=AXIS2_PLACEMENT_3D('',#8,#12,#5);\
        #12=DIRECTION('',(0.,1.,0.));\
        #20=CARTESIAN_POINT('',(0.,0.,0.00005));#21=CARTESIAN_POINT('',(1.,0.,0.00005));\
        #22=VERTEX_POINT('',#20);#23=VERTEX_POINT('',#21);#24=EDGE_CURVE('',#22,#23,#1,.T.);\
        ENDSEC;END-ISO-10303-21;";
    let table = Table::from_step(text).unwrap();
    let raw = table.surface_curve[&1].clone().into_owned(&table).unwrap();
    let imported = Curve3D::try_from(&raw).unwrap();
    let edge = table.edge_curve[&24]
        .clone()
        .into_owned(&table)
        .unwrap()
        .parse_curve3d()
        .unwrap();
    for t in [0., 0.25, 0.5, 0.75, 1.] {
        assert_eq!(imported.evaluate(t), Point3::new(t, 0., 0.00005));
        assert_eq!(edge.evaluate(t), imported.evaluate(t));
    }
    let Curve3D::SurfaceCurve(source) = imported else {
        panic!("retain original associations")
    };
    assert_eq!(source.associated_geometry().len(), 2);
}

#[test]
fn a_selected_native_line_with_unretained_direction_is_refused() {
    let text = include_str!("fixtures/source-pcurve-cube.step");
    let authored = "#67 = CARTESIAN_POINT('',(0.,0.));";
    assert_eq!(text.matches(authored).count(), 1);
    let text = text.replace(authored, "#67 = CARTESIAN_POINT('',(1.E20,1.E20));");
    let table = Table::from_step(&text).unwrap();
    let source = table.surface_curve[&59].clone().into_owned(&table).unwrap();
    assert!(
        Curve3D::try_from(&source).is_err(),
        "refuse the lossy working LINE instead of manufacturing a different selected master"
    );
}

#[test]
fn nested_selected_views_reverse_in_the_full_native_source_frame() {
    let table = cube();
    let raw = table.surface_curve[&59].clone().into_owned(&table).unwrap();
    let Curve3D::SurfaceCurve(inner) = Curve3D::try_from(&raw).unwrap() else { panic!("source") };
    let inner = inner.try_with_active_trim(Some((-2., 4.))).unwrap();
    let outer = SurfaceCurve3D::new(SurfaceCurveKind::SurfaceCurve,
        Box::new(Curve3D::SurfaceCurve(inner)), vec![], SurfaceCurveRepresentation::Curve3D)
        .try_with_active_trim(Some((-1., 1.25))).unwrap();
    let reversed = outer.inverse();
    assert_eq!(reversed.range_tuple(), (-0.25, 2.));
    assert_eq!(reversed.front(), outer.back());
    assert_eq!(reversed.back(), outer.front());
    assert_eq!(reversed.inverse(), outer);
}
