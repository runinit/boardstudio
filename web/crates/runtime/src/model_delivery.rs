//! Private model-asset selection, identity, validation, and bounded mesh reuse.
//!
//! Runtime byte access, renderer decoding, STEP reading, viewer composition, and
//! renderer scene submission are supplied by the page owners. In particular,
//! model-batch liveness is independent of the renderer's scene sequence.

use super::layout_viewer_source::{LayoutPreviewSnapshot, LayoutSourceLease};
use crate::case_preview::CasePreviewOwnerLease;
use crate::parts_preview::PartsPreviewOwnerLease;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_model::model::{
    Asset, BoardReference, ModuleModelPlacement, PcbModel, Pose2, ProjectDoc, Side, Vec2, Vec3,
};
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
        if actual != expected_sha256 {
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
            && bytes.sha256() != asset.sha256
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
    if !boardstudio_web_host::cad_jobs::is_sha256_hex(sha256) {
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
    Refreshing {
        task_token: u64,
        batch: String,
        is_current: Rc<dyn Fn() -> bool>,
    },
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
    evidence_attempts: BTreeMap<String, String>,
    evidence_batch: Option<String>,
}

impl Default for ModelMeshCache {
    fn default() -> Self {
        Self {
            slots: BTreeMap::new(),
            recency: VecDeque::new(),
            next_task_token: 1,
            evidence_attempts: BTreeMap::new(),
            evidence_batch: None,
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
                // An evidence batch does not carry a comparable preview owner.
                CacheSlot::Pending { .. } | CacheSlot::Refreshing { .. } => {}
            }
        }

        let task_token = self.allocate_task_token()?;
        let superseded = match self.slots.get(sha256) {
            Some(
                CacheSlot::Pending { task_token, .. } | CacheSlot::Refreshing { task_token, .. },
            ) => Some(EvictedTask {
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
            Some(CacheSlot::Pending { task_token: current, .. } | CacheSlot::Refreshing { task_token: current, .. }) if *current == task_token);
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
            Some(CacheSlot::Pending { task_token: current, .. } | CacheSlot::Refreshing { task_token: current, .. }) if *current == task_token);
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

    fn claim_evidence_refresh(
        &mut self,
        key: &str,
        batch: &str,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<MeshCacheClaim, String> {
        if self.evidence_batch.as_deref() != Some(batch) {
            self.evidence_attempts.clear();
            self.evidence_batch = Some(batch.into());
        }
        if let Some(CacheSlot::Refreshing {
            task_token,
            batch: owner,
            is_current,
        }) = self.slots.get(key)
            && owner == batch
            && is_current()
        {
            self.evidence_attempts.insert(key.into(), batch.into());
            return Ok(MeshCacheClaim::JoinPending {
                task_token: *task_token,
            });
        }
        if self
            .evidence_attempts
            .get(key)
            .is_some_and(|current| current == batch)
        {
            return match self.slots.get(key) {
                Some(CacheSlot::Ready(asset)) => Ok(MeshCacheClaim::Reuse(asset.clone())),
                _ => Err("evidence preparation already failed in this batch".into()),
            };
        }
        let task_token = self.allocate_task_token()?;
        // Ordinary preview Pending has a typed preview owner, not an evidence
        // batch. Without comparable ownership, neither path may join the other.
        let superseded = match self.slots.get(key) {
            Some(
                CacheSlot::Pending { task_token, .. } | CacheSlot::Refreshing { task_token, .. },
            ) => Some(EvictedTask {
                sha256: key.into(),
                task_token: *task_token,
            }),
            _ => None,
        };
        let evicted = self.make_room_for(key);
        self.evidence_attempts.insert(key.into(), batch.into());
        self.slots.insert(
            key.into(),
            CacheSlot::Refreshing {
                task_token,
                batch: batch.into(),
                is_current,
            },
        );
        self.touch(key);
        Ok(MeshCacheClaim::Start {
            task_token,
            superseded,
            evicted,
        })
    }

    fn make_room_for(&mut self, sha256: &str) -> Option<EvictedTask> {
        if self.slots.contains_key(sha256) || self.slots.len() < MESH_CACHE_CAPACITY {
            return None;
        }
        let victim = self.recency.pop_front()?;
        let slot = self.slots.remove(&victim)?;
        match slot {
            CacheSlot::Pending { task_token, .. } | CacheSlot::Refreshing { task_token, .. } => {
                Some(EvictedTask {
                    sha256: victim,
                    task_token,
                })
            }
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
        let wakers = {
            let mut state = self.state.borrow_mut();
            if state.value.is_some() {
                return;
            }
            state.value = Some(value);
            std::mem::take(&mut state.wakers)
        };
        for waker in wakers {
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

/// Only a START owner may retire its token when its caller drops the future.
/// JOIN waiters never own this guard; old tokens cannot fail a replacement slot.
struct RefreshProducer<'a> {
    cache: &'a std::cell::RefCell<ModelMeshCache>,
    pending: &'a std::cell::RefCell<PendingModelWaiters>,
    task: &'a TaskKey,
    armed: bool,
}

impl Drop for RefreshProducer<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.cache.borrow_mut().fail(&self.task.0, self.task.1);
            settle_task(
                self.pending,
                self.task,
                Err("evidence preparation cancelled".into()),
            );
        }
    }
}

impl ModelDeliveryAdapter {
    /// A missing proof lease may replace a ready full asset once per check batch.
    /// Concurrent callers join the same task and only its token can settle it.
    pub async fn refresh_step_evidence(
        &self,
        sha: &str,
        options: boardstudio_web_host::cad_jobs::CadStepImportOptions,
        batch: &str,
        is_current: Rc<dyn Fn() -> bool>,
        load: impl Future<Output = Result<MeshArrays, String>>,
    ) -> Result<Rc<ValidatedMesh>, String> {
        if !is_current() {
            return Err("evidence preparation became stale".into());
        }
        let key = derived_step_cache_key(sha, options)?;
        let claim =
            self.cache
                .borrow_mut()
                .claim_evidence_refresh(&key, batch, is_current.clone())?;
        let (task_token, start) = match claim {
            MeshCacheClaim::Reuse(asset) => return Ok(asset),
            MeshCacheClaim::JoinPending { task_token } => (task_token, false),
            MeshCacheClaim::Start {
                task_token,
                evicted,
                superseded,
            } => {
                if let Some(task) = superseded {
                    settle_pending(
                        &self.pending,
                        &task,
                        Err("evidence preparation superseded".into()),
                    );
                }
                if let Some(task) = evicted {
                    settle_pending(
                        &self.pending,
                        &task,
                        Err("evidence preparation evicted".into()),
                    );
                }
                (task_token, true)
            }
        };
        let cell = Rc::new(WaitCell::new());
        let task = (key, task_token);
        self.pending
            .borrow_mut()
            .entry(task.clone())
            .or_default()
            .push(cell.clone());
        if start {
            let mut producer = RefreshProducer {
                cache: &self.cache,
                pending: &self.pending,
                task: &task,
                armed: true,
            };
            let result = load
                .await
                .and_then(|arrays| {
                    if arrays.vendor_step.is_none() {
                        return Err("evidence preparation omitted the full STEP asset".into());
                    }
                    ValidatedMesh::try_from(arrays)
                })
                .map(Rc::new);
            let result = if !is_current() {
                Err("evidence preparation became stale after parsing".into())
            } else {
                result
            };
            let result = match result {
                Ok(mesh) => {
                    let completed = self.cache.borrow_mut().complete_with_step_options(
                        sha,
                        options,
                        task_token,
                        mesh.clone(),
                    );
                    match completed {
                        Ok(true) => Ok(mesh),
                        Ok(false) => Err("evidence preparation was superseded".into()),
                        Err(error) => {
                            self.cache.borrow_mut().fail(&task.0, task_token);
                            Err(error)
                        }
                    }
                }
                Err(error) => {
                    self.cache.borrow_mut().fail(&task.0, task_token);
                    Err(error)
                }
            };
            settle_task(&self.pending, &task, result);
            producer.armed = false;
        }
        let result = WaitCellFuture { cell }.await;
        if !is_current() {
            return Err("evidence preparation became stale after settlement".into());
        }
        result
    }

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
    let waiters = pending.borrow_mut().remove(key);
    if let Some(waiters) = waiters {
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

mod collision;
pub use collision::*;

#[cfg(test)]
mod tests;
