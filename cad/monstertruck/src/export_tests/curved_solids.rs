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
