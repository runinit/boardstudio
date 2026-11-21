use super::*;
use std::f64::consts::PI;

// OCCT 7.9 BRepPrimAPI_MakeCone at (3,-2,7), +Z, radii 4/2, height 5.
// Exported via STEPControl_Writer AP214IS with surface curves enabled.
const FRUSTUM: &str = include_str!("../../../../tests/fixtures/occt-frustum-pcurves.step");
// Same OCCT export with top radius zero.
const APEX: &str = include_str!("../../../../tests/fixtures/occt-cone-apex.step");
// FreeCAD 1.1.4 Part.makeCone with the same dimensions, cut by a 60° sector
// from azimuth 40°, spanning z = 8..10. Shape.exportStep writes AP214.
const NOTCHED: &str = include_str!("../../../../tests/fixtures/freecad-notched-frustum.step");

fn replace_once(source: &str, from: &str, to: &str) -> String {
    assert_eq!(source.matches(from).count(), 1);
    source.replace(from, to)
}

fn measure(model: &ImportedStep, deflection: f64, slope: f64, notched: bool) -> (f64, f64) {
    let volume_origin = Point3::new(3.0, -2.0, 7.0);
    let mut volume = 0.0;
    let mut area = 0.0;
    for (points, &tag) in model
        .preview_positions_f64
        .chunks_exact(3)
        .zip(&model.triangle_faces)
    {
        let p = [0, 1, 2].map(|i| Point3::from(points[i]) - volume_origin);
        let normal = (p[1] - p[0]).cross(p[2] - p[0]);
        volume += p[0].dot(p[1].cross(p[2])) / 6.0;
        let face = &model.source_faces[model.face_occurrences[tag].source_face];
        if face.support_kind != "CONICAL_SURFACE" {
            continue;
        }
        area += normal.magnitude() * 0.5;
        for vertex in p {
            let radial = vertex.x.hypot(vertex.y);
            assert!((radial - (4.0 - slope * vertex.z)).abs() < 1e-8);
        }
        for (a, b) in [(p[0], p[1]), (p[1], p[2]), (p[2], p[0])] {
            let a = Vector3::new(a.x, a.y, 0.0);
            let b = Vector3::new(b.x, b.y, 0.0);
            if a.magnitude2() > 1e-16 && b.magnitude2() > 1e-16 {
                assert!(a.normalize().dot(b.normalize()) >= ANGULAR_TOLERANCE.cos() - 1e-10);
            }
        }
        for i in 0..=8 {
            for j in 0..=8 - i {
                let q = (p[0] * i as f64 + p[1] * j as f64 + p[2] * (8 - i - j) as f64) / 8.0;
                let radial = q.x.hypot(q.y);
                assert!(((4.0 - slope * q.z) - radial).abs() <= deflection + 1e-8);
                if radial > 1e-8 {
                    assert!(normal.dot(Vector3::new(q.x / radial, q.y / radial, slope)) > 0.0);
                }
                let angle = q.y.atan2(q.x).to_degrees();
                assert!(
                    !notched
                        || !(40.0 + 1e-7..100.0 - 1e-7).contains(&angle)
                        || !(1.0 + 1e-8..3.0 - 1e-8).contains(&q.z),
                    "cone triangle crosses the authored hole at {angle} degrees, z={}",
                    q.z,
                );
            }
        }
    }
    assert!(area > 0.0);
    (volume, area)
}

#[test]
fn exported_conical_apex_seam_and_hole_keep_shared_boundaries() {
    for (source, faces, slope, notched) in [
        (APEX, 2, 0.8_f64, false),
        (FRUSTUM, 3, 0.4, false),
        (NOTCHED, 7, 0.4, true),
    ] {
        let (mut prior_volume_error, mut prior_area_error) = (f64::INFINITY, f64::INFINITY);
        let mut previous_triangles = 0;
        let small_radius = 4.0 - 5.0 * slope;
        let volume = 5.0 * PI / 3.0 * (16.0 + 4.0 * small_radius + small_radius * small_radius)
            - if notched {
                (PI / 6.0) * (2.0 / 3.0) * (3.6 * 3.6 + 3.6 * 2.8 + 2.8 * 2.8)
            } else {
                0.0
            };
        let area = (5.0 * PI * (4.0 + small_radius)
            - if notched { (PI / 3.0) * 2.0 * 3.2 } else { 0.0 })
            * (1.0 + slope * slope).sqrt();
        for deflection in [0.1, 0.001] {
            let model = native_model(
                source.as_bytes(),
                StepImportOptions {
                    target_deflection_mm: Some(deflection),
                    ..Default::default()
                },
            );
            assert_eq!(model.solid_count(), 1);
            assert_eq!(model.occurrence_count(), 1);
            assert_eq!(model.face_occurrences().len(), faces);
            assert_eq!(model.bounds(), [[-1.0, -6.0, 7.0], [7.0, 2.0, 12.0]]);
            let triangles: Vec<_> = model
                .preview_positions_f64
                .chunks_exact(3)
                .map(|p| [0, 1, 2].map(|i| Point3::from(p[i])))
                .collect();
            assert!(crate::mesh::closed_oriented_edges(&triangles, |p| {
                Some([p.x, p.y, p.z].map(|v| if v == 0.0 { 0 } else { v.to_bits() }))
            }));
            let (mesh_volume, mesh_area) = measure(&model, deflection, slope, notched);
            let volume_error = (mesh_volume - volume).abs();
            let area_error = (mesh_area - area).abs();
            assert!(volume_error < prior_volume_error && area_error < prior_area_error);
            if deflection == 0.001 {
                assert!(volume_error / volume < 0.001, "{mesh_volume} vs {volume}");
                assert!(area_error / area < 0.001, "{mesh_area} vs {area}");
            }
            assert!(model.triangle_faces.len() > previous_triangles);
            previous_triangles = model.triangle_faces.len();
            prior_volume_error = volume_error;
            prior_area_error = area_error;
        }
    }
}

#[test]
fn thin_frustum_with_mismatched_rim_counts_refuses_tilted_facets() {
    let source = FRUSTUM
        .replace("(5.,-2.,12.)", "(6.999999,-2.,7.001)")
        .replace("(3.,-2.,12.)", "(3.,-2.,7.001)")
        .replace("#26,2.)", "#26,3.999999)")
        .replace("#50,2.)", "#50,3.999999)")
        .replace(
            "4.,0.380506377112)",
            &format!("4.,{:.17})", 0.001_f64.atan()),
        )
        .replace("(-0.,-5.)", "(-0.,-0.001)")
        .replace(
            "(-0.371390676354,9.096448060604E-17,0.928476690885)",
            "(-0.001,0.,1.)",
        );
    let options = StepImportOptions {
        target_deflection_mm: Some(0.0874095861385776),
        ..Default::default()
    };
    let error = native_candidate(source.as_bytes(), options).err().unwrap();
    assert!(
        error.contains("facet normal exceeds angular tolerance"),
        "{error}"
    );
}

#[test]
fn native_cone_uses_representation_angle_and_length_units() {
    let degrees = replace_once(
        FRUSTUM,
        "4.,0.380506377112)",
        &format!("4.,{:.15})", 0.380506377112_f64.to_degrees()),
    );
    let degrees = replace_once(&degrees, "(-6.28318530718,0.)", "(-360.,0.)");
    let degrees = replace_once(
        &degrees,
        "ENDSEC;\nEND-ISO-10303-21;",
        "#200 = ( CONVERSION_BASED_UNIT('DEGREE',#201) NAMED_UNIT(#202) PLANE_ANGLE_UNIT() );\n\
         #201 = PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(0.017453292519943295),#115);\n\
         #202 = DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.);\nENDSEC;\nEND-ISO-10303-21;",
    );
    let degrees = replace_once(&degrees, "(#114,#115,#116)", "(#114,#200,#116)");
    let degree_model = native_model(degrees.as_bytes(), StepImportOptions::default());
    let radian_model = native_model(FRUSTUM.as_bytes(), StepImportOptions::default());
    assert_eq!(degree_model.triangle_faces, radian_model.triangle_faces);
    for (degree, radian) in degree_model
        .preview_positions_f64
        .iter()
        .zip(&radian_model.preview_positions_f64)
    {
        assert!((Point3::from(*degree) - Point3::from(*radian)).magnitude() < 1e-10);
    }

    let centimetres = replace_once(
        &degrees,
        "SI_UNIT(.MILLI.,.METRE.)",
        "SI_UNIT(.CENTI.,.METRE.)",
    );
    let centimetre_model = native_model(centimetres.as_bytes(), StepImportOptions::default());
    assert_eq!(
        centimetre_model.bounds(),
        radian_model.bounds().map(|p| p.map(|v| v * 10.0))
    );
    assert!(centimetre_model.triangle_faces.len() > radian_model.triangle_faces.len());
}

#[test]
fn native_cone_refuses_conflicting_carriers_and_trim_winding() {
    let options = StepImportOptions::default();
    for (from, to, reason) in [
        ("(-6.28318530718,0.)", "(0.,0.)", "not one period apart"),
        ("(-0.,-5.)", "(-0.,-4.)", "pcurve disagrees"),
        ("4.,0.380506377112)", "4.,1.570796326795)", "semi-angle"),
    ] {
        let mutated = replace_once(FRUSTUM, from, to);
        let error = native_candidate(mutated.as_bytes(), options).err().unwrap();
        assert!(error.contains(reason), "{error}");
    }
    let flipped_hole = replace_once(
        NOTCHED,
        "#45 = FACE_BOUND('',#46,.T.);",
        "#45 = FACE_BOUND('',#46,.F.);",
    );
    assert!(native_candidate(flipped_hole.as_bytes(), options).is_err());
    let off_cone = replace_once(NOTCHED, "4.,0.380506377112)", "4.,0.4)");
    let error = native_candidate(off_cone.as_bytes(), options)
        .err()
        .unwrap();
    assert!(error.contains("off its conical chart"), "{error}");
}

#[test]
fn native_cone_keeps_product_occurrence_placement() {
    let placed = replace_once(
        FRUSTUM,
        "ENDSEC;\nEND-ISO-10303-21;",
        "#200 = CARTESIAN_POINT('',(10.,20.,30.));\n\
         #201 = DIRECTION('',(1.,0.,0.));\n\
         #202 = DIRECTION('',(0.,1.,0.));\n\
         #203 = AXIS2_PLACEMENT_3D('',#200,#201,#202);\n\
         #204 = SHAPE_REPRESENTATION('',(#203),#113);\n\
         #205 = PRODUCT_DEFINITION('design','',#206,#9);\n\
         #206 = PRODUCT_DEFINITION_FORMATION('','',#207);\n\
         #207 = PRODUCT('assembly','assembly','',(#8));\n\
         #208 = PRODUCT_DEFINITION_SHAPE('','',#205);\n\
         #209 = SHAPE_DEFINITION_REPRESENTATION(#208,#204);\n\
         #210 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('1','rotated cone','',#205,#5,$);\n\
         #211 = PRODUCT_DEFINITION_SHAPE('','',#210);\n\
         #212 = CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#213,#211);\n\
         #213 = ( REPRESENTATION_RELATIONSHIP('','',#10,#204)\n\
           REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#214) SHAPE_REPRESENTATION_RELATIONSHIP() );\n\
         #214 = ITEM_DEFINED_TRANSFORMATION('','',#11,#203);\nENDSEC;\nEND-ISO-10303-21;",
    );
    let standalone = native_model(FRUSTUM.as_bytes(), StepImportOptions::default());
    let assembly = native_model(placed.as_bytes(), StepImportOptions::default());
    assert_eq!(assembly.bounds(), [[17.0, 19.0, 24.0], [22.0, 27.0, 32.0]]);
    assert_eq!(standalone.triangle_faces, assembly.triangle_faces);
    assert_eq!(standalone.source_faces, assembly.source_faces);
    for ([x, y, z], placed) in standalone
        .preview_positions_f64
        .iter()
        .zip(&assembly.preview_positions_f64)
    {
        assert_eq!(*placed, [z + 10.0, x + 20.0, y + 30.0]);
    }
}

#[test]
fn bundled_power_switch_matches_legacy_catalogue_and_source_bounds() {
    let source = include_bytes!(
        "../../../../../../ergogen/library/vendor/infused-kim/3d_models/Switch_Power.step"
    );
    let options = StepImportOptions::default();
    let native = native_model(source, options);
    let exchange = parser::parse(&units::decode_step_bytes(source)).unwrap();
    let legacy = import_legacy(
        source.to_vec(),
        options,
        TOLERANCE_MM,
        units::precision::resolve(&exchange).unwrap(),
        units::length_factors(&exchange).unwrap(),
        units::representation_items(&exchange).unwrap(),
    )
    .unwrap();
    assert_eq!(native.solid_count(), 5);
    assert_eq!(native.occurrence_count(), legacy.occurrence_count());
    let catalogue = |model: &ImportedStep| {
        model
            .face_occurrences
            .iter()
            .map(|f| {
                let source = &model.source_faces[f.source_face];
                (
                    source.solid_id,
                    source.face_id,
                    source.support_id,
                    f.representation_id,
                )
            })
            .collect::<HashSet<_>>()
    };
    assert_eq!(catalogue(&native), catalogue(&legacy));
    assert_eq!(native.face_occurrences.len(), 143);
    let cone_ids = native
        .source_faces
        .iter()
        .filter(|f| f.support_kind == "CONICAL_SURFACE")
        .map(|f| f.support_id)
        .collect::<HashSet<_>>();
    assert_eq!(cone_ids, HashSet::from([1025, 1949, 2556, 2848]));
    // Independently measured with OCCT via FreeCAD Part.read.
    let expected = [[-3.8608, -0.508, -2.1844], [3.8608, 1.397, 2.794]];
    for model in [&native, &legacy] {
        for (actual, expected) in model
            .bounds()
            .into_iter()
            .flatten()
            .zip(expected.into_iter().flatten())
        {
            assert!((actual - expected).abs() < 1e-10);
        }
    }
}
