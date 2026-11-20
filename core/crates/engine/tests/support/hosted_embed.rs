use crate::support::canonicalize;
use boardstudio_core_mirrored_half::mirrored_half::inverse::Canonical;
use boardstudio_model::model::*;
use serde_json::{Value, json};
#[path = "../../../mirrored-half/src/mirrored_half/inverse/hosted_fixtures.rs"]
mod fixtures;
use fixtures::*;

fn circuit_definition() -> Value {
    json!({"id": "circuit-definition", "name": "Circuit", "family": "f", "variant": "v",
        "source": {"repository": "https://example.invalid/c", "revision": "r",
                   "path": "c.kicad_pcb", "license": "MIT"},
        "board": {"contours": []}, "mounts": [], "volumes": [], "openings": [], "models": [],
        "gates": [], "interfaces": [],
        "electrical": {"protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3},
        "constituents": [],
        "circuit": {
            "definitions": [{"id": "cd", "name": "CD", "kind": "custom", "courtyard": [],
                "pads": [{"id": "pad-1", "number": "1", "at": {"x": 0, "y": 0},
                          "size": {"x": 1, "y": 1}, "shape": "circle"}]}],
            "parts": [part("U1", "cd", "U", 1.0, 0.0, 0.0), part("U2", "cd", "U", 2.0, 0.0, 0.0)],
            "nets": [{"id": "N1", "name": "N1", "pins": [pin("U1", "pad-1")]},
                     {"id": "SIG", "name": "SIG", "pins": [pin("U2", "pad-1")]}],
            "ports": {"signal": "SIG"}, "adaptations": []}})
}

#[test]
fn a_circuit_embedded_on_the_mirror_board_is_a_source_embed_with_a_planned_map() {
    let request = json!({"kind": "embed-module-circuit", "id": "right-extra",
        "definition": circuit_definition(), "hostBoardId": "right",
        "pose": {"at": {"x": 180, "y": 7}, "rotation": 100}, "side": "back",
        "joins": {"signal": "net-right"}});
    let document = fixture();
    let result = run(&document, &request).unwrap();
    expect(
        &document,
        request.clone(),
        json!({"kind": "embed-module-circuit", "id": "left-extra",
            "definition": circuit_definition(), "hostBoardId": "left",
            "pose": {"at": {"x": 20, "y": 7}, "rotation": 80}, "side": "back",
            "joins": {"signal": "net-left"}}),
    );
    let plan = &result.hosted.circuits[0];
    assert_eq!(plan.source_id, "left-extra");
    assert_eq!(plan.mirror_id, "right-extra");
    // Standalone mirror embedding uses its board-local allocator. The reconciled plan uses a
    // document-wide set plus the staged source references, so these allocations intentionally differ.
    let mut mirror_run = document.clone();
    boardstudio_core_mechanical::modules::embed(
        &mut mirror_run,
        "right-extra",
        &serde_json::from_value(circuit_definition()).unwrap(),
        "right",
        Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 100.0,
        },
        boardstudio_model::model::Side::Back,
        &[("signal".to_owned(), "net-right".to_owned())]
            .into_iter()
            .collect(),
    )
    .unwrap();
    for (source_member, mirror_member) in &plan.member_ids {
        let created = mirror_run
            .parts
            .iter()
            .find(|part| &part.id == mirror_member)
            .expect("the mirror embed creates the planned member");
        let standalone_reference = if source_member.ends_with("/U1") {
            "U"
        } else {
            "U1"
        };
        assert_eq!(created.reference, standalone_reference);
    }
    assert_eq!(plan.member_references["embedded/left-extra/U1"], "U3");
    assert_eq!(plan.member_references["embedded/left-extra/U2"], "U4");
    assert_eq!(plan.member_ids.len(), 2);
    assert_eq!(
        plan.member_ids["embedded/left-extra/U1"],
        "embedded/right-extra/U1"
    );
    assert_eq!(
        plan.definition_ids["embedded/left-extra/definition/cd"],
        "embedded/right-extra/definition/cd"
    );
    // The joined host net maps through its correspondence; the private net by namespace.
    assert_eq!(plan.net_ids["net-left"], "net-right");
    assert_eq!(
        plan.net_ids["embedded/left-extra/N1"],
        "embedded/right-extra/N1"
    );
    let mirror_circuit = mirror_run
        .embedded_circuits
        .iter()
        .find(|circuit| circuit.id == "right-extra")
        .unwrap();
    let mut mirror_nets: Vec<_> = plan.net_ids.values().cloned().collect();
    let mut created_nets = mirror_circuit.net_ids.clone();
    mirror_nets.sort();
    created_nets.sort();
    assert_eq!(mirror_nets, created_nets);
}

#[test]
fn mirror_circuit_references_reserve_unrelated_board_parts_and_all_members() {
    let mut document = fixture();
    let mut unrelated_board = document.boards[0].clone();
    unrelated_board.id = "other".into();
    unrelated_board.name = "Other".into();
    unrelated_board.net_ids.clear();
    unrelated_board.part_ids = vec!["other/C3".into()];
    document.boards.push(unrelated_board);

    let mut unrelated_part = document.parts[0].clone();
    unrelated_part.id = "other/C3".into();
    unrelated_part.reference = "C3".into();
    document.parts.push(unrelated_part);

    let mut definition = circuit_definition();
    for member in definition["circuit"]["parts"].as_array_mut().unwrap() {
        member["reference"] = json!("C1");
    }
    let request = EditOperation::EmbedModuleCircuit {
        id: "right-extra".into(),
        definition: serde_json::from_value(definition).unwrap(),
        host_board_id: "right".into(),
        pose: Pose2 {
            at: Vec2 { x: 180.0, y: 7.0 },
            rotation: 100.0,
        },
        side: boardstudio_model::model::Side::Back,
        joins: Default::default(),
    };

    let result = canonicalize(&document, &request).expect("the embed is accepted");
    let plan = &result.hosted.circuits[0];
    let references: Vec<_> = plan.member_references.values().cloned().collect();
    assert_eq!(references, ["C4", "C5"]);
    assert_eq!(
        document
            .parts
            .iter()
            .find(|part| part.id == "other/C3")
            .unwrap()
            .reference,
        "C3",
        "reference planning leaves unrelated board parts unchanged"
    );
    assert_eq!(plan.member_ids.len(), 2, "every member gets a planned id");
    assert_eq!(
        plan.member_references.len(),
        2,
        "every member gets a planned reference"
    );
    assert!(
        references
            .iter()
            .all(|reference| reference != "C1" && reference != "C2" && reference != "C3")
    );
}

#[test]
fn a_rotated_map_places_the_source_circuit() {
    let request = json!({"kind": "embed-module-circuit", "id": "right-extra",
        "definition": circuit_definition(), "hostBoardId": "right",
        "pose": {"at": {"x": 60, "y": -100}, "rotation": 200}, "side": "front", "joins": {}});
    let result = run(&with_mapping(ROTATED), &request).unwrap();
    let EditOperation::EmbedModuleCircuit { pose, .. } = result.operation else {
        panic!();
    };
    for (actual, expected) in [(pose.at.x, 70.0), (pose.at.y, 40.0), (pose.rotation, 70.0)] {
        assert!(
            (actual - expected).abs() < 1e-9,
            "{actual} is not {expected}"
        );
    }
}

#[test]
fn circuit_embeds_on_the_source_board_run_as_written_and_unjoinable_hosts_refuse() {
    let source = json!({"kind": "embed-module-circuit", "id": "left-extra",
        "definition": circuit_definition(), "hostBoardId": "left",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 0}, "side": "front", "joins": {}});
    let source_result = run(&fixture(), &source).expect("source-authored circuit is planned");
    assert_close(
        &serde_json::to_value(&source_result.operation).unwrap(),
        &source,
        "source-authored operation remains written as authored",
    );
    let source_plan = &source_result.hosted.circuits[0];
    assert_eq!(source_plan.source_id, "left-extra");
    assert_eq!(source_plan.mirror_id, "right-extra");
    assert_eq!(source_plan.member_ids.len(), 2);
    assert_eq!(
        source_plan.member_ids["embedded/left-extra/U1"],
        "embedded/right-extra/U1"
    );
    assert_eq!(
        source_plan.member_ids["embedded/left-extra/U2"],
        "embedded/right-extra/U2"
    );
    assert_eq!(
        source_plan.definition_ids["embedded/left-extra/definition/cd"],
        "embedded/right-extra/definition/cd"
    );
    assert_eq!(
        source_plan.net_ids["embedded/left-extra/SIG"],
        "embedded/right-extra/SIG"
    );
    assert_eq!(
        source_plan.net_ids["embedded/left-extra/N1"],
        "embedded/right-extra/N1"
    );
    assert!(
        source_plan.member_poses.is_empty(),
        "source-authored member poses remain as written"
    );
    let mut joined_source = source.clone();
    joined_source["joins"] = json!({"signal": "net-left"});
    let joined_result =
        run(&fixture(), &joined_source).expect("a joined source net reuses its exact counterpart");
    assert_eq!(
        joined_result.hosted.circuits[0].net_ids["net-left"],
        "net-right"
    );

    let mut unjoinable_source = source.clone();
    unjoinable_source["joins"] = json!({"signal": "net-right"});
    let message = run(&fixture(), &unjoinable_source)
        .expect_err("a source circuit cannot join a mirror-only net")
        .to_string();
    assert!(
        message.contains("net-right") && message.contains("no mirror"),
        "{message}"
    );

    for (state, label) in [
        (MirroredExceptionState::Local, "local"),
        (MirroredExceptionState::Omitted, "omitted"),
    ] {
        let message = run(&with_exception("net", "net-right", state), &joined_source)
            .expect_err("source circuits cannot join a Local or Omitted counterpart")
            .to_string();
        assert!(message.contains(label), "{message}");
    }

    let mut mirror = json!({"kind": "embed-module-circuit", "id": "right-extra",
        "definition": circuit_definition(), "hostBoardId": "right",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 0}, "side": "front",
        "joins": {"signal": "right-only-net"}});
    let message = refused(&fixture(), mirror.clone());
    assert!(message.contains("right-only-net"), "{message}");
    // The mirror namespace must be free so the forward transition can create it.
    mirror["joins"] = json!({});
    mirror["id"] = json!("right-circuit");
    unchanged(&fixture(), mirror);
}

/// Ticket 146: a circuit whose members sit off its x axis and are turned inside it.
fn circuit_with_placed_members() -> Value {
    let mut definition = circuit_definition();
    definition["circuit"]["parts"][0]["pose"] = json!({"at": {"x": 1.5, "y": 2.0}, "rotation": 30});
    definition["circuit"]["parts"][1]["pose"] =
        json!({"at": {"x": -2.0, "y": -1.25}, "rotation": 75});
    definition
}

/// Heading difference modulo 360.
fn heading_gap(a: f64, b: f64) -> f64 {
    let gap = (a - b).rem_euclid(360.0);
    gap.min(360.0 - gap)
}

#[test]
fn a_mirror_embed_lands_every_member_exactly_as_typed() {
    let maps = [
        AXIS,
        ROTATED,
        Pose2 {
            at: Vec2 { x: 50.0, y: 20.0 },
            rotation: 30.0,
        },
        Pose2 {
            at: Vec2 { x: -40.0, y: 15.0 },
            rotation: 0.0,
        },
    ];
    let definition: boardstudio_core_mechanical::modules::ModuleDefinition =
        serde_json::from_value(circuit_with_placed_members()).unwrap();
    for mapping in maps {
        for side in [
            boardstudio_model::model::Side::Front,
            boardstudio_model::model::Side::Back,
        ] {
            let document = with_mapping(mapping);
            let typed = Pose2 {
                at: Vec2 { x: 180.0, y: 7.0 },
                rotation: 100.0,
            };
            let request = EditOperation::EmbedModuleCircuit {
                id: "right-extra".into(),
                definition: definition.clone(),
                host_board_id: "right".into(),
                pose: typed,
                side: side.clone(),
                joins: Default::default(),
            };
            let result = canonicalize(&document, &request).expect("the embed is accepted");
            let plan = &result.hosted.circuits[0];
            let EditOperation::EmbedModuleCircuit {
                id,
                definition: shared,
                host_board_id,
                pose,
                side: source_side,
                joins,
            } = result.operation
            else {
                panic!();
            };
            // The shared definition travels unchanged and registers once.
            assert_eq!(shared, definition);
            // What the user typed, member by member: a real embed on the mirror half.
            let mut mirror = document.clone();
            boardstudio_core_mechanical::modules::embed(
                &mut mirror,
                "right-extra",
                &definition,
                "right",
                typed,
                side.clone(),
                &Default::default(),
            )
            .unwrap();
            // The prepared source embed with its overrides applied, then the forward rule.
            let mut source = document.clone();
            boardstudio_core_mechanical::modules::embed(
                &mut source,
                &id,
                &shared,
                &host_board_id,
                pose,
                source_side,
                &joins,
            )
            .unwrap();
            assert_eq!(plan.member_poses.len(), 2, "{mapping:?} {side:?}");
            for part in &mut source.parts {
                if let Some(over) = plan.member_poses.get(&part.id) {
                    part.pose = *over;
                }
            }
            for member in ["U1", "U2"] {
                let typed_member = mirror
                    .parts
                    .iter()
                    .find(|part| part.id == format!("embedded/right-extra/{member}"))
                    .unwrap();
                let source_member = source
                    .parts
                    .iter()
                    .find(|part| part.id == format!("embedded/{id}/{member}"))
                    .unwrap();
                let landed = boardstudio_core_mirrored_half::mirrored_half::mirror_pose(
                    mapping,
                    source_member.pose,
                );
                let context = format!("{member} {mapping:?} {side:?}");
                assert!(
                    (landed.at.x - typed_member.pose.at.x).abs() < 1e-9,
                    "{context}"
                );
                assert!(
                    (landed.at.y - typed_member.pose.at.y).abs() < 1e-9,
                    "{context}"
                );
                assert!(
                    heading_gap(landed.rotation, typed_member.pose.rotation) < 1e-9,
                    "{context}: {} against {}",
                    landed.rotation,
                    typed_member.pose.rotation
                );
                assert_eq!(source_member.side, typed_member.side, "{context}");
            }
        }
    }
}

#[test]
fn a_rotated_map_source_half_embed_runs_as_written() {
    let request = json!({"kind": "embed-module-circuit", "id": "left-extra",
        "definition": circuit_definition(), "hostBoardId": "left",
        "pose": {"at": {"x": 1, "y": 2}, "rotation": 33}, "side": "front", "joins": {}});
    unchanged(&with_mapping(ROTATED), request);
}
