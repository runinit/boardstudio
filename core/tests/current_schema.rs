use boardstudio_core::model::{MechanicalConfiguration, PartDefinition};

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
