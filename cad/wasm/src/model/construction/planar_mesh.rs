use super::*;

// A bounded conforming XY grid exactly partitions an orthogonal prism. Shared
// grid vertices prevent T-junctions; only material/void boundaries receive walls.
// Solids are still built by the kernel, and any unsupported profile uses its mesh.
pub(super) fn plate_mesh(body: &CaseBody, region: &PreparedRegion) -> Option<MeshData> {
    const MAX_EDGES: usize = 1024;
    const MAX_CELLS: usize = 4096;
    if !body.features.is_empty()
        || body.kind != CaseKind::Plate
        || body
            .openings
            .as_ref()
            .is_some_and(|openings| !openings.is_empty())
        || !region.mounts.is_empty()
        || !region.cavities.is_empty()
        || !region.gaskets.is_empty()
        || region.outer.len() != 4
        || !simple_orthogonal_polygon(&region.outer)
        || region.holes.iter().map(Vec::len).sum::<usize>() > MAX_EDGES
        || region
            .holes
            .iter()
            .any(|hole| !simple_orthogonal_polygon(hole))
    {
        return None;
    }
    let z0 = body.z.unwrap_or(0.);
    let z1 = z0 + body.thickness;
    if !z0.is_finite()
        || !z1.is_finite()
        || body.thickness <= 0.
        || !(z0 as f32).is_finite()
        || !(z1 as f32).is_finite()
        || z0 as f32 >= z1 as f32
    {
        return None;
    }
    let min_x = region.outer.iter().map(|p| p.x).reduce(f64::min)?;
    let max_x = region.outer.iter().map(|p| p.x).reduce(f64::max)?;
    let min_y = region.outer.iter().map(|p| p.y).reduce(f64::min)?;
    let max_y = region.outer.iter().map(|p| p.y).reduce(f64::max)?;
    if region
        .holes
        .iter()
        .flatten()
        .any(|p| p.x <= min_x || p.x >= max_x || p.y <= min_y || p.y >= max_y)
    {
        return None;
    }
    // Hole contact and overlap are deliberately left to the kernel mesher.
    for (index, hole) in region.holes.iter().enumerate() {
        for other in &region.holes[index + 1..] {
            for (a, b) in hole.iter().zip(hole.iter().cycle().skip(1)) {
                for (c, d) in other.iter().zip(other.iter().cycle().skip(1)) {
                    if a.x.min(b.x).max(c.x.min(d.x)) <= a.x.max(b.x).min(c.x.max(d.x))
                        && a.y.min(b.y).max(c.y.min(d.y)) <= a.y.max(b.y).min(c.y.max(d.y))
                    {
                        return None;
                    }
                }
            }
        }
    }
    let _stage = Stage::new("planarPlateMesh");
    let vertices: Vec<_> = region
        .outer
        .iter()
        .chain(region.holes.iter().flatten())
        .collect();
    let mut xs: Vec<_> = vertices.iter().map(|p| p.x).collect();
    let mut ys: Vec<_> = vertices.iter().map(|p| p.y).collect();
    for coordinates in [&mut xs, &mut ys] {
        coordinates.sort_by(f64::total_cmp);
        coordinates.dedup();
        if coordinates.iter().any(|&value| !(value as f32).is_finite())
            || coordinates
                .windows(2)
                .any(|pair| pair[0] as f32 >= pair[1] as f32)
        {
            return None;
        }
    }
    let width = xs.len() - 1;
    let height = ys.len() - 1;
    if width.checked_mul(height)? > MAX_CELLS {
        return None;
    }
    let mut material = vec![true; width * height];
    for (x, pair_x) in xs.windows(2).enumerate() {
        for (y, pair_y) in ys.windows(2).enumerate() {
            let px = pair_x[0] + (pair_x[1] - pair_x[0]) / 2.;
            let py = pair_y[0] + (pair_y[1] - pair_y[0]) / 2.;
            if px <= pair_x[0] || px >= pair_x[1] || py <= pair_y[0] || py >= pair_y[1] {
                return None;
            }
            let mut holes = 0;
            for hole in &region.holes {
                let crossings = hole
                    .iter()
                    .zip(hole.iter().cycle().skip(1))
                    .filter(|(a, b)| {
                        a.x == b.x && a.x > px && py > a.y.min(b.y) && py < a.y.max(b.y)
                    })
                    .count();
                holes += crossings % 2;
            }
            if holes > 1 {
                return None;
            }
            material[y * width + x] = holes == 0;
        }
    }
    let mut mesh = MeshData {
        positions: Vec::new(),
        normals: Vec::new(),
    };
    for (x, pair_x) in xs.windows(2).enumerate() {
        for (y, pair_y) in ys.windows(2).enumerate() {
            if !material[y * width + x] {
                continue;
            }
            let [x0, x1] = [pair_x[0], pair_x[1]];
            let [y0, y1] = [pair_y[0], pair_y[1]];
            quad(
                &mut mesh,
                [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
                [0., 0., 1.],
            );
            quad(
                &mut mesh,
                [[x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]],
                [0., 0., -1.],
            );
            if x == 0 || !material[y * width + x - 1] {
                quad(
                    &mut mesh,
                    [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]],
                    [-1., 0., 0.],
                );
            }
            if x + 1 == width || !material[y * width + x + 1] {
                quad(
                    &mut mesh,
                    [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
                    [1., 0., 0.],
                );
            }
            if y == 0 || !material[(y - 1) * width + x] {
                quad(
                    &mut mesh,
                    [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
                    [0., -1., 0.],
                );
            }
            if y + 1 == height || !material[(y + 1) * width + x] {
                quad(
                    &mut mesh,
                    [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
                    [0., 1., 0.],
                );
            }
        }
    }
    metrics::count("exactPlanarMeshes", 1);
    metrics::count("tessellatedTriangles", mesh.positions.len() / 9);
    Some(mesh)
}

fn quad(mesh: &mut MeshData, points: [[f64; 3]; 4], normal: [f32; 3]) {
    for index in [0, 1, 2, 0, 2, 3] {
        mesh.positions
            .extend(points[index].map(|value| value as f32));
        mesh.normals.extend(normal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fixture() -> PreparedAssembly {
        serde_json::from_str(include_str!(
            "../../../../bench/fixtures/live-numeric-plates-regression.json"
        ))
        .unwrap()
    }

    #[test]
    fn exact_plate_mesh_is_watertight_and_matches_kernel_material() {
        for mut body in fixture().bodies {
            for reverse in [false, true] {
                if reverse {
                    body.body.z = Some(-2.5);
                }
                for region in &mut body.regions {
                    if reverse {
                        region.outer.reverse();
                        for hole in &mut region.holes {
                            hole.reverse();
                        }
                    }
                    let mesh = plate_mesh(&body.body, region)
                        .expect("orthogonal plate takes the exact mesh path");
                    let solids = build_region(&body.body, region).unwrap();
                    let mut edges: HashMap<([u32; 3], [u32; 3]), (usize, i32)> = HashMap::new();
                    let mut volume = 0.;
                    let sample_stride = (mesh.positions.len() / 9 / 32).max(1);
                    for (triangle_index, (triangle, normals)) in mesh
                        .positions
                        .chunks_exact(9)
                        .zip(mesh.normals.chunks_exact(9))
                        .enumerate()
                    {
                        let points: Vec<_> = triangle
                            .chunks_exact(3)
                            .map(|p| DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64))
                            .collect();
                        let cross = (points[1] - points[0]).cross(points[2] - points[0]);
                        assert!(cross.length() > 0.);
                        let normal =
                            DVec3::new(normals[0] as f64, normals[1] as f64, normals[2] as f64);
                        assert!((cross.normalize() - normal).length() < 1e-7);
                        let center = (points[0] + points[1] + points[2]) / 3.;
                        if triangle_index % sample_stride == 0 {
                            assert!(
                                solids
                                    .iter()
                                    .any(|solid| solid.contains(center - normal * 0.0001)),
                                "triangle is outside the kernel solid"
                            );
                            assert!(
                                !solids
                                    .iter()
                                    .any(|solid| solid.contains(center + normal * 0.0001)),
                                "triangle is inside the kernel solid"
                            );
                        }
                        volume += points[0].dot(points[1].cross(points[2])) / 6.;
                        let keys: Vec<[u32; 3]> = triangle
                            .chunks_exact(3)
                            .map(|p| [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()])
                            .collect();
                        for i in 0..3 {
                            let a = keys[i];
                            let b = keys[(i + 1) % 3];
                            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
                            let entry = edges.entry(key).or_default();
                            entry.0 += 1;
                            entry.1 += sign;
                        }
                    }
                    assert!(edges
                        .values()
                        .all(|&(count, direction)| count == 2 && direction == 0));
                    let expected: f64 = solids.iter().map(Solid::volume).sum();
                    assert!(
                        (volume - expected).abs() < 0.05,
                        "mesh {volume}, kernel {expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn exact_plate_mesh_falls_back_for_modified_or_invalid_geometry() {
        let mut ir = fixture();
        let body = &mut ir.bodies[0];
        let region = &mut body.regions[0];
        let original = region.holes[0][0].x;
        region.holes[0][0].x += 0.2;
        assert!(plate_mesh(&body.body, region).is_none());
        region.holes[0][0].x = original;
        body.body.openings = Some(vec![CaseOpening {
            points: vec![],
            z: 0.,
            height: 1.,
        }]);
        assert!(plate_mesh(&body.body, region).is_none());
        body.body.openings = None;
        body.body.kind = CaseKind::Tray;
        assert!(plate_mesh(&body.body, region).is_none());
        body.body.kind = CaseKind::Plate;
        body.body.thickness = 0.;
        assert!(plate_mesh(&body.body, region).is_none());
        body.body.thickness = 1.;
        region.holes[0][0].x = f64::NAN;
        assert!(plate_mesh(&body.body, region).is_none());
    }

    #[test]
    fn exact_plate_mesh_bounds_work_and_rejects_hole_contact_or_overlap() {
        let mut ir = fixture();
        let body = &mut ir.bodies[0];
        let region = &mut body.regions[0];
        let square = |x: f64, y: f64, size: f64| {
            vec![
                Vec2 { x, y },
                Vec2 { x: x + size, y },
                Vec2 {
                    x: x + size,
                    y: y + size,
                },
                Vec2 { x, y: y + size },
            ]
        };
        region.outer = square(0., 0., 100.);
        for (x, y, size) in [
            (20., 10., 10.),
            (15., 15., 10.),
            (12., 12., 2.),
            (10., 10., 10.),
        ] {
            region.holes = vec![square(10., 10., 10.), square(x, y, size)];
            assert!(plate_mesh(&body.body, region).is_none());
        }
        region.holes = vec![square(0., 10., 10.)];
        assert!(plate_mesh(&body.body, region).is_none());
        region.holes = (0..34)
            .map(|i| square(1. + i as f64 * 2., 1. + i as f64 * 2., 1.))
            .collect();
        assert!(
            plate_mesh(&body.body, region).is_none(),
            "grid exceeds 4096 cells"
        );
        region.holes.clear();
        assert!(plate_mesh(&body.body, region).is_some());
        body.body.z = Some(1e20);
        assert!(
            plate_mesh(&body.body, region).is_none(),
            "float coordinates collapse the height"
        );
    }
}
