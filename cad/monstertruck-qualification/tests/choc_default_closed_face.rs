use monstertruck_io::step::load::Table;
use monstertruck_meshing::analyzers::WithPointCloud;
use monstertruck_meshing::prelude::*;
use std::collections::BTreeMap;

fn check(input: &str) {
    let table = Table::from_step(input).unwrap();
    table.entity_report.require_empty().unwrap();
    let (source, report) = table
        .to_compressed_shell_reported(table.shell.values().next().unwrap())
        .unwrap();
    report.require_lossless().unwrap();
    let mesh = source.robust_triangulation(0.0005);
    let face = mesh.faces[0].surface.as_ref().unwrap();
    let point = |p: Point3| [p.x, p.y, p.z].map(|x| (x * 1e7).round() as i64);
    let edge = |a, b| if a < b { [a, b] } else { [b, a] };
    let mut expected = BTreeMap::<_, usize>::new();
    for edge_use in mesh.faces[0].boundaries.iter().flatten() {
        for segment in mesh.edges[edge_use.index].curve.windows(2) {
            *expected
                .entry(edge(point(segment[0]), point(segment[1])))
                .or_default() += 1;
        }
    }
    let mut actual = BTreeMap::<_, usize>::new();
    let mut uv_area = 0.0;
    let surface = &source.faces[0].surface;
    for triangle in face.faces().triangle_iter() {
        let uv = triangle.map(|vertex| face.uv_coords()[vertex.uv.unwrap()]);
        let a = uv[1] - uv[0];
        let b = uv[2] - uv[0];
        let area = a.x * b.y - a.y * b.x;
        assert!(area > 0.0, "reversed UV facet");
        uv_area += area * 0.5;
        let xyz = triangle.map(|vertex| face.positions()[vertex.pos]);
        let centroid = (uv[0] + uv[1] + uv[2]) / 3.0;
        assert!(
            (xyz[1] - xyz[0])
                .cross(xyz[2] - xyz[0])
                .dot(surface.normal(centroid.x, centroid.y))
                > 0.0,
            "source-normal reversal"
        );
        let p = xyz.map(point);
        for i in 0..3 {
            assert_ne!(p[i], p[(i + 1) % 3], "collapsed source face facet");
            *actual.entry(edge(p[i], p[(i + 1) % 3])).or_default() += 1;
        }
    }
    let (Some(u), Some(v)) = surface.try_range_tuple() else {
        panic!("bounded authentic source patch")
    };
    assert!(
        (uv_area - (u.1 - u.0) * (v.1 - v.0)).abs() <= 1.0e-9,
        "full source chart coverage changed: {uv_area}"
    );
    let witnesses = (0..=64)
        .flat_map(|i| {
            (0..=128).map(move |j| {
                surface.evaluate(
                    u.0 + (u.1 - u.0) * i as f64 / 64.0,
                    v.0 + (v.1 - v.0) * j as f64 / 128.0,
                )
            })
        })
        .collect::<Vec<_>>();
    assert!(
        face.neighborhood_include(&witnesses, 0.0005),
        "original curved source patch witnesses missing"
    );
    let referenced = face
        .faces()
        .triangle_iter()
        .flat_map(|triangle| triangle.map(|vertex| vertex.pos))
        .collect::<std::collections::BTreeSet<_>>();
    for edge_use in mesh.faces[0].boundaries.iter().flatten() {
        let curve = &mesh.edges[edge_use.index].curve;
        for point in &curve[1..curve.len() - 1] {
            assert!(
                referenced
                    .iter()
                    .any(|index| face.positions()[*index] == *point),
                "original interior shared sample must remain exactly referenced: {point:?}"
            );
        }
    }
    println!(
        "triangles={} shared_segment_count={}",
        face.faces().triangle_iter().count(),
        expected.len()
    );
    for (segment, count) in &expected {
        assert_eq!(
            actual.get(segment),
            Some(count),
            "declared shared boundary segment missing: {segment:?}"
        );
    }
    for (segment, count) in actual {
        assert_eq!(
            count,
            expected.get(&segment).copied().unwrap_or(2),
            "unexpected open/interior edge: {segment:?}"
        );
    }
}

#[test]
fn default_closed_source_face_30282_positive_control() {
    check(include_str!("fixtures/choc-face30282.step"));
}

#[test]
fn default_closed_source_face_30604_retains_both_end_loops_and_shared_seam() {
    check(include_str!("fixtures/choc-face30604.step"));
}
