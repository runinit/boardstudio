use monstertruck_modeling::bounding::certified_solid_bounding_box;
use monstertruck_qualification::bounded_dish::{Profile, cutter_ceiling, dish_cutter};

#[test]
fn public_profile_and_constructor_api_build_and_certify_a_spherical_cutter() {
    let profile = Profile {
        label: "api-dsa",
        width: 18.2,
        depth: 18.2,
        top_w: 13.2,
        top_d: 13.2,
        height: 7.5,
        tilt: 0.0,
        dish_depth: 0.65,
        wall: 1.2,
        spherical: true,
    };
    assert!(cutter_ceiling(profile).is_finite());
    let cutter = dish_cutter(profile).unwrap().expect("positive dish depth");
    assert!(cutter.is_geometric_consistent());
    assert!(certified_solid_bounding_box(&cutter).is_some());
}

#[test]
fn public_constructor_api_preserves_zero_depth_bypass() {
    let profile = Profile {
        label: "api-no-dish",
        width: 18.2,
        depth: 18.2,
        top_w: 13.2,
        top_d: 13.2,
        height: 7.5,
        tilt: 0.0,
        dish_depth: 0.0,
        wall: 1.2,
        spherical: false,
    };
    assert!(dish_cutter(profile).unwrap().is_none());
}

#[test]
fn public_constructor_api_preserves_positive_scale_refusal() {
    let profile = Profile {
        label: "api-uncertifiable-scale",
        width: 18.2,
        depth: 18.2,
        top_w: 13.2,
        top_d: 13.2,
        height: 7.5,
        tilt: 0.0,
        dish_depth: f64::from_bits(1),
        wall: 1.2,
        spherical: true,
    };
    let error = dish_cutter(profile).expect_err("uncertifiable positive depth must refuse");
    assert!(
        error.contains("scale") || error.contains("certif"),
        "{error}"
    );
}
