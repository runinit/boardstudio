use super::*;
use boardstudio_model::model::{
    EditCommand, EditPhase, MatrixPlacementChange, OutlineFeature, RowLayout,
};

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

async fn profile_runtime(mut document: ProjectDoc, suffix: &str) -> Rc<Runtime> {
    document.id.push_str(suffix);
    let mut engine = boardstudio_core::CoreEngine::new();
    let runtime = runtime_with_core(Box::new(move |request| {
        let _timer = Timer::start(Phase::Core);
        engine.handle(request)
    }));
    let opened = submit(&runtime, |operation_id| Event::Open {
        operation_id,
        document,
    });
    assert_eq!(wait_for_outcome(&opened).await, TerminalOutcome::Completed);
    runtime
}

fn legacy_rename(runtime: &Rc<Runtime>, name: &'static str) -> EditTicket {
    EditTicket::begin(
        runtime,
        "project name",
        None,
        EditResolver::new("rename", move |accepted| {
            let mut document = (*accepted.document).clone();
            document.name = name.into();
            Resolution::submit(
                vec![document.id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        }),
    )
}

async fn profile_ticket(
    runtime: &Rc<Runtime>,
    submit: impl FnOnce() -> EditTicket,
) -> (AcceptedSnapshot, [f64; 5]) {
    let initial = runtime.model().accepted.unwrap();
    let revision = initial.document.revision + 1;
    SAMPLE.set(Some([None; 4]));
    let start = now();
    let ticket = submit();
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
    let [core, save, publish, settlement] = SAMPLE.take().unwrap().map(Option::unwrap);
    (accepted, [elapsed, core, save, publish, settlement])
}

fn report(size: &str, label: &str, samples: &[[f64; 5]]) {
    for (index, phase) in [
        "ticket submit",
        "Core handle",
        "prepare save + notify",
        "publish snapshot + notify",
        "deliver settlement",
    ]
    .into_iter()
    .enumerate()
    {
        let mut values: Vec<_> = samples.iter().map(|sample| sample[index]).collect();
        values.sort_by(f64::total_cmp);
        web_sys::console::log_1(
            &format!(
                "{size} / {label} / {phase}: n=100, p50={:.3} ms, p95={:.3} ms",
                values[49], values[94],
            )
            .into(),
        );
    }
}

#[wasm_bindgen_test]
async fn page_core_profiles_rename_and_stagger_submission() {
    for (size, large) in [("2 keys", false), ("60 keys", true)] {
        let document = fixture(large);
        let legacy = profile_runtime(document.clone(), "-legacy-rename").await;
        let typed = profile_runtime(document.clone(), "-typed-rename").await;
        let mut rename_samples = [Vec::with_capacity(100), Vec::with_capacity(100)];
        for i in 0..124 {
            let name = ["First name", "Second name"][i % 2];
            // Give both operations the same sequence of names and alternate which
            // runs first, within the same release build and browser process.
            for lane in [i % 2, 1 - i % 2] {
                let (accepted, sample) = if lane == 0 {
                    profile_ticket(&legacy, || legacy_rename(&legacy, name)).await
                } else {
                    profile_ticket(&typed, || rename(&typed, name)).await
                };
                assert_eq!(accepted.document.name, name);
                if i >= 24 {
                    rename_samples[lane].push(sample);
                }
            }
        }
        let legacy_snapshot = legacy.model().accepted.unwrap();
        let typed_snapshot = typed.model().accepted.unwrap();
        assert_eq!(legacy_snapshot.scene, typed_snapshot.scene);
        let mut expected = (*legacy_snapshot.document).clone();
        expected.id.clone_from(&typed_snapshot.document.id);
        assert_eq!(&expected, typed_snapshot.document.as_ref());
        report(size, "legacy rename", &rename_samples[0]);
        report(size, "typed rename", &rename_samples[1]);

        let runtime = profile_runtime(document, "-column-stagger").await;
        let mut samples = Vec::with_capacity(100);
        for i in 0..124 {
            let value = [2.0, 4.0][i % 2];
            let (accepted, sample) =
                profile_ticket(&runtime, || stagger(&runtime, "matrix", value)).await;
            assert_eq!(
                accepted.document.matrices[0].column_layout[0].stagger,
                value
            );
            if i >= 24 {
                samples.push(sample);
            }
        }
        report(size, "column stagger", &samples);
    }
}

#[wasm_bindgen_test]
fn page_core_profiles_stagger_preview() {
    for (size, large) in [("2 keys", false), ("60 keys", true)] {
        let document = fixture(large);
        let columns = document.matrices[0].columns;
        let mut engine = boardstudio_core::CoreEngine::new();
        assert!(matches!(
            engine.handle(CoreRequest::Open {
                id: "preview-open".into(),
                document,
            }),
            CoreReply::Scene { .. }
        ));
        for (label, column) in [("first column", 0), ("last column", columns - 1)] {
            let mut samples = Vec::with_capacity(100);
            for i in 0..124 {
                let request = CoreRequest::Edit {
                    id: "stagger-preview".into(),
                    command: EditCommand {
                        base_revision: 0,
                        transaction_id: "stagger-preview".into(),
                        phase: EditPhase::Preview,
                        target_ids: vec!["matrix".into()],
                        operation: EditOperation::SetMatrixPlacement {
                            matrix_id: "matrix".into(),
                            change: MatrixPlacementChange::ColumnStagger {
                                column,
                                value: [2.0, 4.0][i % 2],
                            },
                        },
                    },
                };
                let start = now();
                let reply = std::hint::black_box(engine.handle(request));
                let elapsed = now() - start;
                assert!(matches!(reply, CoreReply::Preview { scene, .. }
                    if scene.revision == 0 && !scene.changed_ids.is_empty()));
                if i >= 24 {
                    samples.push(elapsed);
                }
            }
            samples.sort_by(f64::total_cmp);
            web_sys::console::log_1(
                &format!(
                    "{size} / stagger preview / {label}: n=100, p50={:.3} ms, p95={:.3} ms",
                    samples[49], samples[94],
                )
                .into(),
            );
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
