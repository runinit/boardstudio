//! Version admission for canonical project documents.

pub const CURRENT_VERSION: u32 = 8;

pub fn require_current_version(version: u32) -> Result<(), String> {
    if version == CURRENT_VERSION {
        Ok(())
    } else {
        Err("Unsupported formatVersion: expected 8".into())
    }
}

pub fn require_current_value(document: &serde_json::Value) -> Result<(), String> {
    if document["formatVersion"].as_u64() != Some(u64::from(CURRENT_VERSION)) {
        return Err("Unsupported formatVersion: expected 8".into());
    }
    if document["format"].as_str() != Some("boardstudio/v2") {
        return Err("Unsupported document format".into());
    }
    Ok(())
}
