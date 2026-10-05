//! `ceoloide/utility_router`: tracks and vias from a compact route string.
//!
//! Ported from `utility_router.js` of ceoloide/ergogen-footprints.
//! Copyright (c) 2025 Marco Massarelli
//! SPDX-License-Identifier: MIT
//! Author: @yanshay + @ceoloide improvements
//!
//! Routes are strings of `f`/`b` (set the layer), `(x,y)` positions, `v` (via and
//! switch layer) and `x` or `|` (lift the pen). Global nets (`<name>`) were never
//! supported. A position that is not valid JSON is an error; the source's own
//! "not a number" check could never fire, so unparsable numbers print as `NaN`.
use serde_json::Value;

use crate::Result;
use crate::context::RenderContext;
use crate::error::GeneratorError;
use crate::generators::util::yes_no;
use crate::number::{js_number as n, js_to_number};
use crate::params::ParamSpec;
use crate::registry::{GeneratorSpec, License};
use crate::types::PartKind;

static PARAMS: [ParamSpec; 8] = [
    ParamSpec::SIDE,
    ParamSpec::net_default("net", ""),
    ParamSpec::number("width", 0.25),
    ParamSpec::number("via_size", 0.6),
    ParamSpec::number("via_drill", 0.3),
    ParamSpec::boolean("locked", false),
    ParamSpec::array("routes", "[]"),
    ParamSpec::string("route", ""),
];

pub static SPEC: GeneratorSpec = GeneratorSpec {
    source: "ceoloide/utility_router",
    display_name: "utility router",
    kind: PartKind::Utility,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "@yanshay + @ceoloide improvements",
    },
    body,
};

type Position = [f64; 2];

fn rejected(message: impl Into<String>) -> GeneratorError {
    GeneratorError::rejected(SPEC.source, None, message)
}

/// `(x y)`, `(x,y)` or `( x , y )` to numbers; missing numbers are NaN.
fn parse_position(text: &str) -> Result<Position> {
    let spaced = text.replace(' ', ",");
    let mut collapsed = String::new();
    for c in spaced.chars() {
        if !(c == ',' && collapsed.ends_with(',')) {
            collapsed.push(c);
        }
    }
    let chars: Vec<char> = collapsed.chars().collect();
    let mut json = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' => {
                json.push('[');
                while chars.get(i + 1) == Some(&',') {
                    i += 1;
                }
            }
            ',' if chars.get(i + 1) == Some(&')') => {}
            ')' => json.push(']'),
            c => json.push(c),
        }
        i += 1;
    }
    let value: Value = serde_json::from_str(&json)
        .map_err(|_| rejected(format!("Invalid position encountered: {json}")))?;
    let Value::Array(items) = value else {
        return Err(rejected(format!("Invalid position encountered: {json}")));
    };
    let number = |index: usize| match items.get(index) {
        Some(Value::Number(value)) => value.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(text)) => js_to_number(text),
        Some(Value::Bool(flag)) => f64::from(u8::from(*flag)),
        Some(Value::Null) => 0.0,
        _ => f64::NAN,
    };
    Ok([number(0), number(1)])
}

fn traces(p: &RenderContext<'_>, route: &str, net: u32) -> Result<String> {
    let chars: Vec<char> = route.chars().collect();
    let locked = yes_no(p.flag("locked"));
    let mut out = String::new();
    let mut layer: Option<&str> = None;
    let mut start: Option<Position> = None;
    let mut i = 0;
    while i < chars.len() {
        match chars[i].to_lowercase().to_string().as_str() {
            "f" => layer = Some("F.Cu"),
            "b" => layer = Some("B.Cu"),
            "v" => {
                let Some(position) = start else {
                    return Err(rejected(
                        "Can't place via when position is not set, use (x,y) to set position",
                    ));
                };
                out.push_str(&format!(
                    "\n  (via\n    (at {})\n    (size {})\n    (drill {})\n    (layers \"F.Cu\" \"B.Cu\")\n    (locked {locked})\n    (net {net})\n  )\n",
                    p.eaxy(position[0], position[1]),
                    n(p.number("via_size")),
                    n(p.number("via_drill")),
                ));
                layer = match layer {
                    Some("F.Cu") => Some("B.Cu"),
                    Some("B.Cu") => Some("F.Cu"),
                    other => other,
                };
            }
            "(" => {
                let mut tuple = String::from("(");
                i += 1;
                while i < chars.len() {
                    tuple.push(chars[i]);
                    if chars[i] == ')' {
                        break;
                    }
                    i += 1;
                }
                let position = parse_position(&tuple)?;
                if let Some(from) = start {
                    let Some(layer) = layer else {
                        return Err(rejected(
                            "Can't place segment before layer is set, use 'f' or 'b', to set starting layer",
                        ));
                    };
                    out.push_str(&format!(
                        "\n  (segment\n    (start {})\n    (end {})\n    (width {})\n    (locked {locked})\n    (layer {layer})\n    (net {net})\n  )\n",
                        p.eaxy(from[0], from[1]),
                        p.eaxy(position[0], position[1]),
                        n(p.number("width")),
                    ));
                }
                start = Some(position);
            }
            "<" => {
                return Err(rejected(format!(
                    "Global nets are not yet supported (character position {i}). See https://github.com/ergogen/ergogen/pull/109"
                )));
            }
            "x" | "|" => start = None,
            " " => {}
            other => {
                return Err(rejected(format!(
                    "Unsupported character '{other}' at position {i}."
                )));
            }
        }
        i += 1;
    }
    Ok(out)
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    let net = p.net("net").index;
    let mut combined = String::new();
    let route = p.text("route");
    if !route.is_empty() {
        combined.push_str(&traces(p, route, net)?);
    }
    for route in p.list("routes") {
        let Value::String(route) = route else {
            return Err(GeneratorError::InvalidParameter {
                generator: SPEC.source.to_owned(),
                parameter: "routes".to_owned(),
                expected: "a list of text",
            });
        };
        combined.push_str(&traces(p, route, net)?);
    }
    Ok(combined)
}
