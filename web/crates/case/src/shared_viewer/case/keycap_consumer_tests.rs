use super::*;
use crate::presentation::shared_viewer::SharedViewerProjectionProbe;
use crate::runtime::KeycapsPreviewInput;
use boardstudio_application::{Event, OperationId, Session};
use boardstudio_core::CoreEngine;
use boardstudio_model::model::{CoreReply, CoreRequest, PreparedCaseAssemblyIR};
use boardstudio_web_host::cad_jobs::{CadBodyMesh, CadResult};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use futures_channel::oneshot;
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Float32Array, Reflect};
use sha2::{Digest, Sha256};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

const ROUTED_SOURCE: &[u8] = b"(kicad_pcb (version 20240108))";

fn document() -> ProjectDoc {
    let mut value =
        serde_json::to_value(ProjectDoc::empty("case-keycaps-mounted", "Case caps")).unwrap();
    value["definitions"] =
        serde_json::json!([{"id":"mx","name":"MX","kind":"switch","courtyard":[],"pads":[]}]);
    value["parts"] = serde_json::json!([{"id":"key","definitionId":"mx","reference":"SW1","pose":{"at":{"x":20,"y":30},"rotation":15},"side":"front"}]);
    value["boards"] = serde_json::json!([{"id":"board","name":"Board","partIds":["key"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
    value["keycaps"] = serde_json::json!({"boards":{},"matrices":{},"keys":{"key":{"profile":"sa","mount":"mx","units":{"x":7,"y":1},"color":"#22aa44","legend":""}}});
    value["assets"] = serde_json::json!([{"id":"routed","name":"board.kicad_pcb","mediaType":"application/vnd.kicad.pcb","sha256":Sha256::digest(ROUTED_SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>()}]);
    value["boardReferences"] = serde_json::json!([{"id":"ref","boardId":"board","assetId":"routed","enabled":true,"pose":{"at":{"x":4.2,"y":-3.1},"rotation":15},"elevation":2.5,"modelAssets":{}}]);
    value["hardware"] = serde_json::json!({"topology":"unibody","transport":"none","boards":[],"instances":[
        {"id":"primary","name":"Primary","boardId":"board","half":"left","role":"central","flipped":false,"controllerPartId":null,"mechanical":null,"constructionLinked":false},
        {"id":"flipped","name":"Flipped","boardId":"board","half":"right","role":"peripheral","flipped":true,"controllerPartId":null,"mechanical":null,"constructionLinked":false}
    ]});
    serde_json::from_value(value).unwrap()
}

fn mounted_host() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    use_hook(|| {
        runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }))
    });
    let _ = version();
    crate::use_empty_test_instance_selection();
    crate::test_contexts::use_case_viewer_test_contexts();
    crate::use_test_case_generation_state();
    let mut mounted = use_signal(|| true);
    let mut cad = use_signal(|| false);
    let instance = use_context::<InstanceSelection>();
    let other = runtime.clone();
    let restore = runtime.clone();
    let use_cad = runtime.clone();
    rsx! {
        Button { id: "case-caps-flipped", onclick: move |_| {
            let scope = other.scope().unwrap();
            instance.reconcile(scope.session_epoch, scope.document_id, "flipped".into());
            other.submit(Event::Navigate { operation_id: OperationId(202), board_id: "board".into(), instance_id: Some("flipped".into()) });
            other.install_case_pcb_preview_test();
            cad.set(false);
        }, "Flipped physical instance" }
        Button { id: "case-caps-primary", onclick: move |_| {
            let scope = restore.scope().unwrap();
            instance.reconcile(scope.session_epoch, scope.document_id, "primary".into());
            restore.submit(Event::Navigate { operation_id: OperationId(203), board_id: "board".into(), instance_id: Some("primary".into()) });
            restore.install_case_pcb_preview_test();
            cad.set(false);
        }, "Original physical instance" }
        Button { id: "case-caps-cad", onclick: move |_| {
            let accepted = use_cad.model().accepted.unwrap();
            use_cad.set_cad_scene_test(Some(Rc::new(CadScene {
                scope: use_cad.scope().unwrap(), token: accepted.token,
                result: CadResult { revision: accepted.document.revision, ..Default::default() },
                prepared: PreparedCaseAssemblyIR { revision: accepted.document.revision, bodies: vec![] },
                snapshot: accepted, physical_fingerprint: None, mechanical: None, exact: true, contours: vec![],
            })));
            cad.set(true);
        }, "Case geometry consumer" }
        Button { id: "case-caps-unmount", onclick: move |_| mounted.set(false), "Leave Case" }
        if mounted() {
            if cad() {
                CaseViewport { scene: runtime.cad_scene(), preview: runtime.native_case_preview(), model_rows: None, mechanical_settings: None }
            } else if let Some(preview) = runtime.native_case_preview() {
                CaseViewport { scene: None, preview: Some(preview), model_rows: None, mechanical_settings: None }
            }
        }
    }
}

fn meshes(input: &KeycapsPreviewInput) -> Vec<CadBodyMesh> {
    input
        .specs
        .iter()
        .map(|spec| CadBodyMesh {
            id: format!("keycap:{}", spec.id),
            name: spec.reference.clone(),
            positions: vec![
                spec.pose.at.x as f32,
                spec.pose.at.y as f32,
                spec.z as f32,
                21.0,
                30.0,
                10.0,
                20.0,
                31.0,
                10.0,
            ],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
        })
        .collect()
}

fn cap(packet: &JsValue) -> Option<JsValue> {
    let bodies = Reflect::get(packet, &"bodies".into()).ok()?;
    if !Array::is_array(&bodies) {
        return None;
    }
    Array::from(&bodies).iter().find(|body| {
        Reflect::get(body, &"id".into())
            .ok()
            .and_then(|id| id.as_string())
            .as_deref()
            == Some("keycap:key")
    })
}

fn click(root: &web_sys::Element, selector: &str) {
    root.query_selector(selector)
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

#[wasm_bindgen_test]
async fn mounted_case_keycaps_reach_both_packets_and_reject_late_physical_owner() {
    let runtime = support::new_runtime();
    support::install(
        &runtime,
        Session::new(CoreEngine::validate_open),
        CoreEngine::new(),
    );
    runtime
        .store
        .save_asset(boardstudio_web_host::host::AssetBytes {
            sha256: Sha256::digest(ROUTED_SOURCE)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            bytes: ROUTED_SOURCE.to_vec(),
        })
        .await
        .expect("required reference bytes exist before actual Open/Persist");
    runtime.submit(Event::Open {
        operation_id: OperationId(200),
        document: document(),
    });
    support::run_pending(&runtime).await;
    runtime.submit(Event::Navigate {
        operation_id: OperationId(201),
        board_id: "board".into(),
        instance_id: Some("primary".into()),
    });
    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.unwrap_or_else(|| {
        panic!(
            "fixture Open/Persist did not accept its document: {:?}",
            runtime.model()
        )
    });
    let physical = captured_case_document(&accepted, &runtime.scope().unwrap()).unwrap();
    let CoreReply::KeycapsResolved { result, .. } =
        CoreEngine::new().handle(CoreRequest::ResolveKeycaps {
            id: "fixture-proof".into(),
            document: physical,
            board_id: "board".into(),
            cases: None,
        })
    else {
        panic!("real Core resolves accepted physical keycaps");
    };
    assert_eq!(result.specs.len(), 1);
    assert!(
        result
            .findings
            .iter()
            .all(|finding| finding.severity != boardstudio_model::model::Severity::Error)
    );
    assert!(
        (result.specs[0].size.x - 132.5).abs() < 1e-9,
        "accepted 7u override resolves through Core"
    );
    runtime.install_case_pcb_preview_test();
    assert!(
        runtime
            .native_case_preview()
            .unwrap()
            .board_reference
            .is_some()
    );
    let requests = Rc::new(RefCell::new(Vec::<KeycapsPreviewInput>::new()));
    let (release, held) = oneshot::channel::<()>();
    let held = Rc::new(RefCell::new(Some(held)));
    runtime.set_keycaps_preview_executor_test(Rc::new({
        let requests = requests.clone();
        move |input| {
            requests.borrow_mut().push(input.clone());
            let wait = if input.scope.instance_id.as_deref() == Some("flipped") {
                held.borrow_mut().take()
            } else {
                None
            };
            Box::pin(async move {
                if let Some(wait) = wait {
                    wait.await
                        .map_err(|_| "controlled CAD reply dropped".to_owned())?;
                }
                Ok(meshes(&input))
            })
        }
    }));
    let packets = Rc::new(RefCell::new(Vec::<(ViewerIdentity, JsValue)>::new()));
    let probe = SharedViewerProjectionProbe(Rc::new({
        let packets = packets.clone();
        move |id, packet| packets.borrow_mut().push((id, packet))
    }));
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    root.set_id("mounted-case-keycap-consumer");
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(probe);
    let _mounted = MountedFixture::new(root.clone(), dom);
    for _ in 0..200 {
        if packets
            .borrow()
            .last()
            .is_some_and(|(_, packet)| cap(packet).is_some())
        {
            break;
        }
        TimeoutFuture::new(10).await;
    }
    assert!(
        root.query_selector("canvas").unwrap().is_some(),
        "actual Case viewer mounted"
    );
    let body = packets.borrow().last().and_then(|(_, packet)| cap(packet));
    assert!(
        body.is_some(),
        "mounted Case renderer packet must contain generated authored keycaps, distinct from PCB model rows"
    );
    let body = body.unwrap();
    assert_eq!(
        Reflect::get(&body, &"color".into())
            .unwrap()
            .as_string()
            .as_deref(),
        Some("#22aa44")
    );
    let mesh = Reflect::get(&body, &"mesh".into()).unwrap();
    assert_eq!(
        Float32Array::new(&Reflect::get(&mesh, &"positions".into()).unwrap()).to_vec()[0],
        20.0
    );
    assert_eq!(
        requests.borrow()[0].scope.instance_id.as_deref(),
        Some("primary")
    );

    let before_cad = requests.borrow().len();
    click(&root, "#case-caps-cad");
    for _ in 0..200 {
        if requests.borrow().len() > before_cad
            && packets
                .borrow()
                .last()
                .is_some_and(|(_, packet)| cap(packet).is_some())
        {
            break;
        }
        TimeoutFuture::new(10).await;
    }
    assert!(
        requests.borrow().len() > before_cad,
        "Case geometry consumer requests its own caps"
    );
    assert!(
        packets
            .borrow()
            .last()
            .is_some_and(|(_, packet)| cap(packet).is_some()),
        "Case geometry packet retains generated caps"
    );

    click(&root, "#case-caps-flipped");
    for _ in 0..200 {
        if requests
            .borrow()
            .iter()
            .any(|input| input.scope.instance_id.as_deref() == Some("flipped"))
        {
            break;
        }
        TimeoutFuture::new(10).await;
    }
    let flipped = requests
        .borrow()
        .iter()
        .find(|input| input.scope.instance_id.as_deref() == Some("flipped"))
        .cloned()
        .expect("new physical owner reaches CAD");
    assert_eq!(
        flipped.specs[0].pose.at.x, -20.0,
        "Case resolves physical coordinates rather than canonical Layout coordinates"
    );
    assert_eq!(flipped.specs[0].pose.rotation, -15.0);
    assert!(
        packets
            .borrow()
            .last()
            .is_some_and(
                |(id, packet)| id.scope.instance_id.as_deref() == Some("flipped")
                    && cap(packet).is_none()
            ),
        "old caps retire while the new physical owner is pending"
    );
    click(&root, "#case-caps-primary");
    for _ in 0..200 {
        if packets.borrow().last().is_some_and(|(id, packet)| {
            id.scope.instance_id.as_deref() == Some("primary") && cap(packet).is_some()
        }) {
            break;
        }
        TimeoutFuture::new(10).await;
    }
    release.send(()).unwrap();
    TimeoutFuture::new(50).await;
    {
        let packets = packets.borrow();
        let (id, packet) = packets.last().unwrap();
        assert_eq!(id.scope.instance_id.as_deref(), Some("primary"));
        let body = cap(packet).expect("late flipped CAD must not replace current keycaps");
        let mesh = Reflect::get(&body, &"mesh".into()).unwrap();
        assert_eq!(
            Float32Array::new(&Reflect::get(&mesh, &"positions".into()).unwrap()).to_vec()[0],
            20.0
        );
    }
    assert_eq!(runtime.model().accepted.as_ref(), Some(&accepted));
    assert_eq!(
        runtime
            .store
            .load_document(accepted.document.id.clone())
            .await
            .unwrap()
            .as_ref(),
        Some(accepted.document.as_ref()),
        "display generation does not edit or persist the accepted design"
    );
    click(&root, "#case-caps-unmount");
    TimeoutFuture::new(30).await;
    runtime.unsubscribe();
    root.remove();
    runtime
        .store
        .delete_project(accepted.document.id.clone())
        .await
        .unwrap();
}
