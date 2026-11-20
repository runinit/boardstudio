use boardstudio_model::model::EditOperation;
use serde_json::Value;

pub(crate) fn op(value: Value) -> EditOperation {
    serde_json::from_value(value).expect("the operation is valid")
}

/// Compare JSON numbers within tolerance for rotated maps.
pub(crate) fn assert_close(actual: &Value, expected: &Value, path: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!((a - e).abs() < 1e-9, "{path}: {a} is not {e}");
        }
        (Value::Object(a), Value::Object(e)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                e.keys().collect::<Vec<_>>(),
                "{path}: keys"
            );
            for (key, value) in e {
                assert_close(&a[key], value, &format!("{path}.{key}"));
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(
                a.len(),
                e.len(),
                "{path}: length of {actual} against {expected}"
            );
            for (index, (a, e)) in a.iter().zip(e).enumerate() {
                assert_close(a, e, &format!("{path}[{index}]"));
            }
        }
        _ => assert_eq!(actual, expected, "{path}"),
    }
}
