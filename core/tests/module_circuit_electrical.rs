use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

const HAPTIC_SOURCE: &str = include_str!(
    "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb"
);

fn request(engine: &mut CoreEngine, value: Value) -> Value {
    serde_json::from_str(&engine.request(&value.to_string())).unwrap()
}

fn imported_haptic(id: &str) -> Value {
    let source = json!({
        "id":"import", "kind":"import-module-board", "definitionId":id,
        "name":"DRV2605L reference circuit", "source":HAPTIC_SOURCE,
        "provenance":{"repository":"https://github.com/sadekbaroudi/vik",
            "revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553",
            "path":"pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb",
            "license":"CERN-OHL-S-2.0"},
        "family":"feedback", "variant":"3V3 pullups with JP1 bridged",
        "repair":"drv2605l-pullups3v3"
    });
    let reply: Value =
        serde_json::from_str(&boardstudio_core::artifact::request(&source.to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    reply["result"].clone()
}

fn host_document() -> Value {
    let mut document = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "host-project",
        "Host",
    ))
    .unwrap();
    document["boards"] = json!([{"id":"host","name":"Host","partIds":[],
        "netIds":["ground","rail-3v3","clock","data"],"outlineIds":[],"thickness":1.6}]);
    document["nets"] = json!([
        {"id":"ground","name":"GND","pins":[]},
        {"id":"rail-3v3","name":"+3V3","pins":[]},
        {"id":"clock","name":"SCL","pins":[]},
        {"id":"data","name":"SDA","pins":[]}
    ]);
    document
}

fn embed(engine: &mut CoreEngine, revision: u64, id: &str, definition: Value) -> Value {
    embed_with_joins(
        engine,
        revision,
        id,
        definition,
        json!({"gnd":"ground","v3v3":"rail-3v3","scl":"clock","sda":"data"}),
    )
}

fn embed_with_joins(
    engine: &mut CoreEngine,
    revision: u64,
    id: &str,
    definition: Value,
    joins: Value,
) -> Value {
    request(
        engine,
        json!({
            "id":format!("embed-{id}"), "kind":"edit", "command":{
                "baseRevision":revision, "transactionId":format!("embed-{id}"),
                "phase":"commit", "targetIds":[id], "operation":{
                "kind":"embed-module-circuit", "id":id, "definition":definition,
                "hostBoardId":"host", "pose":{"at":{"x":0,"y":0},"rotation":0},
                "side":"front", "joins":joins
                }
            }
        }),
    )
}

#[test]
fn repaired_haptic_import_records_the_electrical_profile_needed_for_shared_bus_checks() {
    let definition = imported_haptic("haptic");
    assert_eq!(definition["electrical"]["protocol"], "i2c");
    assert_eq!(
        definition["electrical"]["requiredSignals"],
        json!(["gnd", "v3v3", "scl", "sda"])
    );
    assert_eq!(definition["electrical"]["logicVoltage"], 3.3);
    assert_eq!(definition["electrical"]["i2cAddress"], 90);
    assert_eq!(definition["electrical"]["pullupOhms"], 4700.0);
    assert!(
        !definition["gates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gate| gate["code"] == "pullup-supply")
    );
}

#[test]
fn kiwano_import_marks_only_the_real_vik_module_connector_and_preserves_contact_order() {
    let source =
        include_str!("../../catalogue/modules/sources/Ariamelon-Kiwano/PCB/Kiwano.kicad_pcb");
    let request = json!({
        "id":"kiwano", "kind":"import-module-board", "definitionId":"kiwano",
        "name":"Kiwano", "source":source,
        "provenance":{"repository":"https://github.com/Ariamelon/Kiwano",
            "revision":"1f59c4cd6c829ef414e1f67927971d8f2da21fad",
            "path":"PCB/Kiwano.kicad_pcb", "license":"CERN-OHL-S-2.0"},
        "family":"pointing", "variant":"trackball"
    });
    let reply: Value =
        serde_json::from_str(&boardstudio_core::artifact::request(&request.to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    let definition = &reply["result"];
    let interfaces = definition["interfaces"].as_array().unwrap();
    assert_eq!(
        interfaces.len(),
        1,
        "Cirque is a separate sensor connector, not VIK"
    );
    assert_eq!(interfaces[0]["id"], "J1");
    assert_eq!(interfaces[0]["role"], "module");
    assert_eq!(
        interfaces[0]["signals"],
        json!([
            "sclk", "miso", "cs", "gpio2", "mosi", "gpio1", "v5", "rgb", "scl", "sda", "gnd",
            "v3v3"
        ])
    );
    let sclk = definition["circuit"]["ports"]["sclk"].as_str().unwrap();
    let sclk_net = definition["circuit"]["nets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|net| net["id"] == sclk)
        .unwrap();
    assert_eq!(sclk_net["name"], "SCK");
    assert!(!interfaces.iter().any(|interface| interface["id"] == "J2"));
}

#[test]
fn two_repaired_haptics_on_the_same_nets_report_the_source_derived_parallel_pullup_value() {
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":host_document()}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let definition = imported_haptic("haptic");
    let first = embed(&mut engine, 0, "first", definition.clone());
    assert_eq!(first["kind"], "scene", "{first}");
    let second = embed(&mut engine, 1, "second", definition);
    assert_eq!(second["kind"], "scene", "{second}");
    let finding = second["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"].as_str().unwrap().ends_with("/i2c-pullups"))
        .expect("Shared source pull-ups need a review finding");
    assert_eq!(finding["severity"], "warning");
    assert!(finding["message"].as_str().unwrap().contains("2350"));
    assert!(
        finding["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("first"))
    );
    assert!(
        finding["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("second"))
    );
}

#[test]
fn embedded_circuit_requires_explicit_host_signal_and_matching_power_rail_joins() {
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":host_document()}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let unjoined = embed_with_joins(
        &mut engine,
        0,
        "unjoined",
        imported_haptic("haptic"),
        json!({}),
    );
    assert_eq!(unjoined["kind"], "scene", "{unjoined}");
    assert!(
        unjoined["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["id"].as_str().unwrap().ends_with("/unjoined/v3v3") }),
        "findings: {:?}",
        unjoined["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| &f["id"])
            .collect::<Vec<_>>()
    );
    assert!(
        unjoined["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["id"].as_str().unwrap().ends_with("/unjoined/scl") })
    );

    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":host_document()}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let crossed_rails = embed_with_joins(
        &mut engine,
        0,
        "crossed",
        imported_haptic("haptic"),
        json!({"gnd":"rail-3v3","v3v3":"ground","scl":"clock","sda":"data"}),
    );
    assert_eq!(crossed_rails["kind"], "scene", "{crossed_rails}");
    assert!(
        crossed_rails["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["id"].as_str().unwrap().ends_with("/rail/gnd") })
    );
    assert!(
        crossed_rails["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| { finding["id"].as_str().unwrap().ends_with("/rail/v3v3") })
    );
}

#[test]
fn modules_on_one_host_connector_cannot_assign_a_shared_contact_to_different_mcu_pins() {
    let mut document = host_document();
    document["boards"][0]["partIds"] = json!(["mcu", "connector"]);
    document["definitions"] = json!([
        {"id":"mcu-definition","name":"Controller","kind":"controller",
            "courtyard":[],"pads":[],"terminals":{"P0":["pad-0"],"P1":["pad-1"]}},
        {"id":"vik-host-definition","name":"VIK host","kind":"connector",
            "courtyard":[],"pads":[],"terminals":{},
            "hardwareProfile":{"source":{"repository":"https://github.com/sadekbaroudi/vik",
                "revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553",
                "path":"kicad/vik.pretty/vik-keyboard-connector-horizontal.kicad_mod",
                "license":"CERN-OHL-P-2.0"},"gates":[],"vikRole":"host",
                "footprintSurfaceVolumes":true}}
    ]);
    document["parts"] = json!([
        {"id":"mcu","definitionId":"mcu-definition","reference":"U1",
            "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},
        {"id":"connector","definitionId":"vik-host-definition","reference":"J1",
            "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"}
    ]);
    let module_definition = json!({
        "id":"i2c-module","name":"I2C module","family":"test","variant":"source",
        "source":{"repository":"https://example.invalid/module","revision":"pinned",
            "path":"module.kicad_pcb","license":"MIT"},
        "board":{"contours":[{"hole":false,"points":[{"x":-5,"y":-5},{"x":5,"y":-5},{"x":5,"y":5},{"x":-5,"y":5}]}],"thickness":1.6},
        "mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],
        "interfaces":[{"id":"input","role":"module","signals":["sclk","miso","cs","gpio2","mosi","gpio1","v5","rgb","scl","sda","gnd","v3v3"]}],
        "electrical":{"protocol":"i2c","requiredSignals":["scl","sda"],"logicVoltage":3.3},
        "constituents":[]
    });
    document["moduleDefinitions"] = json!([module_definition]);
    let connection = |scl, sda| {
        json!({
            "hostConnectorPartId":"connector","modulePortId":"input","busId":"i2c",
            "assignments":{"scl":scl,"sda":sda},"cableType":"type-a-12-0.5"
        })
    };
    let mounted = |id, scl, sda| {
        json!({
            "id":id,"definitionId":"i2c-module","hostBoardId":"host",
            "hostFace":"front","facingFace":"back","at":{"x":0,"y":0},
            "rotation":0,"gap":3,"attachment":"board",
            "connection":connection(scl,sda)
        })
    };
    document["modules"] = json!([
        mounted("module-a", "P1", "P0"),
        mounted("module-b", "P0", "P1")
    ]);
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let conflict = opened["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["id"]
                .as_str()
                .unwrap()
                .ends_with("/host-contact-assignment/scl")
        })
        .expect("Two copies cannot describe different maps for the same host connector contact");
    assert!(
        conflict["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("module-a"))
    );
    assert!(
        conflict["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("module-b"))
    );
}

#[test]
fn removing_an_embedded_circuit_preserves_existing_host_net_copper() {
    let mut document = host_document();
    document["boards"][0]["traces"] = json!([{
        "id":"host-clock-trace", "start":{"x":0,"y":0}, "end":{"x":10,"y":0},
        "width":0.25, "layer":"front", "netId":"clock"
    }]);
    document["boards"][0]["vias"] = json!([{
        "id":"host-clock-via", "at":{"x":10,"y":0}, "size":0.6,
        "drill":0.3, "netId":"clock"
    }]);
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let mut definition = imported_haptic("haptic");
    definition["electrical"] = json!({"protocol":"nonstandard","requiredSignals":[]});
    let embedded = embed(&mut engine, 0, "copy", definition);
    assert_eq!(embedded["kind"], "scene", "{embedded}");
    let removed = request(
        &mut engine,
        json!({
            "id":"remove", "kind":"edit", "command":{
                "baseRevision":1,"transactionId":"remove","phase":"commit","targetIds":["copy"],
                "operation":{"kind":"remove-embedded-circuit","id":"copy"}
            }
        }),
    );
    assert_eq!(removed["kind"], "scene", "{removed}");
    assert!(
        removed["document"]["boards"][0]["netIds"]
            .as_array()
            .unwrap()
            .contains(&json!("clock"))
    );
    assert!(
        removed["document"]["nets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|net| net["id"] == "clock")
    );
    assert!(
        removed["document"]["boards"][0]["traces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|trace| trace["id"] == "host-clock-trace")
    );
    assert!(
        removed["document"]["boards"][0]["vias"]
            .as_array()
            .unwrap()
            .iter()
            .any(|via| via["id"] == "host-clock-via")
    );
}

#[test]
fn circuit_ports_must_resolve_to_real_source_nets_before_embedding() {
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":host_document()}),
    );
    assert_eq!(opened["kind"], "scene", "{opened}");
    let mut definition = imported_haptic("haptic");
    definition["circuit"]["ports"]["scl"] = json!("missing-local-net");
    let reply = embed(&mut engine, 0, "invalid", definition);
    assert_eq!(
        reply["kind"], "error",
        "Broken source port must not disappear during embedding: {reply}"
    );
    let accepted = embed(&mut engine, 0, "invalid", imported_haptic("haptic"));
    assert_eq!(
        accepted["kind"], "scene",
        "Rejected source must not consume the document revision: {accepted}"
    );
}
