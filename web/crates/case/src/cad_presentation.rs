//! Canvas lifetime and case controls consume immutable session snapshots.
use crate::runtime::Runtime;
use boardstudio_application::{Event, GenerationStatus};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

use crate::case_generation_lifecycle::{
    CaseGenerationOwner, CaseGeometryStatus, case_generation_title,
};

#[component]
pub fn CasePanel(
    generation_ready: bool,
    mechanical_settings: Option<crate::presentation::MechanicalSettingsProps>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<crate::presentation::InstanceSelection>();
    let runtime_version = use_context::<Signal<u64>>();
    let _ = runtime_version();
    crate::case_preview_lifecycle::use_native_case_preview(
        runtime.clone(),
        runtime.native_case_preview_key(),
    );
    let accepted_preview = runtime.native_case_preview();
    use_effect(use_reactive((&runtime_version(),), {
        let runtime = runtime.clone();
        move |_| {
            // Runtime snapshots are not Dioxus signals. Re-read them when its
            // subscribed version changes so a preview published after mount
            // starts delivery, and so the completed batch becomes visible.
            if let Some(preview) = runtime.native_case_preview() {
                let runtime = runtime.clone();
                spawn_local(async move {
                    if let Err(error) = runtime.deliver_native_case_models(preview).await {
                        runtime.report(format!("Case model delivery failed: {error}"));
                    }
                });
            }
        }
    }));
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let case_generation = use_context::<crate::presentation::CaseGenerationState>();
    let mut live_preview = case_generation.live_preview;
    let mut automatic_generation = case_generation.automatic;
    let current_scope = runtime.scope();
    let generation_owner = current_scope.clone().map(|scope| CaseGenerationOwner {
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
    });
    let current_scene = runtime.cad_scene();
    let has_reusable_result = generation_owner.as_ref().is_some_and(|owner| {
        current_scene.as_ref().is_some_and(|scene| {
            scene.exact
                && scene.scope == owner.scope
                && scene.token == owner.token
                && scene.snapshot.document.revision == owner.revision
                && scene.prepared.revision == owner.revision
        })
    });
    let generation_busy = matches!(
        model.generation,
        GenerationStatus::Preparing { .. } | GenerationStatus::Running { .. }
    );
    let mechanical_export_configured = current_scope.as_ref().is_some_and(|scope| {
        boardstudio_web_host::cad_jobs::captured_case_document(snapshot, scope)
            .ok()
            .and_then(|document| {
                document
                    .mechanical
                    .as_ref()
                    .map(|configuration| configuration.board_id == scope.board_id)
            })
            .unwrap_or(false)
    });
    let export_outline_ready = snapshot
        .scene
        .board_readiness
        .iter()
        .find(|item| item.board_id == model.active_board_id)
        .map_or(
            snapshot.document.boards.len() <= 1 && snapshot.scene.readiness.outline,
            |item| item.outline,
        );
    let exact_generation = matches!(
        model.generation,
        GenerationStatus::Ready { exact: true, .. }
    );
    let mechanical_export_ready = mechanical_export_configured
        && generation_ready
        && !generation_busy
        && export_outline_ready
        && exact_generation
        && has_reusable_result
        && current_scene.as_ref().is_some_and(|scene| {
            scene.mechanical.as_ref().is_some_and(|assembly| {
                assembly.revision == snapshot.document.revision
                    && !assembly.generation_blocked
                    && !assembly
                        .diagnostics
                        .iter()
                        .any(|finding| finding.severity == boardstudio_core::model::Severity::Error)
            })
        });
    use_effect(use_reactive(
        (
            &runtime_version(),
            &generation_ready,
            &generation_owner,
            &live_preview(),
        ),
        {
            let runtime = runtime.clone();
            move |(version, generation_ready, owner, live)| {
                let _ = version;
                let owner = owner.clone();
                let Some(owner) = owner else { return };
                let accepted_is_current = runtime.scope().as_ref() == Some(&owner.scope)
                    && runtime.model().accepted.as_ref().is_some_and(|accepted| {
                        accepted.token == owner.token
                            && accepted.document.revision == owner.revision
                    });
                let mut generation = automatic_generation;
                if generation.write().observe(
                    Some(owner.clone()),
                    generation_ready && live && accepted_is_current,
                    has_reusable_result,
                    generation_busy,
                ) {
                    runtime.submit(Event::StartGeneration {
                        operation_id: runtime.operation(),
                        scope: owner.scope,
                    });
                }
            }
        },
    ));
    let generate = runtime.clone();
    let cancel = runtime.clone();
    let export_mechanical = runtime.clone();
    let export_owner = generation_owner.clone();
    let export_instance_selection = instance_selection;
    let export_mechanical_click = move |_| {
        let model = export_mechanical.model();
        if let Some(owner) = export_owner.as_ref()
            && export_mechanical.scope().as_ref() == Some(&owner.scope)
            && export_instance_selection.is_current(&model)
            && model.accepted.as_ref().is_some_and(|accepted| {
                accepted.token == owner.token
                    && accepted.document.revision == owner.revision
                    && accepted.session_epoch == owner.scope.session_epoch
            })
            && export_mechanical.cad_scene().is_some_and(|scene| {
                scene.exact
                    && scene.scope == owner.scope
                    && scene.token == owner.token
                    && scene.snapshot.document.revision == owner.revision
                    && scene.prepared.revision == owner.revision
            })
        {
            export_mechanical.export_mechanical();
        }
    };
    let scene = current_scene;
    let mut live_preview_control = live_preview;
    let mut automatic_generation_control = automatic_generation;
    let toggle_runtime = runtime.clone();
    let generation_enabled = automatic_generation.read().enabled();
    let stale = scene
        .as_ref()
        .is_some_and(|scene| scene.token != snapshot.token);
    let geometry = if has_reusable_result {
        CaseGeometryStatus::CurrentExact
    } else if stale {
        CaseGeometryStatus::Previous
    } else if scene.is_some() {
        CaseGeometryStatus::Preview
    } else {
        CaseGeometryStatus::Missing
    };
    let title = case_generation_title(&model.generation, geometry);
    rsx! {
        section { class: "m1-case-panel", "aria-label": "Case assembly",
            div { class: "m1-case-header",
                h2 { "Case assembly" }
                label { class: "m1-case-live-toggle",
                    input {
                        r#type: "checkbox",
                        role: "switch",
                        checked: generation_enabled,
                        onchange: move |event: FormEvent| {
                            let enabled = event.checked();
                            live_preview_control.set(enabled);
                            automatic_generation_control.write().set_enabled(enabled);
                            if !enabled && generation_busy {
                                toggle_runtime.submit(Event::CancelGeneration {
                                    operation_id: toggle_runtime.operation(),
                                });
                            }
                        }
                    }
                    span { "Live preview" }
                }
                button { disabled: !generation_ready || generation_busy, onclick: move |_| if instance_selection.is_current(&generate.model()) && let Some(scope) = generate.scope() { generate.submit(Event::StartGeneration { operation_id: generate.operation(), scope }); }, "Update preview" }
                button {
                    disabled: !generation_busy,
                    onclick: move |_| {
                        live_preview.set(false);
                        automatic_generation.write().set_enabled(false);
                        cancel.submit(Event::CancelGeneration { operation_id: cancel.operation() });
                    },
                    "Cancel"
                }
                if mechanical_export_configured {
                    button {
                        disabled: !mechanical_export_ready,
                        onclick: export_mechanical_click,
                        "Export geometry"
                    }
                }
            }
            p { class: "m1-case-live-status", role: "status", "aria-live": "polite", "{title}" }
            if runtime.native_case_preview_pending() {
                p { class: "m1-case-live-status is-secondary", role: "status", "aria-live": "polite", "Preparing the accepted PCB preview…" }
            } else if runtime.native_case_preview_error().is_some() {
                p { class: "m1-case-live-status is-error", role: "alert", "The accepted PCB preview could not be prepared." }
            }
            if let Some(preview) = accepted_preview.as_ref()
                && let Some(rows) = runtime.native_model_delivery(preview)
                && (rows.delivered.len() != preview.preview.models.len() || !rows.failures.is_empty())
            {
                details { class: "m1-case-model-delivery",
                    summary { "Board models: {rows.delivered.len()} of {preview.preview.models.len()} decoded" }
                    for failure in rows.failures.iter() {
                        p { role: "alert", "{failure.reference}: {failure.reason}" }
                    }
                }
            }
            if scene.is_none()
                && let Some(preview) = accepted_preview.clone()
            {
                crate::presentation::CasePreviewViewer {
                    key: "{preview.owner.scope.session_epoch.0}:{preview.owner.scope.board_id}:{preview.owner.scope.instance_id:?}:preview",
                    model_rows: runtime.native_model_delivery(&preview),
                    preview,
                }
            }
            if let Some(scene) = scene {
                crate::presentation::CaseViewer {
                    key: "{scene.scope.session_epoch.0}:{scene.scope.board_id}:{scene.scope.instance_id:?}",
                    scene,
                    model_rows: accepted_preview.as_ref().and_then(|preview| runtime.native_model_delivery(preview)),
                    preview: accepted_preview,
                    mechanical_settings,
                }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::runtime::{firmware_export_test_support, project_name_test_support as support};
    use boardstudio_application::{
        Completion, Effect, Event as AppEvent, GenerationStatus, JobId, Scope,
    };
    use boardstudio_core::model::{
        Board, EditCommand, EditOperation, EditPhase, MechanicalAssembly, Operation,
        OutlineFeature, ProjectDoc, Vec2,
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::HtmlInputElement;

    wasm_bindgen_test_configure!(run_in_browser);

    fn host() -> Element {
        let mut runtime_version = use_signal(|| 0u64);
        let mut mounted = use_signal(|| true);
        use_context_provider(|| runtime_version);
        crate::presentation::use_empty_test_instance_selection();
        crate::presentation::use_test_case_generation_state();
        rsx! {
            button { id: "case-live-refresh", onclick: move |_| runtime_version += 1, "Refresh" }
            button { id: "case-live-unmount", onclick: move |_| mounted.set(false), "Leave Case" }
            button { id: "case-live-remount", onclick: move |_| mounted.set(true), "Enter Case" }
            if mounted() { CasePanel { generation_ready: true } }
        }
    }

    fn export_host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let mut runtime_version = use_signal(|| 0u64);
        use_context_provider(|| runtime_version);
        crate::presentation::use_empty_test_instance_selection();
        crate::test_contexts::use_case_viewer_test_contexts();
        crate::presentation::use_test_case_generation_state();
        let generation_ready =
            crate::test_contexts::use_case_generation_readiness_test_bridge(runtime);
        rsx! {
            button { id: "case-export-refresh", onclick: move |_| runtime_version += 1, "Refresh" }
            CasePanel { generation_ready }
        }
    }

    /// A one-board project with a rectangular outline, so Core reports the board outline-ready
    /// and the mounted Case panel can admit generation and export.
    fn outlined_board_document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("case-live-test", "Case live preview test");
        document.boards.push(Board {
            id: "board-left".into(),
            name: "Left".into(),
            outline_ids: vec!["outline-left".into()],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.outline.push(OutlineFeature::Polygon {
            anchor_part_id: None,
            id: "outline-left".into(),
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 60.0, y: 0.0 },
                Vec2 { x: 60.0, y: 40.0 },
                Vec2 { x: 0.0, y: 40.0 },
            ],
            operation: Operation::Add,
        });
        document
    }

    /// Start generation through the real Session and return the job it started. Generation
    /// workers are outside the in-process adapter, so the job's effect is taken, not run.
    fn start_generation(runtime: &Rc<Runtime>, scope: &Scope) -> JobId {
        runtime.submit(AppEvent::StartGeneration {
            operation_id: runtime.operation(),
            scope: scope.clone(),
        });
        support::take_held_effects(runtime)
            .into_iter()
            .find_map(|effect| match effect {
                Effect::RunGeneration { job_id, .. } => Some(job_id),
                _ => None,
            })
            .expect("Session starts the requested generation job")
    }

    /// Run a generation to completion: Session starts the job and the test delivers the
    /// completion the worker would have.
    fn finish_generation(runtime: &Rc<Runtime>, scope: &Scope, exact: bool) {
        let job_id = start_generation(runtime, scope);
        support::complete(
            runtime,
            Completion::GenerationFinished {
                job_id,
                scope: scope.clone(),
                exact,
            },
        );
        let _ = support::take_held_effects(runtime);
    }

    /// Run a generation to its failure: Session starts the job and the test delivers the
    /// failure the worker would have.
    fn fail_generation(runtime: &Rc<Runtime>, scope: &Scope, reason: &str) {
        let job_id = start_generation(runtime, scope);
        support::complete(
            runtime,
            Completion::GenerationFailed {
                job_id,
                scope: scope.clone(),
                reason: reason.into(),
            },
        );
        let _ = support::take_held_effects(runtime);
    }

    /// The job effects Session emitted, in order. Settlements are bookkeeping this test does
    /// not inspect.
    fn job_effects(effects: &[Effect]) -> Vec<&'static str> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::RunGeneration { .. } => Some("run-generation"),
                Effect::CancelJob { .. } => Some("cancel-job"),
                Effect::RunExport { .. } => Some("run-export"),
                Effect::CancelExport { .. } => Some("cancel-export"),
                _ => None,
            })
            .collect()
    }

    fn started_generations(effects: &[Effect]) -> Vec<Scope> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::RunGeneration { scope, .. } => Some(scope.clone()),
                _ => None,
            })
            .collect()
    }

    fn started_exports(effects: &[Effect]) -> Vec<Scope> {
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::RunExport { scope, .. } => Some(scope.clone()),
                _ => None,
            })
            .collect()
    }

    /// An exact Case project: a real accepted board with mechanical settings, a finished exact
    /// generation, and the completed geometry the CAD worker would have produced.
    async fn configured_exact_mechanical_runtime() -> (Rc<Runtime>, Scope) {
        let runtime = support::new_runtime();
        let mut document = outlined_board_document();
        document.mechanical = Some(
            boardstudio_web_host::case_settings::initial_settings(&document, "board-left")
                .expect("test board has initial mechanical settings"),
        );
        support::open_document(&runtime, document).await;
        let accepted = runtime.model().accepted.expect("exact fixture accepted");
        let scope = runtime.scope().expect("exact fixture scope");
        finish_generation(&runtime, &scope, true);
        let mechanical = serde_json::from_value::<MechanicalAssembly>(serde_json::json!({
            "suggestedMounts": [],
            "nominalPlateContours": [],
            "revision": accepted.document.revision,
            "plateContours": [],
            "case": {
                "revision": accepted.document.revision,
                "bodies": [{
                    "revision": accepted.document.revision,
                    "body": {
                        "id": "plate",
                        "name": "Plate body",
                        "boardId": scope.board_id,
                        "kind": "plate",
                        "thickness": 1.5,
                        "clearance": 0.2
                    },
                    "contours": []
                }]
            },
            "stack": [{ "id": "plate", "z": 0.0, "thickness": 1.5 }],
            "diagnostics": [],
            "generationBlocked": false
        }))
        .expect("minimal exact mechanical assembly fixture");
        runtime.set_cad_scene_test(Some(Rc::new(crate::runtime::CadScene {
            scope: scope.clone(),
            token: accepted.token,
            snapshot: accepted.clone(),
            result: boardstudio_web_host::cad_jobs::CadResult {
                revision: accepted.document.revision,
                ..Default::default()
            },
            prepared: boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: accepted.document.revision,
                bodies: Vec::new(),
            },
            physical_fingerprint: None,
            mechanical: Some(mechanical),
            exact: true,
            contours: Vec::new(),
        })));
        (runtime, scope)
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(100).await;
    }

    fn live_preview_toggle() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#case-live-preview-mounted input[role='switch']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn case_button(root_id: &str, label: &str) -> web_sys::HtmlElement {
        let buttons = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector_all(&format!("#{root_id} button"))
            .unwrap();
        (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .find(|button| button.text_content().as_deref() == Some(label))
            .unwrap_or_else(|| panic!("mounted Case has {label}"))
            .dyn_into()
            .unwrap()
    }

    fn export_geometry_button_in(root_id: &str) -> web_sys::HtmlElement {
        case_button(root_id, "Export geometry")
    }

    fn refresh_case_in(root_id: &str) {
        case_button(root_id, "Refresh").click();
    }

    fn remove_case_root(root_id: &str) {
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(root) = document.get_element_by_id(root_id)
            && let Some(parent) = root.parent_node()
        {
            let _ = parent.remove_child(&root);
        }
    }

    fn export_geometry_button() -> web_sys::HtmlElement {
        let buttons = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector_all("#case-local-export-mounted button")
            .unwrap();
        (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .find(|button| button.text_content().as_deref() == Some("Export geometry"))
            .expect("Case mounts Export geometry")
            .dyn_into()
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn mounted_case_local_export_uses_current_exact_mechanical_scope() {
        let (runtime, scope) = configured_exact_mechanical_runtime().await;
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("case-local-export-mounted");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(export_host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        let export = export_geometry_button();
        assert!(
            !export.has_attribute("disabled"),
            "current exact geometry is exportable"
        );
        let _ = support::take_held_effects(&runtime);
        export.click();
        settle().await;
        let effects = support::take_held_effects(&runtime);
        assert_eq!(job_effects(&effects), ["run-export"]);
        assert_eq!(started_exports(&effects), [scope.clone()]);

        // A preview-quality generation replaces the exact one and drops the exact geometry.
        finish_generation(&runtime, &scope, false);
        assert!(matches!(
            runtime.model().generation,
            GenerationStatus::Ready { exact: false, .. }
        ));
        runtime.set_cad_scene_test(None);
        refresh_case_in("case-local-export-mounted");
        settle().await;
        let blocked_export = export_geometry_button();
        let _ = support::take_held_effects(&runtime);
        assert!(
            blocked_export.has_attribute("disabled"),
            "preview geometry cannot be exported"
        );
        blocked_export.click();
        settle().await;
        assert!(
            started_exports(&support::take_held_effects(&runtime)).is_empty(),
            "preview geometry must not start a mechanical export"
        );
        remove_case_root("case-local-export-mounted");
    }

    #[wasm_bindgen_test]
    async fn mounted_failed_current_generation_keeps_previous_geometry_and_retries_owner() {
        let (runtime, scope) = configured_exact_mechanical_runtime().await;
        let previous = runtime.model().accepted.expect("exact fixture accepted");
        let previous_scene = runtime.cad_scene().expect("exact fixture geometry");
        let root_id = "case-failed-current-export-mounted";
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id(root_id);
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(export_host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        assert!(
            !export_geometry_button_in(root_id).has_attribute("disabled"),
            "fixture first proves exact current readiness through the real owner hook"
        );
        let _ = support::take_held_effects(&runtime);

        // Disable automatic generation before changing the owner; explicit retry is asserted below.
        let live = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{root_id} input[role='switch']"))
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        live.click();
        settle().await;
        assert!(!live.checked());
        let _ = support::take_held_effects(&runtime);

        let mut edited = previous.document.as_ref().clone();
        edited.name.push_str(" updated");
        let current =
            support::replace_document(&runtime, "case-current-owner-edit", &scope.board_id, edited)
                .await;
        assert!(current.document.revision > previous.document.revision);
        fail_generation(&runtime, &scope, "controlled current generation failure");
        refresh_case_in(root_id);
        settle().await;
        let text = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(root_id)
            .unwrap()
            .text_content()
            .unwrap_or_default();
        assert!(text.contains("Case generation failed: controlled current generation failure"));
        assert!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(&format!("#{root_id} .m1-case-panel"))
                .unwrap()
                .is_some()
        );
        let retained = runtime
            .cad_scene()
            .expect("previous exact geometry is retained");
        assert_eq!(retained.scope, scope);
        assert_eq!(retained.token, previous_scene.token);
        assert_eq!(
            retained.snapshot.document.revision,
            previous.document.revision
        );
        assert_ne!(retained.token, current.token);
        let export = export_geometry_button_in(root_id);
        assert!(export.has_attribute("disabled"));
        let _ = support::take_held_effects(&runtime);
        export.click();
        settle().await;
        assert!(started_exports(&support::take_held_effects(&runtime)).is_empty());

        case_button(root_id, "Update preview").click();
        settle().await;
        let model = runtime.model();
        assert_eq!(model.accepted.as_ref().unwrap().token, current.token);
        assert_eq!(
            model.accepted.as_ref().unwrap().document.revision,
            current.document.revision
        );
        assert_eq!(
            started_generations(&support::take_held_effects(&runtime)),
            [scope.clone()],
            "Update preview retries generation for the current owner"
        );
        remove_case_root(root_id);
    }

    #[wasm_bindgen_test]
    async fn mounted_read_model_preview_and_session_gesture_block_export_until_cancelled() {
        let (runtime, scope) = configured_exact_mechanical_runtime().await;
        let accepted = runtime.model().accepted.expect("exact fixture accepted");
        let root_id = "case-session-draft-export-mounted";
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id(root_id);
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(export_host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"));
        let _ = support::take_held_effects(&runtime);

        // A transient Core preview of an edit becomes the Session's display preview.
        let mut previewed = accepted.document.as_ref().clone();
        previewed.name.push_str(" preview");
        runtime.submit(AppEvent::Edit {
            operation_id: runtime.operation(),
            command: EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "case-export-draft-preview".into(),
                phase: EditPhase::Preview,
                target_ids: vec![scope.board_id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(previewed),
                },
            },
        });
        support::run_pending(&runtime).await;
        assert!(
            runtime.model().display_preview.is_some(),
            "Session accepted the Core preview"
        );
        refresh_case_in(root_id);
        settle().await;
        assert!(export_geometry_button_in(root_id).has_attribute("disabled"));
        let _ = support::take_held_effects(&runtime);
        export_geometry_button_in(root_id).click();
        settle().await;
        assert!(started_exports(&support::take_held_effects(&runtime)).is_empty());

        runtime.submit(AppEvent::ClearPreview {
            operation_id: runtime.operation(),
            token: accepted.token,
            revision: accepted.document.revision,
            board_id: scope.board_id.clone(),
            transaction_id: "case-export-draft-preview".into(),
        });
        support::run_pending(&runtime).await;
        assert!(
            runtime.model().display_preview.is_none(),
            "Session retired the preview"
        );
        refresh_case_in(root_id);
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"));

        // The real Session reducer owns GestureBegin/Cancel. This does not claim browser
        // pointer capture or movement was exercised.
        let pointer_id = 41;
        runtime.submit(AppEvent::GestureBegin {
            operation_id: runtime.operation(),
            pointer_id,
            target_ids: vec![],
            transaction_id: "case-export-session-gesture".into(),
            start: vec![],
            pitch: Vec2 { x: 1.0, y: 1.0 },
            snap_fraction: 1.0,
            geometry_snap: false,
            gap: None,
            alt: false,
        });
        assert!(
            runtime.model().gesture.is_some(),
            "Session reducer accepted GestureBegin"
        );
        refresh_case_in(root_id);
        settle().await;
        assert!(export_geometry_button_in(root_id).has_attribute("disabled"));
        let _ = support::take_held_effects(&runtime);
        export_geometry_button_in(root_id).click();
        settle().await;
        assert!(started_exports(&support::take_held_effects(&runtime)).is_empty());

        runtime.submit(AppEvent::GestureCancel { pointer_id });
        assert!(
            runtime.model().gesture.is_none(),
            "Session reducer accepted GestureCancel"
        );
        let _ = support::take_held_effects(&runtime);
        refresh_case_in(root_id);
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"));
        remove_case_root(root_id);
    }

    #[wasm_bindgen_test]
    async fn mounted_case_panel_starts_once_pauses_and_resumes_for_current_context() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, firmware_export_test_support::board_document()).await;
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("case-live-preview-mounted");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.into()),
        );
        settle().await;
        assert_eq!(
            job_effects(&support::take_held_effects(&runtime)),
            ["run-generation"],
            "mounting Case starts one generation"
        );
        assert!(matches!(
            runtime.model().generation,
            GenerationStatus::Preparing { .. }
        ));

        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("case-live-refresh")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#case-live-preview-mounted .m1-case-header button:nth-of-type(2)")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert_eq!(
            job_effects(&support::take_held_effects(&runtime)),
            ["cancel-job"],
            "Cancel stops the running generation"
        );
        assert!(
            !live_preview_toggle().checked(),
            "Cancel also pauses Live preview"
        );
        assert!(matches!(
            runtime.model().generation,
            GenerationStatus::Cancelled { .. }
        ));
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("case-live-refresh")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;

        assert!(!live_preview_toggle().checked());
        assert!(
            job_effects(&support::take_held_effects(&runtime)).is_empty(),
            "Cancel leaves automatic generation paused"
        );

        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("case-live-unmount")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("case-live-remount")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(
            !live_preview_toggle().checked(),
            "workspace remount keeps the app-level pause"
        );
        assert!(job_effects(&support::take_held_effects(&runtime)).is_empty());

        live_preview_toggle().click();
        settle().await;
        assert!(live_preview_toggle().checked());
        assert_eq!(
            job_effects(&support::take_held_effects(&runtime)),
            ["run-generation"],
            "resuming Live preview starts a generation"
        );
        assert!(matches!(
            runtime.model().generation,
            GenerationStatus::Preparing { .. }
        ));

        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("case-live-refresh")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        live_preview_toggle().click();
        settle().await;
        assert_eq!(
            job_effects(&support::take_held_effects(&runtime)),
            ["cancel-job"],
            "pausing Live preview cancels the running generation"
        );
        assert!(!live_preview_toggle().checked());
    }
}
