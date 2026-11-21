//! Complete-check collision tests and their shared pair oracle.
use super::*;

fn vendor_row_transform(offset_x: f64) -> [f64; 16] {
    let mut transform = IDENTITY_ROW_TRANSFORM_F64;
    transform[12] = offset_x;
    transform
}

fn collision_binding(finding: &crate::model_delivery::CadCollisionPairFinding) -> &Binding {
    &finding
        .result
        .query_qualification
        .as_ref()
        .expect("a settled finding carries its query qualification")
        .binding
}

fn pair_finding<'a>(
    findings: &'a [crate::model_delivery::CadCollisionPairFinding],
    left: &str,
    right: &str,
) -> &'a crate::model_delivery::CadCollisionPairFinding {
    findings
        .iter()
        .find(|finding| {
            (finding.left_id == left && finding.right_id == right)
                || (finding.left_id == right && finding.right_id == left)
        })
        .unwrap_or_else(|| panic!("missing pair {left} <-> {right}"))
}

/// The mechanical fixture has a fixed operand set: two repeated vendor placements,
/// the four generated Case bodies and the physical PCB reference. Every unordered
/// pair is reported exactly once.
fn assert_complete_unordered_pairs(findings: &[crate::model_delivery::CadCollisionPairFinding]) {
    let mut operands = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    for finding in findings {
        assert_ne!(finding.left_id, finding.right_id, "a subject is not a pair");
        operands.insert(finding.left_id.clone());
        operands.insert(finding.right_id.clone());
        let pair = if finding.left_id < finding.right_id {
            (finding.left_id.clone(), finding.right_id.clone())
        } else {
            (finding.right_id.clone(), finding.left_id.clone())
        };
        assert!(pairs.insert(pair), "an unordered pair was reported twice");
    }
    let expected = [
        "switch-1",
        "switch-2",
        "plate",
        "bottom",
        "plate-foam",
        "bottom-foam",
        "pcb-reference",
    ]
    .into_iter()
    .map(String::from)
    .collect::<BTreeSet<String>>();
    assert_eq!(
        operands, expected,
        "the report covers the exact mechanical operand set"
    );
    assert_eq!(
        findings.len(),
        21,
        "seven operands produce 21 unordered pairs"
    );
}

/// Publish one accepted native preview plus its delivered rows so the Runtime
/// input currency and readiness match a mounted Case view.
fn publish_collision_rows(
    runtime: &Rc<Runtime>,
    scene: &CadScene,
    rows: ModelDeliveryRows,
    models: Vec<PcbModel>,
) -> Rc<crate::case_preview::NativePreviewSnapshot> {
    let accepted = runtime
        .model()
        .accepted
        .clone()
        .expect("the fixture snapshot is accepted");
    let generation = runtime.native_case_preview.borrow().generation + 1;
    let core = runtime.core.borrow().clone();
    let capture = crate::case_preview::capture_native_preview(
        &accepted,
        &scene.scope,
        generation,
        generation,
        runtime.session.borrow().core_executor_epoch().0,
        core_executor_identity(&core),
        format!("collision-check-{generation}"),
    )
    .expect("valid accepted Case preview input");
    let owner = capture.owner.clone();
    let lease = capture.lease.clone();
    runtime.native_case_preview.borrow_mut().generation = generation;
    runtime
        .native_case_preview
        .borrow_mut()
        .begin(owner.clone(), lease);
    let preview = Rc::new(
        crate::case_preview::accept_native_preview(
            capture,
            PcbPreview {
                revision: accepted.document.revision,
                thickness: 1.6,
                contours: vec![],
                surfaces: vec![],
                holes: vec![],
                models,
                diagnostics: vec![],
            },
        )
        .expect("controlled payload preserves the accepted preview owner"),
    );
    runtime
        .native_case_preview
        .borrow_mut()
        .publish((*preview).clone())
        .expect("the controlled preview publishes");
    runtime.native_model_delivery.borrow_mut().published = Some((owner, rows.clone()));
    runtime.changed();
    preview
}

/// A real Runtime/CadWorker complete-check fixture: one mechanical Case scene with
/// generated subjects, one physical PCB and two placements of one vendor source.
async fn complete_check_fixture() -> (Rc<Runtime>, Rc<CadScene>, String, CollisionCheckInput) {
    let (runtime, scene, _) = fixture_document_scene(mechanical_document()).await;
    let bytes = b"controlled complete-check vendor source".to_vec();
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    runtime.assets.borrow_mut().insert(sha.clone(), bytes);
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap();
    let activation = contract["qualificationV2"]["activation"].clone();
    let source_faces = vec![CadStepSourceFace {
        solid_id: "10".into(),
        face_id: "20".into(),
        support_id: "30".into(),
        support_kind: "PLANE".into(),
        ..Default::default()
    }];
    let face_occurrences = vec![CadStepFaceOccurrence {
        representation_id: "40".into(),
        ..Default::default()
    }];
    let base = CadVendorStep {
        source_sha256: sha.clone(),
        surface_positions_mm_f64: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
        triangle_face_occurrence_indices: vec![0],
        source_faces: source_faces.clone(),
        face_occurrences: face_occurrences.clone(),
        ..Default::default()
    };
    // The delivered row source already carries a valid descriptor, so the default
    // check binds without any STEP import; the tight check replaces it.
    let mut row_mesh: AnalysisMesh =
        serde_json::from_value(activation["descriptors"]["vendor"].clone()).unwrap();
    row_mesh.mesh_identity = ROW_MESH_IDENTITY.into();
    let row_source = Source::VendorStep {
        source_sha256: sha.clone(),
        import_options: ImportOptions::from(CadStepImportOptions::default()),
    };
    let row_ref = AnalysisRef {
        worker_generation: WORKER_GENERATION.into(),
        mesh_identity: ROW_MESH_IDENTITY.into(),
        derivation_identity: derivation_identity(&row_source, &row_mesh),
        source: row_source,
    };
    let vendor_step = Rc::new(CadVendorStep {
        analysis_ref: Some(row_ref),
        analysis_mesh: Some(row_mesh),
        source_qualification: Some(SourceQualification::Unknown {
            reasons: vec!["controlled-vendor-source-unavailable".into()],
        }),
        source_sha256: sha.clone(),
        surface_positions_mm_f64: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
        triangle_face_occurrence_indices: vec![0],
        source_faces,
        face_occurrences,
        ..Default::default()
    });
    let source = COMPLETE_CHECK_WORKER
        .replace("__PAYLOAD__", &contract["directPayload"].to_string())
        .replace("__ACTIVATION__", &activation.to_string())
        .replace("__ASSET__", &serde_json::to_string(&base).unwrap());
    let properties = BlobPropertyBag::new();
    properties.set_type("text/javascript");
    let blob =
        Blob::new_with_str_sequence_and_options(&Array::of1(&source.into()), &properties).unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();
    let worker = Rc::new(CadWorker::new(&url).unwrap());
    worker.ready().await.unwrap();
    *runtime.cad_worker.borrow_mut() = Some((scene.scope.clone(), worker));
    let mesh = Rc::new(ValidatedMesh {
        positions: Rc::from([0.0_f32; 9]),
        normals: Rc::from([0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]),
        colors: None,
        step_bounds: None,
        vendor_step: Some(vendor_step),
    });
    let rows = ModelDeliveryRows {
        delivered: vec![
            DeliveredModel {
                id: "switch-1".into(),
                mesh: mesh.clone(),
                matrix: Some(vendor_row_transform(5.)),
            },
            DeliveredModel {
                id: "switch-2".into(),
                mesh,
                matrix: Some(vendor_row_transform(25.)),
            },
        ],
        pending: vec![],
        failures: vec![],
    };
    let preview_model = |id: &str, reference: &str| PcbModel {
        id: id.into(),
        reference: reference.into(),
        path: "models/switch.step".into(),
        pose: Pose2 {
            at: Vec2 { x: 0., y: 0. },
            rotation: 0.,
        },
        side: Side::Front,
        offset: Vec3 {
            x: 0.,
            y: 0.,
            z: 0.,
        },
        rotation: Vec3 {
            x: 0.,
            y: 0.,
            z: 0.,
        },
        scale: Vec3 {
            x: 1.,
            y: 1.,
            z: 1.,
        },
    };
    let preview = publish_collision_rows(
        &runtime,
        &scene,
        rows.clone(),
        vec![
            preview_model("switch-1", "SW1"),
            preview_model("switch-2", "SW2"),
        ],
    );
    let input = CollisionCheckInput::new(scene.clone(), Some(preview), Some(rows));
    (runtime, scene, url, input)
}

#[wasm_bindgen_test]
async fn complete_check_shares_one_vendor_refinement_across_repeated_placements() {
    let (runtime, _scene, url, input) = complete_check_fixture().await;
    let default = runtime
        .check_collisions(input.clone(), 0.1, None, Rc::new(|| true))
        .await
        .expect("the default complete check publishes a report");
    assert!(default.errors.is_empty(), "{:?}", default.errors);
    assert_eq!(default.message, None);
    assert_complete_unordered_pairs(&default.findings);
    let default_pair = pair_finding(&default.findings, "switch-1", "switch-2");
    assert_eq!(default_pair.left_label, "SW1");
    assert_eq!(default_pair.right_label, "SW2");
    assert_eq!(
        collision_binding(default_pair).left_row_transform_f64,
        vendor_row_transform(5.)
    );
    assert_eq!(
        collision_binding(default_pair).right_row_transform_f64,
        vendor_row_transform(25.)
    );
    assert_eq!(
        collision_binding(default_pair).left_mesh_identity,
        ROW_MESH_IDENTITY,
        "the default check reuses the delivered source without refinement"
    );
    assert_eq!(
        collision_binding(default_pair).right_mesh_identity,
        ROW_MESH_IDENTITY
    );

    let tight = runtime
        .check_collisions(
            input,
            0.1,
            Some(TIGHT_COLLISION_ACCURACY_MM),
            Rc::new(|| true),
        )
        .await
        .expect("the tight complete check publishes a report");
    assert!(tight.errors.is_empty(), "{:?}", tight.errors);
    assert_eq!(tight.message, None);
    assert_complete_unordered_pairs(&tight.findings);
    let tight_pair = pair_finding(&tight.findings, "switch-1", "switch-2");
    assert_eq!(tight_pair.left_id, "switch-1");
    assert_eq!(tight_pair.right_id, "switch-2");
    assert_eq!(tight_pair.left_label, "SW1");
    assert_eq!(tight_pair.right_label, "SW2");
    assert_eq!(
        collision_binding(tight_pair).left_row_transform_f64,
        vendor_row_transform(5.)
    );
    assert_eq!(
        collision_binding(tight_pair).right_row_transform_f64,
        vendor_row_transform(25.)
    );
    assert_eq!(
        collision_binding(tight_pair).left_mesh_identity,
        REFINED_VENDOR_MESH_IDENTITY,
        "repeated source and settings share exactly one refinement"
    );
    assert_eq!(
        collision_binding(tight_pair).right_mesh_identity,
        REFINED_VENDOR_MESH_IDENTITY,
        "the second placement keeps its own occurrence on the shared refinement"
    );
    assert!(
        tight_pair.provenance_details()[0].starts_with("SW1 [switch-1]:"),
        "{}",
        tight_pair.provenance_details()[0]
    );
    assert!(
        tight_pair.provenance_details()[1].starts_with("SW2 [switch-2]:"),
        "{}",
        tight_pair.provenance_details()[1]
    );
    for finding in &tight.findings {
        let binding = collision_binding(finding);
        if finding.left_id == "switch-1" {
            assert_eq!(binding.left_row_transform_f64, vendor_row_transform(5.));
            assert_eq!(binding.left_mesh_identity, REFINED_VENDOR_MESH_IDENTITY);
        }
        if finding.right_id == "switch-2" {
            assert_eq!(binding.right_row_transform_f64, vendor_row_transform(25.));
            assert_eq!(binding.right_mesh_identity, REFINED_VENDOR_MESH_IDENTITY);
        }
    }
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn obsolete_false_view_entry_cannot_retire_an_active_complete_check() {
    let (runtime, _scene, url, input) = complete_check_fixture().await;
    let core = project_name_test_support::in_process_core(&runtime);
    let (entered, release) = core.gate_matching_reply(|request| {
        matches!(
            request,
            CoreRequest::PrepareCase { ir, .. }
                if ir.bodies.iter().any(|body| body.body.id == "pcb-reference")
        )
    });
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_input = input.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let report = task_runtime
            .check_collisions(task_input, 0.1, None, Rc::new(|| true))
            .await;
        let _ = done.send(report);
    });
    entered
        .await
        .expect("the active check reached physical PCB preparation");

    assert!(
        runtime
            .check_collisions(input, 0.1, None, Rc::new(|| false))
            .await
            .is_none(),
        "an obsolete view request publishes no report"
    );

    release.send(()).unwrap();
    let report = receive
        .await
        .unwrap()
        .expect("the newer active check survived the obsolete request");
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(report.message, None);
    assert_complete_unordered_pairs(&report.findings);
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}
