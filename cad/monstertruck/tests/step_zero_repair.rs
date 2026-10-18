use boardstudio_cad::{StepImportOptions, import_step, import_step_with_options};

const BOX: &[u8] = include_bytes!("fixtures/zero-repair-cuboid-mm.step");
const OFFSET_BOX: &[u8] = include_bytes!("fixtures/zero-repair-cuboid-offset-mm.step");

#[test]
fn original_integer_cuboids_have_complete_zero_repair_without_analysis_certification() {
    for (bytes, bounds) in [
        (BOX, [[0., 0., 0.], [2., 4., 8.]]),
        (OFFSET_BOX, [[-3., 2., -4.], [5., 8., 2.]]),
    ] {
        let model = import_step(bytes.to_vec()).expect("literal native LINE/plane cuboid imports");
        assert_eq!(model.bounds(), bounds);
        assert_eq!(model.original_bytes(), bytes);
        assert_eq!(model.source_faces().len(), 6);
        assert_eq!(model.face_occurrences().len(), 6);
        assert_eq!(model.accuracy().max_repair_displacement_mm, Some(0.0));
        assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    }
}
