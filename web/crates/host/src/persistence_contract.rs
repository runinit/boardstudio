use sha2::{Digest, Sha256};

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub fn validate_exact_json_integers(value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Number(number) => {
            if number
                .as_i64()
                .is_some_and(|number| number < -(MAX_SAFE_INTEGER as i64))
                || number
                    .as_u64()
                    .is_some_and(|number| number > MAX_SAFE_INTEGER)
            {
                return Err(
                    "document contains an integer outside JavaScript's exact Number range".into(),
                );
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                validate_exact_json_integers(value)?;
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                validate_exact_json_integers(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_SAFE_INTEGER, sha256_bytes, validate_exact_json_integers};
    use serde_json::json;

    #[test]
    fn allows_safe_json_integer_values() {
        assert!(
            validate_exact_json_integers(&json!({
                "positive": MAX_SAFE_INTEGER,
                "negative": -9_007_199_254_740_991_i64,
                "float": 0.125,
            }))
            .is_ok()
        );
    }

    #[test]
    fn rejects_unsafe_json_integer_values_before_indexeddb_serialization() {
        assert!(
            validate_exact_json_integers(&json!({
                "revision": 9_007_199_254_740_992_u64
            }))
            .is_err()
        );
        assert!(
            validate_exact_json_integers(&json!([
                {"nested": -9_007_199_254_740_992_i64}
            ]))
            .is_err()
        );
    }

    #[test]
    fn asset_sha256_matches_standard_digest() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
