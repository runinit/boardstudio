use super::*;
use crate::presentation::shared_viewer::{HandleGesturePhase, SharedViewerEscapeProbe};
use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Session};
use boardstudio_core::CoreEngine;
use boardstudio_model::model::{
    Board, CaseKind, Mount, MountKind, PreparedCaseAssemblyIR, PreparedCaseIR, PreparedCaseRegion,
    ProjectDoc, Vec2,
};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn outer() -> Vec<Vec2> {
    vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 100.0, y: 0.0 },
        Vec2 { x: 100.0, y: 100.0 },
        Vec2 { x: 0.0, y: 100.0 },
    ]
}

async fn accepted_case_fixture() -> (Rc<Runtime>, AcceptedSnapshot, Scope, Rc<CadScene>) {
    let mount = Mount {
        id: "mount-1".into(),
        at: Vec2 { x: 20.0, y: 20.0 },
        kind: MountKind::Boss,
        hole_diameter: 2.0,
        boss_diameter: Some(4.0),
        height: Some(5.0),
    };
    let body = boardstudio_model::model::CaseBody {
        features: None,
        openings: None,
        id: "bottom".into(),
        name: "Bottom".into(),
        board_id: "board-1".into(),
        kind: CaseKind::Tray,
        thickness: 1.5,
        clearance: 0.2,
        material_id: None,
        z: Some(0.0),
        wall_height: Some(20.0),
        wall_thickness: Some(3.0),
        mounts: Some(vec![mount.clone()]),
        pcb_source: None,
        gasket: None,
    };
    let mut document = ProjectDoc::empty("case-gesture-cancel", "Gesture cancellation");
    document.boards.push(Board {
        id: "board-1".into(),
        name: "Board 1".into(),
        outline_ids: vec!["outline-1".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
        .outline
        .push(boardstudio_model::model::OutlineFeature::Polygon {
            anchor_part_id: None,
            id: "outline-1".into(),
            points: outer(),
            operation: boardstudio_model::model::Operation::Add,
        });
    document.case_bodies.push(body.clone());
    document.hardware = Some(boardstudio_model::model::HardwareConfiguration {
        instances: ["primary", "alternate"]
            .into_iter()
            .map(|id| boardstudio_model::model::PhysicalBoardInstance {
                id: id.into(),
                name: id.into(),
                board_id: "board-1".into(),
                half: "left".into(),
                role: "central".into(),
                flipped: false,
                controller_part_id: None,
                mechanical: None,
                construction_ownership:
                    boardstudio_model::model::MechanicalConstructionOwnership::Local,
                mechanical_identity_reservation: None,
            })
            .collect(),
        ..Default::default()
    });

    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::install(
        &runtime,
        Session::new(boardstudio_core::CoreEngine::validate_open),
        CoreEngine::new(),
    );
    runtime.submit(Event::Open {
        operation_id: OperationId(100),
        document,
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    runtime.submit(Event::Navigate {
        operation_id: OperationId(101),
        board_id: "board-1".into(),
        instance_id: Some("primary".into()),
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    let accepted = runtime
        .model()
        .accepted
        .expect("runtime accepted Case fixture");
    let scope = runtime.scope().expect("runtime accepted board scope");
    let polygon = outer();
    let scene = Rc::new(CadScene {
        scope: scope.clone(),
        token: accepted.token,
        snapshot: accepted.clone(),
        result: boardstudio_web_host::cad_jobs::CadResult {
            revision: accepted.document.revision,
            ..Default::default()
        },
        prepared: PreparedCaseAssemblyIR {
            revision: accepted.document.revision,
            bodies: vec![PreparedCaseIR {
                revision: accepted.document.revision,
                body,
                regions: vec![PreparedCaseRegion {
                    outer: polygon,
                    holes: vec![],
                    cavities: vec![],
                    gaskets: vec![],
                    mounts: vec![mount],
                }],
            }],
        },
        physical_fingerprint: None,
        mechanical: None,
        exact: true,
        contours: vec![],
    });
    (runtime, accepted, scope, scene)
}

// Keep real Core preparation; substitute only the CAD worker's mesh output.
fn prepare_preview_for_test(snapshot: AcceptedSnapshot, scope: Scope) -> Result<CadScene, String> {
    use boardstudio_model::model::{CaseAssemblyIR, CaseIR, CoreReply, CoreRequest};
    use boardstudio_web_host::cad_jobs::{CadBodyMesh, CadBounds, CadResult, captured_case_scene};
    let document =
        captured_case_document(&snapshot, &scope).map_err(|error| format!("{error:?}"))?;
    let contours = captured_case_scene(&snapshot, &scope)
        .map_err(|error| format!("{error:?}"))?
        .board_contours
        .into_iter()
        .find(|entry| entry.board_id == scope.board_id)
        .ok_or("accepted Case fixture has no board contours")?
        .contours;
    let ir = CaseAssemblyIR {
        revision: document.revision,
        bodies: document
            .case_bodies
            .iter()
            .filter(|body| body.board_id == scope.board_id)
            .map(|body| CaseIR {
                revision: document.revision,
                body: body.clone(),
                contours: contours.clone(),
            })
            .collect(),
    };
    let prepared = match CoreEngine::new().handle(CoreRequest::PrepareCase {
        id: "case-gesture-test-prepare".into(),
        ir,
    }) {
        CoreReply::CasePrepared { ir, .. } => ir,
        reply => {
            return Err(format!(
                "real Core Case preparation rejected fixture: {reply:?}"
            ));
        }
    };
    let result = CadResult {
        revision: document.revision,
        bodies: prepared
            .bodies
            .iter()
            .map(|body| CadBodyMesh {
                id: body.body.id.clone(),
                name: body.body.name.clone(),
                positions: vec![0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 100.0, 0.0],
                normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            })
            .collect(),
        bounds: Some(CadBounds {
            min: [0.0, 0.0, 0.0],
            max: [100.0, 100.0, 20.0],
        }),
        ..Default::default()
    };
    Ok(CadScene {
        scope,
        token: snapshot.token,
        snapshot,
        result,
        prepared,
        physical_fingerprint: None,
        mechanical: None,
        exact: false,
        contours,
    })
}

fn mounted_case_viewer() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    use_hook(|| {
        runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }));
    });
    let _ = version();
    crate::use_empty_test_instance_selection();
    crate::test_contexts::use_case_viewer_test_contexts();
    crate::use_test_case_generation_state();
    let mut selection = use_context::<CaseSelection>();
    use_hook(move || selection.body_edit_portal.editable.set(true));
    let original = use_context::<Rc<CadScene>>();
    let mut scene = use_signal(|| original.clone());
    let mut mounted = use_signal(|| true);
    let navigate = runtime.clone();
    let away_source = original.clone();
    let restore = runtime.clone();
    let instance = use_context::<InstanceSelection>();
    rsx! {
        style { {include_str!("../../../../../assets/workbench-base.css")} }
        Button { id: "case-gesture-scope", onclick: move |_| {
            instance.reconcile(away_source.scope.session_epoch, away_source.scope.document_id.clone(), "alternate".into());
            navigate.submit(Event::Navigate {
                operation_id: OperationId(102), board_id: away_source.scope.board_id.clone(),
                instance_id: Some("alternate".into()),
            });
            let next = Rc::new(CadScene {
                scope: navigate.scope().expect("accepted alternate physical scope"),
                token: away_source.token,
                snapshot: away_source.snapshot.clone(),
                result: away_source.result.clone(),
                prepared: away_source.prepared.clone(),
                physical_fingerprint: away_source.physical_fingerprint,
                mechanical: away_source.mechanical.clone(),
                exact: away_source.exact,
                contours: away_source.contours.clone(),
            });
            navigate.set_cad_scene_test(Some(next.clone()));
            scene.set(next);
        }, "Other physical assembly" }
        Button { id: "case-gesture-unmount", onclick: move |_| mounted.set(false), "Leave Case viewer" }
        Button { id: "case-gesture-restore", onclick: move |_| {
            instance.reconcile(original.scope.session_epoch, original.scope.document_id.clone(), original.scope.instance_id.clone().unwrap());
            restore.submit(Event::Navigate {
                operation_id: OperationId(103), board_id: original.scope.board_id.clone(),
                instance_id: original.scope.instance_id.clone(),
            });
            restore.set_cad_scene_test(Some(original.clone()));
            scene.set(original.clone());
            mounted.set(true);
        }, "Return to Case viewer" }
        if mounted() {
            CaseViewport {
                scene: Some(scene()),
                preview: None,
                model_rows: None,
                mechanical_settings: None,
            }
        }
    }
}

async fn wait_for_provisional_scene(runtime: &Runtime, source: &CadScene) -> Rc<CadScene> {
    for _ in 0..300 {
        if let Some(scene) = runtime.case_gesture_preview_scene(source) {
            return scene;
        }
        if runtime
            .case_gesture_preview_message()
            .as_deref()
            .is_some_and(|message| message.starts_with("Case preview update failed:"))
        {
            panic!(
                "real provisional Case preview failed: {:?}",
                runtime.case_gesture_preview_message()
            );
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    panic!("real provisional Case preview did not publish within the bounded wait");
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Retirement {
    Escape,
    ScopeChange,
    Unmount,
}

fn click(root: &web_sys::Element, selector: &str) {
    use wasm_bindgen::JsCast;
    root.query_selector(selector)
        .unwrap()
        .expect("mounted retirement control")
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

async fn assert_provisional_retirement(retirement: Retirement) {
    let (runtime, accepted, _scope, scene) = accepted_case_fixture().await;
    runtime.set_cad_scene_test(Some(scene.clone()));
    runtime.set_case_gesture_preview_executor_test(Rc::new(prepare_preview_for_test));
    let saved_before = runtime
        .store
        .load_document(accepted.document.id.clone())
        .await
        .expect("read actual persisted Case fixture")
        .expect("Case Open persisted its accepted document");
    assert_eq!(&saved_before, accepted.document.as_ref());
    let before_model = runtime.model();
    assert_eq!(before_model.accepted.as_ref(), Some(&accepted));

    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-gesture-retirement-test");
    document.body().unwrap().append_child(&root).unwrap();
    let gesture_probe = CaseViewerGestureProbe::default();
    let escape_probe = SharedViewerEscapeProbe::default();
    let dom = VirtualDom::new(mounted_case_viewer);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(scene.clone());
    dom.provide_root_context(gesture_probe.clone());
    dom.provide_root_context(escape_probe.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);

    let canvas = {
        let mut mounted = None;
        for _ in 0..100 {
            mounted = root.query_selector("canvas").unwrap();
            if mounted.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        mounted.expect("mounted CaseViewer exposes its real canvas")
    };
    let mount_handle = "case-mount:bottom/mount-1";
    gesture_probe.emit(HandleGesturePhase::Start, mount_handle, None);
    gesture_probe.emit(
        HandleGesturePhase::Move,
        mount_handle,
        Some([30.0, 30.0, 0.8]),
    );
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document.revision,
        accepted.document.revision
    );
    let provisional = wait_for_provisional_scene(&runtime, &scene).await;
    assert_eq!(provisional.token, accepted.token);
    assert_eq!(
        provisional.snapshot.document.revision,
        accepted.document.revision
    );
    assert!(!provisional.exact);
    assert_eq!(
        provisional.snapshot.document.case_bodies[0]
            .mounts
            .as_ref()
            .unwrap()[0]
            .at,
        Vec2 { x: 30.0, y: 30.0 }
    );
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
    for _ in 0..100 {
        if root
            .text_content()
            .unwrap_or_default()
            .contains("Release to save mount position.")
        {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    assert!(
        root.text_content()
            .unwrap_or_default()
            .contains("Release to save mount position.")
    );

    assert!(
        runtime.case_gesture_preview_active_test(),
        "actual Runtime owner is active before retirement"
    );
    match retirement {
        Retirement::Escape => {
            escape_probe.arm_handle_capture(mount_handle);
            let init = web_sys::KeyboardEventInit::new();
            init.set_key("Escape");
            init.set_bubbles(true);
            init.set_cancelable(true);
            let escape =
                web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap();
            canvas.dispatch_event(&escape).unwrap();
            assert!(
                escape.default_prevented(),
                "Escape cancels the mounted handle capture"
            );
        }
        Retirement::ScopeChange => click(&root, "#case-gesture-scope"),
        Retirement::Unmount => click(&root, "#case-gesture-unmount"),
    }
    for _ in 0..100 {
        let text = root.text_content().unwrap_or_default();
        let view_settled = match retirement {
            Retirement::Escape => text.contains("Move cancelled."),
            Retirement::ScopeChange => {
                runtime.scope().unwrap().instance_id.as_deref() == Some("alternate")
                    && !text.contains("Release to save mount position.")
            }
            Retirement::Unmount => root.query_selector("canvas").unwrap().is_none(),
        };
        if view_settled && !runtime.case_gesture_preview_active_test() {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    assert!(
        !runtime.case_gesture_preview_active_test(),
        "retirement clears the actual owner, not merely a stale filtered getter"
    );
    assert!(runtime.case_gesture_preview_scene(&scene).is_none());
    assert!(runtime.case_gesture_preview_message().is_none());
    match retirement {
        Retirement::Escape => assert!(
            root.text_content()
                .unwrap_or_default()
                .contains("Move cancelled.")
        ),
        Retirement::ScopeChange => {
            assert_eq!(
                runtime.scope().unwrap().instance_id.as_deref(),
                Some("alternate")
            );
            assert!(
                !root
                    .text_content()
                    .unwrap_or_default()
                    .contains("Release to save mount position.")
            );
        }
        Retirement::Unmount => assert!(root.query_selector("canvas").unwrap().is_none()),
    }
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
    if retirement != Retirement::Escape {
        click(&root, "#case-gesture-restore");
        for _ in 0..100 {
            if runtime.scope().as_ref() == Some(&scene.scope)
                && root.query_selector("canvas").unwrap().is_some()
                && !root
                    .text_content()
                    .unwrap_or_default()
                    .contains("Release to save mount position.")
            {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(runtime.scope().as_ref(), Some(&scene.scope));
        assert!(root.query_selector("canvas").unwrap().is_some());
        assert!(
            !root
                .text_content()
                .unwrap_or_default()
                .contains("Release to save mount position.")
        );
    }
    assert_eq!(
        runtime.model(),
        before_model,
        "retirement and return do not change accepted state, durability, or history-facing revision"
    );
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));

    // A Move after retirement/return cannot resume the discarded CaseViewer draft.
    gesture_probe.emit(
        HandleGesturePhase::Move,
        mount_handle,
        Some([35.0, 35.0, 0.8]),
    );
    assert!(runtime.case_gesture_preview_scene(&scene).is_none());
    assert!(runtime.case_gesture_preview_message().is_none());
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
    assert_eq!(
        runtime
            .store
            .load_document(accepted.document.id.clone())
            .await
            .unwrap()
            .as_ref(),
        Some(&saved_before),
        "retirement does not persist a provisional document"
    );
    runtime.submit(Event::Undo {
        operation_id: OperationId(104),
    });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&runtime).await;
    assert_eq!(
        runtime.model().accepted.as_ref().unwrap().document,
        accepted.document,
        "retirement does not add an undo entry to the freshly opened Session"
    );
    click(&root, "#case-gesture-unmount");
    for _ in 0..100 {
        if root.query_selector("canvas").unwrap().is_none() {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    assert!(root.query_selector("canvas").unwrap().is_none());
    runtime.unsubscribe();
    root.remove();
    runtime
        .store
        .delete_project(accepted.document.id.clone())
        .await
        .unwrap();
}

#[wasm_bindgen_test]
async fn mounted_escape_cancels_provisional_case_gesture_without_session_edit() {
    assert_provisional_retirement(Retirement::Escape).await;
}

#[wasm_bindgen_test]
async fn mounted_scope_change_retires_provisional_case_gesture_without_session_edit() {
    assert_provisional_retirement(Retirement::ScopeChange).await;
}

#[wasm_bindgen_test]
async fn mounted_unmount_retires_provisional_case_gesture_without_session_edit() {
    assert_provisional_retirement(Retirement::Unmount).await;
}
