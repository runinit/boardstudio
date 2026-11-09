use monstertruck_modeling::*;

fn cylinder(frame: Matrix4) -> Surface {
    Surface::RevolutionSurface(Processor::with_transform(
        RevolutionSurface::by_revolution(
            Curve::Line(Line(Point3::new(1.6, 0., 2.), Point3::new(1.6, 0., 4.))),
            Point3::origin(),
            Vector3::unit_z(),
        ),
        frame,
    ))
}

fn generator(radius: f64) -> Curve {
    Curve::Line(Line(
        Point3::new(radius, 0., 2.25),
        Point3::new(radius, 0., 3.75),
    ))
}

#[test]
fn translated_rotated_native_cylinder_includes_only_its_world_curves() {
    let a = builder::vertex(Point3::new(1.6, 0., 3.));
    let b = builder::vertex(Point3::new(-1.6, 0., 3.));
    let edge: Edge = builder::try_circle_arc(&a, &b, Vector3::unit_y()).unwrap();
    let arc = edge.curve();
    for frame in [
        Matrix4::identity(),
        Matrix4::from_translation(Vector3::new(10., 1., -3.)),
        Matrix4::from_translation(Vector3::new(10., 1., -3.))
            * Matrix4::from_angle_x(Rad(0.4))
            * Matrix4::from_angle_z(Rad(-0.7)),
    ] {
        let surface = cylinder(frame);
        // These points/curves are authored in the cylinder's local analytic frame.
        assert!(surface.include(&generator(1.6).transformed(frame)));
        let spline = Curve::BsplineCurve(BsplineCurve::from(Line(
            Point3::new(1.6, 0., 2.25),
            Point3::new(1.6, 0., 3.75),
        )));
        assert!(surface.include(&spline.transformed(frame)));
        assert!(surface.include(&arc.transformed(frame)));
        assert!(!surface.include(&generator(1.7).transformed(frame)));
    }
}

#[test]
fn invalid_revolution_frames_fail_closed() {
    for frame in [
        Matrix4::from_nonuniform_scale(0., 1., 1.),
        Matrix4::from_nonuniform_scale(1e-310, 1., 1.),
        Matrix4::from_translation(Vector3::new(f64::NAN, 0., 0.)),
        Matrix4::from_translation(Vector3::new(f64::INFINITY, 0., 0.)),
    ] {
        assert!(!cylinder(frame).include(&generator(1.6)));
    }
}
