use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn generated_analysis_shared_contract_decodes_and_rejects_mutated_identity_provenance_and_buffers()
{
    use crate::cad_jobs::*;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap();
    let body: boardstudio_model::model::PreparedCaseIR =
        serde_json::from_value(fixture["preparedBody"].clone()).unwrap();
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
        prepared: Some(boardstudio_model::model::PreparedCaseAssemblyIR {
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
    let body: boardstudio_model::model::PreparedCaseIR = serde_json::from_value(serde_json::json!({
        "revision":7,"body":{"id":"plate","name":"Plate","boardId":"board","kind":"plate","thickness":2,"clearance":0},"regions":[]
    })).unwrap();
    let metadata = crate::cad_jobs::CadGeneratedAnalysisRequest {
        body_id: "plate".into(),
        geometry_identity: crate::cad_jobs::generated_geometry_identity(&body).unwrap(),
        target_deflection_mm: Some(0.01),
    };
    let mut request = WireRequest {
        header: CadEnvelope {
            request_id: "analysis-1".into(),
            job_id: "analysis-1".into(),
            identity: CadSnapshotIdentity {
                token: 3,
                session_epoch: 1,
                document_id: "doc".into(),
                board_id: "board".into(),
                instance_id: None,
                revision: 7,
            },
            operation: CadOperation::AnalyzeGeneratedBody,
        },
        prepared: Some(boardstudio_model::model::PreparedCaseAssemblyIR {
            revision: 7,
            bodies: vec![body],
        }),
        generated_analysis_request: Some(metadata.clone()),
        source_sha256: None,
        step_import_options: None,
        collision_query: None,
        collision_meshes: None,
        keycaps: None,
        input_bytes: vec![],
        packaged_step: false,
    };
    let queue = Rc::new(RefCell::new(WorkerQueue::default()));
    let module = super::qualification_tests::controlled_provider(true);
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
    let module = Rc::new(super::qualification_tests::controlled_provider(true));
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
        "header": {
            "requestId": "raw-preview-missing",
            "jobId": "job-missing",
            "identity": identity,
            "operation": "preview",
        },
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

    let body: boardstudio_model::model::PreparedCaseIR = serde_json::from_value(
        serde_json::json!({
            "revision": 7,
            "body": {"id": "plate", "name": "Plate", "boardId": "board-a", "kind": "plate", "thickness": 2, "clearance": 0},
            "regions": []
        }),
    )
    .unwrap();
    let case_frame = serde_json::json!({
        "header": {
            "requestId": "raw-preview-case",
            "jobId": "job-case",
            "identity": identity,
            "operation": "preview",
        },
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

    let keycap = boardstudio_model::model::KeycapSpec {
        id: "k1".into(),
        reference: "A1".into(),
        profile: boardstudio_model::model::KeycapProfile::Cherry,
        mount: boardstudio_model::model::KeycapMount::Mx,
        row: 1,
        size: boardstudio_model::model::Vec2 { x: 18.0, y: 18.0 },
        top_size: boardstudio_model::model::Vec2 { x: 12.0, y: 12.0 },
        height: 8.0,
        tilt: 0.0,
        dish_depth: 0.0,
        spherical: false,
        wall_thickness: 1.2,
        pose: boardstudio_model::model::Pose2 {
            at: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: boardstudio_model::model::Side::Front,
        z: 8.4,
        travel: 4.0,
        legend: String::new(),
        color: "#b0b5bd".into(),
        legend_color: "#202124".into(),
    };
    let keycap_frame = serde_json::json!({
        "header": {
            "requestId": "raw-preview-keycaps",
            "jobId": "job-keycaps",
            "identity": identity,
            "operation": "preview",
        },
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
