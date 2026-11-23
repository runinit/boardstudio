//! Shared test-only geometry probes, independent of keycap construction.
use super::MeshData;
use std::collections::BTreeSet;

pub(super) fn triangles(mesh: &MeshData) -> impl Iterator<Item = [[f64; 3]; 3]> + '_ {
    mesh.positions.as_chunks::<9>().0.iter().map(|triangle| {
        std::array::from_fn(|vertex| {
            std::array::from_fn(|axis| f64::from(triangle[vertex * 3 + axis]))
        })
    })
}

pub(super) fn closed_edges<K: Copy + Ord>(
    mesh: &MeshData,
    key: impl Fn([f64; 3]) -> K,
) -> Result<BTreeSet<(K, K)>, String> {
    let mut edges = BTreeSet::new();
    for triangle in triangles(mesh) {
        let [a, b, c] = triangle.map(&key);
        for edge in [(a, b), (b, c), (c, a)] {
            if !edges.insert(edge) {
                return Err("duplicate directed edge".into());
            }
        }
    }
    if edges.iter().any(|&(from, to)| !edges.contains(&(to, from))) {
        return Err("open or inverted edge".into());
    }
    Ok(edges)
}

pub(super) fn assert_finite_mesh(mesh: &MeshData) {
    assert!(mesh
        .positions
        .iter()
        .chain(&mesh.normals)
        .all(|value| value.is_finite()));
}

pub(super) fn contains(mesh: &MeshData, point: [f64; 3]) -> bool {
    let mut angle = 0.0;
    for triangle in triangles(mesh) {
        let [a, b, c] =
            triangle.map(|vertex| std::array::from_fn(|axis| vertex[axis] - point[axis]));
        let length = |p: [f64; 3]| dot(p, p).sqrt();
        let denominator = length(a) * length(b) * length(c)
            + dot(a, b) * length(c)
            + dot(b, c) * length(a)
            + dot(c, a) * length(b);
        angle += 2.0 * dot(a, cross(b, c)).atan2(denominator);
    }
    angle.abs() > std::f64::consts::TAU
}

pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
