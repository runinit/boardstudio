/// Storage startup protocol; unknown workers and extra windows defer collection.
pub const STORAGE_WINDOW_QUERY: &str = "boardstudio-storage-windows-v1";
pub const STORAGE_WINDOW_ALONE: &str = "boardstudio-storage-alone-v1";
/// Ticket 019 adoption protocol: posted to the waiting worker, with a reply
/// port, to ask it to take over now. The worker checks its own live window
/// clients at that moment: it answers `ADOPT_DECLINED` when any other editor is
/// open, and otherwise calls `skipWaiting` and answers `ADOPT_ACCEPTED`. The
/// page reloads only after its controller changes and refresh safety is
/// rechecked.
pub const ADOPT_WAITING_MESSAGE: &str = "boardstudio-adopt-waiting-v1";
pub const ADOPT_ACCEPTED: &str = "boardstudio-adopt-accepted-v1";
pub const ADOPT_DECLINED: &str = "boardstudio-adopt-declined-v1";

/// Why a waiting build could not be adopted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdoptError {
    /// The worker found another open editor and declined. Nothing changed.
    OtherTabs,
    /// The worker never answered or never took over.
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineManifest {
    pub version: String,
    pub assets: Vec<String>,
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
        if self.assets.is_empty() {
            return Err("offline manifest must contain at least one required asset".into());
        }
        if !self.assets.iter().any(|asset| asset == "index.html") {
            return Err("offline manifest must include the application shell index.html".into());
        }
        let mut unique = std::collections::BTreeSet::new();
        for asset in &self.assets {
            validate_asset_path(asset)?;
            if !unique.insert(asset) {
                return Err(format!(
                    "offline manifest contains duplicate asset path: {asset}"
                ));
            }
        }
        Ok(())
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

/// A point-in-time view of the release lifecycle for one service worker registration.
///
/// `waiting` is set when a newly installed worker is waiting to activate,
/// `installing` while a worker is still installing, and `controlled` when this
/// page is already controlled by an active worker (so a waiting worker is a
/// newer build, not the initial install).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateSnapshot {
    pub waiting: bool,
    pub installing: bool,
    pub controlled: bool,
}

/// Whether the snapshot describes a newer build waiting while this page runs
/// an older one. A waiting worker with no controlling worker is the initial
/// install, not an update; an installing, failed or unknown worker never counts.
/// Ticket 018 shows no action and no header notice for this state; the Refresh
/// action arrives with 019.
pub fn is_update_waiting(snapshot: UpdateSnapshot) -> bool {
    snapshot.waiting && snapshot.controlled
}

#[cfg(test)]
mod tests {
    use super::{OfflineManifest, UpdateSnapshot, is_update_waiting, scope_cache_prefix};

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
            assets: vec!["index.html".into(), "assets/app.js".into()],
        };
        assert!(valid.validate().is_ok());
        for invalid in [
            "/root",
            "../escape",
            "assets/../escape",
            "app.js?x",
            "assets/%2e%2e/escape",
            "https://host/a",
        ] {
            let manifest = OfflineManifest {
                version: "v1".into(),
                assets: vec!["index.html".into(), invalid.into()],
            };
            assert!(manifest.validate().is_err(), "accepted {invalid}");
        }
        for manifest in [
            OfflineManifest {
                version: String::new(),
                assets: vec!["index.html".into()],
            },
            OfflineManifest {
                version: "v1".into(),
                assets: Vec::new(),
            },
            OfflineManifest {
                version: "v1".into(),
                assets: vec!["assets/app.js".into()],
            },
            OfflineManifest {
                version: "v1/slash".into(),
                assets: vec!["index.html".into()],
            },
        ] {
            assert!(
                manifest.validate().is_err(),
                "accepted invalid manifest {manifest:?}"
            );
        }
    }

    #[test]
    fn only_a_waiting_worker_under_a_controlled_page_is_an_update() {
        assert!(is_update_waiting(UpdateSnapshot {
            waiting: true,
            installing: false,
            controlled: true,
        }));
        for snapshot in [
            UpdateSnapshot {
                waiting: true,
                installing: false,
                controlled: false,
            },
            UpdateSnapshot {
                waiting: false,
                installing: true,
                controlled: true,
            },
            UpdateSnapshot {
                waiting: false,
                installing: false,
                controlled: true,
            },
            UpdateSnapshot {
                waiting: false,
                installing: false,
                controlled: false,
            },
        ] {
            assert!(
                !is_update_waiting(snapshot),
                "treated {snapshot:?} as an update"
            );
        }
    }
}
