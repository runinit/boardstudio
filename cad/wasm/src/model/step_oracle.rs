//! Client for the test-only STEP oracle (`cad/step-oracle`).
//!
//! The oracle reads STEP with OCCT's own `STEPControl_Reader` in a separate process, so test
//! evidence never passes through Cadrum's reader (which sews orphan faces) and never shares
//! OCCT global state with the code under test. It is adapter-independent, not
//! kernel-independent. Build it with `python3 cad/scripts/test-cadrum.py` (or
//! `cargo build --manifest-path cad/step-oracle/Cargo.toml` with `OCCT_ROOT` set), or point
//! `STEP_ORACLE_BIN` at a binary.

use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

/// `IFSelect_RetDone` / `IFSelect_RetFail` as reported by OCCT.
pub const READ_DONE: i64 = 1;
pub const READ_FAIL: i64 = 3;

#[derive(Debug, Clone)]
pub struct Report {
    pub read_status: i64,
    pub error: String,
    pub solids: usize,
    pub shells: usize,
    pub faces: usize,
    /// Faces in the shape that belong to no solid.
    pub orphan_faces: usize,
    /// `BRepCheck_Analyzer::IsValid`.
    pub valid: bool,
    pub volume: f64,
    pub min: [f64; 3],
    pub max: [f64; 3],
}

fn binary() -> PathBuf {
    if let Some(path) = std::env::var_os("STEP_ORACLE_BIN") {
        return path.into();
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../step-oracle/target/debug/boardstudio-step-oracle")
}

fn triple(value: &Value) -> [f64; 3] {
    [0, 1, 2].map(|axis| value[axis].as_f64().expect("oracle coordinate"))
}

pub fn inspect_file(path: &std::path::Path) -> Report {
    let binary = binary();
    assert!(
        binary.exists(),
        "STEP oracle binary missing at {}; run `python3 cad/scripts/test-cadrum.py` or build cad/step-oracle",
        binary.display()
    );
    let output = Command::new(&binary)
        .arg(path)
        .output()
        .expect("run STEP oracle");
    assert!(
        output.status.success(),
        "STEP oracle crashed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.starts_with('{'))
        .unwrap_or_else(|| panic!("STEP oracle printed no report: {stdout}"));
    let value: Value = serde_json::from_str(line).expect("oracle JSON report");
    Report {
        read_status: value["readStatus"].as_i64().unwrap(),
        error: value["error"].as_str().unwrap().to_string(),
        solids: value["solids"].as_u64().unwrap() as usize,
        shells: value["shells"].as_u64().unwrap() as usize,
        faces: value["faces"].as_u64().unwrap() as usize,
        orphan_faces: value["orphanFaces"].as_u64().unwrap() as usize,
        valid: value["valid"].as_bool().unwrap(),
        volume: value["volume"].as_f64().unwrap(),
        min: triple(&value["min"]),
        max: triple(&value["max"]),
    }
}

pub fn inspect(step: &[u8]) -> Report {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "boardstudio-oracle-{}-{}.step",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, step).expect("write STEP for the oracle");
    let report = inspect_file(&path);
    let _ = fs::remove_file(&path);
    report
}
