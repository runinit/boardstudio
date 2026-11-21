use super::*;
use boardstudio_model::model::{Pose2, Side, Vec2, Vec3};

struct InactiveTestLease;

impl ModelSourceLease for InactiveTestLease {
    fn is_active(&self) -> bool {
        false
    }
}

fn model(id: &str, reference: &str, path: &str) -> PcbModel {
    PcbModel {
        id: id.into(),
        reference: reference.into(),
        path: path.into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        offset: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        rotation: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }
}

fn asset(id: &str, sha256: &str, name: &str) -> Asset {
    Asset {
        id: id.into(),
        name: name.into(),
        media_type: "model/stl".into(),
        sha256: sha256.into(),
        license: None,
        source: None,
    }
}

fn valid_arrays() -> MeshArrays {
    MeshArrays {
        positions: vec![0.0; 9],
        normals: vec![1.0; 9],
        colors: None,
        step_bounds: None,
        vendor_step: None,
    }
}

fn batch(generation: u64) -> ModelBatchIdentity {
    // The cache tests need only stable semantic identity; scene liveness is
    // covered by the page owner integration which owns CadScene creation.
    let source_owner: Rc<dyn ModelSourceLease> = Rc::new(InactiveTestLease);
    ModelBatchIdentity {
        owner: ModelOwnerIdentity {
            scope: Scope {
                session_epoch: boardstudio_application::SessionEpoch(1),
                document_id: "doc".into(),
                board_id: "board".into(),
                instance_id: None,
            },
            snapshot_token: SnapshotToken(1),
            viewer_instance: 1,
            projection_generation: 1,
            source_owner: Rc::downgrade(&source_owner),
        },
        accepted_revision: 1,
        batch_generation: generation,
    }
}

fn document_asset(id: &str, sha256: &str, filename: &str) -> ResolvedModelAsset {
    ResolvedModelAsset {
        id: id.into(),
        sha256: sha256.into(),
        filename: filename.into(),
        source: ModelAssetSource::Document,
    }
}

fn archived(id: &str, sha256: &str, filename: &str) -> AssetSelection {
    AssetSelection::Archived(document_asset(id, sha256, filename))
}

fn valid_mesh() -> Rc<ValidatedMesh> {
    Rc::new(ValidatedMesh::try_from(valid_arrays()).unwrap())
}

/// Ports serving `bytes` for any requested digest; every decoder yields `valid_arrays`.
fn ports(bytes: Vec<u8>) -> ModelDeliveryPorts {
    ModelDeliveryPorts {
        load_verified_bytes: Rc::new(move |asset| {
            let bytes = bytes.clone();
            Box::pin(async move { Ok(Some(VerifiedModelBytes::verify(bytes, &asset.sha256)?)) })
        }),
        decode_stl: Rc::new(|_| Box::pin(async { Ok(valid_arrays()) })),
        decode_wrl: Rc::new(|_| Box::pin(async { Ok(valid_arrays()) })),
        read_step: Rc::new(|_, _| Box::pin(async { Ok(valid_arrays()) })),
    }
}

fn live() -> Rc<dyn Fn() -> bool> {
    Rc::new(|| true)
}

fn poll<F: Future>(future: &mut Pin<Box<F>>) -> Poll<F::Output> {
    future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
}

/// A refresh with default import options for `batch`.
fn refresh<'a>(
    adapter: &'a ModelDeliveryAdapter,
    sha: &'a str,
    batch: &'a str,
    is_current: Rc<dyn Fn() -> bool>,
    load: impl Future<Output = Result<MeshArrays, String>> + 'a,
) -> Pin<Box<impl Future<Output = Result<Rc<ValidatedMesh>, String>> + 'a>> {
    Box::pin(adapter.refresh_step_evidence(sha, Default::default(), batch, is_current, load))
}

/// A provider that must never be called: a joined or refused refresh cannot load.
fn never(reason: &'static str) -> impl Future<Output = Result<MeshArrays, String>> {
    async move { panic!("{reason}") }
}

/// A provider that settles only when its sender is used.
fn gated() -> (
    futures_channel::oneshot::Sender<Result<MeshArrays, String>>,
    impl Future<Output = Result<MeshArrays, String>>,
) {
    let (release, wait) = futures_channel::oneshot::channel();
    (release, async { wait.await.unwrap() })
}

const TRIANGLE_MM: [f64; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];

/// A one-face vendor STEP asset whose single triangle has `surface` coordinates.
fn vendor_step(sha: &str, surface: [f64; 9]) -> boardstudio_web_host::cad_jobs::CadVendorStep {
    use boardstudio_web_host::cad_jobs::*;
    CadVendorStep {
        source_sha256: sha.into(),
        surface_positions_mm_f64: surface.to_vec(),
        source_faces: vec![CadStepSourceFace {
            solid_id: "10".into(),
            shell_index: 0,
            face_id: "20".into(),
            support_id: "30".into(),
            support_kind: "PLANE".into(),
        }],
        face_occurrences: vec![CadStepFaceOccurrence {
            source_face_index: 0,
            solid_occurrence_index: 0,
            representation_id: "40".into(),
            source_uncertainty_mm: None,
            repair_displacement_mm: None,
        }],
        triangle_face_occurrence_indices: vec![0],
        accuracy: CadStepAccuracy {
            preview_deflection_mm: 0.1,
            preview_angular_rad: 0.5,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn separated(
    distance: f64,
    point1_mm: [f64; 3],
    point2_mm: [f64; 3],
) -> boardstudio_web_host::cad_jobs::CadCollisionObservation {
    use boardstudio_web_host::cad_jobs::*;
    let triangle = CadTriangleProvenance {
        triangle_index: 0,
        face_occurrence_index: 0,
    };
    CadCollisionObservation::SurfaceSeparated {
        surface_distance_mm: distance,
        closest_points: CadClosestPointsWitness {
            triangles: CadTrianglePairProvenance {
                left: triangle.clone(),
                right: triangle,
            },
            point1_mm,
            point2_mm,
        },
    }
}

#[test]
fn asset_selection_obeys_attached_then_native_then_ergogen_precedence() {
    let assets = vec![
        asset("attached", "a", "attached.stl"),
        asset("native", "b", "native.wrl"),
        asset("ergogen", "c", "ergogen.stl"),
    ];
    let mut mapping = BTreeMap::new();
    mapping.insert("body.stl".to_owned(), "attached".to_owned());
    let mut native = BTreeMap::new();
    native.insert("body.stl".to_owned(), "native".to_owned());
    let selection = select_model_asset("body.stl", None, &native, &assets, |_| {
        Some("ergogen".into())
    });
    assert_eq!(selection, archived("native", "b", "native.wrl"));
    let reference = BoardReference {
        id: "ref".into(),
        board_id: "board".into(),
        asset_id: "board-file".into(),
        enabled: true,
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        elevation: 0.0,
        model_assets: mapping,
    };
    assert_eq!(
        select_model_asset("body.stl", Some(&reference), &native, &assets, |_| {
            Some("ergogen".into())
        }),
        archived("attached", "a", "attached.stl")
    );
    assert_eq!(
        select_model_asset("missing.stl", None, &BTreeMap::new(), &[], |_| {
            Some("bundled-model:vendor/missing.stl".into())
        }),
        AssetSelection::MissingBundledProvider {
            asset_id: "bundled-model:vendor/missing.stl".into()
        }
    );
}

#[test]
fn accepted_kiswitch_preview_path_resolves_to_the_known_packaged_descriptor() {
    let path = "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp";
    let id = "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp";
    let document = ProjectDoc::empty("case-models", "Case model path test");
    let expected = ResolvedModelAsset {
        id: id.to_owned(),
        sha256: "a".repeat(64),
        filename: "SW_Cherry_MX_PCB.stp".to_owned(),
        source: ModelAssetSource::Packaged {
            url_path: "assets/ergogen-models/model-test.stp".to_owned(),
        },
    };
    let resolved = resolve_preview_assets(
        &[model("mesh-row", "SW1", path)],
        None,
        &BTreeMap::new(),
        &document,
        |preview_path| (preview_path == path).then(|| id.to_owned()),
        |asset_id| (asset_id == id).then(|| expected.clone()),
    );
    assert_eq!(
        resolved,
        vec![("mesh-row".to_owned(), AssetSelection::Packaged(expected))]
    );
}

#[test]
fn core_packaged_preview_path_joins_the_native_table_to_its_exact_descriptor() {
    let id = "bundled-model:kiswitch/SW_Cherry_MX_PCB.stp";
    let bundled = crate::bundled_models::bundled_model(id).unwrap();
    assert_eq!(
        bundled.url_path,
        "assets/ergogen-models/model-dd931656985824ce.stp"
    );
    let source = BTreeMap::from([(id.to_owned(), bundled.url_path.to_owned())]);
    let native = native_model_path_assets(&source);
    let path = format!("${{KIPRJMOD}}/{}", bundled.url_path);
    let expected = ResolvedModelAsset {
        id: id.to_owned(),
        sha256: bundled.sha256.to_owned(),
        filename: bundled.filename.to_owned(),
        source: ModelAssetSource::Packaged {
            url_path: bundled.url_path.to_owned(),
        },
    };
    let resolved = resolve_preview_assets(
        &[model("preview-switch-row", "SW1", &path)],
        None,
        &native,
        &ProjectDoc::empty("case-models", "Case model path test"),
        |_| None,
        |asset_id| (asset_id == id).then(|| expected.clone()),
    );
    assert_eq!(
        resolved,
        vec![(
            "preview-switch-row".to_owned(),
            AssetSelection::Packaged(expected)
        )]
    );
}

#[test]
fn native_export_path_table_is_inverted_and_preserves_document_asset_precedence() {
    let source = BTreeMap::from([("asset-1".to_owned(), "models/hash.stl".to_owned())]);
    let native = native_model_path_assets(&source);
    assert_eq!(native["models/hash.stl"], "asset-1");
    // Core archived preview paths carry the KiCad project prefix; native export rows do not.
    for (path, bundled) in [
        (
            "${KIPRJMOD}/models/hash.stl",
            Some("bundled-model:other.stl"),
        ),
        ("models/hash.stl", None),
    ] {
        assert_eq!(
            select_model_asset(
                path,
                None,
                &native,
                &[asset("asset-1", "ab", "part.stl")],
                |_| bundled.map(str::to_owned),
            ),
            archived("asset-1", "ab", "part.stl"),
            "{path}"
        );
    }
}

#[test]
fn verified_bytes_reject_wrong_digest_bounds_and_uppercase_expected_digest() {
    let bytes = b"model".to_vec();
    let digest = sha256_hex(&bytes);
    let verified = VerifiedModelBytes::verify(bytes.clone(), &digest).unwrap();
    assert_eq!(verified.bytes(), bytes);
    assert_eq!(verified.sha256(), digest);
    assert!(VerifiedModelBytes::verify(bytes.clone(), "00").is_err());
    assert!(VerifiedModelBytes::verify(Vec::new(), &digest).is_err());
    assert!(
        VerifiedModelBytes::verify(bytes, &digest.to_ascii_uppercase()).is_err(),
        "an uppercase expected hash is not the stored lowercase identity"
    );
}

#[test]
fn format_selection_is_case_insensitive_and_rejects_other_extensions() {
    for (filename, format) in [
        ("BODY.STL", ModelFormat::Stl),
        ("body.WrL", ModelFormat::Wrl),
        ("model.STP", ModelFormat::Step),
    ] {
        assert_eq!(ModelFormat::from_filename(filename).unwrap(), format);
    }
    assert!(ModelFormat::from_filename("body.obj").is_err());
}

#[test]
fn mesh_validation_requires_complete_finite_matching_buffers() {
    assert!(ValidatedMesh::try_from(valid_arrays()).is_ok());
    let invalid: [fn(&mut MeshArrays); 5] = [
        |arrays| {
            arrays.positions.pop();
        },
        |arrays| {
            arrays.normals.pop();
        },
        |arrays| arrays.positions[0] = f32::NAN,
        |arrays| arrays.colors = Some(vec![0.0; 8]),
        |arrays| arrays.colors = Some(vec![f32::INFINITY; 9]),
    ];
    for (index, mutate) in invalid.into_iter().enumerate() {
        let mut arrays = valid_arrays();
        mutate(&mut arrays);
        assert!(ValidatedMesh::try_from(arrays).is_err(), "case {index}");
    }
}

#[test]
fn new_batch_replaces_pending_task_and_old_token_cannot_settle_it() {
    let mut cache = ModelMeshCache::default();
    let first = batch(1);
    let second = batch(2);
    let MeshCacheClaim::Start {
        task_token: first_token,
        ..
    } = cache.claim("sha", &first).unwrap()
    else {
        panic!("first request should start");
    };
    assert!(matches!(
        cache.claim("sha", &first).unwrap(),
        MeshCacheClaim::JoinPending { task_token } if task_token == first_token
    ));
    let MeshCacheClaim::Start {
        task_token: replacement_token,
        superseded,
        ..
    } = cache.claim("sha", &second).unwrap()
    else {
        panic!("new batch must own a replacement task");
    };
    assert_ne!(first_token, replacement_token);
    assert_eq!(
        superseded,
        Some(EvictedTask {
            sha256: "sha".into(),
            task_token: first_token,
        })
    );
    let mesh = valid_mesh();
    assert!(!cache.complete("sha", first_token, mesh.clone()));
    assert!(!cache.fail("sha", first_token));
    assert!(cache.complete("sha", replacement_token, mesh.clone()));
    assert!(matches!(
        cache.claim("sha", &first).unwrap(),
        MeshCacheClaim::Reuse(cached) if Rc::ptr_eq(&cached, &mesh)
    ));
}

#[test]
fn failed_current_task_is_removed_for_retry() {
    let mut cache = ModelMeshCache::default();
    let batch = batch(1);
    let MeshCacheClaim::Start { task_token, .. } = cache.claim("sha", &batch).unwrap() else {
        panic!("first request should start");
    };
    assert!(cache.fail("sha", task_token));
    assert!(matches!(
        cache.claim("sha", &batch).unwrap(),
        MeshCacheClaim::Start { .. }
    ));
}

#[test]
fn cache_is_bounded_and_reports_evicted_pending_task_token() {
    let mut cache = ModelMeshCache::default();
    let owner_batch = batch(1);
    let mut first_token = None;
    for index in 0..MESH_CACHE_CAPACITY {
        let key = format!("sha-{index}");
        let MeshCacheClaim::Start { task_token, .. } = cache.claim(&key, &owner_batch).unwrap()
        else {
            panic!("new key should start");
        };
        if index == 0 {
            first_token = Some(task_token);
        }
    }
    let MeshCacheClaim::Start { evicted, .. } = cache.claim("sha-new", &owner_batch).unwrap()
    else {
        panic!("new key should start");
    };
    assert_eq!(
        evicted,
        Some(EvictedTask {
            sha256: "sha-0".into(),
            task_token: first_token.unwrap(),
        })
    );
}

#[test]
fn row_merge_preserves_exact_preview_model_ids_and_references() {
    let models = vec![model("renderer-id", "SW8", "body.stl")];
    let mesh = valid_mesh();
    let outcomes = BTreeMap::from([("renderer-id".into(), Ok(mesh.clone()))]);
    let rows = merge_model_rows(&models, &outcomes);
    assert_eq!(rows.delivered[0].id, "renderer-id");
    assert!(Rc::ptr_eq(&rows.delivered[0].mesh, &mesh));
    let pending = merge_model_rows(&models, &BTreeMap::new());
    assert_eq!(pending.pending, ["renderer-id"]);
    assert!(pending.failures.is_empty());
    assert!(pending.delivered.is_empty());
    let failed = merge_model_rows(
        &models,
        &BTreeMap::from([("renderer-id".into(), Err("decode rejected".into()))]),
    );
    assert!(failed.pending.is_empty());
    assert_eq!(failed.failures[0].reference, "SW8");
    assert_eq!(failed.failures[0].reason, "decode rejected");
}

#[test]
fn batch_delivery_decodes_each_sha_once_and_keeps_model_row_ids() {
    let model_decode_count = Rc::new(std::cell::Cell::new(0));
    let decode_count = model_decode_count.clone();
    let bytes = b"same-model".to_vec();
    let digest = sha256_hex(&bytes);
    let mut ports = ports(bytes);
    ports.decode_stl = Rc::new(move |_| {
        decode_count.set(decode_count.get() + 1);
        Box::pin(async { Ok(valid_arrays()) })
    });
    let assets = ["model-1", "model-2"]
        .map(|id| (id.to_owned(), archived("asset-1", &digest, "switch.stl")))
        .into();
    let models = [
        model("model-1", "SW1", "switch.stl"),
        model("model-2", "SW2", "switch.stl"),
    ];
    let adapter = ModelDeliveryAdapter::default();

    let rows = block_on(adapter.deliver_models(1, &models, &assets, &ports, &batch(1), live()))
        .expect("current model batch completes");

    assert_eq!(model_decode_count.get(), 1);
    assert_eq!(
        rows.delivered
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["model-1", "model-2"]
    );
    assert!(rows.pending.is_empty());
    assert!(rows.failures.is_empty());
}

#[test]
fn mounted_module_placement_uses_direct_asset_id_and_preserves_core_matrix() {
    let bytes = b"mounted-module-model".to_vec();
    let digest = sha256_hex(&bytes);
    let mut ports = ports(bytes);
    let load = ports.load_verified_bytes.clone();
    ports.load_verified_bytes = Rc::new(move |asset| {
        assert_eq!(asset.id, "module-mesh");
        load(asset)
    });
    let placement = ModuleModelPlacement {
        id: "module-model/placement-a/0".into(),
        asset_id: "module-mesh".into(),
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 12.0, 13.0, 14.0, 1.0,
        ],
    };
    let selected = select_model_asset_id(
        &placement.asset_id,
        &[asset("module-mesh", &digest, "radio.stl")],
        |_| None,
    );
    assert!(matches!(selected, AssetSelection::Archived(_)));
    let rows = block_on(ModelDeliveryAdapter::default().deliver_module_placements(
        1,
        std::slice::from_ref(&placement),
        &BTreeMap::from([(placement.id.clone(), selected)]),
        &ports,
        &batch(1),
        live(),
    ))
    .expect("current module delivery should complete");

    assert_eq!(rows.delivered.len(), 1);
    assert_eq!(rows.delivered[0].id, placement.id);
    assert_eq!(rows.delivered[0].matrix, Some(placement.matrix));
    assert!(rows.failures.is_empty());
    assert!(rows.pending.is_empty());
}

#[test]
fn step_cache_keys_isolate_repair_budgets_and_retain_full_vendor_asset() {
    use boardstudio_web_host::cad_jobs::{CadBounds, CadStepImportOptions};

    let sha = "a".repeat(64);
    let default_opts = CadStepImportOptions::default();
    let repair_a = CadStepImportOptions {
        preview_repair_budget_mm: Some(0.003),
        ..Default::default()
    };
    let repair_b = CadStepImportOptions {
        preview_repair_budget_mm: Some(0.005),
        ..Default::default()
    };
    assert_ne!(
        derived_step_cache_key(&sha, default_opts).unwrap(),
        derived_step_cache_key(&sha, repair_a).unwrap()
    );
    assert_ne!(
        derived_step_cache_key(&sha, repair_a).unwrap(),
        derived_step_cache_key(&sha, repair_b).unwrap()
    );
    for bad in [f64::NAN, 0.0, -0.001, f64::INFINITY] {
        assert!(
            derived_step_cache_key(
                &sha,
                CadStepImportOptions {
                    preview_repair_budget_mm: Some(bad),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }

    let vendor_step = Rc::new(vendor_step(&sha, TRIANGLE_MM));
    let validated = Rc::new(
        ValidatedMesh::try_from(MeshArrays {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            colors: None,
            step_bounds: Some(CadBounds {
                min: [0.0, 0.0, 0.0],
                max: [1.0, 1.0, 0.0],
            }),
            vendor_step: Some(vendor_step.clone()),
        })
        .expect("complete STEP asset validates"),
    );

    let mut cache = ModelMeshCache::default();
    let owner_batch = batch(1);
    let MeshCacheClaim::Start { task_token, .. } = cache
        .claim_with_step_options(&sha, default_opts, &owner_batch)
        .unwrap()
    else {
        panic!("default STEP claim starts");
    };
    assert!(
        cache
            .complete_with_step_options(&sha, default_opts, task_token, validated.clone())
            .unwrap()
    );
    assert!(
        matches!(
            cache
                .claim_with_step_options(&sha, repair_a, &owner_batch)
                .unwrap(),
            MeshCacheClaim::Start { .. }
        ),
        "changing repair budget must not reuse default cached mesh"
    );
}

#[test]
fn collision_assessment_distinguishes_unknown_supported_bands_and_tight_0_01mm_requests() {
    let make_asset = |sha: &str, analysis_bound: Option<f64>| {
        let mut asset = vendor_step(sha, TRIANGLE_MM);
        asset.face_occurrences[0].source_uncertainty_mm = Some(0.001);
        asset.accuracy = boardstudio_web_host::cad_jobs::CadStepAccuracy {
            analysis_error_bound_mm: analysis_bound,
            bounds_from_source_geometry: true,
            mesh_closed_oriented: true,
            preview_floor_mm: 0.02,
            preview_declared_multiplier: 2.0,
            ..asset.accuracy
        };
        asset
    };
    let obs = separated(0.5, [0.0, 0.0, 0.0], [0.5, 0.0, 0.0]);

    let uncertified_left = make_asset(&"a".repeat(64), None);
    let uncertified_right = make_asset(&"b".repeat(64), None);
    let result = assess_collision_observation(
        &uncertified_left,
        &uncertified_right,
        0.2,
        None,
        obs.clone(),
    );
    assert!(result.left_provenance.is_some());
    assert!(result.right_provenance.is_some());
    assert!(matches!(
        result.assessment,
        CadCollisionAssessment::Unknown {
            refinement_requested: false,
            ..
        }
    ));

    let certified_left = make_asset(&"a".repeat(64), Some(0.02));
    let certified_right = make_asset(&"b".repeat(64), Some(0.02));
    let clear =
        assess_collision_observation(&certified_left, &certified_right, 0.2, None, obs.clone());
    assert!(
        matches!(clear.assessment, CadCollisionAssessment::Unknown { .. }),
        "caller-authored bounds and containment cannot activate qualification v2"
    );

    let containment_unknown =
        assess_collision_observation(&certified_left, &certified_right, 0.2, None, obs.clone());
    assert!(matches!(
        containment_unknown.assessment,
        CadCollisionAssessment::Unknown { .. }
    ));

    let violation =
        assess_collision_observation(&certified_left, &certified_right, 0.6, None, obs.clone());
    assert!(matches!(
        violation.assessment,
        CadCollisionAssessment::Unknown { .. }
    ));

    let tight = assess_collision_observation(
        &certified_left,
        &certified_right,
        0.2,
        Some(TIGHT_COLLISION_ACCURACY_MM),
        obs,
    );
    assert!(matches!(
        tight.assessment,
        CadCollisionAssessment::Unknown {
            refinement_requested: true,
            ..
        }
    ));

    let row = DeliveredModel {
        id: "pcb-part-1".into(),
        mesh: valid_mesh(),
        matrix: None,
    };
    assert!(
        resolve_physical_model_row_transform(&row, None).is_err(),
        "PCB row with matrix: None must never substitute identity"
    );
}

#[test]
fn generated_collision_preserves_numeric_face_provenance_and_unknown_accuracy() {
    use boardstudio_web_host::cad_jobs::{CadGeneratedAnalysis, generated_geometry_identity};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap();
    let body: boardstudio_model::model::PreparedCaseIR =
        serde_json::from_value(fixture["preparedBody"].clone()).unwrap();
    let mut payload = fixture["directPayload"].clone();
    payload["geometryIdentity"] = generated_geometry_identity(&body).unwrap().into();
    let analysis: CadGeneratedAnalysis = serde_json::from_value(payload).unwrap();
    let left = CadCollisionSubject::GeneratedCaseBody {
        id: body.body.id.clone(),
        name: body.body.name.clone(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        analysis: Some(Rc::new(analysis)),
    };
    let mut right = left.clone();
    if let CadCollisionSubject::GeneratedCaseBody {
        id,
        row_transform_f64,
        ..
    } = &mut right
    {
        *id = "other".into();
        row_transform_f64[12] = 21.0;
    }
    let operand = right.operand().unwrap();
    assert!(operand.source_sha256.is_none() && operand.generated_geometry_identity.is_some());
    assert_eq!(operand.row_transform_f64[12], 21.0);
    let result = assess_subject_collision_observation(
        &left,
        &right,
        0.1,
        Some(0.01),
        None,
        separated(1.0, [20., 0., 0.], [21., 0., 0.]),
    );
    assert!(
        matches!(result.left_provenance, Some(CadSubjectFaceProvenance::Generated { ref source, .. }) if source.face_index == 0)
    );
    assert!(matches!(
        result.assessment,
        CadCollisionAssessment::Unknown {
            refinement_requested: true,
            ..
        }
    ));
    let finding = CadCollisionPairFinding {
        left_id: left.id().into(),
        left_label: left.label().into(),
        right_id: right.id().into(),
        right_label: right.label().into(),
        result,
    };
    let details = finding.provenance_details();
    assert!(
        details[0].contains("generated body contract-plate")
            && details[0].contains("source face index 0")
    );
    assert!(
        !details[0].contains("STEP"),
        "generated indices are not STEP entities"
    );
    let summary = finding.summary(0.1);
    assert!(summary.contains("Uncertain") && summary.contains("surface separation 1.000 mm"));
    assert!(
        summary.contains("contract-plate") && summary.contains("other"),
        "operand IDs must survive duplicate or mutable labels"
    );
    assert!(
        summary.contains("20.00, 0.00, 0.00")
            && summary.contains("requested accuracy remains unqualified")
    );
}

#[test]
fn collision_provenance_projection_distinguishes_step_occurrences_and_unresolved_identity() {
    let face = CadResolvedFaceProvenance {
        triangle_index: 12,
        face_occurrence_index: 2,
        solid_occurrence_index: 7,
        representation_id: "80".into(),
        solid_id: "10".into(),
        shell_index: 1,
        face_id: "20".into(),
        support_id: "30".into(),
        support_kind: "PLANE".into(),
    };
    let mut other = face.clone();
    other.solid_occurrence_index = 8;
    other.face_occurrence_index = 3;
    let mut finding = CadCollisionPairFinding {
        left_id: "module-A/U1".into(),
        left_label: "U1".into(),
        right_id: "module-B/U1".into(),
        right_label: "U1".into(),
        result: CadCollisionResult {
            source_interval_mm: None,
            query_qualification: None,
            observation: None,
            left_provenance: Some(CadSubjectFaceProvenance::Vendor(face)),
            right_provenance: Some(CadSubjectFaceProvenance::Vendor(other)),
            assessment: CadCollisionAssessment::Unknown {
                reasons: vec!["query bound unresolved".into()],
                refinement_requested: false,
            },
        },
    };
    let details = finding.provenance_details();
    for (detail, id, occurrence) in [
        (&details[0], "module-A/U1", "solid occurrence 7"),
        (&details[1], "module-B/U1", "solid occurrence 8"),
    ] {
        for expected in [
            id,
            occurrence,
            "STEP representation #80",
            "source solid #10",
            "source face #20",
            "support #30 (PLANE)",
            "triangle 12",
        ] {
            assert!(detail.contains(expected), "missing {expected}: {detail}");
        }
    }
    assert!(details[0].contains("face occurrence 2") && details[1].contains("face occurrence 3"));
    finding.result.right_provenance = None;
    assert_eq!(
        finding.provenance_details()[1],
        "U1 [module-B/U1]: source face/occurrence identity unresolved"
    );
}

#[test]
fn collision_physical_pcb_transform_composes_reference_side_and_offsets_once() {
    let model: PcbModel = serde_json::from_value(serde_json::json!({
            "id":"part", "reference":"U1", "path":"part.step", "pose":{"at":{"x":10,"y":20},"rotation":90}, "side":"front",
            "offset":{"x":1,"y":2,"z":3}, "rotation":{"x":0,"y":0,"z":0}, "scale":{"x":2,"y":1,"z":1}
        })).unwrap();
    let reference: BoardReference = serde_json::from_value(serde_json::json!({
            "id":"ref","boardId":"board","assetId":"pcb","enabled":true,"pose":{"at":{"x":100,"y":200},"rotation":90},"elevation":5,"modelAssets":{}
        })).unwrap();
    let point = |matrix: [f64; 16]| {
        [
            matrix[0] + matrix[12],
            matrix[1] + matrix[13],
            matrix[2] + matrix[14],
        ]
    };
    for (side, expected) in [
        (Side::Front, [77., 208., 8.]),
        (Side::Back, [77., 212., 0.4]),
    ] {
        let model = PcbModel {
            side,
            ..model.clone()
        };
        let actual = point(
            physical_pcb_model_row_transform_f64(&model, 1.6, Some(&reference), true).unwrap(),
        );
        for axis in 0..3 {
            assert!((actual[axis] - expected[axis]).abs() < 1e-10);
        }
    }
}

#[test]
fn collision_step_refinement_identity_keeps_repair_and_target_separate() {
    use boardstudio_web_host::cad_jobs::CadStepImportOptions;
    let sha = "a".repeat(64);
    let base = CadStepImportOptions {
        preview_repair_budget_mm: Some(0.02),
        ..Default::default()
    };
    let refined = CadStepImportOptions {
        target_deflection_mm: Some(0.01),
        ..base
    };
    assert!(!base.same_identity(refined));
    assert_ne!(
        derived_step_cache_key(&sha, base).unwrap(),
        derived_step_cache_key(&sha, refined).unwrap()
    );
    assert!(refined.same_identity(refined));
    for target in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(
            derived_step_cache_key(
                &sha,
                CadStepImportOptions {
                    target_deflection_mm: Some(target),
                    ..base
                }
            )
            .is_err()
        );
    }
}

#[test]
fn evidence_refresh_joins_pending_and_never_reprepares_a_failed_or_evicted_asset_in_one_batch() {
    let mut cache = ModelMeshCache::default();
    let token = match cache.claim("asset", &batch(1)).unwrap() {
        MeshCacheClaim::Start { task_token, .. } => task_token,
        _ => panic!("initial task"),
    };
    let refreshed = match cache
        .claim_evidence_refresh("asset", "check-1", live())
        .unwrap()
    {
        MeshCacheClaim::Start {
            task_token,
            superseded: Some(old),
            ..
        } => {
            assert_eq!(old.task_token, token);
            task_token
        }
        _ => panic!("an evidence batch cannot join an unrelated preview owner"),
    };
    let mesh = Rc::new(ValidatedMesh::try_from(valid_arrays()).unwrap());
    assert!(!cache.complete("asset", token, mesh.clone()));
    assert!(cache.complete("asset", refreshed, mesh.clone()));
    assert!(matches!(
        cache
            .claim_evidence_refresh("asset", "check-1", live())
            .unwrap(),
        MeshCacheClaim::Reuse(_)
    ));
    let newer = match cache
        .claim_evidence_refresh("asset", "check-2", live())
        .unwrap()
    {
        MeshCacheClaim::Start { task_token, .. } => task_token,
        _ => panic!("explicit refresh"),
    };
    assert!(!cache.complete("asset", token, mesh));
    assert!(cache.fail("asset", newer));
    assert!(
        cache
            .claim_evidence_refresh("asset", "check-2", live())
            .is_err()
    );
    for index in 0..81 {
        cache
            .claim_evidence_refresh(&format!("other-{index}"), "check-2", live())
            .unwrap();
    }
    assert!(
        cache
            .claim_evidence_refresh("asset", "check-2", live())
            .is_err(),
        "LRU pressure cannot erase the attempt marker"
    );
    assert!(matches!(
        cache
            .claim_evidence_refresh("asset", "check-3", live())
            .unwrap(),
        MeshCacheClaim::Start { .. }
    ));

    let adapter = ModelDeliveryAdapter::default();
    let sha = "a".repeat(64);
    let (release, wait) = gated();
    let mut first = refresh(&adapter, &sha, "batch", live(), wait);
    let mut second = refresh(
        &adapter,
        &sha,
        "batch",
        live(),
        never("joined task must not run another provider call"),
    );
    assert!(poll(&mut first).is_pending());
    assert!(poll(&mut second).is_pending());
    release
        .send(Err("controlled provider refusal".into()))
        .unwrap();
    assert_eq!(block_on(first).unwrap_err(), "controlled provider refusal");
    assert_eq!(block_on(second).unwrap_err(), "controlled provider refusal");
    assert!(
        block_on(refresh(
            &adapter,
            &sha,
            "batch",
            live(),
            never("same-batch retry loop")
        ))
        .is_err()
    );
}

fn refresh_arrays(sha: &str) -> MeshArrays {
    let mut arrays = valid_arrays();
    arrays.step_bounds = Some(boardstudio_web_host::cad_jobs::CadBounds {
        min: [0.; 3],
        max: [0.; 3],
    });
    arrays.vendor_step = Some(Rc::new(vendor_step(sha, [0.; 9])));
    arrays
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn dropped_refresh_producer_settles_owned_waiters_and_new_batch_recovers() {
    let adapter = ModelDeliveryAdapter::default();
    let sha = "a".repeat(64);
    let (_release, wait) = gated();
    let mut producer = refresh(&adapter, &sha, "batch-1", live(), wait);
    let mut join = refresh(
        &adapter,
        &sha,
        "batch-1",
        live(),
        never("a live same-batch join cannot prepare"),
    );
    assert!(poll(&mut producer).is_pending());
    assert!(poll(&mut join).is_pending());
    drop(producer);
    assert!(
        matches!(poll(&mut join), Poll::Ready(Err(reason)) if reason.contains("cancelled")),
        "dropping the refresh producer must settle its joined waiters"
    );
    assert!(adapter.pending.borrow().is_empty());
    assert!(
        block_on(refresh(
            &adapter,
            &sha,
            "batch-1",
            live(),
            never("drop cannot authorize a same-batch retry"),
        ))
        .is_err()
    );
    let mut recovered = refresh(&adapter, &sha, "batch-2", live(), async {
        Ok(refresh_arrays(&sha))
    });
    assert!(
        matches!(poll(&mut recovered), Poll::Ready(Ok(_))),
        "a new batch must complete rather than joining an abandoned token"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn retired_refresh_is_superseded_before_old_producer_settles() {
    let adapter = ModelDeliveryAdapter::default();
    let sha = "a".repeat(64);
    let live_old = Rc::new(std::cell::Cell::new(true));
    let current = live_old.clone();
    let (release, wait) = gated();
    let mut old = refresh(
        &adapter,
        &sha,
        "old-scope",
        Rc::new(move || current.get()),
        wait,
    );
    let mut old_join = refresh(
        &adapter,
        &sha,
        "old-scope",
        live(),
        never("live matching refresh must join"),
    );
    assert!(poll(&mut old).is_pending());
    assert!(poll(&mut old_join).is_pending());
    live_old.set(false);
    let mut same_batch = refresh(
        &adapter,
        &sha,
        "old-scope",
        live(),
        never("retirement does not authorize a second same-batch preparation"),
    );
    assert!(matches!(poll(&mut same_batch), Poll::Ready(Err(_))));
    let mut replacement = refresh(&adapter, &sha, "new-scope", live(), async {
        Ok(refresh_arrays(&sha))
    });
    let Poll::Ready(Ok(current)) = poll(&mut replacement) else {
        panic!("a new batch must start before the retired producer settles");
    };
    assert!(matches!(poll(&mut old_join), Poll::Ready(Err(_))));
    release.send(Ok(refresh_arrays(&sha))).unwrap();
    assert!(matches!(poll(&mut old), Poll::Ready(Err(_))));
    let reused = block_on(refresh(
        &adapter,
        &sha,
        "new-scope",
        live(),
        never("old settlement cannot remove the new ready slot"),
    ))
    .unwrap();
    assert!(Rc::ptr_eq(&current, &reused));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn dropped_refresh_join_and_old_owner_cannot_cancel_the_current_producer() {
    let adapter = ModelDeliveryAdapter::default();
    let sha = "a".repeat(64);
    let (_old_release, old_wait) = gated();
    let mut old = refresh(&adapter, &sha, "old", live(), old_wait);
    assert!(poll(&mut old).is_pending());
    let (release, wait) = gated();
    let mut producer = refresh(&adapter, &sha, "new", live(), wait);
    assert!(poll(&mut producer).is_pending());
    let mut dropped_join = refresh(
        &adapter,
        &sha,
        "new",
        live(),
        never("matching live producer must be shared"),
    );
    assert!(poll(&mut dropped_join).is_pending());
    drop(dropped_join);
    drop(old);
    assert!(poll(&mut producer).is_pending());
    let mut surviving_join = refresh(
        &adapter,
        &sha,
        "new",
        live(),
        never("old cleanup must leave the replacement's waiters and slot intact"),
    );
    assert!(poll(&mut surviving_join).is_pending());
    release.send(Ok(refresh_arrays(&sha))).unwrap();
    let current = block_on(producer).unwrap();
    assert!(Rc::ptr_eq(&current, &block_on(surviving_join).unwrap()));
    assert!(adapter.pending.borrow().is_empty());
    let reused = block_on(refresh(
        &adapter,
        &sha,
        "new",
        live(),
        never("successful settlement must disarm cleanup"),
    ))
    .unwrap();
    assert!(Rc::ptr_eq(&current, &reused));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn ordinary_delivery_and_refresh_supersede_incompatible_producers_without_cross_settlement() {
    let bytes = b"controlled model delivery lifecycle".to_vec();
    let sha = sha256_hex(&bytes);
    let mut ports = ports(bytes);
    ports.decode_stl = Rc::new(|_| panic!("STEP lifecycle control"));
    ports.decode_wrl = Rc::new(|_| panic!("STEP lifecycle control"));
    ports.read_step =
        Rc::new(|bytes, _| Box::pin(async move { Ok(refresh_arrays(bytes.sha256())) }));
    let models = [model("model", "U1", "body.step")];
    let selections = BTreeMap::from([("model".into(), archived("asset", &sha, "body.step"))]);
    let old_batch = batch(1);
    let new_batch = batch(2);
    let adapter = ModelDeliveryAdapter::default();
    let live_old = Rc::new(std::cell::Cell::new(true));
    let current = live_old.clone();
    let (release, wait) = gated();
    let mut refreshing = refresh(
        &adapter,
        &sha,
        "old-scope",
        Rc::new(move || current.get()),
        wait,
    );
    assert!(poll(&mut refreshing).is_pending());
    live_old.set(false);
    let mut delivery = deliver_models(&adapter, &models, &selections, &ports, &new_batch, live());
    let Poll::Ready(Some(rows)) = poll(&mut delivery) else {
        panic!("ordinary delivery cannot join an evidence producer with unknown preview ownership");
    };
    assert_eq!(rows.delivered.len(), 1);
    release.send(Ok(refresh_arrays(&sha))).unwrap();
    assert!(matches!(poll(&mut refreshing), Poll::Ready(Err(_))));
    let cached = block_on(deliver_models(
        &adapter,
        &models,
        &selections,
        &ports,
        &new_batch,
        live(),
    ))
    .unwrap();
    assert!(Rc::ptr_eq(
        &rows.delivered[0].mesh,
        &cached.delivered[0].mesh
    ));

    // Reverse transition: an evidence batch has no comparable preview owner
    // and must not join a still-running ordinary Pending task either.
    let adapter = ModelDeliveryAdapter::default();
    let (release, wait) = gated();
    let gate = Rc::new(std::cell::RefCell::new(Some(wait)));
    let mut gated_ports = ports.clone();
    gated_ports.read_step = Rc::new(move |_, _| {
        let wait = gate
            .borrow_mut()
            .take()
            .expect("only the ordinary producer loads");
        Box::pin(wait)
    });
    let live_old = Rc::new(std::cell::Cell::new(true));
    let current = live_old.clone();
    let mut delivery = deliver_models(
        &adapter,
        &models,
        &selections,
        &gated_ports,
        &old_batch,
        Rc::new(move || current.get()),
    );
    assert!(poll(&mut delivery).is_pending());
    let mut old_join = deliver_models(
        &adapter,
        &models,
        &selections,
        &gated_ports,
        &old_batch,
        live(),
    );
    assert!(poll(&mut old_join).is_pending());
    live_old.set(false);
    let mut replacement = refresh(&adapter, &sha, "new-scope", live(), async {
        Ok(refresh_arrays(&sha))
    });
    let Poll::Ready(Ok(current)) = poll(&mut replacement) else {
        panic!("evidence recovery must start before the unrelated preview settles");
    };
    assert!(matches!(poll(&mut old_join), Poll::Ready(Some(rows)) if rows.failures.len() == 1));
    release.send(Ok(refresh_arrays(&sha))).unwrap();
    assert!(matches!(poll(&mut delivery), Poll::Ready(None)));
    let cached = block_on(deliver_models(
        &adapter,
        &models,
        &selections,
        &ports,
        &new_batch,
        live(),
    ))
    .unwrap();
    assert!(Rc::ptr_eq(&current, &cached.delivered[0].mesh));
}

fn deliver_models<'a>(
    adapter: &'a ModelDeliveryAdapter,
    models: &'a [PcbModel],
    selections: &'a BTreeMap<String, AssetSelection>,
    ports: &'a ModelDeliveryPorts,
    batch: &'a ModelBatchIdentity,
    is_current: Rc<dyn Fn() -> bool>,
) -> Pin<Box<impl Future<Output = Option<ModelDeliveryRows>> + 'a>> {
    Box::pin(adapter.deliver_models(1, models, selections, ports, batch, is_current))
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = std::task::Waker::noop();
    let mut context = std::task::Context::from_waker(waker);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}
