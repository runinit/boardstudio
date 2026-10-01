use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;
use std::time::Instant;

const SAMPLES: usize = 100;
const WARMUP: usize = 10;
const KEY_PITCH_MM: f64 = 19.05;

fn fixture(keys: usize) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("bench", "Benchmark");
    doc.definitions.push(PartDefinition {
        input_profile: None,
        mechanical_profile: None,
        id: "switch".into(),
        name: "Switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
        pads: vec![],
        models: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        generator: None,
    });
    for index in 0..keys {
        doc.parts.push(Part {
            id: format!("key-{index}"),
            definition_id: "switch".into(),
            reference: format!("SW{}", index + 1),
            pose: Pose2 {
                at: Vec2 {
                    x: (index % 20) as f64 * KEY_PITCH_MM,
                    y: (index / 20) as f64 * KEY_PITCH_MM,
                },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
    }
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        id: "boundary".into(),
        part_ids: doc.parts.iter().map(|part| part.id.clone()).collect(),
        settings: Default::default(),
        margin: 3.0,
        operation: Operation::Add,
    });
    doc
}

fn measure(keys: usize) {
    let mut engine = CoreEngine::new();
    engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: fixture(keys),
    });
    let mut samples = Vec::with_capacity(SAMPLES);
    for revision in 0..(SAMPLES + WARMUP) {
        let command = EditCommand {
            base_revision: revision as u64,
            transaction_id: format!("move-{revision}"),
            phase: EditPhase::Commit,
            target_ids: vec!["key-0".into()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id: "key-0".into(),
                    at: Vec2 {
                        x: revision as f64 * 0.1,
                        y: 0.0,
                    },
                }],
            },
        };
        let start = Instant::now();
        let reply = engine.handle(CoreRequest::Edit {
            id: "edit".into(),
            command,
        });
        let elapsed = start.elapsed();
        assert!(matches!(reply, CoreReply::Scene { .. }));
        if revision >= WARMUP {
            samples.push(elapsed.as_micros());
        }
    }
    samples.sort_unstable();
    println!(
        "{keys} keys, {SAMPLES} samples: p50={} µs, p95={} µs",
        samples[SAMPLES / 2],
        samples[SAMPLES * 95 / 100]
    );
}

#[test]
#[ignore = "Run with cargo test --release --test performance -- --ignored --nocapture"]
fn outline_100_keys() {
    measure(100);
}

#[test]
#[ignore = "Run with cargo test --release --test performance -- --ignored --nocapture"]
fn outline_200_keys() {
    measure(200);
}
