use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;
use std::time::Instant;

const SAMPLES: usize = 100;
const PITCH_MM: f64 = 19.05;

fn pad(id: &str) -> Pad {
    Pad {
        id: id.into(),
        number: id.into(),
        at: Vec2::default(),
        size: Vec2 { x: 1.0, y: 1.0 },
        shape: PadShape::Circle,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: None,
    }
}

fn definition(id: &str, pads: Vec<Pad>) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: id.into(),
        name: id.into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
        pads,
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    }
}

#[test]
#[ignore = "Run with cargo test --release --test matrix_performance -- --ignored --nocapture"]
fn guided_matrix_preview() {
    let mut engine = CoreEngine::new();
    let mut doc = ProjectDoc::empty("bench", "Matrix benchmark");
    doc.definitions
        .push(definition("mx-switch", vec![pad("one"), pad("two")]));
    doc.definitions.push(definition(
        "matrix-diode",
        vec![pad("anode"), pad("cathode")],
    ));
    doc.definitions.push(definition(
        "rgb-led",
        vec![pad("vdd"), pad("gnd"), pad("din"), pad("dout")],
    ));
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "edge".into(),
        part_ids: vec![],
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec!["edge".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    });
    let cells = (0..6)
        .flat_map(|row| {
            (0..5).map(move |column| MatrixCell {
                assemblies_local: None,
                row,
                column,
                enabled: true,

                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![MatrixAssembly {
                    id: "led".into(),
                    definition_id: "rgb-led".into(),
                    offset: Vec2 { x: 0.0, y: 8.0 },
                    rotation: None,
                    side: None,
                }],
            })
        })
        .collect();
    let matrix = Matrix {
        id: "main".into(),
        name: None,
        rows: 6,
        columns: 5,
        pitch: Vec2 {
            x: PITCH_MM,
            y: PITCH_MM,
        },
        origin: Vec2::default(),
        definition_id: "mx-switch".into(),
        part_ids: vec![],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: None,
        row_offsets: vec![],
        column_offsets: vec![],
        column_staggers: vec![],
        column_splays: vec![],
        column_origins: vec![],
        cells,
    };
    assert!(matches!(
        engine.handle(CoreRequest::Edit {
            id: "create".into(),
            command: EditCommand {
                base_revision: 0,
                transaction_id: "create".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: EditOperation::SetMatrix {
                    matrix,
                    definitions: None
                },
            }
        }),
        CoreReply::Scene { .. }
    ));
    let mut samples = Vec::with_capacity(SAMPLES);
    for index in 0..SAMPLES + 10 {
        let start = Instant::now();
        let reply = engine.handle(CoreRequest::Edit {
            id: "preview".into(),
            command: EditCommand {
                base_revision: 1,
                transaction_id: "drag".into(),
                phase: EditPhase::Preview,
                target_ids: vec!["matrix/main/r0c0".into()],
                operation: EditOperation::MoveParts {
                    positions: vec![Position {
                        id: "matrix/main/r0c0".into(),
                        at: Vec2 {
                            x: index as f64 * 0.1,
                            y: 0.0,
                        },
                    }],
                },
            },
        });
        assert!(matches!(reply, CoreReply::Preview { .. }));
        if index >= 10 {
            samples.push(start.elapsed().as_micros());
        }
    }
    samples.sort_unstable();
    println!(
        "6x5 with diode and RGB (90 parts) preview: p50={} µs, p95={} µs",
        samples[SAMPLES / 2],
        samples[SAMPLES * 95 / 100]
    );
}
