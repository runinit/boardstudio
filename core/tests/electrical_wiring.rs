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
    doc.definitions.push(definition(
        "diode",
        PartKind::Passive,
        diode_terms,
        None,
    ));
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
        .map(|assignment| (assignment.key_id.as_str(), (assignment.row_pin.as_str(), assignment.column_pin.as_str())))
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
    let row_zero = plan.nets.iter().find(|net| net.id.ends_with("/row/0")).unwrap();
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
    assert!(plan.diagnostics.iter().all(|item| item.code != "protected-pin-change"));
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
