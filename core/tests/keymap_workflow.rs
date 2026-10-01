use boardstudio_core::{CoreEngine, model::*};
use serde_json::{Value, json};

fn firmware_request(keymap: Value) -> CoreRequest {
    serde_json::from_value(json!({"kind":"generate-firmware","id":"keymap-test","request":{
        "controller_profile":"ceoloide/mcu_nice_nano","board_name":"board","mode":"matrix",
        "rows":[{"terminal":"P21","gpio":"P0.31"}],"columns":[{"terminal":"P20","gpio":"P0.29"}],
        "keys":[{"id":"k","row":0,"column":0}],"diode_direction":"col2row","keymap":keymap
    }})).unwrap()
}
fn layered() -> Value {
    json!({"layers":[
        {"id":"base","name":"Base","bindings":{"k":{"kind":"mod-tap","hold":"LSHIFT","tap":"A"}}},
        {"id":"nav","name":"Navigation","bindings":{"k":{"kind":"layer-tap","layerId":"base","tap":"SPACE"}}}
    ],"macros":[]})
}
#[test]
fn public_firmware_export_emits_typed_layers_and_hold_tap_bindings() {
    let reply = CoreEngine::new().handle(firmware_request(layered()));
    let CoreReply::FirmwareGenerated { package, .. } = reply else {
        panic!("{reply:?}")
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
        assert!(!matches!(
            CoreEngine::new().handle(firmware_request(map)),
            CoreReply::FirmwareGenerated { .. }
        ));
    }
}
#[test]
fn macros_waits_and_encoder_actions_are_in_generated_source() {
    let mut map = layered();
    map["macros"] = json!([{"id":"hello","name":"Hello","tapMs":30,"waitMs":5,"steps":[{"kind":"tap","binding":{"kind":"key-press","keycode":"LC(A)"}},{"kind":"wait","ms":100},{"kind":"release","binding":{"kind":"key-press","keycode":"A"}}]}]);
    map["layers"][0]["bindings"]["k"] = json!({"kind":"macro","macroId":"hello"});
    map["layers"][0]["sensors"] = json!({"encoder":{"clockwise":{"kind":"key-press","keycode":"C_VOL_UP"},"counterclockwise":{"kind":"key-press","keycode":"C_VOL_DN"}}});
    let mut request = serde_json::to_value(firmware_request(map)).unwrap();
    request["request"]["encoder_ids"] = json!(["encoder"]);
    let reply = CoreEngine::new().handle(serde_json::from_value(request).unwrap());
    let CoreReply::FirmwareGenerated { package, .. } = reply else {
        panic!("{reply:?}")
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
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc.clone(),
    });
    let reply=engine.handle(serde_json::from_value(json!({"kind":"edit","id":"edit","command":{"baseRevision":0,"transactionId":"binding","phase":"commit","targetIds":["key"],"operation":{"kind":"edit-keymap","change":{"kind":"binding","layerId":"base","keyId":"key","binding":{"kind":"mod-tap","hold":"LSHIFT","tap":"A"}}}}})).unwrap());
    let CoreReply::Scene { document, .. } = reply else {
        panic!("{reply:?}")
    };
    assert_eq!(document.keycaps, doc.keycaps);
    assert!(document.keymap.is_some());
    let decoded: ProjectDoc =
        serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
    assert_eq!(decoded.keymap, document.keymap);
    let mut legacy_engine = CoreEngine::new();
    legacy_engine.handle(CoreRequest::Open {
        id: "reopen".into(),
        document: document.as_ref().clone(),
    });
    let reply = legacy_engine.handle(CoreRequest::Edit {
        id: "legacy".into(),
        command: EditCommand {
            base_revision: document.revision,
            transaction_id: "legacy".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["key".into()],
            operation: EditOperation::SetKeyBinding {
                board_id: "board".into(),
                key_id: "key".into(),
                binding: "&kp B".into(),
            },
        },
    });
    let CoreReply::Scene {
        document: updated, ..
    } = reply
    else {
        panic!("{reply:?}")
    };
    assert_eq!(
        updated.keymap.unwrap().layers[0].bindings["key"],
        KeyBinding::KeyPress {
            keycode: "B".into()
        }
    );

    let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Undo { id: "undo".into() })
    else {
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
    assert!(matches!(
        CoreEngine::new().handle(CoreRequest::Open {
            id: "invalid".into(),
            document: doc
        }),
        CoreReply::Error { .. }
    ));
}
#[test]
fn accepted_large_macros_receive_sufficient_behavior_queue_capacity() {
    let mut map = layered();
    map["macros"] = json!([{"id":"long","name":"Long","tapMs":30,"waitMs":0,"steps":vec![json!({"kind":"tap","binding":{"kind":"key-press","keycode":"A"}});128]}]);
    let CoreReply::FirmwareGenerated { package, .. } =
        CoreEngine::new().handle(firmware_request(map))
    else {
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
            matches!(
                CoreEngine::new().handle(firmware_request(map)),
                CoreReply::FirmwareGenerated { .. }
            ),
            accepted,
            "{count} waits"
        );
    }
}

#[test]
fn encoder_push_without_switches_exports_a_real_direct_scanner() {
    let mut request = serde_json::to_value(firmware_request(layered())).unwrap();
    request["request"]["mode"] = json!("direct");
    request["request"]["rows"] = json!([]);
    request["request"]["columns"] = json!([]);
    request["request"]["auxiliary_pins"] = json!([{"terminal":"P19","gpio":"P0.02"}]);
    request["request"]["keys"] = json!([{"id":"k","row":1,"column":0}]);
    let reply = CoreEngine::new().handle(serde_json::from_value(request).unwrap());
    let CoreReply::FirmwareGenerated { package, .. } = reply else {
        panic!("{reply:?}")
    };
    let overlay = &package.files["config/boards/shields/boardstudio/boardstudio.overlay"];
    assert!(overlay.contains("input-gpios = <&gpio0 2"));
    assert!(!overlay.contains("input-gpios = <>"));
    assert!(overlay.contains("RC(0, 0)"));
}
