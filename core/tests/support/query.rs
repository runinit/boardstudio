//! Queries on public JSON replies and intentionally untyped fixtures.

use boardstudio_core::model::ProjectDoc;
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
