//! Helpers shared by generator bodies.
use serde_json::Value;

use crate::context::RenderContext;
use crate::error::{GeneratorError, Result};
use crate::number::js_number;

/// `(xy x y) ` for each `[x, y]` of a list-of-pairs parameter.
pub(crate) fn points_text(generator: &str, p: &RenderContext<'_>, name: &str) -> Result<String> {
    let mut out = String::new();
    for point in p.list(name) {
        let pair: Option<Vec<f64>> = point
            .as_array()
            .and_then(|items| items.iter().map(Value::as_f64).collect());
        match pair.as_deref() {
            Some(&[x, y]) => out.push_str(&format!("(xy {} {}) ", js_number(x), js_number(y))),
            _ => {
                return Err(GeneratorError::InvalidParameter {
                    generator: generator.to_owned(),
                    parameter: name.to_owned(),
                    expected: "a list of [x, y] pairs",
                });
            }
        }
    }
    Ok(out)
}

/// `x y z` for a three-number vector.
#[allow(dead_code)]
pub(crate) fn xyz(vector: [f64; 3]) -> String {
    format!(
        "{} {} {}",
        js_number(vector[0]),
        js_number(vector[1]),
        js_number(vector[2])
    )
}

/// JSON text of a string, as `JSON.stringify` writes it.
pub(crate) fn json_string(text: &str) -> String {
    serde_json::to_string(text).expect("strings serialize")
}

pub(crate) fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}
