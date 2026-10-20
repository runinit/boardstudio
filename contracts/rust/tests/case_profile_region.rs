use boardstudio_contracts::normalize_case_profile_region;

#[test]
fn accepts_a_strictly_inside_near_collinear_hole() {
    let dx = 11_463_935_597_161.09_f64;
    let dy = 15_139_984_008_548.072_f64;
    let inside_y = f64::from_bits(dy.to_bits() + 1);
    let hole_x = f64::from_bits(dx.to_bits() - 5);
    let lower_hole_y = f64::from_bits(inside_y.to_bits() - 5);
    let upper_hole_y = f64::from_bits(lower_hole_y.to_bits() + 1);

    let exterior = [[0.0, 0.0], [2.0 * dx, 2.0 * dy], [-2.0 * dy, 2.0 * dx]];
    let hole = vec![
        [dx, inside_y],
        [hole_x, lower_hole_y],
        [hole_x, upper_hole_y],
    ];

    // The exact determinant for the first hole vertex against the exterior's
    // first edge is positive, but the ordinary f64 determinant rounds to zero.
    assert_eq!(
        (exterior[1][0] - exterior[0][0]) * (hole[0][1] - exterior[0][1])
            - (exterior[1][1] - exterior[0][1]) * (hole[0][0] - exterior[0][0]),
        0.0
    );

    let result = normalize_case_profile_region(&exterior, &[hole]);
    assert!(
        result.is_ok(),
        "strictly contained exact hole rejected: {:?}",
        result.err()
    );
}
