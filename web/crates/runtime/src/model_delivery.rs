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
use boardstudio_model::model::{Asset, BoardReference, ModuleModelPlacement, PcbModel, ProjectDoc};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    future::{Future, poll_fn},
    pin::Pin,
    rc::{Rc, Weak},
    task::Poll,
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
    fn capture(
        scope: Scope,
        snapshot_token: SnapshotToken,
        viewer_instance: u64,
        projection_generation: u64,
        source_owner: Rc<dyn ModelSourceLease>,
    ) -> Self {
        Self {
            scope,
            snapshot_token,
            viewer_instance,
            projection_generation,
            source_owner: Rc::downgrade(&source_owner),
        }
    }

    pub fn new(
        scope: Scope,
        snapshot_token: SnapshotToken,
        viewer_instance: u64,
        projection_generation: u64,
        source_owner: &Rc<CasePreviewOwnerLease>,
    ) -> Self {
        Self::capture(
            scope,
            snapshot_token,
            viewer_instance,
            projection_generation,
            source_owner.clone(),
        )
    }

    pub fn new_layout(
        scope: Scope,
        snapshot_token: SnapshotToken,
        source_generation: u64,
        source_owner: &Rc<LayoutSourceLease>,
    ) -> Self {
        Self::capture(
            scope,
            snapshot_token,
            0,
            source_generation,
            source_owner.clone(),
        )
    }

    pub fn new_parts(
        scope: Scope,
        snapshot_token: SnapshotToken,
        source_generation: u64,
        source_owner: &Rc<PartsPreviewOwnerLease>,
    ) -> Self {
        Self::capture(
            scope,
            snapshot_token,
            0,
            source_generation,
            source_owner.clone(),
        )
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
        current_owner.identity_matches(
            current_scope,
            current_token,
            current_viewer_instance,
            current_projection_generation,
        ) && self.matches(
            current_scope,
            current_token,
            current_viewer_instance,
            current_projection_generation,
            current_owner.clone(),
        )
    }

    pub fn is_current_layout_owner(
        &self,
        current_scope: &Scope,
        current_token: SnapshotToken,
        source_generation: u64,
        current_owner: &Rc<LayoutSourceLease>,
    ) -> bool {
        self.matches(
            current_scope,
            current_token,
            0,
            source_generation,
            current_owner.clone(),
        )
    }

    pub fn is_current_parts_owner(
        &self,
        current_scope: &Scope,
        current_token: SnapshotToken,
        source_generation: u64,
        current_owner: &Rc<PartsPreviewOwnerLease>,
    ) -> bool {
        self.matches(
            current_scope,
            current_token,
            0,
            source_generation,
            current_owner.clone(),
        )
    }

    fn matches(
        &self,
        scope: &Scope,
        token: SnapshotToken,
        viewer: u64,
        generation: u64,
        owner: Rc<dyn ModelSourceLease>,
    ) -> bool {
        self.scope == *scope
            && self.snapshot_token == token
            && self.viewer_instance == viewer
            && self.projection_generation == generation
            && owner.is_active()
            && Weak::ptr_eq(&self.source_owner, &Rc::downgrade(&owner))
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

/// Prefer the reference mapping, then preview paths, then the bundled asset ID.
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

    select_model_asset_id(&asset_id, document_assets, |_| None)
}

/// Bytes crossing the page-model adapter must be revalidated even if they
/// came from the current-import memory map rather than BrowserStore.
#[derive(Clone, Debug)]
pub struct VerifiedModelBytes {
    bytes: Rc<[u8]>,
    sha256: String,
    #[cfg(target_arch = "wasm32")]
    packaged: bool,
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
            #[cfg(target_arch = "wasm32")]
            packaged: false,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn with_packaged_source(mut self) -> Self {
        self.packaged = true;
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn is_packaged(&self) -> bool {
        self.packaged
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
#[derive(Clone)]
pub struct ModelDeliveryPorts {
    pub load_verified_bytes:
        Rc<dyn Fn(ResolvedModelAsset) -> ModelFuture<Option<VerifiedModelBytes>>>,
    pub decode_stl: Rc<dyn Fn(VerifiedModelBytes) -> ModelFuture<MeshArrays>>,
    pub decode_wrl: Rc<dyn Fn(VerifiedModelBytes) -> ModelFuture<MeshArrays>>,
    pub read_step: Rc<dyn Fn(VerifiedModelBytes, ModelOwnerIdentity) -> ModelFuture<MeshArrays>>,
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

type ModelResult = Result<Rc<ValidatedMesh>, String>;
type DeliveryTask<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

#[derive(Default)]
pub struct ModelDeliveryAdapter {
    meshes: RefCell<VecDeque<(String, Rc<ValidatedMesh>)>>,
}

impl ModelDeliveryAdapter {
    fn cached(&self, key: &str) -> Option<Rc<ValidatedMesh>> {
        let mut meshes = self.meshes.borrow_mut();
        let index = meshes.iter().position(|(id, _)| id == key)?;
        let entry = meshes.remove(index).expect("located cached mesh");
        let mesh = entry.1.clone();
        meshes.push_back(entry);
        Some(mesh)
    }

    fn cache(&self, key: String, mesh: Rc<ValidatedMesh>) {
        let mut meshes = self.meshes.borrow_mut();
        meshes.retain(|(id, _)| id != &key);
        if meshes.len() == MESH_CACHE_CAPACITY {
            meshes.pop_front();
        }
        meshes.push_back((key, mesh));
    }

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
        // A new collision batch may need a fresh worker proof for the same source.
        let key = format!("evidence/{batch}/{}", derived_step_cache_key(sha, options)?);
        if let Some(mesh) = self.cached(&key) {
            return Ok(mesh);
        }
        let arrays = load.await?;
        if !is_current() {
            return Err("evidence preparation became stale after parsing".into());
        }
        if arrays.vendor_step.is_none() {
            return Err("evidence preparation omitted the full STEP asset".into());
        }
        let mesh = Rc::new(ValidatedMesh::try_from(arrays)?);
        self.cache(key, mesh.clone());
        Ok(mesh)
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
        self.deliver(
            preview_revision,
            models
                .iter()
                .map(|model| (model.id.as_str(), model.reference.as_str(), None)),
            selections,
            ports,
            batch,
            is_current,
        )
        .await
    }

    pub async fn deliver_module_placements(
        &self,
        preview_revision: u64,
        placements: &[ModuleModelPlacement],
        selections: &BTreeMap<String, AssetSelection>,
        ports: &ModelDeliveryPorts,
        batch: &ModelBatchIdentity,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Option<ModelDeliveryRows> {
        self.deliver(
            preview_revision,
            placements.iter().map(|placement| {
                (
                    placement.id.as_str(),
                    placement.id.as_str(),
                    Some(placement.matrix),
                )
            }),
            selections,
            ports,
            batch,
            is_current,
        )
        .await
    }

    async fn deliver<'a>(
        &self,
        preview_revision: u64,
        models: impl Iterator<Item = (&'a str, &'a str, Option<[f64; 16]>)>,
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

        let mut assets = BTreeMap::new();
        let models = models
            .map(|(id, reference, matrix)| {
                let asset = selected_asset(selections.get(id));
                if let Ok(asset) = asset {
                    assets.entry(asset.sha256.as_str()).or_insert(asset);
                }
                (id, reference, matrix, asset)
            })
            .collect::<Vec<_>>();

        let results = RefCell::new(BTreeMap::<&str, ModelResult>::new());
        let mut tasks: Vec<DeliveryTask<'_>> = Vec::new();
        for (sha, asset) in assets {
            let results = &results;
            let current = is_current.clone();
            tasks.push(Box::pin(async move {
                let result = async {
                    if !current() {
                        return Err("Model request became stale before loading".into());
                    }
                    let format = ModelFormat::from_filename(&asset.filename)?;
                    if let Some(mesh) = self.cached(sha) {
                        return Ok(mesh);
                    }
                    let mesh = Rc::new(
                        load_and_decode(ports, asset, format, batch.owner().clone(), current)
                            .await?,
                    );
                    self.cache(sha.to_owned(), mesh.clone());
                    Ok(mesh)
                }
                .await;
                results.borrow_mut().insert(sha, result);
            }));
        }
        join_all(tasks).await;
        if !is_current() {
            return None;
        }

        let results = results.into_inner();
        let mut rows = ModelDeliveryRows::default();
        for (id, reference, matrix, asset) in models {
            let result = asset.as_ref().map_err(String::as_str).and_then(|asset| {
                results[asset.sha256.as_str()]
                    .as_ref()
                    .map_err(String::as_str)
            });
            match result {
                Ok(mesh) => rows.delivered.push(DeliveredModel {
                    id: id.to_owned(),
                    mesh: mesh.clone(),
                    matrix,
                }),
                Err(reason) => rows.failures.push(ModelFailure {
                    reference: reference.to_owned(),
                    reason: reason.to_owned(),
                }),
            }
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

fn selected_asset(selection: Option<&AssetSelection>) -> Result<&ResolvedModelAsset, String> {
    match selection {
        Some(AssetSelection::Archived(asset) | AssetSelection::Packaged(asset)) => Ok(asset),
        Some(AssetSelection::MissingDocumentAsset { asset_id }) => {
            Err(format!("Document model asset {asset_id} is missing"))
        }
        Some(AssetSelection::MissingBundledProvider { asset_id }) => Err(format!(
            "Bundled model provider is unavailable for {asset_id}"
        )),
        Some(AssetSelection::NoAssetId) => Err("No model asset ID is mapped".into()),
        None => Err("No model asset mapping was provided".into()),
    }
}

async fn join_all(futures: Vec<DeliveryTask<'_>>) {
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
