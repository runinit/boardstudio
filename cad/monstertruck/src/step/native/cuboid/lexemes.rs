//! Integer authority for the narrow cuboid proof, borrowed from original spellings.
use super::{EntityInstance, parts};
use crate::step::repair::spelling;

pub(super) struct Scalars {
    values: [f64; 3],
    len: usize,
}
impl Scalars {
    pub(super) fn as_slice(&self) -> &[f64] {
        &self.values[..self.len]
    }
}

/// Every DATA spelling binds to exactly one parsed record of the same kind and
/// every parsed record has exactly one spelling. `records` is sorted by ID.
pub(super) fn validate<'a>(bytes: &'a [u8], records: &[&EntityInstance]) -> Option<&'a [u8]> {
    if !bytes.is_ascii() {
        return None;
    }
    let mut seen = Vec::new();
    seen.try_reserve_exact(records.len()).ok()?;
    seen.resize(records.len(), false);
    let data = spelling::data_body(bytes)?;
    for instance in spelling::data_instances(Some(data)) {
        let instance = instance?;
        let index = records
            .binary_search_by_key(&instance.id, |record| parts(record).0)
            .ok()?;
        let kind = match records[index] {
            EntityInstance::Simple { record, .. } => record.name.as_bytes(),
            EntityInstance::Complex { .. } => b"",
        };
        if std::mem::replace(&mut seen[index], true) || instance.kind != kind {
            return None;
        }
        if matches!(instance.kind, b"CARTESIAN_POINT" | b"DIRECTION" | b"VECTOR") {
            geometry_numbers(instance.parameters)?;
        }
    }
    seen.iter().all(|seen| *seen).then_some(data)
}

pub(super) fn values(data: &[u8], id: u64) -> Option<Scalars> {
    // Document bound every DATA spelling one-to-one to its parsed record before
    // these lookups; retain no per-entity numeric map or label copies.
    for instance in spelling::data_instances(Some(data)) {
        let instance = instance?;
        if instance.id == id {
            return geometry_numbers(instance.parameters);
        }
    }
    None
}

fn geometry_numbers(mut input: &[u8]) -> Option<Scalars> {
    let mut result = Scalars {
        values: [0.0; 3],
        len: 0,
    };
    while !input.is_empty() {
        input = spelling::separators(input)?;
        if input.is_empty() {
            break;
        }
        if input.starts_with(b"'") {
            input = spelling::string(input)?;
        } else if input.starts_with(b"#") {
            input = spelling::reference(input)?.0;
        } else if matches!(input[0], b'+' | b'-' | b'0'..=b'9') {
            let (rest, literal) = spelling::number(input)?;
            let (whole, fraction) = literal.split_once('.').unwrap_or((literal, ""));
            if !fraction.bytes().all(|b| b == b'0') {
                return None;
            }
            let integer = whole.parse::<i64>().ok()?;
            if !(-(1 << 20)..=(1 << 20)).contains(&integer) {
                return None;
            }
            *result.values.get_mut(result.len)? = integer as f64;
            result.len += 1;
            input = rest;
        } else if matches!(input[0], b'(' | b')' | b',') {
            input = &input[1..];
        } else {
            return None;
        }
    }
    Some(result)
}
