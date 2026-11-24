use boardstudio_core::model::{PreparedCaseAssemblyIR, PreparedCaseIR};
use boardstudio_web_host::cad_jobs::{CadOperation, CadReplyOutcome, CadSnapshotIdentity};
use boardstudio_web_host::cad_worker::test_hooks::{
    WireReply, WorkerQueue, decode_result, js_message, receive_worker_message, result_transfers,
    run_request, validate_generated_payload_fields, worker_message, yield_to_worker,
};
use js_sys::{Float64Array, Object, Reflect, Uint32Array};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::DedicatedWorkerGlobalScope;

#[wasm_bindgen_test]
fn generated_analysis_shared_contract_decodes_and_rejects_mutated_identity_provenance_and_buffers()
{
    use boardstudio_web_host::cad_jobs::*;
    let fixture = super::cad_worker_qualification_tests::fixture();
    let body: PreparedCaseIR = serde_json::from_value(fixture["preparedBody"].clone()).unwrap();
    let geometry_identity = generated_geometry_identity(&body).unwrap();
    assert!(
        geometry_identity.starts_with(
            fixture["qualificationV2"]["generatedIdentityPrefix"]
                .as_str()
                .unwrap()
        )
    );
    let metadata = CadGeneratedAnalysisRequest {
        body_id: body.body.id.clone(),
        geometry_identity: geometry_identity.clone(),
        target_deflection_mm: None,
    };
    let identity = CadSnapshotIdentity {
        token: 1,
        session_epoch: 2,
        document_id: "fixture".into(),
        board_id: body.body.board_id.clone(),
        instance_id: None,
        revision: body.revision,
    };
    let request = CadRequest {
        request_id: "fixture-analysis".into(),
        job_id: "fixture-analysis".into(),
        identity: identity.clone(),
        operation: CadOperation::AnalyzeGeneratedBody,
        prepared: Some(PreparedCaseAssemblyIR {
            revision: body.revision,
            bodies: vec![body],
        }),
        generated_analysis_request: Some(metadata),
        source_sha256: None,
        step_import_options: None,
        collision_query: None,
        collision_meshes: None,
        input_bytes: vec![],
    };
    let decode_with_target = |payload: &serde_json::Value,
                              hash: &str,
                              typed: bool,
                              target: Option<f64>|
     -> Result<CadResult, String> {
        let mut request = request.clone();
        request
            .generated_analysis_request
            .as_mut()
            .unwrap()
            .target_deflection_mm = target;
        let direct = js_sys::JSON::parse(&payload.to_string()).map_err(js_message)?;
        validate_generated_payload_fields(&direct, false)?;
        if typed {
            Reflect::set(
                &direct,
                &"surfacePositionsMmF64".into(),
                &Float64Array::new(
                    &Reflect::get(&direct, &"surfacePositionsMmF64".into()).map_err(js_message)?,
                ),
            )
            .map_err(js_message)?;
            Reflect::set(
                &direct,
                &"triangleFaceOccurrenceIndices".into(),
                &Uint32Array::new(
                    &Reflect::get(&direct, &"triangleFaceOccurrenceIndices".into())
                        .map_err(js_message)?,
                ),
            )
            .map_err(js_message)?;
        }
        Reflect::set(&direct, &"geometryIdentity".into(), &hash.into()).map_err(js_message)?;
        let wrapper = Object::new();
        Reflect::set(
            &wrapper,
            &"revision".into(),
            &(identity.revision as f64).into(),
        )
        .map_err(js_message)?;
        Reflect::set(&wrapper, &"generatedAnalysis".into(), &direct).map_err(js_message)?;
        let result = decode_result(wrapper.into())?;
        validate_reply(
            &request,
            CadReply {
                request_id: request.request_id.clone(),
                job_id: request.job_id.clone(),
                identity: identity.clone(),
                operation: request.operation,
                outcome: CadReplyOutcome::Completed,
                result: Some(result),
                error: None,
            },
            &identity,
        )
        .map_err(|error| format!("{error:?}"))
    };
    let decode = |payload: &serde_json::Value, hash: &str, typed: bool| {
        decode_with_target(payload, hash, typed, None)
    };
    let payload = &fixture["directPayload"];
    let mut accepted_bad_settings = Vec::new();
    for (target, returned) in [
        (None, 1.0),
        (Some(1.0), 0.5),
        (Some(0.01), 0.02),
        (Some(0.01), 0.005),
    ] {
        let mut malformed = payload.clone();
        malformed["deflectionMm"] = returned.into();
        if decode_with_target(&malformed, &geometry_identity, true, target).is_ok() {
            accepted_bad_settings.push((target, returned));
        }
    }
    assert!(
        accepted_bad_settings.is_empty(),
        "accepted wrong selected deflections: {accepted_bad_settings:?}"
    );
    for (target, selected) in [(None, 0.1), (Some(1.0), 0.1), (Some(0.01), 0.01)] {
        let mut valid = payload.clone();
        valid["deflectionMm"] = selected.into();
        let accepted = decode_with_target(&valid, &geometry_identity, true, target).unwrap();
        assert_eq!(
            accepted.generated_analysis.unwrap().analysis_error_bound_mm,
            None
        );
    }
    let result = decode(payload, &geometry_identity, true).unwrap();
    assert!(result.vendor_step.is_none() && result.mesh.is_none() && result.bodies.is_empty());
    let analysis = result.generated_analysis.unwrap();
    assert_eq!(
        analysis.analysis_error_bound_mm, None,
        "deflection and closed topology are not certificates"
    );
    assert_eq!(
        analysis.deflection_mm,
        payload["deflectionMm"].as_f64().unwrap()
    );
    for field in [
        "bodyId",
        "revision",
        "triangleFaceOccurrenceIndices",
        "faceOccurrences",
        "surfacePositionsMmF64",
    ] {
        let mut bad = payload.clone();
        match field {
            "bodyId" => bad[field] = "another-body".into(),
            "revision" => bad[field] = (identity.revision + 1).into(),
            "triangleFaceOccurrenceIndices" => bad[field][0] = u32::MAX.into(),
            "faceOccurrences" => bad[field][0]["sourceFaceIndex"] = u32::MAX.into(),
            _ => {
                bad[field].as_array_mut().unwrap().pop();
            }
        }
        assert!(
            decode(&bad, &geometry_identity, true).is_err(),
            "reject mutated {field}"
        );
    }
    assert!(decode(payload, "another-geometry", true).is_err());
    assert!(
        decode(payload, &geometry_identity, false).is_err(),
        "ordinary JSON buffers cannot cross the worker boundary"
    );
    let mut bad = payload.clone();
    bad["sourceSha256"] = "a".repeat(64).into();
    assert!(
        decode(&bad, &geometry_identity, true).is_err(),
        "generated output is separate from STEP identity"
    );
}

#[wasm_bindgen_test]
fn generated_analysis_transfers_the_original_typed_buffers() {
    let positions = Float64Array::from(&[0.0, 0.0, 0.0][..]);
    let tags = Uint32Array::from(&[0][..]);
    let analysis = Object::new();
    Reflect::set(&analysis, &"surfacePositionsMmF64".into(), &positions).unwrap();
    Reflect::set(&analysis, &"triangleFaceOccurrenceIndices".into(), &tags).unwrap();
    let result = Object::new();
    Reflect::set(&result, &"generatedAnalysis".into(), &analysis).unwrap();
    let transfers = result_transfers(&CadOperation::AnalyzeGeneratedBody, &result).unwrap();
    assert_eq!(transfers.length(), 2);
    assert!(
        Object::is(&transfers.get(0), &positions.buffer()),
        "transfer the actual result buffer"
    );
    assert!(
        Object::is(&transfers.get(1), &tags.buffer()),
        "transfer the actual provenance buffer"
    );
}

#[wasm_bindgen_test]
async fn generated_analysis_worker_selects_one_body_checks_identity_and_recovers_after_failure() {
    let body: PreparedCaseIR = serde_json::from_value(serde_json::json!({
        "revision":7,"body":{"id":"plate","name":"Plate","boardId":"board","kind":"plate","thickness":2,"clearance":0},"regions":[]
    })).unwrap();
    let mut request = super::cad_worker_qualification_tests::analysis_request(
        "analysis-1",
        3,
        "doc",
        "board",
        body,
        Some(0.01),
    );
    let metadata = request.generated_analysis_request.clone().unwrap();
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = super::cad_worker_qualification_tests::controlled_provider(true);
    let (_, result) = run_request(&queue, &module, request.clone(), vec![])
        .await
        .unwrap_or_else(|(_, error, _)| panic!("{error}"));
    let decoded = decode_result(result).unwrap();
    let analysis = decoded.generated_analysis.unwrap();
    assert_eq!(analysis.geometry_identity, metadata.geometry_identity);
    assert_eq!(analysis.deflection_mm, 0.01);
    assert_eq!(analysis.analysis_error_bound_mm, None);
    assert_eq!(analysis.source_faces[0].face_index, 0);
    request
        .generated_analysis_request
        .as_mut()
        .unwrap()
        .geometry_identity
        .push('x');
    let failed = run_request(&queue, &module, request.clone(), vec![]).await;
    assert!(matches!(failed, Err((_, reason, false)) if reason.contains("identity")));
    request.generated_analysis_request = Some(metadata);
    queue
        .borrow_mut()
        .cancelled
        .insert(request.header.job_id.clone());
    assert!(matches!(
        run_request(&queue, &module, request.clone(), vec![]).await,
        Err((_, _, true))
    ));
    queue.borrow_mut().cancelled.clear();
    assert!(
        run_request(&queue, &module, request, vec![]).await.is_ok(),
        "failed/cancelled work must not poison the worker"
    );
}

#[wasm_bindgen_test]
async fn raw_worker_preview_without_prepared_or_keycap_input_fails_cleanly_and_worker_recovers() {
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = Rc::new(super::cad_worker_qualification_tests::controlled_provider(
        true,
    ));
    let replies: Rc<RefCell<Vec<WireReply>>> = Rc::new(RefCell::new(Vec::new()));
    let captured = replies.clone();
    let post_message = Closure::<dyn FnMut(JsValue, JsValue)>::new(
        move |message: JsValue, _transfers: JsValue| {
            if Reflect::get(&message, &"kind".into())
                .ok()
                .and_then(|v| v.as_string())
                .as_deref()
                == Some("reply")
                && let Some(frame) = Reflect::get(&message, &"frame".into())
                    .ok()
                    .and_then(|v| v.as_string())
            {
                captured
                    .borrow_mut()
                    .push(serde_json::from_str::<WireReply>(&frame).unwrap());
            }
        },
    );
    let target = Object::new();
    Reflect::set(&target, &"postMessage".into(), post_message.as_ref()).unwrap();
    let scope: DedicatedWorkerGlobalScope = target.unchecked_into();

    let identity = CadSnapshotIdentity {
        token: 19,
        session_epoch: 4,
        document_id: "raw-preview-doc".into(),
        board_id: "board-a".into(),
        instance_id: Some("inst-1".into()),
        revision: 7,
    };
    let missing_frame = serde_json::json!({
        "requestId": "raw-preview-missing",
        "jobId": "job-missing",
        "identity": identity,
        "operation": "preview",
        "prepared": null
    })
    .to_string();
    receive_worker_message(
        &queue,
        &scope,
        &module,
        worker_message("request", &missing_frame),
    )
    .unwrap();
    yield_to_worker().await;
    yield_to_worker().await;

    assert!(
        !queue.borrow().active,
        "worker queue must settle after failure"
    );
    assert!(queue.borrow().pending.is_empty());
    assert_eq!(queue.borrow().active_request, None);
    assert_eq!(queue.borrow().active_job, None);
    assert_eq!(replies.borrow().len(), 1);
    {
        let failed = &replies.borrow()[0];
        assert_eq!(failed.header.request_id, "raw-preview-missing");
        assert_eq!(failed.header.job_id, "job-missing");
        assert_eq!(failed.header.identity, identity);
        assert_eq!(failed.header.operation, CadOperation::Preview);
        assert_eq!(failed.outcome, CadReplyOutcome::Failed);
        assert!(
            failed
                .error
                .as_deref()
                .is_some_and(|msg| msg.contains("prepared")),
            "expected descriptive missing-prepared error, got {:?}",
            failed.error
        );
    }

    let body: PreparedCaseIR = serde_json::from_value(
        serde_json::json!({
            "revision": 7,
            "body": {"id": "plate", "name": "Plate", "boardId": "board-a", "kind": "plate", "thickness": 2, "clearance": 0},
            "regions": []
        }),
    )
    .unwrap();
    let case_frame = serde_json::json!({
        "requestId": "raw-preview-case",
        "jobId": "job-case",
        "identity": identity,
        "operation": "preview",
        "prepared": { "revision": 7, "bodies": [body] }
    })
    .to_string();
    receive_worker_message(
        &queue,
        &scope,
        &module,
        worker_message("request", &case_frame),
    )
    .unwrap();
    for _ in 0..4 {
        yield_to_worker().await;
    }
    assert!(!queue.borrow().active);
    assert_eq!(replies.borrow().len(), 2);
    assert_eq!(replies.borrow()[1].header.request_id, "raw-preview-case");
    assert_eq!(replies.borrow()[1].outcome, CadReplyOutcome::Completed);

    let mut keycap = super::cad_worker_keycaps_tests::spec();
    keycap.id = "k1".into();
    let keycap_frame = serde_json::json!({
        "requestId": "raw-preview-keycaps",
        "jobId": "job-keycaps",
        "identity": identity,
        "operation": "preview",
        "keycaps": { "revision": 7, "specs": [keycap] }
    })
    .to_string();
    receive_worker_message(
        &queue,
        &scope,
        &module,
        worker_message("request", &keycap_frame),
    )
    .unwrap();
    for _ in 0..4 {
        yield_to_worker().await;
    }
    assert!(!queue.borrow().active);
    assert_eq!(replies.borrow().len(), 3);
    assert_eq!(replies.borrow()[2].header.request_id, "raw-preview-keycaps");
    assert_eq!(replies.borrow()[2].outcome, CadReplyOutcome::Completed);
}
