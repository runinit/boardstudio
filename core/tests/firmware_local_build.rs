#![cfg(unix)]
use boardstudio_core::firmware::*;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn local_script_executes_exact_build_arguments_and_preserves_manifest() {
    let root = std::env::temp_dir().join(format!("boardstudio local zmk {}", std::process::id()));
    fs::create_dir_all(root.join("bin")).unwrap();
    let request = FirmwareRequest {
        controller_profile: "ceoloide/mcu_nice_nano".into(),
        board_name: "test".into(),
        rows: vec![ScanPin {
            terminal: "P21".into(),
            gpio: "P0.31".into(),
        }],
        columns: vec![ScanPin {
            terminal: "P20".into(),
            gpio: "P0.29".into(),
        }],
        keys: vec![FirmwareKey {
            id: "key".into(),
            row: 0,
            column: 0,
        }],
        diode_direction: "col2row".into(),
        ..FirmwareRequest::default()
    };
    let package = generate(&request).unwrap();
    for (name, content) in &package.files {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    let west = root.join("bin/west");
    fs::write(
        &west,
        r#"#!/bin/sh
printf '%s\n' BEGIN "$@" >> "$TEST_LOG"
case "$1" in
  topdir) exit 1 ;;
  init) mkdir .west ;;
  config) printf '%s\n' "${TEST_MANIFEST:-config}" ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&west, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        root.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |manifest: &str| {
        Command::new("sh")
            .arg("build-local.sh")
            .current_dir(&root)
            .env("PATH", &path)
            .env("TEST_LOG", root.join("calls"))
            .env("TEST_MANIFEST", manifest)
            .output()
            .unwrap()
    };
    let first = run("config");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let calls = fs::read_to_string(root.join("calls")).unwrap();
    assert!(calls.contains("init\n-l\nconfig"));
    assert!(calls.contains(
        "build\n-s\nzmk/app\n-d\nbuild/boardstudio\n-b\nnice_nano_v2\n--\n-DSHIELD=boardstudio"
    ));
    assert!(calls.contains(&format!("-DZMK_CONFIG={}/config", root.display())));
    assert!(run("config").status.success());
    assert!(!run("different-manifest").status.success());
    assert!(
        fs::read_to_string(root.join("config/west.yml"))
            .unwrap()
            .contains("v0.3.0")
    );
    fs::remove_dir_all(root).unwrap();
}
