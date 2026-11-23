//! Small builders used by more than one integration test.

use boardstudio_core::model::{Operation, OutlineFeature, Vec2};
use serde_json::json;

pub(crate) fn rect(id: &str, center: Vec2, size: Vec2, operation: Operation) -> OutlineFeature {
    serde_json::from_value(json!({
        "id": id, "kind": "rect", "center": center, "size": size,
        "radius": 0, "operation": operation
    }))
    .unwrap()
}
