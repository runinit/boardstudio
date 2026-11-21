//! Controlled corruption tests the validator, not rejection at the import boundary.
use super::*;

fn valid_tetrahedron() -> ImportedStep {
    let model = import_step(include_bytes!("../../tests/fixtures/tetra-mm.step").to_vec())
        .expect("tetrahedron imports");
    assert_eq!(model.validate_invariants(), Ok(()));
    model
}

fn assert_late_nonfinite_position_rejected(value: f32) {
    let mut model = valid_tetrahedron();
    let index = model.mesh.positions.len() - 1;
    assert!(index > 10);
    assert!(model.mesh.positions[..index].iter().all(|v| v.is_finite()));
    model.mesh.positions[index] = value;
    assert_eq!(
        model.validate_invariants(),
        Err(vec![format!(
            "mesh position {index} is not finite: {value}"
        )])
    );
}

#[test]
fn rejects_nan_position_after_finite_prefix_beyond_ten() {
    assert_late_nonfinite_position_rejected(f32::NAN);
}

#[test]
fn rejects_positive_infinite_position_after_finite_prefix_beyond_ten() {
    assert_late_nonfinite_position_rejected(f32::INFINITY);
}

#[test]
fn rejects_negative_infinite_position_after_finite_prefix_beyond_ten() {
    assert_late_nonfinite_position_rejected(f32::NEG_INFINITY);
}

#[test]
fn rejects_empty_mesh() {
    let mut model = valid_tetrahedron();
    model.mesh.positions.clear();
    model.triangle_faces.clear();
    assert_eq!(
        model.validate_invariants(),
        Err(vec!["mesh has no positions".to_string()])
    );
}

#[test]
fn rejects_missing_source_faces() {
    let mut model = valid_tetrahedron();
    model.source_faces.clear();
    assert_eq!(
        model.validate_invariants(),
        Err(vec!["no source faces retained".to_string()])
    );
}

#[test]
fn rejects_inconsistent_nonempty_triangle_face_count() {
    let mut model = valid_tetrahedron();
    let triangles = model.mesh.positions.len() / 9;
    assert!(triangles > 1);
    model
        .triangle_faces
        .pop()
        .expect("tetrahedron has face tags");
    assert!(!model.triangle_faces.is_empty());
    assert_eq!(
        model.validate_invariants(),
        Err(vec![format!(
            "triangle_faces count ({}) != triangle count ({triangles})",
            triangles - 1
        )])
    );
}

#[test]
fn rejects_nonfinite_bounds_on_each_axis_and_endpoint() {
    let mut model = valid_tetrahedron();
    let valid_bounds = model.bounds;
    for endpoint in 0..2 {
        for axis in 0..3 {
            for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                model.bounds = valid_bounds;
                model.bounds[endpoint][axis] = value;
                assert_eq!(
                    model.validate_invariants(),
                    Err(vec![format!(
                        "bounds[{endpoint}][{axis}] is not finite: {value}"
                    )])
                );
            }
        }
    }
}

#[test]
fn rejects_bounds_without_any_increasing_axis_for_nonempty_mesh() {
    let mut model = valid_tetrahedron();
    for bounds in [[[0.0; 3]; 2], [[1.0; 3], [0.0; 3]]] {
        model.bounds = bounds;
        assert_eq!(
            model.validate_invariants(),
            Err(vec!["bounds are degenerate (zero volume)".to_string()])
        );
    }
}
