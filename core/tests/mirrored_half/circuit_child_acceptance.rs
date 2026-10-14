//! Public Core acceptance for lifecycle exceptions on an embedded circuit's member.
use super::*;

const PAIR_ID: &str = "parent19-circuit-child-pair";

fn circuit_child_commit(
    engine: &mut CoreEngine,
    document: &Value,
    id: &str,
    targets: &[&str],
    operation: Value,
) -> Value {
    request(
        engine,
        json!({"id":id,"kind":"edit","command":{
            "baseRevision":document["revision"],"transactionId":id,"phase":"commit",
            "targetIds":targets,"operation":operation
        }}),
    )
}

fn assert_child_history(actual: &Value, expected: &Value, label: &str) {
    let mut expected = expected.clone();
    expected["revision"] = actual["revision"].clone();
    assert_eq!(actual, &expected, "{label} restores the complete document");
}

fn circuit_child_save_open(document: &Value) -> (CoreEngine, Value) {
    let (packed_reply, packed_buffers) = boardstudio_core::archive::request(
        &json!({"kind":"pack-project","projectJson":document.to_string(),"assets":[]}).to_string(),
        &[],
    );
    let packed: Value = serde_json::from_str(&packed_reply).unwrap();
    assert_eq!(packed["kind"], "packed", "{packed}");
    let (unpacked_reply, buffers) =
        boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &packed_buffers);
    let unpacked: Value = serde_json::from_str(&unpacked_reply).unwrap();
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    assert!(buffers.is_empty());
    let saved: Value = serde_json::from_str(unpacked["projectJson"].as_str().unwrap()).unwrap();
    assert_eq!(
        saved, *document,
        "archive preserves the pair and member overlay"
    );
    open(&saved)
}

/// A member's Local/Omitted overlay survives edits to its source circuit, archive Save/Open,
/// and Rejoin. The neighboring member remains linked and the owning circuits retain their nets.
#[test]
fn parent19_embedded_circuit_member_local_and_omitted_are_selective_and_rejoin_latest_source() {
    for state in ["local", "omitted"] {
        let source = synthetic_half::with_construction_features(runtime_supported_document());
        let (mut engine, seed) = seed(&source, "left");
        let created = create(
            &mut engine,
            &seed,
            PAIR_ID,
            "left",
            new_board("right", "right"),
            mapping(200.0),
        )
        .unwrap();
        let pair_document = created["document"].clone();
        let embedded_id = "right/parent19-runtime-circuit";
        let embedded = circuit_child_commit(
            &mut engine,
            &pair_document,
            "parent19-circuit-child-embed",
            &[embedded_id],
            json!({"kind":"embed-module-circuit","id":embedded_id,
                "definition":imported_haptic(),"hostBoardId":"right",
                "pose":{"at":{"x":120.0,"y":40.0},"rotation":0.0},
                "side":"front","joins":{}}),
        );
        assert_eq!(embedded["kind"], "scene", "{embedded}");
        let linked = embedded["document"].clone();
        let child_pair = pairs_of_kind(&linked, PAIR_ID, "part")
            .into_iter()
            .find(|(source_id, mirror_id)| {
                source_id.starts_with("embedded/left/parent19-runtime-circuit/")
                    && mirror_id.starts_with("embedded/right/parent19-runtime-circuit/")
            })
            .expect("an imported circuit member has a durable mirrored correspondence");
        let source_id = child_pair.0;
        let mirror_id = child_pair.1;
        let sibling = pairs_of_kind(&linked, PAIR_ID, "part")
            .into_iter()
            .find(|(source, mirror)| {
                source != &source_id
                    && mirror != &mirror_id
                    && source.starts_with("embedded/left/parent19-runtime-circuit/")
                    && mirror.starts_with("embedded/right/parent19-runtime-circuit/")
            })
            .expect("the imported circuit has a sibling member");
        let sibling_before = find_part(&linked, &sibling.1).unwrap().clone();
        let sibling_correspondence = pairs_of_kind(&linked, PAIR_ID, "part")
            .into_iter()
            .find(|pair| pair.0 == sibling.0 && pair.1 == sibling.1)
            .expect("sibling correspondence exists");
        let original_mirror = find_part(&linked, &mirror_id).unwrap().clone();
        let owner_nets: Vec<_> = [
            "left/parent19-runtime-circuit",
            "right/parent19-runtime-circuit",
        ]
        .iter()
        .filter_map(|id| {
            linked["embeddedCircuits"]
                .as_array()?
                .iter()
                .find(|c| c["id"] == *id)
        })
        .map(|c| json!({"id":c["id"],"ports":c["ports"],"netIds":c["netIds"]}))
        .collect();

        let exceptional = circuit_child_commit(
            &mut engine,
            &linked,
            "parent19-circuit-child-exception",
            &[&mirror_id],
            json!({"kind":"set-half-exception","pairId":PAIR_ID,
                "itemKind":"part","mirrorId":mirror_id,"state":state}),
        );
        assert_eq!(exceptional["kind"], "scene", "{exceptional}");
        let exception_doc = exceptional["document"].clone();
        let source_pose = find_part(&exception_doc, &source_id).unwrap()["pose"]["at"].clone();
        let moved_to = json!({"x":source_pose["x"].as_f64().unwrap()+7.0,
            "y":source_pose["y"].as_f64().unwrap()-4.0});
        let moved = circuit_child_commit(
            &mut engine,
            &exception_doc,
            "parent19-circuit-child-source-edit",
            &[&source_id],
            json!({"kind":"move-parts","positions":[{"id":source_id,"at":moved_to}]}),
        );
        assert_eq!(moved["kind"], "scene", "{moved}");
        let edited = moved["document"].clone();
        assert_eq!(
            find_part(&edited, &sibling.1).unwrap(),
            &sibling_before,
            "sibling correspondence remains linked and unchanged"
        );
        assert!(
            pairs_of_kind(&edited, PAIR_ID, "part")
                .iter()
                .any(
                    |pair| pair.0 == sibling_correspondence.0 && pair.1 == sibling_correspondence.1
                ),
            "sibling correspondence remains linked"
        );
        assert_eq!(
            edited["embeddedCircuits"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["id"] == "left/parent19-runtime-circuit"
                    || c["id"] == "right/parent19-runtime-circuit")
                .map(|c| json!({"id":c["id"],"ports":c["ports"],"netIds":c["netIds"]}))
                .collect::<Vec<_>>(),
            owner_nets,
            "editing a member preserves both owning circuits' net references"
        );
        if state == "local" {
            assert_eq!(find_part(&edited, &mirror_id).unwrap(), &original_mirror);
        } else {
            assert!(
                find_part(&edited, &mirror_id).is_none(),
                "Omitted member remains absent"
            );
        }

        let (mut reopened_engine, reopened) = circuit_child_save_open(&edited);
        assert_eq!(
            pair(&reopened, PAIR_ID)["halves"]["exceptions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["mirrorId"] == mirror_id)
                .unwrap()["state"],
            state
        );
        assert_eq!(
            pairs_of_kind(&reopened, PAIR_ID, "part")
                .iter()
                .find(|(_, mirror)| mirror == &mirror_id)
                .unwrap()
                .0,
            source_id
        );
        let before_rejoin = snapshot(&mut reopened_engine);
        let rejoined = circuit_child_commit(
            &mut reopened_engine,
            &before_rejoin,
            "parent19-circuit-child-rejoin",
            &[&mirror_id],
            json!({"kind":"set-half-exception","pairId":PAIR_ID,
                "itemKind":"part","mirrorId":mirror_id,"state":"linked"}),
        );
        assert_eq!(rejoined["kind"], "scene", "{rejoined}");
        let rejoined_doc = rejoined["document"].clone();
        let pose = &find_part(&rejoined_doc, &mirror_id).unwrap()["pose"]["at"];
        let source = &find_part(&rejoined_doc, &source_id).unwrap()["pose"]["at"];
        let (sx, sy) = (source["x"].as_f64().unwrap(), source["y"].as_f64().unwrap());
        // `mapping(200.0)` is an x-reflection translated to x=200: x' = 200 - x.
        assert!((pose["x"].as_f64().unwrap() - (200.0 - sx)).abs() < 1e-9);
        assert!((pose["y"].as_f64().unwrap() - sy).abs() < 1e-9);
        assert!(
            pairs_of_kind(&rejoined_doc, PAIR_ID, "part")
                .iter()
                .any(|(source, mirror)| source == &source_id && mirror == &mirror_id)
        );
        assert!(
            pair(&rejoined_doc, PAIR_ID)["halves"]["exceptions"]
                .as_array()
                .is_none_or(|exceptions| exceptions.iter().all(|e| e["mirrorId"] != mirror_id)),
            "Rejoin clears only selected overlay"
        );
        let undo = request(
            &mut reopened_engine,
            json!({"id":"parent19-circuit-child-undo","kind":"undo"}),
        );
        assert_eq!(undo["kind"], "scene", "{undo}");
        assert_child_history(&undo["document"], &before_rejoin, "Rejoin Undo");
        if state == "omitted" {
            assert!(find_part(&undo["document"], &mirror_id).is_none());
        }
        let redo = request(
            &mut reopened_engine,
            json!({"id":"parent19-circuit-child-redo","kind":"redo"}),
        );
        assert_eq!(redo["kind"], "scene", "{redo}");
        assert_child_history(&redo["document"], &rejoined_doc, "Rejoin Redo");
    }
}
