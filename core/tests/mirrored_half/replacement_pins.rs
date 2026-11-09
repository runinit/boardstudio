use super::*;

#[path = "replacement_pins/capacity_rejoin.rs"]
mod capacity_rejoin;
use crate::support::replacement_electrical;

fn catalogue_definition(source: &str) -> boardstudio_core::model::PartDefinition {
    boardstudio_core::generators::catalogue()
        .unwrap()
        .into_iter()
        .find(|definition| {
            definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
        .unwrap()
}

fn supported_mirror_pair() -> (CoreEngine, Value, String) {
    let document = replacement_electrical::supported_source_document();
    let (mut engine, seed) = open(&document);
    let created = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&document),
    )
    .unwrap();
    let mut target = created["document"].clone();
    let display = catalogue_definition("ceoloide/display_ssd1306");
    let display_id = display.id.clone();
    target["definitions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(display).unwrap());
    let (engine, target) = open(&target);
    (engine, target, display_id)
}

fn mirror_pair_without_electrical_configuration() -> (CoreEngine, Value) {
    let input = synthetic_half::document();
    let mut document = seed_document(&input, "left");
    document["hardware"]["boards"] = json!([]);
    let (mut engine, seed) = open(&document);
    let created = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&input),
    )
    .unwrap();
    let (engine, target) = open(&created["document"]);
    (engine, target)
}

fn keyless_supported_mirror_pair() -> (CoreEngine, Value, String) {
    let input = synthetic_half::document();
    let mut document = seed_document(&input, "left");
    let controller = catalogue_definition("ceoloide/mcu_nice_nano");
    document["definitions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(controller).unwrap());
    document["parts"]
        .as_array_mut()
        .unwrap()
        .retain(|part| !part["id"].as_str().unwrap().starts_with("matrix/"));
    let source_part_ids: Vec<_> = document["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["id"].clone())
        .collect();
    document["boards"][0]["partIds"] = json!(source_part_ids);
    document["boards"][0]["netIds"] = json!([]);
    document["matrices"] = json!([]);
    document["layouts"] = json!([]);
    document["nets"] = json!([]);
    let controller_part = document["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "left/U1")
        .unwrap();
    controller_part["definitionId"] = json!("generator:ceoloide/mcu_nice_nano");
    let controller_config = &mut document["hardware"]["boards"][0];
    controller_config["controllerPartId"] = json!("left/U1");
    controller_config["mode"] = json!("matrix");
    let (mut engine, seed) = open(&document);
    let created = create(
        &mut engine,
        &seed,
        "pair-right",
        "left",
        new_board("right", "right"),
        synthetic_mapping(&input),
    )
    .unwrap();
    let mut target = created["document"].clone();
    let display = catalogue_definition("ceoloide/display_ssd1306");
    let display_id = display.id.clone();
    target["definitions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(display).unwrap());
    let (engine, target) = open(&target);
    (engine, target, display_id)
}

#[test]
fn open_without_generated_mirror_nets_preserves_linked_controller_terminal_parameters() {
    let (_, mut document, _) = supported_mirror_pair();
    let controller_definition = document["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == "generator:ceoloide/mcu_nice_nano")
        .unwrap();
    let terminal = controller_definition["terminals"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let source_controller = document["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|part| part["id"] == "left/U1")
        .unwrap();
    source_controller["generatorParameters"] =
        serde_json::Map::from_iter([(terminal.clone(), json!("AUTHORED_WITHOUT_GENERATED_NETS"))])
            .into();

    let (mut engine, opened) = open(&document);
    assert!(opened["nets"].as_array().unwrap().iter().all(|net| {
        !net["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("generated/electrical/right/"))
    }));
    assert_eq!(
        part(&opened, "right/U1")["generatorParameters"][terminal],
        "AUTHORED_WITHOUT_GENERATED_NETS"
    );
    assert_eq!(
        snapshot(&mut engine),
        opened,
        "Open leaves the prepared controller state intact"
    );
}

#[test]
fn open_with_generated_and_manual_target_nets_preserves_manual_controller_terminal_label() {
    let (mut engine, target, display_id) = supported_mirror_pair();
    let operation = add_display_operation("right", "right/OLED", &display_id);
    let placed: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"manual-terminal-display","kind":"edit","command":{
                "baseRevision":target["revision"],"transactionId":"manual-terminal-display",
                "phase":"commit","targetIds":[],"operation":operation
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(placed["kind"], "scene", "{placed}");
    let mut document = placed["document"].clone();
    let controller = part(&document, "right/U1").clone();
    let definition = document["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == controller["definitionId"])
        .unwrap();
    let generated_prefix = "generated/electrical/right/";
    let generated_nets: Vec<_> = document["nets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|net| {
            net["id"]
                .as_str()
                .is_some_and(|id| id.starts_with(generated_prefix))
        })
        .collect();
    let (terminal, pad_id) = definition["terminals"]
        .as_object()
        .unwrap()
        .iter()
        .find_map(|(terminal, pads)| {
            pads.as_array()?.iter().find_map(|pad| {
                let pad_id = pad.as_str()?;
                (!generated_nets.iter().any(|net| {
                    net["pins"].as_array().is_some_and(|pins| {
                        pins.iter()
                            .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == pad_id)
                    })
                }))
                .then(|| (terminal.clone(), pad_id.to_owned()))
            })
        })
        .expect("the controller profile has a GPIO pad unused by generated target wiring");
    let manual_net_id = "manual/right/controller-terminal";
    let manual_net_name = "MANUAL_CONTROLLER_TERMINAL";
    document["nets"].as_array_mut().unwrap().push(json!({
        "id":manual_net_id,"name":manual_net_name,
        "pins":[{"partId":"right/U1","padId":pad_id.clone()}]
    }));
    let target_board = document["boards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|board| board["id"] == "right")
        .unwrap();
    target_board["netIds"]
        .as_array_mut()
        .unwrap()
        .push(json!(manual_net_id));

    let (_, reopened) = open(&document);
    assert!(reopened["nets"].as_array().unwrap().iter().any(|net| {
        net["id"] == manual_net_id
            && net["pins"].as_array().is_some_and(|pins| {
                pins.iter()
                    .any(|pin| pin["partId"] == "right/U1" && pin["padId"] == pad_id)
            })
    }));
    assert_eq!(
        part(&reopened, "right/U1")["generatorParameters"][terminal],
        manual_net_name,
        "terminal sync considers manual mirror nets alongside generated target nets"
    );
}

fn add_display_operation(board_id: &str, id: &str, definition_id: &str) -> Value {
    json!({
        "kind":"add-mirror-only-part",
        "part":{
            "id":id, "definitionId":definition_id, "reference":"OLED1",
            "pose":{"at":{"x":-40.0,"y":-10.0},"rotation":0.0}, "side":"front"
        },
        "boardId":board_id, "definition":null, "layoutId":null
    })
}

#[test]
fn mirror_only_ssd1306_is_assigned_in_the_same_public_transaction() {
    let (mut engine, mut target, display_id) = supported_mirror_pair();
    let before_placement = target.clone();
    let document_state = |mut document: Value| {
        document.as_object_mut().unwrap().remove("revision");
        document
    };
    let operation = add_display_operation("right", "right/OLED", &display_id);
    let source_parts_before: Vec<_> = target["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part["id"].as_str().unwrap().starts_with("left/"))
        .cloned()
        .collect();
    let source_electrical_before = target["hardware"]["boards"][0].clone();
    let source_nets_before: Vec<_> = target["nets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|net| net["id"].as_str().unwrap().starts_with("left/"))
        .cloned()
        .collect();
    let preview: Value = serde_json::from_str(
        &engine.request(&json!({"id":"preview-display","kind":"edit","command":{
            "baseRevision":target["revision"],"transactionId":"preview-display","phase":"preview",
            "targetIds":[],"operation":operation.clone()
        }}).to_string()),
    )
    .unwrap();
    assert_eq!(preview["kind"], "preview", "{preview}");
    assert_eq!(
        snapshot(&mut engine),
        target,
        "Preview leaves the saved document intact"
    );
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"add-display","kind":"edit","command":{
                "baseRevision":target["revision"],"transactionId":"add-display","phase":"commit",
                "targetIds":[],"operation":operation
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(reply["kind"], "scene", "{reply}");
    target = reply["document"].clone();
    let board = target["hardware"]["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|board| board["boardId"] == "right")
        .unwrap();
    assert!(board["assignments"]["i2c/SDA"].as_str().is_some());
    assert!(board["assignments"]["i2c/SCL"].as_str().is_some());
    assert_ne!(
        board["assignments"]["i2c/SDA"],
        board["assignments"]["i2c/SCL"]
    );
    for (function, pin) in board["locks"].as_object().unwrap() {
        assert_eq!(board["assignments"][function], *pin, "locked {function}");
    }
    assert_eq!(target["hardware"]["boards"][0], source_electrical_before);
    let source_parts_after: Vec<_> = target["parts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|part| part["id"].as_str().unwrap().starts_with("left/"))
        .cloned()
        .collect();
    assert_eq!(source_parts_after, source_parts_before);
    let source_nets_after: Vec<_> = target["nets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|net| net["id"].as_str().unwrap().starts_with("left/"))
        .cloned()
        .collect();
    assert_eq!(source_nets_after, source_nets_before);
    assert!(
        target["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == "right/OLED")
    );
    let undone = request(&mut engine, json!({"id":"undo-display","kind":"undo"}));
    assert_eq!(undone["kind"], "scene", "{undone}");
    assert_eq!(
        document_state(undone["document"].clone()),
        document_state(before_placement),
        "Placement Undo restores the complete previous electrical and mechanical state"
    );
    assert!(
        !undone["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == "right/OLED")
    );
    let redone = request(&mut engine, json!({"id":"redo-display","kind":"redo"}));
    assert_eq!(redone["kind"], "scene", "{redone}");
    assert_eq!(
        document_state(redone["document"].clone()),
        document_state(target.clone()),
        "Placement Redo restores all assignments, locks, nets and part state"
    );
    assert_eq!(
        redone["document"]["hardware"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|board| board["boardId"] == "right")
            .unwrap()["assignments"]["i2c/SDA"],
        board["assignments"]["i2c/SDA"]
    );
    let (_, reopened) = open(&redone["document"]);
    assert!(
        reopened["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == "right/OLED")
    );
    let (packed, buffers) = boardstudio_core::archive::request(
        &json!({
            "kind":"pack-project",
            "projectJson":redone["document"].to_string(),
            "archiveJson":json!({"embedUsedModels":false}).to_string(),
            "assets":[]
        })
        .to_string(),
        &[],
    );
    let packed: Value = serde_json::from_str(&packed).unwrap();
    assert_eq!(packed["kind"], "packed", "{packed}");
    let (unpacked, _unpacked_buffers) =
        boardstudio_core::archive::request(&json!({"kind":"unpack-project"}).to_string(), &buffers);
    let unpacked: Value = serde_json::from_str(&unpacked).unwrap();
    assert_eq!(unpacked["kind"], "unpacked", "{unpacked}");
    let archived: Value = serde_json::from_str(unpacked["projectJson"].as_str().unwrap()).unwrap();
    assert_eq!(archived, redone["document"]);
}

#[test]
fn padless_placement_needs_no_mirror_electrical_configuration() {
    let (mut engine, document) = mirror_pair_without_electrical_configuration();
    assert!(
        document["hardware"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .all(|board| board["boardId"] != "right")
    );
    let operation = json!({
        "kind":"add-mirror-only-part",
        "part":{
            "id":"right/cosmetic", "definitionId":"synthetic/led", "reference":"D1",
            "pose":{"at":{"x":-40.0,"y":-10.0},"rotation":0.0}, "side":"front"
        },
        "boardId":"right", "definition":null, "layoutId":null
    });
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"cosmetic","kind":"edit","command":{
                "baseRevision":document["revision"],"transactionId":"cosmetic","phase":"commit",
                "targetIds":[],"operation":operation
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(reply["kind"], "scene", "{reply}");
    assert!(
        reply["document"]["parts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|part| part["id"] == "right/cosmetic")
    );
    assert!(
        reply["document"]["hardware"]["boards"]
            .as_array()
            .unwrap()
            .iter()
            .all(|board| board["boardId"] != "right")
    );
}

#[test]
fn keyless_mirror_peripheral_refuses_the_existing_no_keys_error_atomically() {
    let (mut engine, document, display_id) = keyless_supported_mirror_pair();
    let accepted = document.clone();
    let operation = add_display_operation("right", "right/OLED", &display_id);
    let reply: Value = serde_json::from_str(
        &engine.request(&json!({"id":"matrixless-display","kind":"edit","command":{
            "baseRevision":document["revision"],"transactionId":"matrixless-display","phase":"commit",
            "targetIds":[],"operation":operation
        }}).to_string()),
    )
    .unwrap();
    assert_eq!(reply["kind"], "error", "{reply}");
    assert_eq!(
        reply["message"],
        "Place a key layout on this PCB before resolving wiring"
    );
    assert_eq!(snapshot(&mut engine), accepted, "Refusal is atomic");
}

#[test]
fn unsupported_controller_refuses_electrical_replacement_atomically() {
    let (_, mut document, display_id) = supported_mirror_pair();
    let controller = document["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "generator:ceoloide/mcu_nice_nano")
        .unwrap();
    controller["generator"]["source"] = json!("unsupported/controller");
    let (mut engine, document) = open(&document);
    let accepted = document.clone();
    let operation = add_display_operation("right", "right/OLED", &display_id);
    let reply: Value = serde_json::from_str(
        &engine.request(&json!({"id":"unsupported-display","kind":"edit","command":{
            "baseRevision":document["revision"],"transactionId":"unsupported-display","phase":"commit",
            "targetIds":[],"operation":operation
        }}).to_string()),
    )
    .unwrap();
    assert_eq!(reply["kind"], "error", "{reply}");
    assert_eq!(
        reply["message"],
        "This controller needs a reviewed pin profile"
    );
    assert_eq!(snapshot(&mut engine), accepted, "Refusal is atomic");
}

#[test]
fn unknown_plated_mirror_part_refuses_without_a_fallback_profile() {
    let (_, mut document, _) = supported_mirror_pair();
    let mut definition =
        serde_json::to_value(catalogue_definition("ceoloide/display_ssd1306")).unwrap();
    definition["id"] = json!("third-party/electrical");
    definition["name"] = json!("Unprofiled plated part");
    definition["kind"] = json!("custom");
    definition["generator"] = Value::Null;
    document["definitions"]
        .as_array_mut()
        .unwrap()
        .push(definition);
    let (mut engine, document) = open(&document);
    let accepted = document.clone();
    let operation = json!({
        "kind":"add-mirror-only-part",
        "part":{
            "id":"right/UNPROFILED", "definitionId":"third-party/electrical",
            "reference":"X1", "pose":{"at":{"x":-40.0,"y":-10.0},"rotation":0.0},
            "side":"front"
        },
        "boardId":"right", "definition":null, "layoutId":null
    });
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id":"unprofiled","kind":"edit","command":{
                "baseRevision":document["revision"],"transactionId":"unprofiled","phase":"commit",
                "targetIds":[],"operation":operation
            }})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(reply["kind"], "error", "{reply}");
    assert_eq!(
        reply["message"],
        "X1 needs a reviewed electrical profile before automatic handoff"
    );
    assert_eq!(snapshot(&mut engine), accepted, "Refusal is atomic");
}

#[test]
fn duplicate_default_address_ssd1306_refuses_the_second_placement_atomically() {
    let (mut engine, document, display_id) = supported_mirror_pair();
    let first = add_display_operation("right", "right/OLED1", &display_id);
    let committed = edit_for_replacement_test(&mut engine, &document, "first-oled", first);
    assert_eq!(committed["kind"], "scene", "{committed}");
    let accepted = committed["document"].clone();
    let second = add_display_operation("right", "right/OLED2", &display_id);
    let refused = edit_for_replacement_test(&mut engine, &accepted, "second-oled", second);
    assert_eq!(refused["kind"], "error", "{refused}");
    assert_eq!(
        refused["message"],
        "The SSD1306 displays share address 0x3C; configure distinct addresses before wiring"
    );
    assert_eq!(snapshot(&mut engine), accepted, "Refusal is atomic");
}

fn edit_for_replacement_test(
    engine: &mut CoreEngine,
    document: &Value,
    transaction: &str,
    operation: Value,
) -> Value {
    serde_json::from_str(
        &engine.request(
            &json!({"id":transaction,"kind":"edit","command":{
                "baseRevision":document["revision"],"transactionId":transaction,"phase":"commit",
                "targetIds":[],"operation":operation
            }})
            .to_string(),
        ),
    )
    .unwrap()
}
