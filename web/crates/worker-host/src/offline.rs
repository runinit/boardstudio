#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineManifest {
    pub version: String,
    pub shell_assets: Vec<String>,
    pub runtime_cache_prefixes: Vec<String>,
}

impl OfflineManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.version.is_empty()
            || !self
                .version
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err("offline manifest version must contain only letters, digits, dot, dash or underscore".into());
        }
        if !self.shell_assets.iter().any(|asset| asset == "index.html") {
            return Err("offline manifest must include the application shell index.html".into());
        }
        let mut unique = std::collections::BTreeSet::new();
        for prefix in &self.runtime_cache_prefixes {
            if !prefix.ends_with('/') || !unique.insert(prefix) {
                return Err(format!(
                    "invalid or duplicate runtime cache prefix: {prefix}"
                ));
            }
            validate_asset_path(prefix.trim_end_matches('/'))?;
        }
        unique.clear();
        for asset in &self.shell_assets {
            validate_asset_path(asset)?;
            if !unique.insert(asset) || self.is_runtime_cacheable(asset) {
                return Err(format!("duplicate or runtime-only shell asset: {asset}"));
            }
        }
        Ok(())
    }

    pub fn is_runtime_cacheable(&self, relative_path: &str) -> bool {
        validate_asset_path(relative_path).is_ok()
            && self
                .runtime_cache_prefixes
                .iter()
                .any(|prefix| relative_path.starts_with(prefix))
    }
}

fn validate_asset_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path
            .chars()
            .any(|character| matches!(character, '\\' | '?' | '#' | ':' | '%'))
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(format!(
            "offline asset path must be a normalized scope-relative path: {path}"
        ));
    }
    Ok(())
}

pub fn scope_cache_prefix(scope_url: &str) -> Result<String, String> {
    let (_, authority_and_path) = scope_url
        .split_once("://")
        .ok_or_else(|| "service worker registration scope must be an absolute URL".to_owned())?;
    let path_start = authority_and_path
        .find('/')
        .ok_or_else(|| "service worker scope URL has no path".to_owned())?;
    let scope_path = &authority_and_path[path_start..];
    if !scope_path.starts_with('/') || !scope_path.ends_with('/') {
        return Err("service worker scope URL must have a slash-terminated path".into());
    }
    let mut encoded = String::with_capacity(scope_path.len() * 2);
    for byte in scope_path.bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(format!("boardstudio-m1-offline-path-{encoded}-"))
}

#[cfg(test)]
mod tests {
    use super::{OfflineManifest, scope_cache_prefix};

    #[test]
    fn path_hex_encoding_separates_root_and_subpath_cache_names() {
        let root = scope_cache_prefix("https://example.test/").expect("root scope");
        let subpath = scope_cache_prefix("https://example.test/boardstudio/").expect("subpath");
        assert_ne!(root, subpath);
        assert_eq!(root, "boardstudio-m1-offline-path-2f-");
        assert_ne!(
            scope_cache_prefix("https://example.test/a/b/").unwrap(),
            scope_cache_prefix("https://example.test/a_b/").unwrap()
        );
        assert!(scope_cache_prefix("/boardstudio/").is_err());
        assert!(scope_cache_prefix("https://example.test/boardstudio").is_err());
    }

    #[test]
    fn validates_complete_normalized_manifest() {
        let valid = OfflineManifest {
            version: "20261001-a1".into(),
            shell_assets: vec!["index.html".into(), "assets/app.js".into()],
            runtime_cache_prefixes: vec!["assets/modules/".into()],
        };
        assert!(valid.validate().is_ok());
        assert!(valid.is_runtime_cacheable("assets/modules/keyboard.json"));
        for path in [
            "/root",
            "../escape",
            "assets/../escape",
            "app.js?x",
            "assets/%2e%2e/escape",
            "https://host/a",
            "assets/modules/a?x",
            "assets/modules/../escape",
        ] {
            let mut manifest = valid.clone();
            manifest.shell_assets.push(path.into());
            assert!(manifest.validate().is_err(), "accepted {path}");
            assert!(!valid.is_runtime_cacheable(path));
        }
        for path in ["assets/modules/a", "index.html"] {
            let mut duplicate_or_runtime = valid.clone();
            duplicate_or_runtime.shell_assets.push(path.into());
            assert!(duplicate_or_runtime.validate().is_err());
        }
        for prefix in ["", "/", "assets", "assets/../", "assets/modules/"] {
            let mut invalid = valid.clone();
            invalid.runtime_cache_prefixes.push(prefix.into());
            assert!(invalid.validate().is_err(), "accepted {prefix}");
        }
        assert!(!valid.is_runtime_cacheable("assets/modules-index.json"));
        for manifest in [
            OfflineManifest {
                version: String::new(),
                ..valid.clone()
            },
            OfflineManifest {
                version: "v1/slash".into(),
                ..valid.clone()
            },
            OfflineManifest {
                shell_assets: vec![],
                ..valid.clone()
            },
            OfflineManifest {
                shell_assets: vec!["assets/app.js".into()],
                ..valid
            },
        ] {
            assert!(manifest.validate().is_err(), "accepted {manifest:?}");
        }
    }
}
