//! Records and verifies the golden request/reply transcript that the browser
//! test (`web/tests/boundary_parity.rs`) must reproduce through the WASM
//! exports. Set `BOUNDARY_BLESS=1` to rewrite the golden after an intended
//! protocol change.
use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};

#[path = "support/boundary.rs"]
mod boundary;

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/boundary/transcript.json")
}

#[test]
fn fixture_hashes_match_their_bytes() {
    let hex = |bytes: &[u8]| Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    assert_eq!(hex(boundary::ASSET), boundary::ASSET_SHA256);
    assert_eq!(hex(&boundary::payload()), boundary::PAYLOAD_SHA256);
}

#[test]
fn native_transport_reproduces_the_golden_transcript() {
    let mut engine = boardstudio_core::CoreEngine::new();
    let transcript = boundary::run(
        &mut |request| engine.request(request),
        &mut |request, buffers| {
            let (reply, outputs) = boardstudio_core::archive::request(&request.to_string(), buffers);
            (serde_json::from_str(&reply).unwrap(), outputs)
        },
    );
    if std::env::var_os("BOUNDARY_BLESS").is_some() {
        let mut text = serde_json::to_string_pretty(&transcript).unwrap();
        text.push('\n');
        std::fs::write(golden_path(), text).unwrap();
        return;
    }
    let golden: Value = serde_json::from_str(&std::fs::read_to_string(golden_path()).expect("golden transcript; run with BOUNDARY_BLESS=1")).unwrap();
    boundary::assert_golden(&transcript, &golden);
}
