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
            }
            p { role: "status", "aria-live": "polite", "{title}" }
            if runtime.native_case_preview_pending() {
                p { role: "status", "aria-live": "polite", "Preparing the accepted PCB preview…" }
            } else if runtime.native_case_preview_error().is_some() {
                p { role: "alert", "The accepted PCB preview could not be prepared." }
            } else if runtime.native_case_preview().is_some() {
                p { role: "status", "Accepted PCB preview is ready for the Case viewer." }
            }
            if let Some(preview) = accepted_preview.as_ref()
                && let Some(rows) = runtime.native_model_delivery(preview)
            {
                p { role: "status", "{rows.delivered.len()} of {preview.preview.models.len()} board models decoded." }
                for failure in rows.failures.iter().take(3) {
                    p { role: "status", "{failure.reference}: {failure.reason}" }
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
    use boardstudio_core::model::{Board, ProjectDoc, Readiness, SceneDelta};
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
