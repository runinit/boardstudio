use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=BOARDSTUDIO_OFFLINE_MANIFEST");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    let manifest = match env::var_os("BOARDSTUDIO_OFFLINE_MANIFEST") {
        Some(path) => {
            let path = PathBuf::from(path);
            println!("cargo:rerun-if-changed={}", path.display());
            fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "could not read BOARDSTUDIO_OFFLINE_MANIFEST {}: {error}",
                    path.display()
                )
            })
        }
        None if env::var_os("CARGO_FEATURE_SERVICE_WORKER").is_some() => {
            panic!("service-worker builds require BOARDSTUDIO_OFFLINE_MANIFEST")
        }
        None => r#"{"version":"development","assets":[]}"#.to_owned(),
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&manifest).expect("offline release manifest must be valid JSON");
    let version = parsed
        .get("version")
        .and_then(serde_json::Value::as_str)
        .expect("offline manifest must have a string version");
    assert!(
        !version.is_empty()
            && version
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')),
        "offline manifest version contains unsupported characters"
    );
    let assets = parsed
        .get("assets")
        .and_then(serde_json::Value::as_array)
        .expect("offline manifest must have an asset array");
    let requires_manifest = env::var_os("CARGO_FEATURE_SERVICE_WORKER").is_some();
    if requires_manifest {
        assert!(!assets.is_empty(), "offline manifest has no assets");
    }
    let mut paths = std::collections::BTreeSet::new();
    let mut rust_assets = Vec::new();
    for asset in assets {
        let path = asset.as_str().expect("offline assets must be strings");
        assert!(
            !path.is_empty()
                && !path.starts_with('/')
                && !path
                    .chars()
                    .any(|character| matches!(character, '\\' | '?' | '#' | ':' | '%'))
                && !path
                    .split('/')
                    .any(|segment| segment.is_empty() || matches!(segment, "." | "..")),
            "offline asset path is not normalized and scope-relative: {path}"
        );
        assert!(paths.insert(path), "duplicate offline asset path: {path}");
        rust_assets.push(serde_json::to_string(path).expect("encode Rust string literal"));
    }
    if requires_manifest {
        assert!(
            paths.contains("index.html"),
            "offline manifest must include index.html"
        );
    }
    let assets_source = rust_assets
        .iter()
        .map(|asset| format!("    {asset},\n"))
        .collect::<String>();
    let generated = format!(
        "pub const BUILD_OFFLINE_VERSION: &str = {};\npub const BUILD_OFFLINE_ASSETS: &[&str] = &[\n{assets_source}];\n",
        serde_json::to_string(version).expect("encode Rust version literal")
    );
    fs::write(output.join("offline_manifest.rs"), generated)
        .expect("write compiled offline release manifest");
}
