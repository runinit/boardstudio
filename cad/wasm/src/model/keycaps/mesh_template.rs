// Local-coordinate keycap display-mesh constructor.

use super::{empty_mesh, lettering, validate, MeshData};
use boardstudio_contracts::{KeycapMount, KeycapSpec};

pub(crate) struct KeycapMeshTemplate {
    pub(crate) shell: MeshData,
    // Stem and socket stay separate for display, even where they overlap.
    pub(crate) stem: MeshData,
    pub(crate) socket: MeshData,
    pub(crate) legend: Option<MeshData>,
}

/// Build a local-coordinate display template. Placement, cache ownership and
/// public body IDs remain with the existing CAD worker.
pub(crate) fn build_keycap_mesh_template(spec: &KeycapSpec) -> Result<KeycapMeshTemplate, String> {
    validate(spec)?;
    let shell = build_shell_mesh(spec)?;
    let (stem, socket) = build_mount_meshes(spec)?;
    let legend = lettering::mesh(spec)?;
    Ok(KeycapMeshTemplate {
        shell,
        stem,
        socket,
        legend,
    })
}

fn build_shell_mesh(spec: &KeycapSpec) -> Result<MeshData, String> {
    let wall = spec.wall_thickness;
    let outer_bottom = rounded_ring(spec.size.x, spec.size.y, 0.0, 0.0);
    let inner_bottom = rounded_ring(spec.size.x - 2.0 * wall, spec.size.y - 2.0 * wall, 0.0, 0.0);
    let outer_top = dish_ring(
        rounded_ring(spec.top_size.x, spec.top_size.y, spec.height, spec.tilt),
        spec,
        spec.height,
    );
    let inner_top = dish_ring(
        rounded_ring(
            spec.top_size.x - 2.0 * wall,
            spec.top_size.y - 2.0 * wall,
            spec.height - wall,
            spec.tilt,
        ),
        spec,
        spec.height - wall,
    );

    if outer_bottom.len() != inner_bottom.len()
        || outer_top.len() != inner_top.len()
        || outer_bottom.len() != outer_top.len()
    {
        return Err("Keycap mesh profile rings do not match".into());
    }

    let mut mesh = empty_mesh();
    let count = outer_bottom.len();
    for index in 0..count {
        let next = (index + 1) % count;
        // The outer profile is counter-clockwise, so these triangles face out.
        add_quad(
            &mut mesh,
            [
                outer_bottom[index],
                outer_bottom[next],
                outer_top[next],
                outer_top[index],
            ],
        )?;
        // Reverse the inner wall so its normal points into the socket cavity.
        add_quad(
            &mut mesh,
            [
                inner_top[index],
                inner_top[next],
                inner_bottom[next],
                inner_bottom[index],
            ],
        )?;
        // Join the lower rings with a sloped annulus; its normal faces down.
        add_triangle(
            &mut mesh,
            [outer_bottom[index], inner_bottom[next], outer_bottom[next]],
        )?;
        add_triangle(
            &mut mesh,
            [outer_bottom[index], inner_bottom[index], inner_bottom[next]],
        )?;
    }
    add_roof(&mut mesh, &outer_top, spec, spec.height, true)?;
    add_roof(&mut mesh, &inner_top, spec, spec.height - wall, false)?;
    Ok(mesh)
}

fn rounded_ring(width: f64, depth: f64, base_z: f64, tilt_degrees: f64) -> Vec<[f64; 3]> {
    let radius = 0.8_f64.min(width / 5.0).min(depth / 5.0);
    let corners = [
        (width / 2.0 - radius, depth / 2.0 - radius, 0.0_f64),
        (-width / 2.0 + radius, depth / 2.0 - radius, 90.0),
        (-width / 2.0 + radius, -depth / 2.0 + radius, 180.0),
        (width / 2.0 - radius, -depth / 2.0 + radius, 270.0),
    ];
    let arcs = corners.map(|(x, y, start)| {
        (0..=4)
            .map(|step| {
                let angle = (start + f64::from(step) * 22.5).to_radians();
                let px = x + radius * angle.cos();
                let py = y + radius * angle.sin();
                [px, py, base_z + py * tilt_degrees.to_radians().tan()]
            })
            .collect::<Vec<_>>()
    });
    let mut points = Vec::with_capacity(24);
    for index in 0..arcs.len() {
        points.extend_from_slice(&arcs[index]);
        let end = arcs[index][4];
        let next = arcs[(index + 1) % arcs.len()][0];
        points.push([
            (end[0] + next[0]) / 2.0,
            (end[1] + next[1]) / 2.0,
            base_z + ((end[1] + next[1]) / 2.0) * tilt_degrees.to_radians().tan(),
        ]);
    }
    points
}

fn dish_ring(mut ring: Vec<[f64; 3]>, spec: &KeycapSpec, baseline: f64) -> Vec<[f64; 3]> {
    if spec.dish_depth == 0.0 {
        return ring;
    }
    for point in &mut ring {
        let sag = dish_sag(spec, point[0], point[1]);
        point[2] = baseline + point[1] * spec.tilt.to_radians().tan() - sag;
    }
    ring
}

pub(super) fn dish_sag(spec: &KeycapSpec, x: f64, y: f64) -> f64 {
    let depth = spec.dish_depth;
    if depth == 0.0 {
        return 0.0;
    }
    let maximum_radius = spec.top_size.y / 2.0;
    let distance = if spec.spherical { x.hypot(y) } else { y.abs() };
    let radius = (maximum_radius * maximum_radius + depth * depth) / (2.0 * depth);
    let under_root = (radius * radius - distance * distance).max(0.0);
    (depth - (radius - under_root.sqrt())).clamp(0.0, depth)
}

fn add_roof(
    mesh: &mut MeshData,
    boundary: &[[f64; 3]],
    spec: &KeycapSpec,
    baseline: f64,
    upwards: bool,
) -> Result<(), String> {
    const RINGS: usize = 12;
    let center = [
        0.0,
        0.0,
        baseline
            - if spec.dish_depth == 0.0 {
                0.0
            } else {
                spec.dish_depth
            },
    ];
    let mut inner = (0..boundary.len()).map(|_| center).collect::<Vec<_>>();
    for ring_index in 1..=RINGS {
        let scale = ring_index as f64 / RINGS as f64;
        let outer = boundary
            .iter()
            .map(|point| {
                let x = point[0] * scale;
                let y = point[1] * scale;
                [
                    x,
                    y,
                    baseline + y * spec.tilt.to_radians().tan() - dish_sag(spec, x, y),
                ]
            })
            .collect::<Vec<_>>();
        if ring_index == 1 {
            for index in 0..boundary.len() {
                let next = (index + 1) % boundary.len();
                let triangle = if upwards {
                    [center, outer[index], outer[next]]
                } else {
                    [center, outer[next], outer[index]]
                };
                add_triangle(mesh, triangle)?;
            }
        } else {
            for index in 0..boundary.len() {
                let next = (index + 1) % boundary.len();
                let triangles = if upwards {
                    [
                        [inner[index], outer[index], outer[next]],
                        [inner[index], outer[next], inner[next]],
                    ]
                } else {
                    [
                        [inner[index], outer[next], outer[index]],
                        [inner[index], inner[next], outer[next]],
                    ]
                };
                add_triangle(mesh, triangles[0])?;
                add_triangle(mesh, triangles[1])?;
            }
        }
        inner = outer;
    }
    Ok(())
}

pub(super) fn add_quad(mesh: &mut MeshData, points: [[f64; 3]; 4]) -> Result<(), String> {
    add_triangle(mesh, [points[0], points[1], points[2]])?;
    add_triangle(mesh, [points[0], points[2], points[3]])
}

pub(super) fn add_triangle(mesh: &mut MeshData, points: [[f64; 3]; 3]) -> Result<(), String> {
    let a = points[0];
    let b = points[1];
    let c = points[2];
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let normal = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length == 0.0
        || !length.is_finite()
        || points
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite() || !(*coordinate as f32).is_finite())
    {
        return Err("Keycap mesh contains a degenerate triangle".into());
    }
    let rendered = points.map(|point| point.map(|coordinate| f64::from(coordinate as f32)));
    let rendered_ab = [
        rendered[1][0] - rendered[0][0],
        rendered[1][1] - rendered[0][1],
        rendered[1][2] - rendered[0][2],
    ];
    let rendered_ac = [
        rendered[2][0] - rendered[0][0],
        rendered[2][1] - rendered[0][1],
        rendered[2][2] - rendered[0][2],
    ];
    let rendered_normal = [
        rendered_ab[1] * rendered_ac[2] - rendered_ab[2] * rendered_ac[1],
        rendered_ab[2] * rendered_ac[0] - rendered_ab[0] * rendered_ac[2],
        rendered_ab[0] * rendered_ac[1] - rendered_ab[1] * rendered_ac[0],
    ];
    if rendered_normal.iter().all(|coordinate| *coordinate == 0.0)
        || dot3(rendered_normal, normal) <= 0.0
    {
        return Err("Keycap triangle loses orientation at render precision".into());
    }
    for point in points {
        mesh.positions
            .extend(point.into_iter().map(|coordinate| coordinate as f32));
        mesh.normals.extend(
            normal
                .into_iter()
                .map(|coordinate| (coordinate / length) as f32),
        );
    }
    Ok(())
}

fn build_mount_meshes(spec: &KeycapSpec) -> Result<(MeshData, MeshData), String> {
    let wall = spec.wall_thickness;
    let socket_depth = match spec.mount {
        KeycapMount::ChocV1 | KeycapMount::ChocV2 => 2.3,
        KeycapMount::Mx | KeycapMount::Alps => 3.6,
    };
    let stem_top = spec.height - wall + 0.1;
    if spec.height - spec.dish_depth - wall < socket_depth + 0.2 {
        return Err("Keycap roof is too low for the socket; reduce wall thickness".into());
    }
    if stem_top <= socket_depth {
        return Err("Keycap mount does not fit below the roof".into());
    }

    let mut stems = empty_mesh();
    let mut sockets = empty_mesh();
    match spec.mount {
        KeycapMount::Mx | KeycapMount::ChocV2 => {
            add_cross_stem(&mut stems, 0.0, 6.0, socket_depth, stem_top)?;
            add_cross_socket(&mut sockets, 0.0, socket_depth)?;
        }
        KeycapMount::Alps => {
            add_rectangular_stem(
                &mut stems,
                [-3.0, -3.0, 3.0, 3.0],
                [-2.3, -1.2, 2.3, 1.2],
                socket_depth,
                stem_top,
            )?;
            add_socket_polygon(
                &mut sockets,
                &[[-2.3, -1.2], [2.3, -1.2], [2.3, 1.2], [-2.3, 1.2]],
                socket_depth,
            )?;
        }
        KeycapMount::ChocV1 => {
            for center_x in [-2.85, 2.85] {
                add_rectangular_stem(
                    &mut stems,
                    [center_x - 2.1, -2.1, center_x + 2.1, 2.1],
                    [center_x - 0.65, -1.65, center_x + 0.65, 1.65],
                    socket_depth,
                    stem_top,
                )?;
                add_socket_polygon(
                    &mut sockets,
                    &[
                        [center_x - 0.65, -1.65],
                        [center_x + 0.65, -1.65],
                        [center_x + 0.65, 1.65],
                        [center_x - 0.65, 1.65],
                    ],
                    socket_depth,
                )?;
            }
        }
    }
    Ok((stems, sockets))
}

fn add_cross_stem(
    mesh: &mut MeshData,
    center_x: f64,
    width: f64,
    socket_depth: f64,
    stem_top: f64,
) -> Result<(), String> {
    let half = width / 2.0;
    let cross = 2.1;
    let slot = 0.65;
    let grid = [-half, -cross, -slot, slot, cross, half];
    for y in 0..grid.len() - 1 {
        for x in 0..grid.len() - 1 {
            let cross_cell = ((1..=3).contains(&x) && y == 2) || (x == 2 && (1..=3).contains(&y));
            let bounds = [
                center_x + grid[x],
                grid[y],
                center_x + grid[x + 1],
                grid[y + 1],
            ];
            if !cross_cell {
                add_rect_cap(mesh, bounds, 0.0, false)?;
            }
            if cross_cell {
                add_rect_cap(mesh, bounds, socket_depth, false)?;
            }
            add_rect_cap(mesh, bounds, stem_top, true)?;
        }
    }

    let mut outer = Vec::with_capacity(20);
    for index in 0..grid.len() - 1 {
        outer.push([center_x + grid[index], grid[0]]);
    }
    for index in 0..grid.len() - 1 {
        outer.push([center_x + grid[grid.len() - 1], grid[index]]);
    }
    for index in (1..grid.len()).rev() {
        outer.push([center_x + grid[index], grid[grid.len() - 1]]);
    }
    for index in (1..grid.len()).rev() {
        outer.push([center_x + grid[0], grid[index]]);
    }
    add_wall_loop(mesh, &outer, 0.0, stem_top)?;

    let cross_ccw = [
        [-slot, -cross],
        [slot, -cross],
        [slot, -slot],
        [cross, -slot],
        [cross, slot],
        [slot, slot],
        [slot, cross],
        [-slot, cross],
        [-slot, slot],
        [-cross, slot],
        [-cross, -slot],
        [-slot, -slot],
    ];
    let cross_hole = cross_ccw
        .iter()
        .rev()
        .map(|point| [center_x + point[0], point[1]])
        .collect::<Vec<_>>();
    add_wall_loop(mesh, &cross_hole, 0.0, socket_depth)?;
    Ok(())
}

fn add_rect_cap(
    mesh: &mut MeshData,
    [x0, y0, x1, y1]: [f64; 4],
    z: f64,
    upwards: bool,
) -> Result<(), String> {
    let points = if upwards {
        [[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]]
    } else {
        [[x0, y0, z], [x0, y1, z], [x1, y1, z], [x1, y0, z]]
    };
    add_quad(mesh, points)
}

fn add_wall_loop(
    mesh: &mut MeshData,
    points: &[[f64; 2]],
    bottom: f64,
    top: f64,
) -> Result<(), String> {
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        let [x0, y0] = points[index];
        let [x1, y1] = points[next];
        add_quad(
            mesh,
            [
                [x0, y0, bottom],
                [x1, y1, bottom],
                [x1, y1, top],
                [x0, y0, top],
            ],
        )?;
    }
    Ok(())
}

fn add_rectangular_stem(
    mesh: &mut MeshData,
    outer: [f64; 4],
    slot: [f64; 4],
    socket_depth: f64,
    stem_top: f64,
) -> Result<(), String> {
    let [outer_left, outer_bottom, outer_right, outer_top] = outer;
    let [slot_left, slot_bottom, slot_right, slot_top] = slot;
    let xs = [outer_left, slot_left, slot_right, outer_right];
    let ys = [outer_bottom, slot_bottom, slot_top, outer_top];
    for y in 0..3 {
        for x in 0..3 {
            let bounds = [xs[x], ys[y], xs[x + 1], ys[y + 1]];
            let bottom = if x == 1 && y == 1 { socket_depth } else { 0.0 };
            add_rect_cap(mesh, bounds, bottom, false)?;
            add_rect_cap(mesh, bounds, stem_top, true)?;
        }
    }
    let outer_loop = [
        [xs[0], ys[0]],
        [xs[1], ys[0]],
        [xs[2], ys[0]],
        [xs[3], ys[0]],
        [xs[3], ys[1]],
        [xs[3], ys[2]],
        [xs[3], ys[3]],
        [xs[2], ys[3]],
        [xs[1], ys[3]],
        [xs[0], ys[3]],
        [xs[0], ys[2]],
        [xs[0], ys[1]],
    ];
    add_wall_loop(mesh, &outer_loop, 0.0, stem_top)?;
    let slot_loop = [
        [slot_left, slot_bottom],
        [slot_left, slot_top],
        [slot_right, slot_top],
        [slot_right, slot_bottom],
    ];
    add_wall_loop(mesh, &slot_loop, 0.0, socket_depth)
}

fn add_cross_socket(mesh: &mut MeshData, center_x: f64, z: f64) -> Result<(), String> {
    let points = [
        [center_x - 0.65, -2.1],
        [center_x + 0.65, -2.1],
        [center_x + 0.65, -0.65],
        [center_x + 2.1, -0.65],
        [center_x + 2.1, 0.65],
        [center_x + 0.65, 0.65],
        [center_x + 0.65, 2.1],
        [center_x - 0.65, 2.1],
        [center_x - 0.65, 0.65],
        [center_x - 2.1, 0.65],
        [center_x - 2.1, -0.65],
        [center_x - 0.65, -0.65],
    ];
    add_socket_polygon(mesh, &points, z)
}

fn add_socket_polygon(mesh: &mut MeshData, points: &[[f64; 2]], z: f64) -> Result<(), String> {
    if points.len() < 3 {
        return Err("Keycap socket mesh requires at least three points".into());
    }
    let center = points.iter().fold([0.0, 0.0], |mut center, point| {
        center[0] += point[0] / points.len() as f64;
        center[1] += point[1] / points.len() as f64;
        center
    });
    let center = [center[0], center[1], z - 0.001];
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        // Socket floors face down so they remain visible from the keycap opening.
        add_triangle(
            mesh,
            [
                center,
                [points[next][0], points[next][1], z - 0.001],
                [points[index][0], points[index][1], z - 0.001],
            ],
        )?;
    }
    Ok(())
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_contracts::{KeycapMount, KeycapProfile, Pose2, Side, Vec2};
    use std::collections::BTreeMap;

    fn flat_g20() -> KeycapSpec {
        KeycapSpec {
            id: "g20".into(),
            reference: "SW1".into(),
            profile: KeycapProfile::G20,
            mount: KeycapMount::Mx,
            row: 3,
            size: Vec2 { x: 18.2, y: 18.2 },
            top_size: Vec2 { x: 13.2, y: 13.2 },
            height: 6.5,
            tilt: 0.0,
            dish_depth: 0.0,
            spherical: false,
            wall_thickness: 1.2,
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            z: 0.0,
            travel: 4.0,
            legend: String::new(),
            color: "#eeeeee".into(),
            legend_color: "#222222".into(),
        }
    }

    #[test]
    fn flat_g20_mesh_has_independent_bounds_and_open_underbody() {
        let spec = flat_g20();
        let template = build_keycap_mesh_template(&spec).expect("valid G20 keycap");
        assert_valid_mesh(&template.shell);

        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/keycap-flat-g20.json"))
                .unwrap();
        let expected: [[f64; 3]; 2] = serde_json::from_value(fixture["bounds"].clone()).unwrap();
        let actual = bounds(&template.shell);
        for side in 0..2 {
            for axis in 0..3 {
                assert!(
                    (actual[side][axis] - expected[side][axis]).abs() < 1e-5,
                    "mesh bound {side}/{axis} = {}, expected {}",
                    actual[side][axis],
                    expected[side][axis]
                );
            }
        }
        assert!(
            contains(&template.shell, [8.5, 0.0, 1.0]),
            "outer wall is material"
        );
        assert!(
            !contains(&template.shell, [6.0, 0.0, 1.0]),
            "the underside cavity remains open"
        );
        assert_oriented_closed_mesh(&template.shell);
        let expected_volume = expected_shell_volume(&spec);
        let actual_volume = signed_volume(&template.shell);
        assert!(
            (actual_volume - expected_volume).abs() < 0.05,
            "shell material volume {actual_volume}, independent loft expectation {expected_volume} mm³"
        );
        let outer_roof_x = max_x_at_height(&template.shell, spec.height);
        let inner_roof_x = max_x_at_height(&template.shell, spec.height - spec.wall_thickness);
        assert!(
            ((outer_roof_x - inner_roof_x) - spec.wall_thickness).abs() < 1e-5,
            "roof side clearance is {} mm, expected {} mm",
            outer_roof_x - inner_roof_x,
            spec.wall_thickness
        );
    }

    #[test]
    fn zero_dish_keeps_a_flat_center_surface() {
        let spec = flat_g20();
        let template = build_keycap_mesh_template(&spec).expect("valid zero-dish keycap");
        let center = surface_height_at(&template.shell, 0.0, 0.0).unwrap();
        assert!((center - spec.height).abs() < 1e-5, "center z={center}");
    }

    #[test]
    fn spherical_dish_center_and_off_axis_height_follow_sagitta() {
        let mut spec = flat_g20();
        spec.dish_depth = 0.5;
        spec.spherical = true;
        let template = build_keycap_mesh_template(&spec).expect("valid spherical dish");
        let center = surface_height_at(&template.shell, 0.0, 0.0).unwrap();
        assert!((center - (spec.height - spec.dish_depth)).abs() < 0.02);

        let x = spec.top_size.x * 0.25;
        let actual = surface_height_at(&template.shell, x, 0.0).unwrap();
        let half_depth = spec.top_size.y / 2.0;
        let radius = (half_depth * half_depth + spec.dish_depth.powi(2)) / (2.0 * spec.dish_depth);
        let sag = spec.dish_depth - (radius - (radius * radius - x * x).sqrt());
        let expected = spec.height - sag;
        assert!(
            (actual - expected).abs() < 0.04,
            "spherical off-axis z={actual}, expected {expected}"
        );
    }

    #[test]
    fn cylindrical_dish_curves_across_depth_but_not_along_its_axis() {
        let mut spec = flat_g20();
        spec.dish_depth = 0.5;
        spec.spherical = false;
        let template = build_keycap_mesh_template(&spec).expect("valid cylindrical dish");
        let center = surface_height_at(&template.shell, 0.0, 0.0).unwrap();
        let along_axis = surface_height_at(&template.shell, spec.top_size.x * 0.25, 0.0).unwrap();
        assert!((center - (spec.height - spec.dish_depth)).abs() < 0.02);
        assert!((along_axis - center).abs() < 0.02);

        let y = spec.top_size.y * 0.25;
        let across_axis = surface_height_at(&template.shell, 0.0, y).unwrap();
        let half_depth = spec.top_size.y / 2.0;
        let radius =
            (half_depth * half_depth + spec.dish_depth * spec.dish_depth) / (2.0 * spec.dish_depth);
        let sag = spec.dish_depth - (radius - (radius * radius - y * y).sqrt());
        let expected = spec.height - sag;
        assert!(
            (across_axis - expected).abs() < 0.04,
            "cylindrical across-axis z={across_axis}, expected {expected}"
        );
    }

    #[test]
    fn mount_meshes_keep_stem_and_socket_geometry_separate() {
        let spec = flat_g20();
        let template = build_keycap_mesh_template(&spec).expect("valid MX keycap");
        assert_valid_mesh(&template.stem);
        assert_valid_mesh(&template.socket);
        let stem_bounds = bounds(&template.stem);
        assert!((stem_bounds[0][0] + 3.0).abs() < 1e-5);
        assert!((stem_bounds[1][0] - 3.0).abs() < 1e-5);
        assert!((stem_bounds[1][2] - (spec.height - spec.wall_thickness + 0.1)).abs() < 1e-5);
        let socket_bounds = bounds(&template.socket);
        assert!((socket_bounds[0][0] + 2.1).abs() < 1e-5);
        assert!((socket_bounds[1][0] - 2.1).abs() < 1e-5);
        assert!((socket_bounds[0][1] + 2.1).abs() < 1e-5);
        assert!((socket_bounds[1][1] - 2.1).abs() < 1e-5);
        assert!((socket_bounds[0][2] - 3.599).abs() < 1e-5);
        assert!((socket_bounds[1][2] - 3.599).abs() < 1e-5);
    }

    #[test]
    fn tilted_shell_keeps_flat_base_and_authored_top_plane() {
        let mut spec = flat_g20();
        spec.tilt = 6.0;
        let template = build_keycap_mesh_template(&spec).expect("valid tilted keycap");
        let actual = bounds(&template.shell);
        assert!(actual[0][2].abs() < 1e-5, "bottom remains z=0");
        let expected_top = spec.height + spec.top_size.y / 2.0 * spec.tilt.to_radians().tan();
        assert!(
            (actual[1][2] - expected_top).abs() < 1e-5,
            "top bound {} should follow the authored tilt to {expected_top}",
            actual[1][2]
        );
    }

    #[test]
    fn all_mount_variants_have_finite_separate_display_meshes() {
        for mount in [
            KeycapMount::Mx,
            KeycapMount::Alps,
            KeycapMount::ChocV1,
            KeycapMount::ChocV2,
        ] {
            let mut spec = flat_g20();
            spec.mount = mount;
            if matches!(spec.mount, KeycapMount::ChocV1 | KeycapMount::ChocV2) {
                spec.profile = KeycapProfile::Choc;
                spec.height = 4.2;
                spec.dish_depth = 0.4;
            } else {
                spec.height = 10.0;
                spec.dish_depth = 0.5;
            }
            let template = build_keycap_mesh_template(&spec).expect("valid mount variant");
            assert_valid_mesh(&template.stem);
            assert_valid_mesh(&template.socket);
        }
    }

    #[test]
    fn bundled_o_legend_is_separate_and_keeps_its_counter() {
        let mut spec = flat_g20();
        spec.legend = "O".into();
        let template = build_keycap_mesh_template(&spec).expect("supported bundled-font legend");
        let legend = template.legend.as_ref().expect("legend is a separate mesh");
        assert_valid_mesh(legend);
        let legend_bounds = bounds(legend);
        assert!(legend_bounds[0][0] >= -spec.top_size.x / 2.0 + 1.0);
        assert!(legend_bounds[1][0] <= spec.top_size.x / 2.0 - 1.0);
        assert!(
            legend_bounds[1][2] > spec.height,
            "raised legend has visible relief"
        );
        assert!(
            !contains(legend, [0.0, 0.0, spec.height]),
            "the O counter remains unfilled"
        );

        let single_glyph_width = legend_bounds[1][0] - legend_bounds[0][0];
        spec.legend = "A1".into();
        let multi_legend = build_keycap_mesh_template(&spec)
            .expect("multi-character bundled-font legend")
            .legend
            .expect("multi-character legend remains separate");
        assert_valid_mesh(&multi_legend);
        let multi_bounds = bounds(&multi_legend);
        assert!(
            multi_bounds[1][0] - multi_bounds[0][0] > single_glyph_width,
            "font advances place both glyphs in the fitted legend"
        );
    }

    #[test]
    fn legend_top_tracks_the_spherical_dish_and_tilt() {
        let mut spec = flat_g20();
        spec.dish_depth = 0.5;
        spec.spherical = true;
        spec.tilt = 6.0;
        spec.legend = "A".into();
        let template = build_keycap_mesh_template(&spec).expect("valid tilted legend");
        let legend = template.legend.as_ref().expect("separate legend mesh");
        let mut raised_vertices = 0;
        let mut embedded_vertices = 0;
        for vertex in legend.positions.as_chunks::<3>().0.iter() {
            let x = f64::from(vertex[0]);
            let y = f64::from(vertex[1]);
            let distance = x.hypot(y);
            let half_depth = spec.top_size.y / 2.0;
            let radius = (half_depth.powi(2) + spec.dish_depth.powi(2)) / (2.0 * spec.dish_depth);
            let sag = (spec.dish_depth
                - (radius - (radius.powi(2) - distance.powi(2)).max(0.0).sqrt()))
            .clamp(0.0, spec.dish_depth);
            let surface = spec.height + y * spec.tilt.to_radians().tan() - sag;
            let offset = f64::from(vertex[2]) - surface;
            if (offset - 0.03).abs() < 1e-5 {
                raised_vertices += 1;
            } else if (offset + 0.15).abs() < 1e-5 {
                embedded_vertices += 1;
            } else {
                panic!(
                    "legend vertex ({x},{y},{}) is not attached to its dish/tilt surface",
                    vertex[2]
                );
            }
        }
        assert!(raised_vertices > 0, "legend has raised surface vertices");
        assert!(
            embedded_vertices > 0,
            "legend has embedded surface vertices"
        );
    }

    #[test]
    fn invalid_mesh_parameters_are_rejected() {
        let mut spec = flat_g20();
        spec.size.x = f64::NAN;
        assert!(build_keycap_mesh_template(&spec).is_err());

        let mut spec = flat_g20();
        spec.dish_depth = 0.9;
        assert!(build_keycap_mesh_template(&spec).is_err());
    }

    #[test]
    fn mx_stem_is_closed_and_preserves_independent_blind_socket_material() {
        let template = build_keycap_mesh_template(&flat_g20()).expect("valid MX keycap");
        assert_valid_mesh(&template.stem);
        assert_oriented_closed_mesh(&template.stem);
        // A 6 x 6 x 5.4 stem minus a 9.23 mm² cross bored 3.6 mm deep.
        let actual_volume = signed_volume(&template.stem);
        assert!(
            (actual_volume - 161.172).abs() < 1e-5,
            "MX stem volume {actual_volume}"
        );
        for (point, material) in [
            ([0.137, 0.113, 1.113], false),
            ([1.5, 0.113, 1.113], false),
            ([0.137, 1.5, 1.113], false),
            ([1.0, 1.0, 1.113], true),
            ([2.5, 2.5, 1.113], true),
            ([0.137, 0.113, 3.55], false),
            ([0.137, 0.113, 3.65], true),
            ([3.1, 0.113, 1.113], false),
        ] {
            assert_eq!(
                contains(&template.stem, point),
                material,
                "MX stem material at {point:?}"
            );
        }
    }

    #[test]
    fn rectangular_mount_stems_are_closed_with_independent_blind_socket_material() {
        for (mount, expected_volume, probes) in [
            (
                KeycapMount::Alps,
                154.656,
                [
                    ([0.137, 0.113, 1.113], false),
                    ([2.5, 0.113, 1.113], true),
                    ([0.137, 1.5, 1.113], true),
                    ([0.137, 0.113, 3.55], false),
                    ([0.137, 0.113, 3.65], true),
                    ([3.1, 0.113, 1.113], false),
                ],
            ),
            (
                KeycapMount::ChocV1,
                170.778,
                [
                    ([2.85, 0.113, 1.113], false),
                    ([-2.85, 0.113, 1.113], false),
                    ([4.0, 0.113, 1.113], true),
                    ([2.85, 0.113, 2.25], false),
                    ([2.85, 0.113, 2.35], true),
                    ([0.137, 0.113, 1.113], false),
                ],
            ),
        ] {
            let mut spec = flat_g20();
            spec.mount = mount;
            let template = build_keycap_mesh_template(&spec).expect("valid rectangular mount");
            assert_valid_mesh(&template.stem);
            assert_oriented_closed_mesh(&template.stem);
            // Alps: 6×6×5.4 minus 4.6×2.4×3.6; Choc V1: two
            // 4.2×4.2×5.4 stems, each minus a 1.3×3.3×2.3 socket.
            let actual = signed_volume(&template.stem);
            assert!(
                (actual - expected_volume).abs() < 3e-5,
                "{mount:?} stem volume {actual}, expected {expected_volume}"
            );
            for (point, material) in probes {
                assert_eq!(
                    contains(&template.stem, point),
                    material,
                    "{mount:?} stem material at {point:?}"
                );
            }
        }
    }

    #[test]
    fn empty_legend_has_no_separate_mesh() {
        let template = build_keycap_mesh_template(&flat_g20()).expect("valid empty legend");
        assert!(template.legend.is_none());
    }

    fn assert_valid_mesh(mesh: &MeshData) {
        assert!(!mesh.positions.is_empty());
        assert_eq!(mesh.positions.len() % 9, 0);
        assert_eq!(mesh.positions.len(), mesh.normals.len());
        assert!(mesh.positions.iter().all(|value| value.is_finite()));
        assert!(mesh.normals.iter().all(|value| value.is_finite()));
        for normal in mesh.normals.as_chunks::<3>().0.iter() {
            let length =
                (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            assert!((length - 1.0).abs() < 1e-5, "normal length={length}");
        }
    }

    fn bounds(mesh: &MeshData) -> [[f64; 3]; 2] {
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        for point in mesh.positions.as_chunks::<3>().0.iter() {
            for axis in 0..3 {
                bounds[0][axis] = bounds[0][axis].min(f64::from(point[axis]));
                bounds[1][axis] = bounds[1][axis].max(f64::from(point[axis]));
            }
        }
        bounds
    }

    fn max_x_at_height(mesh: &MeshData, height: f64) -> f64 {
        mesh.positions
            .as_chunks::<3>()
            .0
            .iter()
            .filter_map(|point| {
                let x = f64::from(point[0]);
                let y = f64::from(point[1]);
                let z = f64::from(point[2]);
                ((y.abs() < 1e-5) && (z - height).abs() < 1e-5).then_some(x)
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn signed_volume(mesh: &MeshData) -> f64 {
        mesh.positions
            .as_chunks::<9>()
            .0
            .iter()
            .map(|triangle| {
                let [a, b, c] = [0, 1, 2].map(|vertex| {
                    [
                        f64::from(triangle[vertex * 3]),
                        f64::from(triangle[vertex * 3 + 1]),
                        f64::from(triangle[vertex * 3 + 2]),
                    ]
                });
                dot(a, cross(b, c)) / 6.0
            })
            .sum()
    }

    fn expected_shell_volume(spec: &KeycapSpec) -> f64 {
        let wall = spec.wall_thickness;
        loft_volume(
            spec.size.x,
            spec.size.y,
            spec.top_size.x,
            spec.top_size.y,
            spec.height,
        ) - loft_volume(
            spec.size.x - 2.0 * wall,
            spec.size.y - 2.0 * wall,
            spec.top_size.x - 2.0 * wall,
            spec.top_size.y - 2.0 * wall,
            spec.height - wall,
        )
    }

    fn loft_volume(
        bottom_width: f64,
        bottom_depth: f64,
        top_width: f64,
        top_depth: f64,
        height: f64,
    ) -> f64 {
        let area = |t: f64| {
            let width = bottom_width + (top_width - bottom_width) * t;
            let depth = bottom_depth + (top_depth - bottom_depth) * t;
            let radius = 0.8_f64.min(width / 5.0).min(depth / 5.0);
            // Four 90-degree corners are each polygonized with four 22.5-degree
            // chords, matching the retained rounded profile's stated sampling.
            width * depth - (4.0 - 8.0 * 22.5_f64.to_radians().sin()) * radius * radius
        };
        height * (area(0.0) + 4.0 * area(0.5) + area(1.0)) / 6.0
    }

    fn assert_oriented_closed_mesh(mesh: &MeshData) {
        let mut directed_edges = BTreeMap::new();
        for triangle in mesh.positions.as_chunks::<9>().0.iter() {
            let points = [0, 1, 2].map(|vertex| {
                [
                    (f64::from(triangle[vertex * 3]) * 100_000.0).round() as i64,
                    (f64::from(triangle[vertex * 3 + 1]) * 100_000.0).round() as i64,
                    (f64::from(triangle[vertex * 3 + 2]) * 100_000.0).round() as i64,
                ]
            });
            for edge in 0..3 {
                let a = points[edge];
                let b = points[(edge + 1) % 3];
                *directed_edges.entry((a, b)).or_insert(0_usize) += 1;
            }
        }
        for (&(a, b), &count) in &directed_edges {
            assert_eq!(
                count, 1,
                "directed mesh edge {a:?} -> {b:?} occurs {count} times"
            );
            assert_eq!(
                directed_edges.get(&(b, a)),
                Some(&1),
                "mesh edge {a:?} -> {b:?} has no oppositely oriented neighbor"
            );
        }
        assert!(
            signed_volume(mesh) > 0.0,
            "material shell winding is outward"
        );
    }

    fn surface_height_at(mesh: &MeshData, x: f64, y: f64) -> Option<f64> {
        let mut height = None;
        for triangle in mesh.positions.as_chunks::<9>().0.iter() {
            let [a, b, c] = [0, 1, 2].map(|vertex| {
                [
                    f64::from(triangle[vertex * 3]),
                    f64::from(triangle[vertex * 3 + 1]),
                    f64::from(triangle[vertex * 3 + 2]),
                ]
            });
            let denominator = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if denominator.abs() < 1e-12 {
                continue;
            }
            let wa = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / denominator;
            let wb = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / denominator;
            let wc = 1.0 - wa - wb;
            if wa >= -1e-8 && wb >= -1e-8 && wc >= -1e-8 {
                let sample = wa * a[2] + wb * b[2] + wc * c[2];
                height = Some(height.map_or(sample, |current: f64| current.max(sample)));
            }
        }
        height
    }

    fn contains(mesh: &MeshData, point: [f64; 3]) -> bool {
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

    fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
}
