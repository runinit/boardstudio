//! Complete source surfaces are not automatically eligible solid boundaries.
use super::*;
use std::collections::BTreeSet;

const BOXES: &[u8] = include_bytes!("../../tests/fixtures/touching-unit-boxes.step");
const PANASONIC: &[u8] = include_bytes!(
    "../../../../ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step"
);

fn assert_complete(model: &ImportedStep, bytes: &[u8], count: usize) {
    assert_eq!(model.original_bytes(), bytes);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1);
    assert_eq!(model.source_faces().len(), count);
    assert_eq!(model.face_occurrences().len(), count);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert!(!model.accuracy().mesh_closed_oriented);
    assert_eq!(model.line_presentation().occurrences().len(), 0);
    let mut areas = vec![0.; count];
    for (triangle, &tag) in model
        .preview_positions_f64()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(model.triangle_faces())
    {
        let [a, b, c] = triangle.map(Point3::from);
        let area = (b - a).cross(c - a).magnitude() / 2.;
        assert!(area.is_finite() && area > 0.);
        areas[tag] += area;
    }
    assert!(areas.iter().all(|area| *area > 0.));
    assert_eq!(
        model.preview_positions_f64().len(),
        model.triangle_faces().len() * 3
    );
    assert_eq!(
        model.mesh().positions.len(),
        model.preview_positions_f64().len() * 3
    );
    for (&source, &render) in model
        .preview_positions_f64()
        .iter()
        .flatten()
        .zip(&model.mesh().positions)
    {
        assert!(source.is_finite());
        assert_eq!(source as f32, render);
    }
}

#[test]
fn touching_boxes_preserve_twelve_analytic_faces_without_solid_eligibility() {
    let model = import_step(BOXES.to_vec()).expect("complete touching-box surfaces must preview");
    assert_complete(&model, BOXES, 12);
    assert_eq!(model.bounds(), [[0., 0., 0.], [2., 2., 1.]]);
    assert_eq!(
        model
            .source_faces()
            .iter()
            .map(|face| face.face_id)
            .collect::<BTreeSet<_>>(),
        (1000..1012).collect()
    );
    let mut areas = [0.; 12];
    for (triangle, &tag) in model
        .preview_positions_f64()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(model.triangle_faces())
    {
        let [a, b, c] = triangle.map(Point3::from);
        let source = &model.source_faces()[model.face_occurrences()[tag].source_face];
        areas[(source.face_id - 1000) as usize] += (b - a).cross(c - a).magnitude() / 2.;
    }
    assert!(
        areas.into_iter().all(|area| (area - 1.).abs() < 1e-12),
        "each independently authored unit-box face has area one"
    );
}

#[test]
fn original_panasonic_preserves_every_face_and_boundary_line() {
    assert_eq!(PANASONIC.len(), 123401);
    let model =
        import_step(PANASONIC.to_vec()).expect("complete original Panasonic surfaces must preview");
    assert_complete(&model, PANASONIC, 155);
    let original = parser::parse(std::str::from_utf8(PANASONIC).unwrap()).unwrap();
    let records = source_records(&original);
    let retained = source_records(&model._original_source);
    let expected: BTreeSet<_> = records
        .iter()
        .filter(|(_, records)| records.iter().any(|record| record.name == "ADVANCED_FACE"))
        .map(|(&id, _)| id)
        .collect();
    assert_eq!(
        model
            .source_faces()
            .iter()
            .map(|face| face.face_id)
            .collect::<BTreeSet<_>>(),
        expected
    );
    let mut lines = 0;
    for (&id, records) in &records {
        if records.iter().any(|record| record.name == "LINE") {
            assert_eq!(
                format!("{:?}", retained[&id]),
                format!("{records:?}"),
                "original LINE #{id} geometry/frame"
            );
            lines += 1;
        }
    }
    assert_eq!(
        lines, 353,
        "boundary LINE definitions are not display segments"
    );
    for (actual, expected) in model
        .bounds()
        .into_iter()
        .flatten()
        .zip([-3.2, -1.75, -0.5, 3.2, 2.75, 1.65])
    {
        assert!(
            (actual - expected).abs() < 1e-6,
            "source bounds {actual} vs {expected}"
        );
    }
}
