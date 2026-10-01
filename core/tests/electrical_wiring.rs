use boardstudio_core::CoreEngine;
use boardstudio_core::electrical::{ElectricalMode, ElectricalPlanRequest};
use boardstudio_core::model::*;
use std::collections::{BTreeMap, BTreeSet};

fn definition(
    id: &str,
    kind: PartKind,
    terminals: BTreeMap<String, Vec<String>>,
    source: Option<&str>,
) -> PartDefinition {
    let pads = terminals
        .values()
        .flatten()
        .map(|id| Pad {
            id: id.clone(),
            number: id.clone(),
            at: Vec2::default(),
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Circle,
            drill: None,
            plated: Some(true),
            side: None,
            rotation: None,
            net_id: None,
        })
        .collect();
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals,
        matrix_terminals: Some(MatrixTerminals {
            row: "row".into(),
            column: "column".into(),
        }),
        envelope_notice: None,
        courtyard: vec![],
        pads,
        models: None,
        generator: source.map(|s| PartGenerator {
            source: s.into(),
            version: "test".into(),
            parameters: BTreeMap::new(),
        }),
        mechanical_profile: None,
    }
}

fn part(id: &str, definition_id: &str) -> Part {
    Part {
        id: id.into(),
        definition_id: definition_id.into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    }
}

fn wired_document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("wiring", "Wiring");
    let mut sw_terms = BTreeMap::new();
    sw_terms.insert("row".into(), vec!["1".into()]);
    sw_terms.insert("column".into(), vec!["2".into()]);
    doc.definitions
        .push(definition("switch", PartKind::Switch, sw_terms, None));
    let mut diode_terms = BTreeMap::new();
    diode_terms.insert("anode".into(), vec!["A".into()]);
    diode_terms.insert("cathode".into(), vec!["K".into()]);
    doc.definitions
        .push(definition("diode", PartKind::Passive, diode_terms, None));
    let mut mcu_terms = BTreeMap::new();
    for p in [
        "GND", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P14", "P15", "P16",
        "P18", "P19", "P20", "P21",
    ] {
        mcu_terms.insert(p.into(), vec![p.into()]);
    }
    doc.definitions.push(definition(
        "mcu",
        PartKind::Controller,
        mcu_terms,
        Some("ceoloide/mcu_nice_nano"),
    ));
    let mut matrix_parts = vec![];
    for r in 0..2 {
        for c in 0..2 {
            let id = format!("matrix/m/r{r}c{c}");
            matrix_parts.push(id.clone());
            doc.parts.push(part(&id, "switch"));
            let did = format!("{id}/diode");
            matrix_parts.push(did.clone());
            doc.parts.push(part(&did, "diode"));
        }
    }
    doc.parts.push(part("mcu-left", "mcu"));
    doc.boards.push(Board {
        id: "board-a".into(),
        name: "A".into(),
        outline_ids: vec![],
        part_ids: doc.parts.iter().map(|p| p.id.clone()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.matrices.push(Matrix {
        id: "m".into(),
        name: None,
        rows: 2,
        columns: 2,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: matrix_parts,
        board_id: Some("board-a".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: Some(DiodeDirection::Row2col),
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells: (0..2)
            .flat_map(|r| {
                (0..2).map(move |c| MatrixCell {
                    row: r,
                    column: c,
                    enabled: true,

                    definition_id: Some("switch".into()),
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                })
            })
            .collect(),
    });
    doc
}

fn resolve_wired(
    document: ProjectDoc,
    mode: ElectricalMode,
) -> boardstudio_core::electrical::ElectricalPlan {
    boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document,
        mode,
        locks: BTreeMap::new(),
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        board_id: Some("board-a".into()),
        controller_part_id: Some("mcu-left".into()),
    })
}

fn add_peripheral(document: &mut ProjectDoc, id: &str, source: &str, terminals: &[&str]) {
    let definition_id = format!("def/{id}");
    let terminal_map = terminals
        .iter()
        .map(|terminal| ((*terminal).into(), vec![(*terminal).into()]))
        .collect();
    document.definitions.push(definition(
        &definition_id,
        PartKind::Utility,
        terminal_map,
        Some(source),
    ));
    document.parts.push(part(id, &definition_id));
    document.boards[0].part_ids.push(id.into());
}

#[test]
fn rgb_chain_and_battery_power_switch_map_to_shared_power_nets() {
    let mut document = wired_document();
    let controller = document
        .definitions
        .iter_mut()
        .find(|item| item.id == "mcu")
        .unwrap();
    for terminal in ["RAW", "VCC"] {
        controller
            .terminals
            .insert(terminal.into(), vec![terminal.into()]);
        let mut pad = controller.pads[0].clone();
        pad.id = terminal.into();
        pad.number = terminal.into();
        controller.pads.push(pad);
    }
    add_peripheral(
        &mut document,
        "rgb-a",
        "ceoloide/led_sk6812mini-e",
        &["P1", "P2", "P3", "P4"],
    );
    add_peripheral(
        &mut document,
        "rgb-b",
        "ceoloide/led_sk6812mini-e",
        &["P1", "P2", "P3", "P4"],
    );
    add_peripheral(
        &mut document,
        "battery",
        "ceoloide/battery_connector_jst_ph_2",
        &["BAT_P", "BAT_N"],
    );
    add_peripheral(
        &mut document,
        "power",
        "ceoloide/power_switch_smd_side",
        &["from", "to"],
    );
    let plan = resolve_wired(document.clone(), ElectricalMode::Matrix);
    let net = |suffix: &str| {
        plan.nets
            .iter()
            .find(|net| net.id.ends_with(suffix))
            .unwrap()
    };
    let rgb_chain = net("/rgb-chain/rgb-a");
    assert!(
        rgb_chain
            .pins
            .iter()
            .any(|pin| pin.part_id == "rgb-a" && pin.pad_id == "P2")
    );
    assert!(
        rgb_chain
            .pins
            .iter()
            .any(|pin| pin.part_id == "rgb-b" && pin.pad_id == "P4")
    );
    let battery = net("/power/bat_p");
    assert!(battery.pins.iter().any(|pin| pin.part_id == "battery"));
    assert!(battery.pins.iter().any(|pin| pin.part_id == "power"));
    let raw = net("/power/raw");
    assert!(
        raw.pins
            .iter()
            .any(|pin| pin.part_id == "power" && pin.pad_id == "to")
    );
    assert!(
        raw.pins
            .iter()
            .any(|pin| pin.part_id == "mcu-left" && pin.pad_id == "RAW")
    );
    assert!(!raw.pins.iter().any(|pin| pin.part_id == "battery"));
    assert_eq!(rgb_chain.pins.len(), 2);
    assert!(
        plan.peripherals
            .iter()
            .find(|item| item.part_id == "rgb-b")
            .unwrap()
            .gpio_terminals
            .is_empty()
    );

    // Without a switch, BAT_P must connect directly to the controller RAW terminal.
    document.parts.retain(|part| part.id != "power");
    document.boards[0].part_ids.retain(|id| id != "power");
    let unswitched = resolve_wired(document, ElectricalMode::Matrix);
    assert!(
        !unswitched
            .nets
            .iter()
            .any(|net| net.id.ends_with("/power/bat_p"))
    );
    let raw = unswitched
        .nets
        .iter()
        .find(|net| net.id.ends_with("/power/raw"))
        .unwrap();
    assert!(
        raw.pins
            .iter()
            .any(|pin| pin.part_id == "battery" && pin.pad_id == "BAT_P")
    );
    assert!(
        raw.pins
            .iter()
            .any(|pin| pin.part_id == "mcu-left" && pin.pad_id == "RAW")
    );
}

#[test]
fn direct_mode_constructs_controller_and_ground_nets_deterministically() {
    let first = resolve_wired(wired_document(), ElectricalMode::Direct);
    let second = resolve_wired(wired_document(), ElectricalMode::Direct);
    assert_eq!(first.nets, second.nets);
    assert!(
        first
            .nets
            .iter()
            .any(|net| net.id.ends_with("/direct/matrix/m/r0c0"))
    );
    assert!(first.nets.iter().any(|net| net.id.ends_with("/power/gnd")));
    assert_eq!(first.diagnostics, second.diagnostics);
    assert_eq!(first.assignments, second.assignments);
    let assignment = first
        .assignments
        .iter()
        .find(|item| item.key_id == "matrix/m/r0c0")
        .unwrap();
    let direct = first
        .nets
        .iter()
        .find(|net| net.id.ends_with("/direct/matrix/m/r0c0"))
        .unwrap();
    assert_eq!(
        direct
            .pins
            .iter()
            .map(|pin| (pin.part_id.as_str(), pin.pad_id.as_str()))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ("matrix/m/r0c0", "2"),
            ("mcu-left", assignment.column_pin.as_str()),
        ])
    );
}

#[test]
fn reverse_diode_direction_places_cathodes_on_matrix_rows() {
    let mut document = wired_document();
    document.matrices[0].diode_direction = Some(DiodeDirection::Col2row);
    let plan = resolve_wired(document, ElectricalMode::Matrix);
    let row = plan
        .nets
        .iter()
        .find(|net| net.id.ends_with("/row/0"))
        .unwrap();
    assert!(
        row.pins
            .iter()
            .any(|pin| pin.part_id.ends_with("/diode") && pin.pad_id == "K")
    );
    let link = plan
        .nets
        .iter()
        .find(|net| net.id.ends_with("/link/matrix/m/r0c0"))
        .unwrap();
    assert!(
        link.pins
            .iter()
            .any(|pin| pin.part_id.ends_with("/diode") && pin.pad_id == "A")
    );
}

#[test]
fn resolver_scopes_board_and_honors_controller_and_locks() {
    let doc = wired_document();
    let mut locks = BTreeMap::new();
    locks.insert("row/0".into(), "P1".into());
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: doc,
        mode: ElectricalMode::Matrix,
        locks,
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        board_id: Some("board-a".into()),
        controller_part_id: Some("mcu-left".into()),
    });
    assert_eq!(plan.assignments.len(), 4);
    let assignment_pins = plan
        .assignments
        .iter()
        .map(|assignment| {
            (
                assignment.key_id.as_str(),
                (assignment.row_pin.as_str(), assignment.column_pin.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        assignment_pins,
        BTreeMap::from([
            ("matrix/m/r0c0", ("P1", "P21")),
            ("matrix/m/r0c1", ("P19", "P21")),
            ("matrix/m/r1c0", ("P1", "P20")),
            ("matrix/m/r1c1", ("P19", "P20")),
        ])
    );
    let row_zero = plan
        .nets
        .iter()
        .find(|net| net.id.ends_with("/row/0"))
        .unwrap();
    let row_zero_pins = row_zero
        .pins
        .iter()
        .map(|pin| (pin.part_id.as_str(), pin.pad_id.as_str()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        row_zero_pins,
        BTreeSet::from([
            ("matrix/m/r0c0/diode", "A"),
            ("matrix/m/r1c0/diode", "A"),
            ("mcu-left", "P1"),
        ])
    );
    assert!(
        plan.diagnostics
            .iter()
            .all(|item| item.code != "protected-pin-change")
    );
    assert!(plan.assignments.iter().any(|a| a.locked));
    assert!(
        plan.diagnostics
            .iter()
            .all(|d| d.code != "matrix-diode-required")
    );
}

#[test]
fn apply_materializes_switch_diode_and_controller_pins_and_preserves_manual_net() {
    let mut doc = wired_document();
    doc.nets.push(Net {
        id: "manual/net".into(),
        name: "MANUAL".into(),
        pins: vec![],
    });
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: doc.clone(),
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::new(),
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        board_id: Some("board-a".into()),
        controller_part_id: Some("mcu-left".into()),
    });
    boardstudio_core::electrical::materialize(&mut doc, &plan).unwrap();
    assert!(doc.nets.iter().any(|n| n.id == "manual/net"));
    let row = doc
        .nets
        .iter()
        .find(|n| n.id.contains("/row/0"))
        .expect("row net");
    assert!(row.pins.iter().any(|p| p.part_id == "mcu-left"));
    let controller = doc.parts.iter().find(|p| p.id == "mcu-left").unwrap();
    assert_eq!(
        controller
            .generator_parameters
            .as_ref()
            .and_then(|p| p.get(&plan.row_pins[0])),
        Some(&serde_json::json!(row.name))
    );
    assert!(doc.nets.iter().any(|n| n.id.contains("link/matrix/m/r0c0")
        && n.pins.iter().any(|p| p.part_id.ends_with("/diode"))));
}

#[test]
fn unmanaged_nets_are_not_removed_by_automatic_wiring() {
    let mut managed = wired_document();
    managed.hardware = Some(HardwareConfiguration {
        boards: vec![ElectricalBoardConfiguration {
            board_id: "board-a".into(),
            controller_part_id: Some("mcu-left".into()),
            mode: ElectricalMode::Matrix,
            ..Default::default()
        }],
        ..Default::default()
    });
    managed.nets.push(Net {
        id: "matrix/m/net/led/in".into(),
        name: "Unmanaged net".into(),
        pins: vec![],
    });
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: managed.clone(),
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::new(),
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        board_id: Some("board-a".into()),
        controller_part_id: Some("mcu-left".into()),
    });
    boardstudio_core::electrical::materialize(&mut managed, &plan).unwrap();
    assert!(
        managed
            .nets
            .iter()
            .any(|net| net.id == "matrix/m/net/led/in")
    );

    let mut unrelated = wired_document();
    unrelated.hardware = Some(HardwareConfiguration {
        boards: vec![ElectricalBoardConfiguration {
            board_id: "another-board".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
    unrelated.nets.push(Net {
        id: "matrix/m/net/led/in".into(),
        name: "Unmanaged net".into(),
        pins: vec![],
    });
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: unrelated.clone(),
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::new(),
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        board_id: Some("board-a".into()),
        controller_part_id: Some("mcu-left".into()),
    });
    boardstudio_core::electrical::materialize(&mut unrelated, &plan).unwrap();
    assert!(
        unrelated
            .nets
            .iter()
            .any(|net| net.id == "matrix/m/net/led/in")
    );
}

#[test]
fn missing_controller_is_reported_without_inventing_one() {
    let mut doc = wired_document();
    doc.parts.retain(|part| part.id != "mcu-left");
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: doc,
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::new(),
        controller_profile: None,
        board_id: Some("board-a".into()),
        controller_part_id: None,
    });
    assert!(
        plan.diagnostics
            .iter()
            .any(|d| d.code == "missing-controller"),
        "missing controller must block a complete handoff"
    );
}

#[test]
fn apply_request_is_reversible() {
    let mut engine = CoreEngine::new();
    let doc = wired_document();
    let opened = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let doc = match opened {
        CoreReply::Scene { document, .. } => document,
        other => panic!("{other:?}"),
    };
    let base = doc.revision;
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: doc,
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::new(),
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        board_id: Some("board-a".into()),
        controller_part_id: Some("mcu-left".into()),
    });
    let applied = engine.handle(CoreRequest::ApplyElectrical {
        id: "apply".into(),
        base_revision: base,
        plan,
        draft: false,
    });
    assert!(matches!(applied, CoreReply::Scene { .. }), "{applied:?}");
    assert!(matches!(
        engine.handle(CoreRequest::Undo { id: "undo".into() }),
        CoreReply::Scene { .. }
    ));
}

#[test]
fn handoff_cannot_be_silently_overridden_by_new_locks() {
    let mut doc = wired_document();
    doc.hardware = Some(HardwareConfiguration {
        boards: vec![ElectricalBoardConfiguration {
            board_id: "board-a".into(),
            protected_handoff: Some(ElectricalHandoffBaseline {
                fingerprint: "sent".into(),
                revision: 0,
                assignments: BTreeMap::from([("row/0".into(), "P0.06".into())]),
            }),
            ..Default::default()
        }],
        ..Default::default()
    });
    let plan = boardstudio_core::electrical::resolve(ElectricalPlanRequest {
        instance_id: None,
        document: doc,
        mode: ElectricalMode::Matrix,
        locks: BTreeMap::from([("row/0".into(), "P2".into())]),
        controller_profile: None,
        board_id: Some("board-a".into()),
        controller_part_id: None,
    });
    assert!(
        plan.diagnostics
            .iter()
            .any(|item| item.code == "protected-pin-change")
    );
}
