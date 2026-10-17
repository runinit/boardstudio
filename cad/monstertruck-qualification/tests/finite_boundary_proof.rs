use monstertruck_geometry::prelude::*;
use monstertruck_io::step::load::{Table, step_geometry::Surface};

fn source_surface() -> Surface {
    let table = Table::from_step(include_str!("fixtures/choc-face30604.step")).unwrap();
    let (shell, report) = table
        .to_compressed_shell_reported(table.shell.values().next().unwrap())
        .unwrap();
    report.require_lossless().unwrap();
    shell.faces[0].surface.clone()
}

#[test]
fn actual_coefficient_proof_survives_units_affine_transforms_and_axis_swap() {
    let original = source_surface();
    assert_eq!(original.finite_boundary_equal_axes(), Some([true, false]));
    assert_eq!(original.period_u(), None);
    assert_eq!(original.period_v(), None);
    assert_eq!(
        ParametricSurface3D::finite_boundary_equal_axes(&&original),
        Some([true, false])
    );
    assert_eq!(
        ParametricSurface3D::finite_boundary_equal_axes(&Box::new(original.clone())),
        Some([true, false])
    );
    for factor in [0.001, 1.0, 25.4, 1000.0] {
        let mut transformed = original.clone();
        transformed.transform_by(
            Matrix4::from_translation(Vector3::new(12.0, -7.0, 3.0))
                * Matrix4::from_axis_angle(Vector3::unit_y(), Rad(0.37))
                * Matrix4::from_scale(factor),
        );
        assert_eq!(
            transformed.finite_boundary_equal_axes(),
            Some([true, false])
        );
        transformed.invert();
        assert_eq!(
            transformed.finite_boundary_equal_axes(),
            Some([false, true])
        );
    }
    assert_eq!(original.finite_boundary_equal_axes(), Some([true, false]));
}

#[test]
fn coefficient_proof_refuses_nearby_changed_rows_and_invalid_weights() {
    let Surface::NurbsSurface(original) = source_surface() else {
        panic!("authentic rational surface")
    };
    let mut changed = original.clone();
    let point = changed.control_point_mut(0, 0);
    point.x = f64::from_bits(point.x.to_bits() + 1);
    assert_ne!(point.x, original.control_point(0, 0).x);
    assert_eq!(changed.finite_boundary_equal_axes(), Some([false, false]));
    for weight in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut invalid = original.clone();
        invalid.control_point_mut(0, 0).w = weight;
        assert_eq!(invalid.finite_boundary_equal_axes(), None);
    }
    let mut overflowed = original.clone();
    overflowed.control_point_mut(0, 0).x = f64::INFINITY;
    assert_eq!(overflowed.finite_boundary_equal_axes(), None);
}

#[test]
fn unsupported_transform_wrapper_does_not_inherit_a_stale_boundary_claim() {
    let surface = source_surface();
    assert_eq!(surface.finite_boundary_equal_axes(), Some([true, false]));
    assert_eq!(
        Processor::<_, Matrix4>::new(surface).finite_boundary_equal_axes(),
        None
    );
    assert_eq!(Plane::xy().finite_boundary_equal_axes(), None);
}
