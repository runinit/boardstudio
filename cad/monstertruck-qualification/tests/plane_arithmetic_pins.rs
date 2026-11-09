use monstertruck_geometry::prelude::*;
fn bits(point: Point3) -> [u64; 3] {
    [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()]
}
fn legacy_plane(o: Point3, p: Point3, q: Point3) -> Plane {
    let mut record = serde_json::to_value(Plane::new(o, p, q)).unwrap();
    record.as_object_mut().unwrap().remove("transform");
    serde_json::from_value(record).unwrap()
}
#[test]
fn authored_affine_evaluation_pins_intentional_source_then_matrix_order() {
    let o = Point3::new(17.0, -29.0, 41.0);
    let p = o + Vector3::new(2.0, -1.0, 0.0);
    let q = o + Vector3::new(2.0, 4.0, -5.0);
    let matrix = Matrix4::new(
        1.1, 0.2, 0.3, 0.0, 0.4, 1.7, 0.6, 0.0, 0.7, 0.8, 2.3, 0.0, 3.1, 5.2, -7.3, 1.0,
    );
    let source = Plane::new(o, p, q);
    let moved = source.transformed(matrix);
    let expected = [0x4043b0a3d70a3d71, 0xc01b851eb851eb7b, 0x4051d33333333333];
    assert_eq!(
        bits(matrix.transform_point(o + 0.2 * (p - o) + 0.4 * (q - o))),
        expected
    );
    assert_eq!(bits(moved.evaluate(0.2, 0.4)), expected);
    let (o, p, q) = (
        matrix.transform_point(o),
        matrix.transform_point(p),
        matrix.transform_point(q),
    );
    // This bug fix deliberately replaces independently rounded world-corner interpolation.
    assert_ne!(bits(o + 0.2 * (p - o) + 0.4 * (q - o)), expected);
}
#[test]
fn legacy_identity_records_keep_saved_arithmetic_and_signed_zero_bits() {
    let o = Point3::new(17.125, -29.375, 41.625);
    let p = Point3::new(19.425, -30.875, 41.625);
    let q = Point3::new(19.825, -25.175, 36.025);
    let plane = legacy_plane(o, p, q);
    let expected = [0x4032aa3d70a3d70a, 0xc03bfeb851eb851f, 0x4043b147ae147ae1];
    assert_eq!(bits(o + 0.2 * (p - o) + 0.4 * (q - o)), expected);
    assert_eq!(bits(plane.evaluate(0.2, 0.4)), expected);
    assert_eq!(
        bits(monstertruck_geometry::base::v2::ParametricSurface::evaluate(&plane, 0.2, 0.4)),
        expected
    );
    let converted: BsplineSurface<Point3> = plane.into();
    assert_eq!(bits(converted.control_points()[0][0]), bits(o));
    assert_eq!(bits(converted.control_points()[0][1]), bits(q));
    assert_eq!(bits(converted.control_points()[1][0]), bits(p));
    assert_eq!(bits(converted.control_points()[1][1]), bits(p + (q - o)));
    let o = Point3::new(1e16, 1e16, 1e16);
    let p = Point3::new(1.0, 1e16, 1e16);
    let q = Point3::new(1e16, 1.0, 1e16 + 2.0);
    let converted: BsplineSurface<Point3> = legacy_plane(o, p, q).into();
    assert_eq!(bits(converted.control_points()[1][0]), bits(p));
    assert_eq!(bits(converted.control_points()[1][1]), bits(p + (q - o)));
    let negative_zero = -0.0f64;
    let origin = legacy_plane(
        Point3::new(negative_zero, negative_zero, negative_zero),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
    );
    assert_eq!(bits(origin.origin()), [negative_zero.to_bits(); 3]);
    let axes = legacy_plane(
        Point3::origin(),
        Point3::new(1.0, negative_zero, negative_zero),
        Point3::new(negative_zero, 1.0, negative_zero),
    );
    assert_eq!(
        [
            axes.axis_u().x.to_bits(),
            axes.axis_u().y.to_bits(),
            axes.axis_u().z.to_bits()
        ],
        [
            1.0f64.to_bits(),
            negative_zero.to_bits(),
            negative_zero.to_bits()
        ]
    );
}
