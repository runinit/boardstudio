use boardstudio_core::firmware::FirmwareRequest;
use boardstudio_core::{CommittedScene, CoreEngine, EditOutcome, model::*};
use serde_json::{Value, json};

fn firmware_request(keymap: Value) -> FirmwareRequest {
    serde_json::from_value(json!({
        "controller_profile":"ceoloide/mcu_nice_nano","board_name":"board","mode":"matrix",
        "rows":[{"terminal":"P21","gpio":"P0.31"}],"columns":[{"terminal":"P20","gpio":"P0.29"}],
        "keys":[{"id":"k","row":0,"column":0}],"diode_direction":"col2row","keymap":keymap
    }))
    .unwrap()
}
fn layered() -> Value {
    json!({"layers":[
        {"id":"base","name":"Base","bindings":{"k":{"kind":"mod-tap","hold":"LSHIFT","tap":"A"}}},
        {"id":"nav","name":"Navigation","bindings":{"k":{"kind":"layer-tap","layerId":"base","tap":"SPACE"}}}
    ],"macros":[]})
}
#[test]
fn public_firmware_export_emits_typed_layers_and_hold_tap_bindings() {
    let Ok(package) = CoreEngine::generate_firmware(&firmware_request(layered())) else {
        panic!("keymap export")
    };
    let source = &package.files["config/boards/shields/boardstudio/boardstudio.keymap"];
    assert!(source.contains("label = \"Navigation\""), "{source}");
    assert!(source.contains("&mt LSHIFT A"), "{source}");
    assert!(source.contains("&lt 0 SPACE"), "{source}");
}

#[test]
fn invalid_layer_reference_and_dts_injection_are_rejected() {
    for invalid in [
        json!({"kind":"layer-tap","layerId":"missing","tap":"A"}),
        json!({"kind":"key-press","keycode":"A>; / { hacked"}),
    ] {
        let mut map = layered();
        map["layers"][0]["bindings"]["k"] = invalid;
        assert!(!CoreEngine::generate_firmware(&firmware_request(map)).is_ok());
    }
}
#[test]
fn macros_waits_and_encoder_actions_are_in_generated_source() {
    let mut map = layered();
    map["macros"] = json!([{"id":"hello","name":"Hello","tapMs":30,"waitMs":5,"steps":[{"kind":"tap","binding":{"kind":"key-press","keycode":"LC(A)"}},{"kind":"wait","ms":100},{"kind":"release","binding":{"kind":"key-press","keycode":"A"}}]}]);
    map["layers"][0]["bindings"]["k"] = json!({"kind":"macro","macroId":"hello"});
    map["layers"][0]["sensors"] = json!({"encoder":{"clockwise":{"kind":"key-press","keycode":"C_VOL_UP"},"counterclockwise":{"kind":"key-press","keycode":"C_VOL_DN"}}});
    let mut request = serde_json::to_value(firmware_request(map)).unwrap();
    request["encoder_ids"] = json!(["encoder"]);
    let Ok(package) = CoreEngine::generate_firmware(&serde_json::from_value(request).unwrap())
    else {
        panic!("keymap export")
    };
    let source = &package.files["config/boards/shields/boardstudio/boardstudio.keymap"];
    for text in [
        "&bs_macro_0",
        "<&macro_wait_time 100>, <&macro_press &none>, <&macro_wait_time 5>",
        "&kp LC(A)",
        "zmk,behavior-sensor-rotate",
        "<&kp C_VOL_UP>, <&kp C_VOL_DN>",
        "sensor-bindings = <&bs_sensor_0_0>",
    ] {
        assert!(source.contains(text), "missing {text}: {source}");
    }
}
#[test]
fn public_keymap_edit_and_undo_preserve_keycaps_and_legacy_data() {
    let mut value = serde_json::to_value(ProjectDoc::empty("map", "Map")).unwrap();
    value["definitions"] =
        json!([{"id":"mx","name":"MX","kind":"switch","courtyard":[],"pads":[]}]);
    value["parts"] = json!([{"id":"key","reference":"SW1","definitionId":"mx","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"}]);
    value["boards"] = json!([{"id":"board","name":"Board","partIds":["key"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
    value["keycaps"] = json!({"keys":{"key":{"legend":"Q"}},"matrices":{},"boards":{}});
    let doc: ProjectDoc = serde_json::from_value(value).unwrap();
    let mut engine = CoreEngine::new();
    engine.open(doc.clone()).unwrap();
    let operation: EditOperation = serde_json::from_value(json!({"kind":"edit-keymap","change":{"kind":"binding","layerId":"base","keyId":"key","binding":{"kind":"mod-tap","hold":"LSHIFT","tap":"A"}}})).unwrap();
    let reply = engine.commit_edit(EditCommand {
        base_revision: 0,
        transaction_id: "binding".into(),
        phase: EditPhase::Commit,
        target_ids: vec!["key".into()],
        operation,
    });
    let Ok(CommittedScene { document, .. }) = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(document.keycaps, doc.keycaps);
    assert!(document.keymap.is_some());
    let decoded: ProjectDoc =
        serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
    assert_eq!(decoded.keymap, document.keymap);
    let mut legacy_engine = CoreEngine::new();
    legacy_engine.open(document.clone()).unwrap();
    let reply = legacy_engine.apply_edit(EditCommand {
        base_revision: document.revision,
        transaction_id: "legacy".into(),
        phase: EditPhase::Commit,
        target_ids: vec!["key".into()],
        operation: EditOperation::SetKeyBinding {
            board_id: "board".into(),
            key_id: "key".into(),
            binding: "&kp B".into(),
        },
    });
    let Ok(EditOutcome::Committed(CommittedScene {
        document: updated, ..
    })) = reply
    else {
        panic!("{reply:?}")
    };
    assert_eq!(
        updated.keymap.unwrap().layers[0].bindings["key"],
        KeyBinding::KeyPress {
            keycode: "B".into()
        }
    );

    let Ok(CommittedScene { document, .. }) = engine.undo() else {
        panic!("undo")
    };
    assert_eq!(document.keymap, doc.keymap);
    assert_eq!(document.keycaps, doc.keycaps);
}
#[test]
fn opening_an_invalid_persisted_keymap_returns_an_error() {
    let mut doc = ProjectDoc::empty("invalid", "Invalid");
    doc.keymap = Some(KeymapConfiguration {
        layers: vec![],
        macros: vec![],
    });
    assert!(CoreEngine::new().open(doc).is_err());
}
#[test]
fn accepted_large_macros_receive_sufficient_behavior_queue_capacity() {
    let mut map = layered();
    map["macros"] = json!([{"id":"long","name":"Long","tapMs":30,"waitMs":0,"steps":vec![json!({"kind":"tap","binding":{"kind":"key-press","keycode":"A"}});128]}]);
    let Ok(package) = CoreEngine::generate_firmware(&firmware_request(map)) else {
        panic!("export")
    };
    assert!(
        package
            .files
            .values()
            .any(|text| text.contains("CONFIG_ZMK_BEHAVIORS_QUEUE_SIZE=512"))
    );
}

#[test]
fn macro_expansion_respects_zmk_binding_limit_at_the_boundary() {
    for (count, accepted) in [(64, true), (65, false)] {
        let mut map = layered();
        map["macros"] = json!([{"id":"waits","name":"Waits","tapMs":30,"waitMs":0,"steps":vec![json!({"kind":"wait","ms":1});count]}]);
        assert_eq!(
            CoreEngine::generate_firmware(&firmware_request(map)).is_ok(),
            accepted,
            "{count} waits"
        );
    }
}

#[test]
fn encoder_push_without_switches_exports_a_real_direct_scanner() {
    let mut request = serde_json::to_value(firmware_request(layered())).unwrap();
    request["mode"] = json!("direct");
    request["rows"] = json!([]);
    request["columns"] = json!([]);
    request["auxiliary_pins"] = json!([{"terminal":"P19","gpio":"P0.02"}]);
    request["keys"] = json!([{"id":"k","row":1,"column":0}]);
    let Ok(package) = CoreEngine::generate_firmware(&serde_json::from_value(request).unwrap())
    else {
        panic!("direct scanner export")
    };
    let overlay = &package.files["config/boards/shields/boardstudio/boardstudio.overlay"];
    assert!(overlay.contains("input-gpios = <&gpio0 2"));
    assert!(!overlay.contains("input-gpios = <>"));
    assert!(overlay.contains("RC(0, 0)"));
}

fn configured_vik_rotary_request() -> serde_json::Value {
    let mut map = layered();
    map["layers"][0]["sensors"] = json!({"module/left-encoder":{"clockwise":{"kind":"key-press","keycode":"C_VOL_UP"},"counterclockwise":{"kind":"key-press","keycode":"C_VOL_DN"}}});
    let mut value = serde_json::to_value(firmware_request(map)).unwrap();
    value["encoder_ids"] = json!(["module/left-encoder"]);
    value["encoders"] = json!([{"id":"module/left-encoder","profile":{"a":"gpio1","b":"gpio2","common":"gnd","steps":24,"triggersPerRotation":4,"driver":"ec11"},"aGpio":"P0.02","bGpio":"P1.15"}]);
    value["hardware"] = json!({"boardId":"board","modules":[{"id":"module/left-encoder","name":"EC11 VIK","protocol":"gpio","catalogueRow":"ec11-evqwgd001","source":{"repository":"https://github.com/sadekbaroudi/vik","revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553","path":"pcb/ec11-evqwgd001/ec11-evqwgd001.kicad_pcb","license":"CERN-OHL-S-2.0","sha256":"2b22c0f0b2b99206ac25029b8acd05c75146d9246cb8e3d9ceec5deacae4e033"},"rotaryProfile":{"a":"gpio1","b":"gpio2","common":"gnd","steps":24,"triggersPerRotation":4,"driver":"ec11"},"gates":[{"output":"firmware","code":"module-driver","message":"Local driver requires qualification"}]}],"physicalInstances":[],"moduleFindings":[],"embeddedCircuitIds":[]});
    value
}

#[test]
fn public_firmware_export_binds_a_configured_vik_rotary_module_by_its_mount_id() {
    let request: FirmwareRequest = serde_json::from_value(configured_vik_rotary_request()).unwrap();
    let Ok(package) = CoreEngine::generate_firmware(&request) else {
        panic!("vik rotary export")
    };
    let overlay = &package.files["config/boards/shields/boardstudio/boardstudio.overlay"];
    let keymap = &package.files["config/boards/shields/boardstudio/boardstudio.keymap"];
    assert!(overlay.contains("compatible = \"alps,ec11\""), "{overlay}");
    assert!(overlay.contains("steps = <24>"), "{overlay}");
    assert!(keymap.contains("&kp C_VOL_UP"), "{keymap}");
    assert!(keymap.contains("&kp C_VOL_DN"), "{keymap}");
    assert!(
        package
            .warnings
            .iter()
            .any(|warning| warning.contains("rotation only"))
    );
}

#[test]
fn source_qualified_vik_module_cannot_export_without_matching_encoder_node() {
    let mut value = configured_vik_rotary_request();
    value["encoders"] = json!([]);
    let request: FirmwareRequest = serde_json::from_value(value).unwrap();
    let Err(message) = CoreEngine::generate_firmware(&request) else {
        panic!("A module with no generated encoder node was exported")
    };
    assert!(
        message.contains("matching configured rotary encoder"),
        "{message}"
    );

    let mut value = configured_vik_rotary_request();
    value["encoders"][0]["profile"]["steps"] = json!(12);
    let request: FirmwareRequest = serde_json::from_value(value).unwrap();
    let Err(message) = CoreEngine::generate_firmware(&request) else {
        panic!("A module with a mismatched encoder profile was exported")
    };
    assert!(
        message.contains("matching configured rotary encoder"),
        "{message}"
    );
}
