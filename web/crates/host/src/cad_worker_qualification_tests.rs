//! Controlled provider ownership/transport tests. This double authenticates no geometry.
use super::*;
use crate::cad_jobs::*;
use wasm_bindgen_test::wasm_bindgen_test;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap()
}

pub(super) fn controlled_provider(unqualified: bool) -> JsValue {
    let factory = Function::new_with_args(
        "fixture,unqualified",
        r#"
        const a = fixture.qualificationV2.activation;
        const controls = {cost:1024,prepares:0,vendorReads:0,queries:0,raw:0,frees:0,badQualifier:false,unqualified};
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
            read_step_model(bytes,options) {
                controls.vendorReads++;
                const p = structuredClone(fixture.directPayload);
                const mesh = structuredClone(a.descriptors.vendor);
                mesh.meshIdentity = 'final-f64-v1/controlled-vendor-'+controls.vendorReads;
                mesh.settings.targetDeflectionMm = options.targetDeflectionMm ?? null;
                mesh.settings.previewRepairBudgetMm = options.previewRepairBudgetMm ?? null;
                mesh.settings.selectedDeflectionMm = options.targetDeflectionMm ?? 0.1;
                const positions = new Float64Array(p.surfacePositionsMmF64), tags = new Uint32Array(p.triangleFaceOccurrenceIndices);
                const accuracy = {previewDeflectionMm:mesh.settings.selectedDeflectionMm,previewAngularRad:0.5,analysisErrorBoundMm:null,
                    selectedRepairBudgetMm:options.previewRepairBudgetMm??null,maxRepairDisplacementMm:null,boundsFromSourceGeometry:true,
                    meshClosedOriented:true,previewFloorMm:0.02,previewDeclaredMultiplier:2,relaxedBeyondDeclaredPrecision:false,
                    sampledSupportChecks:0,maxSampledSupportResidualMm:null,fallbackMeshedFaces:0};
                const vendorStep = {sourceSha256:options.sourceSha256,importOptions:{previewRepairBudgetMm:options.previewRepairBudgetMm??null,targetDeflectionMm:options.targetDeflectionMm??null},
                    sourceFaces:p.sourceFaces.map((face,i)=>({solidId:'10',shellIndex:face.shellIndex,faceId:String(20+i),supportId:String(30+i),supportKind:'PLANE'})),
                    faceOccurrences:p.faceOccurrences.map(face=>({...face,representationId:'40',sourceUncertaintyMm:0.01,repairDisplacementMm:null})),
                    precision:[{representationId:'40',contextId:'50',geometricDimension:3,declarations:[{entityId:'60',lengthUnitId:'70',millimetres:0.01}]}],
                    accuracy,surfacePositionsMmF64:positions,triangleFaceOccurrenceIndices:tags,lineSegmentsMmF64:new Float64Array(),lineRenderPositions:new Float32Array(),lineOccurrences:[],segmentLineOccurrenceIndices:new Uint32Array(),
                    analysisMesh:mesh,sourceQualification:unknown('controlled-vendor-source-unavailable'),analysisHandle:new AnalysisHandle(positions,tags)};
                return {mesh:{positions:new Float32Array(positions),normals:new Float32Array(positions.length).fill(1)},bounds:{min:[0,0,0],max:[20,10,2]},vendorStep};
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
                const trianglesRefused = buffers.leftTags.length > q.qualificationLimits.maxTriangles || buffers.rightTags.length > q.qualificationLimits.maxTriangles;
                const domainRefused = [[buffers.leftPositions,q.left.rowTransformF64],[buffers.rightPositions,q.right.rowTransformF64]].some(([p,m])=>{
                    for(let i=0;i<p.length;i+=3) for(let axis=0;axis<3;axis++) {
                        const x=m[axis]*p[i]+m[axis+4]*p[i+1]+m[axis+8]*p[i+2]+m[axis+12];
                        if(x!==0 && (Math.abs(x)<2**q.qualificationLimits.minimumExponent || Math.abs(x)>2**q.qualificationLimits.maximumExponent)) return true;
                    }
                    return false;
                });
                if (controls.unqualified || q.left.analysisRef.source.kind==='vendor-step' || q.right.analysisRef.source.kind==='vendor-step' || trianglesRefused || domainRefused) {
                    const reason = trianglesRefused?'triangle cap':domainRefused?'exponent cap':'controlled-source-unavailable';
                    report.meshDistance=unknown(reason); report.sourceBounds=[unknown(reason),unknown(reason)];
                    report.contactRegion=unknown(reason); report.containment=unknown(reason);
                }
                if (controls.badQualifier) report.binding.snapshot.token='999';
                return {observation:structuredClone(a.rawObservationExample),queryQualification:report};
            },
            query_collision_mesh_pair(lp,lt,rp,rt,q) {
                if ('qualificationLimits' in q || 'analysisRef' in q.left || 'analysisMesh' in q.right) throw new Error('raw metadata must retain its v1 key set');
                controls.raw++; return structuredClone(a.rawObservationExample);
            }
        };
    "#,
    );
    factory
        .call2(
            &JsValue::NULL,
            &js_sys::JSON::parse(&fixture().to_string()).unwrap(),
            &unqualified.into(),
        )
        .unwrap()
}

fn preparation(id: &str) -> WireRequest {
    let body: boardstudio_core::model::PreparedCaseIR =
        serde_json::from_value(fixture()["preparedBody"].clone()).unwrap();
    WireRequest {
        request_id: id.into(),
        job_id: id.into(),
        identity: CadSnapshotIdentity {
            token: 11,
            session_epoch: 2,
            document_id: "contract-document".into(),
            board_id: "contract-board".into(),
            instance_id: None,
            revision: body.revision,
        },
        operation: CadOperation::AnalyzeGeneratedBody,
        generated_analysis_request: Some(CadGeneratedAnalysisRequest {
            body_id: body.body.id.clone(),
            geometry_identity: generated_geometry_identity(&body).unwrap(),
            target_deflection_mm: None,
        }),
        prepared: Some(boardstudio_core::model::PreparedCaseAssemblyIR {
            revision: body.revision,
            bodies: vec![body],
        }),
        source_sha256: None,
        step_import_options: None,
        collision_query: None,
        collision_meshes: None,
        input_bytes: vec![],
        keycaps: None,
    }
}

async fn prepare(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    id: &str,
) -> CadGeneratedAnalysis {
    let (_, result) = run_request(queue, module, preparation(id), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    decode_result(result).unwrap().generated_analysis.unwrap()
}

fn query(asset: &CadGeneratedAnalysis) -> WireRequest {
    let mut request = preparation("query");
    request.operation = CadOperation::CollisionQuery;
    request.prepared = None;
    request.generated_analysis_request = None;
    let binding: qualification::Binding = serde_json::from_value(
        fixture()["qualificationV2"]["activation"]["generatedPlaneQuery"]["binding"].clone(),
    )
    .unwrap();
    let operand = |model: &str, row| CadCollisionOperand {
        model_id: model.into(),
        source_sha256: None,
        generated_geometry_identity: Some(asset.geometry_identity.clone()),
        import_options: Default::default(),
        row_transform_f64: row,
        analysis_ref: asset.analysis_ref.clone(),
        analysis_mesh: asset.analysis_mesh.clone(),
    };
    request.collision_query = Some(CadCollisionQuery {
        left: operand("contract-left", binding.left_row_transform_f64),
        right: operand("contract-right", binding.right_row_transform_f64),
        clearance_threshold_mm: 0.5,
        requested_accuracy_mm: None,
        qualification_limits: Default::default(),
    });
    request.collision_meshes = Some(CadCollisionMeshPair {
        left_positions_mm_f64: asset.surface_positions_mm_f64.clone(),
        right_positions_mm_f64: asset.surface_positions_mm_f64.clone(),
        left_triangle_face_occurrence_indices: asset.triangle_face_occurrence_indices.clone(),
        right_triangle_face_occurrence_indices: asset.triangle_face_occurrence_indices.clone(),
    });
    request
}

fn control(module: &JsValue, key: &str, value: JsValue) {
    Reflect::set(
        &Reflect::get(module, &"controls".into()).unwrap(),
        &key.into(),
        &value,
    )
    .unwrap();
}
fn count(module: &JsValue, key: &str) -> f64 {
    Reflect::get(
        &Reflect::get(module, &"controls".into()).unwrap(),
        &key.into(),
    )
    .unwrap()
    .as_f64()
    .unwrap()
}

#[wasm_bindgen_test]
async fn owned_provider_handle_survives_original_buffer_transfer_and_qualifies_only_matching_requests()
 {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let (_, result) = run_request(&queue, &module, preparation("asset"), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let analysis = Reflect::get(&result, &"generatedAnalysis".into()).unwrap();
    assert!(!Reflect::has(&analysis, &"analysisHandle".into()).unwrap());
    let positions = Reflect::get(&analysis, &"surfacePositionsMmF64".into())
        .unwrap()
        .unchecked_into::<Float64Array>();
    let transfers = result_transfers(&CadOperation::AnalyzeGeneratedBody, &result).unwrap();
    assert!(Object::is(&transfers.get(0), &positions.buffer()));
    let transferred = Function::new_with_args(
        "result,transfer",
        "return structuredClone(result,{transfer:Array.from(transfer)})",
    )
    .call2(&JsValue::NULL, &result, &transfers)
    .unwrap();
    assert_eq!(positions.byte_length(), 0);
    let asset = decode_result(transferred)
        .unwrap()
        .generated_analysis
        .unwrap();
    let request = query(&asset);
    let (_, result) = run_request(&queue, &module, request.clone(), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let report = decode_result(result).unwrap().query_qualification.unwrap();
    assert!(matches!(
        report.mesh_distance,
        qualification::MeshDistance::Bounded {
            witness_triangles: [2, 0],
            ..
        }
    ));
    assert_eq!(
        report.binding.snapshot,
        qualification::Snapshot::from(&request.identity)
    );
    assert_eq!(report.binding.worker_generation, queue.borrow().generation);
    assert_eq!(count(&module, "queries"), 1.);
    for mutation in 0..5 {
        let mut bad = request.clone();
        match mutation {
            0 => bad.collision_meshes.as_mut().unwrap().left_positions_mm_f64[0] = -0.0,
            1 => {
                bad.collision_meshes
                    .as_mut()
                    .unwrap()
                    .left_triangle_face_occurrence_indices[0] += 1
            }
            2 => bad.identity.board_id.push('x'),
            3 => {
                bad.collision_query
                    .as_mut()
                    .unwrap()
                    .left
                    .analysis_mesh
                    .as_mut()
                    .unwrap()
                    .settings
                    .angular_rad = 0.25
            }
            _ => bad
                .collision_query
                .as_mut()
                .unwrap()
                .left
                .generated_geometry_identity
                .as_mut()
                .unwrap()
                .push('x'),
        }
        assert!(
            run_request(&queue, &module, bad, vec![]).await.is_err(),
            "mutation {mutation}"
        );
    }
    control(&module, "badQualifier", true.into());
    assert!(
        run_request(&queue, &module, request.clone(), vec![])
            .await
            .is_err()
    );
    control(&module, "badQualifier", false.into());
    assert!(
        run_request(&queue, &module, request.clone(), vec![])
            .await
            .is_ok(),
        "same-worker recovery"
    );
    for exponent in [false, true] {
        let mut capped = request.clone();
        let limits = &mut capped
            .collision_query
            .as_mut()
            .unwrap()
            .qualification_limits;
        if exponent {
            limits.maximum_exponent = 0;
        } else {
            limits.max_triangles = 1;
        }
        let (_, result) = run_request(&queue, &module, capped, vec![])
            .await
            .unwrap_or_else(|(_, e, _)| panic!("{e}"));
        let result = decode_result(result).unwrap();
        assert!(
            result.collision.is_some(),
            "preflight preserves the raw observation"
        );
        let report = result.query_qualification.unwrap();
        assert!(
            matches!(report.mesh_distance, qualification::MeshDistance::Unknown { reasons } if reasons.iter().all(|r| r.contains("cap")))
        );
        assert!(report.source_bounds.iter().all(|bound| matches!(bound, qualification::WorldSourceBound::Unknown { reasons } if reasons.iter().all(|r| r.contains("cap")))));
        assert!(matches!(
            report.contact_region,
            qualification::ContactRegion::Unknown { .. }
        ));
        assert!(matches!(
            report.containment,
            qualification::Containment::Unknown { .. }
        ));
    }
    drop(queue);
    assert_eq!(count(&module, "frees"), 1.);
}

#[wasm_bindgen_test]
async fn evidence_eviction_cost_refusal_cancel_stale_and_restart_never_resurrect_page_reports() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let old = prepare(&queue, &module, "old").await;
    let old_request = query(&old);
    for index in 0..80 {
        prepare(&queue, &module, &format!("fill-{index}")).await;
    }
    assert_eq!(queue.borrow().evidence.len(), 80);
    assert_eq!(count(&module, "frees"), 1.);
    let (_, fallback) = run_request(&queue, &module, old_request.clone(), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let fallback = decode_result(fallback).unwrap();
    assert!(
        matches!(fallback.query_qualification.unwrap().mesh_distance, qualification::MeshDistance::Unknown { reasons } if reasons == ["evidence-unavailable"])
    );
    let restarted = Rc::new(RefCell::new(WorkerQueue::default()));
    assert_ne!(queue.borrow().generation, restarted.borrow().generation);
    assert!(
        run_request(&restarted, &module, old_request, vec![])
            .await
            .is_ok()
    );
    let before = count(&module, "frees");
    control(&module, "cost", JsValue::NULL);
    prepare(&restarted, &module, "uncountable").await;
    control(
        &module,
        "cost",
        (MAX_COLLISION_QUERY_BYTES as f64 + 1.).into(),
    );
    prepare(&restarted, &module, "oversize").await;
    assert!(restarted.borrow().evidence.is_empty());
    assert_eq!(count(&module, "frees"), before + 2.);
    control(&module, "cost", 1024.into());
    let cancel_queue = restarted.clone();
    let cancel = Closure::<dyn FnMut(JsValue)>::once(move |_| {
        cancel_queue.borrow_mut().cancelled.insert("cancel".into());
    });
    let promise = Promise::resolve(&JsValue::UNDEFINED);
    let _ = promise.then(&cancel);
    assert!(matches!(
        run_request(&restarted, &module, preparation("cancel"), vec![]).await,
        Err((_, _, true))
    ));
    assert!(restarted.borrow().evidence.is_empty());
    restarted.borrow_mut().cancelled.clear();
    let mut latest = preparation("latest").identity;
    latest.token += 1;
    restarted.borrow_mut().latest_snapshot = Some(latest);
    assert!(
        run_request(&restarted, &module, preparation("stale"), vec![])
            .await
            .is_err()
    );
    assert!(restarted.borrow().evidence.is_empty());
    restarted.borrow_mut().latest_snapshot = None;
    prepare(&restarted, &module, "recovery").await;
    assert_eq!(restarted.borrow().evidence.len(), 1);
}

#[wasm_bindgen_test]
fn strict_js_metadata_rejects_unknown_keys_and_detached_or_shared_analysis_buffers() {
    let accuracy =
        js_sys::JSON::parse(&serde_json::to_string(&CadStepAccuracy::default()).unwrap()).unwrap();
    strict_step_accuracy(accuracy.clone()).unwrap();
    Reflect::delete_property(
        accuracy.unchecked_ref::<Object>(),
        &"selectedRepairBudgetMm".into(),
    )
    .unwrap();
    strict_step_accuracy(accuracy.clone()).unwrap();
    Reflect::set(&accuracy, &"callerCertificate".into(), &true.into()).unwrap();
    assert!(strict_step_accuracy(accuracy).is_err());
    let fixture = fixture();
    let valid = &fixture["qualificationV2"]["activation"]["generatedPlaneQuery"];
    let parsed = js_sys::JSON::parse(&valid.to_string()).unwrap();
    strict_metadata::<qualification::QueryQualification>(parsed.clone()).unwrap();
    for (object, key) in [
        ("binding", "callerCertificate"),
        ("limits", "untrackedCap"),
        ("containment", "sourceHull"),
    ] {
        let target = Reflect::get(&parsed, &object.into()).unwrap();
        Reflect::set(&target, &key.into(), &true.into()).unwrap();
        assert!(strict_metadata::<qualification::QueryQualification>(parsed.clone()).is_err());
        Reflect::delete_property(target.unchecked_ref::<Object>(), &key.into()).unwrap();
    }
    let distance = Reflect::get(&parsed, &"meshDistance".into()).unwrap();
    Reflect::set(&distance, &"upperMm".into(), &f64::NAN.into()).unwrap();
    assert!(
        strict_metadata::<qualification::QueryQualification>(parsed).is_err(),
        "materialized $nonFinite mutation"
    );
    let tags = Uint32Array::from(&[0][..]);
    let positions = Float64Array::from(&[0.; 9][..]);
    validate_analysis_buffers(positions.as_ref(), tags.as_ref()).unwrap();
    let detach = Function::new_with_args("p", "structuredClone(p,{transfer:[p.buffer]})");
    detach.call1(&JsValue::NULL, &positions).unwrap();
    assert!(validate_analysis_buffers(positions.as_ref(), tags.as_ref()).is_err());
    let shared = Function::new_no_args("return new Float64Array(new WebAssembly.Memory({initial:1,maximum:1,shared:true}).buffer,0,9)").call0(&JsValue::NULL).unwrap();
    assert!(validate_analysis_buffers(&shared, tags.as_ref()).is_err());
}

#[wasm_bindgen_test]
async fn worker_context_keeps_full_u64_snapshot_token_and_session_epoch() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let mut preparation = preparation("lossless");
    preparation.identity.token = u64::MAX;
    preparation.identity.session_epoch = u64::MAX - 1;
    let (_, result) = run_request(&queue, &module, preparation.clone(), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let asset = decode_result(result).unwrap().generated_analysis.unwrap();
    let mut request = query(&asset);
    request.identity = preparation.identity;
    let (_, result) = run_request(&queue, &module, request, vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let snapshot = decode_result(result)
        .unwrap()
        .query_qualification
        .unwrap()
        .binding
        .snapshot;
    assert_eq!(snapshot.token, u64::MAX.to_string());
    assert_eq!(snapshot.session_epoch, (u64::MAX - 1).to_string());
}

#[wasm_bindgen_test]
async fn post_failure_and_pinned_eviction_release_only_their_own_handles() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let (wire, result) = run_request(&queue, &module, preparation("failed-post"), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let newer = prepare(&queue, &module, "newer").await;
    let target = Object::new();
    Reflect::set(
        &target,
        &"postMessage".into(),
        &Function::new_no_args("throw new Error('controlled post failure')"),
    )
    .unwrap();
    let scope: DedicatedWorkerGlobalScope = target.unchecked_into();
    assert!(post_owned_result(&queue, &scope, wire, result).is_err());
    assert_eq!(queue.borrow().evidence.len(), 1);
    assert_eq!(count(&module, "frees"), 1.);
    let request = query(&newer);
    let operand = &request.collision_query.as_ref().unwrap().left;
    let meshes = request.collision_meshes.as_ref().unwrap();
    let pinned = queue
        .borrow_mut()
        .lookup(
            operand,
            &request.identity,
            &meshes.left_positions_mm_f64,
            &meshes.left_triangle_face_occurrence_indices,
        )
        .unwrap()
        .unwrap();
    queue
        .borrow_mut()
        .remove_evidence(&pinned.reference.mesh_identity);
    assert_eq!(
        count(&module, "frees"),
        1.,
        "an in-flight owner pins the opaque handle"
    );
    drop(pinned);
    assert_eq!(count(&module, "frees"), 2.);
    let restored = prepare(&queue, &module, "restored").await;
    let query = query(&restored);
    queue.borrow_mut().cancelled.insert(query.job_id.clone());
    assert!(run_request(&queue, &module, query, vec![]).await.is_err());
    assert_eq!(
        queue.borrow().evidence.len(),
        1,
        "query cancellation cannot revoke another job's asset"
    );
    queue.borrow_mut().cancel_job("restored");
    assert_eq!(queue.borrow().evidence.len(), 0);
    assert_eq!(
        count(&module, "frees"),
        3.,
        "late preparation cancellation releases its own handle"
    );
}

#[wasm_bindgen_test]
async fn authenticated_step_bytes_bind_owned_unknown_evidence_and_immutable_sources_can_cross_revisions()
 {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    let mut request = preparation("vendor");
    request.operation = CadOperation::ReadStep;
    request.prepared = None;
    request.generated_analysis_request = None;
    request.source_sha256 = Some("a".repeat(64));
    request.step_import_options = Some(CadStepImportOptions {
        preview_repair_budget_mm: Some(0.05),
        target_deflection_mm: Some(0.01),
    });
    assert!(
        run_request(&queue, &module, request.clone(), vec![1])
            .await
            .is_err()
    );
    assert_eq!(
        count(&module, "vendorReads"),
        0.,
        "hash verification precedes the provider call"
    );
    request.source_sha256 = Some(sha256_hex(&[1]));
    let (_, result) = run_request(&queue, &module, request.clone(), vec![1])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    let payload = Reflect::get(&result, &"vendorStep".into()).unwrap();
    let options = Reflect::get(&payload, &"importOptions".into()).unwrap();
    Reflect::set(&options, &"callerCertificate".into(), &true.into()).unwrap();
    assert!(decode_result(result.clone()).is_err());
    Reflect::delete_property(
        options.unchecked_ref::<Object>(),
        &"callerCertificate".into(),
    )
    .unwrap();
    let bad_lines = Function::new_no_args("return [{lineOccurrenceIndex:0,trimmedCurveId:'1',sourceLineId:'2',curveSetId:'3',representationId:'4',sourceUnitToMm:1,placementF64:new Float64Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]),declaredUncertaintyMm:null,callerCertificate:true}]")
        .call0(&JsValue::NULL).unwrap();
    Reflect::set(&payload, &"lineOccurrences".into(), &bad_lines).unwrap();
    assert!(decode_result(result.clone()).is_err());
    Reflect::set(&payload, &"lineOccurrences".into(), &Array::new()).unwrap();
    let asset = decode_result(result).unwrap().vendor_step.unwrap();
    assert_eq!(asset.source_sha256, sha256_hex(&[1]));
    assert_eq!(asset.import_options, request.step_import_options.unwrap());
    assert!(
        matches!(
            asset.source_qualification,
            Some(qualification::SourceQualification::Unknown { .. })
        ),
        "the double supplies no source authentication proof"
    );
    let reference = asset.analysis_ref.clone().unwrap();
    assert!(
        matches!(&reference.source, qualification::Source::VendorStep { source_sha256, .. } if source_sha256 == &sha256_hex(&[1]))
    );
    assert!(
        reference
            .derivation_identity
            .contains(":repair=3fa999999999999a:deflection=3f847ae147ae147b")
    );
    let make_operand = |id: &str, z: f64| {
        let mut row = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        row[14] = z;
        CadCollisionOperand {
            model_id: id.into(),
            source_sha256: Some(asset.source_sha256.clone()),
            generated_geometry_identity: None,
            import_options: asset.import_options,
            row_transform_f64: row,
            analysis_ref: Some(reference.clone()),
            analysis_mesh: asset.analysis_mesh.clone(),
        }
    };
    request.operation = CadOperation::CollisionQuery;
    request.source_sha256 = None;
    request.step_import_options = None;
    request.identity.revision += 1;
    request.identity.token += 1;
    request.collision_query = Some(CadCollisionQuery {
        left: make_operand("left", 0.),
        right: make_operand("right", 3.),
        clearance_threshold_mm: 0.5,
        requested_accuracy_mm: None,
        qualification_limits: Default::default(),
    });
    request.collision_meshes = Some(CadCollisionMeshPair {
        left_positions_mm_f64: asset.surface_positions_mm_f64.clone(),
        right_positions_mm_f64: asset.surface_positions_mm_f64,
        left_triangle_face_occurrence_indices: asset.triangle_face_occurrence_indices.clone(),
        right_triangle_face_occurrence_indices: asset.triangle_face_occurrence_indices,
    });
    let (_, result) = run_request(&queue, &module, request.clone(), vec![])
        .await
        .unwrap_or_else(|(_, e, _)| panic!("{e}"));
    assert!(decode_result(result).unwrap().collision.is_some());
    request.identity.board_id.push('x');
    assert!(
        run_request(&queue, &module, request, vec![]).await.is_err(),
        "a STEP lease cannot cross scope"
    );
}

#[wasm_bindgen_test]
async fn aggregate_provider_cost_evicts_without_freeing_an_inflight_pin() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = controlled_provider(false);
    control(&module, "cost", (32. * 1024. * 1024.).into());
    let asset = prepare(&queue, &module, "heavy-a").await;
    let request = query(&asset);
    let meshes = request.collision_meshes.as_ref().unwrap();
    let pinned = queue
        .borrow_mut()
        .lookup(
            &request.collision_query.as_ref().unwrap().left,
            &request.identity,
            &meshes.left_positions_mm_f64,
            &meshes.left_triangle_face_occurrence_indices,
        )
        .unwrap()
        .unwrap();
    prepare(&queue, &module, "heavy-b").await;
    assert_eq!(queue.borrow().evidence.len(), 1);
    assert!(queue.borrow().retained_bytes <= MAX_COLLISION_QUERY_BYTES);
    assert_eq!(count(&module, "frees"), 0.);
    drop(pinned);
    assert_eq!(count(&module, "frees"), 1.);
    drop(queue);
    assert_eq!(count(&module, "frees"), 2.);
}
