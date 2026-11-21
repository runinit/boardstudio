use super::*;

mod angular_tests;
mod cone_tests;
mod cylinder_band_tests;
mod diode_tests;

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
const NRF: &[u8] =
    include_bytes!("../../../../../ergogen/library/vendor/tsuki/3d_models/nrf52840.step");
const CIRQUE_LEDS: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/vik/3d_models/sadekbaroudi-vik/pcb/per56-cirque-leds/pers60-cirque-leds-pcb.step"
);
const FH12_12: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/vik/3d_models/Ariamelon-Crenshaw/PCB/Crenshaw.pretty/3d/FH12-12S-0.5SH.step"
);
const FH12_30: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/vik/3d_models/Ariamelon-Crenshaw/PCB/Crenshaw.pretty/3d/FH12-30S-0.5SH.step"
);
const SECTOR: &str = include_str!("../../../tests/fixtures/trimmed-cylinder-sector.step");
const MOLEX_CABLE_2: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/infused-kim/3d_models/Molex_Ezmate_Pico_Cable_2pin.step"
);
const MOLEX_CABLE_5: &[u8] = include_bytes!(
    "../../../../../ergogen/library/vendor/infused-kim/3d_models/Molex_Ezmate_Pico_Cable_5pin.step"
);
const RESET_SWITCH: &[u8] =
    include_bytes!("../../../../../ergogen/library/vendor/infused-kim/3d_models/Switch_Reset.step");

fn native_candidate(bytes: &[u8], options: StepImportOptions) -> Result<Asset, String> {
    let exchange = parser::parse(&units::decode_step_bytes(bytes)).unwrap();
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let precision = units::precision::resolve(&exchange).unwrap();
    build(
        &exchange,
        &items,
        &factors,
        &precision,
        options,
        options.target_deflection_mm.unwrap_or(TOLERANCE_MM),
    )
}

fn native_model(bytes: &[u8], options: StepImportOptions) -> ImportedStep {
    let candidate = native_candidate(bytes, options);
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
fn bundled_elliptical_trims_use_native_geometry() {
    let coarse = native_model(RESET_SWITCH, StepImportOptions::default());
    let fine = native_model(
        RESET_SWITCH,
        StepImportOptions {
            target_deflection_mm: Some(0.001),
            ..Default::default()
        },
    );
    for model in [&coarse, &fine] {
        assert_eq!(model.solid_count(), 6);
        assert_eq!(model.occurrence_count(), 6);
        assert_eq!(model.source_faces().len(), 326);
        assert_eq!(model.face_occurrences().len(), 326);
        assert!(model.validate_invariants().is_ok());
        let analysis = crate::qualification::prepare_vendor_analysis(model).unwrap();
        assert!(matches!(
            analysis.source_qualification(),
            crate::qualification::SourceQualification::Unknown { .. }
        ));
        for (actual, expected) in model
            .bounds()
            .into_iter()
            .flatten()
            .zip([-2.35, 0.0, -1.45, 2.35, 1.35, 2.1])
        {
            assert!((actual - expected).abs() < 1e-9);
        }
    }
    assert_eq!(coarse.bounds(), fine.bounds());
    assert_eq!(coarse.source_faces(), fine.source_faces());
    assert_eq!(coarse.face_occurrences(), fine.face_occurrences());
    assert!(fine.triangle_faces().len() > coarse.triangle_faces().len());

    let rim_edges = |model: &ImportedStep, face_id, support_id, kind| {
        let source = model
            .source_faces()
            .iter()
            .position(|face| {
                face.face_id == face_id
                    && face.support_id == support_id
                    && face.support_kind == kind
            })
            .unwrap();
        let (tag, occurrence) = model
            .face_occurrences()
            .iter()
            .enumerate()
            .find(|(_, face)| face.source_face == source)
            .unwrap();
        assert_eq!(model.source_faces()[source].solid_id, 25);
        assert_eq!(occurrence.occurrence, 5);
        assert_eq!(occurrence.representation_id, 12);
        let on_rim = |[x, y, z]: [f64; 3]| {
            ((y - 0.3).hypot(z - 1.9) - 0.2).abs() < 1e-9 && (x + y + 0.45).abs() < 1e-9
        };
        let mut edges = HashSet::new();
        for (points, &face) in model
            .preview_positions_f64()
            .chunks_exact(3)
            .zip(model.triangle_faces())
        {
            if face != tag {
                continue;
            }
            if face_id == 6077 {
                for [_, y, z] in points {
                    assert!(((y - 0.3).hypot(z - 1.9) - 0.2).abs() < 1e-9);
                }
            }
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                if face_id == 6077 {
                    let y = (points[a][1] + points[b][1]) * 0.5;
                    let z = (points[a][2] + points[b][2]) * 0.5;
                    let residual = (0.2 - (y - 0.3).hypot(z - 1.9)).abs();
                    assert!(residual <= model.accuracy().preview_deflection_mm);
                }
                if on_rim(points[a]) && on_rim(points[b]) {
                    let a = points[a].map(f64::to_bits);
                    let b = points[b].map(f64::to_bits);
                    if !edges.remove(&(b, a)) {
                        assert!(edges.insert((a, b)));
                    }
                }
            }
        }
        edges
    };
    for (model, segments) in [(&coarse, 4), (&fine, 10)] {
        let cylinder = rim_edges(model, 6077, 60, "CYLINDRICAL_SURFACE");
        let plane = rim_edges(model, 6180, 5854, "PLANE");
        assert_eq!(cylinder.len(), segments);
        assert_eq!(cylinder, plane.into_iter().map(|(a, b)| (b, a)).collect());
    }
}

#[test]
fn eccentric_ellipse_refines_chord_turn_near_its_tip_and_refuses_excess_work() {
    let endpoint = |t: f64| Point3::new(10.0 * t.cos(), t.sin(), 0.0);
    let edge = Edge {
        vertices: [1, 2],
        points: [endpoint(-0.24), endpoint(0.24)],
        curve: geometry::Curve::Ellipse {
            frame: Matrix4::from_scale(1.0),
            major: 10.0,
            minor: 1.0,
            forward: true,
        },
        carrier: None,
        finite_ellipse: None,
    };
    let points = edge.sample(0.1, 1e-8).unwrap();
    assert_eq!([points[0], *points.last().unwrap()], edge.points);
    assert!(points.len() > 3);
    for p in points.windows(3) {
        let left = p[1] - p[0];
        let right = p[2] - p[1];
        assert!(left.cross(right).magnitude().atan2(left.dot(right)) <= 0.5);
    }
    assert_eq!(
        edge.sample(1e-12, 1e-8).unwrap_err(),
        "native conic subdivision limit"
    );
}

#[test]
fn finite_ellipse_bounds_include_its_rotated_interior_extremum() {
    let exchange = parser::parse(str::from_utf8(RESET_SWITCH).unwrap()).unwrap();
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let scene = scene::Scene::new(&exchange, &items, &factors, &[25]);
    let edge = Edge::read(3655, &source_records(&exchange), &scene, &HashMap::new()).unwrap();
    let world = Matrix4::from_angle_y(cgmath::Rad(std::f64::consts::FRAC_PI_4));
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    edge.include_bounds(world, 1e-8, &mut bounds).unwrap();
    // World X is (source X + source Z) / sqrt(2). The maximum is
    // inside the quarter ellipse, and the minimum is at either endpoint.
    let center = (-0.75 + 1.9) * std::f64::consts::FRAC_1_SQRT_2;
    assert!((bounds[1][0] - (center + 0.2)).abs() < 1e-12);
    assert!((bounds[0][0] - (center + 0.2 * std::f64::consts::FRAC_1_SQRT_2)).abs() < 1e-12);
}

#[test]
fn planar_ellipse_refuses_a_crossing_between_mesh_samples() {
    let exchange = parser::parse(str::from_utf8(RESET_SWITCH).unwrap()).unwrap();
    let items = units::representation_items(&exchange).unwrap();
    let factors = units::length_factors(&exchange).unwrap();
    let scene = scene::Scene::new(&exchange, &items, &factors, &[25]);
    let mut catalogue = Vec::new();
    let mut solid = Solid::read(
        25,
        &source_records(&exchange),
        &scene,
        &HashMap::new(),
        &mut catalogue,
    )
    .unwrap();
    let face = solid
        .faces
        .iter()
        .find(|face| catalogue[face.source].face_id == 6180)
        .unwrap();
    let (chart, _, start, sweep) = solid.edges[&3655].ellipse(1e-8).unwrap();
    let angle = start + sweep / 8.0;
    // The line crosses the ellipse halfway between two coarse samples.
    // Its entire span is outside their chord at radius cos(pi / 16).
    solid.edges.get_mut(&3822).unwrap().points = [0.99, 1.01]
        .map(|r| chart.transform_point(Point3::new(r * angle.cos(), r * angle.sin(), 0.0)));
    assert_eq!(
        face.check_planar_ellipse_trim(&solid.edges, 1e-8)
            .err()
            .as_deref(),
        Some("native planar return path enters the elliptical arc sector")
    );
}

#[test]
fn oblique_cylinder_refuses_the_complementary_ellipse_interval() {
    let original = str::from_utf8(RESET_SWITCH).unwrap();
    let changed = original.replace(
        "#3655=EDGE_CURVE('',#2872,#2875,#13,.T.);",
        "#3655=EDGE_CURVE('',#2872,#2875,#13,.F.);",
    );
    assert_ne!(changed, original);
    assert_eq!(
        native_candidate(changed.as_bytes(), StepImportOptions::default())
            .err()
            .as_deref(),
        Some(
            "native face #6077: native oblique cylinder trims do not have a common single-chart sweep"
        )
    );
}

#[test]
fn oblique_cylinder_refuses_off_support_ellipse_with_unchanged_endpoints() {
    let original = str::from_utf8(RESET_SWITCH).unwrap();
    let pivot = Point3::new(-0.75, 0.3, 2.1);
    let chord = Vector3::new(1.0, -1.0, -1.0).normalize();
    let rotation = Matrix4::from_axis_angle(chord, cgmath::Rad(0.25));
    let center = pivot + rotation.transform_vector(Vector3::new(0.0, 0.0, -0.2));
    let axis = rotation.transform_vector(Vector3::new(1.0, 1.0, 0.0));
    let reference = rotation.transform_vector(Vector3::new(1.0, -1.0, 0.0));
    let changed = original.replace(
        "#6494=AXIS2_PLACEMENT_3D('',#9606,#7819,#7820);",
        &format!(
            "#6494=AXIS2_PLACEMENT_3D('',#10001,#10002,#10003);\n\
             #10001=CARTESIAN_POINT('',({:.17E},{:.17E},{:.17E}));\n\
             #10002=DIRECTION('',({:.17E},{:.17E},{:.17E}));\n\
             #10003=DIRECTION('',({:.17E},{:.17E},{:.17E}));",
            center.x,
            center.y,
            center.z,
            axis.x,
            axis.y,
            axis.z,
            reference.x,
            reference.y,
            reference.z,
        ),
    );
    assert_ne!(changed, original);
    assert_eq!(
        native_candidate(changed.as_bytes(), StepImportOptions::default())
            .err()
            .as_deref(),
        Some("native face #6077: source elliptical boundary is off its CYLINDRICAL_SURFACE")
    );
}

#[test]
fn bundled_molex_cables_remain_on_the_vendor_path() {
    for bytes in [MOLEX_CABLE_2, MOLEX_CABLE_5] {
        assert!(native_candidate(bytes, StepImportOptions::default()).is_err());
        let model = import_step(bytes.to_vec()).unwrap();
        assert!(!model.solids.is_empty());
        assert_eq!(model.original_bytes(), bytes);
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }
}

#[test]
fn native_oblique_trims_use_representation_length_units() {
    let original = str::from_utf8(RESET_SWITCH).unwrap();
    let centimetres = original.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT(.CENTI.,.METRE.)");
    assert_ne!(centimetres, original);
    let original = native_model(
        RESET_SWITCH,
        StepImportOptions {
            target_deflection_mm: Some(0.001),
            ..Default::default()
        },
    );
    let scaled = native_model(
        centimetres.as_bytes(),
        StepImportOptions {
            target_deflection_mm: Some(0.01),
            ..Default::default()
        },
    );
    assert_eq!(scaled.triangle_faces(), original.triangle_faces());
    for (actual, source) in scaled
        .preview_positions_f64()
        .iter()
        .flatten()
        .zip(original.preview_positions_f64().iter().flatten())
    {
        assert!((actual - 10.0 * source).abs() < 1e-10);
    }
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
fn split_cylinder_trims_keep_source_faces_and_refine() {
    for (bytes, count, split_face, support) in [
        (NRF, 110, 2101, 94),
        (FH12_12, 530, 10150, 20),
        (FH12_30, 1106, 21027, 24),
    ] {
        let coarse = native_model(bytes, StepImportOptions::default());
        let fine = native_model(
            bytes,
            StepImportOptions {
                target_deflection_mm: Some(0.001),
                ..Default::default()
            },
        );
        assert_eq!(coarse.source_faces.len(), count);
        assert_eq!(fine.source_faces, coarse.source_faces);
        assert_eq!(fine.face_occurrences, coarse.face_occurrences);
        assert_eq!(fine.bounds, coarse.bounds);
        assert_eq!(fine.occurrences, coarse.occurrences);
        let tag = fine
            .face_occurrences
            .iter()
            .position(|occurrence| {
                let face = &fine.source_faces[occurrence.source_face];
                face.face_id == split_face
                    && face.support_id == support
                    && face.support_kind == "CYLINDRICAL_SURFACE"
            })
            .unwrap();
        let triangles_at = |model: &ImportedStep| {
            model
                .triangle_faces
                .iter()
                .filter(|&&face| face == tag)
                .count()
        };
        assert!(triangles_at(&fine) > triangles_at(&coarse));
    }
}

#[test]
fn split_cylinder_trim_refuses_an_off_support_arc() {
    let source = units::decode_step_bytes(NRF);
    let authored = "#94=CYLINDRICAL_SURFACE('',#2228,1.65);";
    assert_eq!(source.matches(authored).count(), 1);
    let changed = source.replace(authored, "#94=CYLINDRICAL_SURFACE('',#2228,1.75);");
    assert_eq!(
        native_candidate(changed.as_bytes(), StepImportOptions::default())
            .err()
            .as_deref(),
        Some("native face #2101: source circular boundary is off its CYLINDRICAL_SURFACE")
    );
}

#[test]
fn cirque_led_pcb_imports_authored_surface_curves_and_seams() {
    let coarse = native_model(CIRQUE_LEDS, StepImportOptions::default());
    let fine = native_model(
        CIRQUE_LEDS,
        StepImportOptions {
            target_deflection_mm: Some(0.001),
            ..Default::default()
        },
    );
    assert_eq!(coarse.solid_count(), 1);
    assert_eq!(coarse.occurrence_count(), 1);
    assert_eq!(coarse.source_faces.len(), 36);
    assert_eq!(coarse.face_occurrences.len(), 36);
    assert_eq!(fine.source_faces, coarse.source_faces);
    assert_eq!(fine.face_occurrences, coarse.face_occurrences);
    assert_eq!(fine.bounds, coarse.bounds);
    // FreeCAD Part.Shape.optimalBoundingBox on the original AP214 BRep.
    for (actual, expected) in coarse
        .bounds
        .into_iter()
        .flatten()
        .zip([142.24, -152.4, 0.0, 207.01, -88.9, 1.6])
    {
        assert!((actual - expected).abs() < 1e-9);
    }
    assert!(fine.triangle_faces.len() > coarse.triangle_faces.len());

    let source = units::decode_step_bytes(CIRQUE_LEDS);
    let origin = "#16 = CARTESIAN_POINT('',(0.,0.,0.));";
    assert_eq!(source.matches(origin).count(), 1);
    let placed = source.replace(origin, "#16 = CARTESIAN_POINT('',(10.,-20.,30.));");
    let placed = native_model(placed.as_bytes(), StepImportOptions::default());
    assert_eq!(placed.source_faces, coarse.source_faces);
    assert_eq!(placed.face_occurrences, coarse.face_occurrences);
    for (original, moved) in coarse.bounds.into_iter().zip(placed.bounds) {
        for ((actual, original), shift) in moved.into_iter().zip(original).zip([10.0, -20.0, 30.0])
        {
            assert!((actual - original - shift).abs() < 1e-9);
        }
    }
}

#[test]
fn native_carrier_refuses_contradictory_pcurve_geometry_and_seam_branches() {
    let source = units::decode_step_bytes(CIRQUE_LEDS);
    for (authored, changed, error) in [
        (
            "#50 = CARTESIAN_POINT('',(-1.570796326795,0.));",
            "#50 = CARTESIAN_POINT('',(-1.570796326795,0.1));",
            None, // Moving the origin along the unbounded pcurve does not change the trim.
        ),
        (
            "#50 = CARTESIAN_POINT('',(-1.570796326795,0.));",
            "#50 = CARTESIAN_POINT('',(-1.55,0.));",
            Some("native edge #32: native pcurve disagrees with its 3D carrier"),
        ),
        (
            "#52 = DIRECTION('',(-0.,-1.));",
            "#52 = DIRECTION('',(-0.,1.));",
            Some("native edge #32: native pcurve and 3D carrier have incompatible geometry"),
        ),
        (
            "#90 = CIRCLE('',#91,2.54);",
            "#90 = CIRCLE('',#91,2.55);",
            Some("native edge #67: native pcurve disagrees with its 3D carrier"),
        ),
        (
            "#2120 = CARTESIAN_POINT('',(6.28318530718,-0.));",
            "#2120 = CARTESIAN_POINT('',(0.,-0.));",
            Some("native edge #2111: native seam pcurves are not one period apart"),
        ),
        (
            "#54 = PCURVE('',#55,#60);",
            "#54 = PCURVE('',#84,#60);",
            Some("native edge #32: pcurve supports differ from face uses"),
        ),
    ] {
        assert_eq!(source.matches(authored).count(), 1);
        let modified = source.replace(authored, changed);
        assert_eq!(
            native_candidate(modified.as_bytes(), StepImportOptions::default())
                .err()
                .as_deref(),
            error
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
fn native_product_placements_apply_parent_and_child_units_to_their_own_frames() {
    let tetra = include_str!("../../../tests/fixtures/placed-tetra-occurrences.step");
    let mixed = tetra.replace(
        "#220=SHAPE_REPRESENTATION('',(#200,#201,#202),#103);",
        "#208=CARTESIAN_POINT('',(0.5,1.,1.5));\n\
         #209=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));\n\
         #219=(GEOMETRIC_REPRESENTATION_CONTEXT(3)REPRESENTATION_CONTEXT('3D','MODEL')GLOBAL_UNIT_ASSIGNED_CONTEXT((#209)));\n\
         #220=SHAPE_REPRESENTATION('',(#201,#202),#219);",
    )
    .replace("#200=AXIS2_PLACEMENT_3D('',#1,", "#200=AXIS2_PLACEMENT_3D('',#208,")
    .replace("#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),", "#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101,#200),");
    let model = native_model(mixed.as_bytes(), StepImportOptions::default());
    assert_eq!(
        model.bounds,
        [[-5000.5, 6999., 10998.5], [10001., 20001.5, 30002.5]]
    );
    assert_eq!(model.occurrences.len(), 2);
    for (index, ((solid, placement, owner), offset)) in model
        .occurrences
        .iter()
        .zip([[10001., 19999.5, 29998.5], [-5000.5, 6999., 10998.5]])
        .enumerate()
    {
        assert_eq!((*solid, *owner), (101, 104));
        assert_eq!([placement.w.x, placement.w.y, placement.w.z], offset);
        assert_eq!(
            model
                .face_occurrences
                .iter()
                .filter(|f| f.occurrence == index && f.representation_id == *owner)
                .count(),
            4
        );
    }
}

#[test]
fn mirrored_map_reverses_winding_and_preserves_each_face_occurrence() {
    let tetra = include_str!("../../../tests/fixtures/mapped-occurrence.step");
    let mirrored = tetra.replace(
        "#203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#201,1.,$);",
        "#202=DIRECTION('',(0.,-1.,0.));\n\
         #203=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,#202,#201,1.,$);",
    );
    let model = native_model(mirrored.as_bytes(), StepImportOptions::default());
    assert_eq!(model.bounds, [[0., -3., 0.], [102., 3., 4.]]);
    assert_eq!(model.occurrences.len(), 2);
    assert!(model.occurrences[1].1.determinant() < 0.0);
    for (original, reflected) in model.face_occurrences[..4]
        .iter()
        .zip(&model.face_occurrences[4..])
    {
        assert_eq!(original.source_face, reflected.source_face);
        assert_eq!((original.occurrence, original.representation_id), (0, 104));
        assert_eq!(
            (reflected.occurrence, reflected.representation_id),
            (1, 105)
        );
    }
    assert_eq!(model.triangle_faces, (0..8).collect::<Vec<_>>());
    for (original, reflected) in model.preview_positions_f64[..12]
        .chunks_exact(3)
        .zip(model.preview_positions_f64[12..].chunks_exact(3))
    {
        assert_eq!(
            reflected,
            [0, 2, 1].map(|i| {
                let [x, y, z] = original[i];
                [100. + x, -y, z]
            })
        );
    }
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
