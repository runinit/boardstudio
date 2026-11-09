use super::*;
use monstertruck_meshing::{
    analyzers::CalcVolume,
    rexport_polymesh::{PolygonMesh, StandardVertex},
    tessellation::{MeshableShape, shell_to_polygon_strict},
};
use monstertruck_modeling::topology::ShellCondition;
use std::collections::BTreeMap;

const MESH_TOLERANCE: f64 = 0.005;
const ANALYTIC_RELATIVE_TOLERANCE: f64 = 0.01;

fn checked_mesh_volume(solid: &Solid, label: &str) -> f64 {
    assert!(
        solid.is_geometric_consistent(),
        "{label}: inconsistent solid"
    );
    assert!(!solid.boundaries().is_empty(), "{label}: no shells");
    let bounds = monstertruck_modeling::bounding::certified_solid_bounding_box(solid)
        .unwrap_or_else(|| panic!("{label}: no certified bounds"));
    assert!(
        (0..3).all(|i| bounds.min()[i].is_finite() && bounds.max()[i].is_finite()),
        "{label}: non-finite bounds"
    );

    let mut total = 0.0;
    for (shell_index, shell) in solid.boundaries().iter().enumerate() {
        assert_eq!(
            shell.shell_condition(),
            ShellCondition::Closed,
            "{label}: shell {shell_index} is not closed"
        );
        let face_meshes = shell.triangulation(MESH_TOLERANCE);
        for (face_index, face) in face_meshes.face_iter().enumerate() {
            assert!(
                face.surface()
                    .is_some_and(|mesh| mesh.faces().triangle_iter().next().is_some()),
                "{label}: empty/missing mesh on face {face_index}"
            );
        }
        let mesh = shell_to_polygon_strict(shell, MESH_TOLERANCE)
            .unwrap_or_else(|error| panic!("{label}: strict tessellation: {error:?}"));
        let volume = mesh.volume();
        assert!(
            volume.is_finite() && volume > 0.0,
            "{label}: signed volume {volume}"
        );
        let (open_edges, degenerate_edges) = mesh_edge_issues(&mesh);
        assert_eq!(
            open_edges, 0,
            "{label}: open/orientation-invalid mesh edges"
        );
        assert_eq!(degenerate_edges, 0, "{label}: degenerate mesh edges");
        total += volume;
    }
    assert!(
        total.is_finite() && total > 0.0,
        "{label}: non-positive total volume"
    );
    total
}

fn mesh_edge_issues(mesh: &PolygonMesh) -> (usize, usize) {
    type PointKey = [i64; 3];
    let point_key = |vertex: StandardVertex| {
        let point = mesh.positions()[vertex.pos];
        [point.x, point.y, point.z].map(|value| (value / 1.0e-7).round() as i64)
    };
    let mut edges = BTreeMap::<[PointKey; 2], (usize, i32)>::new();
    let mut degenerate = 0;
    for face in mesh.face_iter() {
        assert_eq!(face.len(), 3, "mesh contains a non-triangle face");
    }
    for triangle in mesh.faces().triangle_iter() {
        for index in 0..3 {
            let a = point_key(triangle[index]);
            let b = point_key(triangle[(index + 1) % 3]);
            if a == b {
                degenerate += 1;
                continue;
            }
            let (key, direction) = if a < b { ([a, b], 1) } else { ([b, a], -1) };
            let usage = edges.entry(key).or_default();
            usage.0 += 1;
            usage.1 += direction;
        }
    }
    let invalid = edges
        .values()
        .filter(|&&(uses, direction)| uses != 2 || direction != 0)
        .count();
    (invalid, degenerate)
}

fn primitive_volume_oracle(spherical: bool, radius: f64, height: f64, length: f64) -> f64 {
    if spherical && height >= 2.0 * radius {
        (4.0 / 3.0) * std::f64::consts::PI * radius.powi(3)
    } else if spherical {
        std::f64::consts::PI * height.powi(2) * (radius - height / 3.0)
    } else if height >= 2.0 * radius {
        std::f64::consts::PI * radius.powi(2) * length
    } else {
        let z = radius - height;
        (radius.powi(2) * (z / radius).acos() - z * (height * (2.0 * radius - height)).sqrt())
            * length
    }
}

#[test]
fn primitive_segments_match_independent_closed_form_volume_oracle() {
    let cases = [
        (4.0, 0.1),
        (4.0, 1.0),
        (4.0, 4.0),
        (4.0, 7.0),
        (4.0, 8.0),
        (33.83269230769231, 0.66),
    ];
    let length = 18.2;
    let mut passed = 0;
    for spherical in [true, false] {
        for (radius, height) in cases {
            let solid = if spherical {
                spherical_segment(radius, height).unwrap()
            } else {
                cylindrical_segment(radius, height, length).unwrap()
            };
            let measured = checked_mesh_volume(&solid, "primitive segment");
            let expected = primitive_volume_oracle(spherical, radius, height, length);
            let relative_error = (measured - expected).abs() / expected;
            assert!(
                relative_error <= ANALYTIC_RELATIVE_TOLERANCE,
                "primitive={spherical} radius={radius} height={height}: mesh volume {measured}, closed-form {expected}, relative error {relative_error}"
            );
            if spherical {
                let spherical_faces = solid
                    .boundaries()
                    .iter()
                    .flat_map(|shell| shell.face_iter())
                    .filter(|face| matches!(face.surface(), Surface::SphericalSurface(_)))
                    .count();
                assert!(spherical_faces >= 4, "analytic spherical surface was lost");
            }
            println!(
                "primitive_mesh_volume_check primitive={} radius={radius} height={height} mesh_volume={measured:.12} analytic_volume={expected:.12} relative_error={relative_error:.8}",
                if spherical { "sphere" } else { "cylinder" }
            );
            passed += 1;
        }
    }
    assert_eq!(passed, 12);
}

#[test]
fn transformed_dish_cutter_profiles_remain_certified_closed_solids() {
    let base = profile("dsa").unwrap();
    let profiles = [
        Profile {
            label: "narrow+12",
            top_w: 10.0,
            tilt: 12.0,
            ..base
        },
        Profile {
            label: "narrow-12",
            top_w: 10.0,
            tilt: -12.0,
            ..base
        },
        Profile {
            label: "full-small",
            width: 150.0,
            depth: 150.0,
            top_w: 5.0,
            top_d: 5.0,
            height: 3.0,
            tilt: 12.0,
            dish_depth: 0.8,
            ..base
        },
        Profile {
            label: "maximum",
            width: 150.0,
            depth: 150.0,
            top_w: 148.0,
            top_d: 148.0,
            height: 20.0,
            tilt: -12.0,
            dish_depth: 0.8,
            ..base
        },
        Profile {
            label: "strong-taper",
            width: 150.0,
            depth: 150.0,
            tilt: 12.0,
            dish_depth: 0.001,
            ..base
        },
    ];
    let mut passed = 0;
    for profile in profiles {
        for spherical in [true, false] {
            let p = Profile {
                spherical,
                ..profile
            };
            let solid = dish_cutter(p).unwrap().expect("positive dish depth");
            let volume = checked_mesh_volume(&solid, p.label);
            let half_top_depth = p.top_d / 2.0;
            let radius = (half_top_depth.powi(2) + p.dish_depth.powi(2)) / (2.0 * p.dish_depth);
            // The rounded loft's top ring is z=height+y*tan(theta), so its q projection
            // is zero. The base control hull reaches q=|sin(theta)|*depth/2-cos(theta)*height.
            // This independent closed form is the support ceiling; the geometric cutter
            // segment extends another dish_depth below it. The constructor's directed
            // rounding and the mesh approximation remain inside the unchanged 1% allowance.
            let (sin, cos) = p.tilt.to_radians().sin_cos();
            let base_support = sin.abs() * p.depth / 2.0 - cos * p.height;
            let ceiling = base_support.max(0.0) + 0.01;
            let segment_height = ceiling + p.dish_depth;
            let cylinder_length = 2.0 * p.width;
            let expected =
                primitive_volume_oracle(spherical, radius, segment_height, cylinder_length);
            let relative_error = (volume - expected).abs() / expected;
            assert!(
                relative_error <= ANALYTIC_RELATIVE_TOLERANCE,
                "transformed cutter label={} primitive={} radius={radius} ceiling={ceiling} segment_height={segment_height} length={cylinder_length}: mesh volume={volume}, analytic volume={expected}, relative error={relative_error}",
                p.label,
                if spherical { "sphere" } else { "cylinder" }
            );
            println!(
                "transformed_cutter_volume_check label={} primitive={} radius={radius:.12} ceiling={ceiling:.12} segment_height={segment_height:.12} length={cylinder_length:.12} mesh_volume={volume:.12} analytic_volume={expected:.12} relative_error={relative_error:.8}",
                p.label,
                if spherical { "sphere" } else { "cylinder" }
            );
            passed += 1;
        }
    }
    assert_eq!(passed, 10);
}

#[test]
fn support_bound_covers_the_324_reviewed_loft_profiles() {
    let mut cases = 0;
    for tilt in [-12.0, 0.0, 12.0] {
        for width in [12.0, 18.2, 150.0] {
            for depth in [12.0, 18.2, 150.0] {
                for top_w in [5.0, width - 2.0] {
                    for top_d in [5.0, depth - 2.0] {
                        for height in [3.0, 7.5, 20.0] {
                            let p = Profile {
                                label: "support",
                                width,
                                depth,
                                top_w,
                                top_d,
                                height,
                                tilt,
                                dish_depth: 0.8,
                                wall: 0.8,
                                spherical: true,
                            };
                            let bound = cutter_ceiling(p);
                            let (sin, cos) = tilt.to_radians().sin_cos();
                            let lower: Vec<_> = sampled_wire(width, depth, 0.0, 0.0)
                                .edge_iter()
                                .map(|edge| edge.front().point())
                                .collect();
                            let upper: Vec<_> = sampled_wire(top_w, top_d, height, tilt)
                                .edge_iter()
                                .map(|edge| edge.front().point())
                                .collect();
                            for edge_index in 0..20 {
                                for j in 0..=8 {
                                    for k in 0..=8 {
                                        let u = j as f64 / 8.0;
                                        let v = k as f64 / 8.0;
                                        let a = lower[edge_index]
                                            + (lower[(edge_index + 1) % 20] - lower[edge_index])
                                                * u;
                                        let b = upper[edge_index]
                                            + (upper[(edge_index + 1) % 20] - upper[edge_index])
                                                * u;
                                        let point = a + (b - a) * v;
                                        let q = -sin * point.y + cos * (point.z - height);
                                        assert!(
                                            q < bound,
                                            "support sample outside bound: label={} edge={edge_index} u={u} v={v} q={q} bound={bound}",
                                            p.label
                                        );
                                    }
                                }
                            }
                            cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 324);
}

#[test]
fn source_material_witness_is_included_and_zero_depth_bypasses_build() {
    let mut witness = Profile {
        label: "side-material",
        width: 150.0,
        depth: 150.0,
        top_w: 13.2,
        top_d: 13.2,
        height: 7.5,
        tilt: 12.0,
        dish_depth: 0.001,
        wall: 1.2,
        spherical: true,
    };
    let (sin, cos) = witness.tilt.to_radians().sin_cos();
    let (y, z) = (-74.7, 0.02);
    let q = -sin * y + cos * (z - witness.height);
    let yy = cos * y + sin * (z - witness.height);
    let radius = dish_radius(witness);
    let implicit =
        yy * yy + q * q - 2.0 * q * (radius - witness.dish_depth) - (witness.top_d / 2.0).powi(2);
    let outer_y =
        -75.0 + (75.0 - 6.6) * z / (witness.height - 6.6 * witness.tilt.to_radians().tan());
    let inner_y = -73.8
        + (73.8 - 5.4) * (z + 0.1)
            / (witness.height
                - witness.dish_depth
                - witness.wall
                - 5.4 * witness.tilt.to_radians().tan()
                + 0.1);
    assert!(outer_y < y && y < inner_y && implicit < 0.0 && q > 0.01);
    assert!(
        cutter_ceiling(witness) > q,
        "support bound excludes material"
    );
    witness.dish_depth = 0.0;
    assert!(dish_cutter(witness).unwrap().is_none());
}

#[test]
fn positive_depth_scale_refusals_remain_explicit_for_both_primitives() {
    let base = profile("dsa").unwrap();
    for spherical in [true, false] {
        for dish_depth in [f64::from_bits(1), 1e-16, 7.316180623782399e-15, 1e-10] {
            let result = dish_cutter(Profile {
                dish_depth,
                spherical,
                ..base
            });
            assert!(
                result.is_err(),
                "positive but uncertifiable depth {dish_depth} must refuse; spherical={spherical}"
            );
        }
    }
}

#[test]
fn tilted_cutters_pass_actual_closure_certificate_and_reject_inward_shift() {
    for spherical in [true, false] {
        let p = Profile {
            width: 150.0,
            depth: 150.0,
            top_w: 13.2,
            top_d: 13.2,
            height: 7.5,
            tilt: 12.0,
            dish_depth: 0.001,
            wall: 1.2,
            spherical,
            label: "side-material-regression",
        };
        let cutter = dish_cutter(p).unwrap().unwrap();
        certification::certify_cutter(p, &cutter, true).unwrap();
        let (sin, cos) = p.tilt.to_radians().sin_cos();
        let shifted = builder::translated(&cutter, Vector3::new(0.0, 0.02 * sin, -0.02 * cos));
        let error = certification::certify_cutter(p, &shifted, true).unwrap_err();
        assert!(
            error.contains("actual transformed closing plane"),
            "{error}"
        );
    }
}

#[test]
fn stored_sphere_arc_error_is_refused() {
    let p = profile("dsa").unwrap();
    let cutter = dish_cutter(p).unwrap().unwrap();
    let shifted = builder::translated(&cutter, Vector3::new(0.001, 0.0, 0.0));
    let error = certification::certify_cutter(p, &shifted, true).unwrap_err();
    assert!(
        error.contains("primitive/arc/transform error bound"),
        "{error}"
    );
}
