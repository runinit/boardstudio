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
    use boardstudio_application::{
        AcceptedSnapshot, Event as AppEvent, GenerationStatus, JobId, Scope, SessionEpoch,
        SnapshotToken,
    };
    use boardstudio_core::model::{
        Board, BoardReadiness, MechanicalAssembly, ProjectDoc, Readiness, SceneDelta,
    };
    use std::sync::Arc;
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
        crate::presentation::use_case_viewer_test_contexts();
        crate::presentation::use_test_case_generation_state();
        let generation_ready =
            crate::presentation::use_case_generation_readiness_test_bridge(runtime);
        rsx! {
            button { id: "case-export-refresh", onclick: move |_| runtime_version += 1, "Refresh" }
            CasePanel { generation_ready }
        }
    }

    fn exportable_model(runtime: &Runtime) -> boardstudio_application::ReadModel {
        let mut model = runtime.model();
        let revision = model.accepted.as_ref().expect("accepted fixture").document.revision;
        model.lifecycle = boardstudio_application::Lifecycle::Ready;
        model.durability = boardstudio_application::Durability::Saved { revision };
        model
    }

    fn configured_exact_mechanical_runtime() -> (Rc<Runtime>, Scope) {
        let runtime = Runtime::new().expect("browser runtime fixture initializes");
        let (session, opened, scope) =
            crate::runtime::firmware_export_test_support::opened_session();
        let mut document = (*opened.document).clone();
        document.mechanical = Some(
            boardstudio_web_host::case_settings::initial_settings(&document, &scope.board_id)
                .expect("test board has initial mechanical settings"),
        );
        let mut delta = (*opened.scene).clone();
        delta.readiness.outline = true;
        delta.board_readiness = vec![BoardReadiness {
            board_id: scope.board_id.clone(),
            outline: true,
            pcb: true,
            case_ready: true,
        }];
        let accepted = AcceptedSnapshot {
            token: opened.token,
            session_epoch: opened.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(delta),
        };
        let _ = session;
        runtime.set_definition_name_test_state(accepted.clone(), Some(scope.clone()));
        runtime.set_definition_name_test_generation(GenerationStatus::Ready {
            job_id: JobId(1),
            exact: true,
        });
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
        runtime.set_definition_name_test_model(exportable_model(&runtime));
        (runtime, scope)
    }

    fn accepted(token: u64) -> (AcceptedSnapshot, Scope) {
        let mut document = ProjectDoc::empty("case-live-test", "Case live preview test");
        document.boards.push(Board {
            id: "board-left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let revision = document.revision;
        let epoch = SessionEpoch(7);
        let scope = Scope {
            session_epoch: epoch,
            document_id: document.id.clone(),
            board_id: "board-left".into(),
            instance_id: None,
        };
        let scene = SceneDelta {
            module_scenes: vec![],
            revision,
            transaction_id: "case-live-test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: false,
                pcb: true,
                case_ready: true,
            },
        };
        (
            AcceptedSnapshot {
                token: SnapshotToken(token),
                session_epoch: epoch,
                document: Arc::new(document),
                scene: Arc::new(scene),
            },
            scope,
        )
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(100).await;
    }

    fn take_events(runtime: &Runtime) -> Vec<AppEvent> {
        let mut events = Vec::new();
        while let Some(event) = runtime.take_definition_name_test_event() {
            events.push(event);
        }
        events
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

    fn accepted_after_revision(original: &AcceptedSnapshot) -> AcceptedSnapshot {
        let revision = original.document.revision + 1;
        let mut document = (*original.document).clone();
        document.revision = revision;
        document.name.push_str(" updated");
        let mut scene = (*original.scene).clone();
        scene.revision = revision;
        AcceptedSnapshot {
            token: SnapshotToken(original.token.0 + 1),
            session_epoch: original.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(scene),
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
        let (runtime, scope) = configured_exact_mechanical_runtime();
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
        export.click();
        settle().await;
        assert!(matches!(
            take_events(&runtime).as_slice(),
            [AppEvent::StartExport { scope: actual, .. }] if actual == &scope
        ));

        runtime.set_definition_name_test_generation(GenerationStatus::Ready {
            job_id: JobId(2),
            exact: false,
        });
        let mut preview_model = exportable_model(&runtime);
        preview_model.generation = GenerationStatus::Ready { job_id: JobId(2), exact: false };
        runtime.set_definition_name_test_model(preview_model);
        runtime.set_cad_scene_test(None);
        refresh_case_in("case-local-export-mounted");
        settle().await;
        let blocked_export = export_geometry_button();
        let _ = take_events(&runtime);
        assert!(
            blocked_export.has_attribute("disabled"),
            "preview geometry cannot be exported"
        );
        blocked_export.click();
        settle().await;
        assert!(
            !take_events(&runtime)
                .iter()
                .any(|event| matches!(event, AppEvent::StartExport { .. })),
            "preview geometry must not start a mechanical export"
        );
        remove_case_root("case-local-export-mounted");
    }

    #[wasm_bindgen_test]
    async fn mounted_failed_current_generation_keeps_previous_geometry_and_retries_owner() {
        let (runtime, scope) = configured_exact_mechanical_runtime();
        let previous = runtime.model().accepted.expect("exact fixture accepted");
        let previous_scene = runtime.cad_scene().expect("exact fixture geometry");
        let root_id = "case-failed-current-export-mounted";
        let root = web_sys::window().unwrap().document().unwrap().create_element("div").unwrap();
        root.set_id(root_id);
        web_sys::window().unwrap().document().unwrap().body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(export_host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(dom, dioxus_web::Config::new().rootnode(root.clone().into()));
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"),
            "fixture first proves exact current readiness through the real owner hook");
        let _ = take_events(&runtime);

        // Disable automatic generation before changing the owner; explicit retry is asserted below.
        let live = web_sys::window().unwrap().document().unwrap()
            .query_selector(&format!("#{root_id} input[role='switch']")).unwrap().unwrap()
            .dyn_into::<HtmlInputElement>().unwrap();
        live.click();
        settle().await;
        assert!(!live.checked());
        let _ = take_events(&runtime);

        let current = accepted_after_revision(&previous);
        runtime.set_definition_name_test_state(current.clone(), Some(scope.clone()));
        runtime.set_definition_name_test_generation(GenerationStatus::Failed {
            job_id: JobId(2), reason: "controlled current generation failure".into(),
        });
        let mut failed = exportable_model(&runtime);
        failed.accepted = Some(current.clone());
        failed.durability = boardstudio_application::Durability::Saved {
            revision: current.document.revision,
        };
        failed.generation = GenerationStatus::Failed {
            job_id: JobId(2), reason: "controlled current generation failure".into(),
        };
        runtime.set_definition_name_test_model(failed);
        refresh_case_in(root_id);
        settle().await;
        let text = web_sys::window().unwrap().document().unwrap()
            .get_element_by_id(root_id).unwrap().text_content().unwrap_or_default();
        assert!(text.contains("Case generation failed: controlled current generation failure"));
        assert!(web_sys::window().unwrap().document().unwrap()
            .query_selector(&format!("#{root_id} .m1-case-panel")).unwrap().is_some());
        let retained = runtime.cad_scene().expect("previous exact geometry is retained");
        assert_eq!(retained.scope, scope);
        assert_eq!(retained.token, previous_scene.token);
        assert_eq!(retained.snapshot.document.revision, previous.document.revision);
        assert_ne!(retained.token, current.token);
        let export = export_geometry_button_in(root_id);
        assert!(export.has_attribute("disabled"));
        let _ = take_events(&runtime);
        export.click();
        settle().await;
        assert!(!take_events(&runtime).iter().any(|event|
            matches!(event, AppEvent::StartExport { .. })));

        case_button(root_id, "Update preview").click();
        settle().await;
        let model = runtime.model();
        assert_eq!(model.accepted.as_ref().unwrap().token, current.token);
        assert_eq!(model.accepted.as_ref().unwrap().document.revision, current.document.revision);
        assert!(matches!(take_events(&runtime).as_slice(),
            [AppEvent::StartGeneration { scope: actual, .. }] if actual == &scope));
        remove_case_root(root_id);
    }

    #[wasm_bindgen_test]
    async fn mounted_read_model_preview_and_session_gesture_block_export_until_cancelled() {
        let (runtime, _) = configured_exact_mechanical_runtime();
        let accepted = runtime.model().accepted.expect("exact fixture accepted");
        let root_id = "case-session-draft-export-mounted";
        let root = web_sys::window().unwrap().document().unwrap().create_element("div").unwrap();
        root.set_id(root_id);
        web_sys::window().unwrap().document().unwrap().body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(export_host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(dom, dioxus_web::Config::new().rootnode(root.clone().into()));
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"));

        // Seed the real transient ReadModel field at the mounted owner seam; this does not run a preview worker.
        let mut preview = exportable_model(&runtime);
        preview.display_preview = Some(accepted.scene.clone());
        runtime.set_definition_name_test_model(preview);
        refresh_case_in(root_id);
        settle().await;
        assert!(export_geometry_button_in(root_id).has_attribute("disabled"));
        let _ = take_events(&runtime);
        export_geometry_button_in(root_id).click();
        settle().await;
        assert!(!take_events(&runtime).iter().any(|event|
            matches!(event, AppEvent::StartExport { .. })));

        let mut settled_model = exportable_model(&runtime);
        settled_model.display_preview = None;
        runtime.set_definition_name_test_model(settled_model);
        refresh_case_in(root_id);
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"));

        // Use the real Session reducer for GestureBegin/Cancel, then mount its resulting state.
        // This does not claim browser pointer capture or movement was exercised.
        let (mut session, _, _) = crate::runtime::firmware_export_test_support::opened_session();
        let pointer_id = 41;
        let _ = session.submit(AppEvent::GestureBegin {
            operation_id: runtime.operation(), pointer_id, target_ids: vec![],
            transaction_id: "case-export-session-gesture".into(), start: vec![],
            pitch: boardstudio_core::model::Vec2 { x: 1.0, y: 1.0 },
            snap_fraction: 1.0, geometry_snap: false, gap: None, alt: false,
        });
        let mut gesture = exportable_model(&runtime);
        gesture.gesture = session.read_model().gesture.clone();
        assert!(gesture.gesture.is_some(), "Session reducer accepted GestureBegin");
        runtime.set_definition_name_test_model(gesture);
        refresh_case_in(root_id);
        settle().await;
        assert!(export_geometry_button_in(root_id).has_attribute("disabled"));
        let _ = take_events(&runtime);
        export_geometry_button_in(root_id).click();
        settle().await;
        assert!(!take_events(&runtime).iter().any(|event|
            matches!(event, AppEvent::StartExport { .. })));

        let _ = session.submit(AppEvent::GestureCancel { pointer_id });
        let mut settled = exportable_model(&runtime);
        settled.gesture = session.read_model().gesture.clone();
        assert!(settled.gesture.is_none(), "Session reducer accepted GestureCancel");
        runtime.set_definition_name_test_model(settled);
        refresh_case_in(root_id);
        settle().await;
        assert!(!export_geometry_button_in(root_id).has_attribute("disabled"));
        remove_case_root(root_id);
    }

    #[wasm_bindgen_test]
    async fn mounted_case_panel_starts_once_pauses_and_resumes_for_current_context() {
        let runtime = Runtime::new().unwrap();
        let (snapshot, scope) = accepted(1);
        runtime.set_definition_name_test_state(snapshot.clone(), Some(scope.clone()));
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
        assert!(matches!(
            take_events(&runtime).as_slice(),
            [AppEvent::StartGeneration { .. }]
        ));

        runtime
            .set_definition_name_test_generation(GenerationStatus::Preparing { job_id: JobId(1) });
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
        assert!(matches!(
            take_events(&runtime).as_slice(),
            [AppEvent::CancelGeneration { .. }]
        ));
        assert!(
            !live_preview_toggle().checked(),
            "Cancel also pauses Live preview"
        );
        runtime
            .set_definition_name_test_generation(GenerationStatus::Cancelled { job_id: JobId(1) });
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
            take_events(&runtime).is_empty(),
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
        assert!(take_events(&runtime).is_empty());

        live_preview_toggle().click();
        settle().await;
        assert!(live_preview_toggle().checked());
        assert!(matches!(
            take_events(&runtime).as_slice(),
            [AppEvent::StartGeneration { .. }]
        ));

        runtime
            .set_definition_name_test_generation(GenerationStatus::Preparing { job_id: JobId(2) });
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
        assert!(matches!(
            take_events(&runtime).as_slice(),
            [AppEvent::CancelGeneration { .. }]
        ));
        assert!(!live_preview_toggle().checked());
    }
}
