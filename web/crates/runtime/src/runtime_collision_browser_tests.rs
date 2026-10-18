use super::*;
use crate::model_delivery::{
    CadCollisionAssessment, CadCollisionSubject, IDENTITY_ROW_TRANSFORM_F64,
};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Blob, BlobPropertyBag, Url};

async fn fixture() -> (Rc<Runtime>, Rc<CadScene>, String) {
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, in_process_support::board_document()).await;
    project_name_test_support::navigate(&runtime, "main-board").await;
    let snapshot = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap();
    let mut body: boardstudio_core::model::PreparedCaseIR =
        serde_json::from_value(data["preparedBody"].clone()).unwrap();
    body.revision = snapshot.document.revision;
    let scene = Rc::new(CadScene {
        scope: scope.clone(),
        token: snapshot.token,
        snapshot,
        prepared: boardstudio_core::model::PreparedCaseAssemblyIR {
            revision: body.revision,
            bodies: vec![body],
        },
        result: CadResult::default(),
        physical_fingerprint: None,
        mechanical: None,
        exact: true,
        contours: vec![],
    });
    runtime.set_cad_scene_test(Some(scene.clone()));
    // Real Runtime/CadWorker client and transferable messages, with a controlled
    // provider reply. The parent owns combined acceptance against the CAD WASM.
    let source = format!(
        r#"
        const payload = {};
        const cancelled = new Set();
        self.postMessage({{kind:'ready'}});
        self.onmessage = event => {{
            const message = event.data;
            if (message.kind === 'cancel') {{ cancelled.add(message.jobId); return; }}
            const input = JSON.parse(message.frame);
            setTimeout(() => {{
                const frame = {{requestId:input.requestId,jobId:input.jobId,identity:input.identity,
                    operation:input.operation,outcome:cancelled.has(input.jobId)?'cancelled':'completed'}};
                let result = {{revision:input.identity.revision}};
                if (input.operation === 'analyze-generated-body') {{
                    result.generatedAnalysis = {{...payload,bodyId:input.generatedAnalysisRequest.bodyId,
                        revision:input.identity.revision,geometryIdentity:input.generatedAnalysisRequest.geometryIdentity,
                        deflectionMm:input.generatedAnalysisRequest.targetDeflectionMm ?? payload.deflectionMm,
                        surfacePositionsMmF64:new Float64Array(payload.surfacePositionsMmF64),
                        triangleFaceOccurrenceIndices:new Uint32Array(payload.triangleFaceOccurrenceIndices)}};
                }} else {{
                    if (!(message.leftPositions instanceof Float64Array || message.leftPositions instanceof ArrayBuffer)) throw new Error('missing f64 transfer');
                    result.collision = {{kind:'uncertain',reason:'controlled unknown query bound'}};
                }}
                self.postMessage({{kind:'reply',frame:JSON.stringify(frame),result}});
            }}, 20);
        }};
    "#,
        data["directPayload"]
    );
    let properties = BlobPropertyBag::new();
    properties.set_type("text/javascript");
    let blob =
        Blob::new_with_str_sequence_and_options(&Array::of1(&source.into()), &properties).unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();
    let worker = Rc::new(CadWorker::new(&url).unwrap());
    worker.ready().await.unwrap();
    *runtime.cad_worker.borrow_mut() = Some((scope, worker));
    (runtime, scene, url)
}

#[wasm_bindgen_test]
async fn generated_analysis_cache_isolates_requested_accuracy_and_rejects_stale_or_cancelled_work()
{
    let (runtime, scene, url) = fixture().await;
    let active = Rc::new(Cell::new(true));
    let current: Rc<dyn Fn() -> bool> = {
        let active = active.clone();
        Rc::new(move || active.get())
    };
    let coarse = runtime
        .analyze_generated_case_body(&scene, "contract-plate", None, current.clone())
        .await
        .unwrap();
    let cached = runtime
        .analyze_generated_case_body(&scene, "contract-plate", None, current.clone())
        .await
        .unwrap();
    assert!(
        Rc::ptr_eq(&coarse, &cached),
        "same snapshot/geometry/accuracy hits cache"
    );
    let tight = runtime
        .analyze_generated_case_body(&scene, "contract-plate", Some(0.01), current.clone())
        .await
        .unwrap();
    assert_eq!(tight.deflection_mm, 0.01);
    assert!(
        !Rc::ptr_eq(&coarse, &tight),
        "tight query must not reuse default mesh"
    );
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_scene = scene.clone();
    let task_current = current.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = task_runtime
            .analyze_generated_case_body(&task_scene, "contract-plate", Some(0.005), task_current)
            .await;
        let _ = done.send(result);
    });
    TimeoutFuture::new(5).await;
    active.set(false);
    runtime.cancel_collision_checks();
    assert!(receive.await.unwrap().is_err());
    assert_eq!(
        runtime.generated_analysis_cache.borrow().len(),
        2,
        "cancelled output never enters cache"
    );
    active.set(true);
    assert!(
        runtime
            .analyze_generated_case_body(&scene, "contract-plate", Some(0.005), current.clone())
            .await
            .is_ok(),
        "same worker recovers after cancellation"
    );
    runtime.set_cad_scene_test(None);
    assert!(
        runtime
            .analyze_generated_case_body(&scene, "contract-plate", None, Rc::new(|| true))
            .await
            .is_err()
    );
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn collision_worker_observation_stays_uncertain_and_rejects_a_late_result() {
    let (runtime, scene, url) = fixture().await;
    let analysis = runtime
        .analyze_generated_case_body(&scene, "contract-plate", None, Rc::new(|| true))
        .await
        .unwrap();
    let subject = CadCollisionSubject::GeneratedCaseBody {
        id: "contract-plate".into(),
        name: "Plate".into(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        analysis: Some(analysis),
    };
    let result = runtime
        .query_collision_subjects(
            &subject,
            &subject,
            0.0,
            None,
            scene.scope.clone(),
            scene.token,
            scene.snapshot.document.revision,
            Rc::new(|| true),
        )
        .await
        .unwrap();
    assert!(matches!(
        result.assessment,
        CadCollisionAssessment::Unknown { .. }
    ));
    assert!(result.observation.is_some());
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_scene = scene.clone();
    let active = Rc::new(Cell::new(true));
    let callback = active.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = task_runtime
            .query_collision_subjects(
                &subject,
                &subject,
                0.0,
                Some(0.01),
                task_scene.scope.clone(),
                task_scene.token,
                task_scene.snapshot.document.revision,
                Rc::new(move || callback.get()),
            )
            .await;
        let _ = done.send(result);
    });
    TimeoutFuture::new(5).await;
    active.set(false);
    let result = receive.await.unwrap();
    assert!(matches!(result, Err(reason) if reason.contains("stale after")));
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}
