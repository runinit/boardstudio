//! Recover numeric spellings with the existing Part21 token/instance parsers.
//! This is a strict integer authority for points, directions and vectors, not a STEP parser.
use super::*;

pub(super) fn integers(bytes: &[u8]) -> Option<HashMap<u64, Vec<f64>>> {
    if !bytes.is_ascii() {
        return None;
    }
    let mut input = std::str::from_utf8(bytes).ok()?;
    let mut result = HashMap::new();
    while !input.is_empty() {
        if let Ok((rest, _)) = parser::combinator::separator(input) {
            input = rest;
        } else if input.starts_with('\'') {
            input = parser::token::string(input).ok()?.0;
        } else if input.starts_with('#') {
            let (rest, instance) = parser::exchange::entity_instance(input).ok()?;
            if let EntityInstance::Simple { id, record } = instance
                && matches!(
                    record.name.as_str(),
                    "CARTESIAN_POINT" | "DIRECTION" | "VECTOR"
                )
            {
                let values = geometry_numbers(&input[..input.len() - rest.len()])?;
                if result.insert(id, values).is_some() {
                    return None;
                }
            }
            input = rest;
        } else if input.starts_with('"') {
            return None;
        } else {
            input = &input[1..];
        }
    }
    Some(result)
}

fn geometry_numbers(mut input: &str) -> Option<Vec<f64>> {
    let mut result = Vec::new();
    while !input.is_empty() {
        if let Ok((rest, _)) = parser::combinator::separator(input) {
            input = rest;
        } else if input.starts_with('\'') {
            input = parser::token::string(input).ok()?.0;
        } else if input.starts_with('#') {
            input = parser::token::entity_instance_name(input).ok()?.0;
        } else if input.as_bytes()[0].is_ascii_uppercase() {
            input = parser::token::standard_keyword(input).ok()?.0;
        } else if matches!(input.as_bytes()[0], b'+' | b'-' | b'0'..=b'9') {
            let (rest, parsed) = parser::token::real(input)
                .or_else(|_| {
                    parser::token::integer(input).map(|(rest, value)| (rest, value as f64))
                })
                .ok()?;
            let literal = &input[..input.len() - rest.len()];
            let (whole, fraction) = literal.split_once('.').unwrap_or((literal, ""));
            if !fraction.bytes().all(|b| b == b'0') {
                return None;
            }
            let integer = whole.parse::<i64>().ok()?;
            // Exact binary64 integers and bounded differences/products in this class.
            if !(-(1 << 20)..=(1 << 20)).contains(&integer) || parsed != integer as f64 {
                return None;
            }
            result.push(integer as f64);
            input = rest;
        } else if matches!(input.as_bytes()[0], b'(' | b')' | b',' | b'=' | b';') {
            input = &input[1..];
        } else {
            return None;
        }
    }
    Some(result)
}
