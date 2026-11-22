use crate::model_delivery::{
    CadCollisionAssessment, CadCollisionSubject, IDENTITY_ROW_TRANSFORM_F64,
};
use crate::runtime::{CadScene, Runtime, test_hooks};
use boardstudio_core::model::PreparedCaseIR;
use boardstudio_web_host::{cad_jobs::CadResult, cad_worker::CadWorker};
use boardstudio_web_test_runtime::{in_process_support, project_name_test_support};
use gloo_timers::future::TimeoutFuture;
use js_sys::Array;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Blob, BlobPropertyBag, Url};

async fn fixture() -> (Rc<Runtime>, Rc<CadScene>, String) {
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, in_process_support::board_document()).await;
    project_name_test_support::navigate(&runtime, "main-board").await;
    let snapshot = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap();
    let mut body: PreparedCaseIR = serde_json::from_value(data["preparedBody"].clone()).unwrap();
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
        const activation = {};
        const mode = {};
        let preparations = 0;
        const bits = value => {{ const view = new DataView(new ArrayBuffer(8)); view.setFloat64(0,value); return view.getBigUint64(0).toString(16).padStart(16,'0'); }};
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
                    preparations++;
                    const requested = input.generatedAnalysisRequest.targetDeflectionMm ?? null;
                    const mesh = structuredClone(activation.descriptors.plane);
                    mesh.meshIdentity = 'final-f64-v1/runtime-controlled-'+preparations;
                    mesh.settings.targetDeflectionMm = requested;
                    mesh.settings.selectedDeflectionMm = Math.min(0.1,requested ?? 0.1);
                    const analysisRef = {{workerGeneration:'runtime-controlled',meshIdentity:mesh.meshIdentity,
                        source:{{kind:'generated-body',bodyId:input.generatedAnalysisRequest.bodyId,revision:input.identity.revision,geometryIdentity:input.generatedAnalysisRequest.geometryIdentity}},
                        derivationIdentity:input.generatedAnalysisRequest.geometryIdentity+'/deflection='+(requested===null?'default':bits(requested))+'/analytic-generated-v1'}};
                    result.generatedAnalysis = {{...payload,analysisMesh:mesh,analysisRef,
                        analysisErrorBoundMm:mode==='bounded'?0:null,
                        sourceQualification:mode==='bounded'?{{...activation.sourceReports.plane,meshIdentity:mesh.meshIdentity}}:{{state:'unknown',reasons:['controlled-runtime-source-unavailable']}},
                        bodyId:input.generatedAnalysisRequest.bodyId,
                        revision:input.identity.revision,geometryIdentity:input.generatedAnalysisRequest.geometryIdentity,
                        deflectionMm:input.generatedAnalysisRequest.targetDeflectionMm ?? payload.deflectionMm,
                        surfacePositionsMmF64:new Float64Array(payload.surfacePositionsMmF64),
                        triangleFaceOccurrenceIndices:new Uint32Array(payload.triangleFaceOccurrenceIndices)}};
                }} else {{
                    if (!(message.leftPositions instanceof Float64Array || message.leftPositions instanceof ArrayBuffer)) throw new Error('missing f64 transfer');
                    result.collision = {{kind:'surface-separated',surfaceDistanceMm:1.0,closestPoints:{{
                        triangles:{{left:{{triangleIndex:0,faceOccurrenceIndex:0}},right:{{triangleIndex:0,faceOccurrenceIndex:0}}}},
                        point1Mm:[20,0,0],point2Mm:[21,0,0]}}}};
                    const q = input.collisionQuery;
                    const unknown = () => ({{state:'unknown',reasons:[mode==='missing'?'evidence-unavailable':'controlled-runtime-proof-unavailable']}});
                    const binding = {{snapshotRevision:input.identity.revision,snapshot:{{...input.identity,token:String(input.identity.token),sessionEpoch:String(input.identity.sessionEpoch)}},
                        workerGeneration:q.left.analysisRef.workerGeneration,leftModelId:q.left.modelId,rightModelId:q.right.modelId,
                        leftMeshIdentity:q.left.analysisRef.meshIdentity,rightMeshIdentity:q.right.analysisRef.meshIdentity,
                        leftAnalysis:q.left.analysisRef,rightAnalysis:q.right.analysisRef,leftMesh:q.left.analysisMesh,rightMesh:q.right.analysisMesh,
                        leftRowTransformF64:q.left.rowTransformF64,rightRowTransformF64:q.right.rowTransformF64,
                        clearanceThresholdMm:q.clearanceThresholdMm,requestedAccuracyMm:q.requestedAccuracyMm}};
                    result.queryQualification = mode==='bounded'?structuredClone(activation.generatedPlaneQuery):{{version:1,algorithm:'aabb-interval-exact-v1',
                        meshDistance:unknown(),sourceBounds:[unknown(),unknown()],contactRegion:unknown(),containment:unknown()}};
                    result.queryQualification.binding=binding; result.queryQualification.limits=q.qualificationLimits;
                    if(mode==='bounded') result.queryQualification.sourceBounds.forEach((bound,i)=>bound.meshIdentity=[q.left.analysisRef,q.right.analysisRef][i].meshIdentity);
                }}
                self.postMessage({{kind:'reply',frame:JSON.stringify(frame),result}});
            }}, 20);
        }};
    "#,
        data["directPayload"], data["qualificationV2"]["activation"], r#""unknown""#
    );
    let properties = BlobPropertyBag::new();
    properties.set_type("text/javascript");
    let blob =
        Blob::new_with_str_sequence_and_options(&Array::of1(&source.into()), &properties).unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();
    let worker = Rc::new(CadWorker::new(&url).unwrap());
    worker.ready().await.unwrap();
    test_hooks::set_cad_worker(&runtime, Some((scene.scope.clone(), worker)));
    (runtime, scene, url)
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
    test_hooks::set_cad_worker(&runtime, None);
    Url::revoke_object_url(&url).unwrap();
}
