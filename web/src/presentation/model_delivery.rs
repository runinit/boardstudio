//! Private model-asset selection, identity, validation, and bounded mesh reuse.
//!
//! Runtime byte access, renderer decoding, STEP reading, viewer composition, and
//! renderer scene submission are supplied by the page owners. In particular,
//! model-batch liveness is independent of the renderer's scene sequence.

use super::layout_viewer_source::{LayoutPreviewSnapshot, LayoutSourceLease};
use crate::case_preview::CasePreviewOwnerLease;
use crate::parts_preview::PartsPreviewOwnerLease;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    Asset, BoardReference, ModuleModelPlacement, PcbModel, Pose2, ProjectDoc, Side, Vec2, Vec3,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    future::{Future, poll_fn},
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
};

const MESH_CACHE_CAPACITY: usize = 80;
const MAX_MODEL_BYTES: usize = 32 * 1024 * 1024;

trait ModelSourceLease {
    fn is_active(&self) -> bool;
}

impl ModelSourceLease for CasePreviewOwnerLease {
    fn is_active(&self) -> bool {
        CasePreviewOwnerLease::is_active(self)
    }
}

impl ModelSourceLease for LayoutSourceLease {
    fn is_active(&self) -> bool {
        LayoutSourceLease::is_active(self)
    }
}

impl ModelSourceLease for PartsPreviewOwnerLease {
    fn is_active(&self) -> bool {
        PartsPreviewOwnerLease::is_active(self)
    }
}

pub(crate) type ModelFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + 'static>>;

/// The owner of model work. Renderer scene submissions intentionally are not
/// part of this identity: healthy model completions can outlive a newer scene
/// sequence while remaining attached to this exact Case projection.
#[derive(Clone)]
pub(crate) struct ModelOwnerIdentity {
    scope: Scope,
    snapshot_token: SnapshotToken,
    viewer_instance: u64,
    projection_generation: u64,
    source_owner: Weak<dyn ModelSourceLease>,
}

impl ModelOwnerIdentity {
    pub(crate) fn new(
        scope: Scope,
        snapshot_token: SnapshotToken,
        viewer_instance: u64,
        projection_generation: u64,
        source_owner: &Rc<CasePreviewOwnerLease>,
    ) -> Self {
        let source_owner: Rc<dyn ModelSourceLease> = source_owner.clone();
        Self {
            scope,
            snapshot_token,
            viewer_instance,
            projection_generation,
            source_owner: Rc::downgrade(&source_owner),
        }
    }

    pub(crate) fn new_layout(
        scope: Scope,
        snapshot_token: SnapshotToken,
        source_generation: u64,
        source_owner: &Rc<LayoutSourceLease>,
    ) -> Self {
        let source_owner: Rc<dyn ModelSourceLease> = source_owner.clone();
        Self {
            scope,
            snapshot_token,
            // These keys are local to model delivery and do not share a sequence
            // domain with the renderer's independent ViewerOwner.
            viewer_instance: 0,
            projection_generation: source_generation,
            source_owner: Rc::downgrade(&source_owner),
        }
    }

    pub(crate) fn new_parts(
        scope: Scope,
        snapshot_token: SnapshotToken,
        source_generation: u64,
        source_owner: &Rc<PartsPreviewOwnerLease>,
    ) -> Self {
        let source_owner: Rc<dyn ModelSourceLease> = source_owner.clone();
        Self {
            scope,
            snapshot_token,
            viewer_instance: 0,
            projection_generation: source_generation,
            source_owner: Rc::downgrade(&source_owner),
        }
    }

    fn same_owner(&self, other: &Self) -> bool {
        self.scope == other.scope
            && self.snapshot_token == other.snapshot_token
            && self.viewer_instance == other.viewer_instance
            && self.projection_generation == other.projection_generation
            && Weak::ptr_eq(&self.source_owner, &other.source_owner)
    }

    /// Checks the captured physical preview against the current Case owner.
    /// This identity exists before mechanical CAD; the accepted revision remains
    /// a separate batch property.
    pub(crate) fn is_current_owner(
        &self,
        current_scope: &Scope,
        current_token: SnapshotToken,
        current_viewer_instance: u64,
        current_projection_generation: u64,
        current_owner: &Rc<CasePreviewOwnerLease>,
    ) -> bool {
        self.scope == *current_scope
            && self.snapshot_token == current_token
            && self.viewer_instance == current_viewer_instance
            && self.projection_generation == current_projection_generation
            && self.source_owner.upgrade().is_some_and(|captured| {
                let current_lease: Rc<dyn ModelSourceLease> = current_owner.clone();
                Rc::ptr_eq(&captured, &current_lease)
                    && captured.is_active()
                    && current_lease.is_active()
                    && current_owner.identity_matches(
                        current_scope,
                        current_token,
                        current_viewer_instance,
                        current_projection_generation,
                    )
            })
    }

    pub(crate) fn is_current_layout_owner(
        &self,
        current_scope: &Scope,
        current_token: SnapshotToken,
        source_generation: u64,
        current_owner: &Rc<LayoutSourceLease>,
    ) -> bool {
        self.scope == *current_scope
            && self.snapshot_token == current_token
            && self.viewer_instance == 0
            && self.projection_generation == source_generation
            && self.source_owner.upgrade().is_some_and(|captured| {
                let current_owner: Rc<dyn ModelSourceLease> = current_owner.clone();
                Rc::ptr_eq(&captured, &current_owner)
                    && captured.is_active()
                    && current_owner.is_active()
            })
    }

    pub(crate) fn is_current_parts_owner(
        &self,
        current_scope: &Scope,
        current_token: SnapshotToken,
        source_generation: u64,
        current_owner: &Rc<PartsPreviewOwnerLease>,
    ) -> bool {
        self.scope == *current_scope
            && self.snapshot_token == current_token
            && self.viewer_instance == 0
            && self.projection_generation == source_generation
            && self.source_owner.upgrade().is_some_and(|captured| {
                let current_owner: Rc<dyn ModelSourceLease> = current_owner.clone();
                Rc::ptr_eq(&captured, &current_owner)
                    && captured.is_active()
                    && current_owner.is_active()
            })
    }
}

/// One request for the model rows of a particular preview under an owner.
#[derive(Clone)]
pub(crate) struct ModelBatchIdentity {
    owner: ModelOwnerIdentity,
    accepted_revision: u64,
    batch_generation: u64,
}

impl ModelBatchIdentity {
    pub(crate) fn new(
        owner: ModelOwnerIdentity,
        accepted_revision: u64,
        batch_generation: u64,
    ) -> Self {
        Self {
            owner,
            accepted_revision,
            batch_generation,
        }
    }

    pub(crate) fn is_current(
        &self,
        current_owner: &ModelOwnerIdentity,
        current_revision: u64,
        current_batch_generation: u64,
    ) -> bool {
        self.owner.same_owner(current_owner)
            && self.accepted_revision == current_revision
            && self.batch_generation == current_batch_generation
    }

    pub(crate) fn owner(&self) -> &ModelOwnerIdentity {
        &self.owner
    }
}

/// Resolved document asset identity. The path is deliberately not used as a
/// byte-store key; only `sha256` crosses the Runtime storage port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedModelAsset {
    pub(crate) id: String,
    pub(crate) sha256: String,
    pub(crate) filename: String,
    pub(crate) source: ModelAssetSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ModelAssetSource {
    Document,
    Packaged { url_path: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AssetSelection {
    Archived(ResolvedModelAsset),
    Packaged(ResolvedModelAsset),
    MissingDocumentAsset { asset_id: String },
    MissingBundledProvider { asset_id: String },
    NoAssetId,
}

/// Apply the same source precedence as React's AssemblyPreview: attached
/// BoardReference mapping, native preview path table, then version-matched
/// Ergogen path helper. The helper is injected because its packaged source is
/// owned by the layout-generator build, not duplicated here.
pub(crate) fn select_model_asset(
    model_path: &str,
    reference: Option<&BoardReference>,
    native_path_assets: &BTreeMap<String, String>,
    document_assets: &[Asset],
    ergogen_model_asset_id: impl FnOnce(&str) -> Option<String>,
) -> AssetSelection {
    let asset_id = reference
        .and_then(|item| item.model_assets.get(model_path).cloned())
        .or_else(|| {
            native_path_assets
                .get(model_path)
                .or_else(|| {
                    // Core emits project-prefixed model paths from the export-relative table.
                    model_path
                        .strip_prefix("${KIPRJMOD}/")
                        .and_then(|path| native_path_assets.get(path))
                })
                .cloned()
        })
        .or_else(|| ergogen_model_asset_id(model_path));
    let Some(asset_id) = asset_id else {
        return AssetSelection::NoAssetId;
    };

    if let Some(asset) = document_assets.iter().find(|asset| asset.id == asset_id) {
        return AssetSelection::Archived(ResolvedModelAsset {
            id: asset.id.clone(),
            sha256: asset.sha256.clone(),
            filename: asset.name.clone(),
            source: ModelAssetSource::Document,
        });
    }

    if asset_id.starts_with("ergogen:model:") {
        AssetSelection::MissingBundledProvider { asset_id }
    } else {
        AssetSelection::MissingDocumentAsset { asset_id }
    }
}

/// Bytes crossing the page-model adapter must be revalidated even if they
/// came from the current-import memory map rather than BrowserStore.
#[derive(Clone, Debug)]
pub(crate) struct VerifiedModelBytes {
    bytes: Rc<[u8]>,
    sha256: String,
}

impl VerifiedModelBytes {
    pub(crate) fn verify(bytes: Vec<u8>, expected_sha256: &str) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_MODEL_BYTES {
            return Err("Model file must be between 1 byte and 32 MiB".into());
        }
        let actual = sha256_hex(&bytes);
        if !actual.eq_ignore_ascii_case(expected_sha256) {
            return Err("Model asset failed SHA-256 verification".into());
        }
        Ok(Self {
            bytes: Rc::from(bytes),
            sha256: actual,
        })
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelFormat {
    Stl,
    Wrl,
    Step,
}

impl ModelFormat {
    pub(crate) fn from_filename(filename: &str) -> Result<Self, String> {
        match filename
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
        {
            Some(ext) if ext == "stl" => Ok(Self::Stl),
            Some(ext) if ext == "wrl" => Ok(Self::Wrl),
            Some(ext) if ext == "step" || ext == "stp" => Ok(Self::Step),
            _ => Err("Unsupported model format; use STL, WRL, STEP, or STP".into()),
        }
    }
}

/// Ports supplied by Runtime and RendererPageHost. `load_verified_bytes`
/// accepts an exact source descriptor; decoding callbacks consume verified bytes.
/// STEP uses the existing identity-checked CAD worker rather than a second
/// parser implementation in presentation code.
pub(crate) struct ModelDeliveryPorts {
    pub(crate) load_verified_bytes:
        Rc<dyn Fn(ResolvedModelAsset) -> ModelFuture<Option<VerifiedModelBytes>>>,
    pub(crate) decode_stl: Rc<dyn Fn(VerifiedModelBytes) -> ModelFuture<MeshArrays>>,
    pub(crate) decode_wrl: Rc<dyn Fn(VerifiedModelBytes) -> ModelFuture<MeshArrays>>,
    pub(crate) read_step:
        Rc<dyn Fn(VerifiedModelBytes, ModelOwnerIdentity) -> ModelFuture<MeshArrays>>,
}

impl Clone for ModelDeliveryPorts {
    fn clone(&self) -> Self {
        Self {
            load_verified_bytes: self.load_verified_bytes.clone(),
            decode_stl: self.decode_stl.clone(),
            decode_wrl: self.decode_wrl.clone(),
            read_step: self.read_step.clone(),
        }
    }
}

impl ModelDeliveryPorts {
    pub(crate) async fn load(
        &self,
        asset: &ResolvedModelAsset,
    ) -> Result<Option<VerifiedModelBytes>, String> {
        let result = (self.load_verified_bytes)(asset.clone()).await?;
        if let Some(bytes) = &result
            && !bytes.sha256().eq_ignore_ascii_case(&asset.sha256)
        {
            return Err("Model byte provider returned a different SHA-256 asset".into());
        }
        Ok(result)
    }

    pub(crate) async fn decode(
        &self,
        format: ModelFormat,
        bytes: VerifiedModelBytes,
        owner: ModelOwnerIdentity,
    ) -> Result<ValidatedMesh, String> {
        let arrays = match format {
            ModelFormat::Stl => (self.decode_stl)(bytes).await?,
            ModelFormat::Wrl => (self.decode_wrl)(bytes).await?,
            ModelFormat::Step => (self.read_step)(bytes, owner).await?,
        };
        ValidatedMesh::try_from(arrays)
    }
}

/// Arrays are held in Rc-backed immutable slices so repeated model rows share
/// one decoded allocation. Adapters should create/copy JS typed arrays at most
/// once per decoded asset and retain their JS handles alongside these slices.
#[derive(Clone, Debug, Default)]
pub(crate) struct MeshArrays {
    pub(crate) positions: Vec<f32>,
    pub(crate) normals: Vec<f32>,
    pub(crate) colors: Option<Vec<f32>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ValidatedMesh {
    pub(crate) positions: Rc<[f32]>,
    pub(crate) normals: Rc<[f32]>,
    pub(crate) colors: Option<Rc<[f32]>>,
}

impl TryFrom<MeshArrays> for ValidatedMesh {
    type Error = String;

    fn try_from(arrays: MeshArrays) -> Result<Self, Self::Error> {
        if arrays.positions.is_empty() || !arrays.positions.len().is_multiple_of(9) {
            return Err("Model positions must contain complete triangles".into());
        }
        if arrays.normals.len() != arrays.positions.len() {
            return Err("Model normals must match the position buffer".into());
        }
        if arrays
            .positions
            .iter()
            .chain(&arrays.normals)
            .any(|value| !value.is_finite())
        {
            return Err("Model mesh contains non-finite positions or normals".into());
        }
        if let Some(colors) = &arrays.colors {
            if colors.len() != arrays.positions.len() {
                return Err("Model RGB colors must match the position buffer".into());
            }
            if colors.iter().any(|value| !value.is_finite()) {
                return Err("Model mesh contains non-finite colors".into());
            }
        }
        Ok(Self {
            positions: Rc::from(arrays.positions),
            normals: Rc::from(arrays.normals),
            colors: arrays.colors.map(Rc::from),
        })
    }
}

#[derive(Clone)]
enum CacheSlot {
    Pending {
        batch: ModelBatchIdentity,
        task_token: u64,
    },
    Ready(Rc<ValidatedMesh>),
}

#[derive(Clone, Debug)]
pub(crate) enum MeshCacheClaim {
    Reuse(Rc<ValidatedMesh>),
    JoinPending {
        task_token: u64,
    },
    Start {
        task_token: u64,
        /// A replaced task belonged to a superseded batch. The owner should
        /// settle/cancel that batch's waiters; it must not affect this slot.
        superseded: Option<EvictedTask>,
        /// LRU eviction only drops cache retention. A current task still
        /// completes its live rows even though it can no longer populate the cache.
        evicted: Option<EvictedTask>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EvictedTask {
    pub(crate) sha256: String,
    pub(crate) task_token: u64,
}

/// SHA cache with a batch-owned pending slot and token-checked settlement.
/// Stale completion/failure cannot overwrite or remove a replacement task.
pub(crate) struct ModelMeshCache {
    slots: BTreeMap<String, CacheSlot>,
    recency: VecDeque<String>,
    next_task_token: u64,
}

impl Default for ModelMeshCache {
    fn default() -> Self {
        Self {
            slots: BTreeMap::new(),
            recency: VecDeque::new(),
            next_task_token: 1,
        }
    }
}

impl ModelMeshCache {
    pub(crate) fn claim(
        &mut self,
        sha256: &str,
        batch: &ModelBatchIdentity,
    ) -> Result<MeshCacheClaim, String> {
        if let Some(slot) = self.slots.get(sha256) {
            match slot {
                CacheSlot::Ready(mesh) => {
                    let mesh = mesh.clone();
                    self.touch(sha256);
                    return Ok(MeshCacheClaim::Reuse(mesh));
                }
                CacheSlot::Pending {
                    batch: pending_batch,
                    task_token,
                } if pending_batch.same_batch(batch) => {
                    let task_token = *task_token;
                    self.touch(sha256);
                    return Ok(MeshCacheClaim::JoinPending { task_token });
                }
                CacheSlot::Pending { .. } => {}
            }
        }

        let task_token = self.allocate_task_token()?;
        let superseded = match self.slots.get(sha256) {
            Some(CacheSlot::Pending { task_token, .. }) => Some(EvictedTask {
                sha256: sha256.to_owned(),
                task_token: *task_token,
            }),
            _ => None,
        };
        let evicted = self.make_room_for(sha256);
        self.slots.insert(
            sha256.to_owned(),
            CacheSlot::Pending {
                batch: batch.clone(),
                task_token,
            },
        );
        self.touch(sha256);
        Ok(MeshCacheClaim::Start {
            task_token,
            superseded,
            evicted,
        })
    }

    pub(crate) fn complete(
        &mut self,
        sha256: &str,
        task_token: u64,
        mesh: Rc<ValidatedMesh>,
    ) -> bool {
        let current = matches!(self.slots.get(sha256),
            Some(CacheSlot::Pending { task_token: current, .. }) if *current == task_token);
        if current {
            self.slots.insert(sha256.to_owned(), CacheSlot::Ready(mesh));
            self.touch(sha256);
        }
        current
    }

    /// A failed current task is removed so the normal retry can claim a new
    /// task. A superseded task has no authority to evict the replacement.
    pub(crate) fn fail(&mut self, sha256: &str, task_token: u64) -> bool {
        let current = matches!(self.slots.get(sha256),
            Some(CacheSlot::Pending { task_token: current, .. }) if *current == task_token);
        if current {
            self.slots.remove(sha256);
            self.recency.retain(|key| key != sha256);
        }
        current
    }

    fn allocate_task_token(&mut self) -> Result<u64, String> {
        let token = self.next_task_token;
        self.next_task_token = token
            .checked_add(1)
            .ok_or_else(|| "Model decode task identity exhausted".to_owned())?;
        Ok(token)
    }

    fn make_room_for(&mut self, sha256: &str) -> Option<EvictedTask> {
        if self.slots.contains_key(sha256) || self.slots.len() < MESH_CACHE_CAPACITY {
            return None;
        }
        let victim = self.recency.pop_front()?;
        let slot = self.slots.remove(&victim)?;
        match slot {
            CacheSlot::Pending { task_token, .. } => Some(EvictedTask {
                sha256: victim,
                task_token,
            }),
            CacheSlot::Ready(_) => None,
        }
    }

    fn touch(&mut self, sha256: &str) {
        self.recency.retain(|key| key != sha256);
        self.recency.push_back(sha256.to_owned());
    }
}

impl ModelBatchIdentity {
    fn same_batch(&self, other: &Self) -> bool {
        self.owner.same_owner(&other.owner)
            && self.accepted_revision == other.accepted_revision
            && self.batch_generation == other.batch_generation
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeliveredModel {
    pub(crate) id: String,
    pub(crate) mesh: Rc<ValidatedMesh>,
    /// Core-resolved module placement in the renderer's accepted board frame.
    pub(crate) matrix: Option<[f64; 16]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelFailure {
    pub(crate) reference: String,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ModelDeliveryRows {
    pub(crate) delivered: Vec<DeliveredModel>,
    pub(crate) pending: Vec<String>,
    pub(crate) failures: Vec<ModelFailure>,
}

/// Settle a Layout model-delivery result only against the exact preview that
/// started it. `current` is supplied by Runtime after the delivery future ends;
/// it must return only a preview whose complete source owner is current.
pub(crate) async fn settle_layout_model_delivery<F, C, R>(
    expected: Rc<LayoutPreviewSnapshot>,
    delivery: F,
    current: C,
    report: R,
) where
    F: Future<Output = Result<(), String>>,
    C: Fn() -> Option<Rc<LayoutPreviewSnapshot>>,
    R: FnOnce(String),
{
    if let Err(error) = delivery.await
        && current().is_some_and(|preview| preview.same_live_source(&expected))
    {
        report(error);
    }
}

impl PartialEq for ModelDeliveryRows {
    fn eq(&self, other: &Self) -> bool {
        self.pending == other.pending
            && self.failures == other.failures
            && self.delivered.len() == other.delivered.len()
            && self
                .delivered
                .iter()
                .zip(&other.delivered)
                .all(|(left, right)| {
                    left.id == right.id
                        && left.matrix == right.matrix
                        && Rc::ptr_eq(&left.mesh, &right.mesh)
                })
    }
}

/// Merge successful/error outcomes without substituting asset or reference
/// IDs for renderer model IDs. Rows preserve the preview's order. Absent
/// outcomes remain pending instead of being misreported as missing; terminal
/// provider/asset errors must be supplied explicitly.
pub(crate) fn merge_model_rows(
    models: &[PcbModel],
    outcomes: &BTreeMap<String, Result<Rc<ValidatedMesh>, String>>,
) -> ModelDeliveryRows {
    let mut rows = ModelDeliveryRows::default();
    for model in models {
        match outcomes.get(&model.id) {
            Some(Ok(mesh)) => rows.delivered.push(DeliveredModel {
                id: model.id.clone(),
                mesh: mesh.clone(),
                matrix: None,
            }),
            Some(Err(reason)) => rows.failures.push(ModelFailure {
                reference: model.reference.clone(),
                reason: reason.clone(),
            }),
            None => rows.pending.push(model.id.clone()),
        }
    }
    rows
}

/// Resolve a mounted-module model's direct asset identity using the same
/// document-first and packaged-provider semantics as preview path assets.
pub(crate) fn select_model_asset_id(
    asset_id: &str,
    document_assets: &[Asset],
    packaged_asset: impl FnOnce(&str) -> Option<ResolvedModelAsset>,
) -> AssetSelection {
    if let Some(asset) = document_assets.iter().find(|asset| asset.id == asset_id) {
        return AssetSelection::Archived(ResolvedModelAsset {
            id: asset.id.clone(),
            sha256: asset.sha256.clone(),
            filename: asset.name.clone(),
            source: ModelAssetSource::Document,
        });
    }
    packaged_asset(asset_id)
        .map(AssetSelection::Packaged)
        .unwrap_or_else(|| {
            if asset_id.starts_with("ergogen:model:") {
                AssetSelection::MissingBundledProvider {
                    asset_id: asset_id.to_owned(),
                }
            } else {
                AssetSelection::MissingDocumentAsset {
                    asset_id: asset_id.to_owned(),
                }
            }
        })
}

type TaskKey = (String, u64);
type PendingModelWaiters = BTreeMap<TaskKey, Vec<Rc<WaitCell<ModelResult>>>>;
type ModelResult = Result<Rc<ValidatedMesh>, String>;
type DeliveryTask = Pin<Box<dyn Future<Output = ()> + 'static>>;

struct WaitState<T> {
    value: Option<T>,
    wakers: Vec<Waker>,
}

impl<T> Default for WaitState<T> {
    fn default() -> Self {
        Self {
            value: None,
            wakers: Vec::new(),
        }
    }
}

struct WaitCell<T> {
    state: std::cell::RefCell<WaitState<T>>,
}

struct WaitCellFuture<T> {
    cell: Rc<WaitCell<T>>,
}

impl<T> WaitCell<T> {
    fn new() -> Self {
        Self {
            state: std::cell::RefCell::new(WaitState::default()),
        }
    }

    fn settle(&self, value: T) {
        let mut state = self.state.borrow_mut();
        if state.value.is_some() {
            return;
        }
        state.value = Some(value);
        for waker in state.wakers.drain(..) {
            waker.wake();
        }
    }
}

impl<T: Clone> Future for WaitCellFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.cell.state.borrow_mut();
        if let Some(value) = &state.value {
            return Poll::Ready(value.clone());
        }
        if !state
            .wakers
            .iter()
            .any(|waker| waker.will_wake(context.waker()))
        {
            state.wakers.push(context.waker().clone());
        }
        Poll::Pending
    }
}

/// Runs the model rows for one accepted preview concurrently, then publishes
/// the completed Promise.all-style row snapshot in preview order. Meshes are
/// cached by verified SHA; model rows retain their distinct renderer IDs.
#[derive(Default)]
pub(crate) struct ModelDeliveryAdapter {
    cache: Rc<std::cell::RefCell<ModelMeshCache>>,
    pending: Rc<std::cell::RefCell<PendingModelWaiters>>,
}

impl ModelDeliveryAdapter {
    pub(crate) async fn deliver_models(
        &self,
        preview_revision: u64,
        models: &[PcbModel],
        selections: &BTreeMap<String, AssetSelection>,
        ports: &ModelDeliveryPorts,
        batch: &ModelBatchIdentity,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Option<ModelDeliveryRows> {
        if !batch.is_current(batch.owner(), preview_revision, batch.batch_generation)
            || !is_current()
        {
            return None;
        }

        let mut direct = BTreeMap::<String, ModelResult>::new();
        let mut completed_by_sha = BTreeMap::<String, ModelResult>::new();
        let mut by_sha = BTreeMap::<String, Vec<String>>::new();
        let mut receivers = BTreeMap::<String, WaitCellFuture<ModelResult>>::new();
        let mut tasks = Vec::<DeliveryTask>::new();

        for model in models {
            if !is_current() {
                return None;
            }
            let Some(selection) = selections.get(&model.id) else {
                direct.insert(
                    model.id.clone(),
                    Err("No model asset mapping was provided".into()),
                );
                continue;
            };
            let asset = match selection {
                AssetSelection::Archived(asset) | AssetSelection::Packaged(asset) => asset.clone(),
                AssetSelection::MissingDocumentAsset { asset_id } => {
                    direct.insert(
                        model.id.clone(),
                        Err(format!("Document model asset {asset_id} is missing")),
                    );
                    continue;
                }
                AssetSelection::MissingBundledProvider { asset_id } => {
                    direct.insert(
                        model.id.clone(),
                        Err(format!(
                            "Bundled model provider is unavailable for {asset_id}"
                        )),
                    );
                    continue;
                }
                AssetSelection::NoAssetId => {
                    direct.insert(model.id.clone(), Err("No model asset ID is mapped".into()));
                    continue;
                }
            };
            let format = match ModelFormat::from_filename(&asset.filename) {
                Ok(format) => format,
                Err(error) => {
                    direct.insert(model.id.clone(), Err(error));
                    continue;
                }
            };

            if let Some(ids) = by_sha.get_mut(&asset.sha256) {
                ids.push(model.id.clone());
                continue;
            }

            let claim = match self.cache.borrow_mut().claim(&asset.sha256, batch) {
                Ok(claim) => claim,
                Err(error) => {
                    direct.insert(model.id.clone(), Err(error));
                    continue;
                }
            };
            let (task_token, start_task) = match claim {
                MeshCacheClaim::Reuse(mesh) => {
                    completed_by_sha.insert(asset.sha256.clone(), Ok(mesh));
                    by_sha.insert(asset.sha256.clone(), vec![model.id.clone()]);
                    continue;
                }
                MeshCacheClaim::JoinPending { task_token } => (task_token, false),
                MeshCacheClaim::Start {
                    task_token,
                    superseded,
                    evicted,
                } => {
                    if let Some(task) = superseded {
                        settle_pending(
                            &self.pending,
                            &task,
                            Err("Model request was superseded".into()),
                        );
                    }
                    if let Some(task) = evicted {
                        settle_pending(
                            &self.pending,
                            &task,
                            Err("Model request was evicted from the bounded cache".into()),
                        );
                    }
                    (task_token, true)
                }
            };

            let task_key = (asset.sha256.clone(), task_token);
            let cell = Rc::new(WaitCell::new());
            self.pending
                .borrow_mut()
                .entry(task_key.clone())
                .or_default()
                .push(cell.clone());
            receivers.insert(asset.sha256.clone(), WaitCellFuture { cell });
            by_sha.insert(asset.sha256.clone(), vec![model.id.clone()]);

            if start_task {
                let cache = self.cache.clone();
                let pending = self.pending.clone();
                let ports = ports.clone();
                let batch_owner = batch.owner().clone();
                let expected_sha = asset.sha256.clone();
                let current = is_current.clone();
                tasks.push(Box::pin(async move {
                    let result = if !current() {
                        Err("Model request became stale before loading".into())
                    } else {
                        load_and_decode(&ports, &asset, format, batch_owner, current.clone()).await
                    };
                    let result = if current() {
                        match result {
                            Ok(mesh) => {
                                let mesh = Rc::new(mesh);
                                if cache.borrow_mut().complete(
                                    &expected_sha,
                                    task_token,
                                    mesh.clone(),
                                ) {
                                    Ok(mesh)
                                } else {
                                    Err("Model request was superseded before completion".into())
                                }
                            }
                            Err(error) => {
                                cache.borrow_mut().fail(&expected_sha, task_token);
                                Err(error)
                            }
                        }
                    } else {
                        cache.borrow_mut().fail(&expected_sha, task_token);
                        Err("Model request became stale".into())
                    };
                    settle_task(&pending, &task_key, result);
                }));
            }
        }

        join_all(tasks).await;

        let mut outcomes = direct;
        for (sha, receiver) in receivers {
            let result = receiver.await;
            if !is_current() {
                return None;
            }
            completed_by_sha.insert(sha, result);
        }
        if !is_current() {
            return None;
        }
        for (sha, ids) in by_sha {
            if let Some(result) = completed_by_sha.get(&sha) {
                for id in ids {
                    outcomes.insert(id, result.clone());
                }
            }
        }
        Some(merge_model_rows(models, &outcomes))
    }

    /// Deliver Core-resolved mounted-module placements through the same
    /// verification, format decoder, SHA cache, and batch liveness checks used
    /// by ordinary preview models. The returned rows retain Core's stable IDs
    /// and carry its authoritative affine transform to the renderer adapter.
    pub(crate) async fn deliver_module_placements(
        &self,
        preview_revision: u64,
        placements: &[ModuleModelPlacement],
        selections: &BTreeMap<String, AssetSelection>,
        ports: &ModelDeliveryPorts,
        batch: &ModelBatchIdentity,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Option<ModelDeliveryRows> {
        let models = placements
            .iter()
            .map(|placement| PcbModel {
                id: placement.id.clone(),
                reference: placement.id.clone(),
                path: placement.asset_id.clone(),
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
            })
            .collect::<Vec<_>>();
        let mut rows = self
            .deliver_models(
                preview_revision,
                &models,
                selections,
                ports,
                batch,
                is_current,
            )
            .await?;
        for model in &mut rows.delivered {
            model.matrix = placements
                .iter()
                .find(|placement| placement.id == model.id)
                .map(|placement| placement.matrix);
        }
        Some(rows)
    }
}

async fn load_and_decode(
    ports: &ModelDeliveryPorts,
    asset: &ResolvedModelAsset,
    format: ModelFormat,
    owner: ModelOwnerIdentity,
    is_current: Rc<dyn Fn() -> bool>,
) -> Result<ValidatedMesh, String> {
    let bytes = ports
        .load(asset)
        .await?
        .ok_or_else(|| format!("Model asset {} is not stored", asset.id))?;
    if !is_current() {
        return Err("Model request became stale after loading bytes".into());
    }
    let mesh = ports.decode(format, bytes, owner).await?;
    if !is_current() {
        return Err("Model request became stale after decoding".into());
    }
    Ok(mesh)
}

fn settle_pending(
    pending: &std::cell::RefCell<BTreeMap<TaskKey, Vec<Rc<WaitCell<ModelResult>>>>>,
    task: &EvictedTask,
    result: ModelResult,
) {
    settle_task(pending, &(task.sha256.clone(), task.task_token), result);
}

fn settle_task(
    pending: &std::cell::RefCell<BTreeMap<TaskKey, Vec<Rc<WaitCell<ModelResult>>>>>,
    key: &TaskKey,
    result: ModelResult,
) {
    if let Some(waiters) = pending.borrow_mut().remove(key) {
        for waiter in waiters {
            waiter.settle(result.clone());
        }
    }
}

async fn join_all(futures: Vec<DeliveryTask>) {
    let mut futures = futures.into_iter().map(Some).collect::<Vec<_>>();
    poll_fn(move |context| {
        let mut all_ready = true;
        for future in &mut futures {
            if let Some(task) = future.as_mut() {
                if task.as_mut().poll(context).is_ready() {
                    *future = None;
                } else {
                    all_ready = false;
                }
            }
        }
        if all_ready {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await
}

/// Resolve all row asset identities without inventing byte locations. The
/// document is supplied as a captured snapshot; no separate asset registry is
/// retained in this module.
pub(crate) fn resolve_preview_assets(
    models: &[PcbModel],
    reference: Option<&BoardReference>,
    native_path_assets: &BTreeMap<String, String>,
    document: &ProjectDoc,
    ergogen_model_asset_id: impl Fn(&str) -> Option<String>,
    packaged_asset: impl Fn(&str) -> Option<ResolvedModelAsset>,
) -> Vec<(String, AssetSelection)> {
    models
        .iter()
        .map(|model| {
            let selection = select_model_asset(
                &model.path,
                reference,
                native_path_assets,
                &document.assets,
                |path| ergogen_model_asset_id(path),
            );
            let selection = match selection {
                AssetSelection::MissingBundledProvider { asset_id } => packaged_asset(&asset_id)
                    .map(AssetSelection::Packaged)
                    .unwrap_or(AssetSelection::MissingBundledProvider { asset_id }),
                selection => selection,
            };
            (model.id.clone(), selection)
        })
        .collect()
}

/// Native preparation records paths by source asset ID because Core needs that
/// direction when materializing export artifacts. `PcbPreview` rows point the
/// other way (path -> model row), so invert only at this consumer boundary.
pub(crate) fn native_model_path_assets(
    source_paths_by_asset_id: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    source_paths_by_asset_id
        .iter()
        .map(|(asset_id, path)| (path.clone(), asset_id.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{Pose2, Side, Vec2, Vec3};

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
        assert_eq!(
            selection,
            AssetSelection::Archived(ResolvedModelAsset {
                id: "native".into(),
                sha256: "b".into(),
                filename: "native.wrl".into(),
                source: ModelAssetSource::Document,
            })
        );
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
            AssetSelection::Archived(ResolvedModelAsset {
                id: "attached".into(),
                sha256: "a".into(),
                filename: "attached.stl".into(),
                source: ModelAssetSource::Document,
            })
        );
        assert_eq!(
            select_model_asset("missing.stl", None, &BTreeMap::new(), &[], |_| {
                Some("ergogen:model:vendor/missing.stl".into())
            }),
            AssetSelection::MissingBundledProvider {
                asset_id: "ergogen:model:vendor/missing.stl".into()
            }
        );
    }

    #[test]
    fn accepted_kiswitch_preview_path_resolves_to_the_known_packaged_descriptor() {
        let path = "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Cherry_MX_PCB.stp";
        let id = "ergogen:model:kiswitch/SW_Cherry_MX_PCB.stp";
        let document = ProjectDoc::empty("case-models", "Case model path test");
        let resolved = resolve_preview_assets(
            &[model("mesh-row", "SW1", path)],
            None,
            &BTreeMap::new(),
            &document,
            |preview_path| (preview_path == path).then(|| id.to_owned()),
            |asset_id| {
                (asset_id == id).then(|| ResolvedModelAsset {
                    id: id.to_owned(),
                    sha256: "a".repeat(64),
                    filename: "SW_Cherry_MX_PCB.stp".to_owned(),
                    source: ModelAssetSource::Packaged {
                        url_path: "assets/ergogen-models/model-test.stp".to_owned(),
                    },
                })
            },
        );
        assert_eq!(
            resolved,
            vec![(
                "mesh-row".to_owned(),
                AssetSelection::Packaged(ResolvedModelAsset {
                    id: id.to_owned(),
                    sha256: "a".repeat(64),
                    filename: "SW_Cherry_MX_PCB.stp".to_owned(),
                    source: ModelAssetSource::Packaged {
                        url_path: "assets/ergogen-models/model-test.stp".to_owned(),
                    },
                })
            )]
        );
    }

    #[test]
    fn core_packaged_preview_path_joins_the_native_table_to_its_exact_descriptor() {
        let id = "ergogen:model:kiswitch/SW_Cherry_MX_PCB.stp";
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
    fn core_archived_preview_path_preserves_document_asset_precedence() {
        let source = BTreeMap::from([("asset-1".to_owned(), "models/hash.stl".to_owned())]);
        let native = native_model_path_assets(&source);
        assert_eq!(
            select_model_asset(
                "${KIPRJMOD}/models/hash.stl",
                None,
                &native,
                &[asset("asset-1", "ab", "part.stl")],
                |_| Some("ergogen:model:other.stl".into()),
            ),
            AssetSelection::Archived(ResolvedModelAsset {
                id: "asset-1".into(),
                sha256: "ab".into(),
                filename: "part.stl".into(),
                source: ModelAssetSource::Document,
            })
        );
    }

    #[test]
    fn native_export_path_table_is_inverted_for_preview_model_rows() {
        let source = BTreeMap::from([("asset-1".to_owned(), "models/hash.stl".to_owned())]);
        let native = native_model_path_assets(&source);
        assert_eq!(native["models/hash.stl"], "asset-1");
        assert_eq!(
            select_model_asset(
                "models/hash.stl",
                None,
                &native,
                &[asset("asset-1", "ab", "part.stl")],
                |_| None
            ),
            AssetSelection::Archived(ResolvedModelAsset {
                id: "asset-1".into(),
                sha256: "ab".into(),
                filename: "part.stl".into(),
                source: ModelAssetSource::Document,
            })
        );
    }

    #[test]
    fn verified_bytes_reject_wrong_digest_and_bounds() {
        let bytes = b"model".to_vec();
        let digest = sha256_hex(&bytes);
        let verified = VerifiedModelBytes::verify(bytes.clone(), &digest).unwrap();
        assert_eq!(verified.bytes(), bytes);
        assert_eq!(verified.sha256(), digest);
        assert!(VerifiedModelBytes::verify(bytes, "00").is_err());
        assert!(VerifiedModelBytes::verify(Vec::new(), &digest).is_err());
    }

    #[test]
    fn format_selection_is_case_insensitive_and_rejects_other_extensions() {
        assert_eq!(
            ModelFormat::from_filename("BODY.STL").unwrap(),
            ModelFormat::Stl
        );
        assert_eq!(
            ModelFormat::from_filename("body.WrL").unwrap(),
            ModelFormat::Wrl
        );
        assert_eq!(
            ModelFormat::from_filename("model.STP").unwrap(),
            ModelFormat::Step
        );
        assert!(ModelFormat::from_filename("body.obj").is_err());
    }

    #[test]
    fn mesh_validation_requires_complete_finite_matching_buffers() {
        assert!(ValidatedMesh::try_from(valid_arrays()).is_ok());
        let mut arrays = valid_arrays();
        arrays.positions.pop();
        assert!(ValidatedMesh::try_from(arrays).is_err());
        let mut arrays = valid_arrays();
        arrays.normals.pop();
        assert!(ValidatedMesh::try_from(arrays).is_err());
        let mut arrays = valid_arrays();
        arrays.positions[0] = f32::NAN;
        assert!(ValidatedMesh::try_from(arrays).is_err());
        let mut arrays = valid_arrays();
        arrays.colors = Some(vec![0.0; 8]);
        assert!(ValidatedMesh::try_from(arrays).is_err());
        let mut arrays = valid_arrays();
        arrays.colors = Some(vec![f32::INFINITY; 9]);
        assert!(ValidatedMesh::try_from(arrays).is_err());
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
        let mesh = Rc::new(ValidatedMesh::try_from(valid_arrays()).unwrap());
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
        let mesh = Rc::new(ValidatedMesh::try_from(valid_arrays()).unwrap());
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
        let expected_digest = digest.clone();
        let ports = ModelDeliveryPorts {
            load_verified_bytes: Rc::new(move |asset| {
                let bytes = bytes.clone();
                let expected_digest = expected_digest.clone();
                Box::pin(async move {
                    let sha = asset.sha256;
                    if sha != expected_digest {
                        return Err("unexpected SHA".into());
                    }
                    Ok(Some(VerifiedModelBytes::verify(bytes, &sha)?))
                })
            }),
            decode_stl: Rc::new(move |_| {
                decode_count.set(decode_count.get() + 1);
                Box::pin(async { Ok(valid_arrays()) })
            }),
            decode_wrl: Rc::new(|_| Box::pin(async { Ok(valid_arrays()) })),
            read_step: Rc::new(|_, _| Box::pin(async { Ok(valid_arrays()) })),
        };
        let mut assets = BTreeMap::new();
        for id in ["model-1", "model-2"] {
            assets.insert(
                id.into(),
                AssetSelection::Archived(ResolvedModelAsset {
                    id: "asset-1".into(),
                    sha256: digest.clone(),
                    filename: "switch.stl".into(),
                    source: ModelAssetSource::Document,
                }),
            );
        }
        let models = [
            model("model-1", "SW1", "switch.stl"),
            model("model-2", "SW2", "switch.stl"),
        ];
        let adapter = ModelDeliveryAdapter::default();

        let rows = block_on(adapter.deliver_models(
            1,
            &models,
            &assets,
            &ports,
            &batch(1),
            Rc::new(|| true),
        ))
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
        let expected_digest = digest.clone();
        let ports = ModelDeliveryPorts {
            load_verified_bytes: Rc::new(move |asset| {
                let bytes = bytes.clone();
                let expected_digest = expected_digest.clone();
                Box::pin(async move {
                    assert_eq!(asset.id, "module-mesh");
                    assert_eq!(asset.sha256, expected_digest);
                    Ok(Some(VerifiedModelBytes::verify(bytes, &asset.sha256)?))
                })
            }),
            decode_stl: Rc::new(|_| Box::pin(async { Ok(valid_arrays()) })),
            decode_wrl: Rc::new(|_| Box::pin(async { Ok(valid_arrays()) })),
            read_step: Rc::new(|_, _| Box::pin(async { Ok(valid_arrays()) })),
        };
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
            Rc::new(|| true),
        ))
        .expect("current module delivery should complete");

        assert_eq!(rows.delivered.len(), 1);
        assert_eq!(rows.delivered[0].id, placement.id);
        assert_eq!(rows.delivered[0].matrix, Some(placement.matrix));
        assert!(rows.failures.is_empty());
        assert!(rows.pending.is_empty());
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
}
