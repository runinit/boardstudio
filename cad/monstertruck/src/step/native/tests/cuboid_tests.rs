//! Cuboids import natively. Only the source-decimal certificate over the
//! delivered asset claims zero repair, and it reaches the same verdict on the
//! legacy route, which is the oracle for every delivered field.
use super::*;

const BOX: &str = include_str!("../../../../tests/fixtures/zero-repair-cuboid-mm.step");
const OFFSET_BOX: &str =
    include_str!("../../../../tests/fixtures/zero-repair-cuboid-offset-mm.step");
const FREECAD_CUBE: &str = include_str!("../../../../tests/fixtures/freecad-cube.step");
const PCURVE_CUBE: &str = include_str!("../../../../tests/fixtures/source-pcurve-cube.step");

fn legacy(bytes: &[u8], options: StepImportOptions) -> Result<ImportedStep, String> {
    let exchange = parser::parse(&units::decode_step_bytes(bytes)).map_err(|e| e.to_string())?;
    let precision = units::precision::resolve(&exchange)?;
    let lengths = units::length_factors(&exchange)?;
    let items = units::representation_items(&exchange)?;
    drop(exchange);
    import_legacy(
        bytes.to_vec(),
        options,
        options
            .target_deflection_mm
            .map_or(TOLERANCE_MM, |target| target.min(TOLERANCE_MM)),
        precision,
        lengths,
        items,
    )
}

type Bounds = [[f64; 3]; 2];

fn replace_once(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "{old}");
    text.replace(old, new)
}

/// Per face occurrence: emitted area and the extents of its emitted points.
fn face_coverage(model: &ImportedStep) -> Vec<(f64, [[f64; 3]; 2])> {
    let mut faces =
        vec![(0.0, [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]]); model.face_occurrences.len()];
    for (&tag, points) in model
        .triangle_faces
        .iter()
        .zip(model.preview_positions_f64.as_chunks::<3>().0)
    {
        let [a, b, c] = [0, 1, 2].map(|i| Vector3::from(points[i]));
        faces[tag].0 += (b - a).cross(c - a).magnitude() * 0.5;
        for p in points {
            for (axis, value) in p.iter().enumerate() {
                faces[tag].1[0][axis] = faces[tag].1[0][axis].min(*value);
                faces[tag].1[1][axis] = faces[tag].1[1][axis].max(*value);
            }
        }
    }
    faces
}

/// Native delivery equals legacy delivery in every field, including the one
/// certificate's zero claim over each route's own triangles.
fn assert_matches_legacy(bytes: &[u8], options: StepImportOptions) -> ImportedStep {
    let native = import_step_with_options(bytes.to_vec(), options).unwrap();
    assert!(native.solids.is_empty(), "cuboid used vendor conversion");
    let legacy = legacy(bytes, options).unwrap();
    assert!(!legacy.solids.is_empty());
    assert_eq!(native.original_bytes(), bytes);
    assert_eq!(legacy.original_bytes(), bytes);
    assert_eq!(native.bounds(), legacy.bounds());
    assert_eq!(native.occurrences, legacy.occurrences);
    assert_eq!(native.source_faces(), legacy.source_faces());
    assert_eq!(native.face_occurrences(), legacy.face_occurrences());
    assert_eq!(native.precision(), legacy.precision());
    assert_eq!(native.accuracy(), legacy.accuracy());
    for ((native_area, native_extent), (legacy_area, legacy_extent)) in face_coverage(&native)
        .into_iter()
        .zip(face_coverage(&legacy))
    {
        assert!((native_area - legacy_area).abs() <= 1e-9 * legacy_area.max(1.0));
        for (n, l) in native_extent
            .iter()
            .flatten()
            .zip(legacy_extent.iter().flatten())
        {
            assert!((n - l).abs() <= 1e-9, "{native_extent:?} {legacy_extent:?}");
        }
    }
    assert!(native.validate_invariants().is_ok());
    native
}

#[test]
fn certified_cuboids_import_natively_and_match_legacy_delivery() {
    let declared = replace_once(
        BOX,
        "#303=(REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#302)));",
        "#303=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#302))GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#900)));",
    )
    .replace(
        "ENDSEC;END-ISO-10303-21;",
        "#900=UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001),#302,'distance_accuracy_value',$);ENDSEC;END-ISO-10303-21;",
    );
    // Equivalent authoring: the bottom PLANE faces up and the face reverses it.
    let reauthored = replace_once(
        &replace_once(
            BOX,
            "#106=DIRECTION('',(0.,0.,-1.));",
            "#106=DIRECTION('',(0.,0.,1.));",
        ),
        "#110=ADVANCED_FACE('',(#105),#109,.T.);",
        "#110=ADVANCED_FACE('',(#105),#109,.F.);",
    );
    let budget = StepImportOptions {
        preview_repair_budget_mm: Some(0.03),
        target_deflection_mm: Some(0.01),
    };
    for (text, options, bounds) in [
        (
            BOX,
            StepImportOptions::default(),
            [[0., 0., 0.], [2., 4., 8.]],
        ),
        (
            OFFSET_BOX,
            StepImportOptions::default(),
            [[-3., 2., -4.], [5., 8., 2.]],
        ),
        (declared.as_str(), budget, [[0., 0., 0.], [2., 4., 8.]]),
        (
            reauthored.as_str(),
            StepImportOptions::default(),
            [[0., 0., 0.], [2., 4., 8.]],
        ),
    ] {
        let model = assert_matches_legacy(text.as_bytes(), options);
        assert_eq!(model.bounds(), bounds);
        assert_eq!(model.accuracy().max_repair_displacement_mm, Some(0.0));
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
        assert_eq!(model.solid_count(), 1);
        assert_eq!(model.occurrence_count(), 1);
        assert_eq!(model.source_faces().len(), 6);
        assert_eq!(model.face_occurrences().len(), 6);
        assert_eq!(model.triangle_faces(), [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
    }
    let model = import_step(declared.into_bytes()).unwrap();
    assert_eq!(model.precision()[0].uncertainty_mm(), Some(0.001));
    assert_eq!(
        model.face_occurrences()[0].source_uncertainty_mm,
        Some(0.001)
    );
}

#[test]
fn bundled_exporter_cubes_import_natively_without_a_zero_claim() {
    for text in [FREECAD_CUBE, PCURVE_CUBE] {
        let model = assert_matches_legacy(text.as_bytes(), StepImportOptions::default());
        // Authored pcurves are outside the decimal certificate's grammar.
        assert_eq!(model.accuracy().max_repair_displacement_mm, None);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
        assert_eq!(model.source_faces().len(), 6);
    }
}

#[test]
fn near_cuboids_never_borrow_the_certificate() {
    let second = OFFSET_BOX
        .split_once("ENDSEC;DATA;")
        .unwrap()
        .1
        .strip_suffix("ENDSEC;END-ISO-10303-21;\n")
        .unwrap();
    let styled = replace_once(
        BOX,
        "ENDSEC;END-ISO-10303-21;",
        "#901=STYLED_ITEM('',(#904),#301);\n\
         #902=MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#901),#303);\n\
         #904=PRESENTATION_STYLE_ASSIGNMENT((#905));\n\
         #905=SURFACE_STYLE_USAGE(.BOTH.,#906);\n\
         #906=SURFACE_SIDE_STYLE('',(#907));\n\
         #907=SURFACE_STYLE_FILL_AREA(#908);\n\
         #908=FILL_AREA_STYLE('',(#909));\n\
         #909=FILL_AREA_STYLE_COLOUR('',#910);\n\
         #910=COLOUR_RGB('',1.,0.,0.);\n\
         ENDSEC;END-ISO-10303-21;",
    );
    // (case, mutated source, native route expected, expected bounds when imported)
    let cases: Vec<(&str, String, bool, Option<Bounds>)> = vec![
        (
            // Rounds to the same binary64 as the authored integer.
            "rounded-decimal",
            replace_once(
                BOX,
                "#2=CARTESIAN_POINT('',(2.,0.,0.));",
                "#2=CARTESIAN_POINT('',(2.0000000000000001,0.,0.));",
            ),
            true,
            Some([[0., 0., 0.], [2., 4., 8.]]),
        ),
        (
            // Within the native attachment epsilon, so the emitted mesh looks
            // like the box; the original decimal is not the box corner.
            "sub-epsilon-corner",
            replace_once(
                BOX,
                "#7=CARTESIAN_POINT('',(2.,4.,8.));",
                "#7=CARTESIAN_POINT('',(2.000000001,4.,8.));",
            ),
            true,
            None,
        ),
        (
            "half-integer-box",
            BOX.replace("CARTESIAN_POINT('',(2.,", "CARTESIAN_POINT('',(2.5,"),
            true,
            Some([[0., 0., 0.], [2.5, 4., 8.]]),
        ),
        (
            "outside-integer-range",
            BOX.replace("CARTESIAN_POINT('',(2.,", "CARTESIAN_POINT('',(2097152.,"),
            true,
            Some([[0., 0., 0.], [2097152., 4., 8.]]),
        ),
        (
            "metre-units",
            BOX.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)"),
            true,
            Some([[0., 0., 0.], [2000., 4000., 8000.]]),
        ),
        (
            "styled-solid",
            styled,
            true,
            Some([[0., 0., 0.], [2., 4., 8.]]),
        ),
        (
            "second-occurrence",
            replace_once(
                BOX,
                "ENDSEC;END-ISO-10303-21;",
                &format!("{second}ENDSEC;END-ISO-10303-21;"),
            ),
            true,
            Some([[-3., 0., -4.], [5., 8., 8.]]),
        ),
        (
            "face-bound-not-outer",
            replace_once(
                BOX,
                "#105=FACE_OUTER_BOUND('',#104,.T.);",
                "#105=FACE_BOUND('',#104,.T.);",
            ),
            true,
            Some([[0., 0., 0.], [2., 4., 8.]]),
        ),
        (
            "off-vertex-line",
            replace_once(
                BOX,
                "#32=LINE('',#1,#31);",
                "#900=CARTESIAN_POINT('',(0.,1.,0.));#32=LINE('',#900,#31);",
            ),
            false,
            None,
        ),
        (
            "offset-plane",
            replace_once(
                BOX,
                "#108=AXIS2_PLACEMENT_3D('',#1,#106,#107);",
                "#900=CARTESIAN_POINT('',(0.,0.,1.));#108=AXIS2_PLACEMENT_3D('',#900,#106,#107);",
            ),
            false,
            None,
        ),
        (
            "reversed-face-sense",
            replace_once(
                BOX,
                "#110=ADVANCED_FACE('',(#105),#109,.T.);",
                "#110=ADVANCED_FACE('',(#105),#109,.F.);",
            ),
            false,
            None,
        ),
        (
            "reversed-edge-sense",
            replace_once(
                BOX,
                "#33=EDGE_CURVE('',#11,#12,#32, .T.);",
                "#33=EDGE_CURVE('',#11,#12,#32, .F.);",
            ),
            false,
            None,
        ),
    ];
    for (name, text, native_route, bounds) in cases {
        assert_ne!(text, BOX, "{name}");
        let bytes = text.into_bytes();
        let result = import_step(bytes.clone());
        let route = match &result {
            Ok(model) if model.solids.is_empty() => "native",
            Ok(_) => "legacy",
            Err(_) => "refused",
        };
        eprintln!("cuboid-route {name}: {route} ({} bytes)", bytes.len());
        if native_route {
            assert_eq!(route, "native", "{name}: {:?}", result.as_ref().err());
        } else {
            assert_ne!(route, "native", "{name}");
        }
        if let Ok(model) = result {
            assert_eq!(model.original_bytes(), bytes, "{name}");
            assert_eq!(model.accuracy().max_repair_displacement_mm, None, "{name}");
            assert_eq!(model.accuracy().analysis_error_bound_mm, None, "{name}");
            if let Some(bounds) = bounds {
                assert_eq!(model.bounds(), bounds, "{name}");
            }
        }
        if let Ok(legacy) = legacy(&bytes, StepImportOptions::default()) {
            assert_eq!(legacy.accuracy().max_repair_displacement_mm, None, "{name}");
        }
    }
}

#[test]
fn gate_refused_cuboids_are_certified_on_legacy_delivery_without_native_admission() {
    let ap242 = replace_once(
        BOX,
        "FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));",
        "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 1 1 4 }'));",
    );
    let exchange = parser::parse(&ap242).unwrap();
    let refusal = build(
        ap242.as_bytes(),
        &exchange,
        &units::representation_items(&exchange).unwrap(),
        &units::length_factors(&exchange).unwrap(),
        &units::precision::resolve(&exchange).unwrap(),
        StepImportOptions::default(),
        TOLERANCE_MM,
    )
    .err()
    .unwrap();
    assert_eq!(refusal, "outside native analytic exchange class");
    let delivered = import_step(ap242.clone().into_bytes()).unwrap();
    assert!(!delivered.solids.is_empty(), "AP242 must not be admitted natively");
    let native = import_step(BOX.as_bytes().to_vec()).unwrap();
    assert!(native.solids.is_empty());
    assert_eq!(delivered.original_bytes(), ap242.as_bytes());
    assert_eq!(delivered.bounds(), native.bounds());
    assert_eq!(delivered.source_faces(), native.source_faces());
    assert_eq!(delivered.face_occurrences(), native.face_occurrences());
    assert_eq!(delivered.precision(), native.precision());
    assert_eq!(delivered.accuracy(), native.accuracy());
    assert_eq!(delivered.accuracy().max_repair_displacement_mm, Some(0.0));
}
