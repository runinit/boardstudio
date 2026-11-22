use super::*;
use boardstudio_model::model::{KeycapMount, KeycapProfile, Pose2, Side, Vec2};
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{Blob, BlobPropertyBag, Url};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

struct WorkerSource(String);

impl WorkerSource {
    fn new(source: &str) -> Self {
        let parts = Array::of1(&JsValue::from_str(source));
        let options = BlobPropertyBag::new();
        options.set_type("text/javascript");
        let blob = Blob::new_with_str_sequence_and_options(&parts, &options).unwrap();
        Self(Url::create_object_url_with_blob(&blob).unwrap())
    }
}

impl Drop for WorkerSource {
    fn drop(&mut self) {
        let _ = Url::revoke_object_url(&self.0);
    }
}

fn spec() -> KeycapSpec {
    KeycapSpec {
        id: "key-1".into(),
        reference: "A1".into(),
        profile: KeycapProfile::Cherry,
        mount: KeycapMount::Mx,
        row: 1,
        size: Vec2 { x: 18.0, y: 18.0 },
        top_size: Vec2 { x: 12.0, y: 12.0 },
        height: 8.0,
        tilt: 0.0,
        dish_depth: 0.0,
        spherical: false,
        wall_thickness: 1.2,
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        z: 8.4,
        travel: 4.0,
        legend: String::new(),
        color: "#b0b5bd".into(),
        legend_color: "#202124".into(),
    }
}

#[wasm_bindgen_test]
async fn keycaps_print_uses_a_distinct_operation_and_one_fused_body() {
    let source = WorkerSource::new(
        r#"
      self.postMessage({kind:'ready'});
      self.onmessage = event => {
        const input = JSON.parse(event.data.frame);
        const correct = input.header.operation === 'print-keycaps';
        const replyOperation = input.header.requestId === 'print-stale' ? 'preview' : input.header.operation;
        const frame = JSON.stringify({header: {...input.header, operation:replyOperation},outcome:correct?'completed':'failed',
          error:correct?null:'Print must not use preview identity'});
        const result = {revision:input.header.identity.revision,step:new Uint8Array(),mesh:null,
          bodies:[{id:'keycap:key-1',name:'A1',positions:new Float32Array([0,0,0,1,0,0,0,1,0]),
            normals:new Float32Array([0,0,1,0,0,1,0,0,1])}],bounds:null};
        if(input.header.requestId === 'print-truncated') {
          result.bodies[0].positions = new Float32Array([0,0,0]);
          result.bodies[0].normals = new Float32Array([0,0,1]);
        }
        self.postMessage({kind:'reply',frame,result});
      };
    "#,
    );
    let worker = CadWorker::new(&source.0).unwrap();
    let identity = CadSnapshotIdentity {
        token: 1,
        session_epoch: 1,
        document_id: "print".into(),
        board_id: "board".into(),
        instance_id: None,
        revision: 1,
    };
    let mut cap = spec();
    cap.legend = "O".into();
    let result = worker
        .request_keycaps_print(
            "print-1".into(),
            "print-job".into(),
            identity.clone(),
            vec![cap.clone()],
        )
        .await
        .expect("print operation must not share preview identity or separate legend IDs");
    assert_eq!(result.bodies.len(), 1);
    for request in ["print-stale", "print-truncated"] {
        assert!(
            worker
                .request_keycaps_print(
                    request.into(),
                    request.into(),
                    identity.clone(),
                    vec![cap.clone()]
                )
                .await
                .is_err(),
            "stale or incomplete print results must refuse"
        );
    }
    worker.close();
}

#[wasm_bindgen_test]
async fn keycaps_worker_failure_late_reply_stale_revision_and_retry_are_correlated() {
    let source = WorkerSource::new(
        r#"
        self.postMessage({ kind: "ready" });
        self.onmessage = (event) => {
          const input = JSON.parse(event.data.frame);
          const respond = (outcome, error, bodyId, revision = input.header.identity.revision) => {
            const frame = JSON.stringify({
              header: input.header,
              outcome,
              error,
            });
            const result = {
              revision,
              step: new Uint8Array([73, 83, 79, 45, 49, 48, 51, 48, 51, 45, 50, 49, 59]),
              mesh: null,
              bodies: input.header.operation === "preview"
                ? [{ id: bodyId, name: "test cap", positions: new Float32Array([0, 0, 0]), normals: new Float32Array([0, 0, 1]) }]
                : [],
              bounds: null,
            };
            self.postMessage({ kind: "reply", frame, result });
          };
          if (input.header.requestId === "keycaps-preview-old") {
            respond("failed", "injected preview provider failure", "keycap:key-1");
            setTimeout(() => respond("completed", null, "keycap:late-old"), 25);
          } else if (input.header.requestId === "keycaps-preview-current") {
            setTimeout(() => respond("completed", null, "keycap:key-1"), 60);
          } else if (input.header.requestId === "keycaps-preview-stale") {
            respond("completed", null, "", input.header.identity.revision + 1);
          }
        };
        "#,
    );
    let worker = CadWorker::new(&source.0).unwrap();
    worker.ready().await.unwrap();

    let identity = CadSnapshotIdentity {
        token: 7,
        session_epoch: 2,
        document_id: "keycaps-worker-test".into(),
        board_id: "left".into(),
        instance_id: None,
        revision: 11,
    };
    let keycap = spec();
    let failed_preview = worker
        .request_keycaps_preview(
            "keycaps-preview-old".into(),
            "preview-old-job".into(),
            identity.clone(),
            vec![keycap.clone()],
        )
        .await
        .unwrap_err();
    assert_eq!(failed_preview.0, "injected preview provider failure");

    let current_preview = worker
        .request_keycaps_preview(
            "keycaps-preview-current".into(),
            "preview-current-job".into(),
            identity.clone(),
            vec![keycap.clone()],
        )
        .await
        .expect("the current preview must settle after the delayed old reply");
    assert_eq!(current_preview.revision, identity.revision);
    assert_eq!(current_preview.bodies[0].id, "keycap:key-1");

    let stale_preview = worker
        .request_keycaps_preview(
            "keycaps-preview-stale".into(),
            "preview-stale-job".into(),
            identity.clone(),
            vec![keycap],
        )
        .await
        .unwrap_err();
    assert_eq!(
        stale_preview.0,
        "Keycap CAD preview meshes are missing, malformed, or stale"
    );
    assert_eq!(
        keycap_operation_error(&CadOperation::ExportStep, true),
        Some(KEYCAP_STEP_UNSUPPORTED)
    );
    assert_eq!(
        keycap_operation_error(&CadOperation::ExportStep, false),
        None
    );
    worker.close();
}

#[wasm_bindgen_test]
async fn read_step_and_collision_query_transfer_typed_arrays_and_validate_replies() {
    use crate::cad_jobs::{
        CadCollisionMeshPair, CadCollisionObservation, CadCollisionOperand, CadCollisionQuery,
        CadStepImportOptions, validate_reply,
    };

    let source = WorkerSource::new(
        r#"
        self.postMessage({ kind: "ready" });
        self.onmessage = (event) => {
          const input = JSON.parse(event.data.frame);
          if (input.header.operation === "read-step") {
            const frame = JSON.stringify({
              header: input.header,
              outcome: "completed",
              error: null,
            });
            const result = {
              revision: input.header.identity.revision,
              step: new Uint8Array(),
              mesh: {
                positions: new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
                normals: new Float32Array([0, 0, 1, 0, 0, 1, 0, 0, 1]),
              },
              bodies: [],
              bounds: { min: [0, 0, 0], max: [2, 2, 0] },
              vendorStep: {
                sourceSha256: input.sourceSha256,
                importOptions: input.stepImportOptions ?? { previewRepairBudgetMm: null },
                surfacePositionsMmF64: new Float64Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
                sourceFaces: [{
                  solidId: "10",
                  shellIndex: 0,
                  faceId: "20",
                  supportId: "30",
                  supportKind: "PLANE",
                }],
                faceOccurrences: [{
                  sourceFaceIndex: 0,
                  solidOccurrenceIndex: 0,
                  representationId: "40",
                  sourceUncertaintyMm: 0.01,
                  repairDisplacementMm: null,
                }],
                triangleFaceOccurrenceIndices: new Uint32Array([0]),
                lineSegmentsMmF64: new Float64Array([0, 0, 0, 2, 2, 0]),
                lineRenderPositions: new Float32Array([0, 0, 0, 2, 2, 0]),
                lineOccurrences: [{
                  lineOccurrenceIndex: 0,
                  trimmedCurveId: "414",
                  sourceLineId: "413",
                  curveSetId: "420",
                  representationId: "430",
                  sourceUnitToMm: 1.0,
                  placementF64: new Float64Array([1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]),
                  declaredUncertaintyMm: null,
                }],
                segmentLineOccurrenceIndices: new Uint32Array([0]),
                precision: [],
                accuracy: {
                  previewDeflectionMm: 0.1,
                  previewAngularRad: 0.5,
                  analysisErrorBoundMm: null,
                  boundsFromSourceGeometry: true,
                  meshClosedOriented: true,
                  previewFloorMm: 0.02,
                  previewDeclaredMultiplier: 2.0,
                  relaxedBeyondDeclaredPrecision: false,
                  sampledSupportChecks: 0,
                  maxSampledSupportResidualMm: null,
                  fallbackMeshedFaces: 0,
                  discardedDisplayTriangles: 0,
                },
              },
            };
            self.postMessage({ kind: "reply", frame, result });
          } else if (input.header.operation === "collision-query") {
            const leftTags = new Uint32Array(event.data.leftTags);
            const rightTags = new Uint32Array(event.data.rightTags);
            const frame = JSON.stringify({
              header: input.header,
              outcome: "completed",
              error: null,
            });
            const result = {
              revision: input.header.identity.revision,
              step: new Uint8Array(),
              mesh: null,
              bodies: [],
              bounds: null,
              collision: {
                kind: "surface-separated",
                surfaceDistanceMm: 1.25,
                closestPoints: {
                  triangles: {
                    left: { triangleIndex: 0, faceOccurrenceIndex: leftTags[0] },
                    right: { triangleIndex: 0, faceOccurrenceIndex: rightTags[0] },
                  },
                  point1Mm: [1.0, 0.0, 0.0],
                  point2Mm: [2.25, 0.0, 0.0],
                },
              },
            };
            const q = input.collisionQuery;
            const unknown = () => ({state:'unknown',reasons:['evidence-unavailable']});
            result.queryQualification = {version:1,algorithm:'aabb-interval-exact-v1',limits:q.qualificationLimits,
              binding:{snapshotRevision:input.header.identity.revision,snapshot:{...input.header.identity,token:String(input.header.identity.token),sessionEpoch:String(input.header.identity.sessionEpoch)},
                workerGeneration:q.left.analysisRef.workerGeneration,leftModelId:q.left.modelId,rightModelId:q.right.modelId,
                leftMeshIdentity:q.left.analysisRef.meshIdentity,rightMeshIdentity:q.right.analysisRef.meshIdentity,
                leftAnalysis:q.left.analysisRef,rightAnalysis:q.right.analysisRef,leftMesh:q.left.analysisMesh,rightMesh:q.right.analysisMesh,
                leftRowTransformF64:q.left.rowTransformF64,rightRowTransformF64:q.right.rowTransformF64,
                clearanceThresholdMm:q.clearanceThresholdMm,requestedAccuracyMm:q.requestedAccuracyMm},
              meshDistance:unknown(),sourceBounds:[unknown(),unknown()],contactRegion:unknown(),containment:unknown()};
            for (const key of ['step','mesh','bodies','bounds']) delete result[key];
            self.postMessage({ kind: "reply", frame, result });
          }
        };
        "#,
    );
    let worker = CadWorker::new(&source.0).unwrap();
    let identity = CadSnapshotIdentity {
        token: 1,
        session_epoch: 1,
        document_id: "step-collision".into(),
        board_id: "board".into(),
        instance_id: None,
        revision: 3,
    };
    let sha_a = "4bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459a";
    let step_req = CadRequest {
        request_id: "step-1".into(),
        job_id: "step-job-1".into(),
        identity: identity.clone(),
        operation: CadOperation::ReadStep,
        prepared: None,
        source_sha256: Some(sha_a.into()),
        step_import_options: Some(CadStepImportOptions::default()),
        collision_query: None,
        generated_analysis_request: None,
        collision_meshes: None,
        input_bytes: vec![1],
    };
    let step_reply = worker.request(step_req.clone()).await.unwrap();
    let validated_step = validate_reply(&step_req, step_reply, &identity).unwrap();
    let vendor_step = validated_step.vendor_step.unwrap();
    assert_eq!(vendor_step.source_sha256, sha_a);
    assert_eq!(vendor_step.line_occurrences.len(), 1);
    assert_eq!(
        vendor_step.line_render_positions,
        [0.0, 0.0, 0.0, 2.0, 2.0, 0.0]
    );

    let id_mat = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap();
    let descriptor: qualification::AnalysisMesh = serde_json::from_value(
        fixture["qualificationV2"]["activation"]["descriptors"]["vendor"].clone(),
    )
    .unwrap();
    let mut reference: qualification::AnalysisRef = serde_json::from_value(
        fixture["qualificationV2"]["activation"]["workerReferences"]["vendor"].clone(),
    )
    .unwrap();
    reference.source = qualification::Source::VendorStep {
        source_sha256: sha_a.into(),
        import_options: CadStepImportOptions::default().into(),
    };
    reference.derivation_identity =
        qualification::derivation_identity(&reference.source, &descriptor);
    let collision_req = CadRequest {
        request_id: "col-1".into(),
        job_id: "col-job-1".into(),
        identity: identity.clone(),
        operation: CadOperation::CollisionQuery,
        prepared: None,
        source_sha256: None,
        step_import_options: None,
        collision_query: Some(CadCollisionQuery {
            left: CadCollisionOperand {
                analysis_ref: Some(reference.clone()),
                analysis_mesh: Some(descriptor.clone()),
                model_id: "m1".into(),
                source_sha256: Some(sha_a.into()),
                generated_geometry_identity: None,
                import_options: CadStepImportOptions::default(),
                row_transform_f64: id_mat,
            },
            right: CadCollisionOperand {
                analysis_ref: Some(reference.clone()),
                analysis_mesh: Some(descriptor.clone()),
                model_id: "m2".into(),
                source_sha256: Some(sha_a.into()),
                generated_geometry_identity: None,
                import_options: CadStepImportOptions::default(),
                row_transform_f64: id_mat,
            },
            clearance_threshold_mm: 0.5,
            requested_accuracy_mm: None,
            qualification_limits: Default::default(),
        }),
        generated_analysis_request: None,
        collision_meshes: Some(CadCollisionMeshPair {
            left_positions_mm_f64: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            left_triangle_face_occurrence_indices: vec![7],
            right_positions_mm_f64: vec![2.25, 0.0, 0.0, 3.25, 0.0, 0.0, 2.25, 1.0, 0.0],
            right_triangle_face_occurrence_indices: vec![11],
        }),
        input_bytes: vec![],
    };
    let col_reply = worker.request(collision_req.clone()).await.unwrap();
    let validated_col = validate_reply(&collision_req, col_reply, &identity).unwrap();
    match validated_col.collision.unwrap() {
        CadCollisionObservation::SurfaceSeparated {
            surface_distance_mm,
            closest_points,
        } => {
            assert_eq!(surface_distance_mm, 1.25);
            assert_eq!(closest_points.triangles.left.face_occurrence_index, 7);
            assert_eq!(closest_points.triangles.right.face_occurrence_index, 11);
        }
        other => panic!("unexpected observation {other:?}"),
    }
    worker.close();
}
