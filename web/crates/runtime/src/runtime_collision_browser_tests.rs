use super::*;
use crate::model_delivery::{
    CadCollisionAssessment, CadCollisionSubject, DeliveredModel, IDENTITY_ROW_TRANSFORM_F64,
    ModelDeliveryRows, TIGHT_COLLISION_ACCURACY_MM, ValidatedMesh,
};
use boardstudio_core::model::{PcbModel, Pose2, Side, Vec2, Vec3};
use boardstudio_web_host::cad_jobs::{
    CadStepFaceOccurrence, CadStepImportOptions, CadStepSourceFace, CadVendorStep,
};
use boardstudio_web_host::cad_qualification::{
    AnalysisMesh, AnalysisRef, Binding, ImportOptions, Source, SourceQualification,
    derivation_identity,
};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Blob, BlobPropertyBag, Url};

async fn fixture() -> (Rc<Runtime>, Rc<CadScene>, String) {
    fixture_document(in_process_support::board_document()).await
}

async fn fixture_document(document: ProjectDoc) -> (Rc<Runtime>, Rc<CadScene>, String) {
    fixture_document_mode(document, "unknown").await
}

async fn fixture_document_mode(
    document: ProjectDoc,
    mode: &str,
) -> (Rc<Runtime>, Rc<CadScene>, String) {
    let runtime = project_name_test_support::new_runtime();
    project_name_test_support::open_document(&runtime, document).await;
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
    let prepared = if snapshot.document.mechanical.is_some() {
        let core = runtime.core.borrow().clone();
        Some(
            prepare_captured_case(
                core.as_ref(),
                &runtime.electrical_preview_executor_epoch().to_string(),
                "collision-mechanical-fixture",
                &snapshot,
                &scope,
            )
            .await
            .unwrap(),
        )
    } else {
        None
    };
    let scene = Rc::new(CadScene {
        scope: scope.clone(),
        token: snapshot.token,
        snapshot,
        prepared: prepared
            .as_ref()
            .map(|prepared| prepared.prepared.clone())
            .unwrap_or(boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: body.revision,
                bodies: vec![body],
            }),
        result: CadResult::default(),
        physical_fingerprint: None,
        mechanical: prepared.and_then(|prepared| prepared.mechanical_assembly),
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
        data["directPayload"],
        data["qualificationV2"]["activation"],
        serde_json::to_string(mode).unwrap()
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

fn mechanical_document() -> ProjectDoc {
    let mut document = in_process_support::board_document();
    document.boards[0].outline_ids.push("outline".into());
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0., y: 0. },
        size: Vec2 { x: 40., y: 30. },
        radius: 0.,
        rotation: None,
        operation: Operation::Add,
    });
    document.mechanical = Some(serde_json::from_value(serde_json::json!({
        "boardId":"main-board","method":"printed","mount":"rigid", "plateThickness":1.5,
        "plateFoamThickness":0.5,"pcbThickness":1.6,"bottomFoamThickness":0.4,"batteryHeight":4.,
        "bottomThickness":3.,"plateToPcb":3.5,"wallThickness":2.,"clearance":0.3,
        "bottomStyle":"shell","middleFrame":false,"openingAllowance":0.25,"mounts":[],"closureMounts":[],"profiles":[]
    })).unwrap());
    document
}

#[wasm_bindgen_test]
async fn source_pcb_collision_preserves_curves_drills_placement_and_complete_cache_identity() {
    let bytes = br#"(kicad_pcb (general (thickness 1.6))
        (gr_circle (center 10 5) (end 20 5) (layer "Edge.Cuts") (tstamp outer))
        (gr_circle (center 10 5) (end 12 5) (layer "Edge.Cuts") (tstamp hole))
        (footprint "fixture" (layer "F.Cu") (at 15 5)
          (pad "1" thru_hole circle (at 0 0) (size 2 2) (drill 1) (layers "*.Cu" "*.Mask"))))"#;
    let sha256 = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut document = mechanical_document();
    document.assets.push(serde_json::from_value(serde_json::json!({
        "id":"pcb-source","name":"curved.kicad_pcb","mediaType":"application/vnd.kicad.pcb","sha256":sha256
    })).unwrap());
    document.board_references.push(
        serde_json::from_value(serde_json::json!({
            "id":"source-ref","boardId":"main-board","assetId":"pcb-source","enabled":true,
            "pose":{"at":{"x":100,"y":200},"rotation":90},"elevation":2.5,"modelAssets":{}
        }))
        .unwrap(),
    );
    let (runtime, scene, url) = fixture_document(document).await;
    runtime
        .assets
        .borrow_mut()
        .insert(sha256.clone(), bytes.to_vec());
    let original = scene.prepared.clone();
    let core = runtime.core.borrow().clone();
    let physical = runtime
        .prepare_physical_step_assembly(
            core.as_ref(),
            &runtime.electrical_preview_executor_epoch().to_string(),
            "collision-source-fixture",
            &scene.snapshot,
            &scene.scope,
            Rc::new(|| true),
        )
        .await
        .unwrap();
    let pcb = physical
        .prepared
        .prepared
        .bodies
        .iter()
        .find(|body| body.body.id == "pcb-reference")
        .unwrap();
    assert_eq!(pcb.body.thickness, 1.6);
    assert!(
        (pcb.body.z.unwrap() - 0.9).abs() < 1e-12,
        "PCB stack height plus reference elevation is applied once"
    );
    let source = pcb.body.pcb_source.as_ref().unwrap();
    assert_eq!(source.contours.len(), 2);
    let supports = source
        .contours
        .iter()
        .flat_map(|contour| &contour.primitives)
        .filter_map(|primitive| primitive.curve_support.as_ref())
        .collect::<Vec<_>>();
    assert!(
        supports
            .iter()
            .any(|support| (support.radius_mm - 10.).abs() < 1e-12)
    );
    assert!(
        supports
            .iter()
            .any(|support| (support.radius_mm - 2.).abs() < 1e-12)
    );
    assert!(
        supports
            .iter()
            .all(|support| (support.center.x - 105.).abs() < 1e-12
                && (support.center.y - 210.).abs() < 1e-12),
        "source placement/rotation and KiCad Y conversion occur once"
    );
    assert_eq!(pcb.body.mounts.as_ref().unwrap().len(), 1);
    assert!(pcb.regions.iter().any(|region| !region.holes.is_empty()));
    assert!(
        pcb.regions.iter().all(|region| region.mounts.is_empty()),
        "drills retain the exact-source contract instead of duplicate compatibility mounts"
    );
    let expected_identity =
        boardstudio_web_host::cad_jobs::generated_geometry_identity(pcb).unwrap();
    let (subjects, errors) = runtime
        .generated_collision_subjects(&scene, None, Rc::new(|| true))
        .await;
    assert!(errors.is_empty(), "{errors:?}");
    let pcb = subjects
        .iter()
        .find(|subject| subject.id() == "pcb-reference")
        .unwrap();
    let CadCollisionSubject::GeneratedCaseBody {
        analysis: Some(analysis),
        row_transform_f64,
        ..
    } = pcb
    else {
        panic!("source PCB analysis was not delivered")
    };
    assert_eq!(analysis.geometry_identity, expected_identity);
    assert_eq!(
        *row_transform_f64, IDENTITY_ROW_TRANSFORM_F64,
        "prepared physical geometry is not transformed again at query time"
    );
    let (again, errors) = runtime
        .generated_collision_subjects(&scene, None, Rc::new(|| true))
        .await;
    assert!(errors.is_empty(), "{errors:?}");
    let CadCollisionSubject::GeneratedCaseBody {
        analysis: Some(cached),
        ..
    } = again
        .iter()
        .find(|subject| subject.id() == "pcb-reference")
        .unwrap()
    else {
        panic!("cached PCB analysis missing")
    };
    assert!(Rc::ptr_eq(analysis, cached));
    let (tight, errors) = runtime
        .generated_collision_subjects(&scene, Some(0.01), Rc::new(|| true))
        .await;
    assert!(errors.is_empty(), "{errors:?}");
    let CadCollisionSubject::GeneratedCaseBody {
        analysis: Some(refined),
        ..
    } = tight
        .iter()
        .find(|subject| subject.id() == "pcb-reference")
        .unwrap()
    else {
        panic!("refined PCB analysis missing")
    };
    assert_eq!(refined.deflection_mm, 0.01);
    assert_eq!(refined.geometry_identity, analysis.geometry_identity);
    assert!(
        !Rc::ptr_eq(analysis, refined),
        "PCB refinement must not reuse the coarse mesh"
    );
    assert_eq!(
        scene.prepared, original,
        "collision preparation does not mutate the rendered scene"
    );
    // Corrupt source bytes must produce an explicit unavailable PCB, even when a prior analysis is cached.
    runtime
        .assets
        .borrow_mut()
        .insert(sha256, b"corrupt source".to_vec());
    let (refused, errors) = runtime
        .generated_collision_subjects(&scene, None, Rc::new(|| true))
        .await;
    assert!(
        errors
            .iter()
            .any(|error| error.contains("PCB unavailable") && error.contains("SHA-256"))
    );
    assert!(matches!(
        refused
            .iter()
            .find(|subject| subject.id() == "pcb-reference"),
        Some(CadCollisionSubject::GeneratedCaseBody { analysis: None, .. })
    ));
    let pcb = refused
        .iter()
        .find(|subject| subject.id() == "pcb-reference")
        .unwrap();
    let case = refused
        .iter()
        .find(|subject| subject.id() == "bottom")
        .unwrap();
    let result = runtime
        .query_collision_subjects(
            pcb,
            case,
            0.1,
            None,
            scene.scope.clone(),
            scene.token,
            scene.snapshot.document.revision,
            Rc::new(|| true),
        )
        .await
        .unwrap();
    assert!(result.observation.is_none());
    let finding = crate::model_delivery::CadCollisionPairFinding {
        left_id: pcb.id().into(),
        left_label: pcb.label().into(),
        right_id: case.id().into(),
        right_label: case.label().into(),
        result,
    };
    assert!(
        finding.summary(0.1).contains("PCB [pcb-reference]")
            && finding.summary(0.1).contains("Uncertain")
    );
    assert!(finding.provenance_details()[0].contains("identity unresolved"));
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn physical_pcb_preparation_cancellation_rejects_analysis_and_recovers() {
    let (runtime, scene, url) = fixture_document(mechanical_document()).await;
    let core = project_name_test_support::in_process_core(&runtime);
    let (entered, release) = core.gate_matching_reply(|request| matches!(request,
        CoreRequest::PrepareCase { ir, .. } if ir.bodies.iter().any(|body| body.body.id == "pcb-reference")));
    let active = Rc::new(Cell::new(true));
    let current: Rc<dyn Fn() -> bool> = {
        let active = active.clone();
        Rc::new(move || active.get())
    };
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_scene = scene.clone();
    let task_current = current.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = task_runtime
            .generated_collision_subjects(&task_scene, None, task_current)
            .await;
        let _ = done.send(result);
    });
    entered
        .await
        .expect("physical PCB preparation reached the actual Core gate");
    active.set(false);
    runtime.cancel_collision_checks();
    release.send(()).unwrap();
    let (subjects, _) = receive.await.unwrap();
    assert!(subjects.is_empty());
    assert!(
        runtime
            .generated_analysis_cache
            .borrow()
            .values()
            .all(|analysis| analysis.body_id != "pcb-reference"),
        "cancelled preparation never dispatches/caches PCB analysis"
    );
    active.set(true);
    let (subjects, errors) = runtime
        .generated_collision_subjects(&scene, None, current)
        .await;
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        subjects
            .iter()
            .any(|subject| subject.id() == "pcb-reference" && subject.mesh().is_some())
    );
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn mechanical_collision_flow_analyzes_physical_pcb_and_reports_case_pair_provenance() {
    let (runtime, scene, url) = fixture_document(mechanical_document()).await;
    assert!(scene.mechanical.is_some());
    assert!(
        scene
            .prepared
            .bodies
            .iter()
            .all(|body| body.body.id != "pcb-reference"),
        "render preparation deliberately excludes the physical PCB"
    );
    let (subjects, errors) = runtime
        .generated_collision_subjects(&scene, None, Rc::new(|| true))
        .await;
    assert!(errors.is_empty(), "{errors:?}");
    let pcb = subjects
        .iter()
        .find(|subject| subject.id() == "pcb-reference")
        .expect("production collision flow must request physical PCB analysis");
    assert!(
        pcb.mesh().is_some(),
        "PCB must reach analyze-generated-body, not an unavailable placeholder"
    );
    assert!(
        runtime
            .generated_analysis_cache
            .borrow()
            .values()
            .any(|analysis| analysis.body_id == "pcb-reference")
    );
    let case = subjects
        .iter()
        .find(|subject| subject.id() == "bottom")
        .unwrap();
    let result = runtime
        .query_collision_subjects(
            pcb,
            case,
            0.1,
            None,
            scene.scope.clone(),
            scene.token,
            scene.snapshot.document.revision,
            Rc::new(|| true),
        )
        .await
        .unwrap();
    let finding = crate::model_delivery::CadCollisionPairFinding {
        left_id: pcb.id().into(),
        left_label: pcb.label().into(),
        right_id: case.id().into(),
        right_label: case.label().into(),
        result,
    };
    assert!(finding.summary(0.1).contains("pcb-reference"));
    assert!(finding.provenance_details()[0].contains("generated body pcb-reference"));
    assert!(finding.provenance_details()[1].contains("generated body bottom"));
    assert!(matches!(
        finding.result.assessment,
        CadCollisionAssessment::Unknown { .. }
    ));
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
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

#[wasm_bindgen_test]
async fn controlled_v2_reply_reaches_runtime_supported_band_with_complete_binding() {
    // The controlled worker supplies a seam report, not authenticated CAD geometry.
    let (runtime, scene, url) =
        fixture_document_mode(in_process_support::board_document(), "bounded").await;
    let analysis = runtime
        .analyze_generated_case_body(&scene, "contract-plate", None, Rc::new(|| true))
        .await
        .unwrap();
    let left = CadCollisionSubject::GeneratedCaseBody {
        id: "contract-left".into(),
        name: "Plate".into(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        analysis: Some(analysis),
    };
    let mut right = left.clone();
    if let CadCollisionSubject::GeneratedCaseBody {
        id,
        row_transform_f64,
        ..
    } = &mut right
    {
        *id = "contract-right".into();
        row_transform_f64[14] = 3.;
    }
    let result = runtime
        .query_collision_subjects(
            &left,
            &right,
            0.5,
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
        CadCollisionAssessment::SupportedBand {
            lower_mm: 1.,
            upper_mm: 1.,
            relation: crate::model_delivery::CadCollisionRelation::Clear
        }
    ));
    let report = result.query_qualification.unwrap();
    assert_eq!(report.binding.snapshot.token, scene.token.0.to_string());
    assert_eq!(
        report.binding.snapshot.session_epoch,
        scene.scope.session_epoch.0.to_string()
    );
    assert_eq!(report.binding.right_row_transform_f64[14], 3.);
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn missing_generated_lease_prepares_once_per_batch_joins_pending_and_second_miss_stays_unknown()
 {
    let (runtime, scene, url) =
        fixture_document_mode(in_process_support::board_document(), "missing").await;
    let analysis = runtime
        .analyze_generated_case_body(&scene, "contract-plate", None, Rc::new(|| true))
        .await
        .unwrap();
    let subject = CadCollisionSubject::GeneratedCaseBody {
        id: "contract-plate".into(),
        name: "Plate".into(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        analysis: Some(analysis.clone()),
    };
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_scene = scene.clone();
    let task_subject = subject.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = task_runtime
            .query_collision_subjects(
                &task_subject,
                &task_subject,
                0.5,
                None,
                task_scene.scope.clone(),
                task_scene.token,
                task_scene.snapshot.document.revision,
                Rc::new(|| true),
            )
            .await;
        let _ = done.send(result);
    });
    let result = runtime
        .query_collision_subjects(
            &subject,
            &subject,
            0.5,
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
    assert!(receive.await.unwrap().is_ok());
    let cached = runtime
        .generated_analysis_cache
        .borrow()
        .values()
        .next()
        .unwrap()
        .clone();
    assert_eq!(
        cached.analysis_ref.as_ref().unwrap().mesh_identity,
        "final-f64-v1/runtime-controlled-2",
        "both concurrent checks join exactly one explicit preparation"
    );
    assert_eq!(cached.deflection_mm, 0.1, "basic recovery does not refine");
    assert!(!Rc::ptr_eq(&analysis, &cached));
    runtime
        .query_collision_subjects(
            &subject,
            &subject,
            0.5,
            None,
            scene.scope.clone(),
            scene.token,
            scene.snapshot.document.revision,
            Rc::new(|| true),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime
            .generated_analysis_cache
            .borrow()
            .values()
            .next()
            .unwrap()
            .analysis_ref
            .as_ref()
            .unwrap()
            .mesh_identity,
        "final-f64-v1/runtime-controlled-2",
        "a second lease miss cannot loop on the ready asset"
    );
    runtime.cancel_collision_checks();
    runtime
        .query_collision_subjects(
            &subject,
            &subject,
            0.5,
            None,
            scene.scope.clone(),
            scene.token,
            scene.snapshot.document.revision,
            Rc::new(|| true),
        )
        .await
        .unwrap();
    assert_eq!(
        runtime
            .generated_analysis_cache
            .borrow()
            .values()
            .next()
            .unwrap()
            .analysis_ref
            .as_ref()
            .unwrap()
            .mesh_identity,
        "final-f64-v1/runtime-controlled-3",
        "a new explicit batch can prepare again"
    );
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

// Controlled STEP reader at the worker transport seam; it certifies no geometry.
async fn vendor_refinement_fixture() -> (Rc<Runtime>, Rc<CadScene>, String, CadCollisionSubject) {
    use boardstudio_web_host::cad_jobs::{CadStepFaceOccurrence, CadStepSourceFace, CadVendorStep};
    let (runtime, scene, old_url) = fixture().await;
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&old_url).unwrap();
    let bytes = b"controlled vendor STEP source".to_vec();
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    runtime.assets.borrow_mut().insert(sha.clone(), bytes);
    let asset = CadVendorStep {
        source_sha256: sha,
        surface_positions_mm_f64: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
        triangle_face_occurrence_indices: vec![0],
        source_faces: vec![CadStepSourceFace {
            solid_id: "10".into(),
            face_id: "20".into(),
            support_id: "30".into(),
            support_kind: "PLANE".into(),
            ..Default::default()
        }],
        face_occurrences: vec![CadStepFaceOccurrence {
            representation_id: "40".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let source = format!(
        r#"
        const asset = {};
        const activation = {};
        const bits = value => {{ const view = new DataView(new ArrayBuffer(8)); view.setFloat64(0,value); return view.getBigUint64(0).toString(16).padStart(16,'0'); }};
        let reads = 0;
        self.postMessage({{kind:'ready'}});
        self.onmessage = event => {{
            if (event.data.kind === 'cancel') return;
            const input = JSON.parse(event.data.frame);
            if (input.operation === 'collision-query') {{
                const q = input.collisionQuery;
                const unknown = () => ({{state:'unknown',reasons:['evidence-unavailable']}});
                const binding = {{snapshotRevision:input.identity.revision,snapshot:{{...input.identity,token:String(input.identity.token),sessionEpoch:String(input.identity.sessionEpoch)}},
                    workerGeneration:q.left.analysisRef.workerGeneration,leftModelId:q.left.modelId,rightModelId:q.right.modelId,
                    leftMeshIdentity:q.left.analysisRef.meshIdentity,rightMeshIdentity:q.right.analysisRef.meshIdentity,
                    leftAnalysis:q.left.analysisRef,rightAnalysis:q.right.analysisRef,leftMesh:q.left.analysisMesh,rightMesh:q.right.analysisMesh,
                    leftRowTransformF64:q.left.rowTransformF64,rightRowTransformF64:q.right.rowTransformF64,
                    clearanceThresholdMm:q.clearanceThresholdMm,requestedAccuracyMm:q.requestedAccuracyMm}};
                self.postMessage({{kind:'reply',frame:JSON.stringify({{requestId:input.requestId,jobId:input.jobId,identity:input.identity,operation:input.operation,outcome:'completed'}}),result:{{
                    revision:input.identity.revision,
                    collision:{{kind:'surface-separated',surfaceDistanceMm:1,closestPoints:{{triangles:{{left:{{triangleIndex:0,faceOccurrenceIndex:0}},right:{{triangleIndex:0,faceOccurrenceIndex:0}}}},point1Mm:[0,0,0],point2Mm:[1,0,0]}}}},
                    queryQualification:{{version:1,algorithm:'aabb-interval-exact-v1',binding,limits:q.qualificationLimits,
                        meshDistance:unknown(),sourceBounds:[unknown(),unknown()],contactRegion:unknown(),containment:unknown()}}
                }}}});
                return;
            }}
            reads++;
            if (input.stepImportOptions.targetDeflectionMm === 0.007) {{
                setTimeout(() => self.postMessage({{kind:'reply',frame:JSON.stringify({{
                    requestId:input.requestId,jobId:input.jobId,identity:input.identity,operation:input.operation,
                    outcome:'failed',error:'controlled vendor refusal'
                }}),result:null}}), 20);
                return;
            }}
            const vendorStep = structuredClone(asset);
            const options = {{previewRepairBudgetMm:input.stepImportOptions.previewRepairBudgetMm??null,targetDeflectionMm:input.stepImportOptions.targetDeflectionMm??null}};
            const mesh = structuredClone(activation.descriptors.vendor);
            mesh.meshIdentity = 'final-f64-v1/controlled-vendor-'+reads;
            mesh.settings = {{targetDeflectionMm:options.targetDeflectionMm,selectedDeflectionMm:options.targetDeflectionMm??0.1,angularRad:0.5,previewRepairBudgetMm:options.previewRepairBudgetMm}};
            vendorStep.analysisMesh = mesh;
            vendorStep.sourceQualification = {{state:'unknown',reasons:['controlled-vendor-source-unavailable']}};
            const derivation = input.sourceSha256+(options.previewRepairBudgetMm===null?'':':repair='+bits(options.previewRepairBudgetMm))+(options.targetDeflectionMm===null?'':':deflection='+bits(options.targetDeflectionMm));
            vendorStep.analysisRef = {{workerGeneration:'controlled-vendor',meshIdentity:mesh.meshIdentity,
                source:{{kind:'vendor-step',sourceSha256:input.sourceSha256,importOptions:options}},derivationIdentity:'vendor-step-analysis-v2/'+derivation}};
            vendorStep.sourceSha256 = input.sourceSha256;
            vendorStep.importOptions = input.stepImportOptions;
            vendorStep.accuracy.previewDeflectionMm = input.stepImportOptions.targetDeflectionMm;
            vendorStep.accuracy.previewAngularRad = 0.5;
            vendorStep.accuracy.selectedRepairBudgetMm = input.stepImportOptions.previewRepairBudgetMm ?? null;
            vendorStep.accuracy.sampledSupportChecks = reads;
            vendorStep.surfacePositionsMmF64 = new Float64Array(asset.surfacePositionsMmF64);
            vendorStep.triangleFaceOccurrenceIndices = new Uint32Array([0]);
            vendorStep.lineSegmentsMmF64 = new Float64Array();
            vendorStep.lineRenderPositions = new Float32Array();
            vendorStep.segmentLineOccurrenceIndices = new Uint32Array();
            setTimeout(() => self.postMessage({{kind:'reply',frame:JSON.stringify({{
                requestId:input.requestId,jobId:input.jobId,identity:input.identity,operation:input.operation,outcome:'completed'
            }}),result:{{revision:input.identity.revision,vendorStep,bounds:{{min:[0,0,0],max:[1,1,0]}},
                mesh:{{positions:new Float32Array(asset.surfacePositionsMmF64),normals:new Float32Array([0,0,1,0,0,1,0,0,1])}}
            }}}}), 20);
        }};
    "#,
        serde_json::to_string(&asset).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap()["qualificationV2"]["activation"]
    );
    let parts = js_sys::Array::new();
    parts.push(&source.into());
    let options = BlobPropertyBag::new();
    options.set_type("text/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&parts, &options).unwrap();
    let url = Url::create_object_url_with_blob(&blob).unwrap();
    let worker = Rc::new(CadWorker::new(&url).unwrap());
    worker.ready().await.unwrap();
    *runtime.cad_worker.borrow_mut() = Some((scene.scope.clone(), worker));
    let subject = CadCollisionSubject::VendorModel {
        id: "switch-1".into(),
        reference: "SW1".into(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        vendor_step: Some(Rc::new(asset)),
    };
    (runtime, scene, url, subject)
}

fn vendor_asset(
    subject: &CadCollisionSubject,
) -> &Rc<boardstudio_web_host::cad_jobs::CadVendorStep> {
    let CadCollisionSubject::VendorModel {
        vendor_step: Some(asset),
        ..
    } = subject
    else {
        panic!("missing vendor asset")
    };
    asset
}

#[wasm_bindgen_test]
async fn tight_vendor_refinement_shares_one_preparation_without_merging_placements() {
    let (runtime, scene, url, left) = vendor_refinement_fixture().await;
    let mut right = left.clone();
    if let CadCollisionSubject::VendorModel {
        id,
        reference,
        row_transform_f64,
        ..
    } = &mut right
    {
        *id = "switch-2".into();
        *reference = "SW2".into();
        row_transform_f64[12] = 19.;
    }
    let first = runtime
        .refine_collision_vendor(&scene, &left, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    let second = runtime
        .refine_collision_vendor(&scene, &right, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(
        vendor_asset(&second).accuracy.sampled_support_checks,
        1,
        "one preparation request for repeated source/settings"
    );
    assert!(Rc::ptr_eq(vendor_asset(&first), vendor_asset(&second)));
    assert_eq!(first.id(), "switch-1");
    assert_eq!(second.id(), "switch-2");
    assert_eq!(second.label(), "SW2");
    let CadCollisionSubject::VendorModel {
        row_transform_f64, ..
    } = second
    else {
        unreachable!()
    };
    assert_eq!(row_transform_f64[12], 19.);
    assert_eq!(
        vendor_asset(&first).source_faces,
        vendor_asset(&left).source_faces
    );
    assert_eq!(
        vendor_asset(&first).face_occurrences,
        vendor_asset(&left).face_occurrences
    );
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn tight_vendor_refinement_isolates_source_repair_and_target_settings() {
    let (runtime, scene, url, subject) = vendor_refinement_fixture().await;
    let first = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    let target = runtime
        .refine_collision_vendor(&scene, &subject, 0.02, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(vendor_asset(&target).accuracy.sampled_support_checks, 2);
    let mut repaired = subject.clone();
    if let CadCollisionSubject::VendorModel {
        vendor_step: Some(asset),
        ..
    } = &mut repaired
    {
        Rc::make_mut(asset).import_options.preview_repair_budget_mm = Some(0.003);
    }
    let repair = runtime
        .refine_collision_vendor(&scene, &repaired, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(vendor_asset(&repair).accuracy.sampled_support_checks, 3);
    let bytes = b"different controlled vendor source".to_vec();
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    runtime.assets.borrow_mut().insert(sha.clone(), bytes);
    let mut other = subject.clone();
    if let CadCollisionSubject::VendorModel {
        vendor_step: Some(asset),
        ..
    } = &mut other
    {
        Rc::make_mut(asset).source_sha256 = sha;
    }
    let different = runtime
        .refine_collision_vendor(&scene, &other, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(vendor_asset(&different).accuracy.sampled_support_checks, 4);
    let repeated = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert!(Rc::ptr_eq(vendor_asset(&first), vendor_asset(&repeated)));
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn tight_vendor_refinement_shares_failure_and_new_check_can_recover() {
    let (runtime, scene, url, subject) = vendor_refinement_fixture().await;
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_scene = scene.clone();
    let task_subject = subject.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = done.send(
            task_runtime
                .refine_collision_vendor(&task_scene, &task_subject, 0.007, Rc::new(|| true))
                .await,
        );
    });
    let failed = runtime
        .refine_collision_vendor(&scene, &subject, 0.007, Rc::new(|| true))
        .await
        .unwrap_err();
    let joined = receive.await.unwrap().unwrap_err();
    assert!(failed.contains("controlled vendor refusal"), "{failed}");
    assert_eq!(joined, failed);
    assert!(
        runtime
            .refine_collision_vendor(&scene, &subject, 0.007, Rc::new(|| true))
            .await
            .is_err()
    );
    let healthy = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(
        vendor_asset(&healthy).accuracy.sampled_support_checks,
        2,
        "failed placements do not each dispatch an import"
    );
    runtime.cancel_collision_checks();
    let fresh = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(vendor_asset(&fresh).accuracy.sampled_support_checks, 3);
    assert!(!Rc::ptr_eq(vendor_asset(&healthy), vendor_asset(&fresh)));
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn cancelled_tight_vendor_refinement_cannot_settle_a_replacement_check() {
    let (runtime, scene, url, subject) = vendor_refinement_fixture().await;
    let (done, receive) = futures_channel::oneshot::channel();
    let task_runtime = runtime.clone();
    let task_scene = scene.clone();
    let task_subject = subject.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = done.send(
            task_runtime
                .refine_collision_vendor(&task_scene, &task_subject, 0.01, Rc::new(|| true))
                .await,
        );
    });
    for _ in 0..100 {
        if !runtime.native_model_jobs.borrow().is_empty() {
            break;
        }
        TimeoutFuture::new(1).await;
    }
    assert!(
        !runtime.native_model_jobs.borrow().is_empty(),
        "preparation reached the worker"
    );
    runtime.cancel_collision_checks();
    let fresh = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert!(receive.await.unwrap().is_err());
    let repeated = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    assert!(Rc::ptr_eq(vendor_asset(&fresh), vendor_asset(&repeated)));
    assert!(
        runtime
            .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| false))
            .await
            .is_err(),
        "retired callers cannot reuse a ready result"
    );
    assert!(runtime.native_model_jobs.borrow().is_empty());
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

#[wasm_bindgen_test]
async fn tight_vendor_refinement_keeps_one_separate_bounded_evidence_recovery() {
    let (runtime, scene, url, subject) = vendor_refinement_fixture().await;
    let refined = runtime
        .refine_collision_vendor(&scene, &subject, 0.01, Rc::new(|| true))
        .await
        .unwrap();
    for _ in 0..2 {
        let result = runtime
            .query_collision_subjects(
                &refined,
                &refined,
                0.,
                Some(0.01),
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
        assert!(
            result.observation.is_some(),
            "missing evidence retains the raw observation"
        );
        let binding = result.query_qualification.unwrap().binding;
        assert_eq!(
            binding.left_mesh_identity,
            "final-f64-v1/controlled-vendor-2"
        );
        assert_eq!(binding.right_mesh_identity, binding.left_mesh_identity);
    }
    let other_settings = runtime
        .refine_collision_vendor(&scene, &subject, 0.02, Rc::new(|| true))
        .await
        .unwrap();
    assert_eq!(
        vendor_asset(&other_settings)
            .accuracy
            .sampled_support_checks,
        3,
        "one tight preparation and one recovery; a second lease miss cannot loop"
    );
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&url).unwrap();
}

// Controlled worker for a whole complete check: generated analyses, vendor STEP
// import and collision queries share one worker generation. It exercises the
// message, settings and counters; every geometry qualification stays Unknown.
const COMPLETE_CHECK_WORKER: &str = r#"
const payload = __PAYLOAD__;
const activation = __ACTIVATION__;
const baseAsset = __ASSET__;
let preparations = 0;
let reads = 0;
const bits = value => { const view = new DataView(new ArrayBuffer(8)); view.setFloat64(0,value); return view.getBigUint64(0).toString(16).padStart(16,'0'); };
const cancelled = new Set();
self.postMessage({kind:'ready'});
self.onmessage = event => {
    const message = event.data;
    if (message.kind === 'cancel') { cancelled.add(message.jobId); return; }
    const input = JSON.parse(message.frame);
    const outcome = cancelled.has(input.jobId) ? 'cancelled' : 'completed';
    setTimeout(() => {
        if (input.operation === 'analyze-generated-body') {
            preparations++;
            const requested = input.generatedAnalysisRequest.targetDeflectionMm ?? null;
            const analysisMesh = structuredClone(activation.descriptors.plane);
            analysisMesh.meshIdentity = 'final-f64-v1/runtime-controlled-'+preparations;
            analysisMesh.settings.targetDeflectionMm = requested;
            analysisMesh.settings.selectedDeflectionMm = Math.min(0.1, requested ?? 0.1);
            const analysisRef = {workerGeneration:'runtime-controlled',meshIdentity:analysisMesh.meshIdentity,
                source:{kind:'generated-body',bodyId:input.generatedAnalysisRequest.bodyId,revision:input.identity.revision,geometryIdentity:input.generatedAnalysisRequest.geometryIdentity},
                derivationIdentity:input.generatedAnalysisRequest.geometryIdentity+'/deflection='+(requested===null?'default':bits(requested))+'/analytic-generated-v1'};
            self.postMessage({kind:'reply',frame:JSON.stringify({requestId:input.requestId,jobId:input.jobId,identity:input.identity,operation:input.operation,outcome}),
                result:{revision:input.identity.revision,generatedAnalysis:{...payload,analysisMesh,analysisRef,
                    analysisErrorBoundMm:null,
                    sourceQualification:{state:'unknown',reasons:['controlled-runtime-source-unavailable']},
                    bodyId:input.generatedAnalysisRequest.bodyId,revision:input.identity.revision,
                    geometryIdentity:input.generatedAnalysisRequest.geometryIdentity,
                    deflectionMm:input.generatedAnalysisRequest.targetDeflectionMm ?? payload.deflectionMm,
                    surfacePositionsMmF64:new Float64Array(payload.surfacePositionsMmF64),
                    triangleFaceOccurrenceIndices:new Uint32Array(payload.triangleFaceOccurrenceIndices)}}});
            return;
        }
        if (input.operation === 'collision-query') {
            const q = input.collisionQuery;
            const unknown = () => ({state:'unknown',reasons:['controlled-runtime-proof-unavailable']});
            const binding = {snapshotRevision:input.identity.revision,snapshot:{...input.identity,token:String(input.identity.token),sessionEpoch:String(input.identity.sessionEpoch)},
                workerGeneration:q.left.analysisRef.workerGeneration,leftModelId:q.left.modelId,rightModelId:q.right.modelId,
                leftMeshIdentity:q.left.analysisRef.meshIdentity,rightMeshIdentity:q.right.analysisRef.meshIdentity,
                leftAnalysis:q.left.analysisRef,rightAnalysis:q.right.analysisRef,leftMesh:q.left.analysisMesh,rightMesh:q.right.analysisMesh,
                leftRowTransformF64:q.left.rowTransformF64,rightRowTransformF64:q.right.rowTransformF64,
                clearanceThresholdMm:q.clearanceThresholdMm,requestedAccuracyMm:q.requestedAccuracyMm};
            self.postMessage({kind:'reply',frame:JSON.stringify({requestId:input.requestId,jobId:input.jobId,identity:input.identity,operation:input.operation,outcome}),
                result:{revision:input.identity.revision,
                    collision:{kind:'surface-separated',surfaceDistanceMm:1.0,closestPoints:{triangles:{left:{triangleIndex:0,faceOccurrenceIndex:0},right:{triangleIndex:0,faceOccurrenceIndex:0}},point1Mm:[20,0,0],point2Mm:[21,0,0]}},
                    queryQualification:{version:1,algorithm:'aabb-interval-exact-v1',binding,limits:q.qualificationLimits,
                        meshDistance:unknown(),sourceBounds:[unknown(),unknown()],contactRegion:unknown(),containment:unknown()}}});
            return;
        }
        reads++;
        const vendorStep = structuredClone(baseAsset);
        const options = {previewRepairBudgetMm:input.stepImportOptions.previewRepairBudgetMm??null,targetDeflectionMm:input.stepImportOptions.targetDeflectionMm??null};
        const vendorMesh = structuredClone(activation.descriptors.vendor);
        vendorMesh.meshIdentity = 'final-f64-v1/controlled-vendor-'+reads;
        vendorMesh.settings = {targetDeflectionMm:options.targetDeflectionMm,selectedDeflectionMm:options.targetDeflectionMm??0.1,angularRad:0.5,previewRepairBudgetMm:options.previewRepairBudgetMm};
        vendorStep.analysisMesh = vendorMesh;
        vendorStep.sourceQualification = {state:'unknown',reasons:['controlled-vendor-source-unavailable']};
        const derivation = input.sourceSha256+(options.previewRepairBudgetMm===null?'':':repair='+bits(options.previewRepairBudgetMm))+(options.targetDeflectionMm===null?'':':deflection='+bits(options.targetDeflectionMm));
        vendorStep.analysisRef = {workerGeneration:'runtime-controlled',meshIdentity:vendorMesh.meshIdentity,
            source:{kind:'vendor-step',sourceSha256:input.sourceSha256,importOptions:options},derivationIdentity:'vendor-step-analysis-v2/'+derivation};
        vendorStep.sourceSha256 = input.sourceSha256;
        vendorStep.importOptions = input.stepImportOptions;
        vendorStep.accuracy.previewDeflectionMm = input.stepImportOptions.targetDeflectionMm;
        vendorStep.accuracy.previewAngularRad = 0.5;
        vendorStep.accuracy.selectedRepairBudgetMm = input.stepImportOptions.previewRepairBudgetMm ?? null;
        vendorStep.accuracy.sampledSupportChecks = reads;
        vendorStep.surfacePositionsMmF64 = new Float64Array(baseAsset.surfacePositionsMmF64);
        vendorStep.triangleFaceOccurrenceIndices = new Uint32Array([0]);
        vendorStep.lineSegmentsMmF64 = new Float64Array();
        vendorStep.lineRenderPositions = new Float32Array();
        vendorStep.segmentLineOccurrenceIndices = new Uint32Array();
        self.postMessage({kind:'reply',frame:JSON.stringify({requestId:input.requestId,jobId:input.jobId,identity:input.identity,operation:input.operation,outcome}),
            result:{revision:input.identity.revision,vendorStep,bounds:{min:[0,0,0],max:[1,1,0]},
                mesh:{positions:new Float32Array(baseAsset.surfacePositionsMmF64),normals:new Float32Array([0,0,1,0,0,1,0,0,1])}}});
    }, 20);
};
"#;

const ROW_MESH_IDENTITY: &str = "final-f64-v1/controlled-vendor-row";
const REFINED_VENDOR_MESH_IDENTITY: &str = "final-f64-v1/controlled-vendor-1";
const WORKER_GENERATION: &str = "runtime-controlled";

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
    let (runtime, scene, old_url) = fixture_document_mode(mechanical_document(), "unknown").await;
    runtime.cad_worker.borrow_mut().take().unwrap().1.close();
    Url::revoke_object_url(&old_url).unwrap();
    let bytes = b"controlled complete-check vendor source".to_vec();
    let sha = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    runtime.assets.borrow_mut().insert(sha.clone(), bytes);
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
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
