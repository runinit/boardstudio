//! Minimal shared browser harness for the real-page checks.
//!
//! One deep module per consumer need: Chrome discovery, isolated launch, role
//! lookup, bounded waits, trusted input, evaluation and fatal error capture.
//! Helpers exist only because a current check script uses them.

pub mod driver;

/// A fatal harness or page failure. Messages are actionable: they name the
/// check, the missing prerequisite, or the failed condition.
#[derive(Debug)]
pub struct Error(pub String);

impl Error {
    /// A fatal harness or page failure from any message.
    pub fn new(message: impl Into<String>) -> Self {
        Error(message.into())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// A fallible harness result.
pub type Result<T> = std::result::Result<T, Error>;

/// Canonical JSON: objects with recursively sorted keys, like the old
/// runner's `stable()` helper. Document signatures compare on this form, so
/// the Rust side and the page's own canonicalization must agree byte for byte.
pub fn canonical(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = serde_json::Map::with_capacity(map.len());
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted.insert(key.clone(), canonical(&map[key]));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonical).collect())
        }
        other => other.clone(),
    }
}

/// Locate the Chrome executable the harness must launch.
///
/// `CHROME_BIN` wins when set; otherwise the usual executable names are probed
/// in the same order as `scripts/browser_tools.py`. Errors are actionable: they
/// name the variable the operator must set. No personal absolute pins live here.
pub fn discover_chrome(
    get_env: &dyn Fn(&str) -> Option<String>,
    is_executable: &dyn Fn(&str) -> bool,
) -> std::result::Result<String, String> {
    if let Some(pinned) = get_env("CHROME_BIN") {
        if is_executable(&pinned) {
            return Ok(pinned);
        }
        return Err(format!(
            "drag browser smoke needs Chrome or Chromium; CHROME_BIN points at '{pinned}', \
             which is not an executable file"
        ));
    }
    for name in ["google-chrome", "chromium", "chromium-browser"] {
        if is_executable(name) {
            return Ok(name.to_owned());
        }
    }
    Err("drag browser smoke needs Chrome or Chromium; set CHROME_BIN to its executable".to_owned())
}

/// Process-owned writable directories for one browser run.
///
/// The profile lives under a shallow `TMPDIR` (deep paths exceed Unix socket
/// limits); the child also gets a process-local `XDG_CONFIG_HOME` so a broken
/// user config can never crash the launch. Dropping removes the profile after confirmed termination; two
/// runs never share IndexedDB state.
pub struct IsolatedDirs {
    /// Directory passed as the browser `--user-data-dir`.
    pub profile_dir: std::path::PathBuf,
    config_dir: std::path::PathBuf,
    run_dir: std::path::PathBuf,
    preserve: std::cell::Cell<bool>,
    identity: std::fs::Metadata,
}

fn keep_with_identity(
    temp: tempfile::TempDir,
    metadata: impl FnOnce(&std::path::Path) -> std::io::Result<std::fs::Metadata>,
) -> std::result::Result<(std::path::PathBuf, std::fs::Metadata), String> {
    let identity = match metadata(temp.path()) {
        Ok(identity) => identity,
        Err(error) => {
            let message = format!("cannot record browser directory identity: {error}");
            return Err(match temp.close() {
                Ok(()) => message,
                Err(cleanup) => {
                    format!("{message}; cannot remove temporary browser directory: {cleanup}")
                }
            });
        }
    };
    Ok((temp.keep(), identity))
}

impl IsolatedDirs {
    /// Create a process-owned run directory with a profile and a child
    /// config dir inside `tmp_root`. `tmp_root` must be shallow: deep paths
    /// exceed the browser's Unix socket limits.
    pub fn create(tmp_root: &std::path::Path) -> std::result::Result<Self, String> {
        let temp = tempfile::Builder::new()
            .prefix("bs-drag-")
            .tempdir_in(tmp_root)
            .map_err(|error| {
                format!(
                    "drag browser smoke needs a writable profile under '{}': {error}",
                    tmp_root.display()
                )
            })?;
        let (run_dir, identity) = keep_with_identity(temp, |path| std::fs::symlink_metadata(path))?;
        let dirs = Self {
            profile_dir: run_dir.join("profile"),
            config_dir: run_dir.join("config"),
            run_dir,
            preserve: std::cell::Cell::new(false),
            identity,
        };
        for dir in [&dirs.profile_dir, &dirs.config_dir] {
            if let Err(error) = std::fs::create_dir(dir) {
                let message = format!(
                    "drag browser smoke needs a writable profile under '{}': {error}",
                    tmp_root.display()
                );
                return Err(match dirs.cleanup_after_termination() {
                    Ok(()) => message,
                    Err(cleanup) => {
                        format!("{message}; cannot remove partial browser directory: {cleanup}")
                    }
                });
            }
        }
        Ok(dirs)
    }

    /// Remove the owned directory after the caller has confirmed child termination.
    /// On non-Unix platforms identity cannot currently be verified, so cleanup
    /// returns an error and preserves the directory.
    pub fn cleanup_after_termination(&self) -> Result<()> {
        self.preserve.set(true);
        self.remove_owned()
    }

    fn remove_owned(&self) -> Result<()> {
        if self.profile_dir != self.run_dir.join("profile")
            || self.config_dir != self.run_dir.join("config")
        {
            return Err(Error(
                "browser directory paths do not match recorded ownership".into(),
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let current = std::fs::symlink_metadata(&self.run_dir).map_err(|error| {
                Error(format!(
                    "cannot inspect browser directory {}: {error}",
                    self.run_dir.display()
                ))
            })?;
            if !current.is_dir()
                || current.dev() != self.identity.dev()
                || current.ino() != self.identity.ino()
            {
                return Err(Error(format!(
                    "browser directory ownership changed: {}",
                    self.run_dir.display()
                )));
            }
            std::fs::remove_dir_all(&self.run_dir).map_err(|error| {
                Error(format!(
                    "cannot remove browser directory {}: {error}",
                    self.run_dir.display()
                ))
            })
        }
        #[cfg(not(unix))]
        {
            Err(Error(
                "browser directory identity cannot be verified on this platform".into(),
            ))
        }
    }

    /// Override for the browser child's `XDG_CONFIG_HOME`.
    pub fn xdg_config_home(&self) -> &std::path::Path {
        &self.config_dir
    }
}

impl Drop for IsolatedDirs {
    fn drop(&mut self) {
        // The whole run directory is process-owned: remove profile and child
        // config together so no run ever leaks state.
        if !self.preserve.get() {
            self.remove_owned().ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> = vars
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |key: &str| {
            owned
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn chrome_bin_is_honoured_first() {
        let get_env = env_with(&[("CHROME_BIN", "/opt/chrome")]);
        let is_executable = |_: &str| true;
        assert_eq!(
            discover_chrome(&get_env, &is_executable),
            Ok("/opt/chrome".to_owned())
        );
    }

    #[test]
    fn probe_order_matches_browser_tools() {
        let get_env = env_with(&[]);
        let is_executable = |name: &str| matches!(name, "chromium" | "chromium-browser");
        assert_eq!(
            discover_chrome(&get_env, &is_executable),
            Ok("chromium".to_owned())
        );
    }

    #[test]
    fn missing_chrome_names_chrome_bin() {
        let get_env = env_with(&[]);
        let is_executable = |_: &str| false;
        let error = discover_chrome(&get_env, &is_executable).unwrap_err();
        assert!(
            error.contains("CHROME_BIN"),
            "actionable error names CHROME_BIN, got: {error}"
        );
    }

    #[test]
    fn identity_failure_removes_temporary_directory() {
        let root = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir_in(root.path()).unwrap();
        let path = temp.path().to_owned();
        let error =
            super::keep_with_identity(temp, |_| Err(std::io::Error::other("identity failure")))
                .unwrap_err();
        assert!(error.contains("identity failure"));
        assert!(!path.exists());
    }

    #[test]
    fn isolated_dirs_own_profile_and_config() {
        let root = std::env::temp_dir().join(format!("bs-harness-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let dirs = IsolatedDirs::create(&root).unwrap();
        assert!(dirs.profile_dir.is_dir(), "profile dir exists");
        assert!(dirs.xdg_config_home().is_dir(), "child config dir exists");
        assert_ne!(dirs.profile_dir, dirs.xdg_config_home());
        assert!(dirs.profile_dir.starts_with(&root));
        assert!(dirs.xdg_config_home().starts_with(&root));
    }

    #[test]
    fn isolated_dirs_do_not_adopt_abandoned_run() {
        let root = std::env::temp_dir().join(format!("bs-collision-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let abandoned = root.join(format!("bs-drag-{}-0", std::process::id()));
        std::fs::create_dir(&abandoned).unwrap();
        std::fs::write(abandoned.join("marker"), b"keep").unwrap();
        let dirs = IsolatedDirs::create(&root).unwrap();
        assert_ne!(dirs.run_dir, abandoned);
        drop(dirs);
        assert_eq!(std::fs::read(abandoned.join("marker")).unwrap(), b"keep");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn isolated_dirs_do_not_share_state() {
        let root = std::env::temp_dir().join(format!("bs-harness-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let first = IsolatedDirs::create(&root).unwrap();
        let second = IsolatedDirs::create(&root).unwrap();
        assert_ne!(first.profile_dir, second.profile_dir);
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_keeps_other_run_and_sentinel() {
        let root = tempfile::tempdir().unwrap();
        let sentinel = root.path().join("sentinel");
        std::fs::write(&sentinel, b"keep").unwrap();
        let first = IsolatedDirs::create(root.path()).unwrap();
        let second = IsolatedDirs::create(root.path()).unwrap();
        let first_run = first.run_dir.clone();
        let second_run = second.run_dir.clone();
        drop(first);
        assert!(!first_run.exists());
        assert!(second_run.exists());
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
        drop(second);
        assert!(!second_run.exists());
        assert!(sentinel.exists());
    }

    #[cfg(not(unix))]
    #[test]
    fn cleanup_preserves_directory_when_identity_is_unavailable() {
        let root = tempfile::tempdir().unwrap();
        let dirs = IsolatedDirs::create(root.path()).unwrap();
        let run = dirs.run_dir.clone();
        assert!(dirs.cleanup_after_termination().is_err());
        drop(dirs);
        assert!(run.exists());
    }

    #[test]
    fn explicit_cleanup_uses_recorded_owner_not_profile_path() {
        let root = tempfile::tempdir().unwrap();
        let mut first = IsolatedDirs::create(root.path()).unwrap();
        let second = IsolatedDirs::create(root.path()).unwrap();
        let first_run = first.run_dir.clone();
        let second_run = second.run_dir.clone();
        first.profile_dir = second.profile_dir.clone();
        assert!(first.cleanup_after_termination().is_err());
        assert!(first_run.exists());
        assert!(second_run.exists());
        drop(first);
        assert!(second_run.exists());
    }

    #[test]
    fn cleanup_rejects_replaced_or_missing_run() {
        let root = tempfile::tempdir().unwrap();
        let dirs = IsolatedDirs::create(root.path()).unwrap();
        let original = root.path().join("original");
        std::fs::rename(&dirs.run_dir, &original).unwrap();
        std::fs::create_dir(&dirs.run_dir).unwrap();
        let sentinel = dirs.run_dir.join("sentinel");
        std::fs::write(&sentinel, b"keep").unwrap();
        assert!(dirs.cleanup_after_termination().is_err());
        drop(dirs);
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
        assert!(original.exists());

        let missing = IsolatedDirs::create(root.path()).unwrap();
        std::fs::rename(&missing.run_dir, root.path().join("missing-original")).unwrap();
        assert!(missing.cleanup_after_termination().is_err());
        drop(missing);
        assert!(root.path().join("missing-original").exists());
    }

    #[test]
    fn drop_rejects_replaced_run() {
        let root = tempfile::tempdir().unwrap();
        let dirs = IsolatedDirs::create(root.path()).unwrap();
        std::fs::rename(&dirs.run_dir, root.path().join("original")).unwrap();
        std::fs::create_dir(&dirs.run_dir).unwrap();
        let sentinel = dirs.run_dir.join("sentinel");
        std::fs::write(&sentinel, b"keep").unwrap();
        drop(dirs);
        assert!(sentinel.exists());
    }

    #[test]
    fn canonical_matches_page_stable_form() {
        // The page's `stable()` output, computed independently in Node, is the
        // contract document signatures follow. The suite compares same-side
        // values only, so cross-parser float skew can never gate a check.
        let project: serde_json::Value = serde_json::from_str(
            r#"{"b": 0.30000000000000004, "a": [1, 2.5, 1e+21],
                "c": {"z": 1, "y": null}, "u": "↔"}"#,
        )
        .unwrap();
        let signature = serde_json::to_string(&crate::canonical(&project)).unwrap();
        assert_eq!(
            signature,
            r#"{"a":[1,2.5,1e+21],"b":0.30000000000000004,"c":{"y":null,"z":1},"u":"↔"}"#
        );
        // Key order never affects a signature.
        let shuffled: serde_json::Value = serde_json::from_str(
            r#"{"u": "↔", "c": {"z": 1, "y": null}, "a": [1, 2.5, 1e+21], "b": 0.30000000000000004}"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&crate::canonical(&shuffled)).unwrap(),
            signature
        );
    }

    #[test]
    fn isolated_dirs_clean_up_profile_on_drop() {
        let root = std::env::temp_dir().join(format!("bs-harness-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let profile = {
            let dirs = IsolatedDirs::create(&root).unwrap();
            let profile = dirs.profile_dir.clone();
            assert!(profile.is_dir());
            profile
        };
        assert!(!profile.exists(), "profile removed on drop");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn non_executable_chrome_bin_is_actionable() {
        let get_env = env_with(&[("CHROME_BIN", "/opt/missing-chrome")]);
        let is_executable = |_: &str| false;
        let error = discover_chrome(&get_env, &is_executable).unwrap_err();
        assert!(
            error.contains("CHROME_BIN") && error.contains("/opt/missing-chrome"),
            "actionable error names the bad value, got: {error}"
        );
    }
}
