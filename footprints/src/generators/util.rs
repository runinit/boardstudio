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

/// `list.join(' ')` for a list of numbers.
pub(crate) fn join_numbers(items: &[Value]) -> String {
    items
        .iter()
        .map(|item| js_number(item.as_f64().unwrap_or(f64::NAN)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The parameter names that describe one 3D model in the infused-kim generators.
pub(crate) struct ModelParams<'a> {
    pub filename: &'a str,
    pub scale: &'a str,
    pub rotation: &'a str,
    pub offset: &'a str,
    /// The side selector; empty text follows the footprint side.
    pub side: &'a str,
}

/// Defaults for the front (`_f`) and back (`_b`) mounting of a model.
#[derive(Clone, Copy)]
pub(crate) struct ModelDefaults {
    pub default_side: &'static str,
    pub rotation_f: [f64; 3],
    pub offset_f: [f64; 3],
    pub rotation_b: [f64; 3],
    pub offset_b: [f64; 3],
}

/// Which side a model mounts on: the requested side if valid, otherwise the
/// footprint side (or `default_side` when the footprint is reversible).
pub(crate) fn model_side<'a>(
    p: &'a RenderContext<'_>,
    requested: &'a str,
    default_side: &'a str,
) -> &'a str {
    let side = if requested.is_empty() {
        if p.flag("reverse") {
            default_side
        } else {
            p.side()
        }
    } else {
        requested
    };
    if side == "F" || side == "B" {
        side
    } else {
        default_side
    }
}

/// A `(model ...)` block as the infused-kim generators write it: `(at ...)`
/// offsets in KiCad 5 units (divided by 25.4), and a side that defaults to the
/// footprint side unless the footprint is reversible.
pub(crate) fn infused_model(
    p: &RenderContext<'_>,
    names: ModelParams<'_>,
    defaults: ModelDefaults,
) -> Result<String> {
    let filename = p.text(names.filename);
    if filename.is_empty() {
        return Ok(String::new());
    }
    let front = model_side(p, p.text(names.side), defaults.default_side) == "F";
    let scale = p.vec3(names.scale)?.unwrap_or([1.0, 1.0, 1.0]);
    let rotation = p.vec3(names.rotation)?.unwrap_or(if front {
        defaults.rotation_f
    } else {
        defaults.rotation_b
    });
    let offset = p.vec3(names.offset)?.unwrap_or(if front {
        defaults.offset_f
    } else {
        defaults.offset_b
    });
    let offset = offset.map(|value| value / 25.4);
    Ok(format!(
        "\n  (model {}\n    (at (xyz {}))\n    (scale (xyz {}))\n    (rotate (xyz {}))\n  )\n",
        json_string(filename),
        xyz(offset),
        xyz(scale),
        xyz(rotation),
    ))
}
