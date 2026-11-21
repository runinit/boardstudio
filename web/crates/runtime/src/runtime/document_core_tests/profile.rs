use super::*;
use boardstudio_core::{apply_for_test, validate_document_for_test};
use boardstudio_core_kernel::{geometry, matrix::layout};
use boardstudio_model::model::{
    EditCommand, EditPhase, MatrixPlacementChange, OutlineFeature, RowLayout,
};
use boardstudio_model::wire::encode_core_reply;
use sha2::{Digest, Sha256};
use std::hint::black_box;

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

pub(super) fn fixture(large: bool) -> ProjectDoc {
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

fn legacy_rename(runtime: &Rc<Runtime>, name: &'static str) -> EditOutcome {
    runtime.resolve_edit(
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
    submit: impl FnOnce() -> EditOutcome,
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
async fn page_core_profiles_distinct_stagger_commits() {
    for (size, large) in [("2 keys", false), ("60 keys", true)] {
        let document = fixture(large);
        for matching_preview in [false, true] {
            let label = if matching_preview {
                "distinct stagger after matching Preview"
            } else {
                "distinct direct stagger"
            };
            let runtime = profile_runtime(document.clone(), label).await;
            let mut samples = Vec::with_capacity(100);
            for i in 0..124 {
                let value = 1.0 + f64::from(i) * 0.025;
                let expected_preview = if matching_preview {
                    let preview = submit(&runtime, |operation_id| Event::PreviewEdit {
                        operation_id,
                        transaction_id: "stagger-preview".into(),
                        target_ids: vec!["matrix".into()],
                        operation: EditOperation::SetMatrixPlacement {
                            matrix_id: "matrix".into(),
                            change: MatrixPlacementChange::ColumnStagger { column: 0, value },
                        },
                    });
                    assert_eq!(wait_for_outcome(&preview).await, TerminalOutcome::Completed);
                    runtime.model().display_preview
                } else {
                    None
                };
                let (accepted, sample) =
                    profile_ticket(&runtime, || stagger(&runtime, "matrix", value)).await;
                assert_eq!(
                    accepted.document.matrices[0].column_layout[0]
                        .stagger
                        .to_bits(),
                    value.to_bits()
                );
                if matching_preview {
                    let mut expected = (*expected_preview.expect("published Preview")).clone();
                    expected.revision = accepted.scene.revision;
                    expected
                        .transaction_id
                        .clone_from(&accepted.scene.transaction_id);
                    assert_eq!(accepted.scene.as_ref(), &expected);
                }
                if i >= 24 {
                    samples.push(sample);
                }
            }
            report(size, label, &samples);
        }
    }
}

fn profile_samples(size: &str, label: &str, batch: u32, mut sample: impl FnMut(usize) -> f64) {
    const WARMUPS: usize = 24;
    const SAMPLES: usize = 100;

    let mut samples: Vec<_> = (0..WARMUPS + SAMPLES)
        .map(|i| sample(i) / f64::from(batch))
        .skip(WARMUPS)
        .collect();
    samples.sort_by(f64::total_cmp);
    web_sys::console::log_1(
        &format!(
            "{size} / {label}: n={SAMPLES}, warmups={WARMUPS}, batch={batch}, p50={:.3} ms, p95={:.3} ms",
            samples[49], samples[94],
        )
        .into(),
    );
}

fn profile_isolated<T>(
    size: &str,
    label: &str,
    batch: u32,
    mut operation: impl FnMut() -> T,
    mut check: impl FnMut(&T),
) {
    // Batch cheap operations to resolve sub-tick costs. Reserve storage once and
    // check and drop outputs after stopping the clock.
    let mut outputs = Vec::with_capacity(batch as usize);
    profile_samples(size, label, batch, |_| {
        let start = now();
        for _ in 0..batch {
            outputs.push(black_box(operation()));
        }
        let elapsed = now() - start;
        check(outputs.last().unwrap());
        outputs.clear();
        elapsed
    });
}

fn report_wire_hash(size: &str, label: &str, reply: &CoreReply) {
    let digest: String = Sha256::digest(encode_core_reply(reply))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    web_sys::console::log_1(&format!("{size} / {label} wire SHA-256 = {digest}").into());
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
            profile_samples(size, &format!("typed stagger Preview / {label}"), 1, |i| {
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
                let reply = black_box(engine.handle(request));
                let elapsed = now() - start;
                assert!(matches!(&reply, CoreReply::Preview { scene, .. }
                    if scene.revision == 0 && !scene.changed_ids.is_empty()));
                if i == 24 {
                    report_wire_hash(size, label, &reply);
                }
                elapsed
            });
        }
        assert!(matches!(
            engine.handle(CoreRequest::Snapshot { id: "after-preview".into() }),
            CoreReply::Scene { scene, .. } if scene.revision == 0 && scene.changed_ids.is_empty()
        ));
    }
}

#[wasm_bindgen_test]
fn page_core_profiles_outline_rebuild_in_isolation() {
    for (size, large) in [("2 keys", false), ("60 keys", true)] {
        let mut engine = boardstudio_core::CoreEngine::new();
        let (document, scene) = match engine.handle(CoreRequest::Open {
            id: "outline-profile".into(),
            document: fixture(large),
        }) {
            CoreReply::Scene {
                document, scene, ..
            } => (*document, scene),
            reply => panic!("could not open outline profile fixture: {reply:?}"),
        };
        profile_isolated(
            size,
            "isolated outline rebuild",
            1,
            || geometry::outlines(black_box(&document), None, &[]),
            |output| assert!(!output.1.is_empty()),
        );
        let (cache, _, _) = geometry::outlines(&document, None, &[]);
        let findings = validate_document_for_test(&document);
        profile_isolated(
            size,
            "scene diagnostics including matrix projections",
            32,
            || validate_document_for_test(black_box(&document)),
            |output| assert_eq!(output, &findings),
        );
        profile_isolated(
            size,
            "board_scene_geometry with precomputed cache",
            8,
            || geometry::board_scene_geometry(black_box(&document), black_box(&cache)),
            |output| assert_eq!(output.contours, scene.board_contours),
        );
        profile_isolated(
            size,
            "ProjectDoc clone",
            64,
            || black_box(&document).clone(),
            |output| assert_eq!(output, &document),
        );
        profile_isolated(
            size,
            "SceneDelta clone",
            64,
            || black_box(&scene).clone(),
            |output| assert_eq!(output, &scene),
        );
        let mut snapshot_scene = scene;
        "snapshot".clone_into(&mut snapshot_scene.transaction_id);
        profile_samples(size, "typed Snapshot", 1, |_| {
            let request = CoreRequest::Snapshot {
                id: "snapshot-profile".into(),
            };
            let start = now();
            let reply = black_box(engine.handle(request));
            let elapsed = now() - start;
            let CoreReply::Scene {
                document: output,
                scene,
                ..
            } = reply
            else {
                panic!("could not take profile snapshot: {reply:?}");
            };
            assert_eq!(*output, document);
            assert_eq!(scene, snapshot_scene);
            elapsed
        });
        let operation = EditOperation::SetMatrixPlacement {
            matrix_id: "matrix".into(),
            change: MatrixPlacementChange::ColumnStagger {
                column: document.matrices[0].columns - 1,
                value: 4.0,
            },
        };
        let reply = engine.handle(CoreRequest::Edit {
            id: "stagger-for-outline-profile".into(),
            command: EditCommand {
                base_revision: document.revision,
                transaction_id: "stagger-for-outline-profile".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["matrix".into()],
                operation: operation.clone(),
            },
        });
        report_wire_hash(size, "last column Commit", &reply);
        let CoreReply::Scene {
            document: staggered,
            ..
        } = reply
        else {
            panic!("could not settle the outline profile stagger");
        };
        let moved: Vec<_> = document
            .parts
            .iter()
            .zip(&staggered.parts)
            .filter(|(before, after)| before != after)
            .map(|(_, part)| part.id.clone())
            .collect();
        assert_eq!(moved.len(), document.matrices[0].rows as usize);
        assert_eq!(document.layouts, staggered.layouts);
        assert_eq!(document.boards, staggered.boards);
        assert_eq!(document.outline, staggered.outline);
        profile_samples(size, "last-column edit apply + layout resolve", 1, |_| {
            let mut draft = document.clone();
            let start = now();
            let changed = apply_for_test(&mut draft, &[], &operation);
            let resolved = layout::resolve(&mut draft);
            let elapsed = now() - start;
            assert!(!changed.unwrap().is_empty());
            resolved.unwrap();
            assert_eq!(draft.parts, staggered.parts);
            assert_eq!(draft.matrices, staggered.matrices);
            elapsed
        });
        let (_, expected_contours, expected_findings) = geometry::outlines(&staggered, None, &[]);
        profile_isolated(
            size,
            "settled last-column outline / envelope cache miss",
            1,
            || geometry::outlines(black_box(&staggered), Some(&cache), &moved),
            |(_, contours, findings)| {
                assert_eq!(contours, &expected_contours);
                assert_eq!(findings, &expected_findings);
            },
        );
        let (_, expected_contours, expected_findings) = geometry::outlines(&document, None, &[]);
        profile_isolated(
            size,
            "accepted outline / envelope cache hit",
            8,
            || geometry::outlines(black_box(&document), Some(&cache), &[]),
            |(_, contours, findings)| {
                assert_eq!(contours, &expected_contours);
                assert_eq!(findings, &expected_findings);
            },
        );
    }
}
