//! Controlled provider ownership/transport tests. This double authenticates no geometry.
use super::*;
use crate::cad_jobs::*;
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json")).unwrap()
}

pub(super) fn controlled_provider(unqualified: bool) -> JsValue {
    let factory = Function::new_with_args("fixture,unqualified", r#"
        const a = fixture.qualificationV2.activation;
        const controls = {cost:1024,prepares:0,queries:0,raw:0,frees:0,badQualifier:false,unqualified};
        const unknown = reason => ({state:'unknown',reasons:[reason]});
        class AnalysisHandle {
            #positions; #tags; #freed = false;
            constructor(p,t) { this.#positions = new Float64Array(p); this.#tags = new Uint32Array(t); }
            retainedStorageBytes() { return controls.cost; }
            free() { if (this.#freed) throw new Error('double free'); this.#freed=true; controls.frees++; }
            compare(p,t) {
                if (this.#freed) throw new Error('freed handle');
                const left = new Uint8Array(this.#positions.buffer), right = new Uint8Array(p.buffer,p.byteOffset,p.byteLength);
                if (left.length !== right.length || left.some((v,i)=>v!==right[i]) || t.length!==this.#tags.length || t.some((v,i)=>v!==this.#tags[i])) throw new Error('buffer bits mismatch');
            }
        }
        return {AnalysisHandle,controls,
            analyze_generated_body(body,options) {
                controls.prepares++;
                const p = structuredClone(fixture.directPayload);
                const mesh = structuredClone(a.descriptors.plane);
                mesh.meshIdentity = 'final-f64-v1/controlled-'+controls.prepares;
                mesh.settings.targetDeflectionMm = options.targetDeflectionMm ?? null;
                mesh.settings.selectedDeflectionMm = Math.min(0.1, options.targetDeflectionMm ?? 0.1);
                const source = controls.unqualified ? unknown('controlled-source-unavailable') : {...a.sourceReports.plane,meshIdentity:mesh.meshIdentity};
                const positions = new Float64Array(p.surfacePositionsMmF64), tags = new Uint32Array(p.triangleFaceOccurrenceIndices);
                return {...p,bodyId:body.body.id,revision:body.revision,deflectionMm:mesh.settings.selectedDeflectionMm,
                    surfacePositionsMmF64:positions,triangleFaceOccurrenceIndices:tags,analysisErrorBoundMm:controls.unqualified?null:0,
                    analysisMesh:mesh,sourceQualification:source,analysisHandle:new AnalysisHandle(positions,tags)};
            },
            query_qualified_collision_mesh_pair(buffers,q,context,left,right) {
                controls.queries++;
                left.compare(buffers.leftPositions,buffers.leftTags); right.compare(buffers.rightPositions,buffers.rightTags);
                const report = structuredClone(a.generatedPlaneQuery);
                report.binding = {snapshotRevision:context.snapshot.revision,snapshot:context.snapshot,workerGeneration:context.workerGeneration,
                    leftModelId:q.left.modelId,rightModelId:q.right.modelId,leftMeshIdentity:q.left.analysisRef.meshIdentity,rightMeshIdentity:q.right.analysisRef.meshIdentity,
                    leftAnalysis:q.left.analysisRef,rightAnalysis:q.right.analysisRef,leftMesh:q.left.analysisMesh,rightMesh:q.right.analysisMesh,
                    leftRowTransformF64:q.left.rowTransformF64,rightRowTransformF64:q.right.rowTransformF64,
                    clearanceThresholdMm:q.clearanceThresholdMm,requestedAccuracyMm:q.requestedAccuracyMm};
                report.limits = q.qualificationLimits;
                report.sourceBounds.forEach((bound,i)=>bound.meshIdentity=[q.left.analysisRef,q.right.analysisRef][i].meshIdentity);
                if (controls.unqualified || buffers.leftTags.length > q.qualificationLimits.maxTriangles || buffers.rightTags.length > q.qualificationLimits.maxTriangles) {
                    report.meshDistance=unknown('resource-or-source-unavailable'); report.sourceBounds=[unknown('unavailable'),unknown('unavailable')];
                    report.contactRegion=unknown('unavailable'); report.containment=unknown('unavailable');
                }
                if (controls.badQualifier) report.binding.snapshot.token='999';
                return {observation:structuredClone(a.rawObservationExample),queryQualification:report};
            },
            query_collision_mesh_pair() { controls.raw++; return structuredClone(a.rawObservationExample); }
        };
    "#);
    factory.call2(&JsValue::NULL, &js_sys::JSON::parse(&fixture().to_string()).unwrap(), &unqualified.into()).unwrap()
}

fn preparation(id: &str) -> WireRequest {
    let body: boardstudio_core::model::PreparedCaseIR = serde_json::from_value(fixture()["preparedBody"].clone()).unwrap();
    WireRequest {
        request_id: id.into(), job_id: id.into(), identity: CadSnapshotIdentity { token: 11, session_epoch: 2,
            document_id: "contract-document".into(), board_id: "contract-board".into(), instance_id: None, revision: body.revision },
        operation: CadOperation::AnalyzeGeneratedBody,
        generated_analysis_request: Some(CadGeneratedAnalysisRequest { body_id: body.body.id.clone(), geometry_identity: generated_geometry_identity(&body).unwrap(), target_deflection_mm: None }),
        prepared: Some(boardstudio_core::model::PreparedCaseAssemblyIR { revision: body.revision, bodies: vec![body] }),
        source_sha256: None, step_import_options: None, collision_query: None, collision_meshes: None, input_bytes: vec![], keycaps: None,
    }
}

async fn prepare(queue: &Rc<RefCell<WorkerQueue>>, module: &JsValue, id: &str) -> CadGeneratedAnalysis {
    let (_, result) = run_request(queue, module, preparation(id), vec![]).await.unwrap_or_else(|(_,e,_)| panic!("{e}"));
    decode_result(result).unwrap().generated_analysis.unwrap()
}

fn query(asset: &CadGeneratedAnalysis) -> WireRequest {
    let mut request = preparation("query");
    request.operation = CadOperation::CollisionQuery; request.prepared = None; request.generated_analysis_request = None;
    let binding: qualification::Binding = serde_json::from_value(fixture()["qualificationV2"]["activation"]["generatedPlaneQuery"]["binding"].clone()).unwrap();
    let operand = |model: &str, row| CadCollisionOperand { model_id: model.into(), source_sha256: None,
        generated_geometry_identity: Some(asset.geometry_identity.clone()), import_options: Default::default(), row_transform_f64: row,
        analysis_ref: asset.analysis_ref.clone(), analysis_mesh: asset.analysis_mesh.clone() };
    request.collision_query = Some(CadCollisionQuery { left: operand("contract-left", binding.left_row_transform_f64), right: operand("contract-right", binding.right_row_transform_f64),
        clearance_threshold_mm: 0.5, requested_accuracy_mm: None, qualification_limits: Default::default() });
    request.collision_meshes = Some(CadCollisionMeshPair { left_positions_mm_f64: asset.surface_positions_mm_f64.clone(), right_positions_mm_f64: asset.surface_positions_mm_f64.clone(),
        left_triangle_face_occurrence_indices: asset.triangle_face_occurrence_indices.clone(), right_triangle_face_occurrence_indices: asset.triangle_face_occurrence_indices.clone() });
    request
}

fn control(module: &JsValue, key: &str, value: JsValue) {
    Reflect::set(&Reflect::get(module, &"controls".into()).unwrap(), &key.into(), &value).unwrap();
}
fn count(module: &JsValue, key: &str) -> f64 {
    Reflect::get(&Reflect::get(module, &"controls".into()).unwrap(), &key.into()).unwrap().as_f64().unwrap()
}

#[wasm_bindgen_test]
async fn owned_provider_handle_survives_original_buffer_transfer_and_qualifies_only_matching_requests() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let (_, result) = run_request(&queue, &module, preparation("asset"), vec![]).await.unwrap_or_else(|(_,e,_)| panic!("{e}"));
    let analysis = Reflect::get(&result, &"generatedAnalysis".into()).unwrap();
    assert!(!Reflect::has(&analysis, &"analysisHandle".into()).unwrap());
    let positions = Reflect::get(&analysis, &"surfacePositionsMmF64".into()).unwrap().unchecked_into::<Float64Array>();
    let transfers = result_transfers(&CadOperation::AnalyzeGeneratedBody, &result).unwrap();
    assert!(Object::is(&transfers.get(0), &positions.buffer()));
    let transferred = Function::new_with_args("result,transfer", "return structuredClone(result,{transfer:Array.from(transfer)})")
        .call2(&JsValue::NULL, &result, &transfers).unwrap();
    assert_eq!(positions.byte_length(), 0);
    let asset = decode_result(transferred).unwrap().generated_analysis.unwrap();
    let request = query(&asset);
    let (_, result) = run_request(&queue, &module, request.clone(), vec![]).await.unwrap_or_else(|(_,e,_)| panic!("{e}"));
    let report = decode_result(result).unwrap().query_qualification.unwrap();
    assert!(matches!(report.mesh_distance, qualification::MeshDistance::Bounded { witness_triangles: [2,0], .. }));
    assert_eq!(report.binding.snapshot, qualification::Snapshot::from(&request.identity));
    assert_eq!(report.binding.worker_generation, queue.borrow().generation);
    assert_eq!(count(&module, "queries"), 1.);
    for mutation in 0..5 {
        let mut bad = request.clone();
        match mutation {
            0 => bad.collision_meshes.as_mut().unwrap().left_positions_mm_f64[0] = -0.0,
            1 => bad.collision_meshes.as_mut().unwrap().left_triangle_face_occurrence_indices[0] += 1,
            2 => bad.identity.board_id.push('x'),
            3 => bad.collision_query.as_mut().unwrap().left.analysis_mesh.as_mut().unwrap().settings.angular_rad = 0.25,
            _ => bad.collision_query.as_mut().unwrap().left.generated_geometry_identity.as_mut().unwrap().push('x'),
        }
        assert!(run_request(&queue, &module, bad, vec![]).await.is_err(), "mutation {mutation}");
    }
    control(&module, "badQualifier", true.into());
    assert!(run_request(&queue, &module, request.clone(), vec![]).await.is_err());
    control(&module, "badQualifier", false.into());
    assert!(run_request(&queue, &module, request.clone(), vec![]).await.is_ok(), "same-worker recovery");
    let mut capped = request;
    capped.collision_query.as_mut().unwrap().qualification_limits.max_triangles = 1;
    let (_, result) = run_request(&queue, &module, capped, vec![]).await.unwrap_or_else(|(_,e,_)| panic!("{e}"));
    assert!(matches!(decode_result(result).unwrap().query_qualification.unwrap().mesh_distance, qualification::MeshDistance::Unknown { .. }));
    drop(queue);
    assert_eq!(count(&module, "frees"), 1.);
}

#[wasm_bindgen_test]
async fn evidence_eviction_cost_refusal_cancel_stale_and_restart_never_resurrect_page_reports() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let old = prepare(&queue, &module, "old").await;
    let old_request = query(&old);
    for index in 0..80 { prepare(&queue, &module, &format!("fill-{index}")).await; }
    assert_eq!(queue.borrow().evidence.len(), 80);
    assert_eq!(count(&module, "frees"), 1.);
    let (_, fallback) = run_request(&queue, &module, old_request.clone(), vec![]).await.unwrap_or_else(|(_,e,_)| panic!("{e}"));
    let fallback = decode_result(fallback).unwrap();
    assert!(matches!(fallback.query_qualification.unwrap().mesh_distance, qualification::MeshDistance::Unknown { reasons } if reasons == ["evidence-unavailable"]));
    let restarted = Rc::new(RefCell::new(WorkerQueue::default()));
    assert_ne!(queue.borrow().generation, restarted.borrow().generation);
    assert!(run_request(&restarted, &module, old_request, vec![]).await.is_ok());
    let before = count(&module, "frees");
    control(&module, "cost", JsValue::NULL);
    prepare(&restarted, &module, "uncountable").await;
    control(&module, "cost", (MAX_COLLISION_QUERY_BYTES as f64 + 1.).into());
    prepare(&restarted, &module, "oversize").await;
    assert!(restarted.borrow().evidence.is_empty());
    assert_eq!(count(&module, "frees"), before + 2.);
    control(&module, "cost", 1024.into());
    let cancel_queue = restarted.clone();
    let cancel = Closure::<dyn FnMut(JsValue)>::once(move |_| { cancel_queue.borrow_mut().cancelled.insert("cancel".into()); });
    let promise = Promise::resolve(&JsValue::UNDEFINED);
    let _ = promise.then(&cancel);
    assert!(matches!(run_request(&restarted, &module, preparation("cancel"), vec![]).await, Err((_,_,true))));
    assert!(restarted.borrow().evidence.is_empty());
    restarted.borrow_mut().cancelled.clear();
    let mut latest = preparation("latest").identity;
    latest.token += 1;
    restarted.borrow_mut().latest_snapshot = Some(latest);
    assert!(run_request(&restarted, &module, preparation("stale"), vec![]).await.is_err());
    assert!(restarted.borrow().evidence.is_empty());
    restarted.borrow_mut().latest_snapshot = None;
    prepare(&restarted, &module, "recovery").await;
    assert_eq!(restarted.borrow().evidence.len(), 1);
}

#[wasm_bindgen_test]
fn strict_js_metadata_rejects_unknown_keys_and_detached_or_shared_analysis_buffers() {
    let fixture = fixture();
    let valid = &fixture["qualificationV2"]["activation"]["generatedPlaneQuery"];
    let parsed = js_sys::JSON::parse(&valid.to_string()).unwrap();
    strict_metadata::<qualification::QueryQualification>(parsed.clone()).unwrap();
    for (object, key) in [("binding", "callerCertificate"), ("limits", "untrackedCap"), ("containment", "sourceHull")] {
        let target = Reflect::get(&parsed, &object.into()).unwrap();
        Reflect::set(&target, &key.into(), &true.into()).unwrap();
        assert!(strict_metadata::<qualification::QueryQualification>(parsed.clone()).is_err());
        Reflect::delete_property(target.unchecked_ref::<Object>(), &key.into()).unwrap();
    }
    let tags = Uint32Array::from(&[0][..]);
    let positions = Float64Array::from(&[0.; 9][..]);
    validate_analysis_buffers(positions.as_ref(), tags.as_ref()).unwrap();
    let detach = Function::new_with_args("p", "structuredClone(p,{transfer:[p.buffer]})");
    detach.call1(&JsValue::NULL, &positions).unwrap();
    assert!(validate_analysis_buffers(positions.as_ref(), tags.as_ref()).is_err());
    let shared = Function::new_no_args("return typeof SharedArrayBuffer === 'function' ? new Float64Array(new SharedArrayBuffer(72)) : null").call0(&JsValue::NULL).unwrap();
    if !shared.is_null() { assert!(validate_analysis_buffers(&shared, tags.as_ref()).is_err()); }
}
