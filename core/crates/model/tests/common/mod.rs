use boardstudio_model::model::{Pose2, Vec2};

const TOLERANCE: f64 = 1e-9;

pub fn pair_map() -> Pose2 {
    Pose2 {
        at: Vec2 { x: 20.0, y: -7.0 },
        rotation: 36.86989764584402,
    }
}

pub fn assert_close(actual: Pose2, x: f64, y: f64, rotation: f64) {
    assert!(
        (actual.at.x - x).abs() < TOLERANCE
            && (actual.at.y - y).abs() < TOLERANCE
            && (actual.rotation - rotation).abs() < TOLERANCE,
        "{actual:?} is not ({x}, {y}) heading {rotation}"
    );
}
