use super::*;
use crate::step::load::{
    Table,
    step_geometry::{ElementarySurface, Sphere as StepSphereEntity, Surface as StepSurfaceEntity},
    step_p21::tables::IntoOwned,
};
use crate::step::save::{CompleteStepDisplay, StepDisplay};
use monstertruck_modeling::cgmath::{Rad, SquareMatrix, Transform};
use monstertruck_modeling::Processor;

fn complete_step<T: StepFormat>(surface: T) -> String {
    CompleteStepDisplay::new(StepDisplay::new(surface, 1), Default::default()).to_string()
}

fn count_entities(step: &str) -> usize {
    step.lines().filter(|line| line.trim_start().starts_with('#')).count()
}

fn direct_surface(sphere: Sphere, transform: Matrix4) -> StepSurfaceEntity {
    StepSurfaceEntity::ElementarySurface(ElementarySurface::Sphere(Processor::with_transform(
        StepSphereEntity(sphere),
        transform,
    )))
}

fn assert_rational_fallback(step: &str) {
    assert!(!step.contains("SPHERICAL_SURFACE"));
    assert!(step.contains("RATIONAL_B_SPLINE_SURFACE"));
    let _ = Table::from_step(step).expect("fallback STEP should parse");
}

#[test]
fn transformed_non_origin_sphere_uses_transformed_center() {
    let sphere = Sphere::new(Point3::new(1.25, -2.5, 3.75), 2.0);
    let transform = Matrix4::from_translation(Vector3::new(7.0, -3.0, 11.0))
        * Matrix4::from_angle_z(Rad(0.37))
        * Matrix4::from_scale(2.25);
    let center = transform.transform_point(sphere.center());
    let step = complete_step(Processor::with_transform(sphere, transform));
    let table = Table::from_step(&step).expect("analytic STEP should parse");
    let holder = table
        .spherical_surface
        .values()
        .next()
        .expect("sphere entity");
    let owned = holder.clone().into_owned(&table).expect("load sphere");
    let loaded: crate::step::load::step_geometry::SphericalSurface = (&owned).into();
    let loaded = loaded.map_ref(|surface| surface.0);
    let loaded_center = loaded.transform().transform_point(loaded.entity().center());
    assert!(center.distance(loaded_center) <= 1.0e-8);
    assert!(step.contains("SPHERICAL_SURFACE"));
    assert_eq!(count_entities(&step), StepLength::step_length(&Processor::with_transform(sphere, transform)));
}

#[test]
fn unsupported_affine_transforms_fallback_on_modeling_and_step_surface_routes() {
    let sphere = Sphere::new(Point3::new(1.25, -2.5, 3.75), 2.0);
    let transforms = [
        Matrix4::from_nonuniform_scale(1.0, 1.0, 1.5),
        Matrix4::from_cols(
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.25, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(4.0, 5.0, 6.0, 1.0),
        ),
        Matrix4::from_nonuniform_scale(-1.0, 1.0, 1.0),
    ];

    for transform in transforms {
        let processor = Processor::with_transform(sphere, transform);
        let modeling_step = complete_step(ModelingSurface::SphericalSurface(processor));
        assert_rational_fallback(&modeling_step);
        assert_eq!(
            count_entities(&modeling_step),
            StepLength::step_length(&ModelingSurface::SphericalSurface(processor))
        );

        let direct = direct_surface(sphere, transform);
        let direct_step = complete_step(direct.clone());
        assert_rational_fallback(&direct_step);
        assert_eq!(count_entities(&direct_step), StepLength::step_length(&direct));
    }
}

#[test]
fn tiny_projective_term_with_large_coordinates_never_uses_analytic_sphere() {
    let sphere = Sphere::new(Point3::new(1.0e12, -2.0e12, 3.0e12), 1.0e12);
    let mut transform = Matrix4::identity();
    transform[0].w = 1.0e-15;
    assert!(transform[0].w.abs() < SPHERE_SIMILARITY_EPS);

    let raw = Processor::with_transform(sphere, transform);
    let raw_step = complete_step(raw);
    assert_rational_fallback(&raw_step);
    assert_eq!(count_entities(&raw_step), StepLength::step_length(&raw));

    let modeling_step = complete_step(ModelingSurface::SphericalSurface(raw));
    assert_rational_fallback(&modeling_step);
    assert_eq!(
        count_entities(&modeling_step),
        StepLength::step_length(&ModelingSurface::SphericalSurface(raw))
    );

    let direct = direct_surface(sphere, transform);
    let direct_step = complete_step(direct.clone());
    assert_rational_fallback(&direct_step);
    assert_eq!(count_entities(&direct_step), StepLength::step_length(&direct));
}
