//! Shared test-only geometry probes, independent of keycap construction.
use super::MeshData;

pub(super) fn contains(mesh: &MeshData, point: [f64; 3]) -> bool {
    let mut angle = 0.0;
    for triangle in mesh.positions.as_chunks::<9>().0.iter() {
        let [a, b, c] = [0, 1, 2].map(|vertex| {
            [
                f64::from(triangle[vertex * 3]) - point[0],
                f64::from(triangle[vertex * 3 + 1]) - point[1],
                f64::from(triangle[vertex * 3 + 2]) - point[2],
            ]
        });
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
