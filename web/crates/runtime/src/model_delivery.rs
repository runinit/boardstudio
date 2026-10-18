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

pub type ModelFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + 'static>>;

/// The owner of model work. Renderer scene submissions intentionally are not
/// part of this identity: healthy model completions can outlive a newer scene
/// sequence while remaining attached to this exact Case projection.
#[derive(Clone)]
pub struct ModelOwnerIdentity {
    scope: Scope,
    snapshot_token: SnapshotToken,
    viewer_instance: u64,
    projection_generation: u64,
    source_owner: Weak<dyn ModelSourceLease>,
}

impl ModelOwnerIdentity {
    pub fn new(
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

    pub fn new_layout(
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

    pub fn new_parts(
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
    pub fn is_current_owner(
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

    pub fn is_current_layout_owner(
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

    pub fn is_current_parts_owner(
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
pub struct ModelBatchIdentity {
    owner: ModelOwnerIdentity,
    accepted_revision: u64,
    batch_generation: u64,
}

impl ModelBatchIdentity {
    pub fn new(owner: ModelOwnerIdentity, accepted_revision: u64, batch_generation: u64) -> Self {
        Self {
            owner,
            accepted_revision,
            batch_generation,
        }
    }

    pub fn is_current(
        &self,
        current_owner: &ModelOwnerIdentity,
        current_revision: u64,
        current_batch_generation: u64,
    ) -> bool {
        self.owner.same_owner(current_owner)
            && self.accepted_revision == current_revision
            && self.batch_generation == current_batch_generation
    }

    pub fn owner(&self) -> &ModelOwnerIdentity {
        &self.owner
    }
}

/// Resolved document asset identity. The path is deliberately not used as a
/// byte-store key; only `sha256` crosses the Runtime storage port.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedModelAsset {
    pub id: String,
    pub sha256: String,
    pub filename: String,
    pub source: ModelAssetSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelAssetSource {
    Document,
    Packaged { url_path: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetSelection {
    Archived(ResolvedModelAsset),
    Packaged(ResolvedModelAsset),
    MissingDocumentAsset { asset_id: String },
    MissingBundledProvider { asset_id: String },
    NoAssetId,
}

/// Apply the same source precedence as React's AssemblyPreview: attached
/// BoardReference mapping, native preview path table, then version-matched
/// generator path helper. The helper is injected because its packaged source is
/// owned by the layout-generator build, not duplicated here.
pub fn select_model_asset(
    model_path: &str,
    reference: Option<&BoardReference>,
    native_path_assets: &BTreeMap<String, String>,
    document_assets: &[Asset],
    bundled_model_asset_id: impl FnOnce(&str) -> Option<String>,
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
        .or_else(|| bundled_model_asset_id(model_path));
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

    if asset_id.starts_with("bundled-model:") {
        AssetSelection::MissingBundledProvider { asset_id }
    } else {
        AssetSelection::MissingDocumentAsset { asset_id }
    }
}

/// Bytes crossing the page-model adapter must be revalidated even if they
/// came from the current-import memory map rather than BrowserStore.
#[derive(Clone, Debug)]
pub struct VerifiedModelBytes {
    bytes: Rc<[u8]>,
    sha256: String,
}

impl VerifiedModelBytes {
    pub fn verify(bytes: Vec<u8>, expected_sha256: &str) -> Result<Self, String> {
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

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn sha256(&self) -> &str {
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
pub enum ModelFormat {
    Stl,
    Wrl,
    Step,
}

impl ModelFormat {
    pub fn from_filename(filename: &str) -> Result<Self, String> {
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
pub struct ModelDeliveryPorts {
    pub load_verified_bytes:
        Rc<dyn Fn(ResolvedModelAsset) -> ModelFuture<Option<VerifiedModelBytes>>>,
    pub decode_stl: Rc<dyn Fn(VerifiedModelBytes) -> ModelFuture<MeshArrays>>,
    pub decode_wrl: Rc<dyn Fn(VerifiedModelBytes) -> ModelFuture<MeshArrays>>,
    pub read_step: Rc<dyn Fn(VerifiedModelBytes, ModelOwnerIdentity) -> ModelFuture<MeshArrays>>,
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
    pub async fn load(
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

    pub async fn decode(
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
pub struct MeshArrays {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub colors: Option<Vec<f32>>,
    pub step_bounds: Option<boardstudio_web_host::cad_jobs::CadBounds>,
    pub vendor_step: Option<Rc<boardstudio_web_host::cad_jobs::CadVendorStep>>,
}

#[derive(Clone, Debug)]
pub struct ValidatedMesh {
    pub positions: Rc<[f32]>,
    pub normals: Rc<[f32]>,
    pub colors: Option<Rc<[f32]>>,
    pub step_bounds: Option<boardstudio_web_host::cad_jobs::CadBounds>,
    pub vendor_step: Option<Rc<boardstudio_web_host::cad_jobs::CadVendorStep>>,
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
        if let Some(vendor_step) = &arrays.vendor_step {
            let bounds = arrays
                .step_bounds
                .as_ref()
                .ok_or_else(|| "STEP model asset omitted bounds".to_owned())?;
            boardstudio_web_host::cad_jobs::validate_vendor_step(
                &vendor_step.source_sha256,
                vendor_step.import_options,
                &boardstudio_web_host::cad_jobs::CadMesh {
                    positions: arrays.positions.clone(),
                    normals: arrays.normals.clone(),
                },
                bounds,
                vendor_step,
            )?;
        }
        Ok(Self {
            positions: Rc::from(arrays.positions),
            normals: Rc::from(arrays.normals),
            colors: arrays.colors.map(Rc::from),
            step_bounds: arrays.step_bounds,
            vendor_step: arrays.vendor_step,
        })
    }
}

pub fn derived_step_cache_key(
    sha256: &str,
    options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
) -> Result<String, String> {
    if !boardstudio_web_host::cad_jobs::valid_source_sha256(sha256) {
        return Err("STEP cache key requires a canonical 64-hex SHA-256".into());
    }
    let options = options.validate()?;
    let mut key = match options.preview_repair_budget_mm {
        None => sha256.to_owned(),
        Some(budget) => format!("{sha256}:repair={:016x}", budget.to_bits()),
    };
    if let Some(target) = options.target_deflection_mm {
        key.push_str(&format!(":deflection={:016x}", target.to_bits()));
    }
    Ok(key)
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
pub enum MeshCacheClaim {
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
pub struct EvictedTask {
    pub sha256: String,
    pub task_token: u64,
}

/// SHA cache with a batch-owned pending slot and token-checked settlement.
/// Stale completion/failure cannot overwrite or remove a replacement task.
pub struct ModelMeshCache {
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
    pub fn claim_with_step_options(
        &mut self,
        sha256: &str,
        options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        batch: &ModelBatchIdentity,
    ) -> Result<MeshCacheClaim, String> {
        let key = derived_step_cache_key(sha256, options)?;
        self.claim(&key, batch)
    }

    pub fn complete_with_step_options(
        &mut self,
        sha256: &str,
        options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        task_token: u64,
        mesh: Rc<ValidatedMesh>,
    ) -> Result<bool, String> {
        let key = derived_step_cache_key(sha256, options)?;
        if let Some(vendor_step) = &mesh.vendor_step
            && (vendor_step.source_sha256 != sha256
                || !vendor_step.import_options.same_identity(options))
        {
            return Err("STEP asset identity does not match derived cache key".into());
        }
        Ok(self.complete(&key, task_token, mesh))
    }

    pub fn fail_with_step_options(
        &mut self,
        sha256: &str,
        options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        task_token: u64,
    ) -> Result<bool, String> {
        let key = derived_step_cache_key(sha256, options)?;
        Ok(self.fail(&key, task_token))
    }

    pub fn claim(
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

    pub fn complete(&mut self, sha256: &str, task_token: u64, mesh: Rc<ValidatedMesh>) -> bool {
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
    pub fn fail(&mut self, sha256: &str, task_token: u64) -> bool {
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
pub struct DeliveredModel {
    pub id: String,
    pub mesh: Rc<ValidatedMesh>,
    /// Core-resolved module placement in the renderer's accepted board frame.
    pub matrix: Option<[f64; 16]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelFailure {
    pub reference: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct ModelDeliveryRows {
    pub delivered: Vec<DeliveredModel>,
    pub pending: Vec<String>,
    pub failures: Vec<ModelFailure>,
}

/// Settle a Layout model-delivery result only against the exact preview that
/// started it. `current` is supplied by Runtime after the delivery future ends;
/// it must return only a preview whose complete source owner is current.
pub async fn settle_layout_model_delivery<F, C, R>(
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
pub fn merge_model_rows(
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
pub fn select_model_asset_id(
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
            if asset_id.starts_with("bundled-model:") {
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
pub struct ModelDeliveryAdapter {
    cache: Rc<std::cell::RefCell<ModelMeshCache>>,
    pending: Rc<std::cell::RefCell<PendingModelWaiters>>,
}

impl ModelDeliveryAdapter {
    pub async fn deliver_models(
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
    pub async fn deliver_module_placements(
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
pub fn resolve_preview_assets(
    models: &[PcbModel],
    reference: Option<&BoardReference>,
    native_path_assets: &BTreeMap<String, String>,
    document: &ProjectDoc,
    bundled_model_asset_id: impl Fn(&str) -> Option<String>,
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
                |path| bundled_model_asset_id(path),
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

pub const TIGHT_COLLISION_ACCURACY_MM: f64 = 0.01;
pub const IDENTITY_ROW_TRANSFORM_F64: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

#[derive(Clone, Debug, PartialEq)]
pub enum CadCollisionSubject {
    VendorModel {
        id: String,
        reference: String,
        row_transform_f64: [f64; 16],
        vendor_step: Option<Rc<boardstudio_web_host::cad_jobs::CadVendorStep>>,
    },
    GeneratedCaseBody {
        id: String,
        name: String,
        row_transform_f64: [f64; 16],
        analysis: Option<Rc<boardstudio_web_host::cad_jobs::CadGeneratedAnalysis>>,
    },
}

impl CadCollisionSubject {
    pub fn id(&self) -> &str {
        match self {
            Self::VendorModel { id, .. } | Self::GeneratedCaseBody { id, .. } => id,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::VendorModel { reference, .. } => reference,
            Self::GeneratedCaseBody { name, .. } => name,
        }
    }

    pub fn operand(&self) -> Result<boardstudio_web_host::cad_jobs::CadCollisionOperand, String> {
        use boardstudio_web_host::cad_jobs::{CadCollisionOperand, CadStepImportOptions};
        match self {
            Self::VendorModel {
                id,
                row_transform_f64,
                vendor_step: Some(asset),
                ..
            } => Ok(CadCollisionOperand {
                model_id: id.clone(),
                source_sha256: Some(asset.source_sha256.clone()),
                generated_geometry_identity: None,
                import_options: asset.import_options,
                row_transform_f64: *row_transform_f64,
            }),
            Self::GeneratedCaseBody {
                id,
                row_transform_f64,
                analysis: Some(asset),
                ..
            } => Ok(CadCollisionOperand {
                model_id: id.clone(),
                source_sha256: None,
                generated_geometry_identity: Some(asset.geometry_identity.clone()),
                import_options: CadStepImportOptions::default(),
                row_transform_f64: *row_transform_f64,
            }),
            _ => Err(format!("{} has no source analysis mesh", self.label())),
        }
    }

    pub fn mesh(&self) -> Option<(&[f64], &[u32])> {
        match self {
            Self::VendorModel {
                vendor_step: Some(asset),
                ..
            } => Some((
                &asset.surface_positions_mm_f64,
                &asset.triangle_face_occurrence_indices,
            )),
            Self::GeneratedCaseBody {
                analysis: Some(asset),
                ..
            } => Some((
                &asset.surface_positions_mm_f64,
                &asset.triangle_face_occurrence_indices,
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CadCollisionPairFinding {
    pub left_id: String,
    pub left_label: String,
    pub right_id: String,
    pub right_label: String,
    pub result: CadCollisionResult,
}

impl CadCollisionPairFinding {
    pub fn provenance_details(&self) -> [String; 2] {
        let describe = |id: &str, label: &str, provenance: &Option<CadSubjectFaceProvenance>| {
            let identity = match provenance {
                Some(CadSubjectFaceProvenance::Vendor(face)) => format!(
                    "STEP representation #{}; source solid #{}; shell {}; solid occurrence {}; face occurrence {}; source face #{}; support #{} ({}); triangle {}",
                    face.representation_id,
                    face.solid_id,
                    face.shell_index,
                    face.solid_occurrence_index,
                    face.face_occurrence_index,
                    face.face_id,
                    face.support_id,
                    face.support_kind,
                    face.triangle_index
                ),
                Some(CadSubjectFaceProvenance::Generated {
                    body_id,
                    triangle_index,
                    face_occurrence_index,
                    solid_occurrence_index,
                    source,
                }) => format!(
                    "generated body {body_id}; source solid index {}; shell index {}; solid occurrence {solid_occurrence_index}; face occurrence {face_occurrence_index}; source face index {}; support {}; triangle {triangle_index}",
                    source.solid_index, source.shell_index, source.face_index, source.support_kind
                ),
                None => "source face/occurrence identity unresolved".to_owned(),
            };
            format!("{label} [{id}]: {identity}")
        };
        [
            describe(
                &self.left_id,
                &self.left_label,
                &self.result.left_provenance,
            ),
            describe(
                &self.right_id,
                &self.right_label,
                &self.result.right_provenance,
            ),
        ]
    }

    pub fn summary(&self, clearance_threshold_mm: f64) -> String {
        use boardstudio_web_host::cad_jobs::CadCollisionObservation;

        let pair_label = format!(
            "{} [{}] ↔ {} [{}]",
            self.left_label, self.left_id, self.right_label, self.right_id
        );
        let witness = match &self.result.observation {
            Some(CadCollisionObservation::SurfaceIntersecting {
                contact_witness: Some(witness),
                ..
            }) => format!(
                "surface intersection near ({:.2}, {:.2}, {:.2}) mm",
                witness.point1_mm[0], witness.point1_mm[1], witness.point1_mm[2]
            ),
            Some(CadCollisionObservation::SurfaceIntersecting { .. }) => {
                "surface intersection observed".to_owned()
            }
            Some(CadCollisionObservation::SurfaceSeparated {
                surface_distance_mm,
                closest_points,
            }) => format!(
                "surface separation {:.3} mm between ({:.2}, {:.2}, {:.2}) and ({:.2}, {:.2}, {:.2}) mm",
                surface_distance_mm,
                closest_points.point1_mm[0],
                closest_points.point1_mm[1],
                closest_points.point1_mm[2],
                closest_points.point2_mm[0],
                closest_points.point2_mm[1],
                closest_points.point2_mm[2]
            ),
            Some(CadCollisionObservation::Uncertain { reason, .. }) => {
                format!("uncertain observation ({reason})")
            }
            None => "no mesh observation".to_owned(),
        };
        match &self.result.assessment {
            CadCollisionAssessment::SupportedBand {
                lower_mm,
                upper_mm,
                relation,
            } => {
                let status = match relation {
                    CadCollisionRelation::Violation => "Violation",
                    CadCollisionRelation::Clear => "Clear",
                    CadCollisionRelation::Uncertain => "Uncertain",
                };
                format!(
                    "{pair_label}: {status} [{lower_mm:.3}, {upper_mm:.3}] mm vs {clearance_threshold_mm:.3} mm ({witness})"
                )
            }
            CadCollisionAssessment::Unknown {
                reasons,
                refinement_requested,
            } => {
                let refine = if *refinement_requested {
                    "; requested accuracy remains unqualified"
                } else {
                    ""
                };
                format!(
                    "{pair_label}: Uncertain ({witness}; {}{refine})",
                    reasons.join(", ")
                )
            }
        }
    }
}

pub fn physical_pcb_model_row_transform_f64(
    model: &PcbModel,
    board_thickness: f64,
    reference: Option<&BoardReference>,
    has_mechanical_stack: bool,
) -> Result<[f64; 16], String> {
    let mul = |a: [f64; 16], b: [f64; 16]| -> [f64; 16] {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = a[row] * b[col * 4]
                    + a[4 + row] * b[col * 4 + 1]
                    + a[8 + row] * b[col * 4 + 2]
                    + a[12 + row] * b[col * 4 + 3];
            }
        }
        out
    };
    let translate = |x: f64, y: f64, z: f64| -> [f64; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0,
        ]
    };
    let scale = |x: f64, y: f64, z: f64| -> [f64; 16] {
        [
            x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };
    let rot_x = |rad: f64| -> [f64; 16] {
        let (s, c) = rad.sin_cos();
        [
            1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };
    let rot_y = |rad: f64| -> [f64; 16] {
        let (s, c) = rad.sin_cos();
        [
            c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };
    let rot_z = |rad: f64| -> [f64; 16] {
        let (s, c) = rad.sin_cos();
        [
            c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    };

    let base_z_delta = -if has_mechanical_stack {
        board_thickness
    } else {
        0.0
    };
    let reference_transform = reference
        .filter(|item| item.enabled)
        .map(|item| {
            mul(
                translate(item.pose.at.x, item.pose.at.y, item.elevation),
                rot_z(item.pose.rotation.to_radians()),
            )
        })
        .unwrap_or(IDENTITY_ROW_TRANSFORM_F64);
    let pcb_transform = mul(reference_transform, translate(0.0, 0.0, base_z_delta));

    let back = model.side == Side::Back;
    let side = if back {
        mul(rot_y(std::f64::consts::PI), rot_z(std::f64::consts::PI))
    } else {
        IDENTITY_ROW_TRANSFORM_F64
    };
    let model_transform = [
        translate(
            model.pose.at.x,
            model.pose.at.y,
            if back { 0.0 } else { board_thickness },
        ),
        rot_z(model.pose.rotation.to_radians()),
        side,
        translate(model.offset.x, model.offset.y, model.offset.z),
        rot_z(-model.rotation.z.to_radians()),
        rot_y(-model.rotation.y.to_radians()),
        rot_x(-model.rotation.x.to_radians()),
        scale(model.scale.x, model.scale.y, model.scale.z),
    ]
    .into_iter()
    .fold(IDENTITY_ROW_TRANSFORM_F64, mul);

    let matrix = mul(pcb_transform, model_transform);
    boardstudio_web_host::cad_jobs::validate_affine_row_transform(&matrix)?;
    Ok(matrix)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadCollisionRelation {
    Clear,
    Violation,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CadCollisionAssessment {
    SupportedBand {
        lower_mm: f64,
        upper_mm: f64,
        relation: CadCollisionRelation,
    },
    Unknown {
        reasons: Vec<String>,
        refinement_requested: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CadResolvedFaceProvenance {
    pub triangle_index: u32,
    pub face_occurrence_index: u32,
    pub solid_occurrence_index: u32,
    pub representation_id: String,
    pub solid_id: String,
    pub shell_index: u32,
    pub face_id: String,
    pub support_id: String,
    pub support_kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CadSubjectFaceProvenance {
    Vendor(CadResolvedFaceProvenance),
    Generated {
        body_id: String,
        triangle_index: u32,
        face_occurrence_index: u32,
        solid_occurrence_index: u32,
        source: boardstudio_web_host::cad_jobs::CadGeneratedSourceFace,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CadCollisionResult {
    pub observation: Option<boardstudio_web_host::cad_jobs::CadCollisionObservation>,
    pub left_provenance: Option<CadSubjectFaceProvenance>,
    pub right_provenance: Option<CadSubjectFaceProvenance>,
    pub assessment: CadCollisionAssessment,
}

pub fn resolve_face_provenance(
    asset: &boardstudio_web_host::cad_jobs::CadVendorStep,
    provenance: &boardstudio_web_host::cad_jobs::CadTriangleProvenance,
) -> Option<CadResolvedFaceProvenance> {
    let expected = *asset
        .triangle_face_occurrence_indices
        .get(provenance.triangle_index as usize)?;
    if expected != provenance.face_occurrence_index {
        return None;
    }
    let occurrence = asset
        .face_occurrences
        .get(provenance.face_occurrence_index as usize)?;
    let source_face = asset
        .source_faces
        .get(occurrence.source_face_index as usize)?;
    Some(CadResolvedFaceProvenance {
        triangle_index: provenance.triangle_index,
        face_occurrence_index: provenance.face_occurrence_index,
        solid_occurrence_index: occurrence.solid_occurrence_index,
        representation_id: occurrence.representation_id.clone(),
        solid_id: source_face.solid_id.clone(),
        shell_index: source_face.shell_index,
        face_id: source_face.face_id.clone(),
        support_id: source_face.support_id.clone(),
        support_kind: source_face.support_kind.clone(),
    })
}

pub fn resolve_physical_model_row_transform(
    delivered: &DeliveredModel,
    explicit_pcb_row_transform: Option<[f64; 16]>,
) -> Result<[f64; 16], String> {
    if let Some(matrix) = delivered.matrix {
        boardstudio_web_host::cad_jobs::validate_affine_row_transform(&matrix)?;
        return Ok(matrix);
    }
    let matrix = explicit_pcb_row_transform.ok_or_else(|| {
        format!(
            "PCB model row {} has matrix: None and no explicit physical assembly transform was supplied",
            delivered.id
        )
    })?;
    boardstudio_web_host::cad_jobs::validate_affine_row_transform(&matrix)?;
    Ok(matrix)
}

pub fn resolve_subject_face_provenance(
    subject: &CadCollisionSubject,
    provenance: &boardstudio_web_host::cad_jobs::CadTriangleProvenance,
) -> Option<CadSubjectFaceProvenance> {
    match subject {
        CadCollisionSubject::VendorModel { vendor_step, .. } => {
            resolve_face_provenance(vendor_step.as_deref()?, provenance)
                .map(CadSubjectFaceProvenance::Vendor)
        }
        CadCollisionSubject::GeneratedCaseBody { id, analysis, .. } => {
            let asset = analysis.as_ref()?;
            if *asset
                .triangle_face_occurrence_indices
                .get(provenance.triangle_index as usize)?
                != provenance.face_occurrence_index
            {
                return None;
            }
            let occurrence = asset
                .face_occurrences
                .get(provenance.face_occurrence_index as usize)?;
            Some(CadSubjectFaceProvenance::Generated {
                body_id: id.clone(),
                triangle_index: provenance.triangle_index,
                face_occurrence_index: provenance.face_occurrence_index,
                solid_occurrence_index: occurrence.solid_occurrence_index,
                source: asset
                    .source_faces
                    .get(occurrence.source_face_index as usize)?
                    .clone(),
            })
        }
    }
}

pub fn assess_subject_collision_observation(
    left: &CadCollisionSubject,
    right: &CadCollisionSubject,
    clearance_threshold_mm: f64,
    requested_accuracy_mm: Option<f64>,
    query_numerical_error_bound_mm: Option<f64>,
    containment_ruled_out: bool,
    observation: boardstudio_web_host::cad_jobs::CadCollisionObservation,
) -> CadCollisionResult {
    use boardstudio_web_host::cad_jobs::CadCollisionObservation;

    let pair = match &observation {
        CadCollisionObservation::SurfaceIntersecting {
            intersection_triangles,
            ..
        } => Some(intersection_triangles),
        CadCollisionObservation::SurfaceSeparated { closest_points, .. } => {
            Some(&closest_points.triangles)
        }
        CadCollisionObservation::Uncertain { .. } => None,
    };
    let left_provenance = pair.and_then(|p| resolve_subject_face_provenance(left, &p.left));
    let right_provenance = pair.and_then(|p| resolve_subject_face_provenance(right, &p.right));

    let mut reasons = Vec::new();
    if pair.is_some() && (left_provenance.is_none() || right_provenance.is_none()) {
        reasons.push("witness triangle provenance could not be resolved to source faces".into());
    }
    if let CadCollisionObservation::Uncertain { reason, .. } = &observation {
        reasons.push(format!("raw mesh observation is uncertain: {reason}"));
    }
    let subject_bound =
        |subject: &CadCollisionSubject, side: &str, reasons: &mut Vec<String>| -> Option<f64> {
            match subject {
                CadCollisionSubject::VendorModel { vendor_step, .. } => {
                    let bound = vendor_step
                        .as_ref()
                        .and_then(|step| step.accuracy.analysis_error_bound_mm)
                        .filter(|v| v.is_finite() && *v >= 0.0);
                    if bound.is_none() {
                        reasons.push(format!(
                            "{side} asset has no certified analysisErrorBoundMm"
                        ));
                    }
                    bound
                }
                CadCollisionSubject::GeneratedCaseBody { analysis, .. } => {
                    let bound = analysis
                        .as_ref()
                        .and_then(|asset| asset.analysis_error_bound_mm)
                        .filter(|value| value.is_finite() && *value >= 0.0);
                    if bound.is_none() {
                        reasons.push(format!(
                            "{side} generated body has no certified analysisErrorBoundMm"
                        ));
                    }
                    bound
                }
            }
        };
    let left_bound = subject_bound(left, "left", &mut reasons);
    let right_bound = subject_bound(right, "right", &mut reasons);
    let query_bound = query_numerical_error_bound_mm.filter(|v| v.is_finite() && *v >= 0.0);
    if query_bound.is_none() {
        reasons.push("Parry query has no certified numerical error bound".into());
    }
    if !reasons.is_empty() {
        return CadCollisionResult {
            observation: Some(observation),
            left_provenance,
            right_provenance,
            assessment: CadCollisionAssessment::Unknown {
                reasons,
                refinement_requested: requested_accuracy_mm.is_some(),
            },
        };
    }

    let band_half_width = left_bound.unwrap() + right_bound.unwrap() + query_bound.unwrap();
    if let Some(requested) = requested_accuracy_mm
        && band_half_width > requested
    {
        return CadCollisionResult {
            observation: Some(observation),
            left_provenance,
            right_provenance,
            assessment: CadCollisionAssessment::Unknown {
                reasons: vec![format!(
                    "pair error budget {band_half_width} mm exceeds requested accuracy {requested} mm"
                )],
                refinement_requested: true,
            },
        };
    }

    let assessment = match &observation {
        CadCollisionObservation::SurfaceSeparated {
            surface_distance_mm,
            ..
        } => {
            let lower_mm = (surface_distance_mm - band_half_width).max(0.0);
            let upper_mm = surface_distance_mm + band_half_width;
            let relation = if upper_mm < clearance_threshold_mm {
                CadCollisionRelation::Violation
            } else if lower_mm >= clearance_threshold_mm && containment_ruled_out {
                CadCollisionRelation::Clear
            } else {
                CadCollisionRelation::Uncertain
            };
            CadCollisionAssessment::SupportedBand {
                lower_mm,
                upper_mm,
                relation,
            }
        }
        CadCollisionObservation::SurfaceIntersecting { .. } => {
            let lower_mm = 0.0;
            let upper_mm = band_half_width;
            let relation = if upper_mm < clearance_threshold_mm || band_half_width == 0.0 {
                CadCollisionRelation::Violation
            } else {
                CadCollisionRelation::Uncertain
            };
            CadCollisionAssessment::SupportedBand {
                lower_mm,
                upper_mm,
                relation,
            }
        }
        CadCollisionObservation::Uncertain { .. } => unreachable!(),
    };

    CadCollisionResult {
        observation: Some(observation),
        left_provenance,
        right_provenance,
        assessment,
    }
}

pub fn assess_collision_observation(
    left_asset: &boardstudio_web_host::cad_jobs::CadVendorStep,
    right_asset: &boardstudio_web_host::cad_jobs::CadVendorStep,
    clearance_threshold_mm: f64,
    requested_accuracy_mm: Option<f64>,
    query_numerical_error_bound_mm: Option<f64>,
    containment_ruled_out: bool,
    observation: boardstudio_web_host::cad_jobs::CadCollisionObservation,
) -> CadCollisionResult {
    let left = CadCollisionSubject::VendorModel {
        id: "left".into(),
        reference: "left".into(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        vendor_step: Some(Rc::new(left_asset.clone())),
    };
    let right = CadCollisionSubject::VendorModel {
        id: "right".into(),
        reference: "right".into(),
        row_transform_f64: IDENTITY_ROW_TRANSFORM_F64,
        vendor_step: Some(Rc::new(right_asset.clone())),
    };
    assess_subject_collision_observation(
        &left,
        &right,
        clearance_threshold_mm,
        requested_accuracy_mm,
        query_numerical_error_bound_mm,
        containment_ruled_out,
        observation,
    )
}

/// Native preparation records paths by source asset ID because Core needs that
/// direction when materializing export artifacts. `PcbPreview` rows point the
/// other way (path -> model row), so invert only at this consumer boundary.
pub fn native_model_path_assets(
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
    fn core_archived_preview_path_preserves_document_asset_precedence() {
        let source = BTreeMap::from([("asset-1".to_owned(), "models/hash.stl".to_owned())]);
        let native = native_model_path_assets(&source);
        assert_eq!(
            select_model_asset(
                "${KIPRJMOD}/models/hash.stl",
                None,
                &native,
                &[asset("asset-1", "ab", "part.stl")],
                |_| Some("bundled-model:other.stl".into()),
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

    #[test]
    fn step_cache_keys_isolate_repair_budgets_and_retain_full_vendor_asset() {
        use boardstudio_web_host::cad_jobs::{
            CadBounds, CadStepAccuracy, CadStepFaceOccurrence, CadStepImportOptions,
            CadStepLineOccurrence, CadStepPrecision, CadStepSourceFace, CadStepUncertainty,
            CadVendorStep,
        };

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

        let vendor_step = Rc::new(CadVendorStep {
            source_sha256: sha.clone(),
            import_options: default_opts,
            surface_positions_mm_f64: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            source_faces: vec![CadStepSourceFace {
                solid_id: "101".into(),
                shell_index: 0,
                face_id: "59".into(),
                support_id: "58".into(),
                support_kind: "PLANE".into(),
            }],
            face_occurrences: vec![CadStepFaceOccurrence {
                source_face_index: 0,
                solid_occurrence_index: 0,
                representation_id: "104".into(),
                source_uncertainty_mm: Some(0.01),
                repair_displacement_mm: None,
            }],
            triangle_face_occurrence_indices: vec![0],
            line_segments_mm_f64: vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
            line_render_positions: vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
            line_occurrences: vec![CadStepLineOccurrence {
                line_occurrence_index: 0,
                trimmed_curve_id: "414".into(),
                source_line_id: "413".into(),
                curve_set_id: "420".into(),
                representation_id: "430".into(),
                source_unit_to_mm: 1.0,
                placement_f64: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                declared_uncertainty_mm: None,
            }],
            segment_line_occurrence_indices: vec![0],
            precision: vec![CadStepPrecision {
                representation_id: "104".into(),
                context_id: "103".into(),
                geometric_dimension: Some(3),
                declarations: vec![CadStepUncertainty {
                    entity_id: "110".into(),
                    length_unit_id: "102".into(),
                    millimetres: 0.01,
                }],
            }],
            accuracy: CadStepAccuracy {
                preview_deflection_mm: 0.1,
                preview_angular_rad: 0.5,
                analysis_error_bound_mm: None,
                bounds_from_source_geometry: true,
                mesh_closed_oriented: true,
                preview_floor_mm: 0.02,
                preview_declared_multiplier: 2.0,
                relaxed_beyond_declared_precision: false,
                sampled_support_checks: 0,
                max_sampled_support_residual_mm: None,
                fallback_meshed_faces: 0,
                selected_repair_budget_mm: None,
                max_repair_displacement_mm: None,
            },
        });
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
        use boardstudio_web_host::cad_jobs::{
            CadClosestPointsWitness, CadCollisionObservation, CadStepAccuracy,
            CadStepFaceOccurrence, CadStepImportOptions, CadStepSourceFace,
            CadTrianglePairProvenance, CadTriangleProvenance, CadVendorStep,
        };

        let make_asset = |sha: &str, analysis_bound: Option<f64>| CadVendorStep {
            source_sha256: sha.into(),
            import_options: CadStepImportOptions::default(),
            surface_positions_mm_f64: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
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
                source_uncertainty_mm: Some(0.001),
                repair_displacement_mm: None,
            }],
            triangle_face_occurrence_indices: vec![0],
            line_segments_mm_f64: vec![],
            line_render_positions: vec![],
            line_occurrences: vec![],
            segment_line_occurrence_indices: vec![],
            precision: vec![],
            accuracy: CadStepAccuracy {
                preview_deflection_mm: 0.1,
                preview_angular_rad: 0.5,
                analysis_error_bound_mm: analysis_bound,
                bounds_from_source_geometry: true,
                mesh_closed_oriented: true,
                preview_floor_mm: 0.02,
                preview_declared_multiplier: 2.0,
                relaxed_beyond_declared_precision: false,
                sampled_support_checks: 0,
                max_sampled_support_residual_mm: None,
                fallback_meshed_faces: 0,
                selected_repair_budget_mm: None,
                max_repair_displacement_mm: None,
            },
        };

        let obs = CadCollisionObservation::SurfaceSeparated {
            surface_distance_mm: 0.5,
            closest_points: CadClosestPointsWitness {
                triangles: CadTrianglePairProvenance {
                    left: CadTriangleProvenance {
                        triangle_index: 0,
                        face_occurrence_index: 0,
                    },
                    right: CadTriangleProvenance {
                        triangle_index: 0,
                        face_occurrence_index: 0,
                    },
                },
                point1_mm: [0.0, 0.0, 0.0],
                point2_mm: [0.5, 0.0, 0.0],
            },
        };

        let uncertified_left = make_asset(&"a".repeat(64), None);
        let uncertified_right = make_asset(&"b".repeat(64), None);
        let result = assess_collision_observation(
            &uncertified_left,
            &uncertified_right,
            0.2,
            None,
            None,
            true,
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
        let clear = assess_collision_observation(
            &certified_left,
            &certified_right,
            0.2,
            None,
            Some(0.001),
            true,
            obs.clone(),
        );
        assert_eq!(
            clear.assessment,
            CadCollisionAssessment::SupportedBand {
                lower_mm: 0.5 - 0.041,
                upper_mm: 0.5 + 0.041,
                relation: CadCollisionRelation::Clear,
            }
        );

        let containment_unknown = assess_collision_observation(
            &certified_left,
            &certified_right,
            0.2,
            None,
            Some(0.001),
            false,
            obs.clone(),
        );
        assert!(matches!(
            containment_unknown.assessment,
            CadCollisionAssessment::SupportedBand {
                relation: CadCollisionRelation::Uncertain,
                ..
            }
        ));

        let violation = assess_collision_observation(
            &certified_left,
            &certified_right,
            0.6,
            None,
            Some(0.001),
            true,
            obs.clone(),
        );
        assert!(matches!(
            violation.assessment,
            CadCollisionAssessment::SupportedBand {
                relation: CadCollisionRelation::Violation,
                ..
            }
        ));

        let tight = assess_collision_observation(
            &certified_left,
            &certified_right,
            0.2,
            Some(TIGHT_COLLISION_ACCURACY_MM),
            Some(0.001),
            true,
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
            mesh: Rc::new(ValidatedMesh::try_from(valid_arrays()).unwrap()),
            matrix: None,
        };
        assert!(
            resolve_physical_model_row_transform(&row, None).is_err(),
            "PCB row with matrix: None must never substitute identity"
        );
    }

    #[test]
    fn generated_collision_preserves_numeric_face_provenance_and_unknown_accuracy() {
        use boardstudio_web_host::cad_jobs::*;
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../cad/wasm/tests/fixtures/generated-analysis-contract.json"
        ))
        .unwrap();
        let body: boardstudio_core::model::PreparedCaseIR =
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
            false,
            CadCollisionObservation::SurfaceSeparated {
                surface_distance_mm: 1.0,
                closest_points: CadClosestPointsWitness {
                    triangles: CadTrianglePairProvenance {
                        left: CadTriangleProvenance {
                            triangle_index: 0,
                            face_occurrence_index: 0,
                        },
                        right: CadTriangleProvenance {
                            triangle_index: 0,
                            face_occurrence_index: 0,
                        },
                    },
                    point1_mm: [20., 0., 0.],
                    point2_mm: [21., 0., 0.],
                },
            },
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
        assert!(
            details[0].contains("face occurrence 2") && details[1].contains("face occurrence 3")
        );
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
