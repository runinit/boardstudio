//! Explicit collision checks share the accepted Case scene and model ownership.
use crate::{
    case_preview::NativePreviewSnapshot,
    model_delivery::*,
    runtime::{CadScene, CollisionCheckInput, CollisionCheckReport, Runtime},
};
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    callout::Callout,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    item::{Item, ItemSize},
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Default)]
struct CheckState {
    source: Option<CollisionCheckInput>,
    pending: bool,
    findings: Vec<CadCollisionPairFinding>,
    clearance_threshold_mm: f64,
    message: Option<String>,
    errors: Vec<String>,
    accuracy: Option<f64>,
}

#[component]
pub(crate) fn CaseCollisionChecks(
    scene: Rc<CadScene>,
    preview: Option<Rc<NativePreviewSnapshot>>,
    model_rows: Option<ModelDeliveryRows>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let input = CollisionCheckInput::new(scene, preview, model_rows);
    let mut state = use_signal(CheckState::default);
    let mut clearance = use_signal(|| 0.0_f64);
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let latest = use_hook(|| Rc::new(RefCell::new(input.clone())));
    *latest.borrow_mut() = input.clone();
    use_effect(use_reactive((&input,), {
        let sequence = sequence.clone();
        let runtime = runtime.clone();
        move |_| {
            sequence.set(sequence.get().wrapping_add(1));
            runtime.cancel_collision_checks();
            state.set(CheckState::default());
        }
    }));
    use_drop({
        let alive = alive.clone();
        let runtime = runtime.clone();
        move || {
            alive.set(false);
            runtime.cancel_collision_checks();
        }
    });
    let ready = runtime.collision_check_ready(&input);
    let threshold = Some(clearance());
    let mut start = {
        let runtime = runtime.clone();
        let input = input.clone();
        let sequence = sequence.clone();
        let alive = alive.clone();
        let latest = latest.clone();
        move |accuracy: Option<f64>| {
            let Some(threshold) = threshold else {
                return;
            };
            if !ready {
                return;
            }
            sequence.set(sequence.get().wrapping_add(1));
            let request = sequence.get();
            state.set(CheckState {
                source: Some(input.clone()),
                pending: true,
                accuracy,
                clearance_threshold_mm: threshold,
                ..Default::default()
            });
            let runtime = runtime.clone();
            let input = input.clone();
            let sequence = sequence.clone();
            let alive = alive.clone();
            let latest = latest.clone();
            spawn(async move {
                let current: Rc<dyn Fn() -> bool> = {
                    let alive = alive.clone();
                    let sequence = sequence.clone();
                    let latest = latest.clone();
                    let input = input.clone();
                    Rc::new(move || {
                        alive.get() && sequence.get() == request && *latest.borrow() == input
                    })
                };
                let report: Option<CollisionCheckReport> = runtime
                    .check_collisions(input.clone(), threshold, accuracy, current.clone())
                    .await;
                if current() {
                    state.set(match report {
                        Some(report) => CheckState {
                            source: Some(input),
                            pending: false,
                            findings: report.findings,
                            errors: report.errors,
                            accuracy,
                            clearance_threshold_mm: threshold,
                            message: report.message,
                        },
                        None => CheckState::default(),
                    });
                }
            });
        }
    };
    let shown = state.read().clone();
    let shown = if shown.source.as_ref() == Some(&input) {
        shown
    } else {
        CheckState::default()
    };
    let mut check = start.clone();
    let clearance_change = {
        let sequence = sequence.clone();
        let runtime = runtime.clone();
        move |value: f64| {
            sequence.set(sequence.get().wrapping_add(1));
            runtime.cancel_collision_checks();
            clearance.set(value);
            state.set(CheckState::default());
        }
    };
    rsx! {
        Collapsible { class: "flex-none max-h-[30%] overflow-auto", keep_mounted: true, default_open: true,
            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Collision check", "Collision check" }
            CollapsibleContent { class: "data-[open=false]:hidden",
                NumericField {
                    label: "Required clearance (mm)", min: 0.0, step: 0.01, value: clearance(),
                    on_commit: clearance_change,
                }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !ready || threshold.is_none() || shown.pending,
                    onclick: move |_| check(None), "Check collisions" }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                    disabled: !ready || threshold.is_none() || shown.pending,
                    onclick: move |_| start(Some(TIGHT_COLLISION_ACCURACY_MM)), "Check at 0.01 mm" }
                if shown.pending {
                    p { role: "status", "Checking current assembly…" }
                    Button { variant: ButtonVariant::Secondary, button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                        onclick: move |_| {
                            sequence.set(sequence.get().wrapping_add(1)); runtime.cancel_collision_checks(); state.set(CheckState::default());
                        }, "Cancel check" }
                }
                if !ready { Callout { "Waiting for the current exact assembly and part models." } }
                if shown.accuracy.is_some() { Callout { "Requested accuracy: 0.01 mm. Unsupported accuracy remains uncertain." } }
                for error in &shown.errors { Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } } }
                if let Some(message) = &shown.message { Callout { "{message}" } }
                for (index, finding) in shown.findings.iter().enumerate() {
                    CollisionFindingView { key: "{index}", finding: finding.clone(), clearance_threshold_mm: shown.clearance_threshold_mm }
                }
            }
        }
    }
}

#[component]
fn CollisionFindingView(finding: CadCollisionPairFinding, clearance_threshold_mm: f64) -> Element {
    rsx! {
        Item { size: ItemSize::Sm, class: "items-start",
            p { "{finding.summary(clearance_threshold_mm)}" }
            Collapsible { keep_mounted: true, default_open: false,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Witness identity", "Witness identity" }
                CollapsibleContent { class: "data-[open=false]:hidden",
                    for identity in finding.provenance_details() { p { "{identity}" } }
                    for detail in finding.qualification_details() { p { "{detail}" } }
                }
            }
        }
    }
}

#[cfg(test)]
mod browser_tests {
    use super::*;
    use boardstudio_model::model::{PreparedCaseAssemblyIR, ProjectDoc};
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use gloo_timers::future::TimeoutFuture;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn mounted() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let _ = version();
        match runtime.cad_scene() {
            Some(scene) => rsx! { CaseCollisionChecks { scene, preview: None, model_rows: None } },
            None => rsx! { p { "Assembly changed" } },
        }
    }

    /// Hosts the Case control against a captured scene handed in through root context, so
    /// the mounted props and input identity survive a runtime scene invalidation instead of
    /// the host re-reading `runtime.cad_scene()` on every update.
    fn mounted_captured_scene() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let scene = use_context::<Rc<CadScene>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let _ = version();
        rsx! { CaseCollisionChecks { scene, preview: None, model_rows: None } }
    }

    async fn wait_for(root: &web_sys::Element, text: &str) {
        for _ in 0..100 {
            if root.text_content().unwrap_or_default().contains(text) {
                return;
            }
            TimeoutFuture::new(10).await;
        }
        panic!(
            "expected {text}, got {}",
            root.text_content().unwrap_or_default()
        );
    }

    fn witness_finding() -> CadCollisionPairFinding {
        use boardstudio_web_host::cad_jobs::*;
        CadCollisionPairFinding {
            left_id: "module-A/U1".into(),
            left_label: "U1".into(),
            right_id: "case-bottom".into(),
            right_label: "Bottom".into(),
            result: CadCollisionResult {
                source_interval_mm: None,
                query_qualification: None,
                observation: Some(CadCollisionObservation::SurfaceSeparated {
                    surface_distance_mm: 0.5,
                    closest_points: CadClosestPointsWitness {
                        triangles: CadTrianglePairProvenance {
                            left: CadTriangleProvenance {
                                triangle_index: 12,
                                face_occurrence_index: 2,
                            },
                            right: CadTriangleProvenance {
                                triangle_index: 20,
                                face_occurrence_index: 4,
                            },
                        },
                        point1_mm: [1., 2., 3.],
                        point2_mm: [1.5, 2., 3.],
                    },
                }),
                left_provenance: Some(CadSubjectFaceProvenance::Vendor(
                    CadResolvedFaceProvenance {
                        triangle_index: 12,
                        face_occurrence_index: 2,
                        solid_occurrence_index: 7,
                        representation_id: "80".into(),
                        solid_id: "10".into(),
                        shell_index: 1,
                        face_id: "20".into(),
                        support_id: "30".into(),
                        support_kind: "PLANE".into(),
                    },
                )),
                right_provenance: Some(CadSubjectFaceProvenance::Generated {
                    body_id: "case-bottom".into(),
                    triangle_index: 20,
                    face_occurrence_index: 4,
                    solid_occurrence_index: 3,
                    source: CadGeneratedSourceFace {
                        solid_index: 0,
                        shell_index: 1,
                        face_index: 5,
                        support_kind: "PLANE".into(),
                    },
                }),
                assessment: CadCollisionAssessment::Unknown {
                    reasons: vec!["source and numerical bounds unresolved".into()],
                    refinement_requested: false,
                },
            },
        }
    }

    fn mounted_witness() -> Element {
        rsx! { CollisionFindingView { finding: witness_finding(), clearance_threshold_mm: 0.1 } }
    }

    fn mounted_qualified_witness() -> Element {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap();
        let mut finding = witness_finding();
        finding.result.source_interval_mm = Some([0.98, 1.02]);
        finding.result.query_qualification = Some(
            serde_json::from_value(
                fixture["qualificationV2"]["activation"]["mixedVendorGeneratedQuery"].clone(),
            )
            .unwrap(),
        );
        rsx! { CollisionFindingView { finding, clearance_threshold_mm: 0.1 } }
    }

    #[wasm_bindgen_test]
    async fn mounted_source_band_and_contact_boxes_preserve_witness_uncertainty() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted =
            MountedFixture::new(root.clone(), VirtualDom::new(mounted_qualified_witness));
        wait_for(&root, "Source surface distance:").await;
        let text = root.text_content().unwrap();
        for expected in [
            "0.98",
            "1.02",
            "Contact region: 1 conservative box pairs",
            "declared source precision",
            "does not certify a unique source contact point",
            "module-A/U1",
            "generated body case-bottom",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        assert!(
            text.contains("Uncertain"),
            "a diagnostic interval cannot promote the assessment"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_collision_witness_keeps_operand_and_source_occurrence_identity() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(mounted_witness));
        wait_for(&root, "surface separation 0.500 mm").await;
        let trigger = by_role_name(&root, "button", "Witness identity")
            .expect("witness identity disclosure trigger");
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        trigger.click();
        wait_until("witness identity can be expanded", || {
            by_role_name(&root, "button", "Witness identity").is_some_and(|trigger| {
                trigger.get_attribute("aria-expanded").as_deref() == Some("true")
            })
        })
        .await;
        let text = root.text_content().unwrap();
        for identity in [
            "module-A/U1",
            "case-bottom",
            "STEP representation #80",
            "solid occurrence 7",
            "face occurrence 2",
            "source face #20",
            "generated body case-bottom",
            "solid occurrence 3",
            "face occurrence 4",
            "source face index 5",
        ] {
            assert!(
                text.contains(identity),
                "missing witness identity {identity}: {text}"
            );
        }
        assert!(text.contains("Uncertain") && text.contains("1.00, 2.00, 3.00"));
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_collision_check_is_explicit_and_retires_findings_on_source_change() {
        let runtime = support::new_runtime();
        let mut document = serde_json::to_value(ProjectDoc::empty(
            "collision-controls",
            "Collision controls",
        ))
        .unwrap();
        document["boards"] = serde_json::json!([{"id":"board","name":"Board","partIds":[],"outlineIds":[],"netIds":[],"thickness":1.6}]);
        support::open_document(&runtime, serde_json::from_value(document).unwrap()).await;
        support::navigate(&runtime, "board").await;
        let snapshot = runtime.model().accepted.unwrap();
        let scene = Rc::new(CadScene {
            scope: runtime.scope().unwrap(),
            token: snapshot.token,
            prepared: PreparedCaseAssemblyIR {
                revision: snapshot.document.revision,
                bodies: vec![],
            },
            snapshot,
            result: Default::default(),
            physical_fingerprint: None,
            mechanical: None,
            exact: true,
            contours: vec![],
        });
        runtime.set_cad_scene_test(Some(scene));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted);
        dom.provide_root_context(runtime.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        wait_for(&root, "Check collisions").await;
        let checks_trigger = by_role_name(&root, "button", "Collision check")
            .expect("collision check disclosure trigger");
        assert_eq!(
            checks_trigger.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        assert!(
            !root.text_content().unwrap().contains("Requested accuracy:"),
            "tight accuracy is not automatic"
        );
        by_role_name(&root, "button", "Check collisions")
            .expect("default check control")
            .click();
        wait_for(&root, "no pair was checked").await;
        assert!(!root.text_content().unwrap().contains("Requested accuracy:"));
        by_role_name(&root, "button", "Check at 0.01 mm")
            .expect("tight check control")
            .click();
        wait_for(&root, "Requested accuracy: 0.01 mm").await;
        runtime.set_cad_scene_test(None);
        runtime.report("Assembly changed");
        wait_for(&root, "Assembly changed").await;
        assert!(!root.text_content().unwrap().contains("no pair was checked"));
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_collision_check_clears_pending_when_runtime_retires_the_scene() {
        let runtime = support::new_runtime();
        let mut document = serde_json::to_value(ProjectDoc::empty(
            "collision-retired-scene",
            "Collision retired scene",
        ))
        .unwrap();
        document["boards"] = serde_json::json!([{"id":"board","name":"Board","partIds":[],"outlineIds":[],"netIds":[],"thickness":1.6}]);
        support::open_document(&runtime, serde_json::from_value(document).unwrap()).await;
        support::navigate(&runtime, "board").await;
        let snapshot = runtime.model().accepted.unwrap();
        let scene = Rc::new(CadScene {
            scope: runtime.scope().unwrap(),
            token: snapshot.token,
            prepared: PreparedCaseAssemblyIR {
                revision: snapshot.document.revision,
                bodies: vec![],
            },
            snapshot,
            result: Default::default(),
            physical_fingerprint: None,
            mechanical: None,
            exact: true,
            contours: vec![],
        });
        runtime.set_cad_scene_test(Some(scene.clone()));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_captured_scene);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(scene.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        wait_for(&root, "Check collisions").await;
        by_role_name(&root, "button", "Check collisions")
            .expect("default check control")
            .click();
        // Retire the runtime scene before the spawned check is polled, while the mounted
        // host keeps handing the captured scene in, so the view lifetime stays current and
        // `check_collisions` settles as a retired/unavailable `None`.
        runtime.set_cad_scene_test(None);
        // One event-loop turn drains the spawned check and Dioxus render microtasks.
        TimeoutFuture::new(0).await;
        let text = root.text_content().unwrap();
        assert!(
            !text.contains("Checking current assembly…"),
            "a retired check must not leave the control stuck pending: {text}"
        );
        assert!(
            !text.contains("no pair was checked"),
            "a retired check must not publish an apparently successful empty report: {text}"
        );
        assert!(
            by_role_name(&root, "button", "Witness identity").is_none(),
            "a retired check must not publish stale findings: {text}"
        );
        root.remove();
    }
}
