use boardstudio_core::{
    migrate::CURRENT_VERSION,
    model::{MechanicalConfiguration, PartDefinition},
};

#[test]
fn rejects_obsolete_single_model_field() {
    let definition = serde_json::json!({
        "id": "custom", "name": "Custom", "kind": "custom", "pads": [], "courtyard": [],
        "model": {"assetId":"old", "offset":{"x":0,"y":0,"z":0}, "rotation":{"x":0,"y":0,"z":0}, "scale":{"x":1,"y":1,"z":1}}
    });
    assert!(serde_json::from_value::<PartDefinition>(definition).is_err());
}

#[test]
fn requires_explicit_mechanical_dimensions() {
    let obsolete = serde_json::json!({
        "boardId":"board", "method":"printed", "mount":"tray", "profiles":[]
    });
    assert!(serde_json::from_value::<MechanicalConfiguration>(obsolete).is_err());
}

#[test]
fn rejects_implicit_matrix_companion_flags() {
    let mut matrix = serde_json::json!({
        "id":"matrix", "rows":1, "columns":1, "pitch":{"x":19,"y":19},
        "origin":{"x":0,"y":0}, "definitionId":"switch", "partIds":[]
    });
    assert!(serde_json::from_value::<boardstudio_core::model::Matrix>(matrix.clone()).is_ok());
    matrix["diodes"] = true.into();
    assert!(serde_json::from_value::<boardstudio_core::model::Matrix>(matrix).is_err());
    let cell = serde_json::json!({"row":0,"column":0,"enabled":true,"diode":false});
    assert!(serde_json::from_value::<boardstudio_core::model::MatrixCell>(cell).is_err());
}

#[test]
fn current_model_list_round_trips() {
    let value = serde_json::json!({
        "id":"custom", "name":"Custom", "kind":"custom", "pads":[], "courtyard":[],
        "models":[{"assetId":"model", "offset":{"x":0,"y":0,"z":0}, "rotation":{"x":0,"y":0,"z":0}, "scale":{"x":1,"y":1,"z":1}}]
    });
    let definition: PartDefinition = serde_json::from_value(value).unwrap();
    let encoded = serde_json::to_value(&definition).unwrap();
    assert!(encoded.get("model").is_none());
    assert_eq!(
        serde_json::from_value::<PartDefinition>(encoded).unwrap(),
        definition
    );
}

fn pair_value(extra: serde_json::Value) -> serde_json::Value {
    let mut value = serde_json::json!({
        "id": "pair", "members": ["left", "right"],
        "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
        "construction": { "method": "printed", "mount": "tray", "integratedPlateFrame": false,
            "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
            "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3 },
        "features": []
    });
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    value
}

#[test]
fn a_pair_without_a_source_or_halves_stays_unpaired() {
    let pair: boardstudio_core::model::MechanicalMirrorPair =
        serde_json::from_value(pair_value(serde_json::json!({}))).unwrap();
    assert_eq!(pair.construction_source, None);
    assert_eq!(pair.halves, None);
    let encoded = serde_json::to_value(&pair).unwrap();
    assert!(encoded.get("constructionSource").is_none());
    assert!(encoded.get("halves").is_none());
}

#[test]
fn mirrored_halves_round_trip_in_camel_case() {
    let value = pair_value(serde_json::json!({
        "constructionSource": "left",
        "halves": {
            "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
            "correspondences": [{ "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" }],
            "exceptions": [{ "kind": "part", "mirrorId": "r-extra", "state": "omitted" }]
        }
    }));
    let pair: boardstudio_core::model::MechanicalMirrorPair =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(pair.construction_source.as_deref(), Some("left"));
    let halves = pair.halves.as_ref().expect("halves decode");
    assert_eq!(halves.source_board_id, "left-board");
    assert_eq!(halves.mirror_board_id, "right-board");
    assert_eq!(halves.correspondences.len(), 1);
    assert_eq!(halves.exceptions.len(), 1);
    let encoded = serde_json::to_value(&pair).unwrap();
    assert_eq!(encoded["constructionSource"], value["constructionSource"]);
    assert_eq!(encoded["halves"], value["halves"]);
    let decoded: boardstudio_core::model::MechanicalMirrorPair =
        serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, pair);
}

#[test]
fn unknown_mirrored_item_kinds_and_exception_states_are_refused() {
    let kind = pair_value(serde_json::json!({
        "halves": { "sourceBoardId": "a", "mirrorBoardId": "b",
            "correspondences": [{ "kind": "switch", "sourceId": "x", "mirrorId": "y" }] }
    }));
    assert!(serde_json::from_value::<boardstudio_core::model::MechanicalMirrorPair>(kind).is_err());
    // A linked state is the default, never an exception.
    let state = pair_value(serde_json::json!({
        "halves": { "sourceBoardId": "a", "mirrorBoardId": "b",
            "exceptions": [{ "kind": "part", "mirrorId": "y", "state": "linked" }] }
    }));
    assert!(
        serde_json::from_value::<boardstudio_core::model::MechanicalMirrorPair>(state).is_err()
    );
}

/// Direct Open reads only the current format and never offers a conversion.
#[test]
fn open_refuses_documents_that_are_not_at_the_current_version() {
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{CoreReply, CoreRequest, ProjectDoc};

    let mut engine = CoreEngine::new();
    for version in [CURRENT_VERSION - 1, CURRENT_VERSION + 1] {
        let mut document = ProjectDoc::empty("doc", "Doc");
        document.format_version = version;
        match engine.handle(CoreRequest::Open {
            id: "open".into(),
            document,
        }) {
            CoreReply::Error { message, .. } => {
                assert!(message.contains(&version.to_string()), "{message}");
                assert!(message.contains(&CURRENT_VERSION.to_string()), "{message}");
                let explanation = if version < CURRENT_VERSION {
                    "No conversion is available"
                } else {
                    "compatible newer build"
                };
                assert!(message.contains(explanation), "{message}");
            }
            other => panic!("version {version} must be refused: {other:?}"),
        }
    }
    let current = ProjectDoc::empty("doc", "Doc");
    assert_eq!(current.format_version, CURRENT_VERSION);
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: current,
        }),
        CoreReply::Scene { .. }
    ));
}
