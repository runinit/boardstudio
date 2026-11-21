// SPDX-License-Identifier: Apache-2.0

use super::{triangulate, Rings};
use crate::plane::{cross2, signed_area, Uv};

fn area(points: &[Uv], triangles: &[[usize; 3]]) -> f64 {
    triangles
        .iter()
        .map(|[a, b, c]| cross2(points[*a], points[*b], points[*c]) / 2.0)
        .sum()
}

#[test]
fn a_square_with_a_hole_covers_its_area() {
    let points: Vec<Uv> = vec![
        [0.0, 0.0],
        [4.0, 0.0],
        [4.0, 4.0],
        [0.0, 4.0],
        [1.0, 1.0],
        [1.0, 3.0],
        [3.0, 3.0],
        [3.0, 1.0],
    ];
    let triangles = triangulate(Rings {
        points: &points,
        outer: vec![0, 1, 2, 3],
        holes: vec![vec![4, 5, 6, 7]],
    })
    .unwrap();
    assert!((area(&points, &triangles) - 12.0).abs() < 1e-12);
    for [a, b, c] in &triangles {
        assert!(cross2(points[*a], points[*b], points[*c]) > 0.0);
    }
}

#[test]
fn a_clockwise_outer_ring_is_reversed() {
    let points: Vec<Uv> = vec![[0.0, 0.0], [0.0, 2.0], [3.0, 2.0], [3.0, 0.0]];
    assert!(signed_area(&points) < 0.0);
    let triangles = triangulate(Rings {
        points: &points,
        outer: vec![0, 1, 2, 3],
        holes: Vec::new(),
    })
    .unwrap();
    assert_eq!(triangles.len(), 2);
    assert!((area(&points, &triangles) - 6.0).abs() < 1e-12);
}

#[test]
fn a_concave_ring_keeps_its_notch() {
    let points: Vec<Uv> = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [2.0, 1.0], [0.0, 4.0]];
    let triangles = triangulate(Rings {
        points: &points,
        outer: vec![0, 1, 2, 3, 4],
        holes: Vec::new(),
    })
    .unwrap();
    assert!((area(&points, &triangles) - signed_area(&points) / 2.0).abs() < 1e-12);
}

#[test]
fn a_thin_diagonal_concave_ring_keeps_its_notch_and_area() {
    let d = 5e-12;
    let points: Vec<Uv> = vec![
        [0.0, 0.0],
        [1000.0, 1000.0],
        [1000.0 - d, 1000.0 + d],
        [500.0 - 0.25 * d, 500.0 + 0.25 * d],
        [-d, d],
    ];
    let roundoff = 64.0 * f64::EPSILON * 1000.0 * 1000.0;
    let max_turn = (0..points.len())
        .map(|i| {
            cross2(
                points[(i + points.len() - 1) % points.len()],
                points[i],
                points[(i + 1) % points.len()],
            )
            .abs()
        })
        .fold(0.0_f64, f64::max);
    assert!(
        max_turn < roundoff,
        "expected every local turn ({max_turn}) < roundoff ({roundoff})"
    );
    let expected_twice = super::shifted_ring_area(&points, &[0, 1, 2, 3, 4], points[0]);
    assert!(expected_twice > 0.0);
    assert!(!super::has_crossing_edges(&points, &[0, 1, 2, 3, 4]));

    let triangles = triangulate(Rings {
        points: &points,
        outer: vec![0, 1, 2, 3, 4],
        holes: Vec::new(),
    })
    .unwrap();
    for [a, b, c] in &triangles {
        assert!(cross2(points[*a], points[*b], points[*c]) > 0.0);
        for (idx, &p) in points.iter().enumerate() {
            if idx != *a && idx != *b && idx != *c {
                assert!(
                    cross2(points[*a], points[*b], p) < 0.0
                        || cross2(points[*b], points[*c], p) < 0.0
                        || cross2(points[*c], points[*a], p) < 0.0
                );
            }
        }
    }
    let emitted_twice: f64 = triangles
        .iter()
        .map(|[a, b, c]| super::accurate_cross2(points[*a], points[*b], points[*c]))
        .sum();
    assert!((emitted_twice - expected_twice).abs() <= 1e-10 * expected_twice);
}

#[test]
fn a_simple_ring_with_slit_and_hole_covers_its_area() {
    // Distinct point indices at identical coordinates mimic an authored STEP seam slit.
    let points: Vec<Uv> = vec![
        [0.0, 0.0],
        [6.0, 0.0],
        [6.0, 6.0],
        [3.0, 6.0],
        [3.0, 4.0],
        [3.0, 6.0],
        [0.0, 6.0],
        [1.0, 1.0],
        [1.0, 2.0],
        [2.0, 2.0],
        [2.0, 1.0],
    ];
    let outer = vec![0, 1, 2, 3, 4, 5, 6];
    let hole = vec![7, 8, 9, 10];
    let expected = (super::shifted_ring_area(&points, &outer, points[0])
        + super::shifted_ring_area(&points, &hole, points[0]))
        / 2.0;
    let triangles = triangulate(Rings {
        points: &points,
        outer,
        holes: vec![hole],
    })
    .unwrap();
    assert!((area(&points, &triangles) - expected).abs() < 1e-10);
}

#[test]
fn a_self_intersecting_ring_fails_simple_coverage_and_degenerate_zero_area_is_refused() {
    let bowtie: Vec<Uv> = vec![[0.0, 0.0], [4.0, 0.0], [1.0, 3.0], [3.0, 3.0]];
    assert!(super::has_crossing_edges(&bowtie, &[0, 1, 2, 3]));
    assert!(
        super::triangulate_simple(&bowtie, vec![0, 1, 2, 3], signed_area(&bowtie), 1e-12).is_err()
    );

    let degenerate: Vec<Uv> = vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]];
    assert!(triangulate(Rings {
        points: &degenerate,
        outer: vec![0, 1, 2, 3],
        holes: Vec::new(),
    })
    .is_err());
}
