use monstertruck_geometry::prelude::*;

#[test]
fn authored_plane_serde_roundtrip_reconstructs_certificate_and_legacy_identity() {
    let o = Point3::new(17.0, -29.0, 41.0);
    let source = Plane::new(
        o,
        o + Vector3::new(2.0, -1.0, 0.0),
        o + Vector3::new(2.0, 4.0, -5.0),
    );
    let matrix = Matrix4::new(
        1.1, 0.2, 0.3, 0.0, 0.4, 1.7, 0.6, 0.0, 0.7, 0.8, 2.3, 0.0, 3.1, 5.2, -7.3, 1.0,
    );
    let moved = source.transformed(matrix);
    let encoded = serde_json::to_value(moved).unwrap();
    let restored: Plane = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(restored, moved);
    let surface = monstertruck_modeling::Surface::Plane(restored);
    assert!(surface.affine_plane_certificate().unwrap().coincides(&moved.affine_plane_certificate().unwrap()));
    let boxed = Box::new(restored);
    assert!(boxed.affine_plane_certificate().unwrap().coincides(&moved.affine_plane_certificate().unwrap()));
    assert!(
        restored.affine_plane_certificate().unwrap().coincides(
            &source
                .affine_plane_certificate()
                .unwrap()
                .transformed(matrix.into())
                .unwrap()
        )
    );
    assert_eq!(restored.evaluate(0.2, 0.4), moved.evaluate(0.2, 0.4));
    let mut legacy = serde_json::to_value(source).unwrap();
    legacy.as_object_mut().unwrap().remove("transform");
    let legacy: Plane = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy, source);
    let invalid = source.transformed(Matrix4::from_nonuniform_scale(0.0, 1.0, 1.0));
    assert!(invalid.affine_plane_certificate().is_none());
    // A serialized transform is geometry, not a trusted planarity assertion.
    let mut invalid = encoded;
    let mut projective = Matrix4::identity();
    projective.x.w = 0.01;
    invalid["transform"] = serde_json::to_value(projective).unwrap();
    assert!(serde_json::from_value::<Plane>(invalid).is_err());
}
