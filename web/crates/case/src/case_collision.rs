//! Explicit collision checks share the accepted Case scene and model ownership.
use crate::{
    case_preview::NativePreviewSnapshot,
    model_delivery::*,
    runtime::{CadScene, Runtime},
};
use boardstudio_web_ui_shared::controls::{BoardButton, BoardDisclosure};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, PartialEq)]
struct CollisionSource {
    scene: Rc<CadScene>,
    preview: Option<Rc<NativePreviewSnapshot>>,
    rows: Option<ModelDeliveryRows>,
}

impl CollisionSource {
    fn is_current(&self, runtime: &Runtime) -> bool {
        runtime.scope().as_ref() == Some(&self.scene.scope)
            && runtime.model().accepted.is_some_and(|accepted| {
                accepted.token == self.scene.token
                    && accepted.document.revision == self.scene.snapshot.document.revision
            })
            && runtime
                .cad_scene()
                .is_some_and(|scene| Rc::ptr_eq(&scene, &self.scene))
            && runtime.case_gesture_preview_scene(&self.scene).is_none()
            && self.preview.as_ref().is_none_or(|expected| {
                runtime
                    .native_case_preview()
                    .is_some_and(|current| current == *expected)
                    && runtime.native_model_delivery(expected) == self.rows
            })
    }
}

#[derive(Clone, Default)]
struct CheckState {
    source: Option<CollisionSource>,
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
    let source = CollisionSource {
        scene,
        preview,
        rows: model_rows,
    };
    let mut state = use_signal(CheckState::default);
    let mut clearance = use_signal(|| "0".to_owned());
    let sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    let latest = use_hook(|| Rc::new(RefCell::new(source.clone())));
    *latest.borrow_mut() = source.clone();
    use_effect(use_reactive((&source,), {
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
    let ready = source.scene.exact
        && source.is_current(&runtime)
        && (source
            .preview
            .as_ref()
            .is_none_or(|preview| preview.preview.models.is_empty())
            || source
                .rows
                .as_ref()
                .is_some_and(|rows| rows.pending.is_empty()));
    let threshold = clearance()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0);
    let mut start = {
        let runtime = runtime.clone();
        let source = source.clone();
        let sequence = sequence.clone();
        let alive = alive.clone();
        move |accuracy: Option<f64>| {
            let Some(threshold) = threshold else {
                return;
            };
            if !ready {
                return;
            }
            sequence.set(sequence.get().wrapping_add(1));
            let request = sequence.get();
            runtime.cancel_collision_checks();
            state.set(CheckState {
                source: Some(source.clone()),
                pending: true,
                accuracy,
                clearance_threshold_mm: threshold,
                ..Default::default()
            });
            let runtime = runtime.clone();
            let source = source.clone();
            let sequence = sequence.clone();
            let alive = alive.clone();
            let latest = latest.clone();
            spawn(async move {
                let current: Rc<dyn Fn() -> bool> = {
                    let runtime = runtime.clone();
                    let source = source.clone();
                    Rc::new(move || {
                        alive.get()
                            && sequence.get() == request
                            && *latest.borrow() == source
                            && source.is_current(&runtime)
                    })
                };
                let result =
                    run_checks(&runtime, &source, threshold, accuracy, current.clone()).await;
                if current() {
                    state.set(CheckState {
                        source: Some(source),
                        pending: false,
                        findings: result.0,
                        errors: result.1,
                        accuracy,
                        clearance_threshold_mm: threshold,
                        message: result.2,
                    });
                }
            });
        }
    };
    let shown = state.read().clone();
    let shown = if shown.source.as_ref() == Some(&source) {
        shown
    } else {
        CheckState::default()
    };
    let mut check = start.clone();
    let clearance_change = {
        let sequence = sequence.clone();
        let runtime = runtime.clone();
        move |event: FormEvent| {
            sequence.set(sequence.get().wrapping_add(1));
            runtime.cancel_collision_checks();
            clearance.set(event.value());
            state.set(CheckState::default());
        }
    };
    rsx! {
        BoardDisclosure { class: "m1-case-collision-checks", open: true,
            heading: rsx! { summary { "Collision check" } },
            label { "Required clearance (mm)"
                input { r#type: "number", min: "0", step: "0.01", value: clearance(),
                    oninput: clearance_change
                }
            }
            BoardButton { class: "m1-case-collision-default", disabled: !ready || threshold.is_none() || shown.pending,
                onclick: move |_| check(None), "Check collisions" }
            BoardButton { class: "m1-case-collision-tight", disabled: !ready || threshold.is_none() || shown.pending,
                onclick: move |_| start(Some(TIGHT_COLLISION_ACCURACY_MM)), "Check at 0.01 mm" }
            if shown.pending {
                p { role: "status", "Checking current assembly…" }
                BoardButton { onclick: move |_| {
                    sequence.set(sequence.get().wrapping_add(1)); runtime.cancel_collision_checks(); state.set(CheckState::default());
                }, "Cancel check" }
            }
            if !ready { p { "Waiting for the current exact assembly and part models." } }
            if shown.accuracy.is_some() { p { "Requested accuracy: 0.01 mm. Unsupported accuracy remains uncertain." } }
            for error in &shown.errors { p { role: "alert", "{error}" } }
            if let Some(message) = &shown.message { p { "{message}" } }
            for (index, finding) in shown.findings.iter().enumerate() {
                CollisionFindingView { key: "{index}", finding: finding.clone(), clearance_threshold_mm: shown.clearance_threshold_mm }
            }
        }
    }
}

#[component]
fn CollisionFindingView(finding: CadCollisionPairFinding, clearance_threshold_mm: f64) -> Element {
    rsx! {
        p { "{finding.summary(clearance_threshold_mm)}" }
        BoardDisclosure { class: "m1-case-collision-provenance",
            heading: rsx! { summary { "Witness identity" } },
            for identity in finding.provenance_details() { p { "{identity}" } }
            for detail in finding.qualification_details() { p { "{detail}" } }
        }
    }
}

async fn run_checks(
    runtime: &Rc<Runtime>,
    source: &CollisionSource,
    threshold: f64,
    accuracy: Option<f64>,
    current: Rc<dyn Fn() -> bool>,
) -> (Vec<CadCollisionPairFinding>, Vec<String>, Option<String>) {
    let mut subjects = Vec::new();
    let mut errors = Vec::new();
    if let (Some(preview), Some(rows)) = (&source.preview, &source.rows) {
        for model in &rows.delivered {
            let placement = preview.preview.models.iter().find(|row| row.id == model.id);
            let transform = model.matrix.map(Ok).unwrap_or_else(|| {
                placement
                    .ok_or_else(|| "model placement is missing".to_owned())
                    .and_then(|row| {
                        physical_pcb_model_row_transform_f64(
                            row,
                            preview.preview.thickness,
                            preview.board_reference.as_ref(),
                            source
                                .scene
                                .mechanical
                                .as_ref()
                                .is_some_and(|assembly| !assembly.stack.is_empty()),
                        )
                    })
            });
            match transform {
                Ok(row_transform_f64) => {
                    let mut subject = CadCollisionSubject::VendorModel {
                        id: model.id.clone(),
                        reference: placement
                            .map(|row| row.reference.clone())
                            .unwrap_or_else(|| model.id.clone()),
                        row_transform_f64,
                        vendor_step: model.mesh.vendor_step.clone(),
                    };
                    if let Some(target) = accuracy {
                        match runtime
                            .refine_collision_vendor(
                                &source.scene,
                                &subject,
                                target,
                                current.clone(),
                            )
                            .await
                        {
                            Ok(refined) => subject = refined,
                            Err(error) => {
                                errors.push(error);
                                if let CadCollisionSubject::VendorModel { vendor_step, .. } =
                                    &mut subject
                                {
                                    *vendor_step = None;
                                }
                            }
                        }
                    }
                    subjects.push(subject);
                }
                Err(error) => errors.push(format!("{}: {error}", model.id)),
            }
            if !current() {
                return (Vec::new(), Vec::new(), None);
            }
        }
        errors.extend(
            rows.failures
                .iter()
                .map(|failure| format!("{}: {}", failure.reference, failure.reason)),
        );
    }
    let (generated, generated_errors) = runtime
        .generated_collision_subjects(&source.scene, accuracy, current.clone())
        .await;
    subjects.extend(generated);
    errors.extend(generated_errors);
    if !current() {
        return (Vec::new(), Vec::new(), None);
    }
    let mut findings = Vec::new();
    for (index, left) in subjects.iter().enumerate() {
        for right in &subjects[index + 1..] {
            match runtime
                .query_collision_subjects(
                    left,
                    right,
                    threshold,
                    accuracy,
                    source.scene.scope.clone(),
                    source.scene.token,
                    source.scene.snapshot.document.revision,
                    current.clone(),
                )
                .await
            {
                Ok(result) => findings.push(CadCollisionPairFinding {
                    left_id: left.id().into(),
                    left_label: left.label().into(),
                    right_id: right.id().into(),
                    right_label: right.label().into(),
                    result,
                }),
                Err(error) => errors.push(format!("{} ↔ {}: {error}", left.label(), right.label())),
            }
            if !current() {
                return (Vec::new(), Vec::new(), None);
            }
        }
    }
    let message = (subjects.len() < 2).then(|| {
        "Fewer than two collision operands are available; no pair was checked.".to_owned()
    });
    (findings, errors, message)
}

#[cfg(test)]
mod browser_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::{PreparedCaseAssemblyIR, ProjectDoc};
    use gloo_timers::future::TimeoutFuture;
    use wasm_bindgen::JsCast;
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
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(mounted_qualified_witness),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
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
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(mounted_witness),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_for(&root, "surface separation 0.500 mm").await;
        let details = root
            .query_selector(".m1-case-collision-provenance")
            .unwrap()
            .unwrap();
        assert!(!details.has_attribute("open"));
        details
            .query_selector("summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        assert!(
            details.has_attribute("open"),
            "witness identity can be expanded"
        );
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_for(&root, "Check collisions").await;
        assert!(
            !root.text_content().unwrap().contains("Requested accuracy:"),
            "tight accuracy is not automatic"
        );
        root.query_selector(".m1-case-collision-default")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        wait_for(&root, "no pair was checked").await;
        assert!(!root.text_content().unwrap().contains("Requested accuracy:"));
        root.query_selector(".m1-case-collision-tight")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        wait_for(&root, "Requested accuracy: 0.01 mm").await;
        runtime.set_cad_scene_test(None);
        runtime.report("Assembly changed");
        wait_for(&root, "Assembly changed").await;
        assert!(!root.text_content().unwrap().contains("no pair was checked"));
        root.remove();
    }
}
