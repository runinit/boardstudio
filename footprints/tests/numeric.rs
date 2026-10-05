//! JavaScript number semantics against vectors recorded from V8.
mod common;

use boardstudio_footprints::number::*;
use common::read_json;

#[test]
fn number_to_string_matches_v8() {
    let vectors = read_json("numeric_vectors.json");
    for pair in vectors["format"].as_array().unwrap() {
        let value = pair[0].as_f64().unwrap();
        assert_eq!(js_number(value), pair[1].as_str().unwrap(), "{value:e}");
    }
}

#[test]
fn string_to_number_matches_v8() {
    let vectors = read_json("numeric_vectors.json");
    for entry in vectors["toNumber"].as_array().unwrap() {
        let text = entry["text"].as_str().unwrap();
        let actual = js_to_number(text);
        match &entry["value"] {
            serde_json::Value::String(name) if name == "NaN" => {
                assert!(actual.is_nan(), "{text:?} -> {actual}")
            }
            serde_json::Value::String(name) if name == "-0" => {
                assert!(actual == 0.0 && actual.is_sign_negative(), "{text:?}")
            }
            serde_json::Value::String(name) if name == "Infinity" => {
                assert_eq!(actual, f64::INFINITY, "{text:?}")
            }
            serde_json::Value::String(name) if name == "-Infinity" => {
                assert_eq!(actual, f64::NEG_INFINITY, "{text:?}")
            }
            expected => assert_eq!(actual, expected.as_f64().unwrap(), "{text:?}"),
        }
    }
}

#[test]
fn trigonometry_is_bit_identical_to_v8() {
    let vectors = read_json("numeric_vectors.json");
    for entry in vectors["trig"].as_array().unwrap() {
        let degrees = entry["degrees"].as_f64().unwrap();
        let angle = radians(degrees);
        assert_eq!(
            cos(angle).to_bits(),
            entry["cos"].as_f64().unwrap().to_bits(),
            "cos {degrees}"
        );
        assert_eq!(
            sin(angle).to_bits(),
            entry["sin"].as_f64().unwrap().to_bits(),
            "sin {degrees}"
        );
        assert_eq!(
            js_number(cos(angle)),
            entry["cosText"].as_str().unwrap(),
            "cos text {degrees}"
        );
        assert_eq!(
            js_number(sin(angle)),
            entry["sinText"].as_str().unwrap(),
            "sin text {degrees}"
        );
    }
}

#[test]
fn hypot_matches_v8() {
    let vectors = read_json("numeric_vectors.json");
    for entry in vectors["hypot"].as_array().unwrap() {
        let (x, y, expected) = (
            entry[0].as_f64().unwrap(),
            entry[1].as_f64().unwrap(),
            entry[2].as_f64().unwrap(),
        );
        assert_eq!(hypot(x, y).to_bits(), expected.to_bits(), "hypot({x}, {y})");
    }
}

#[test]
fn uri_component_encoding_matches_v8() {
    let vectors = read_json("numeric_vectors.json");
    for pair in vectors["encodeURIComponent"].as_array().unwrap() {
        let text = pair[0].as_str().unwrap();
        let encoded = encode_uri_component(text);
        assert_eq!(encoded, pair[1].as_str().unwrap(), "{text:?}");
        assert_eq!(decode_uri_component(&encoded).as_deref(), Some(text));
    }
    assert_eq!(decode_uri_component("%E0%A4%A"), None);
    assert_eq!(decode_uri_component("%FF"), None);
}
