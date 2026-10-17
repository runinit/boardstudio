use boardstudio_cad::{ProfileInput, ProfileKind, ProfileRegion, build_profile};
use boardstudio_contracts::Vec2;
#[test]
fn opaque_exact_model_reports_conservative_cache_cost() {
    let model = build_profile(ProfileInput {
        kind: ProfileKind::Plate,
        base_z: 0.,
        thickness: 2.,
        wall_height: 0.,
        regions: vec![ProfileRegion {
            outer: vec![
                Vec2 { x: 0., y: 0. },
                Vec2 { x: 20., y: 0. },
                Vec2 { x: 20., y: 16. },
                Vec2 { x: 0., y: 16. },
            ],
            holes: vec![],
        }],
    })
    .unwrap();
    assert_eq!(
        model.cache_complexity(),
        (1, 30),
        "one box has six faces and24 oriented edge occurrences"
    );
}
