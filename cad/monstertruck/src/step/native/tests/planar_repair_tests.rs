use super::*;
use repair::finite_line::{ExactPlacement, NATIVE_SCANS, capture_native};

const PIN_SOCKET: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/PinSocket_2.54mm_5mm_1x-4.step"
);

const PIN_SOCKET_1X5: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/PinSocket_2.54mm_5mm_1x-5.step"
);
const PIN_SOCKET_2X12: &[u8] = include_bytes!(
    "../../../../../../ergogen/library/vendor/infused-kim/3d_models/PinSocket_2.54mm_5mm_2x-12.step"
);

fn replace_record(bytes: &[u8], old: &str, new: &str) -> String {
    let source = str::from_utf8(bytes).unwrap();
    assert_eq!(source.matches(old).count(), 1);
    source.replacen(old, new, 1)
}

fn budget(mm: f64) -> StepImportOptions {
    StepImportOptions {
        preview_repair_budget_mm: Some(mm),
        ..StepImportOptions::default()
    }
}

fn assert_bounds(model: &ImportedStep, x: [f64; 2], z: f64, translation: f64) {
    // Source vertex and projected LINE extrema in their owning representation.
    for (actual, source) in model.bounds().into_iter().flatten().zip([
        x[0] + translation,
        -3.0,
        -z,
        x[1] + translation,
        5.0,
        z,
    ]) {
        assert!((actual - source).abs() < 1e-10, "{actual} vs {source}");
    }
}

#[test]
fn pin_socket_keeps_all_faces_and_source_bounds_after_shared_repair() {
    let model = native_model(PIN_SOCKET, StepImportOptions::default());
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1);
    assert_eq!(model.source_faces().len(), 153);
    assert_eq!(model.face_occurrences().len(), 153);
    assert!(model.validate_invariants().is_ok());
    assert!(!model.accuracy().relaxed_beyond_declared_precision);
    let analysis = crate::qualification::prepare_vendor_analysis(&model).unwrap();
    assert!(matches!(
        analysis.source_qualification(),
        crate::qualification::SourceQualification::Unknown { .. }
    ));
    assert_bounds(&model, [-25.6500003814697, -14.9900010681152], 1.25, 24.13);

    // #1502 originally lies at x=-25.6500003814697. Both uses must emit the
    // same repaired segment at plane #2520, with opposite orientations.
    let x = -25.65 + 24.13;
    let [a, b] = [[x, 0.3, -1.0], [x, 0.0, -1.0]];
    let mut uses = Vec::new();
    for (points, &tag) in model
        .preview_positions_f64()
        .chunks_exact(3)
        .zip(model.triangle_faces())
    {
        for (start, end) in points.iter().zip(points.iter().cycle().skip(1)) {
            if (start == &a && end == &b) || (start == &b && end == &a) {
                let usage = &model.face_occurrences()[tag];
                let face = &model.source_faces()[usage.source_face];
                assert!((3.8e-7..=4.0e-7).contains(&usage.repair_displacement_mm));
                uses.push((face.face_id, start == &a));
            }
        }
    }
    uses.sort_unstable();
    assert_eq!(uses, [(2673, true), (2683, false)]);
}

#[test]
fn longer_pin_sockets_keep_complete_native_occurrences() {
    for (bytes, solids, faces, x, z, translation) in [
        (
            PIN_SOCKET_1X5,
            1,
            189,
            [-25.6500003814697, -12.4500010681152],
            1.25,
            24.13,
        ),
        // #11 links both solid bodies to root #25274. The transformed child
        // #25275 contains just a frame, so neither solid is translated.
        (
            PIN_SOCKET_2X12,
            2,
            882,
            [-1.5200003814697, 29.4599989318848],
            8.87,
            0.0,
        ),
    ] {
        let model = native_model(bytes, budget(4e-7));
        assert_eq!(model.solid_count(), solids);
        assert_eq!(model.occurrence_count(), solids);
        assert_eq!(model.source_faces().len(), faces);
        assert_eq!(model.face_occurrences().len(), faces);
        assert!(
            model
                .face_occurrences()
                .iter()
                .any(|f| f.repair_displacement_mm > 3.8e-7)
        );
        assert_bounds(&model, x, z, translation);
    }
}

#[test]
fn pin_socket_repair_honors_world_mm_budget_and_retains_refusal() {
    let expected = native_model(PIN_SOCKET, budget(4e-7));
    let error = native_candidate(PIN_SOCKET, budget(3.8e-7)).err().unwrap();
    assert!(
        error.contains("finite LINE displacement exceeds preview repair budget"),
        "{error}"
    );
    let error = import_step_with_options(PIN_SOCKET.to_vec(), budget(3.8e-7))
        .err()
        .expect("the legacy route must also honor the explicit budget");
    assert!(error.contains("finite LINE repair"), "{error}");

    let centimetres = replace_record(
        PIN_SOCKET,
        "SI_UNIT(.MILLI.,.METRE.)",
        "SI_UNIT(.CENTI.,.METRE.)",
    );
    let error = native_candidate(centimetres.as_bytes(), budget(4e-7))
        .err()
        .unwrap();
    assert!(
        error.contains("finite LINE displacement exceeds preview repair budget"),
        "{error}"
    );
    let scaled = native_model(centimetres.as_bytes(), budget(4e-6));
    for (scaled, expected) in scaled
        .bounds()
        .into_iter()
        .flatten()
        .zip(expected.bounds().into_iter().flatten())
    {
        assert!((scaled - expected * 10.0).abs() < 1e-10);
    }
    for (scaled, expected) in scaled
        .face_occurrences()
        .iter()
        .zip(expected.face_occurrences())
    {
        // The independently rounded world endpoints can differ by a few
        // binary64 ulps when the scaled coordinates are emitted.
        assert!(
            (scaled.repair_displacement_mm - expected.repair_displacement_mm * 10.0).abs() < 1e-13
        );
    }
}

#[test]
fn pin_socket_repair_refuses_incompatible_planes_and_detached_line() {
    // Vertex #1291 lies on four planes. The other three already determine its
    // position, so translating #2552 along its normal has no common solution.
    let conflict = replace_record(
        PIN_SOCKET,
        "#4021=CARTESIAN_POINT('Origin',(-24.63,0.3,-0.5));",
        "#4021=CARTESIAN_POINT('Origin',(-24.63,0.3,-0.50001));",
    );
    let error = native_candidate(conflict.as_bytes(), StepImportOptions::default())
        .err()
        .unwrap();
    assert!(
        error.contains("incompatible incident PLANE supports"),
        "{error}"
    );
    if let Ok(model) = import_step(conflict.as_bytes().to_vec()) {
        assert!(!model.solids.is_empty());
        assert_eq!(model.original_bytes(), conflict.as_bytes());
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }

    // Keep the same endpoints and plane uses but move the authored LINE
    // origin farther away. Plane agreement alone cannot admit the segment.
    let detached = replace_record(
        PIN_SOCKET,
        "#3872=CARTESIAN_POINT('',(-25.6500003814697,0.,-1.));",
        "#3872=CARTESIAN_POINT('',(-25.650001,0.,-1.));",
    );
    let error = native_candidate(detached.as_bytes(), budget(4e-7))
        .err()
        .unwrap();
    assert!(error.contains("native edge #1502 shared trim repair: finite LINE displacement exceeds preview repair budget"), "{error}");
}

#[test]
fn shared_line_budget_and_bounds_include_world_translation_rounding() {
    let source = b"ISO-10303-21;HEADER;ENDSEC;DATA;
        #1=CARTESIAN_POINT('',(0.0000001,0.,0.));
        #2=CARTESIAN_POINT('',(0.0000001,1.,0.));
        #3=DIRECTION('',(0.,1.,0.));
        #5=CARTESIAN_POINT('',(0.,0.,0.));
        #6=CARTESIAN_POINT('',(0.,1.,0.));
        ENDSEC;END-ISO-10303-21;";
    let sources = capture_native(
        source,
        &HashMap::from([(4, ([1, 3, 1, 2], true)), (7, ([1, 3, 5, 6], true))]),
        &HashMap::new(),
    )
    .unwrap();
    let t = 2_f64.powi(30);
    let world = Matrix4::from_translation(Vector3::new(t, 0., 0.));
    let exact = ExactPlacement::new(1., world).unwrap();
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let ends = |x| [Point3::new(x, 0., 0.), Point3::new(x, 1., 0.)];
    let line = &sources.lines[&4];
    let result = line.shared_repair_bound(ends(1.25e-7), world, &exact, 3e-8, &mut bounds);
    assert_eq!(
        result,
        Err("finite LINE displacement exceeds preview repair budget")
    );
    assert_eq!(world.transform_point(ends(1.25e-7)[0]).x, t.next_up());
    let movement = line
        .shared_repair_bound(ends(1.25e-7), world, &exact, 1.4e-7, &mut bounds)
        .unwrap();
    assert!((1.384e-7..1.385e-7).contains(&movement));
    assert_eq!(bounds, [[t, 0., 0.], [t.next_up(), 1., 0.]]);

    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let movement = sources.lines[&7]
        .shared_repair_bound(ends(0.), world, &exact, 1.1e-7, &mut bounds)
        .unwrap();
    assert!((1e-7..1.00001e-7).contains(&movement));
    // Only the projected LINE reaches beyond the emitted and original vertex chords.
    assert_eq!(bounds, [[t, 0., 0.], [t.next_up(), 1., 0.]]);
}

#[test]
fn shared_trim_checks_original_plane_spelling_and_emitted_incidence() {
    let source = b"ISO-10303-21;HEADER;ENDSEC;DATA;
        #1=CARTESIAN_POINT('',(999999999.99999995,0.,0.));
        #2=CARTESIAN_POINT('',(1000000000.00000005,0.,0.));
        #3=DIRECTION('',(1.,0.,0.));
        #4=CARTESIAN_POINT('',(0.000000125,0.,0.));
        #5=CARTESIAN_POINT('',(0.,0.,0.));
        #6=DIRECTION('',(1.,1.00000000000000005,0.));
        ENDSEC;END-ISO-10303-21;";
    let sources = capture_native(
        source,
        &HashMap::new(),
        &HashMap::from([(11, [1, 3]), (12, [2, 3]), (13, [4, 3]), (14, [5, 6])]),
    )
    .unwrap();
    assert_eq!(
        "999999999.99999995".parse::<f64>(),
        "1000000000.00000005".parse::<f64>()
    );
    let exact = ExactPlacement::new(1., Matrix4::identity()).unwrap();
    let rounded = Point3::new(1e9, 0., 0.);
    for id in [11, 12] {
        assert_eq!(
            sources.planes[&id].contains(rounded, rounded, &exact, 1e-8, 1e-8),
            Some(false)
        );
        assert_eq!(
            sources.planes[&id].contains(rounded, rounded, &exact, 6e-8, 6e-8),
            Some(true)
        );
    }
    let on_rounded_normal = Point3::new(1e9, -1e9, 0.);
    assert_eq!(
        sources.planes[&14].contains(on_rounded_normal, on_rounded_normal, &exact, 1e-8, 1e-8),
        Some(false)
    );
    let world = Matrix4::from_translation(Vector3::new(2_f64.powi(30), 0., 0.));
    let exact = ExactPlacement::new(1., world).unwrap();
    let local = Point3::new(1.25e-7, 0., 0.);
    assert_eq!(
        sources.planes[&13].contains(local, world.transform_point(local), &exact, 1e-8, 1e-8),
        Some(false)
    );
}

#[test]
fn shared_planar_vertex_ids_cannot_move_in_distinct_solids() {
    // Both BReps use shell #2813. Native repair must stop before a definition
    // can move a vertex shared with another BRep in the same representation.
    let duplicate = replace_record(
        PIN_SOCKET,
        "#16=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#17),#4430);",
        "#16=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#17,#9900),#4430);
         #9900=MANIFOLD_SOLID_BREP('shared shell',#2813);",
    );
    let error = native_candidate(duplicate.as_bytes(), StepImportOptions::default())
        .err()
        .unwrap();
    assert!(error.contains("is shared between solids"), "{error}");
    if let Ok(model) = import_step(duplicate.as_bytes().to_vec()) {
        assert!(!model.solids.is_empty());
        assert_eq!(model.original_bytes(), duplicate.as_bytes());
    }
}

#[test]
fn shared_decimal_capture_scans_once_per_asset() {
    let mapped = replace_record(
        PIN_SOCKET,
        "#4441=SHAPE_REPRESENTATION('',(#2826,#2981),#4429);",
        "#4441=SHAPE_REPRESENTATION('',(#2826,#2981,#9901),#4429);
         #9900=REPRESENTATION_MAP(#2827,#4442);
         #9901=MAPPED_ITEM('',#9900,#9902);
         #9902=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',$,$,#3663,1.,$);",
    );
    for source in [mapped.as_bytes(), PIN_SOCKET_2X12] {
        let before = NATIVE_SCANS.with(|scans| scans.get());
        let asset = native_candidate(source, StepImportOptions::default()).unwrap();
        assert_eq!(asset.occurrences.len(), 2);
        for occurrence in 0..2 {
            assert!(
                asset
                    .face_occurrences
                    .iter()
                    .any(|face| face.occurrence == occurrence && face.repair_displacement_mm > 0.)
            );
        }
        assert_eq!(NATIVE_SCANS.with(|scans| scans.get()), before + 1);
    }
}

#[test]
fn shared_planar_repair_respects_default_cutoff() {
    let with_line_origin = |x| {
        replace_record(
            PIN_SOCKET,
            "#3872=CARTESIAN_POINT('',(-25.6500003814697,0.,-1.));",
            &format!("#3872=CARTESIAN_POINT('',({x},0.,-1.));"),
        )
    };
    let below = with_line_origin("-25.669999999");
    native_model(below.as_bytes(), StepImportOptions::default());
    let above = with_line_origin("-25.670000001");
    let error = native_candidate(above.as_bytes(), StepImportOptions::default())
        .err()
        .unwrap();
    assert!(
        error.contains("finite LINE displacement exceeds preview repair budget"),
        "{error}"
    );
    if let Ok(model) = import_step(above.as_bytes().to_vec()) {
        assert!(!model.solids.is_empty());
        assert_eq!(model.original_bytes(), above.as_bytes());
    }
}
