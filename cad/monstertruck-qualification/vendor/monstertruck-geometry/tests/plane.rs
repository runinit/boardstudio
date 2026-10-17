#[test]
fn into_bspline() {
    use monstertruck_geometry::prelude::*;
    let pt0 = Point3::new(0.0, 1.0, 2.0);
    let pt1 = Point3::new(1.0, 1.0, 3.0);
    let pt2 = Point3::new(0.0, 2.0, 3.0);
    let plane: Plane = Plane::new(pt0, pt1, pt2);
    let surface: BsplineSurface<Point3> = plane.into();
    assert_eq!(surface.range_tuple(), ((0.0, 1.0), (0.0, 1.0)));

    const N: usize = 100;
    for i in 0..=N {
        for j in 0..=N {
            let u = i as f64 / N as f64;
            let v = j as f64 / N as f64;
            let res = surface.subs(u, v);
            let ans = plane.subs(u, v);
            assert_near!(ans, res);
        }
    }
}

#[test]
fn common_plane_certificate_survives_authored_affine_transform() {
    use monstertruck_geometry::prelude::*;
    let o = Point3::new(17.0, -29.0, 41.0);
    let u = Vector3::new(2.0, -1.0, 0.0);
    let v = Vector3::new(2.0, 4.0, -5.0);
    let a = Plane::new(o, o + u, o + v);
    let b = Plane::new(o + u, o + 3.0*u, o + 2.0*u + v);
    assert!(a.affine_plane_certificate().unwrap().coincides(&b.affine_plane_certificate().unwrap()));
    let matrix = Matrix4::new(1.1,0.2,0.3,0.0, 0.4,1.7,0.6,0.0, 0.7,0.8,2.3,0.0, 3.1,5.2,-7.3,1.0);
    let processed = Processor::new(a).transformed(matrix);
    let a = a.transformed(matrix);
    let b = b.transformed(matrix);
    assert!(a.affine_plane_certificate().unwrap().coincides(&processed.affine_plane_certificate().unwrap()));
    assert!(a.affine_plane_certificate().unwrap().coincides(&b.affine_plane_certificate().unwrap()), "shared authored affine transform must preserve exact common-plane relation");
}

#[test]
fn authored_plane_transport_preserves_affine_geometry_and_conversion() {
    use monstertruck_geometry::prelude::*;
    let o = Point3::new(17.0, -29.0, 41.0);
    let source = Plane::new(o, o + Vector3::new(2.0,-1.0,0.0), o + Vector3::new(2.0,4.0,-5.0));
    let matrix = Matrix4::new(1.1,0.2,0.3,0.0, 0.4,1.7,0.6,0.0, 0.7,0.8,2.3,0.0, 3.1,5.2,-7.3,1.0);
    let moved = source.transformed(matrix);
    assert_near!(moved.origin(), matrix.transform_point(source.origin()));
    assert_near!(moved.axis_u(), matrix.transform_vector(source.axis_u()));
    assert_near!(moved.axis_v(), matrix.transform_vector(source.axis_v()));
    assert_near!(moved.normal(), moved.axis_u().cross(moved.axis_v()).normalize());
    let spline: BsplineSurface<Point3> = moved.into();
    for (u,v) in [(0.0,0.0), (0.2,0.4), (-3.0,7.0), (1.0,1.0)] {
        let expected = matrix.transform_point(source.evaluate(u,v));
        assert_near!(moved.evaluate(u,v), expected);
        assert_near!(moved.derivative_mn(1,0,u,v), matrix.transform_vector(source.axis_u()));
        assert_near!(moved.derivative_v(u,v), matrix.transform_vector(source.axis_v()));
        assert_near!(moved.inverse().evaluate(v,u), expected);
        let parameter = moved.parameter(expected + 0.25*moved.normal());
        assert_near!(parameter, Vector3::new(u,v,0.25));
        assert_near!(spline.evaluate(u,v), expected);
    }
    let again = Matrix4::from_translation(Vector3::new(-2.3,5.4,8.7))*Matrix4::from_nonuniform_scale(2.1,0.7,1.3);
    assert_near!(moved.transformed(again).evaluate(0.2,0.4), again.transform_point(moved.evaluate(0.2,0.4)));
}

#[test]
fn projective_plane_transform_retains_historical_three_point_image() {
    use monstertruck_geometry::prelude::*;
    let source = Plane::new(Point3::new(2.0,3.0,4.0), Point3::new(3.0,3.0,4.0), Point3::new(2.0,4.0,4.0));
    let mut matrix = Matrix4::identity();
    matrix.x.w = 0.01;
    let historical = Plane::new(matrix.transform_point(source.origin()), matrix.transform_point(source.evaluate(1.0,0.0)), matrix.transform_point(source.evaluate(0.0,1.0)));
    let moved = source.transformed(matrix);
    assert_near!(moved.evaluate(0.2,0.4), historical.evaluate(0.2,0.4));
    assert!(moved.affine_plane_certificate().unwrap().coincides(&historical.affine_plane_certificate().unwrap()));
}
