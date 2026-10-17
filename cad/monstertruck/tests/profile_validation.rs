use boardstudio_cad::{ProfileInput, ProfileKind, ProfileRegion, build_profile};
use boardstudio_contracts::Vec2;

fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn profile(outer: Vec<Vec2>, holes: Vec<Vec<Vec2>>) -> Result<boardstudio_cad::ExactModel, String> {
    build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.0,
        thickness: 1.0,
        wall_height: 0.0,
        regions: vec![ProfileRegion { outer, holes }],
    })
}

#[test]
fn accepts_a_strictly_inside_near_collinear_hole() {
    let dx = 11_463_935_597_161.09_f64;
    let dy = 15_139_984_008_548.072_f64;
    let inside_y = f64::from_bits(dy.to_bits() + 1);
    let hole_x = f64::from_bits(dx.to_bits() - 5);
    let lower_hole_y = f64::from_bits(inside_y.to_bits() - 5);
    let upper_hole_y = f64::from_bits(lower_hole_y.to_bits() + 1);

    let exterior = [
        point(0.0, 0.0),
        point(2.0 * dx, 2.0 * dy),
        point(-2.0 * dy, 2.0 * dx),
    ];
    let hole = [
        point(dx, inside_y),
        point(hole_x, lower_hole_y),
        point(hole_x, upper_hole_y),
    ];

    // The exact determinant for the first hole vertex against the exterior's
    // first edge is positive, but the ordinary f64 determinant rounds to zero.
    assert_eq!(
        (exterior[1].x - exterior[0].x) * (hole[0].y - exterior[0].y)
            - (exterior[1].y - exterior[0].y) * (hole[0].x - exterior[0].x),
        0.0
    );

    let result = profile(exterior.to_vec(), vec![hole.to_vec()]);
    assert!(
        result.is_ok(),
        "strictly contained exact hole rejected: {:?}",
        result.err()
    );
}

#[test]
fn rejects_holes_that_touch_or_cross_the_exterior() {
    let exterior = vec![
        point(0.0, 0.0),
        point(10.0, 0.0),
        point(10.0, 10.0),
        point(0.0, 10.0),
    ];
    let touching = vec![
        point(0.0, 2.0),
        point(3.0, 2.0),
        point(3.0, 4.0),
        point(0.0, 4.0),
    ];
    let crossing = vec![
        point(8.0, 3.0),
        point(12.0, 3.0),
        point(12.0, 5.0),
        point(8.0, 5.0),
    ];

    for hole in [touching, crossing] {
        let result = profile(exterior.clone(), vec![hole]);
        assert!(result.is_err(), "boundary contact must be rejected");
    }
}
