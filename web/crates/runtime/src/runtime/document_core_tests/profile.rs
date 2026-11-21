use super::*;
use boardstudio_model::model::{EditCommand, EditPhase, OutlineFeature, RowLayout};

#[derive(Clone, Copy)]
pub(in crate::runtime) enum Phase {
    Core,
    PrepareSave,
    PublishSnapshot,
    DeliverSettlement,
}

thread_local! {
    static CLOCK: (JsValue, Function) = {
        let performance =
            Reflect::get(web_sys::window().unwrap().as_ref(), &"performance".into()).unwrap();
        let now = Reflect::get(&performance, &"now".into()).unwrap().dyn_into().unwrap();
        (performance, now)
    };
    static SAMPLE: Cell<Option<[Option<f64>; 4]>> = const { Cell::new(None) };
}

fn now() -> f64 {
    CLOCK.with(|(performance, now)| now.call0(performance).unwrap().as_f64().unwrap())
}

pub(in crate::runtime) struct Timer(Phase, f64);

impl Timer {
    pub(in crate::runtime) fn start(phase: Phase) -> Option<Self> {
        SAMPLE.get().map(|_| Self(phase, now()))
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = now() - self.1;
        let mut sample = SAMPLE.get().unwrap();
        assert!(sample[self.0 as usize].replace(elapsed).is_none());
        SAMPLE.set(Some(sample));
    }
}

fn fixture(large: bool) -> ProjectDoc {
    let mut document: ProjectDoc = serde_json::from_str(include_str!(
        "../../../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .unwrap();
    if !large {
        return document;
    }
    let mut matrix = document.matrices[0].clone();
    matrix.rows = 5;
    matrix.columns = 12;
    matrix.row_layout = vec![
        RowLayout::default(),
        RowLayout {
            shift: boardstudio_model::model::Vec2 { x: 4.75, y: 0.0 },
            ..Default::default()
        },
    ];
    // Use the authoring path to materialize a 60-key board with the same envelope
    // type as the small control. The setup commits are outside the sample window.
    let mut engine = boardstudio_core::CoreEngine::new();
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "fixture-open".into(),
            document,
        }),
        CoreReply::Scene { .. }
    ));
    document = match engine.handle(CoreRequest::Edit {
        id: "fixture-resize".into(),
        command: EditCommand {
            base_revision: 0,
            transaction_id: "fixture-resize".into(),
            phase: EditPhase::Commit,
            target_ids: vec![matrix.id.clone()],
            operation: EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        },
    }) {
        CoreReply::Scene { document, .. } => *document,
        reply => panic!("could not build large fixture: {reply:?}"),
    };
    assert_eq!(document.matrices[0].part_ids.len(), 60);
    let OutlineFeature::PartEnvelope { part_ids, .. } = &mut document.outline[0] else {
        panic!("expected a part envelope");
    };
    *part_ids = document.parts.iter().map(|part| part.id.clone()).collect();
    document.id.push_str("-60-keys");
    document.revision = 0;
    document
}

#[wasm_bindgen_test]
async fn page_core_profiles_rename_and_stagger_submission() {
    for (size, large) in [("2 keys", false), ("60 keys", true)] {
        let mut engine = boardstudio_core::CoreEngine::new();
        let runtime = runtime_with_core(Box::new(move |request| {
            let _timer = Timer::start(Phase::Core);
            engine.handle(request)
        }));
        let document = fixture(large);
        let opened = submit(&runtime, |operation_id| Event::Open {
            operation_id,
            document,
        });
        assert_eq!(wait_for_outcome(&opened).await, TerminalOutcome::Completed);

        for label in ["rename", "column stagger"] {
            let mut samples: [Vec<f64>; 5] = std::array::from_fn(|_| Vec::with_capacity(100));
            for i in 0..124 {
                let initial = runtime.model().accepted.unwrap();
                let revision = initial.document.revision + 1;
                let name = ["First name", "Second name"][i % 2];
                let value = [2.0, 4.0][i % 2];
                SAMPLE.set(Some([None; 4]));
                let start = now();
                let ticket = if label == "rename" {
                    rename(&runtime, name)
                } else {
                    stagger(&runtime, "matrix", value)
                };
                let elapsed = now() - start;
                assert_eq!(runtime.model().durability, Durability::Saving { revision });
                assert_eq!(runtime.model().accepted.unwrap().token, initial.token);
                let outcome = runtime.observe_operation(ticket.operation());
                assert_eq!(wait_for_outcome(&outcome).await, TerminalOutcome::Completed);
                assert_eq!(ticket.settlement(true), Settlement::Landed { revision });
                let accepted = runtime.model().accepted.unwrap();
                assert_ne!(accepted.token, initial.token);
                assert_eq!(accepted.session_epoch, initial.session_epoch);
                assert_eq!(accepted.scene.revision, revision);
                if label == "rename" {
                    assert_eq!(accepted.document.name, name);
                } else {
                    assert_eq!(
                        accepted.document.matrices[0].column_layout[0].stagger,
                        value
                    );
                }
                let stages = SAMPLE.take().unwrap().map(Option::unwrap);
                if i >= 24 {
                    for (sample, value) in
                        samples.iter_mut().zip([elapsed].into_iter().chain(stages))
                    {
                        sample.push(value);
                    }
                }
            }
            for (phase, samples) in [
                "ticket submit",
                "Core handle",
                "prepare save + notify",
                "publish snapshot + notify",
                "deliver settlement",
            ]
            .into_iter()
            .zip(&mut samples)
            {
                samples.sort_by(f64::total_cmp);
                web_sys::console::log_1(
                    &format!(
                        "{size} / {label} / {phase}: n=100, p50={:.3} ms, p95={:.3} ms",
                        samples[49], samples[94],
                    )
                    .into(),
                );
            }
        }
    }
}

#[wasm_bindgen_test]
fn page_core_profiles_outline_rebuild_in_isolation() {
    for (size, large) in [("2 keys", false), ("60 keys", true)] {
        let mut engine = boardstudio_core::CoreEngine::new();
        let document = match engine.handle(CoreRequest::Open {
            id: "outline-profile".into(),
            document: fixture(large),
        }) {
            CoreReply::Scene { document, .. } => document,
            reply => panic!("could not open outline profile fixture: {reply:?}"),
        };
        // Measure the full outline rebuild on the authored fixture, independent
        // of any reuse exercised by the edit submission profiler.
        let mut samples = Vec::with_capacity(100);
        for i in 0..124 {
            let start = now();
            let output = std::hint::black_box(boardstudio_core_kernel::geometry::outlines(
                &document,
                None,
                &[],
            ));
            let elapsed = now() - start;
            assert!(!output.1.is_empty());
            if i >= 24 {
                samples.push(elapsed);
            }
        }
        samples.sort_by(f64::total_cmp);
        web_sys::console::log_1(
            &format!(
                "{size} / isolated outline rebuild: n=100, p50={:.3} ms, p95={:.3} ms",
                samples[49], samples[94],
            )
            .into(),
        );
    }
}
