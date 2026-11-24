//! The persisted companion pose survives a matrix refresh and an engine `Open` exactly.
//!
//! It lives here, one layer above `boardstudio-core-kernel`, so that crate's own tests do not
//! depend on the engine (a dev-dependency cycle would hold its test binary back until the whole
//! engine graph is built).

use boardstudio_core_engine::model::{MatrixCompanionMode, Pose2, ProjectDoc, Vec2};
use boardstudio_core_engine::{CommittedScene, CoreEngine};
use boardstudio_core_kernel::matrix::{layout, set_matrix};
use serde_json::json;

#[test]
fn following_companion_preserves_materialized_pose_exactly_on_refresh_and_reopen() {
    let mut doc = ProjectDoc::empty("exact-companion", "Exact companion");
    doc.definitions = serde_json::from_value(json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"diode","name":"Diode","kind":"passive","courtyard":[],"pads":[]}
    ]))
    .unwrap();
    let matrix = serde_json::from_value(json!({
        "id":"m","rows":1,"columns":1,"pitch":{"x":19,"y":19},
        "origin":{"x":1,"y":-1},"definitionId":"switch","partIds":[],
        "cells":[{"row":0,"column":0,"enabled":true,"rotation":90,
            "assemblies":[{"id":"diode","definitionId":"diode",
                "offset":{"x":1,"y":1},"rotation":17}]}]
    }))
    .unwrap();
    set_matrix(&mut doc, &matrix).unwrap();
    let companion = |doc: &ProjectDoc| {
        doc.parts
            .iter()
            .find(|part| part.id == "matrix/m/r0c0/diode")
            .unwrap()
            .pose
    };
    let key = doc
        .parts
        .iter()
        .find(|part| part.id == "matrix/m/r0c0")
        .unwrap();
    assert_eq!(
        key.pose,
        Pose2 {
            at: Vec2 { x: 1.0, y: -1.0 },
            rotation: 90.0
        }
    );
    let (sin, cos) = 90_f64.to_radians().sin_cos();
    // Pin the original materialization grouping: persisted addition is not associative.
    let expected = Pose2 {
        at: Vec2 {
            x: (1.0 + cos) - sin,
            y: (-1.0 + sin) + cos,
        },
        rotation: 90.0 + 17.0,
    };
    assert_ne!(
        expected.at,
        Vec2 {
            x: 1.0 + (cos - sin),
            y: -1.0 + (sin + cos)
        }
    );
    let materialized = companion(&doc);
    assert_eq!(materialized, expected, "original materialization order");
    assert_eq!(
        layout::companion_mode(&doc, "matrix/m/r0c0/diode"),
        MatrixCompanionMode::Follows
    );
    layout::refresh_existing(&mut doc).unwrap();
    assert_eq!(
        companion(&doc),
        materialized,
        "refresh must preserve persisted pose exactly"
    );
    let saved = serde_json::to_string(&doc).unwrap();
    let mut engine = CoreEngine::new();
    let reply = engine.open(serde_json::from_str(&saved).unwrap());
    let Ok(CommittedScene { document, .. }) = reply else {
        panic!("expected reopened scene: {reply:?}");
    };
    assert_eq!(
        companion(&document),
        materialized,
        "Open must preserve persisted pose exactly"
    );
}
