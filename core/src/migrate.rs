//! Strict document admission. Format 7 has no conversion from older schemas.
use serde_json::Value;

/// The version this build reads and writes.
pub const CURRENT_VERSION: u32 = 7;
/// The version of a document that does not state one.
pub const LEGACY_VERSION: u32 = 1;

/// Typed Core admission never migrates or modifies the candidate document.
pub(crate) fn require_current_version(version: u32) -> Result<(), String> {
    if version == 0 {
        return Err("Project data failed validation: invalid formatVersion 0".into());
    }
    if version < CURRENT_VERSION {
        return Err(format!(
            "This project uses format {version}. This build requires format {CURRENT_VERSION}. No conversion is available. Your saved data has not been changed."
        ));
    }
    if version > CURRENT_VERSION {
        return Err(format!(
            "This project requires format {version}. This build supports format {CURRENT_VERSION}. Open it with a compatible newer build."
        ));
    }
    Ok(())
}

/// Admit a current document without modifying any saved data. The bool remains
/// false: callers must never write a normalized version back during list/load.
pub fn migrate_value(document: Value) -> Result<(Value, bool), String> {
    require_current_value(&document)?;
    Ok((document, false))
}

/// Inspect the version before schema-dependent deserialization or replacement effects.
pub fn require_current_value(document: &Value) -> Result<(), String> {
    let object = document
        .as_object()
        .ok_or("Project data failed validation: document must be an object")?;
    if object.get("format").and_then(Value::as_str) != Some("boardstudio/v2") {
        return Err("Unsupported document format".into());
    }
    let value = object.get("formatVersion").ok_or_else(|| format!("This legacy project has no formatVersion. This build requires format {CURRENT_VERSION}. No conversion is available. Your saved data has not been changed."))?;
    let version = value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("Project data failed validation: invalid formatVersion")?;
    require_current_version(version)
}

/// [`migrate_value`] for JSON text. Text already at the current version is returned unchanged.
pub fn migrate_json(text: &str) -> Result<(String, bool), String> {
    let value: Value =
        serde_json::from_str(text).map_err(|error| format!("Invalid project JSON: {error}"))?;
    require_current_value(&value)?;
    Ok((text.to_owned(), false))
}
