//! Test-only wiring fixtures shared by the queued-edit browser tests and native mode-owner tests.

use boardstudio_core::model::*;
use std::collections::BTreeMap;

pub(crate) fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Mode test");
    document.definitions.push(test_definition(
        "switch-definition",
        PartKind::Switch,
        BTreeMap::from([
            ("row".into(), vec!["1".into()]),
            ("column".into(), vec!["2".into()]),
        ]),
        Some(MatrixTerminals {
            row: "row".into(),
            column: "column".into(),
        }),
        None,
    ));
    document.definitions.push(test_definition(
        "diode-definition",
        PartKind::Passive,
        BTreeMap::from([
            ("anode".into(), vec!["A".into()]),
            ("cathode".into(), vec!["K".into()]),
        ]),
        None,
        None,
    ));
    document.definitions.push(test_definition(
        "mcu-definition",
        PartKind::Controller,
        [
            "GND", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P14", "P15",
            "P16", "P18", "P19", "P20", "P21",
        ]
        .into_iter()
        .map(|pin| (pin.to_owned(), vec![pin.to_owned()]))
        .collect(),
        None,
        Some("ceoloide/mcu_nice_nano"),
    ));
    let electrical_parts = [
        test_part("matrix/m/r0c0", "switch-definition"),
        test_part("matrix/m/r0c0/diode", "diode-definition"),
        test_part("mcu-left", "mcu-definition"),
    ];
    document.boards.push(Board {
        id: "left".into(),
        name: "Left".into(),
        outline_ids: vec![],
        part_ids: electrical_parts
            .iter()
            .map(|part| part.id.clone())
            .collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.extend(electrical_parts);
    document.boards.push(Board {
        id: "right".into(),
        name: "Right".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.matrices.push(Matrix {
        id: "m".into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch-definition".into(),
        part_ids: vec!["matrix/m/r0c0".into(), "matrix/m/r0c0/diode".into()],
        board_id: Some("left".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: Some(DiodeDirection::Row2col),
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("switch-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    });
    document
}

pub(crate) fn test_part(id: &str, definition_id: &str) -> Part {
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
        anchor: None,
        properties: None,
        generator_parameters: None,
    }
}

pub(crate) fn test_definition(
    id: &str,
    kind: PartKind,
    terminals: BTreeMap<String, Vec<String>>,
    matrix_terminals: Option<MatrixTerminals>,
    generator_source: Option<&str>,
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
        matrix_terminals,
        envelope_notice: None,
        courtyard: vec![],
        pads,
        models: None,
        generator: generator_source.map(|source| PartGenerator {
            source: source.into(),
            version: "test".into(),
            parameters: BTreeMap::new(),
        }),
        mechanical_profile: None,
    }
}
