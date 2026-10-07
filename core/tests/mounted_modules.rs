use boardstudio_core::CoreEngine;
use serde_json::{Value, json};

fn document() -> Value {
    let mut doc = serde_json::to_value(boardstudio_core::model::ProjectDoc::empty(
        "modules", "Modules",
    ))
    .unwrap();
    doc["boards"] = json!([{"id":"host","name":"Host","partIds":[],"netIds":[],"outlineIds":["outline"],"thickness":1.6}]);
    doc["outline"] = json!([{"kind":"rect","id":"outline","center":{"x":0,"y":0},"size":{"x":80,"y":80},"radius":0,"operation":"add"}]);
    doc
}

fn definition() -> Value {
    json!({"id":"reference","name":"Reference daughterboard","family":"expansion","variant":"bare board",
        "source":{"repository":"https://github.com/sadekbaroudi/vik","revision":"cd5d16e4cd9137a229fc673412a89d75f4e64553","path":"pcb/vik-splitter","license":"CC-BY-SA-4.0"},
        "board":{"thickness":1.6,"contours":[{"hole":false,"points":[{"x":-12.5,"y":-12.5},{"x":12.5,"y":-12.5},{"x":12.5,"y":12.5},{"x":-12.5,"y":12.5}]}]},
        "mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],
        "interfaces":[{"id":"input","role":"module","signals":["sclk","miso","cs","gpio2","mosi","gpio1","v5","rgb","scl","sda","gnd","v3v3"]}],
        "electrical":{"protocol":"pass-through","requiredSignals":[],"logicVoltage":3.3},"constituents":[]
    })
}

fn request(engine: &mut CoreEngine, value: Value) -> Value {
    serde_json::from_str(&engine.request(&value.to_string())).unwrap()
}

fn placement(face: &str, facing: &str, gap: f64) -> Value {
    json!({"id":"module-1","definitionId":"reference","hostBoardId":"host","hostFace":face,"facingFace":facing,
        "at":{"x":15,"y":7},"rotation":30,"gap":gap,"attachment":"board"})
}

fn haptic() -> Value {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb"
    );
    let reply:Value=serde_json::from_str(&boardstudio_core::artifact::request(&json!({"id":"haptic","kind":"import-module-board",
        "definitionId":"vik:haptic","name":"DRV2605L","source":source,"provenance":definition()["source"],"family":"feedback","variant":"source"}).to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    reply["result"].clone()
}

#[test]
fn haptic_pullup_repair_is_an_explicit_source_preserving_variant() {
    let original = haptic();
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/haptic-drv2605l/haptic-drv2605l.kicad_pcb"
    );
    let reply: Value = serde_json::from_str(&boardstudio_core::artifact::request(&json!({
        "id":"repair", "kind":"import-module-board", "definitionId":"vik:haptic:repaired",
        "name":"DRV2605L · repaired pullups", "source":source, "provenance":definition()["source"],
        "family":"feedback", "variant":"3V3 pullups with JP1 bridged", "repair":"drv2605l-pullups3v3"
    }).to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    let repaired = &reply["result"];
    let nets = repaired["circuit"]["nets"].as_array().unwrap();
    let supply = nets.iter().find(|net| net["name"] == "+3V3").unwrap();
    for reference in ["R1", "R2", "JP1"] {
        assert!(
            supply["pins"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pin| pin["partId"] == reference),
            "{reference} pullup supply must reach 3.3V"
        );
    }
    assert!(
        !nets
            .iter()
            .any(|net| net["name"] == "VCC" || net["name"] == "I2C_3V3")
    );
    assert_eq!(repaired["source"]["sha256"], original["source"]["sha256"]);
    assert!(
        !repaired["circuit"]["adaptations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        original["circuit"]["nets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|net| net["name"] == "VCC")
    );
}

#[test]
fn invalid_model_scale_cannot_enter_a_project_module_snapshot() {
    for scale in [0.0, -1.0] {
        let mut engine = CoreEngine::new();
        request(
            &mut engine,
            json!({"id":"open","kind":"open","document":document()}),
        );
        let mut invalid = definition();
        invalid["models"] = json!([{"assetId":"source.step","offset":{"x":0,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0},"scale":{"x":scale,"y":1,"z":1}}]);
        let reply = request(
            &mut engine,
            json!({"id":"invalid","kind":"edit","command":{"baseRevision":0,"transactionId":"invalid","phase":"commit","targetIds":[],"operation":{"kind":"set-module-definition","definition":invalid}}}),
        );
        assert_eq!(
            reply["kind"], "error",
            "Invalid scale must leave the document unchanged: {reply}"
        );
    }
}

#[test]
fn removing_unknown_assembly_gate_requires_reviewed_geometry() {
    let mut engine = CoreEngine::new();
    request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document()}),
    );
    let mut gated = definition();
    gated["gates"] = json!([{"output":"mechanical","code":"assembled-envelope","message":"Assembly dimensions remain unknown"}]);
    let before = request(
        &mut engine,
        json!({"id":"attach","kind":"edit","command":{"baseRevision":0,"transactionId":"attach","phase":"commit","targetIds":[],"operation":{"kind":"set-mounted-module","instance":placement("front","back",3.0),"definition":gated}}}),
    );
    assert_eq!(before["kind"], "scene");
    gated["gates"] = json!([]);
    let reply = request(
        &mut engine,
        json!({"id":"clear","kind":"edit","command":{"baseRevision":1,"transactionId":"clear","phase":"commit","targetIds":[],"operation":{"kind":"set-module-definition","definition":gated}}}),
    );
    assert_eq!(
        reply["kind"], "error",
        "Missing assembly dimensions cannot become a qualified bare PCB"
    );
    let undo = request(&mut engine, json!({"id":"undo","kind":"undo"}));
    assert_eq!(undo["document"]["revision"], 2);
    assert!(
        undo["document"]
            .get("modules")
            .is_none_or(|m| m.as_array().unwrap().is_empty())
    );
}

#[test]
fn project_module_profiles_are_editable_and_undoable_without_rewriting_provenance() {
    let mut engine = CoreEngine::new();
    request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document()}),
    );
    let before = request(
        &mut engine,
        json!({"id":"attach","kind":"edit","command":{"baseRevision":0,"transactionId":"attach","phase":"commit","targetIds":[],"operation":{"kind":"set-mounted-module","instance":placement("front","back",3.0),"definition":definition()}}}),
    );
    let mut changed = before["document"]["moduleDefinitions"][0].clone();
    changed["volumes"] = json!([{"id":"body","purpose":"occupied","source":"Measured actuator drawing, revision A","qualified":true,"geometry":{"points":[{"x":-5,"y":-3},{"x":5,"y":-3},{"x":5,"y":3},{"x":-5,"y":3}],"z":0.8,"height":2.0}}]);
    let edited = request(
        &mut engine,
        json!({"id":"profile","kind":"edit","command":{"baseRevision":1,"transactionId":"profile","phase":"commit","targetIds":["reference"],"operation":{"kind":"set-module-definition","definition":changed}}}),
    );
    assert_eq!(edited["kind"], "scene", "{edited}");
    assert_eq!(
        edited["scene"]["moduleScenes"][0]["volumes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let mut invalid = changed.clone();
    invalid["source"]["revision"] = json!("unrelated-new-source");
    let rejected = request(
        &mut engine,
        json!({"id":"invalid","kind":"edit","command":{"baseRevision":2,"transactionId":"invalid","phase":"commit","targetIds":[],"operation":{"kind":"set-module-definition","definition":invalid}}}),
    );
    assert_eq!(rejected["kind"], "error");
    let undo = request(&mut engine, json!({"id":"undo","kind":"undo"}));
    assert_eq!(
        undo["document"]["moduleDefinitions"],
        before["document"]["moduleDefinitions"]
    );
}

#[test]
fn module_preview_uses_the_resolved_frame_and_preserves_board_holes() {
    let mut doc = document();
    let mut def = definition();
    def["mounts"] = json!([{"sourceId":"MH1","at":{"x":0,"y":0},"diameter":2.2}]);
    def["models"] = json!([{"assetId":"model","offset":{"x":2,"y":3,"z":4},"rotation":{"x":0,"y":0,"z":0},"scale":{"x":1,"y":1,"z":1}}]);
    doc["moduleDefinitions"] = json!([def]);
    let mut instance = placement("back", "back", 3.0);
    instance["rotation"] = json!(0);
    doc["modules"] = json!([instance]);
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"preview","kind":"resolve-modules","document":doc,"boardId":"host","previewTopZ":1.6}),
    );
    assert_eq!(reply["kind"], "modules-resolved", "{reply}");
    let preview = &reply["result"]["preview"];
    let body = &preview["bodies"][0];
    assert!((body["body"]["z"].as_f64().unwrap() + 4.6).abs() < 1e-9);
    assert_eq!(body["regions"][0]["holes"].as_array().unwrap().len(), 1);
    let transform = reply["result"]["modelPlacements"][0]["matrix"]
        .as_array()
        .unwrap();
    // Independent rigid-frame expectation: flipped X/Z, translated host-top frame + view offset.
    assert_eq!(transform[0], -1.0);
    assert_eq!(transform[5], 1.0);
    assert_eq!(transform[10], -1.0);
    assert_eq!(transform[12], 13.0);
    assert_eq!(transform[13], 10.0);
    assert!((transform[14].as_f64().unwrap() + 7.8).abs() < 1e-9);
}

#[test]
fn two_embedded_circuits_own_their_definitions_and_internal_nets_and_undo_atomically() {
    let mut engine = CoreEngine::new();
    request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document()}),
    );
    let circuit = haptic();
    let first = request(
        &mut engine,
        json!({"id":"first","kind":"edit","command":{"baseRevision":0,"transactionId":"first","phase":"commit","targetIds":["first"],
        "operation":{"kind":"embed-module-circuit","id":"first","definition":circuit,"hostBoardId":"host","pose":{"at":{"x":-20,"y":0},"rotation":90},"side":"front","joins":{}}}}),
    );
    assert_eq!(first["kind"], "scene", "{first}");
    let second = request(
        &mut engine,
        json!({"id":"second","kind":"edit","command":{"baseRevision":1,"transactionId":"second","phase":"commit","targetIds":["second"],
        "operation":{"kind":"embed-module-circuit","id":"second","definition":circuit,"hostBoardId":"host","pose":{"at":{"x":20,"y":0},"rotation":0},"side":"back","joins":{}}}}),
    );
    assert_eq!(second["kind"], "scene", "{second}");
    let doc = &second["document"];
    let parts = doc["parts"].as_array().unwrap();
    let one = parts
        .iter()
        .find(|p| p["id"] == "embedded/first/U1")
        .unwrap();
    let two = parts
        .iter()
        .find(|p| p["id"] == "embedded/second/U1")
        .unwrap();
    assert_ne!(one["id"], two["id"]);
    assert_ne!(one["definitionId"], two["definitionId"]);
    assert_ne!(
        one["reference"], two["reference"],
        "Host PCB references must be unique across circuit copies"
    );
    assert_eq!(one["properties"]["moduleSourceReference"], "U1");
    assert_eq!(two["properties"]["moduleSourceReference"], "U1");
    let references: std::collections::BTreeSet<_> = parts
        .iter()
        .map(|p| p["reference"].as_str().unwrap())
        .collect();
    assert_eq!(references.len(), parts.len());
    let nets = doc["nets"].as_array().unwrap();
    for prefix in ["embedded/first/", "embedded/second/"] {
        let vdd = nets
            .iter()
            .find(|n| n["id"].as_str().unwrap().starts_with(prefix) && n["name"] == "+3V3")
            .unwrap();
        // Independent source pin expectations: U1 pin10, C1 pin1 and U1 EN pin5.
        let u1id = if prefix.contains("first") {
            &one["definitionId"]
        } else {
            &two["definitionId"]
        };
        let u1def = doc["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| &d["id"] == u1id)
            .unwrap();
        for number in ["5", "10"] {
            let physical = &u1def["terminals"][number][0];
            assert!(
                vdd["pins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["partId"] == format!("{prefix}U1") && p["padId"] == *physical)
            );
        }
        assert!(
            vdd["pins"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["partId"].as_str().unwrap().starts_with(prefix))
        );
    }
    assert!(
        doc.get("modules")
            .is_none_or(|m| m.as_array().unwrap().is_empty())
    );
    let undone = request(&mut engine, json!({"id":"undo","kind":"undo"}));
    assert_eq!(undone["document"]["parts"], first["document"]["parts"]);
    assert_eq!(undone["document"]["nets"], first["document"]["nets"]);
}

#[test]
fn copying_the_unrepaired_haptic_retains_its_precise_electrical_blocker() {
    let mut engine = CoreEngine::new();
    request(
        &mut engine,
        json!({"id":"open","kind":"open","document":document()}),
    );
    let reply = request(
        &mut engine,
        json!({"id":"copy","kind":"edit","command":{"baseRevision":0,"transactionId":"copy","phase":"commit","targetIds":[],"operation":{"kind":"embed-module-circuit","id":"source","definition":haptic(),"hostBoardId":"host","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front","joins":{}}}}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let finding = reply["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"].as_str().unwrap().ends_with("pullup-supply"))
        .expect("Copying a circuit must not erase the source pullup defect");
    assert_eq!(finding["scope"], "pcb");
    assert_eq!(finding["severity"], "error");
    let marker = reply["scene"]["findingMarkers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|marker| marker["findingId"] == finding["id"])
        .unwrap();
    assert_eq!(
        marker["contours"].as_array().unwrap().len(),
        3,
        "Focus R1, R2 and JP1 rather than the whole copied circuit"
    );
}

#[test]
fn catalogue_retains_all_twenty_nine_rows_and_real_source_variants() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../catalogue/modules/imported-modules.json");
    let catalogue: Value = serde_json::from_str(
        &std::fs::read_to_string(file).expect("Generated module catalogue is missing"),
    )
    .unwrap();
    let entries = catalogue["modules"].as_array().unwrap();
    let rows: std::collections::BTreeSet<_> =
        entries.iter().map(|e| e["row"].as_str().unwrap()).collect();
    assert_eq!(rows.len(), 29);
    assert!(entries.len() >= 36);
    assert!(
        entries
            .iter()
            .all(|e| e["definition"]["source"]["sha256"].as_str().unwrap().len() == 64)
    );
    assert!(
        rows.contains("vik-splitter") && rows.contains("nice!view") && rows.contains("pmw3610-xxs")
    );
    let splitter = entries.iter().find(|e| e["row"] == "vik-splitter").unwrap();
    assert_eq!(
        splitter["definition"]["source"]["upstreamStatus"],
        "Untested"
    );
    assert!(
        splitter["definition"]["gates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["output"] == "mechanical")
    );
}

#[test]
fn modules_on_the_same_physical_i2c_contacts_report_address_conflicts_with_local_markers() {
    let mut doc = document();
    let mut def = definition();
    def["electrical"] = json!({"protocol":"i2c","requiredSignals":["gnd","v3v3","scl","sda"],"logicVoltage":3.3,"i2cAddress":90,"pullupOhms":4700});
    doc["moduleDefinitions"] = json!([def]);
    let mut a = placement("front", "back", 3.0);
    let mut b = a.clone();
    a["at"] = json!({"x":-20,"y":0});
    b["at"] = json!({"x":20,"y":0});
    b["id"] = json!("module-2");
    a["connection"] = json!({"hostConnectorPartId":"connector","modulePortId":"input","busId":"bus-a","assignments":{"scl":"P1","sda":"P0","v3v3":"VCC","gnd":"GND"},"cableType":"type-a-12-0.5"});
    b["connection"] = a["connection"].clone();
    b["connection"]["busId"] = json!("different-label-same-wires");
    doc["modules"] = json!([a, b]);
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":doc}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let finding = reply["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"].as_str().unwrap().ends_with("/i2c-address"))
        .expect("Two 0x5A devices on the same wires must conflict");
    assert!(
        finding["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("module-1"))
            && finding["targetIds"]
                .as_array()
                .unwrap()
                .contains(&json!("module-2"))
    );
    let marker = reply["scene"]["findingMarkers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["findingId"] == finding["id"])
        .expect("Conflict must focus the two modules");
    assert_eq!(marker["contours"].as_array().unwrap().len(), 2);
    assert!(marker["contours"].as_array().unwrap().iter().all(|c| {
        c["points"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["y"].as_f64().unwrap().abs() < 18.0)
    }));
    let mut scoped = reply["document"].clone();
    scoped["physicalInstanceId"] = json!("left");
    scoped["modules"][0]["hostInstanceId"] = json!("left");
    scoped["modules"][1]["hostInstanceId"] = json!("right");
    scoped["hardware"] = json!({"topology":"split","transport":"wireless","boards":[],"instances":[{"id":"left","name":"Left","boardId":"host","half":"left","role":"central","flipped":false,"constructionLinked":false},{"id":"right","name":"Right","boardId":"host","half":"right","role":"peripheral","flipped":false,"constructionLinked":false}]});
    let isolated = request(
        &mut engine,
        json!({"id":"scoped","kind":"open","document":scoped}),
    );
    assert!(
        !isolated["scene"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"].as_str().unwrap().ends_with("/i2c-address")),
        "Separate physical MCU instances must not share an address ledger"
    );
    scoped.as_object_mut().unwrap().remove("physicalInstanceId");
    scoped["definitions"] = json!([{"id":"controller","name":"Controller","kind":"controller","courtyard":[],"pads":[],"terminals":{"P1":["clock"],"P0":["data"]}}]);
    scoped["parts"] = json!([{"id":"mcu","definitionId":"controller","reference":"U1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"}]);
    scoped["boards"][0]["partIds"] = json!(["mcu"]);
    let plan = request(
        &mut engine,
        json!({"id":"wiring","kind":"resolve-electrical","request":{"document":scoped,"boardId":"host","instanceId":"left","mode":"matrix","locks":{},"controllerPartId":"mcu","controllerProfile":"ceoloide/mcu_nice_nano"}}),
    );
    assert_eq!(plan["kind"], "electrical-resolved", "{plan}");
    assert_eq!(plan["plan"]["controllerPartId"], "mcu");
    assert!(
        !plan["plan"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["code"].as_str().unwrap().ends_with("/i2c-address")),
        "The public electrical request must scope modules by its validated physical instance"
    );
}

#[test]
fn private_module_contacts_use_actual_wires_across_mounted_and_embedded_devices() {
    let mut doc = document();
    let mut mounted = definition();
    mounted["electrical"] = json!({"protocol":"spi","requiredSignals":["cs"],"logicVoltage":3.3});
    doc["moduleDefinitions"] = json!([mounted]);
    doc["definitions"] = json!([{"id":"mcu","name":"Controller","kind":"controller","courtyard":[],"pads":[],"terminals":{"P1":["first"],"P0":["second"]}}]);
    doc["parts"] = json!([{"id":"mcu","definitionId":"mcu","reference":"U1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"}]);
    doc["boards"][0]["partIds"] = json!(["mcu"]);
    doc["boards"][0]["netIds"] = json!(["shared-wire"]);
    doc["nets"] = json!([{"id":"shared-wire","name":"CS","pins":[{"partId":"mcu","padId":"first"},{"partId":"mcu","padId":"second"}]}]);
    let mut first = placement("front", "back", 3.0);
    first["connection"] = json!({"hostConnectorPartId":"connector","modulePortId":"input","busId":"first","assignments":{"cs":"P1"},"cableType":"type-a-12-0.5"});
    let mut second = first.clone();
    second["id"] = json!("module-2");
    second["connection"]["assignments"]["cs"] = json!("P0");
    doc["modules"] = json!([first, second]);
    let mut engine = CoreEngine::new();
    let opened = request(
        &mut engine,
        json!({"id":"open","kind":"open","document":doc}),
    );
    let conflict = opened["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["id"]
                .as_str()
                .unwrap()
                .ends_with("/shared-private-contact")
        })
        .expect("Different terminal names joined to one wire cannot provide independent CS");
    for id in ["module-1", "module-2"] {
        assert!(
            conflict["targetIds"]
                .as_array()
                .unwrap()
                .contains(&json!(id))
        );
    }
    doc["modules"].as_array_mut().unwrap().pop();
    request(
        &mut engine,
        json!({"id":"reopen","kind":"open","document":doc}),
    );
    let mut embedded = haptic();
    embedded["electrical"] = json!({"protocol":"spi","requiredSignals":["cs"],"logicVoltage":3.3});
    embedded["circuit"]["ports"]["cs"] = embedded["circuit"]["ports"]["scl"].clone();
    let copied = request(
        &mut engine,
        json!({"id":"copy","kind":"edit","command":{"baseRevision":0,"transactionId":"copy","phase":"commit","targetIds":[],"operation":{"kind":"embed-module-circuit","id":"driver","definition":embedded,"hostBoardId":"host","pose":{"at":{"x":-20,"y":0},"rotation":0},"side":"front","joins":{"cs":"shared-wire"}}}}),
    );
    assert_eq!(copied["kind"], "scene", "{copied}");
    let conflict = copied["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["id"]
                .as_str()
                .unwrap()
                .ends_with("/shared-private-contact")
        })
        .expect("Embedded and mounted devices cannot privately consume one CS wire");
    for id in ["module-1", "driver"] {
        assert!(
            conflict["targetIds"]
                .as_array()
                .unwrap()
                .contains(&json!(id))
        );
    }
}

#[test]
fn mounted_and_embedded_fixed_addresses_share_the_actual_wire_ledger() {
    let mut doc = document();
    let mut mounted = definition();
    mounted["electrical"] = json!({"protocol":"i2c","requiredSignals":["scl","sda"],"logicVoltage":3.3,"i2cAddress":90});
    doc["moduleDefinitions"] = json!([mounted]);
    doc["definitions"] = json!([{"id":"mcu","name":"Controller","kind":"controller","courtyard":[],"pads":[],"terminals":{"P1":["clock"],"P0":["data"]}}]);
    doc["parts"] = json!([{"id":"mcu","definitionId":"mcu","reference":"U1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"}]);
    doc["boards"][0]["partIds"] = json!(["mcu"]);
    doc["boards"][0]["netIds"] = json!(["clock-net", "data-net"]);
    doc["nets"] = json!([{"id":"clock-net","name":"SCL","pins":[{"partId":"mcu","padId":"clock"}]},{"id":"data-net","name":"SDA","pins":[{"partId":"mcu","padId":"data"}]}]);
    let mut module = placement("front", "back", 3.0);
    module["connection"] = json!({"hostConnectorPartId":"connector","modulePortId":"input","busId":"local","assignments":{"scl":"P1","sda":"P0"},"cableType":"type-a-12-0.5"});
    doc["modules"] = json!([module]);
    let mut engine = CoreEngine::new();
    request(
        &mut engine,
        json!({"id":"open","kind":"open","document":doc}),
    );
    let mut driver = haptic();
    driver["electrical"]["i2cAddress"] = json!(90);
    let reply = request(
        &mut engine,
        json!({"id":"copy","kind":"edit","command":{"baseRevision":0,"transactionId":"copy","phase":"commit","targetIds":[],"operation":{"kind":"embed-module-circuit","id":"driver","definition":driver,"hostBoardId":"host","pose":{"at":{"x":-20,"y":0},"rotation":0},"side":"front","joins":{"scl":"clock-net","sda":"data-net"}}}}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let finding = reply["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"].as_str().unwrap().ends_with("/i2c-address"))
        .expect("Mounted and embedded 0x5A devices on the same wires must conflict");
    assert!(
        finding["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("module-1"))
    );
    assert!(
        finding["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("driver"))
    );
}

#[test]
fn local_firmware_export_rejects_an_unqualified_module_function() {
    let mut doc = document();
    let mut def = definition();
    def["electrical"]["protocol"] = json!("spi");
    doc["moduleDefinitions"] = json!([def]);
    doc["modules"] = json!([placement("front", "back", 3.0)]);
    let mut engine = CoreEngine::new();
    let mut payload = json!({"id":"firmware","kind":"generate-firmware","request":{
        "controller_profile":"ceoloide/mcu_nice_nano","board_name":"Module keyboard","rows":[{"terminal":"P1","gpio":"P0.06"}],"columns":[{"terminal":"P0","gpio":"P0.08"}],
        "keys":[{"id":"key","row":0,"column":0}],"diode_direction":"row2col","key_bindings":["&kp A"],"hardware":{
            "boardId":"host","physicalInstanceId":null,"physicalInstances":[],"moduleFindings":[],"embeddedCircuitIds":[],
            "modules":[{"id":"front","name":"Test module","hostInstanceId":null,"protocol":"spi","gates":[]}]
        }
    }});
    let reply = request(&mut engine, payload.clone());
    assert_eq!(
        reply["kind"], "error",
        "An unqualified module must not silently disappear from functional firmware output"
    );
    assert!(reply["message"].as_str().unwrap().contains("driver"));
    let hardware = &mut payload["request"]["hardware"];
    hardware["physicalInstanceId"] = json!("missing");
    hardware["physicalInstances"] = json!([{"id":"left","boardId":"host"}]);
    hardware["modules"][0]["hostInstanceId"] = json!("left");
    let invalid = request(&mut engine, payload.clone());
    assert_eq!(
        invalid["kind"], "error",
        "An invalid physical context cannot hide unqualified module functions"
    );
    assert!(invalid["message"].as_str().unwrap().contains("physical"));
    payload["request"]["hardware"]
        .as_object_mut()
        .unwrap()
        .remove("physicalInstanceId");
    let unscoped = request(&mut engine, payload);
    assert_eq!(unscoped["kind"], "error");
    assert!(unscoped["message"].as_str().unwrap().contains("physical"));
}

#[test]
fn case_preparation_accounts_for_the_same_below_module_extent_and_function_opening() {
    let mut doc = document();
    let mut def = definition();
    def["openings"] = json!([{"id":"access","geometry":{"points":[{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}],"z":-8,"height":20},"purpose":"service","source":"fixture","qualified":true}]);
    doc["moduleDefinitions"] = json!([def]);
    let mut instance = placement("back", "front", 3.0);
    instance["at"] = json!({"x":0,"y":0});
    instance["rotation"] = json!(0);
    instance["serviceClearance"] = json!(1.0);
    doc["modules"] = json!([instance]);
    doc["mechanical"] = json!({"boardId":"host","method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell",
        "plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,
        "plateToPcb":3.5,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});
    let contours = json!([{"hole":false,"points":[{"x":-40,"y":-40},{"x":40,"y":-40},{"x":40,"y":40},{"x":-40,"y":40}]}]);
    let mut engine = CoreEngine::new();
    let resolved = request(
        &mut engine,
        json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":contours}),
    );
    assert_eq!(resolved["kind"], "mechanical-resolved", "{resolved}");
    let bottom = resolved["assembly"]["case"]["bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["body"]["id"] == "bottom")
        .unwrap();
    assert!(
        (bottom["body"]["z"].as_f64().unwrap() + 9.2).abs() < 1e-9,
        "Module bottom −6.2, authored service 1 and bottom thickness 2 must produce bottom −9.2"
    );
    let opening = bottom["body"]["openings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["points"].as_array().is_some_and(|p| p.len() == 4))
        .unwrap();
    let points = opening["points"].as_array().unwrap();
    assert!(
        points
            .iter()
            .all(|p| (p["x"].as_f64().unwrap().abs() - 3.0).abs() < 1e-9
                && (p["y"].as_f64().unwrap().abs() - 3.0).abs() < 1e-9)
    );
    assert!(
        (resolved["assembly"]["modules"][0]["board"][0]["z"]
            .as_f64()
            .unwrap()
            + 6.2)
            .abs()
            < 1e-9
    );
    assert_eq!(doc["outline"][0]["size"], json!({"x":80,"y":80}));
    let prepared = request(
        &mut engine,
        json!({"id":"prepare","kind":"prepare-case","ir":resolved["assembly"]["case"]}),
    );
    assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
    assert_eq!(prepared["ir"]["revision"], doc["revision"]);
}

#[test]
fn module_clearance_cannot_pass_through_solid_case_material() {
    let contours = json!([{"hole":false,"points":[{"x":-40,"y":-40},{"x":40,"y":-40},{"x":40,"y":40},{"x":-40,"y":40}]}]);
    for (face, x, clearance, aperture, boss, blocked) in [
        ("front", 0.0, 0.0, false, false, true),
        ("front", 0.0, 0.0, true, false, false),
        ("back", 0.0, 0.0, false, false, false),
        ("back", 27.0, 1.0, false, false, true),
        ("back", 0.0, 0.0, false, true, true),
    ] {
        let mut doc = document();
        let mut def = definition();
        if aperture {
            def["openings"] = json!([{"id":"plate-access","purpose":"opening","source":"Measured test aperture","qualified":true,"geometry":{"points":[{"x":-14,"y":-14},{"x":14,"y":-14},{"x":14,"y":14},{"x":-14,"y":14}],"z":-2,"height":6}}]);
        }
        doc["moduleDefinitions"] = json!([def]);
        let mut instance = placement(face, "back", 3.0);
        instance["at"] = json!({"x":x,"y":0});
        instance["rotation"] = json!(0);
        instance["serviceClearance"] = json!(clearance);
        doc["modules"] = json!([instance]);
        doc["mechanical"] = json!({"boardId":"host","method":"printed","mount":"rigid","integratedPlateFrame":false,"bottomStyle":"shell",
            "plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,
            "plateToPcb":3.5,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});
        if boss {
            doc["mechanical"]["mounts"] = json!([{"id":"user-boss","at":{"x":0,"y":0},"kind":"boss","holeDiameter":2.2,"bossDiameter":6,"height":8}]);
        }
        let reply = request(
            &mut CoreEngine::new(),
            json!({"id":"case","kind":"resolve-mechanical","document":doc,"contours":contours}),
        );
        assert_eq!(reply["kind"], "mechanical-resolved", "{reply}");
        let collisions = reply["assembly"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["id"].as_str().unwrap().contains("case-material"))
            .collect::<Vec<_>>();
        assert_eq!(
            !collisions.is_empty(),
            blocked,
            "{face}, aperture {aperture}, x {x}: {}",
            reply["assembly"]["diagnostics"]
        );
        if blocked {
            assert!(collisions.iter().all(|finding| {
                finding["severity"] == "error"
                    && finding["targetIds"]
                        .as_array()
                        .unwrap()
                        .contains(&json!("module-1"))
            }));
            assert_eq!(reply["assembly"]["generationBlocked"], true);
            let markers = reply["assembly"]["findingMarkers"]
                .as_array()
                .expect("Case failures need localized Rust geometry");
            assert!(collisions.iter().all(|finding| {
                markers.iter().any(|marker| {
                    marker["findingId"] == finding["id"] && marker["boardId"] == "host"
                })
            }));
            if face == "back" && x > 0.0 {
                assert!(
                    markers
                        .iter()
                        .flat_map(|marker| marker["contours"].as_array().unwrap())
                        .flat_map(|contour| contour["points"].as_array().unwrap())
                        .all(|point| point["x"].as_f64().unwrap() > 39.5),
                    "Highlight the intersecting wall strip, not the whole module or host board"
                );
            }
        }
    }
}

#[test]
fn a_module_follows_only_its_selected_host_instance_and_physical_flip_once() {
    let mut doc = document();
    doc["moduleDefinitions"] = json!([definition()]);
    let mut m = placement("front", "back", 3.0);
    m["hostInstanceId"] = json!("right");
    doc["modules"] = json!([m]);
    doc["hardware"] = json!({"topology":"split","transport":"wireless","boards":[],"instances":[{"id":"right","name":"Right","boardId":"host","half":"right","role":"peripheral","flipped":true,"constructionLinked":false},{"id":"left","name":"Left","boardId":"host","half":"left","role":"central","flipped":false,"constructionLinked":false}]});
    doc["physicalInstanceId"] = json!("right");
    let mut engine = CoreEngine::new();
    let result = request(
        &mut engine,
        json!({"id":"resolve","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert_eq!(result["kind"], "modules-resolved", "{result}");
    let module = &result["result"]["modules"][0];
    assert_eq!(module["at"]["x"].as_f64(), Some(-15.0));
    assert_eq!(module["at"]["y"].as_f64(), Some(7.0));
    assert_eq!(module["rotation"].as_f64(), Some(-30.0));
    assert_eq!(module["flipped"], true);
    assert_eq!(module["midplaneZ"], json!(-4.6));
    doc["physicalInstanceId"] = json!("left");
    let other = request(
        &mut engine,
        json!({"id":"resolve","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert_eq!(other["result"]["modules"], json!([]));
}

#[test]
fn module_service_clearance_checks_both_axes_and_host_pcb_material() {
    let mut doc = document();
    doc["moduleDefinitions"] = json!([definition()]);
    let mut a = placement("front", "back", 3.0);
    a["at"] = json!({"x":0,"y":0});
    a["rotation"] = json!(0);
    a["serviceClearance"] = json!(0.4);
    let mut b = a.clone();
    b["id"] = json!("module-2");
    b["gap"] = json!(5.3);
    doc["modules"] = json!([a, b]);
    let mut engine = CoreEngine::new();
    let vertical = request(
        &mut engine,
        json!({"id":"vertical","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert!(
        vertical["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"].as_str().unwrap().contains("/collision/")),
        "Two 0.4 mm service envelopes cannot fit into a 0.7 mm vertical gap"
    );
    doc["modules"][0]["serviceClearance"] = json!(0.6);
    doc["modules"][1]["serviceClearance"] = json!(0.6);
    doc["modules"][1]["gap"] = json!(3);
    doc["modules"][1]["at"]["x"] = json!(26);
    let lateral = request(
        &mut engine,
        json!({"id":"lateral","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert!(
        lateral["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"].as_str().unwrap().contains("/collision/")),
        "Two 0.6 mm service envelopes cannot fit into a 1 mm lateral gap"
    );
    doc["modules"] = json!([a]);
    doc["modules"][0]["serviceClearance"] = json!(0);
    doc["moduleDefinitions"][0]["volumes"] = json!([{"id":"body","purpose":"occupied","source":"fixture","qualified":true,"geometry":{"points":[{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}],"z":-4.5,"height":1}}]);
    let host = request(
        &mut engine,
        json!({"id":"host","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert!(
        host["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"].as_str().unwrap().ends_with("/host-pcb")),
        "An attached component cannot pass through the solid host PCB"
    );
}

#[test]
fn module_collision_checks_include_z_and_focus_the_intersection() {
    let mut doc = document();
    doc["moduleDefinitions"] = json!([definition()]);
    let a = placement("front", "back", 3.0);
    let mut b = placement("back", "front", 3.0);
    b["id"] = json!("module-2");
    doc["modules"] = json!([a, b]);
    let mut engine = CoreEngine::new();
    let clear = request(
        &mut engine,
        json!({"id":"clear","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert_eq!(clear["kind"], "modules-resolved", "{clear}");
    assert!(
        !clear["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"].as_str().unwrap().contains("collision"))
    );
    doc["modules"][1]["hostFace"] = json!("front");
    doc["modules"][1]["facingFace"] = json!("back");
    doc["modules"][1]["at"]["x"] = json!(35.0);
    let clash = request(
        &mut engine,
        json!({"id":"clash","kind":"open","document":doc}),
    );
    let finding = clash["scene"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"].as_str().unwrap().contains("collision"))
        .expect("Overlapping occupied volumes must conflict");
    let marker = clash["scene"]["findingMarkers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["findingId"] == finding["id"])
        .unwrap();
    assert_eq!(
        marker["contours"].as_array().unwrap().len(),
        1,
        "Highlight the intersection, not both whole outlines"
    );
}

#[test]
fn case_fixed_modules_check_relative_gasket_travel_against_host_components() {
    let mut doc = document();
    doc["moduleDefinitions"] = json!([definition()]);
    let mut instance = placement("front", "back", 2.5);
    instance["rotation"] = json!(0);
    instance["attachment"] = json!("case");
    doc["modules"] = json!([instance]);
    doc["definitions"] = json!([{"id":"component","name":"Measured component","kind":"custom","pads":[],"courtyard":[{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}],"mechanicalProfile":{"definitionId":"component","source":"Measured drawing A","plateToPcb":0,"cutouts":[],"clearanceVolumes":[{"points":[{"x":-3,"y":-3},{"x":3,"y":-3},{"x":3,"y":3},{"x":-3,"y":3}],"z":0,"height":2}]}}]);
    doc["parts"] = json!([{"id":"body","definitionId":"component","reference":"U1","pose":{"at":{"x":15,"y":7},"rotation":0},"side":"front"}]);
    doc["boards"][0]["partIds"] = json!(["body"]);
    doc["mechanical"] = json!({"boardId":"host","kind":"tray","mount":"gasket","method":"printed","gasketTravel":0.6,"bottomStyle":"shell","plateMaterialId":"fr4","caseMaterialId":"pla","plateThickness":1.5,"plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,"batteryHeight":0,"bottomThickness":2,"plateToPcb":3.5,"wallThickness":2,"clearance":0.2,"profiles":[],"mounts":[]});
    let mut engine = CoreEngine::new();
    let reply = request(
        &mut engine,
        json!({"id":"fixed","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert_eq!(reply["kind"], "modules-resolved", "{reply}");
    let finding = reply["result"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"].as_str().unwrap().contains("host-component"))
        .expect(
            "The PCB can travel 0.6mm into a case-fixed module across the 0.5mm static clearance",
        );
    assert!(
        finding["targetIds"]
            .as_array()
            .unwrap()
            .contains(&json!("body"))
    );
    doc["modules"][0]["attachment"] = json!("board");
    let together = request(
        &mut engine,
        json!({"id":"together","kind":"resolve-modules","document":doc,"boardId":"host"}),
    );
    assert!(
        !together["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["id"].as_str().unwrap().contains("host-component")),
        "PCB-attached parts move together and retain the static clearance"
    );
}

#[test]
fn module_attachment_is_atomic_and_resolves_facing_surface_gap_above_and_below() {
    let mut engine = CoreEngine::new();
    let original = document();
    assert_eq!(
        request(
            &mut engine,
            json!({"id":"open","kind":"open","document":original})
        )["kind"],
        "scene"
    );
    let reply = request(
        &mut engine,
        json!({"id":"attach","kind":"edit","command":{
            "baseRevision":0,"transactionId":"attach","phase":"commit","targetIds":["module-1"],
            "operation":{"kind":"set-mounted-module","instance":placement("front","back",3.0),"definition":definition()}
        }}),
    );
    assert_eq!(reply["kind"], "scene", "{reply}");
    let above = reply["document"].clone();
    assert_eq!(
        above["parts"],
        json!([]),
        "Mounted copper must stay out of the host"
    );
    let resolved = request(
        &mut engine,
        json!({"id":"resolve","kind":"resolve-modules","document":above,"boardId":"host"}),
    );
    assert_eq!(resolved["kind"], "modules-resolved", "{resolved}");
    assert_eq!(resolved["result"]["modules"][0]["midplaneZ"], json!(4.6));
    assert_eq!(resolved["result"]["modules"][0]["flipped"], false);
    // Existing mechanical consumers use host top at zero: module bottom = gap.
    assert_eq!(
        resolved["result"]["modules"][0]["board"][0]["z"],
        json!(3.0)
    );
    let bad = request(
        &mut engine,
        json!({"id":"bad","kind":"edit","command":{
            "baseRevision":1,"transactionId":"bad","phase":"commit","targetIds":["module-1"],
            "operation":{"kind":"set-mounted-module","instance":placement("back","front",-1.0)}
        }}),
    );
    assert_eq!(bad["kind"], "error", "{bad}");
    assert_eq!(
        request(&mut engine, json!({"id":"snapshot","kind":"snapshot"}))["document"],
        above
    );
    let below = request(
        &mut engine,
        json!({"id":"below","kind":"edit","command":{
            "baseRevision":1,"transactionId":"below","phase":"commit","targetIds":["module-1"],
            "operation":{"kind":"set-mounted-module","instance":placement("back","front",3.0)}
        }}),
    );
    let resolved = request(
        &mut engine,
        json!({"id":"resolve","kind":"resolve-modules","document":below["document"],"boardId":"host"}),
    );
    assert_eq!(resolved["result"]["modules"][0]["midplaneZ"], json!(-4.6));
    assert!(
        (resolved["result"]["modules"][0]["board"][0]["z"]
            .as_f64()
            .unwrap()
            + 6.2)
            .abs()
            < 1e-9
    );
    assert_eq!(
        request(&mut engine, json!({"id":"undo","kind":"undo"}))["document"]["modules"],
        above["modules"]
    );
}

#[test]
fn pinned_splitter_import_preserves_real_outline_mounts_and_reversed_interface_contacts() {
    let source = include_str!(
        "../../catalogue/modules/sources/sadekbaroudi-vik/pcb/vik-splitter/vik-splitter.kicad_pcb"
    );
    let reply:Value = serde_json::from_str(&boardstudio_core::artifact::request(&json!({
        "id":"import","kind":"import-module-board","definitionId":"vik:splitter","name":"VIK splitter",
        "source":source,"provenance":definition()["source"],"family":"expansion","variant":"source population"
    }).to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    let imported = &reply["result"];
    assert!(
        !imported["gates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gate| gate["code"] == "board-hole-geometry"),
        "valid unlocked source text must not prevent importing board drills: {}",
        imported["gates"]
    );
    // Two mounts, twelve header contacts and twenty-four vias in the pinned PCB.
    assert_eq!(imported["board"]["holes"].as_array().unwrap().len(), 38);
    assert_eq!(imported["board"]["thickness"], 1.6);
    assert_eq!(imported["mounts"].as_array().unwrap().len(), 2);
    assert_eq!(imported["interfaces"].as_array().unwrap().len(), 4);
    let input = imported["interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "J1003")
        .unwrap();
    let output = imported["interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "J1001")
        .unwrap();
    assert_eq!(input["signals"][0], "sclk");
    assert_eq!(output["signals"][11], "sclk");
    let points = imported["board"]["contours"][0]["points"]
        .as_array()
        .unwrap();
    let (min, max) = points
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
            (a.min(x), b.max(x))
        });
    assert!((max - min - 25.0).abs() < 0.02);
    let nets = imported["circuit"]["nets"].as_array().unwrap();
    // Preserve the literal source net name; the interface role is separate.
    let sclk = nets.iter().find(|n| n["name"] == "SCLK_IN").unwrap();
    assert_eq!(sclk["pins"].as_array().unwrap().len(), 4);
    assert_eq!(imported["circuit"]["ports"]["sclk"], sclk["id"]);
}
