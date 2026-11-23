//! Queries on public JSON replies and intentionally untyped fixtures.

use boardstudio_core::model::{Contour, ProjectDoc};
use serde_json::Value;

#[track_caller]
pub(crate) fn arr(value: &Value) -> &[Value] {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected an array, got {value}"))
}

#[track_caller]
pub(crate) fn ids<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    arr(&value[key])
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect()
}

#[track_caller]
pub(crate) fn any(value: &Value, key: &str, predicate: impl FnMut(&Value) -> bool) -> bool {
    arr(&value[key]).iter().any(predicate)
}

pub(crate) fn empty_project(id: &str, name: &str) -> Value {
    serde_json::to_value(ProjectDoc::empty(id, name)).unwrap()
}

/// Independent ray-cast oracle: deliberately not the production predicate it checks.
pub(crate) fn contains(contours: &[Contour], x: f64, y: f64) -> bool {
    contours.iter().fold(false, |inside, contour| {
        let mut hit = false;
        for (a, b) in contour
            .points
            .iter()
            .zip(contour.points.iter().cycle().skip(1))
            .take(contour.points.len())
        {
            if (a.y > y) != (b.y > y) && x < (b.x - a.x) * (y - a.y) / (b.y - a.y) + a.x {
                hit = !hit;
            }
        }
        inside != hit
    })
}

/// Rigid printed-case mechanical configuration shared by the mounted-module scenario tests.
/// `plate_to_pcb` varies per scenario (6 for standoff stacks, 3.5 for shells).
pub(crate) fn rigid_mechanical(board_id: &str, plate_to_pcb: f64) -> Value {
    serde_json::json!({"boardId":board_id,"method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell",
        "plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,
        "plateToPcb":plate_to_pcb,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]})
}

/// Host board plus 80x80 outline shared by the mounted-module scenario tests.
pub(crate) fn module_host_document() -> Value {
    let mut doc = empty_project("modules", "Modules");
    doc["boards"] = serde_json::json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    doc["outline"] = serde_json::json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}]);
    doc
}

/// Reference mounted-module definition with its `mh1` mount at (`x`, `y`).
pub(crate) fn module_reference_definition(x: f64, y: f64) -> Value {
    serde_json::json!([{
        "id":"reference","name":"Mounted module","family":"test","variant":"support",
        "source":{"repository":"https://example.invalid/module","revision":"test","path":"pcb","license":"CC0-1.0"},
        "board":{"thickness":1.6,"contours":[{"hole":false,"points":[{"x":-12.5,"y":-12.5},{"x":12.5,"y":-12.5},{"x":12.5,"y":12.5},{"x":-12.5,"y":12.5}]}]},
        "mounts":[{"sourceId":"mh1","at":{"x":x,"y":y},"diameter":2.2}],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[],
        "electrical":{"protocol":"pass-through","requiredSignals":[]},"constituents":[]
    }])
}
