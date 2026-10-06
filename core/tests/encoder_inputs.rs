use boardstudio_core::{CoreEngine, model::*};
use serde_json::json;

fn encoder_project() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("encoders", "Encoder inputs");
    doc.definitions.push(serde_json::from_value(json!({
        "id":"encoder", "name":"Switched encoder", "kind":"encoder",
        "courtyard":[{"x":-6,"y":-6},{"x":6,"y":-6},{"x":6,"y":6},{"x":-6,"y":6}],
        "pads":(["A","B","C","S1","S2"].iter().enumerate().map(|(i,id)| json!({
            "id":id,"number":id,"at":{"x":i,"y":0},"size":{"x":1,"y":1},"shape":"circle"
        })).collect::<Vec<_>>()),
        "terminals":{"A":["A"],"B":["B"],"C":["C"],"S1":["S1"],"S2":["S2"]},
        "matrixTerminals":{"row":"S1","column":"S2"},
        "generator":{"source":"ceoloide/rotary_encoder_ec11_ec12","version":"test","parameters":{}}
    })).unwrap());
    doc.parts.push(
        serde_json::from_value(json!({
            "id":"matrix/main/r0c0","definitionId":"encoder","reference":"SW1",
            "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"
        }))
        .unwrap(),
    );
    doc.boards.push(serde_json::from_value(json!({
        "id":"board","name":"Board","partIds":["matrix/main/r0c0"],"outlineIds":[],"netIds":[],"thickness":1.6
    })).unwrap());
    doc.matrices.push(
        serde_json::from_value(json!({
            "id":"main","rows":1,"columns":1,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},
            "definitionId":"encoder","partIds":["matrix/main/r0c0"],"boardId":"board"
        }))
        .unwrap(),
    );
    doc
}

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("Expected successful public edit, got {other:?}"),
    }
}

#[test]
fn matrix_encoder_press_accepts_layer_bindings_and_undo() {
    let doc = encoder_project();
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc.clone(),
    }));
    let operation = serde_json::from_value(json!({
        "kind":"edit-keymap", "change":{"kind":"binding","layerId":"base","keyId":"matrix/main/r0c0", "binding":{"kind":"mod-tap","hold":"LCTRL","tap":"A"}}
    })).unwrap();
    let (_, edited) = scene(engine.handle(CoreRequest::Edit {
        id: "bind".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "bind".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["matrix/main/r0c0".into()],
            operation,
        },
    }));
    let saved = serde_json::to_value(&edited).unwrap();
    assert_eq!(
        saved["keymap"]["layers"][0]["bindings"]["matrix/main/r0c0"]["kind"],
        "mod-tap"
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.parts, doc.parts);
    assert_eq!(undone.keymap, doc.keymap);
}

#[test]
fn matrix_press_is_not_also_a_direct_input_or_grounded() {
    let request = serde_json::from_value(json!({
        "document":encoder_project(), "boardId":"board", "mode":"matrix"
    }))
    .unwrap();
    let plan = boardstudio_core::electrical::resolve(request);
    let encoder = plan
        .peripherals
        .iter()
        .find(|p| p.kind == "encoder")
        .unwrap();
    assert_eq!(
        encoder
            .gpio_terminals
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "C"]
    );
    assert_eq!(encoder.fixed_terminals, vec![("B".into(), "GND".into())]);
    assert!(!plan.nets.iter().any(|net| {
        net.id.ends_with("/power/gnd")
            && net
                .pins
                .iter()
                .any(|pin| pin.part_id == "matrix/main/r0c0" && pin.pad_id == "S2")
    }));
}

#[test]
fn an_explicit_press_only_profile_does_not_invent_legacy_encoder_rotation() {
    let mut value = serde_json::to_value(encoder_project()).unwrap();
    value["definitions"][0]["inputProfile"] =
        json!({"press":{"row":"S1","column":"S2","independent":true}});
    let plan = boardstudio_core::electrical::resolve(
        serde_json::from_value(json!({"document":value,"boardId":"board","mode":"matrix"}))
            .unwrap(),
    );
    assert!(
        plan.peripherals
            .iter()
            .all(|peripheral| peripheral.kind != "encoder"),
        "Explicit capabilities must override the legacy generator name"
    );
}

#[test]
fn pcb_export_rejects_different_nets_on_repeated_physical_pad_numbers() {
    let mut doc = encoder_project();
    doc.definitions[0].generator = None;
    doc.definitions[0].pads[0].number = "1".into();
    doc.definitions[0].pads[2].number = "1".into();
    let part_id = doc.parts[0].id.clone();
    doc.nets=serde_json::from_value(json!([{ "id":"a","name":"A","pins":[{"partId":part_id,"padId":"A"}]},{"id":"b","name":"B","pins":[{"partId":part_id,"padId":"C"}]}])).unwrap();
    doc.boards[0].net_ids = vec!["a".into(), "b".into()];
    let reply:serde_json::Value=serde_json::from_str(&boardstudio_core::artifact::request(&json!({"id":"unsafe","kind":"export-pcb","request":{"snapshotToken":"committed","expectedRevision":0,"document":doc,"target":{"kind":"board","boardId":"board"},"contours":[{"hole":false,"points":[{"x":-10,"y":-10},{"x":10,"y":-10},{"x":10,"y":10},{"x":-10,"y":10}]}],"modelPaths":{}}}).to_string())).unwrap();
    assert_eq!(
        reply["kind"], "error",
        "A stateless export must enforce repeated-terminal net consistency"
    );
    assert!(reply["error"]["message"].as_str().unwrap().contains("1"));
}

#[test]
fn explicit_input_profile_works_without_a_source_name_or_encoder_category() {
    let mut value = serde_json::to_value(encoder_project()).unwrap();
    value["definitions"][0]["kind"] = json!("custom");
    value["definitions"][0]
        .as_object_mut()
        .unwrap()
        .remove("generator");
    value["definitions"][0]["inputProfile"] = json!({
        "press":{"row":"S1","column":"S2","independent":true},
        "rotary":{"a":"A","b":"C","common":"B","steps":80,"triggersPerRotation":20,"driver":"ec11"}
    });
    let mut engine = CoreEngine::new();
    let reply = engine.request(&json!({"id":"open","kind":"open","document":value}).to_string());
    let (_, doc) = scene(serde_json::from_str(&reply).unwrap());
    let request =
        serde_json::from_value(json!({"document":doc,"boardId":"board","mode":"matrix"})).unwrap();
    let plan = boardstudio_core::electrical::resolve(request);
    let encoder = plan
        .peripherals
        .iter()
        .find(|p| p.part_id == "matrix/main/r0c0")
        .unwrap();
    assert_eq!(encoder.kind, "encoder");
    assert_eq!(
        encoder
            .gpio_terminals
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "C"]
    );
    assert_eq!(encoder.fixed_terminals, vec![("B".into(), "GND".into())]);
}

#[test]
fn press_scan_modes_are_exclusive_and_invalid_matrix_assignment_is_atomic() {
    let mut doc = encoder_project();
    doc.parts[0].id = "knob".into();
    doc.boards[0].part_ids = vec!["knob".into()];
    doc.matrices.clear();
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc.clone(),
    }));
    let edit = |engine: &mut CoreEngine, revision, mode: &str| {
        let response=engine.request(&json!({"id":"scan","kind":"edit","command":{
            "baseRevision":revision,"transactionId":"scan","phase":"commit","targetIds":["knob"],
            "operation":{"kind":"set-input-scan-mode","partId":"knob","mode":mode}
        }}).to_string());
        serde_json::from_str::<CoreReply>(&response).unwrap()
    };
    let (_, unassigned) = scene(edit(&mut engine, 0, "unassigned"));
    let plan = boardstudio_core::electrical::resolve(
        serde_json::from_value(json!({"document":unassigned,"boardId":"board"})).unwrap(),
    );
    assert_eq!(plan.peripherals[0].gpio_terminals.len(), 2);
    assert_eq!(
        plan.peripherals[0].fixed_terminals,
        vec![("B".into(), "GND".into())]
    );
    assert!(matches!(
        edit(&mut engine, 1, "matrix"),
        CoreReply::Error { .. }
    ));
    let (_, after) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert_eq!(after, unassigned);
    let (_, direct) = scene(edit(&mut engine, 1, "direct"));
    let plan = boardstudio_core::electrical::resolve(
        serde_json::from_value(json!({"document":direct,"boardId":"board"})).unwrap(),
    );
    assert_eq!(plan.peripherals[0].gpio_terminals.len(), 3);
    let (_, undo) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undo.parts, unassigned.parts);
}

#[test]
fn reversible_contacts_compile_as_distinct_physical_pads_on_one_terminal() {
    let mut doc = encoder_project();
    let def = &mut doc.definitions[0];
    let mut back = def.pads[0].clone();
    back.id = "A-back".into();
    back.at.x = -4.9;
    back.side = Some(Side::Back);
    def.pads.push(back);
    def.terminals
        .insert("A".into(), vec!["A".into(), "A-back".into()]);
    def.generator = None;
    let reply:serde_json::Value=serde_json::from_str(&boardstudio_core::artifact::request(&json!({
        "id":"compile","kind":"compile-footprints","jobs":[{"id":"encoder","definition":def,"side":"front"}]
    }).to_string())).unwrap();
    assert_eq!(reply["kind"], "compile-footprints", "{reply}");
    let pads = reply["result"][0]["geometry"]["pads"].as_array().unwrap();
    assert_eq!(pads.iter().filter(|p| p["number"] == "A").count(), 2);
    assert_ne!(pads[0]["id"], pads[5]["id"]);
}

#[test]
fn firmware_uses_profile_rotation_settings_and_rejects_unknown_characteristics() {
    let base = json!({
        "controller_profile":"ceoloide/mcu_nice_nano", "board_name":"Encoder keyboard",
        "rows":[{"terminal":"P1","gpio":"P0.06"}],"columns":[{"terminal":"P0","gpio":"P0.08"}],
        "keys":[{"id":"matrix/main/r0c0","row":0,"column":0}],"diode_direction":"row2col", "key_bindings":["&kp A"],
        "encoder_ids":["knob"],
        "encoders":[{"id":"knob","aGpio":"P0.17","bGpio":"P0.20","profile":{"a":"A","b":"B","common":"C","steps":24,"triggersPerRotation":12,"driver":"ec11"}}]
    });
    let request = serde_json::from_value(base.clone()).unwrap();
    let package = boardstudio_core::firmware::generate(&request).unwrap();
    let overlay = &package.files["config/boards/shields/boardstudio/boardstudio.overlay"];
    assert!(overlay.contains("steps = <24>"), "{overlay}");
    assert!(
        overlay.contains("triggers-per-rotation = <12>"),
        "{overlay}"
    );
    assert!(overlay.contains("a-gpios = <&gpio0 17"), "{overlay}");
    let mut unknown = base;
    unknown["encoders"][0]["profile"]
        .as_object_mut()
        .unwrap()
        .remove("steps");
    let request = serde_json::from_value(unknown).unwrap();
    let message = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(
        message.contains("knob") && message.contains("steps"),
        "{message}"
    );
}

#[test]
fn matrix_rejects_rotation_only_profile_without_changing_the_document() {
    let original = encoder_project();
    let mut replacement = serde_json::to_value(&original.definitions[0]).unwrap();
    replacement["id"] = json!("rotation-only");
    replacement["inputProfile"] = json!({"rotary":{"a":"A","b":"C","common":"B"}});
    let definition = serde_json::from_value(replacement).unwrap();
    let mut matrix = original.matrices[0].clone();
    matrix.definition_id = "rotation-only".into();
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: original.clone(),
    }));
    let reply = engine.handle(CoreRequest::Edit {
        id: "replace".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "replace".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["main".into()],
            operation: EditOperation::SetMatrix {
                matrix,
                definitions: Some(vec![definition]),
            },
        },
    });
    assert!(matches!(reply, CoreReply::Error { .. }), "{reply:?}");
    let (_, after) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert_eq!(after, original);
}

#[test]
fn switching_scan_mode_removes_only_the_previous_generated_press_routes() {
    let mut doc = encoder_project();
    doc.parts[0].properties = Some(std::collections::BTreeMap::from([(
        "pressScanMode".into(),
        json!("direct"),
    )]));
    doc.nets = serde_json::from_value(json!([
        {"id":"generated/electrical/board/push", "name":"PUSH", "pins":[{"partId":"matrix/main/r0c0","padId":"S1"}]},
        {"id":"generated/electrical/board/power/gnd", "name":"GND", "pins":[{"partId":"matrix/main/r0c0","padId":"S2"},{"partId":"matrix/main/r0c0","padId":"B"}]},
        {"id":"manual", "name":"User wire", "pins":[{"partId":"matrix/main/r0c0","padId":"A"}]}
    ])).unwrap();
    doc.boards[0].net_ids = doc.nets.iter().map(|net| net.id.clone()).collect();
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc.clone(),
    }));
    let (_, edited) = scene(engine.handle(CoreRequest::Edit {
        id: "scan".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "scan".into(),
            phase: EditPhase::Commit,
            target_ids: vec![doc.parts[0].id.clone()],
            operation: EditOperation::SetInputScanMode {
                part_id: doc.parts[0].id.clone(),
                mode: PressScanMode::Matrix,
            },
        },
    }));
    assert!(!edited.nets.iter().any(|net| {
        net.pins
            .iter()
            .any(|pin| pin.pad_id == "S1" || pin.pad_id == "S2")
    }));
    assert_eq!(
        edited.nets.iter().find(|net| net.id == "manual"),
        doc.nets.iter().find(|net| net.id == "manual")
    );
    assert_eq!(
        edited
            .nets
            .iter()
            .find(|net| net.name == "GND")
            .unwrap()
            .pins[0]
            .pad_id,
        "B"
    );
    assert!(
        !edited.boards[0]
            .net_ids
            .iter()
            .any(|id| id.ends_with("/push"))
    );
    let (_, undo) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undo.nets, doc.nets);
}

#[test]
fn changing_scan_mode_removes_an_obsolete_controller_only_press_net() {
    let mut doc = encoder_project();
    doc.definitions.push(serde_json::from_value(json!({"id":"controller","name":"Controller","kind":"controller","courtyard":[],"pads":[{"id":"P1","number":"P1","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"}]})).unwrap());
    doc.parts.push(serde_json::from_value(json!({"id":"mcu","definitionId":"controller","reference":"MCU","pose":{"at":{"x":20,"y":0},"rotation":0},"side":"front"})).unwrap());
    doc.boards[0].part_ids.push("mcu".into());
    doc.nets=serde_json::from_value(json!([{"id":"generated/electrical/board/row/0","name":"ROW0","pins":[{"partId":"matrix/main/r0c0","padId":"S1"},{"partId":"mcu","padId":"P1"}]}])).unwrap();
    doc.boards[0].net_ids = vec![doc.nets[0].id.clone()];
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let (_, after) = scene(engine.handle(CoreRequest::Edit {
        id: "scan".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "scan".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetInputScanMode {
                part_id: "matrix/main/r0c0".into(),
                mode: PressScanMode::Unassigned,
            },
        },
    }));
    assert!(
        after.nets.is_empty(),
        "The removed press must not leave a controller-only generated net behind"
    );
    assert!(after.boards[0].net_ids.is_empty());
}

#[test]
fn changing_scan_mode_preserves_routed_work_and_rejects_the_edit_atomically() {
    let mut doc = encoder_project();
    doc.nets=serde_json::from_value(json!([{"id":"generated/electrical/board/push","name":"PUSH","pins":[{"partId":"matrix/main/r0c0","padId":"S1"}]}])).unwrap();
    doc.boards[0].net_ids = vec![doc.nets[0].id.clone()];
    doc.boards[0].traces=serde_json::from_value(json!([{"id":"routed","start":{"x":0,"y":0},"end":{"x":10,"y":0},"width":0.25,"layer":"front","netId":"generated/electrical/board/push"}])).unwrap();
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc.clone(),
    }));
    let reply = engine.handle(CoreRequest::Edit {
        id: "scan".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "scan".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::SetInputScanMode {
                part_id: "matrix/main/r0c0".into(),
                mode: PressScanMode::Unassigned,
            },
        },
    });
    assert!(
        matches!(reply, CoreReply::Error { .. }),
        "Existing routed copper needs explicit rerouting before a scan-mode change"
    );
    let (_, after) = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }));
    assert_eq!(after, doc);
}

#[test]
fn repeated_logical_contacts_reject_conflicting_nets_and_locate_both_physical_pads() {
    let mut doc = encoder_project();
    let mut repeated = doc.definitions[0].pads[0].clone();
    repeated.id = "A-other".into();
    repeated.at.x = -4.9;
    doc.definitions[0].pads.push(repeated);
    doc.nets = serde_json::from_value(json!([
        {"id":"net-a","name":"A","pins":[{"partId":"matrix/main/r0c0","padId":"A"}]},
        {"id":"net-b","name":"B","pins":[{"partId":"matrix/main/r0c0","padId":"A-other"}]}
    ]))
    .unwrap();
    let mut engine = CoreEngine::new();
    let (scene, _) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let conflict = scene
        .findings
        .iter()
        .find(|f| f.message.contains("Repeated physical pads"));
    assert!(
        conflict.is_some(),
        "Logical contact A cannot carry two different nets"
    );
    let marker = scene
        .finding_markers
        .iter()
        .find(|m| m.finding_id == conflict.unwrap().id)
        .unwrap();
    assert_eq!(marker.contours.len(), 2);
    assert!(marker.contours.iter().all(|c| {
        c.points
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max)
            - c.points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min)
            <= 1.01
    }));
}

#[test]
fn encoder_replacement_retains_keycap_settings_and_reports_unverified_fit() {
    let mut doc = encoder_project();
    doc.keycaps = Some(
        serde_json::from_value(json!({
            "matrices":{"main":{"profile":"cherry","mount":"mx"}},
            "keys":{"matrix/main/r0c0":{"legend":"A","color":"#112233"}}
        }))
        .unwrap(),
    );
    let original = doc.keycaps.clone();
    let mut engine = CoreEngine::new();
    let (opened, saved) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc.clone(),
    }));
    assert_eq!(saved.keycaps, original);
    let finding = opened
        .findings
        .iter()
        .find(|finding| finding.id == "keycaps/matrix/main/r0c0/unsupported-input")
        .expect("Inherited MX keycap settings on an encoder require a visible fit finding");
    assert_eq!(finding.target_ids, vec!["matrix/main/r0c0"]);
    let CoreReply::KeycapsResolved { result, .. } = engine.handle(CoreRequest::ResolveKeycaps {
        id: "caps".into(),
        board_id: "board".into(),
        document: saved,
        cases: None,
    }) else {
        panic!("Expected public keycap resolution")
    };
    assert!(
        result.specs.is_empty(),
        "An encoder cannot acquire a fabricated MX stem from inherited settings"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.id.ends_with("/unsupported-input"))
    );
}

#[test]
fn replacing_a_key_with_a_profiled_encoder_preserves_its_companion_and_all_layers() {
    let mut original = encoder_project();
    let mut replacement = serde_json::to_value(&original.definitions[0]).unwrap();
    replacement["id"] = json!("custom-input");
    replacement["kind"] = json!("custom");
    replacement.as_object_mut().unwrap().remove("generator");
    replacement["inputProfile"] = json!({
        "press":{"row":"S1","column":"S2","independent":true},
        "rotary":{"a":"A","b":"C","common":"B","steps":80,"triggersPerRotation":20,"driver":"ec11"}
    });
    let mut companion = original.definitions[0].clone();
    companion.id = "companion".into();
    companion.kind = PartKind::Custom;
    companion.generator = None;
    companion.matrix_terminals = None;
    original.definitions.push(companion);
    original.keymap = Some(serde_json::from_value(json!({"layers":[
        {"id":"base","name":"Base","bindings":{"matrix/main/r0c0":{"kind":"key-press","keycode":"A"}},"sensors":{}},
        {"id":"fn","name":"Fn","bindings":{"matrix/main/r0c0":{"kind":"layer-tap","layerId":"base","tap":"B"}},"sensors":{}}
    ],"macros":[]})).unwrap());
    original.matrices[0].cells = serde_json::from_value(json!([{
        "row":0,"column":0,"enabled":true,"offset":{"x":3,"y":4},"rotation":17,
        "assemblies":[{"id":"diode","definitionId":"companion","offset":{"x":8,"y":3},"rotation":90,"side":"back"}]
    }])).unwrap();
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: original.clone(),
    }));
    let (_, before) = scene(engine.handle(CoreRequest::Edit {
        id: "materialize".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "materialize".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["main".into()],
            operation: EditOperation::SetMatrix {
                matrix: original.matrices[0].clone(),
                definitions: None,
            },
        },
    }));
    let mut matrix = before.matrices[0].clone();
    matrix.cells[0].definition_id = Some("custom-input".into());
    let (findings, after) = scene(engine.handle(CoreRequest::Edit {
        id: "replace".into(),
        command: EditCommand {
            base_revision: before.revision,
            transaction_id: "replace".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["main".into()],
            operation: EditOperation::SetMatrix {
                matrix,
                definitions: Some(vec![serde_json::from_value(replacement).unwrap()]),
            },
        },
    }));
    assert_eq!(after.keymap, before.keymap);
    assert_eq!(after.boards[0].part_ids, before.boards[0].part_ids);
    assert_eq!(after.matrices[0].part_ids, before.matrices[0].part_ids);
    assert_eq!(
        after.matrices[0].cells[0].assemblies,
        before.matrices[0].cells[0].assemblies
    );
    let key = |doc: &ProjectDoc| {
        doc.parts
            .iter()
            .find(|p| p.id == "matrix/main/r0c0")
            .unwrap()
            .clone()
    };
    assert_eq!(key(&after).pose, key(&before).pose);
    let attached = |doc: &ProjectDoc| {
        doc.parts
            .iter()
            .find(|p| p.id == "matrix/main/r0c0/diode")
            .unwrap()
            .clone()
    };
    assert_eq!(attached(&after), attached(&before));
    assert!(
        findings
            .findings
            .iter()
            .any(|f| f.id == "input/matrix/main/r0c0/assembly-fit")
    );
    let (_, undone) = scene(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.parts, before.parts);
    let (_, redone) = scene(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.parts, after.parts);
    let mut reopened = CoreEngine::new();
    let (_, persisted) = scene(reopened.handle(CoreRequest::Open {
        id: "reload".into(),
        document: after.clone(),
    }));
    assert_eq!(persisted.parts, after.parts);
    assert_eq!(persisted.keymap, after.keymap);
}

#[test]
fn press_only_profiles_obey_matrix_direct_and_unassigned_modes() {
    let mut doc = encoder_project();
    doc.definitions[0].input_profile = Some(
        serde_json::from_value(json!({
            "press":{"row":"S1","column":"S2","independent":true}
        }))
        .unwrap(),
    );
    let mut engine = CoreEngine::new();
    scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    let change = |engine: &mut CoreEngine, revision, mode: &str| {
        scene(serde_json::from_str(&engine.request(&json!({
        "id":"mode","kind":"edit","command":{"baseRevision":revision,"transactionId":"mode","phase":"commit",
        "targetIds":["matrix/main/r0c0"],"operation":{"kind":"set-input-scan-mode","partId":"matrix/main/r0c0","mode":mode}}
    }).to_string())).unwrap()).1
    };
    let unassigned = change(&mut engine, 0, "unassigned");
    let plan = boardstudio_core::electrical::resolve(
        serde_json::from_value(json!({"document":unassigned,"boardId":"board","mode":"matrix"}))
            .unwrap(),
    );
    assert!(
        plan.assignments.is_empty(),
        "Unassigned presses must not be scanned: {:?}",
        plan.assignments
    );
    assert!(plan.peripherals.is_empty());
    let direct = change(&mut engine, 1, "direct");
    let plan = boardstudio_core::electrical::resolve(
        serde_json::from_value(json!({"document":direct,"boardId":"board","mode":"matrix"}))
            .unwrap(),
    );
    assert!(plan.assignments.is_empty());
    let press = plan
        .peripherals
        .iter()
        .find(|p| p.part_id == "matrix/main/r0c0")
        .expect("Direct press needs a GPIO requirement");
    assert_eq!(
        press.gpio_terminals,
        vec![("S1".into(), "matrix/main/r0c0/input-push".into())]
    );
    assert_eq!(press.fixed_terminals, vec![("S2".into(), "GND".into())]);
    assert!(press.rotary.is_none());
    let matrix = change(&mut engine, 2, "matrix");
    let plan = boardstudio_core::electrical::resolve(
        serde_json::from_value(json!({"document":matrix,"boardId":"board","mode":"matrix"}))
            .unwrap(),
    );
    assert_eq!(plan.assignments.len(), 1);
    assert!(plan.peripherals.is_empty());
}

#[test]
fn wired_split_encoders_cannot_reuse_either_uart_gpio() {
    let half = json!({
        "controller_profile":"ceoloide/mcu_nice_nano","board_name":"Wired encoder","diode_direction":"row2col",
        "rows":[{"terminal":"P21","gpio":"P0.31"}],"columns":[{"terminal":"P20","gpio":"P0.29"}],
        "keys":[{"id":"key","row":0,"column":0}],"key_bindings":["&kp A"],
        "transport":"wired-uart","uart_tx":{"terminal":"P1","gpio":"P0.06"},"uart_rx":{"terminal":"P0","gpio":"P0.08"}
    });
    let mut central = half.clone();
    central["peripheral"] = half;
    central["encoder_ids"] = json!(["knob"]);
    central["encoders"] = json!([{"id":"knob","aGpio":"P0.17","bGpio":"P0.20","profile":{
        "a":"A","b":"B","common":"C","steps":24,"triggersPerRotation":12,"driver":"ec11"
    }}]);
    assert!(
        boardstudio_core::firmware::generate(&serde_json::from_value(central.clone()).unwrap())
            .is_ok()
    );
    for gpio in ["P0.06", "P0.08"] {
        let mut conflict = central.clone();
        conflict["encoders"][0]["aGpio"] = json!(gpio);
        let error =
            boardstudio_core::firmware::generate(&serde_json::from_value(conflict).unwrap())
                .unwrap_err();
        assert!(error.contains("knob") && error.contains(gpio), "{error}");
    }
}
