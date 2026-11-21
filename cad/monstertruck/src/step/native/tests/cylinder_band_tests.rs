use super::*;

const CONNECTOR: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/vik/3d_models/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp"
);
// Two complete connector solids with their original analytic supports,
// edge IDs, vertices, styles, and AP203 unit and product contexts.
const BANDS: &str = include_str!("../../../../tests/fixtures/vik-connector-bands.step");

#[test]
fn source_connector_cylindrical_bands_use_both_complete_rims() {
    let mut previous_triangles = 0;
    for deflection in [0.1, 0.001] {
        let model = native_model(
            BANDS.as_bytes(),
            StepImportOptions {
                target_deflection_mm: Some(deflection),
                ..Default::default()
            },
        );
        assert_eq!(model.solid_count(), 2);
        assert_eq!(model.occurrence_count(), 2);
        assert_eq!(model.source_faces().len(), 15);
        assert_eq!(model.face_occurrences().len(), 15);
        for (actual, expected) in model
            .bounds()
            .into_iter()
            .flatten()
            .zip([7.2, -5.25, 0.2, 7.7, -0.75, 2.1])
        {
            assert!((actual - expected).abs() < 1e-10);
        }
        let (positions, remainder) = model.preview_positions_f64.as_chunks::<3>();
        assert!(remainder.is_empty());
        let triangles: Vec<_> = positions.iter().map(|p| p.map(Point3::from)).collect();
        assert!(triangles.len() > previous_triangles);
        previous_triangles = triangles.len();
        assert!(crate::mesh::closed_oriented_edges(&triangles, |p| {
            Some([p.x, p.y, p.z].map(|v| if v == 0.0 { 0 } else { v.to_bits() }))
        }));

        // The AP203 surfaces #7269 and #11518 share an axis in the X direction.
        // #717 is the outside of a pin and #6108 is the inside of a recess.
        for (id, from, to, radius, sense) in [
            (717, 7.2_f64, 7.5_f64, 0.35, 1.0),
            (6108, 7.3_f64, 7.5_f64, 0.4, -1.0),
        ] {
            let mut area = 0.0;
            for (&p, &tag) in triangles.iter().zip(&model.triangle_faces) {
                let face = &model.source_faces[model.face_occurrences[tag].source_face];
                if face.face_id != id {
                    continue;
                }
                let radial = |v: Point3| Vector3::new(0.0, v.y + 4.0, v.z - 1.4);
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                area += n.magnitude() * 0.5;
                for v in p {
                    assert!((v.x - from).abs().min((v.x - to).abs()) < 1e-10);
                    assert!((radial(v).magnitude() - radius).abs() < 1e-10);
                }
                for i in 0..=6 {
                    for j in 0..=6 - i {
                        let r = (radial(p[0]) * i as f64
                            + radial(p[1]) * j as f64
                            + radial(p[2]) * (6 - i - j) as f64)
                            / 6.0;
                        assert!((r.magnitude() - radius).abs() <= deflection);
                        assert!(
                            n.normalize().dot(r.normalize()) * sense >= ANGULAR_TOLERANCE.cos()
                        );
                    }
                }
            }
            let exact = std::f64::consts::TAU * radius * (to - from);
            assert!(
                area > 0.99 * exact && area < exact,
                "band #{id} area {area}"
            );
        }
    }
}

#[test]
fn cylindrical_bands_refuse_moved_misaligned_or_reversed_trims() {
    for (from, to, error) in [
        (
            "#7269=CYLINDRICAL_SURFACE('',#7270,0.35);",
            "#7269=CYLINDRICAL_SURFACE('',#7270,0.36);",
            "source circular boundary is off its CYLINDRICAL_SURFACE",
        ),
        (
            "#22951=CARTESIAN_POINT('',(7.2,-3.65,1.4));",
            "#22951=CARTESIAN_POINT('',(7.2,-4.35,1.4));",
            "native cylindrical band rims are not aligned",
        ),
        (
            "#717=ADVANCED_FACE('',(#7265,#7267),#7269,.T.);",
            "#717=ADVANCED_FACE('',(#7265,#7267),#7269,.F.);",
            "native cylindrical band exceeds facet tolerance",
        ),
    ] {
        assert_eq!(BANDS.matches(from).count(), 1);
        let changed = BANDS.replace(from, to);
        assert_eq!(
            native_candidate(changed.as_bytes(), StepImportOptions::default())
                .err()
                .unwrap(),
            format!("native face #717: {error}")
        );
    }
}

#[test]
fn malformed_ap203_application_context_refuses_public_import() {
    let valid = "APPLICATION_CONTEXT('configuration controlled 3d designs of mechanical parts and assemblies')";
    assert_eq!(BANDS.matches(valid).count(), 1);
    let invalid = BANDS.replace(valid, "APPLICATION_CONTEXT(123)");
    assert!(import_step(invalid.as_bytes().to_vec()).is_err());
    assert!(native_candidate(invalid.as_bytes(), StepImportOptions::default()).is_err());
}

#[test]
fn cylindrical_band_refinement_and_bounds_use_mm() {
    let options = StepImportOptions {
        target_deflection_mm: Some(0.001),
        ..Default::default()
    };
    let original = native_model(BANDS.as_bytes(), options);
    let mm_assignment = "SI_UNIT(.MILLI.,.METRE.)";
    assert_eq!(BANDS.matches(mm_assignment).count(), 1);
    let centimetres = BANDS.replace(mm_assignment, "SI_UNIT(.CENTI.,.METRE.)");
    let scaled = native_model(centimetres.as_bytes(), options);
    assert!(scaled.triangle_faces.len() > original.triangle_faces.len());
    for (actual, expected) in scaled
        .bounds()
        .into_iter()
        .flatten()
        .zip(original.bounds().into_iter().flatten())
    {
        assert!((actual - 10.0 * expected).abs() < 1e-10);
    }

    let error = native_candidate(
        BANDS.as_bytes(),
        StepImportOptions {
            target_deflection_mm: Some(1e-8),
            ..Default::default()
        },
    )
    .err()
    .expect("oversized rim subdivision must fail closed");
    assert!(error.contains("native conic subdivision limit"), "{error}");
}

#[test]
fn full_connector_keeps_the_vendor_route_for_detached_source_lines() {
    let error = native_candidate(CONNECTOR, StepImportOptions::default())
        .err()
        .expect("detached source lines must prevent native admission");
    assert!(
        error.contains("source boundary is off its oriented LINE"),
        "{error}"
    );
    let model = import_step(CONNECTOR.to_vec()).unwrap();
    assert!(!model.solids.is_empty());
    assert_eq!(model.original_bytes(), CONNECTOR);
    assert_eq!(model.face_occurrences().len(), 740);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}
