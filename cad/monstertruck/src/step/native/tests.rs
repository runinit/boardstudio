use super::*;

const MOLEX: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/infused-kim/3d_models/Molex_Ezmate_Pico_Socket_2pin.step"
);
const JST: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/kicad/3d_models/JST_PH_S2B-PH-K_1x02_P2.00mm_Horizontal.step"
);
const NICE_VIEW: &[u8] =
    include_bytes!("../../../../../ergogen/library/vendor/infused-kim/3d_models/Nice_View.step");
const RESISTOR: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Resistor.step"
);
const SECTOR: &str = include_str!("../../../tests/fixtures/trimmed-cylinder-sector.step");

fn native_model(bytes: &[u8], options: StepImportOptions) -> ImportedStep {
    let exchange = parser::parse(&units::decode_step_bytes(bytes)).unwrap();
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let precision = units::precision::resolve(&exchange).unwrap();
    let candidate = build(
        &exchange,
        &items,
        &factors,
        &precision,
        options,
        options.target_deflection_mm.unwrap_or(TOLERANCE_MM),
    );
    assert!(candidate.is_ok(), "{}", candidate.err().unwrap());
    let model = import_step_with_options(bytes.to_vec(), options).unwrap();
    assert!(
        model.solids.is_empty(),
        "entry point unexpectedly used vendor conversion"
    );
    assert_eq!(model.original_bytes(), bytes);
    assert!(model.accuracy.mesh_closed_oriented);
    assert!(model.accuracy.bounds_from_source_geometry);
    assert_eq!(model.accuracy.analysis_error_bound_mm, None);
    assert_eq!(model.accuracy.max_repair_displacement_mm, None);
    assert_eq!(model.accuracy.fallback_meshed_faces, 0);
    assert_eq!(
        model.triangle_faces.len() * 3,
        model.preview_positions_f64.len()
    );
    assert_eq!(
        model.triangle_faces.iter().copied().collect::<HashSet<_>>(),
        (0..model.face_occurrences.len()).collect()
    );
    model
}

#[test]
fn bundled_analytic_assets_use_native_geometry_and_keep_every_scene_face() {
    for (bytes, solids, faces, cylinders) in
        [(MOLEX, 1, 176, 4), (JST, 1, 110, 6), (RESISTOR, 4, 32, 0)]
    {
        let model = native_model(bytes, StepImportOptions::default());
        assert_eq!(model.solid_count(), solids);
        assert_eq!(model.occurrence_count(), solids);
        assert_eq!(model.source_faces.len(), faces);
        assert_eq!(model.face_occurrences.len(), faces);
        assert_eq!(
            model
                .source_faces
                .iter()
                .filter(|f| f.support_kind == "CYLINDRICAL_SURFACE")
                .count(),
            cylinders,
        );
    }
}

#[test]
fn round_plane_trim_leaves_the_bundled_plated_hole_open() {
    let model = native_model(NICE_VIEW, StepImportOptions::default());
    let tag = model
        .face_occurrences
        .iter()
        .position(|occurrence| model.source_faces[occurrence.source_face].face_id == 1041)
        .unwrap();
    // Face #1041 is the annulus centered at (7, 1.3) in the z=0.01 plane.
    // The authored hole radius is 0.4064; exact winding excludes its center.
    let mut area = 0.0;
    for (triangle, _) in model
        .preview_positions_f64
        .chunks_exact(3)
        .zip(&model.triangle_faces)
        .filter(|(_, face)| **face == tag)
    {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
        let side =
            |p: [f64; 3], q: [f64; 3]| (q[0] - p[0]) * (1.3 - p[1]) - (q[1] - p[1]) * (7.0 - p[0]);
        let sides = [side(a, b), side(b, c), side(c, a)];
        assert!(sides.iter().any(|value| *value > 0.0) && sides.iter().any(|value| *value < 0.0));
        area += ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0;
    }
    assert!((area - std::f64::consts::PI * (0.6985_f64.powi(2) - 0.4064_f64.powi(2))).abs() < 0.05);
}

#[test]
fn trimmed_cylinder_sector_refines_in_world_mm() {
    let coarse = native_model(SECTOR.as_bytes(), StepImportOptions::default());
    let fine = native_model(
        SECTOR.as_bytes(),
        StepImportOptions {
            target_deflection_mm: Some(0.005),
            ..Default::default()
        },
    );
    assert_eq!(fine.source_faces, coarse.source_faces);
    assert_eq!(fine.bounds, coarse.bounds);
    assert!(fine.triangle_faces.len() > coarse.triangle_faces.len());
    assert!((coarse.bounds[0][0] - 0.0).abs() < 1e-12);
    assert!((coarse.bounds[1][0] - 5.0 * 3.0_f64.sqrt()).abs() < 1e-12);
    assert!((coarse.bounds[1][1] - 5.0 * 3.0_f64.sqrt()).abs() < 1e-12);
}

#[test]
fn native_does_not_admit_open_or_off_support_geometry() {
    for changed in [
        SECTOR.replace("(#110,#125,#140,#155,#170)", "(#110,#125,#140,#155)"),
        SECTOR.replace(
            "CYLINDRICAL_SURFACE('',#20,10",
            "CYLINDRICAL_SURFACE('',#20,11",
        ),
    ] {
        assert_ne!(changed, SECTOR);
        let exchange = parser::parse(&changed).unwrap();
        assert!(
            build(
                &exchange,
                &units::representation_items(&exchange).unwrap(),
                &units::length_factors(&exchange).unwrap(),
                &units::precision::resolve(&exchange).unwrap(),
                StepImportOptions::default(),
                TOLERANCE_MM,
            )
            .is_err()
        );
        assert!(import_step(changed.into_bytes()).is_err());
    }
}

#[test]
fn native_instances_consume_product_placements_and_units_once() {
    let tetra = include_str!("../../../tests/fixtures/placed-tetra-occurrences.step");
    let model = native_model(tetra.as_bytes(), StepImportOptions::default());
    assert_eq!(model.bounds, [[-5., 7., 11.], [10., 22., 34.]]);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 2);
    assert_eq!(model.source_faces.len(), 4);
    assert_eq!(model.face_occurrences.len(), 8);

    let tetra = include_str!("../../../tests/fixtures/tetra-mm.step");
    let metres = tetra.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    assert_ne!(metres, tetra);
    let model = native_model(metres.as_bytes(), StepImportOptions::default());
    assert_eq!(model.bounds, [[0., 0., 0.], [2000., 3000., 4000.]]);
}

#[test]
fn wireframes_unsupported_surfaces_and_proof_candidates_keep_legacy_delivery() {
    let wires = include_bytes!("../../../tests/fixtures/tetra-and-finite-wires.step");
    let model = import_step(wires.to_vec()).unwrap();
    assert!(!model.solids.is_empty());
    assert_eq!(model.line_presentation().occurrences().len(), 2);

    let sphere = include_bytes!("../../../tests/fixtures/unsupported-non-ruled-sphere.step");
    let model = import_step(sphere.to_vec()).unwrap();
    assert!(!model.solids.is_empty());
    assert!(!model.accuracy.bounds_from_source_geometry);
    assert_eq!(model.accuracy.analysis_error_bound_mm, None);

    let cuboid = include_bytes!("../../../tests/fixtures/zero-repair-cuboid-mm.step");
    let model = import_step(cuboid.to_vec()).unwrap();
    assert!(!model.solids.is_empty());
    assert_eq!(model.accuracy.max_repair_displacement_mm, Some(0.0));
    assert_eq!(model.accuracy.analysis_error_bound_mm, None);
}
