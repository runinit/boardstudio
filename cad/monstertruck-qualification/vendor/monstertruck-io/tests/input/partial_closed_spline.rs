use monstertruck_io::step::load::{
    Table,
    report::{ConnectivityIncidence, LossCategory},
    step_geometry::{
        Curve3D,
        re_exports::{BoundedCurve, BsplineCurve, MetricSpace, ParametricCurve, Point3},
    },
    step_p21::tables::IntoOwned,
};

const SOURCE: &str = include_str!("partial_closed_spline/solid32377-source.step");
fn table() -> Table {
    let table = Table::from_step(SOURCE).unwrap();
    table.entity_report.require_empty().unwrap();
    table
}
fn basis(table: &Table, id: u64) -> BsplineCurve<Point3> {
    let source = table.b_spline_curve_with_knots[&id]
        .clone()
        .into_owned(table)
        .unwrap();
    (&source).try_into().unwrap()
}
#[test]
fn parsed_source_coefficients_expose_only_the_finite_endpoint_candidates() {
    let table = table();
    for curve_id in [32104, 32204] {
        let curve = basis(&table, curve_id);
        assert_eq!(curve.degree(), 3);
        assert_eq!(curve.range_tuple(), (0.0, 1.0));
        let controls = curve.control_points();
        assert_eq!(controls.len(), 97);
        assert_eq!(controls[0], controls[96]);
        assert!(controls.iter().all(|p| p.z == controls[0].z));
        let (lo, hi) = (controls[48].x, controls[0].x);
        assert!(controls.iter().all(|p| lo <= p.x && p.x <= hi));
        for span in 0..32 {
            let segment = &controls[span * 3..=span * 3 + 3];
            assert!(segment.iter().any(|p| p.x < hi));
            assert!(segment.iter().any(|p| p.x > lo));
        }
        let seam_knots: Vec<_> = (0..=32)
            .filter(|i| controls[*i * 3] == controls[0])
            .collect();
        let midpoint_knots: Vec<_> = (0..=32)
            .filter(|i| controls[*i * 3] == controls[48])
            .collect();
        assert_eq!(seam_knots, [0, 32]);
        assert_eq!(midpoint_knots, [16]);
        assert_eq!(curve.evaluate(0.0), controls[0]);
        assert_eq!(curve.evaluate(0.5), controls[48]);
        assert_eq!(curve.evaluate(1.0), controls[96]);
        println!("SOURCE curve={curve_id} degree=3 controls=97 spans=32 seam=[0,1] midpoint=[0.5]");
    }
    let edge = table.edge_curve[&32105].clone().into_owned(&table).unwrap();
    let source_vertex = Point3::from(&edge.edge_start.vertex_geometry);
    let anchor = basis(&table, 32104).front();
    assert_eq!(source_vertex.x, anchor.x);
    assert_eq!(source_vertex.y, anchor.y);
    assert_ne!(source_vertex.z, anchor.z);
    let discrepancy = source_vertex.distance(anchor);
    assert!(discrepancy > 1e-5 && discrepancy < 0.001);
    println!(
        "SOURCE vertex32006={source_vertex:?} seam_anchor={anchor:?} discrepancy_mm={discrepancy:.17e}"
    );
}
#[test]
fn the_unmodified_source_solid_requires_both_oriented_half_views() {
    let table = table();
    let (_, report) = table
        .to_compressed_shell_in_representation_reported(&table.shell[&32376], 17)
        .unwrap();
    println!(
        "DIAGNOSTIC lossless={} edges_lost={} edge_uses_lost={} report={report}",
        report.is_lossless(),
        report.lost(LossCategory::Edge),
        report.lost(LossCategory::EdgeUse)
    );
    let source = table
        .to_source_bound_solid_in_representation_checked(&table.manifold_solid_brep[&32377], 17)
        .expect(
            "source coefficients prove one finite interval for each partial closed B-spline edge",
        );
    assert_eq!(source.shells().len(), 1);
    let shell = &source.shells()[0];
    assert_eq!(shell.source_face_ids().len(), 12);
    assert_eq!(shell.shell().edges.len(), 26);
    for (edge_id, curve_id, interval) in [(32105, 32104, (0.0, 0.5)), (32205, 32204, (0.5, 1.0))] {
        let attachment = shell
            .connectivity_attachments()
            .iter()
            .find(|a| {
                a.edge_id() == edge_id && a.incidence() == ConnectivityIncidence::VertexOnEdge
            })
            .unwrap();
        assert_eq!(attachment.accuracy().representation_id(), 17);
        assert_eq!(attachment.accuracy().measure_id(), 9);
        assert_eq!(attachment.accuracy().bound(), 0.001);
        let view = &shell.shell().edges[attachment.edge_index()].curve;
        assert_eq!(view.range_tuple(), interval);
        let original = basis(&table, curve_id);
        for j in 0..=16 {
            let t = interval.0 + (interval.1 - interval.0) * (j as f64 / 16.0);
            assert_eq!(view.evaluate(t), original.evaluate(t));
        }
    }
    let raw = table.edge_curve[&32105].clone().into_owned(&table).unwrap();
    let vertex = Point3::from(&raw.edge_start.vertex_geometry);
    assert!(shell.shell().vertices.contains(&vertex));
    let a = shell
        .connectivity_attachments()
        .iter()
        .find(|a| {
            a.edge_id() == 32105
                && a.incidence() == ConnectivityIncidence::VertexOnEdge
                && !a.is_back_endpoint()
        })
        .unwrap();
    assert_ne!(
        shell.shell().edges[a.edge_index()].curve.front(),
        vertex,
        "preserve the curve anchor and original off-curve vertex separately"
    );
}
#[test]
fn exact_vertex_control_still_requires_a_half_of_the_closed_basis() {
    let table = table();
    let source = table.b_spline_curve_with_knots[&32104]
        .clone()
        .into_owned(&table)
        .unwrap();
    let mut edge = table.edge_curve[&32105].clone().into_owned(&table).unwrap();
    edge.edge_start.vertex_geometry = source.control_points_list[0].clone();
    let curve = edge.parse_curve3d().expect("exact source endpoint and interior knot must select the first finite half without any uncertainty allowance");
    assert_eq!(curve.range_tuple(), (0.0, 0.5));
    assert_eq!(curve.front(), basis(&table, 32104).front());
    assert_eq!(curve.back(), basis(&table, 32104).evaluate(0.5));
}
#[test]
fn full_closed_span_remains_the_existing_supported_control() {
    let table = table();
    let source = table.b_spline_curve_with_knots[&32104]
        .clone()
        .into_owned(&table)
        .unwrap();
    let mut edge = table.edge_curve[&32105].clone().into_owned(&table).unwrap();
    edge.edge_start.vertex_geometry = source.control_points_list[0].clone();
    edge.edge_end = edge.edge_start.clone();
    let curve = edge.parse_curve3d().unwrap();
    assert!(matches!(curve, Curve3D::BsplineCurve(_)));
    assert_eq!(curve.range_tuple(), (0.0, 1.0));
    assert_eq!(curve.front(), curve.back());
    assert_ne!(curve.evaluate(0.25), curve.front());
}

// Two polynomial Bezier arcs with exact seam A=(1,0,0), join B=(-1,0,0).
// The coefficient extrema, rather than a sampled circle assumption, identify their aliases.
fn synthetic_source(
    controls: &[[f64; 3]],
    knots: &[f64],
    multiplicities: &[usize],
    p: [f64; 3],
    q: [f64; 3],
    sense: bool,
    rational: bool,
) -> String {
    use std::fmt::Write;
    let mut source =
        String::from("ISO-10303-21; HEADER; FILE_SCHEMA(('AUTOMOTIVE_DESIGN')); ENDSEC; DATA;\n");
    for (i, c) in controls.iter().enumerate() {
        writeln!(
            &mut source,
            "#{}=CARTESIAN_POINT('',({:.17e},{:.17e},{:.17e}));",
            100 + i,
            c[0],
            c[1],
            c[2]
        )
        .unwrap();
    }
    let references = (0..controls.len())
        .map(|i| format!("#{}", 100 + i))
        .collect::<Vec<_>>()
        .join(",");
    let mults = multiplicities
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let knots = knots
        .iter()
        .map(|t| format!("{t:.17e}"))
        .collect::<Vec<_>>()
        .join(",");
    if rational {
        let weights = vec!["1."; controls.len()].join(",");
        writeln!(&mut source, "#4=(BOUNDED_CURVE() B_SPLINE_CURVE(2,({references}),.UNSPECIFIED.,.T.,.F.) B_SPLINE_CURVE_WITH_KNOTS(({mults}),({knots}),.UNSPECIFIED.) CURVE() GEOMETRIC_REPRESENTATION_ITEM() RATIONAL_B_SPLINE_CURVE(({weights})) REPRESENTATION_ITEM(''));").unwrap();
    } else {
        writeln!(&mut source, "#4=B_SPLINE_CURVE_WITH_KNOTS('',2,({references}),.UNSPECIFIED.,.T.,.F.,({mults}),({knots}),.UNSPECIFIED.);").unwrap();
    }
    for (id, c) in [(1, p), (2, q)] {
        writeln!(
            &mut source,
            "#{id}=CARTESIAN_POINT('',({:.17e},{:.17e},{:.17e}));",
            c[0], c[1], c[2]
        )
        .unwrap();
    }
    writeln!(&mut source,"#10=VERTEX_POINT('',#1); #11=VERTEX_POINT('',#2); #20=EDGE_CURVE('',#10,#11,#4,.{}.); ENDSEC; END-ISO-10303-21;",if sense { "T" } else { "F" }).unwrap();
    source.replace('e', "E")
}
const ARCS: [[f64; 3]; 5] = [
    [1., 0., 0.],
    [1., 1., 0.],
    [-1., 0., 0.],
    [-1., -1., 0.],
    [1., 0., 0.],
];
fn parse_synthetic(
    source: &str,
) -> Result<Curve3D, monstertruck_io::step::load::step_geometry::StepConvertingError> {
    let table = Table::from_step(source).unwrap();
    table.entity_report.require_empty().unwrap();
    table.edge_curve[&20]
        .clone()
        .into_owned(&table)
        .unwrap()
        .parse_curve3d()
}
#[test]
fn join_intervals_preserve_native_labels_sense_and_axis_permutations() {
    for axis in 0..3 {
        let map = |point: [f64; 3]| {
            let mut p = [0.; 3];
            for j in 0..3 {
                p[(j + axis) % 3] = point[j];
            }
            p
        };
        let controls = ARCS.map(map);
        for (knots, first, second) in [
            ([0., 0.5, 1.], (0., 0.5), (0.5, 1.)),
            ([3., 4., 5.], (3., 4.), (4., 5.)),
            (
                [0., 1.0 / 131072.0, 1.0 / 65536.0],
                (0., 1.0 / 131072.0),
                (1.0 / 131072.0, 1.0 / 65536.0),
            ),
        ] {
            for (p, q, interval) in [
                (map(ARCS[0]), map(ARCS[2]), first),
                (map(ARCS[2]), map(ARCS[0]), second),
            ] {
                let source = synthetic_source(&controls, &knots, &[3, 2, 3], p, q, true, false);
                let curve = parse_synthetic(&source).unwrap();
                assert_eq!(curve.range_tuple(), interval);
                let Curve3D::BsplineCurve(spline) = &curve else {
                    panic!("polynomial source changed kind")
                };
                let expected = if p == controls[0] {
                    &controls[..3]
                } else {
                    &controls[2..]
                };
                assert_eq!(
                    spline.control_points(),
                    &expected
                        .iter()
                        .copied()
                        .map(Point3::from)
                        .collect::<Vec<_>>()
                );
                // Scaled-knot basis arithmetic can round endpoint evaluation by a few ulps.
                // The source coefficients and native interval above remain exact.
                assert!(curve.front().distance(Point3::from(p)) < 1e-14);
                assert!(curve.back().distance(Point3::from(q)) < 1e-14);
                let reverse = parse_synthetic(&synthetic_source(
                    &controls,
                    &knots,
                    &[3, 2, 3],
                    q,
                    p,
                    false,
                    false,
                ))
                .unwrap();
                assert_eq!(reverse.range_tuple(), interval);
                assert!(reverse.front().distance(curve.back()) < 1e-14);
                assert!(reverse.back().distance(curve.front()) < 1e-14);
                let t = interval.0 + (interval.1 - interval.0) * 0.25;
                let reflected = interval.0 + (interval.1 - t);
                assert!(reverse.evaluate(t).distance(curve.evaluate(reflected)) < 1e-12);
            }
        }
    }
}
#[test]
fn all_join_aliases_are_counted_before_selecting_an_interval() {
    let double = [ARCS.as_slice(), &ARCS[1..]].concat();
    let result = parse_synthetic(&synthetic_source(
        &double,
        &[0., 0.25, 0.5, 0.75, 1.],
        &[3, 2, 2, 2, 3],
        ARCS[0],
        ARCS[2],
        true,
        false,
    ));
    assert!(
        result.is_err(),
        "three increasing seam-to-opposite pairs are ambiguous"
    );
}
#[test]
fn flat_support_spans_and_unexcluded_interior_aliases_are_refused() {
    // A lies at an interior point of the second span as well as the seam.
    // Neither of that flat-X span's endpoint joins equals A, so alias counting alone
    // would incorrectly leave one increasing pair to B at t=.75.
    let flat = [
        [1., 0., 0.],
        [0., 1., 0.],
        [1., 1., 0.],
        [1., 0., 0.],
        [1., -1., 0.],
        [0., -1., 0.],
        [-1., 0., 0.],
        [0., -1., 0.],
        [1., 0., 0.],
    ];
    assert!(
        parse_synthetic(&synthetic_source(
            &flat,
            &[0., 0.25, 0.5, 0.75, 1.],
            &[3, 2, 2, 2, 3],
            ARCS[0],
            ARCS[2],
            true,
            false
        ))
        .is_err()
    );
    // The second straight span crosses A in its interior; join-only enumeration is insufficient.
    let crossing = [
        [0., 0., 0.],
        [-1., 0., 0.],
        [-2., 0., 0.],
        [0., 0., 0.],
        [2., 0., 0.],
        [2., 1., 0.],
        [0., 0., 0.],
    ];
    assert!(
        parse_synthetic(&synthetic_source(
            &crossing,
            &[0., 0.25, 0.5, 1.],
            &[3, 2, 2, 3],
            crossing[0],
            crossing[4],
            true,
            false
        ))
        .is_err()
    );
}
#[test]
fn nearby_off_curve_or_nonjoin_vertices_do_not_become_parameter_authority() {
    for q in [[-1. + 1e-12, 0., 0.], [-1., 0., 2e-6], [0.5, 0.5, 0.]] {
        assert!(
            parse_synthetic(&synthetic_source(
                &ARCS,
                &[0., 0.5, 1.],
                &[3, 2, 3],
                ARCS[0],
                q,
                true,
                false
            ))
            .is_err(),
            "q={q:?}"
        );
    }
    // A nonjoin curve point is exactly on the first arc at t=.25, but has no proved join inverse.
    assert!(
        parse_synthetic(&synthetic_source(
            &ARCS,
            &[0., 0.5, 1.],
            &[3, 2, 3],
            ARCS[0],
            [0.5, 0.5, 0.],
            true,
            false
        ))
        .is_err()
    );
}
#[test]
fn polynomial_proof_does_not_extend_to_rational_or_nonbezier_bases() {
    assert!(
        parse_synthetic(&synthetic_source(
            &ARCS,
            &[0., 0.5, 1.],
            &[3, 2, 3],
            ARCS[0],
            ARCS[2],
            true,
            true
        ))
        .is_err()
    );
    assert!(
        parse_synthetic(&synthetic_source(
            &ARCS,
            &[0., 0.25, 0.75, 1.],
            &[3, 1, 1, 3],
            ARCS[0],
            ARCS[2],
            true,
            false
        ))
        .is_err()
    );
    assert!(
        parse_synthetic(&synthetic_source(
            &ARCS,
            &[0., 0.5, 1.],
            &[3, 3, 3],
            ARCS[0],
            ARCS[2],
            true,
            false
        ))
        .is_err()
    );
}
#[test]
fn relabeled_narrow_source_and_new_narrow_join_interval_remain_unproved() {
    assert!(
        parse_synthetic(&synthetic_source(
            &ARCS,
            &[3., 3.00000004, 3.00000008],
            &[3, 2, 3],
            ARCS[0],
            ARCS[2],
            true,
            false
        ))
        .is_err()
    );
    assert!(
        parse_synthetic(&synthetic_source(
            &ARCS,
            &[0., 1e-8, 1.],
            &[3, 2, 3],
            ARCS[0],
            ARCS[2],
            true,
            false
        ))
        .is_err()
    );
    // Preserve the pre-existing narrow FULL-domain path, without claiming original labels.
    let full = parse_synthetic(&synthetic_source(
        &ARCS,
        &[3., 3.00000004, 3.00000008],
        &[3, 2, 3],
        ARCS[0],
        ARCS[0],
        true,
        false,
    ))
    .unwrap();
    assert_eq!(full.range_tuple(), (0., 1.));
    assert_eq!(full.front(), Point3::from(ARCS[0]));
    assert_eq!(full.back(), full.front());
}

#[test]
fn additional_nonbinary_reverse_refuses_when_original_native_labels_would_move() {
    let source = synthetic_source(
        &ARCS,
        &[0., 5e-6, 1e-5],
        &[3, 2, 3],
        ARCS[0],
        ARCS[2],
        false,
        false,
    );
    let error = parse_synthetic(&source).expect_err("new join admission must not relabel the chosen [5e-6,1e-5] interval during EDGE_CURVE reversal");
    assert!(
        error.to_string().contains("native endpoint labels"),
        "{error}"
    );
    // The identical second half is supported in forward sense without any relabel.
    let forward = parse_synthetic(&synthetic_source(
        &ARCS,
        &[0., 5e-6, 1e-5],
        &[3, 2, 3],
        ARCS[2],
        ARCS[0],
        true,
        false,
    ))
    .unwrap();
    assert_eq!(forward.range_tuple(), (5e-6, 1e-5));
}

#[test]
fn legacy_nonbinary_full_and_monotone_reverse_remain_accepted() {
    // Pin legacy admission only; inherited-reverse-domain-baseline.log separately
    // records its exact-label defect. The new refusal must not alter that old path.
    let full = parse_synthetic(&synthetic_source(
        &ARCS,
        &[5e-6, 7.5e-6, 1e-5],
        &[3, 2, 3],
        ARCS[0],
        ARCS[0],
        false,
        false,
    ))
    .unwrap();
    assert_eq!(
        full.range_tuple(),
        (5.000000000000001e-6, 1.0000000000000003e-5)
    );
    let controls = [[0., 0., 0.], [1., 2., 0.], [2., 0., 0.]];
    let partial = parse_synthetic(&synthetic_source(
        &controls,
        &[5e-6, 1e-5],
        &[3, 3],
        [1., 1., 0.],
        controls[0],
        false,
        false,
    ))
    .unwrap();
    assert!(partial.front().distance(Point3::new(1., 1., 0.)) < 1e-12);
    assert!(partial.back().distance(Point3::new(0., 0., 0.)) < 1e-12);
}

#[test]
fn new_join_admission_requires_consistent_original_source_metadata() {
    let valid = synthetic_source(
        &ARCS,
        &[0., 0.5, 1.],
        &[3, 2, 3],
        ARCS[0],
        ARCS[2],
        true,
        false,
    );
    assert!(parse_synthetic(&valid).is_ok());
    let cases = [
        (
            "wrong-degree",
            valid.replace(
                "B_SPLINE_CURVE_WITH_KNOTS('',2,",
                "B_SPLINE_CURVE_WITH_KNOTS('',3,",
            ),
        ),
        (
            "negative-degree",
            valid.replace(
                "B_SPLINE_CURVE_WITH_KNOTS('',2,",
                "B_SPLINE_CURVE_WITH_KNOTS('',-1,",
            ),
        ),
        (
            "duplicate-written-knots",
            synthetic_source(
                &ARCS,
                &[0., 0.5, 0.5, 1.],
                &[3, 1, 1, 3],
                ARCS[0],
                ARCS[2],
                true,
                false,
            ),
        ),
        (
            "extra-written-knot",
            synthetic_source(
                &ARCS,
                &[0., 0.5, 1., 2.],
                &[3, 2, 3],
                ARCS[0],
                ARCS[2],
                true,
                false,
            ),
        ),
        (
            "extra-written-multiplicity",
            synthetic_source(
                &ARCS,
                &[0., 0.5, 1.],
                &[3, 2, 3, 1],
                ARCS[0],
                ARCS[2],
                true,
                false,
            ),
        ),
        (
            "zero-written-multiplicity",
            synthetic_source(
                &ARCS,
                &[0., 0.25, 0.5, 1.],
                &[3, 0, 2, 3],
                ARCS[0],
                ARCS[2],
                true,
                false,
            ),
        ),
    ];
    let mut accepted = Vec::new();
    for (name, source) in cases {
        match parse_synthetic(&source) {
            Ok(_) => accepted.push(name),
            Err(error) => assert!(
                error.to_string().contains("original source metadata"),
                "{name}: {error}"
            ),
        }
    }
    assert!(
        accepted.is_empty(),
        "new admission accepted invalid written metadata: {accepted:?}"
    );
}
