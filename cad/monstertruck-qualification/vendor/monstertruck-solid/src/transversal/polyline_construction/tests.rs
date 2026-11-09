// BoardStudio adds exact-duplicate and distinct-segment regression controls.
use super::*;

#[test]
fn polyline_walk_marks_closed_topology_and_near_open_endpoints_correctly() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(3.0, 0.0, 0.0);
    let c = Point3::new(0.0, 3.0, 0.0);
    // The final coordinate equals `a`, though supplied as a distinct value.
    let same_coordinate_as_a = Point3::new(0.0, 0.0, 0.0);
    let closed = construct_polylines(&[(a, b), (b, c), (c, same_coordinate_as_a)]);
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].first(), closed[0].last());

    // These endpoints are only 0.2e-6 apart but lie on opposite PointIndex
    // cells, so the connected component remains an open chain.
    let near_start = Point3::new(0.9999999e-6, 0.0, 0.0);
    let far_mid = Point3::new(5.0e-6, 0.0, 0.0);
    let near_end = Point3::new(1.2e-6, 0.0, 0.0);
    assert!(near_start.near(&near_end));
    let open = construct_polylines(&[(near_start, far_mid), (far_mid, near_end)]);
    assert_eq!(open.len(), 1);
    assert_ne!(open[0].first(), open[0].last());
}

#[test]
fn construct_polylines_positive0() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 1.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 1);
    assert_eq!(polyline[0].len(), 9);

    let mut sign = None;
    for line in polyline[0].windows(2) {
        let a = line[0][0] + line[0][1] * 2.0 + line[0][2] * 4.0;
        let b = line[1][0] + line[1][1] * 2.0 + line[1][2] * 4.0;
        let x = b - a;
        assert!(f64::abs(x) == 1.0 || f64::abs(x) == 7.0);
        let s = f64::signum(x * (x - 2.0) * (x + 2.0));
        if let Some(sign) = sign {
            assert!(s == sign);
        } else {
            sign = Some(s);
        }
    }
}

#[test]
fn construct_polylines_positive1() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(1.0, 1.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(0.0, 0.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 2);
    assert_eq!(polyline[0].len(), 5);
    assert_eq!(polyline[1].len(), 5);
}

#[test]
fn construct_polylines_positive2() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 1.0), Point3::new(0.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 1);
    assert_eq!(polyline[0].len(), 9);

    let mut sign = None;
    for line in polyline[0].windows(2) {
        let a = line[0][0] + line[0][1] * 2.0 + line[0][2] * 4.0;
        let b = line[1][0] + line[1][1] * 2.0 + line[1][2] * 4.0;
        let x = b - a;
        assert!(f64::abs(x) == 1.0 || f64::abs(x) == 7.0);
        let s = f64::signum(x * (x - 2.0) * (x + 2.0));
        if let Some(sign) = sign {
            assert!(s == sign);
        } else {
            sign = Some(s);
        }
    }
}

#[test]
fn construct_polylines_positive3() {
    let lines = vec![
        (Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)),
        (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(0.0, 0.0, 1.0)),
        (Point3::new(0.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0)),
        (Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 0.0, 1.0)),
        (Point3::new(1.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(0.0, 1.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        (Point3::new(1.0, 0.0, 1.0), Point3::new(0.0, 1.0, 1.0)),
    ];
    let polyline = construct_polylines(&lines);
    assert_eq!(polyline.len(), 1);
    assert_eq!(polyline[0].len(), 8);

    let mut sign = None;
    for line in polyline[0].windows(2) {
        let a = line[0][0] + line[0][1] * 2.0 + line[0][2] * 4.0;
        let b = line[1][0] + line[1][1] * 2.0 + line[1][2] * 4.0;
        let x = b - a;
        assert!(f64::abs(x) == 1.0);
        let s = f64::signum(x * (x - 2.0) * (x + 2.0));
        if let Some(sign) = sign {
            assert!(s == sign);
        } else {
            sign = Some(s);
        }
    }
}

#[test]
fn duplicated_mesh_interference_segments_form_one_open_chain() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(1.0, 0.0, 0.0);
    let c = Point3::new(2.0, 0.0, 0.0);
    let result = construct_polylines(&[(a, b), (b, a), (b, c), (c, b)]);
    assert_eq!(
        result.len(),
        1,
        "triangle-pair duplication must not create extra branches"
    );
    assert_eq!(result[0].len(), 3);
    assert_ne!(result[0].first(), result[0].last());
}

#[test]
fn exact_duplicates_preserve_a_closed_loop_and_signed_zero_endpoints() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(2.0, 0.0, 0.0);
    let c = Point3::new(0.0, 2.0, 0.0);
    let minus_zero = Point3::new(-0.0, -0.0, -0.0);
    let result = construct_polylines(&[
        (a, b),
        (b, minus_zero),
        (b, c),
        (c, b),
        (c, a),
        (minus_zero, c),
    ]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 4);
    assert_eq!(result[0].first(), result[0].last());
}
#[test]
fn nearby_distinct_segments_keep_their_original_geometry() {
    let first = [Point3::new(0.0, 0.9e-6, 0.0), Point3::new(2.0, 0.9e-6, 0.0)];
    let second = [Point3::new(0.0, 1.1e-6, 0.0), Point3::new(2.0, 1.1e-6, 0.0)];
    assert!(first[0].near(&second[0]));
    let result = construct_polylines(&[(first[0], first[1]), (second[0], second[1])]);
    assert_eq!(result.len(), 2);
    for source in [first, second] {
        assert!(
            result
                .iter()
                .any(|line| line.as_slice() == source || line.as_slice() == [source[1], source[0]])
        );
    }
}
#[test]
fn exact_duplicate_removal_retains_all_branch_contact_edges() {
    let a = Point3::new(0.0, 0.0, 0.0);
    let b = Point3::new(1.0, 0.0, 0.0);
    let c = Point3::new(2.0, 0.0, 0.0);
    let d = Point3::new(1.0, 1.0, 0.0);
    let result = construct_polylines(&[(a, b), (b, a), (b, c), (c, b), (b, d), (d, b)]);
    let edges = result
        .iter()
        .flat_map(|line| line.windows(2))
        .collect::<Vec<_>>();
    assert_eq!(edges.len(), 3);
    for source in [[a, b], [b, c], [b, d]] {
        assert!(
            edges
                .iter()
                .any(|edge| **edge == source || **edge == [source[1], source[0]])
        );
    }
}
