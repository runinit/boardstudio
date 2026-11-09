use monstertruck_io::step::load::{
    Table,
    step_geometry::{Surface, re_exports::*},
    step_p21::tables::IntoOwned,
};

// Exact source entities rooted at both complete Choc cone faces, including every trim.
// Source SHA256 0542ee277eeb6988412e0780ae86b23d96fdd118f8242e0bf3224cff7dd7acc4.
const SOURCE: &str = include_str!("fixtures/choc-cones.step");

#[test]
fn public_choc_cone_inverse_reconstructs_source_apices_and_negative_v_trims() {
    let table = Table::from_step(SOURCE).unwrap();
    assert_eq!(table.face_surface.len(), 2);
    assert_eq!(table.edge_curve.len(), 6);
    for (face_id, x) in [(8194, -0.4), (8241, 0.4)] {
        let raw = table.face_surface[&face_id]
            .face_geometry
            .clone()
            .into_owned(&table)
            .unwrap();
        let surface = Surface::try_from(&raw).unwrap();
        for (point, expected_v) in [
            (Point3::new(x, -3.3, 0.2), -0.4),
            (Point3::new(x, -3.1, 0.4), -0.2),
            (Point3::new(x, -2.9, 0.6), 0.0),
        ] {
            let exact = surface
                .search_parameter(point, None, 100)
                .unwrap_or_else(|| panic!("source face {face_id} exact inverse refused {point:?}"));
            let nearest = surface
                .search_nearest_parameter(point, None, 100)
                .unwrap_or_else(|| {
                    panic!("source face {face_id} nearest inverse refused {point:?}")
                });
            for (u, v) in [exact, nearest] {
                let residual = surface.evaluate(u, v).distance(point);
                assert!(u.is_finite() && v.is_finite());
                assert!(residual <= 1e-6, "face={face_id} residual={residual}");
                assert!((v - expected_v).abs() <= 1e-6, "face={face_id} v={v}");
            }
        }
    }
}

#[test]
fn public_source_cone_inverse_preserves_inverted_parameter_hints() {
    let table = Table::from_step(SOURCE).unwrap();
    let raw = table.face_surface[&8194]
        .face_geometry
        .clone()
        .into_owned(&table)
        .unwrap();
    let surface = Surface::try_from(&raw).unwrap();
    let point = Point3::new(-0.4, -3.3, 0.2);
    for inverted in [false, true] {
        let surface = if inverted {
            surface.inverse()
        } else {
            surface.clone()
        };
        let expected = if inverted { (-0.4, 7.3) } else { (7.3, -0.4) };
        for nearest in [false, true] {
            let found = if nearest {
                surface.search_nearest_parameter(point, expected, 100)
            } else {
                surface.search_parameter(point, expected, 100)
            }
            .unwrap();
            assert!(
                (found.0 - expected.0).abs() < 1e-6 && (found.1 - expected.1).abs() < 1e-6,
                "inverted={inverted} nearest={nearest} expected={expected:?} found={found:?}"
            );
            assert!(surface.evaluate(found.0, found.1).distance(point) <= 1e-6);
        }
    }
}

#[test]
fn both_complete_source_cone_trims_remain_on_the_unchanged_surface() {
    let table = Table::from_step(SOURCE).unwrap();
    for (face_id, edge_ids) in [(8194, [8198, 6351, 8207]), (8241, [8221, 6370, 8247])] {
        let raw = table.face_surface[&face_id]
            .face_geometry
            .clone()
            .into_owned(&table)
            .unwrap();
        let surface = Surface::try_from(&raw).unwrap();
        for edge_id in edge_ids {
            let raw = table.edge_curve[&edge_id]
                .clone()
                .into_owned(&table)
                .unwrap();
            let curve = raw.parse_curve3d().unwrap();
            let source_start = Point3::from(&raw.edge_start.vertex_geometry);
            let source_end = Point3::from(&raw.edge_end.vertex_geometry);
            assert!(curve.front().distance(source_start) <= 1e-6);
            assert!(curve.back().distance(source_end) <= 1e-6);
            let points = curve.parameter_division(curve.range_tuple(), 1e-4).1;
            for point in points.into_iter().chain([source_start, source_end]) {
                let (u, v) = surface
                    .search_parameter(point, None, 100)
                    .unwrap_or_else(|| panic!("face={face_id} edge={edge_id} refused {point:?}"));
                assert!(u.is_finite() && v.is_finite());
                assert!(surface.evaluate(u, v).distance(point) <= 1e-6);
            }
        }
    }
}

#[test]
fn source_cone_modeling_route_keeps_apex_reconstruction() {
    let table = Table::from_step(SOURCE).unwrap();
    for (face_id, x) in [(8194, -0.4), (8241, 0.4)] {
        let raw = table.face_surface[&face_id]
            .face_geometry
            .clone()
            .into_owned(&table)
            .unwrap();
        let surface = Surface::try_from(&raw).unwrap();
        let modeling: monstertruck_modeling::Surface = (&surface).try_into().unwrap();
        let point = Point3::new(x, -3.3, 0.2);
        for nearest in [false, true] {
            let (u, v) = if nearest {
                modeling.search_nearest_parameter(point, None, 100)
            } else {
                modeling.search_parameter(point, None, 100)
            }
            .unwrap();
            assert!(u.is_finite() && v.is_finite());
            assert!(modeling.evaluate(u, v).distance(point) <= 1e-6);
        }
    }
}

#[test]
fn source_cone_inverse_preserves_inverted_range_hints() {
    let table = Table::from_step(SOURCE).unwrap();
    let raw = table.face_surface[&8194]
        .face_geometry
        .clone()
        .into_owned(&table)
        .unwrap();
    let surface = Surface::try_from(&raw).unwrap();
    let point = Point3::new(-0.4, -3.3, 0.2);
    for inverted in [false, true] {
        let surface = if inverted {
            surface.inverse()
        } else {
            surface.clone()
        };
        let hint = if inverted {
            SearchParameterHint2D::Range((-0.5, -0.3), (7.0, 8.0))
        } else {
            SearchParameterHint2D::Range((7.0, 8.0), (-0.5, -0.3))
        };
        let expected = if inverted { (-0.4, 7.0) } else { (7.0, -0.4) };
        for nearest in [false, true] {
            let found = if nearest {
                surface.search_nearest_parameter(point, hint, 100)
            } else {
                surface.search_parameter(point, hint, 100)
            }
            .unwrap();
            assert!(
                (found.0 - expected.0).abs() < 1e-6 && (found.1 - expected.1).abs() < 1e-6,
                "inverted={inverted} nearest={nearest} found={found:?}"
            );
            assert!(surface.evaluate(found.0, found.1).distance(point) <= 1e-6);
        }
    }
}
