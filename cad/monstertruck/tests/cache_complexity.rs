mod common;

use boardstudio_cad::build_case;
use boardstudio_contracts::Vec2;
#[test]
fn opaque_exact_model_reports_conservative_cache_cost() {
    let model = build_case(&common::plate(
        2.,
        vec![
            Vec2 { x: 0., y: 0. },
            Vec2 { x: 20., y: 0. },
            Vec2 { x: 20., y: 16. },
            Vec2 { x: 0., y: 16. },
        ],
        vec![],
    ))
    .unwrap();
    assert_eq!(
        model.cache_complexity(),
        (1, 30),
        "one box has six faces and24 oriented edge occurrences"
    );
}
