//! Ticket 34: per-key Omit/Rejoin resolves to the typed half-exception intents with
//! atomicity and Undo. Native: no browser needed.

use super::super::{
    KeyHalfException, key_half_exception, key_half_exception_at_resolver,
    key_half_exception_resolver,
};
use boardstudio_application::{AcceptedSnapshot, Resolution, SessionEpoch, SnapshotToken};
use boardstudio_model::model::{
    EditOperation, HalfExceptionState, MirroredItemKind, ProjectDoc, SceneDelta,
};
use std::sync::Arc;

fn pair_doc() -> ProjectDoc {
    serde_json::from_value(serde_json::json!({
        "id": "omit", "name": "Omit", "format": "boardstudio/v2", "revision": 0,
        "parameters": {},
        "formatVersion": boardstudio_model::migrate::CURRENT_VERSION,
        "boards": [
            {"id": "source-board", "name": "Source", "partIds": ["source-key"],
             "outlineIds": [], "netIds": [], "thickness": 1.6},
            {"id": "mirror-board", "name": "Mirror", "partIds": ["mirror-key"],
             "outlineIds": [], "netIds": [], "thickness": 1.6}
        ],
        "parts": [
            {"id": "source-key", "definitionId": "sw", "reference": "S1",
             "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front"},
            {"id": "mirror-key", "definitionId": "sw", "reference": "S2",
             "pose": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0}, "side": "front"}
        ],
        "definitions": [{"id": "sw", "name": "SW", "kind": "switch",
            "courtyard": [], "pads": []}],
        "matrices": [], "layouts": [], "nets": [], "outline": [],
        "boardOutlines": [], "caseBodies": [], "materials": [], "assets": [],
        "scripts": [], "constraints": [],
        "hardware": {"topology": "split", "transport": "wired", "instances": [
            {"id": "s", "name": "Source", "boardId": "source-board",
             "half": "left", "role": "central", "flipped": false,
             "controllerPartId": null, "mechanical": null},
            {"id": "m", "name": "Mirror", "boardId": "mirror-board",
             "half": "right", "role": "central", "flipped": false,
             "controllerPartId": null, "mechanical": null}
        ], "pairs": [{
            "id": "pair", "members": ["s", "m"],
            "constructionSource": "s",
            "mapping": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0},
            "construction": {
                "method": "printed", "mount": "rigid",
                "integratedPlateFrame": false, "plateThickness": 1.5,
                "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                "bottomThickness": 2.0, "plateToPcb": 3.5,
                "wallThickness": 0.2, "clearance": 0.2
            },
            "halves": {
                "sourceBoardId": "source-board", "mirrorBoardId": "mirror-board",
                "correspondences": [
                    {"kind": "matrix", "sourceId": "source-m", "mirrorId": "m"},
                    {"kind": "part", "sourceId": "source-key", "mirrorId": "mirror-key"}
                ],
                "exceptions": []
            }
        }]}
    }))
    .unwrap()
}

/// `pair_doc` with an Omitted overlay on the named mirror member.
fn omitted(mut document: ProjectDoc, mirror_id: &str) -> ProjectDoc {
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(
            serde_json::from_value(serde_json::json!(
                {"kind": "part", "mirrorId": mirror_id, "state": "omitted"}
            ))
            .unwrap(),
        );
    document
}

fn snapshot(document: ProjectDoc) -> AcceptedSnapshot {
    AcceptedSnapshot {
        token: SnapshotToken(1),
        session_epoch: SessionEpoch(1),
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 0,
            transaction_id: "key-omit-test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: boardstudio_model::model::Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        }),
    }
}

fn half_exception(mirror_id: &str, state: HalfExceptionState) -> EditOperation {
    EditOperation::SetHalfException {
        pair_id: "pair".into(),
        kind: MirroredItemKind::Part,
        mirror_id: mirror_id.into(),
        state,
    }
}

fn resolve(accepted: &AcceptedSnapshot, pair: &str, mirror: &str, omit: bool) -> Resolution {
    key_half_exception_resolver(pair.into(), mirror.into(), omit).resolve(accepted)
}

fn submitted(resolution: Resolution) -> EditOperation {
    match resolution {
        Resolution::Submit(command) => command.operation,
        Resolution::Unchanged => panic!("the intent must submit"),
        Resolution::Retire(message) => panic!("the intent must not retire: {message}"),
    }
}

#[test]
fn omit_submits_part_omitted_and_rejoin_submits_linked() {
    let accepted = snapshot(pair_doc());
    assert_eq!(
        submitted(resolve(&accepted, "pair", "mirror-key", true)),
        half_exception("mirror-key", HalfExceptionState::Omitted)
    );
    // Rejoining a linked key changes nothing.
    let rejoin = resolve(&accepted, "pair", "mirror-key", false);
    assert!(matches!(rejoin, Resolution::Unchanged));
}

#[test]
fn rejoin_clears_an_omitted_overlay_and_unknown_members_retire() {
    let accepted = snapshot(omitted(pair_doc(), "mirror-key"));
    assert_eq!(
        submitted(resolve(&accepted, "pair", "mirror-key", false)),
        half_exception("mirror-key", HalfExceptionState::Linked)
    );
    // The same state is Unchanged; an unknown member or pair retires.
    let omit = resolve(&accepted, "pair", "mirror-key", true);
    assert!(matches!(omit, Resolution::Unchanged));
    for (pair, mirror) in [("pair", "ghost"), ("ghost", "mirror-key")] {
        let resolution = resolve(&accepted, pair, mirror, true);
        assert!(
            matches!(resolution, Resolution::Retire(_)),
            "{pair}/{mirror}"
        );
    }
}

#[test]
fn projection_helper_reports_pair_and_omitted_state() {
    let accepted = snapshot(pair_doc());
    assert_eq!(
        key_half_exception(&accepted.document, "mirror-key"),
        Some(KeyHalfException {
            pair_id: "pair".into(),
            mirror_id: "mirror-key".into(),
            omitted: false,
        })
    );
    assert_eq!(key_half_exception(&accepted.document, "source-key"), None);
    assert_eq!(key_half_exception(&accepted.document, "ghost"), None);
    let omitted = snapshot(omitted(pair_doc(), "mirror-key"));
    assert!(
        key_half_exception(&omitted.document, "mirror-key")
            .is_some_and(|exception| exception.omitted),
        "the projection surfaces the Omitted overlay"
    );
}

#[test]
fn rejoin_resolves_an_omitted_key_without_a_scene_cell() {
    // Mirrored matrix members carry derived identities, so the Omitted overlay names the
    // canonical member the scene no longer shows.
    const MEMBER: &str = "matrix/m/r0c0";
    let mut document = pair_doc();
    for part in document
        .parts
        .iter_mut()
        .filter(|part| part.id == "mirror-key")
    {
        part.id = MEMBER.into();
    }
    for id in document
        .boards
        .iter_mut()
        .flat_map(|board| board.part_ids.iter_mut())
        .filter(|id| id.as_str() == "mirror-key")
    {
        *id = MEMBER.into();
    }
    for link in document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .iter_mut()
        .filter(|link| link.mirror_id == "mirror-key")
    {
        link.mirror_id = MEMBER.into();
    }
    let accepted = snapshot(omitted(document, MEMBER));
    // No scene cells exist, yet the Omitted overlay resolves the member.
    let rejoin = key_half_exception_at_resolver("m".into(), 0, 0, false).resolve(&accepted);
    let operation = submitted(rejoin);
    assert_eq!(
        operation,
        half_exception(MEMBER, HalfExceptionState::Linked)
    );
}
