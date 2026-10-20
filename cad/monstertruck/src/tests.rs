use crate::{ProfileInput, ProfileKind, ProfileRegion, build_profile, mesh_profile};
use boardstudio_contracts::Vec2;
use serde_json::Value;
use std::collections::HashMap;

fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn region(outer: Vec<Vec2>, holes: Vec<Vec<Vec2>>) -> ProfileRegion {
    ProfileRegion { outer, holes }
}

fn input(kind: ProfileKind, regions: Vec<ProfileRegion>) -> ProfileInput {
    ProfileInput {
        kind,
        base_z: 4.0,
        thickness: 2.0,
        wall_height: 3.0,
        regions,
    }
}

fn json_ring(value: &Value) -> Vec<Vec2> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|pair_value| {
            let pair = pair_value.as_array().unwrap();
            point(pair[0].as_f64().unwrap(), pair[1].as_f64().unwrap())
        })
        .collect()
}

fn ring_area(points: &[Vec2]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f64>()
        .abs()
        * 0.5
}

fn expected_volume(region: &ProfileRegion, height: f64) -> f64 {
    (ring_area(&region.outer) - region.holes.iter().map(|hole| ring_area(hole)).sum::<f64>())
        * height
}

fn assert_model_matches(
    input: ProfileInput,
    expected_volumes: &[f64],
    xy_bounds: [[f64; 2]; 2],
    z_bounds: [f64; 2],
) {
    let model = build_profile(input).expect("valid prepared profile");
    assert_eq!(model.solids.len(), expected_volumes.len());
    for solid in &model.solids {
        assert!(solid.is_geometric_consistent());
        assert_eq!(solid.boundaries().len(), 1);
    }
    let mesh = mesh_profile(&model).expect("complete strict profile mesh");
    assert_mesh_closed_and_oriented(&mesh);
    assert_eq!(mesh.positions.len(), mesh.normals.len());
    assert_eq!(mesh.positions.len() % 9, 0);
    assert!(
        mesh.positions
            .iter()
            .chain(&mesh.normals)
            .all(|value| value.is_finite())
    );

    let actual_volume = signed_mesh_volume(&mesh);
    let expected = expected_volumes.iter().sum::<f64>();
    assert!(actual_volume > 0.0, "mesh winding must point outward");
    assert!(
        (actual_volume - expected).abs() <= expected.max(1.0) * 1.0e-5,
        "mesh material volume {actual_volume}, analytic profile volume {expected}"
    );

    let bounds = mesh_bounds(&mesh);
    for axis in 0..2 {
        assert!((bounds[0][axis] - xy_bounds[0][axis]).abs() < 1.0e-4);
        assert!((bounds[1][axis] - xy_bounds[1][axis]).abs() < 1.0e-4);
    }
    assert!((bounds[0][2] - z_bounds[0]).abs() < 1.0e-4);
    assert!((bounds[1][2] - z_bounds[1]).abs() < 1.0e-4);
}

fn assert_mesh_closed_and_oriented(mesh: &crate::MeshBuffers) {
    let mut edges = HashMap::<([u32; 3], [u32; 3]), (usize, isize)>::new();
    for triangle in mesh.positions.as_chunks::<9>().0 {
        let point = |index: usize| {
            [0, 1, 2].map(|axis| {
                let value = triangle[index * 3 + axis];
                if value == 0.0 { 0 } else { value.to_bits() }
            })
        };
        for index in 0..3 {
            let a = point(index);
            let b = point((index + 1) % 3);
            let (key, orientation) = if a <= b { ((a, b), 1) } else { ((b, a), -1) };
            let edge = edges.entry(key).or_default();
            edge.0 += 1;
            edge.1 += orientation;
        }
    }
    assert!(!edges.is_empty());
    assert!(
        edges
            .values()
            .all(|(uses, orientation)| *uses == 2 && *orientation == 0)
    );
    for normal in mesh.normals.as_chunks::<3>().0 {
        let length = normal
            .iter()
            .map(|value| f64::from(*value).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!((length - 1.0).abs() < 1.0e-5);
    }
}

fn signed_mesh_volume(mesh: &crate::MeshBuffers) -> f64 {
    mesh.positions
        .as_chunks::<9>()
        .0
        .iter()
        .map(|triangle| {
            let p = |index: usize| {
                [
                    f64::from(triangle[index * 3]),
                    f64::from(triangle[index * 3 + 1]),
                    f64::from(triangle[index * 3 + 2]),
                ]
            };
            let a = p(0);
            let b = p(1);
            let c = p(2);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

fn mesh_bounds(mesh: &crate::MeshBuffers) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for point in mesh.positions.as_chunks::<3>().0 {
        for axis in 0..3 {
            let coordinate = f64::from(point[axis]);
            bounds[0][axis] = bounds[0][axis].min(coordinate);
            bounds[1][axis] = bounds[1][axis].max(coordinate);
        }
    }
    bounds
}

#[test]
fn concave_profile_is_winding_independent_and_uses_tray_height() {
    let value: Value = serde_json::from_str(include_str!(
        "../../monstertruck-qualification/fixtures/profile-topology/rotated-concave.json"
    ))
    .unwrap();
    let mut outer = json_ring(&value["outer"]);
    let expected_region = region(outer.clone(), vec![]);
    let expected = expected_volume(&expected_region, 5.0);
    let mut reversed = outer.clone();
    reversed.reverse();
    outer = reversed;
    assert_model_matches(
        ProfileInput {
            kind: ProfileKind::Tray,
            base_z: 4.0,
            thickness: 2.0,
            wall_height: 3.0,
            regions: vec![region(outer, vec![])],
        },
        &[expected],
        [
            [-29.237170472273675, 0.0],
            [172.13485607334638, 148.25738244639615],
        ],
        [4.0, 9.0],
    );
}

#[test]
fn multiple_holes_and_reversed_loops_keep_the_analytic_material() {
    let value: Value = serde_json::from_str(include_str!(
        "../../monstertruck-qualification/fixtures/profile-topology/multi-hole-region.json"
    ))
    .unwrap();
    let outer = json_ring(&value["outer"]);
    let holes = value["holes"]
        .as_array()
        .unwrap()
        .iter()
        .map(json_ring)
        .collect::<Vec<_>>();
    assert_eq!(holes.len(), 35);
    let expected_region = region(outer.clone(), holes.clone());
    let expected = expected_volume(&expected_region, 2.0);
    let mut reversed_outer = outer;
    reversed_outer.reverse();
    let mut reversed_holes = holes;
    for hole in &mut reversed_holes {
        hole.reverse();
    }
    assert_model_matches(
        input(
            ProfileKind::Plate,
            vec![region(reversed_outer, reversed_holes)],
        ),
        &[expected],
        [[-2.5, -2.5], [147.5, 107.5]],
        [4.0, 6.0],
    );
}

#[test]
fn separated_regions_remain_separate_exact_solids() {
    let value: Value = serde_json::from_str(include_str!(
        "../../monstertruck-qualification/fixtures/profile-topology/split-regions.json"
    ))
    .unwrap();
    let regions = value["islands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|island| region(json_ring(island), vec![]))
        .collect::<Vec<_>>();
    let expected = regions
        .iter()
        .map(|region| expected_volume(region, 5.0))
        .collect::<Vec<_>>();
    assert_model_matches(
        input(ProfileKind::Lid, regions),
        &expected,
        [[0.0, 0.0], [300.0, 70.0]],
        [4.0, 9.0],
    );
}

#[test]
fn self_crossing_profiles_are_rejected_as_invalid_contours() {
    let crossed_with_nonzero_area = build_profile(input(
        ProfileKind::Plate,
        vec![region(
            vec![
                point(0.0, 0.0),
                point(4.0, 4.0),
                point(1.0, 3.0),
                point(4.0, 0.0),
            ],
            vec![],
        )],
    ));
    let error = match crossed_with_nonzero_area {
        Ok(_) => panic!("crossing contour must not create a solid"),
        Err(error) => error,
    };
    assert!(error.contains("self-intersecting"), "{error}");

    let zero_area_bow_tie = build_profile(input(
        ProfileKind::Plate,
        vec![region(
            vec![
                point(0.0, 0.0),
                point(4.0, 4.0),
                point(0.0, 4.0),
                point(4.0, 0.0),
            ],
            vec![],
        )],
    ));
    let error = match zero_area_bow_tie {
        Ok(_) => panic!("zero-area bow-tie must not create a solid"),
        Err(error) => error,
    };
    assert!(error.contains("zero-area ring"), "{error}");
}

#[test]
fn non_finite_contours_are_rejected() {
    let error = match build_profile(input(
        ProfileKind::Plate,
        vec![region(
            vec![point(0.0, 0.0), point(f64::NAN, 1.0), point(0.0, 2.0)],
            vec![],
        )],
    )) {
        Ok(_) => panic!("non-finite contour must not create a solid"),
        Err(error) => error,
    };
    assert!(error.contains("not finite"), "{error}");
}

#[test]
fn holes_must_be_strictly_inside_and_disjoint() {
    let outer = vec![
        point(0.0, 0.0),
        point(10.0, 0.0),
        point(10.0, 10.0),
        point(0.0, 10.0),
    ];
    let outside = vec![
        point(11.0, 1.0),
        point(12.0, 1.0),
        point(12.0, 2.0),
        point(11.0, 2.0),
    ];
    let outside_error = match build_profile(input(
        ProfileKind::Plate,
        vec![region(outer.clone(), vec![outside])],
    )) {
        Ok(_) => panic!("outside hole must be rejected"),
        Err(error) => error,
    };
    assert!(outside_error.contains("strictly inside"), "{outside_error}");

    let first = vec![
        point(1.0, 1.0),
        point(7.0, 1.0),
        point(7.0, 7.0),
        point(1.0, 7.0),
    ];
    let nested = vec![
        point(2.0, 2.0),
        point(3.0, 2.0),
        point(3.0, 3.0),
        point(2.0, 3.0),
    ];
    let nested_error = match build_profile(input(
        ProfileKind::Plate,
        vec![region(outer, vec![first, nested])],
    )) {
        Ok(_) => panic!("nested holes must be rejected"),
        Err(error) => error,
    };
    assert!(
        nested_error.contains("must not be nested"),
        "{nested_error}"
    );
}

#[test]
fn profile_mesh_rejects_triangles_collapsed_by_render_precision() {
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 1_000_000_000.0,
        thickness: 2.0,
        wall_height: 0.0,
        regions: vec![region(
            vec![
                point(0.0, 0.0),
                point(4.0, 0.0),
                point(4.0, 3.0),
                point(0.0, 3.0),
            ],
            vec![],
        )],
    })
    .expect("the exact solid can represent a two-millimetre plate at this elevation");
    let error =
        mesh_profile(&model).expect_err("f32 collapses the top and bottom at this elevation");
    assert!(error.contains("render precision"), "{error}");
}

#[test]
fn render_conversion_rejects_nonzero_triangles_with_reversed_winding() {
    use monstertruck_modeling::Point3;

    let points = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0000088214874268, 1.0000065565109253, 0.0),
        Point3::new(0.9999908208847046, 0.9999885747553706, 0.0),
    ];
    let mut buffers = crate::MeshBuffers::default();
    let error = crate::mesh::push_triangle(&mut buffers, points)
        .expect_err("f32 reverses this counter-clockwise triangle without collapsing it");
    assert!(error.contains("render precision"), "{error}");
}
