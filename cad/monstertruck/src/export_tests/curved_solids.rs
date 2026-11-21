use super::*;

fn circular_extrusion_step(shape: crate::step_accuracy::CircularExtrusion) -> String {
    circular_profile_step(shape, &[shape.radius])
}

fn annular_extrusion_step(shape: crate::step_accuracy::CircularExtrusion, inner: f64) -> String {
    circular_profile_step(shape, &[shape.radius, inner])
}

fn circular_profile_step(shape: crate::step_accuracy::CircularExtrusion, radii: &[f64]) -> String {
    use monstertruck_modeling::{Curve, Point3, Surface, Vector3, primitive, profile};
    let rings = radii
        .iter()
        .map(|radius| {
            primitive::circle::<Curve>(
                Point3::new(shape.center[0] + radius, shape.center[1], shape.z[0]),
                Point3::new(shape.center[0], shape.center[1], shape.z[0]),
                Vector3::unit_z(),
                2,
            )
        })
        .collect();
    let solid = profile::solid_from_planar_profile::<Curve, Surface>(
        rings,
        Vector3::new(0.0, 0.0, shape.z[1] - shape.z[0]),
    )
    .unwrap();
    step_text(&crate::ExactModel {
        solids: vec![solid],
    })
}

fn is_true(value: &Parameter) -> bool {
    matches!(value, Parameter::Enumeration(sense) if sense == "T")
}

#[test]
fn exact_circle_extrusion_has_independent_volume_material_and_topology_certificate() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let text = circular_extrusion_step(CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 3.0],
    });
    let certificate = certify_circular_extrusion(
        &text,
        CircularExtrusion {
            center: [0.0, 0.0],
            radius: 2.0,
            z: [0.0, 3.0],
        },
    )
    .expect("independent complete circular-extrusion source certificate");
    let analytic_volume = 12.0 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 1.0e-5);
    assert_eq!(certificate.bounds, [[-2.0, -2.0, 0.0], [2.0, 2.0, 3.0]]);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.topology, [4, 6, 4]);
    assert_eq!(certificate.contains([0.0, 0.0, 1.0]), Some(true));
    assert_eq!(certificate.contains([1.0, 1.0, 2.0]), Some(true));
    assert_eq!(certificate.contains([1.5, 1.5, 1.0]), Some(false));
    assert_eq!(certificate.contains([0.0, 0.0, 3.1]), Some(false));
    assert_eq!(certificate.contains([2.0, 0.0, 1.0]), None);
}

#[test]
fn circular_certificate_checks_interior_source_weights_and_complete_trim_topology() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 3.0],
    };
    let original = circular_extrusion_step(shape);
    let distort_interior = map_points(&original, |mut p| {
        if (p[1] - 2.0 / 3.0).abs() < 1.0e-12 {
            p[1] += 0.01;
        }
        p
    });
    assert_ne!(distort_interior, original);
    assert!(certify_circular_extrusion(&distort_interior, shape).is_err());
    for (old, new) in [
        (
            "RATIONAL_B_SPLINE_CURVE((1.0, 0.75",
            "RATIONAL_B_SPLINE_CURVE((1.0, 0.76",
        ),
        (
            "RATIONAL_B_SPLINE_SURFACE(((1.0, 1.0), (0.75, 0.75)",
            "RATIONAL_B_SPLINE_SURFACE(((1.0, 1.0), (0.75, 0.76)",
        ),
        (".CURVE_3D.", ".PCURVE_S1."),
        (
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#3, #4, #5))",
            "GLOBAL_UNIT_ASSIGNED_CONTEXT((#999999, #4, #5))",
        ),
    ] {
        assert!(
            original.contains(old),
            "negative control must edit actual source: {old}"
        );
        let bad = original.replacen(old, new, 1);
        assert!(certify_circular_extrusion(&bad, shape).is_err(), "{old}");
    }
    let records = StepRecords::parse(&original);
    let (shell, params) = records.named("CLOSED_SHELL").next().unwrap();
    let last_face = id(list(&params[1]).last().unwrap());
    let (edge_use, _) = records
        .named("ORIENTED_EDGE")
        .find(|(_, p)| is_true(&p[4]))
        .unwrap();
    for broken in [
        edit_record(&original, shell, &format!(", #{last_face}"), ""),
        edit_record(&original, edge_use, ".T.", ".F."),
    ] {
        assert!(certify_circular_extrusion(&broken, shape).is_err());
    }
    let (representation, _) = records.named("SHAPE_REPRESENTATION").next().unwrap();
    let extra_graph = append_step_records(
        &original,
        &format!("#9000=REPRESENTATION_RELATIONSHIP('','',#{representation},#{representation});"),
    );
    assert!(certify_circular_extrusion(&extra_graph, shape).is_err());
    let wrong_radius = CircularExtrusion {
        radius: 1.9,
        ..shape
    };
    assert!(certify_circular_extrusion(&original, wrong_radius).is_err());
    let wrong_height = CircularExtrusion {
        z: [0.0, 3.01],
        ..shape
    };
    assert!(certify_circular_extrusion(&original, wrong_height).is_err());
}

#[test]
fn circular_certificate_preserves_fractional_translated_circle_geometry() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let shape = CircularExtrusion {
        center: [11.25, -7.5],
        radius: 1.25,
        z: [-1.0, 1.5],
    };
    let text = circular_extrusion_step(shape);
    let certificate = certify_circular_extrusion(&text, shape).unwrap();
    let analytic_volume = 3.90625 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.1);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.contains([11.25, -7.5, 0.0]), Some(true));
    assert_eq!(certificate.contains([12.25, -6.5, 0.0]), Some(false));
}

#[test]
fn external_freecad_cylinder_has_independent_analytic_and_selected_chart_certificate() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../../tests/fixtures/freecad-cylinder.step");
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 10.0],
    };
    let certificate =
        certify_circular_extrusion(source, shape).expect("external analytic cylinder source");
    let analytic_volume = 40.0 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.1);
    assert_eq!(certificate.topology, [2, 3, 3]);
    assert_eq!(certificate.bounds, [[-2.0, -2.0, 0.0], [2.0, 2.0, 10.0]]);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.contains([0.0, 0.0, 5.0]), Some(true));
    assert_eq!(certificate.contains([1.5, 1.5, 5.0]), Some(false));
    let reparametrized = source.replace(
        "#38 = CARTESIAN_POINT('',(0.,10.));",
        "#38 = CARTESIAN_POINT('',(13.4,10.));",
    );
    assert_ne!(reparametrized, source);
    assert!(certify_circular_extrusion(&reparametrized, shape).is_ok());
    let second_master = source.replace(".PCURVE_S1.", ".PCURVE_S2.");
    assert_ne!(second_master, source);
    assert!(certify_circular_extrusion(&second_master, shape).is_ok());
    let angle_units = source.replace("SI_UNIT($,.RADIAN.)", "SI_UNIT($,.DEGREE.)");
    assert_ne!(angle_units, source);
    assert!(certify_circular_extrusion(&angle_units, shape).is_err());
    for (old, new) in [
        (
            "#38 = CARTESIAN_POINT('',(0.,10.));",
            "#38 = CARTESIAN_POINT('',(0.,10.01));",
        ),
        (
            "#66 = CARTESIAN_POINT('',(6.28318530718,-0.));",
            "#66 = CARTESIAN_POINT('',(6.28,-0.));",
        ),
        (
            "#51 = CARTESIAN_POINT('',(0.,0.));",
            "#51 = CARTESIAN_POINT('',(0.01,0.));",
        ),
        (
            "#31 = CYLINDRICAL_SURFACE('',#32,2.);",
            "#31 = CYLINDRICAL_SURFACE('',#32,2.01);",
        ),
    ] {
        assert!(source.contains(old));
        let bad = source.replacen(old, new, 1);
        assert!(certify_circular_extrusion(&bad, shape).is_err(), "{old}");
    }
}

#[test]
fn circular_certificate_refuses_malformed_source_records_without_panicking() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../../tests/fixtures/freecad-cylinder.step");
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 10.0],
    };
    for (old, new) in [
        (
            "#25 = CIRCLE('',#26,2.);",
            "#25 = (CIRCLE('',#26,2.)CIRCLE('',#26,3.));",
        ),
        (
            "#25 = CIRCLE('',#26,2.);",
            "#25 = (CIRCLE('',#26,2.)PCURVE('',#31,#36));",
        ),
        ("CYLINDRICAL_SURFACE('',#32,2.)", "CYLINDRICAL_SURFACE()"),
    ] {
        let malformed = source.replace(old, new);
        assert_ne!(malformed, source, "{old}");
        assert!(
            certify_circular_extrusion(&malformed, shape).is_err(),
            "accepted {new}"
        );
    }
}

#[test]
fn circular_certificate_checks_both_seam_endpoints_within_reported_bound() {
    use crate::step_accuracy::{CircularExtrusion, Error, certify_circular_extrusion};
    let broken = include_str!("../../tests/fixtures/off-seam-top-vertex.step");
    assert!(
        matches!(
            certify_circular_extrusion(
                broken,
                CircularExtrusion {
                    center: [0.0, 0.0],
                    radius: 2.0,
                    z: [0.0, 10.0],
                }
            ),
            Err(Error::Geometry)
        ),
        "top seam vertex cannot exceed the reported physical bound"
    );

    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 2.0,
        z: [0.0, 10.0],
    };
    let within_bound = broken.replace("(2.,0.00000005,10.)", "(2.,0.00000000005,10.)");
    assert_ne!(within_bound, broken);
    let accepted = certify_circular_extrusion(&within_bound, shape).unwrap();
    assert_eq!(
        accepted.bound_error, 8.0e-10,
        "the physical bound was not enlarged"
    );
    let opposite = broken.replace("(2.,0.00000005,10.)", "(2.,-0.00000005,10.)");
    assert_ne!(opposite, broken);
    assert!(matches!(
        certify_circular_extrusion(&opposite, shape),
        Err(Error::Geometry)
    ));
    let original = include_str!("../../tests/fixtures/freecad-cylinder.step");
    let bottom = original.replace(
        "#57 = CARTESIAN_POINT('',(2.,-4.898587196589E-16,0.));",
        "#57 = CARTESIAN_POINT('',(2.,0.00000005,0.));",
    );
    assert_ne!(bottom, original);
    assert!(matches!(
        certify_circular_extrusion(&bottom, shape),
        Err(Error::Geometry)
    ));
}

#[test]
fn true_annulus_export_has_independent_curved_drill_volume_and_material() {
    use crate::step_accuracy::{CircularExtrusion, certify_annular_extrusion};
    let text = annular_extrusion_step(
        CircularExtrusion {
            center: [0.0, 0.0],
            radius: 3.0,
            z: [0.0, 3.0],
        },
        1.4,
    );
    let certificate = certify_annular_extrusion(
        &text,
        CircularExtrusion {
            center: [0.0, 0.0],
            radius: 3.0,
            z: [0.0, 3.0],
        },
        1.4,
    )
    .expect("complete independent true circular annulus source certificate");
    let analytic_volume = 21.12 * std::f64::consts::PI;
    assert!(certificate.volume[0] <= analytic_volume && certificate.volume[1] >= analytic_volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 1.0e-5);
    assert_eq!(certificate.topology, [8, 12, 6]);
    assert_eq!(certificate.bounds, [[-3.0, -3.0, 0.0], [3.0, 3.0, 3.0]]);
    assert!(certificate.bound_error < 0.01);
    assert_eq!(certificate.contains([0.0, 0.0, 1.5]), Some(false));
    assert_eq!(certificate.contains([1.3, 0.0, 1.5]), Some(false));
    assert_eq!(certificate.contains([1.45, 0.0, 1.5]), Some(true));
    assert_eq!(certificate.contains([3.1, 0.0, 1.5]), Some(false));
    assert_eq!(certificate.contains([1.4, 0.0, 1.5]), None);
}

#[test]
fn annular_certificate_rejects_filled_drill_wrong_winding_and_interior_drift() {
    use crate::step_accuracy::{
        CircularExtrusion, certify_annular_extrusion, certify_circular_extrusion,
    };
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 3.0,
        z: [0.0, 3.0],
    };
    let source = annular_extrusion_step(shape, 1.4);
    assert!(
        certify_circular_extrusion(&source, shape).is_err(),
        "an annulus is not a filled disk"
    );
    assert!(
        certify_annular_extrusion(&circular_extrusion_step(shape), shape, 1.4).is_err(),
        "a requested drill must exist in source"
    );
    assert!(
        certify_annular_extrusion(&annular_extrusion_step(shape, 1.3), shape, 1.4).is_err(),
        "wrong drill geometry must not reuse expected geometry"
    );
    let drift = map_points(&source, |mut point| {
        if (point[1] - 1.4 / 3.0).abs() < 1.0e-12 {
            point[1] += 0.01;
        }
        point
    });
    assert_ne!(drift, source);
    assert!(
        certify_annular_extrusion(&drift, shape, 1.4).is_err(),
        "interior drill curvature is checked beyond its endpoints"
    );
    let records = StepRecords::parse(&source);
    let (_, cap) = records
        .named("PLANE")
        .find(|(_, p)| {
            let placement = records.args(id(&p[1]), "AXIS2_PLACEMENT_3D");
            records.xyz(id(&placement[1]), "CARTESIAN_POINT")[2] == 0.0
        })
        .unwrap();
    let placement = records.args(id(&cap[1]), "AXIS2_PLACEMENT_3D");
    let (origin, normal) = (id(&placement[1]), id(&placement[2]));
    let (face, _) = records
        .named("ADVANCED_FACE")
        .find(|(_, p)| {
            !is_true(&p[3])
                && records
                    .named("RATIONAL_B_SPLINE_SURFACE")
                    .any(|(surface, _)| surface == id(&p[2]))
        })
        .unwrap();
    let (bound, _) = records
        .named("FACE_BOUND")
        .find(|(_, p)| !is_true(&p[2]))
        .unwrap();
    let tilt =
        |step: &str, y| replace_record(step, normal, &format!("DIRECTION('', (0.0, {y}, 1.0));"));
    let [x, y, _] = records.xyz(origin, "CARTESIAN_POINT");
    for broken in [
        tilt(&source, 1e-8),
        replace_record(
            &source,
            origin,
            &format!("CARTESIAN_POINT('', ({x:?}, {y:?}, 0.00000005));"),
        ),
        edit_record(&source, face, ".F.", ".T."),
        edit_record(&source, bound, ".F.", ".T."),
    ] {
        assert!(certify_annular_extrusion(&broken, shape, 1.4).is_err());
    }

    let rebased = replace_record(&source, origin, "CARTESIAN_POINT('', (0.0, 0.0, 0.0));");
    assert!(
        certify_annular_extrusion(&rebased, shape, 1.4).is_ok(),
        "an in-plane origin rebase preserves the cap geometry"
    );
    assert!(
        certify_annular_extrusion(&tilt(&rebased, 1e-8), shape, 1.4).is_err(),
        "the entire cap plane is checked, not just its height at the center"
    );
    let bounded_tilt = tilt(&source, 1e-12);
    let bounded = certify_annular_extrusion(&bounded_tilt, shape, 1.4).unwrap();
    assert!(bounded.bound_error < 0.01);
    let analytic_volume = 21.12 * std::f64::consts::PI;
    assert!(bounded.volume[0] <= analytic_volume && bounded.volume[1] >= analytic_volume);
}

#[test]
fn annular_boss_component_keeps_legacy_analytic_volume_delta() {
    use crate::step_accuracy::{CircularExtrusion, certify_annular_extrusion};
    let shape = CircularExtrusion {
        center: [0.0, 0.0],
        radius: 3.0,
        z: [-7.2, -6.2],
    };
    let source = annular_extrusion_step(shape, 1.1);
    let certificate = certify_annular_extrusion(&source, shape, 1.1).unwrap();
    let amount = &expected("module-support-annular-boss")["expect"]["volumeDeltaVersusPlain"];
    let value = amount["value"].as_f64().unwrap();
    let tolerance = amount["tolerance"].as_f64().unwrap();
    assert!(
        (certificate.volume[0] - value).abs() <= tolerance
            && (certificate.volume[1] - value).abs() <= tolerance
    );
    assert_eq!(certificate.contains([0.0, 0.0, -6.5]), Some(false));
    assert_eq!(certificate.contains([2.0, 0.0, -6.5]), Some(true));
    assert_eq!(certificate.bounds, [[-3.0, -3.0, -7.2], [3.0, 3.0, -6.2]]);
}

#[test]
fn frozen_013_countersink_has_independent_raw_volume_material_and_complete_trims() {
    use crate::step_accuracy::{Countersink, certify_countersink};
    // Captured unchanged public writer output; dimensions are the frozen PreparedCaseIR workload.
    let text = include_str!("../../tests/fixtures/countersink-013.step");
    let expected = Countersink {
        bounds: [[0., 0., 0.], [20., 16., 4.]],
        center: [10., 8.],
        z: [2., 4.],
        radii: [2., 1.],
    };
    let certificate = certify_countersink(text, expected).unwrap();
    assert_eq!(certificate.bounds, expected.bounds);
    let volume = 1265.3392342832476;
    assert!(certificate.volume[0] <= volume && certificate.volume[1] >= volume);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.002);
    assert_eq!(certificate.topology, [16, 20, 8]);
    for (point, material) in [
        ([10., 8., 1.], true),
        ([10., 8., 3.], false),
        ([11.7, 8., 2.5], false),
        ([11.7, 8., 3.5], true),
        ([21., 8., 2.], false),
    ] {
        assert_eq!(certificate.contains(point), Some(material));
    }
}

#[test]
fn countersink_certificate_refuses_changed_tensor_trim_material_and_source_graph() {
    use crate::step_accuracy::{Countersink, certify_countersink};
    let source = include_str!("../../tests/fixtures/countersink-013.step");
    let shape = Countersink {
        bounds: [[0., 0., 0.], [20., 16., 4.]],
        center: [10., 8.],
        z: [2., 4.],
        radii: [2., 1.],
    };
    let (old, new) = (
        "#157 = CARTESIAN_POINT('', (12.0, 10.0, 2.0));",
        "#157 = CARTESIAN_POINT('', (12.0, 10.01, 2.0));",
    );
    assert_eq!(source.matches(old).count(), 1);
    assert!(
        certify_countersink(&source.replacen(old, new, 1), shape).is_err(),
        "accepted {new}"
    );
}

#[test]
fn frozen_source_circle_periodic_cylinder_has_independent_8pi_certificate() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    // Authored source(center10,8,radius2,thickness2), not its diamond preview or built solid.
    let source = include_str!("../../tests/fixtures/source-circle-006.step");
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    let certificate = certify_circular_extrusion(source, shape).unwrap();
    let expected = 25.132741228718345;
    assert!(certificate.volume[0] <= expected && certificate.volume[1] >= expected);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.002);
    assert_eq!(certificate.bounds, [[8., 6., 0.], [12., 10., 2.]]);
    assert_eq!(certificate.topology, [4, 4, 3]);
    for (point, material) in [
        ([10., 8., 1.], true),
        ([11.3, 9.3, 1.], true),
        ([12.1, 8., 1.], false),
        ([10., 8., 2.1], false),
    ] {
        assert_eq!(certificate.contains(point), Some(material));
    }
    assert_eq!(certificate.contains([12., 8., 1.]), None);
}

#[test]
fn periodic_source_circle_refuses_geometry_and_directed_source_loss() {
    use crate::step_accuracy::{CircularExtrusion, certify_circular_extrusion};
    let source = include_str!("../../tests/fixtures/source-circle-006.step");
    let shape = CircularExtrusion {
        center: [10., 8.],
        radius: 2.,
        z: [0., 2.],
    };
    let (old, new) = (
        "#66 = CARTESIAN_POINT('', (8.0, 8.666666666666666, 0.0));",
        "#66 = CARTESIAN_POINT('', (8.0, 8.676666666666666, 0.0));",
    );
    assert_eq!(source.matches(old).count(), 1);
    assert!(
        certify_circular_extrusion(&source.replacen(old, new, 1), shape).is_err(),
        "accepted {new}"
    );
}
