#[allow(dead_code)]
#[path = "support.rs"]
mod support;

use crate::support::fixture;
use boardstudio_core::CoreEngine;
use boardstudio_core::model::*;
use std::time::Instant;

const SAMPLES: usize = 100;
const WARMUP: usize = 10;
const KEY_PITCH_MM: f64 = 19.05;

fn fixture(keys: usize) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("bench", "Benchmark");
    doc.definitions.push(PartDefinition {
        courtyard: vec![
            Vec2 { x: -7.0, y: -7.0 },
            Vec2 { x: 7.0, y: -7.0 },
            Vec2 { x: 7.0, y: 7.0 },
            Vec2 { x: -7.0, y: 7.0 },
        ],
        ..fixture::part_definition("switch", "Switch", PartKind::Switch)
    });
    for index in 0..keys {
        doc.parts.push(Part {
            pose: Pose2 {
                at: Vec2 {
                    x: (index % 20) as f64 * KEY_PITCH_MM,
                    y: (index / 20) as f64 * KEY_PITCH_MM,
                },
                rotation: 0.0,
            },
            ..fixture::part(format!("key-{index}"), "switch", format!("SW{}", index + 1))
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
    engine.handle(fixture::open(fixture(keys)));
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
                        x: (revision + 1) as f64 * 0.1,
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
