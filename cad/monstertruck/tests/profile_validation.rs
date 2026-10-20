mod common;

use boardstudio_cad::build_case;
use boardstudio_contracts::Vec2;

fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn profile(outer: Vec<Vec2>, holes: Vec<Vec<Vec2>>) -> Result<boardstudio_cad::ExactModel, String> {
    build_case(&common::plate(1.0, outer, holes))
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
