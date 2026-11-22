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

    let cached = block_on(adapter.deliver_models(1, &models, &assets, &ports, &batch(2), live()))
        .expect("new batch reuses verified mesh");
    assert_eq!(model_decode_count.get(), 1);
    assert!(Rc::ptr_eq(
        &rows.delivered[0].mesh,
        &cached.delivered[1].mesh
    ));
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
