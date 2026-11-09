//! Unit tests for the parent module.
//!
//! Split out of the module file so the source stays readable; the module
//! path is unchanged.

use super::*;

fn direction(ratios: [f64; 3]) -> Direction {
    Direction {
        label: String::new(),
        direction_ratios: ratios.to_vec(),
    }
}

/// Direct infallible conversion keeps a defensive fallback for parallel
/// inputs. Fallible STEP transformation and surface paths reject explicit
/// parallel directions under ISO 10303-42 WR4.
#[test]
fn axis2_placement3d_parallel_axis_and_ref_direction_is_finite() {
    let placement = Axis2Placement3d {
        label: String::new(),
        location: CartesianPoint {
            label: String::new(),
            coordinates: vec![1.0, 2.0, 3.0],
        },
        axis: Some(direction([0.0, 0.0, 1.0])),
        // Parallel to `axis` -- the degenerate case.
        ref_direction: Some(direction([0.0, 0.0, 1.0])),
    };

    let matrix = Matrix4::from(&placement);
    let (x, y, z) = (
        matrix.x.truncate(),
        matrix.y.truncate(),
        matrix.z.truncate(),
    );
    for component in [x.x, x.y, x.z, y.x, y.y, y.z, z.x, z.y, z.z] {
        assert!(component.is_finite(), "placement basis must be finite");
    }
    // The recovered basis is orthonormal.
    assert!((x.magnitude() - 1.0).abs() < 1.0e-9);
    assert!((y.magnitude() - 1.0).abs() < 1.0e-9);
    assert!(x.dot(z).abs() < 1.0e-9);
    assert!(y.dot(z).abs() < 1.0e-9);
}

#[test]
fn placement_validation_defaults_and_scaled_directions_preserve_frame() {
    let make = |axis, reference| Axis2Placement3d {
        label: String::new(),
        location: CartesianPoint {
            label: String::new(),
            coordinates: vec![2.0, 3.0, 4.0],
        },
        axis,
        ref_direction: reference,
    };
    let direction = |ratios: [f64; 3]| Direction {
        label: String::new(),
        direction_ratios: ratios.to_vec(),
    };
    let unit = make(
        Some(direction([0.0, 0.0, 1.0])),
        Some(direction([1.0, 0.0, 1.0])),
    );
    let scaled_nonorthogonal = make(
        Some(direction([0.0, 0.0, 2.0])),
        Some(direction([2.0, 0.0, 2.0])),
    );
    let unit_frame = Matrix4::from(&unit);
    let scaled_frame = Matrix4::from(&scaled_nonorthogonal);
    for (a, b) in [
        (unit_frame.x, scaled_frame.x),
        (unit_frame.y, scaled_frame.y),
        (unit_frame.z, scaled_frame.z),
        (unit_frame.w, scaled_frame.w),
    ] {
        for (a, b) in [(a.x, b.x), (a.y, b.y), (a.z, b.z), (a.w, b.w)] {
            assert!((a - b).abs() < 1e-12, "{a} != {b}");
        }
    }

    let defaults = make(None, None);
    assert!(validate_axis2_placement3d(&defaults).is_ok());
    assert_eq!(Matrix4::from(&defaults).z.truncate(), Vector3::unit_z());
    assert_eq!(Matrix4::from(&defaults).x.truncate(), Vector3::unit_x());
}

#[test]
fn placement_validation_is_invariant_to_tiny_and_huge_direction_ratios() {
    let direction = |ratios: [f64; 3]| Direction {
        label: String::new(),
        direction_ratios: ratios.to_vec(),
    };
    let frame = |axis, reference| Axis2Placement3d {
        label: String::new(),
        location: CartesianPoint::default(),
        axis: Some(direction(axis)),
        ref_direction: Some(direction(reference)),
    };
    let reference = frame([0.0, 0.0, 1.0], [1.0, 1.0, 0.0]);
    let reference_matrix = Matrix4::from(&reference);
    for scale in [1e-300, 1e-4, 1e200, 1e300] {
        let scaled_axis = frame([0.0, 0.0, scale], [1.0, 1.0, 0.0]);
        let scaled_reference = frame([0.0, 0.0, 1.0], [scale, scale, 0.0]);
        let scaled_both = frame([0.0, 0.0, scale], [scale, scale, 0.0]);
        for candidate in [scaled_axis, scaled_reference, scaled_both] {
            let matrix = Matrix4::from(&candidate);
            for (a, b) in [
                (reference_matrix.x, matrix.x),
                (reference_matrix.y, matrix.y),
                (reference_matrix.z, matrix.z),
                (reference_matrix.w, matrix.w),
            ] {
                for (a, b) in [(a.x, b.x), (a.y, b.y), (a.z, b.z), (a.w, b.w)] {
                    assert!((a - b).abs() < 1e-12, "{a} != {b} at scale {scale}");
                }
            }
        }
        assert!(validate_axis2_placement3d(&frame([0.0, 0.0, scale], [scale, scale, 0.0])).is_ok());
    }
}

#[test]
fn placement_validation_preserves_small_nonorthogonal_reference_orientation() {
    let frame = |reference: [f64; 3]| Axis2Placement3d {
        label: String::new(),
        location: CartesianPoint::default(),
        axis: Some(Direction {
            label: String::new(),
            direction_ratios: vec![0.0, 0.0, 1.0],
        }),
        ref_direction: Some(Direction {
            label: String::new(),
            direction_ratios: reference.to_vec(),
        }),
    };
    let unit = Matrix4::from(&frame([1.0, 1.0, 0.0]));
    let small = Matrix4::from(&frame([1e-4, 1e-4, 0.0]));
    assert!((unit.x.x - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((unit.x.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
    assert!((small.x.x - unit.x.x).abs() < 1e-12);
    assert!((small.x.y - unit.x.y).abs() < 1e-12);
}

#[test]
fn placement_validation_fallible_axis2placement_conversion_rejects_invalid_3d_directions() {
    let direction = |ratios: [f64; 3]| Direction {
        label: String::new(),
        direction_ratios: ratios.to_vec(),
    };
    let placement = |axis, reference| {
        Axis2Placement::Axis2Placement3d(Axis2Placement3d {
            label: String::new(),
            location: CartesianPoint::default(),
            axis: Some(direction(axis)),
            ref_direction: Some(direction(reference)),
        })
    };
    let valid = placement([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);
    assert!(Matrix4::try_from(&valid).is_ok());
    for invalid in [
        placement([0.0, 0.0, 1.0], [0.0, 0.0, 2.0]),
        placement([0.0, 0.0, 1.0], [1e-13, 0.0, 1.0]),
    ] {
        assert!(Matrix4::try_from(&invalid).is_err());
    }
}
