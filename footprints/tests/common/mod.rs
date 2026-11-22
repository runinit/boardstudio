#![allow(dead_code)]
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

pub fn read_json(relative: &str) -> Value {
    let path = golden_dir().join(relative);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Parse a golden JSON file, keeping the raw text so callers can recover
/// details the parsed `Value` forgets (notably object key order, which
/// `serde_json` only preserves with `preserve_order`).
pub fn read_json_raw(relative: &str) -> (Value, String) {
    let path = golden_dir().join(relative);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let value: Value =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    (value, text)
}

fn generator_fixture_paths() -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(golden_dir().join("generators"))
        .expect("golden generators directory")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();
    paths
}

/// Every generator fixture file, in name order.
pub fn generator_fixtures() -> Vec<Value> {
    generator_fixture_paths()
        .iter()
        .map(|path| serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap())
        .collect()
}

/// Every generator fixture with its raw text, in file name order.
pub fn generator_fixture_raws() -> Vec<(Value, String)> {
    generator_fixture_paths()
        .iter()
        .map(|path| {
            let text = fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let value: Value = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            (value, text)
        })
        .collect()
}

/// JSON equality that compares numbers by value (the fixtures write `1`, serde
/// writes `1.0`) and ignores object key order.
pub fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| json_eq(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(key, value)| y.get(key).is_some_and(|other| json_eq(value, other)))
        }
        _ => a == b,
    }
}

pub fn assert_json_eq(actual: &Value, expected: &Value, context: &str) {
    assert!(
        json_eq(actual, expected),
        "{context}\n  actual:   {actual}\n  expected: {expected}"
    );
}

pub mod qualification;
