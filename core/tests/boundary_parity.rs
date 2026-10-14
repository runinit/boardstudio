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
    let hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    assert_eq!(hex(boundary::ASSET), boundary::ASSET_SHA256);
    assert_eq!(hex(&boundary::payload()), boundary::PAYLOAD_SHA256);
}

#[test]
fn native_transport_reproduces_the_golden_transcript() {
    let mut engine = boardstudio_core::CoreEngine::new();
    let transcript = boundary::run(
        &mut |request| engine.request(request),
        &mut |request, buffers| {
            let (reply, outputs) =
                boardstudio_core::archive::request(&request.to_string(), buffers);
            (serde_json::from_str(&reply).unwrap(), outputs)
        },
    );
    if std::env::var_os("BOUNDARY_BLESS").is_some() {
        let mut text = serde_json::to_string_pretty(&transcript).unwrap();
        text.push('\n');
        std::fs::write(golden_path(), text).unwrap();
        return;
    }
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(golden_path())
            .expect("golden transcript; run with BOUNDARY_BLESS=1"),
    )
    .unwrap();
    boundary::assert_golden(&transcript, &golden);
}

#[test]
fn paired_preview_history_case_is_ready_for_boundary_blessing() {
    let mut engine = boardstudio_core::CoreEngine::new();
    let scenario = boundary::paired_preview_history_case(&mut |request| engine.request(request));
    assert_eq!(scenario["preview"]["kind"], "preview");
    assert_eq!(scenario["undo"]["kind"], "scene");
    assert_eq!(scenario["redo"]["kind"], "scene");
}

#[test]
fn linked_and_mirror_only_additions_cross_the_native_wasm_boundary() {
    let mut engine = boardstudio_core::CoreEngine::new();
    let scenario = boundary::mirror_addition_boundary_case(
        &mut |request| engine.request(request),
        &mut |request, buffers| {
            let (reply, outputs) =
                boardstudio_core::archive::request(&request.to_string(), buffers);
            (serde_json::from_str(&reply).unwrap(), outputs)
        },
    );
    assert!(
        scenario["linkedAdditions"]["sourceSideCorrespondence"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenario["linkedAdditions"]["mirrorSideCorrespondence"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenario["linkedAdditions"]["sourceAndMirrorLayoutsRegistered"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenario["mirrorOnly"]["previewNonpublishing"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenario["mirrorOnly"]["singleMirrorPart"]
            .as_bool()
            .unwrap()
    );
    assert!(scenario["mirrorOnly"]["sourceUnchanged"].as_bool().unwrap());
    assert!(
        scenario["mirrorOnly"]["noPairProvenance"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenario["mirrorOnly"]["mirrorBoardLayoutEnvelopeMembership"]
            .as_bool()
            .unwrap()
    );
    assert!(
        scenario["mirrorOnly"]["undoRemovedAndRedoRestored"]
            .as_bool()
            .unwrap()
    );
    assert!(scenario["mirrorOnly"]["openRetained"].as_bool().unwrap());
    assert!(
        scenario["mirrorOnly"]["saveOpenRetained"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(scenario["archiveRoundTrip"]["save"]["kind"], "packed");
    assert_eq!(
        scenario["archiveRoundTrip"]["save"]["buffers"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(scenario["archiveRoundTrip"]["open"]["kind"], "unpacked");
    assert!(
        scenario["archiveRoundTrip"]["open"]["projectJson"]
            .as_str()
            .is_some_and(|digest| !digest.is_empty())
    );
}

#[test]
fn electrical_mirror_replacement_is_complete_before_boundary_golden_blessing() {
    let mut engine = boardstudio_core::CoreEngine::new();
    let scenario = boundary::electrical_mirror_replacement_case(
        &mut |request| engine.request(request),
        &mut |request, buffers| {
            let (reply, outputs) =
                boardstudio_core::archive::request(&request.to_string(), buffers);
            (serde_json::from_str(&reply).unwrap(), outputs)
        },
    );
    for key in [
        "omitReleasesPerPartCs",
        "mirrorOnlyPlacementAutomaticallyPinned",
        "freedCsPinWasAllocatedToReplacement",
        "rejoinRestoresSourcePinsAndRepinsReplacement",
        "sourceUnchanged",
        "previewNonpublishing",
        "singleStepUndoRedo",
        "archiveSaveOpenPreservesAssignments",
        "omittedReplacementArchiveOpenStable",
        "capacityPlacementRefusalAtomic",
        "capacityRejoinRefusalAtomic",
    ] {
        assert_eq!(scenario[key], true, "{key}: {scenario}");
    }
}

#[test]
#[should_panic(expected = "core[1:splay-preview].n")]
fn a_mismatch_inside_an_array_names_the_entry_by_its_id() {
    let got = serde_json::json!([{"id": "open", "n": 1}, {"id": "splay-preview", "n": 3}]);
    let want = serde_json::json!([{"id": "open", "n": 1}, {"id": "splay-preview", "n": 7}]);
    boundary::equivalent(&got, &want, "core");
}

#[test]
#[should_panic(expected = "core[0].n")]
fn an_array_entry_without_an_id_keeps_its_index() {
    boundary::equivalent(
        &serde_json::json!([{"n": 3}]),
        &serde_json::json!([{"n": 7}]),
        "core",
    );
}
