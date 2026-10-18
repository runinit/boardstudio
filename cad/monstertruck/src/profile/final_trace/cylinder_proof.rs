//! Private exact radial correspondence for generated cylindrical extrusions.
use super::*;

fn qualify(_model: &ExactModel, _mesh: &GeneratedAnalysisMesh, _trace: &AnalysisTrace) -> Result<f64, &'static str> {
    Err("missing symmetric cylindrical/cap proof")
}

#[test]
fn drilled_plate_has_a_complete_symmetric_source_bound() {
    let mut input = prepared_plate();
    input.body.thickness = 2.;
    let model = crate::build_case(&input).unwrap();
    let (mesh, trace) = checked_trace(&model, None);
    let bound = qualify(&model, &mesh, &trace).expect("whole drilled plate needs a symmetric proof");
    assert!(bound > 0. && bound < 0.2, "{bound}");
}

#[test]
fn inspect_generated_cylinder_inputs() {
    let model = crate::build_case(&prepared_plate()).unwrap();
    let (_, trace) = checked_trace(&model, None);
    for face in &trace.faces {
        eprintln!("FACE {:?} surface {:?}", face.source, face.surface);
        for wire in &face.boundaries {
            eprintln!("WIRE {}", wire.len());
            for edge in wire.edge_iter() {eprintln!("EDGE {:?}", edge.curve());}
        }
        if matches!(face.surface, Surface::RevolutionSurface(_)) {
            for tri in trace.triangles.iter().filter(|t|t.occurrence_index==face.source.face_index).take(6) {
                eprintln!("TRI {:?} UV {:?}",tri.positions_mm_f64,tri.uv);
            }
        }
    }
}
