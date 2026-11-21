use super::*;

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
        assert!(
            (scaled.repair_displacement_mm - expected.repair_displacement_mm * 10.0).abs() < 1e-15
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
