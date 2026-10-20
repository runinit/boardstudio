//! Asset identity: one lowercase hex SHA-256 shared by every stored, archived
//! or delivered asset.

use sha2::{Digest, Sha256};

/// Lowercase hexadecimal SHA-256 of `bytes`.
///
/// Every stored, archived or delivered asset is identified by this exact text,
/// so callers compare it without normalizing case.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_matches_the_known_empty_input_vector() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
