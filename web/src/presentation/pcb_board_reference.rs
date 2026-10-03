//! Scoped controls and file-backed editing for routed-board references.
use boardstudio_application::TerminalOutcome;
use boardstudio_core::model::{Asset, BoardReference};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_timers::future::TimeoutFuture;
use js_sys::Uint8Array;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{File, HtmlInputElement};

use boardstudio_web::host::AssetBytes;

use crate::runtime::Runtime;

const MAX_REFERENCE_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone)]
pub(super) struct BoardReferenceRuntimeHandle(Rc<Runtime>);

impl BoardReferenceRuntimeHandle {
    pub(super) fn new(runtime: Rc<Runtime>) -> Self {
        Self(runtime)
    }
}

impl PartialEq for BoardReferenceRuntimeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub(super) struct BoardReferenceAdapterHandle(super::SelectionAdapter);

impl BoardReferenceAdapterHandle {
    pub(super) fn new(adapter: super::SelectionAdapter) -> Self {
        Self(adapter)
    }
}

impl PartialEq for BoardReferenceAdapterHandle {
    fn eq(&self, other: &Self) -> bool {
        self.0.selected_context == other.0.selected_context
            && self.0.anchor_scope == other.0.anchor_scope
            && self.0.generation == other.0.generation
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Action {
    SetEnabled(bool),
    SetPositionX(f64),
    SetPositionY(f64),
    SetRotation(f64),
    SetElevation(f64),
    SetModelAsset {
        path: String,
        asset_id: Option<String>,
    },
    Remove,
}

async fn read_kicad_file(file: &File) -> Result<(Vec<u8>, String), String> {
    let filename = file.name();
    if !filename.to_ascii_lowercase().ends_with(".kicad_pcb") {
        return Err("Choose a .kicad_pcb file smaller than 32 MiB".into());
    }
    if file.size() <= 0.0 || file.size() > MAX_REFERENCE_BYTES {
        return Err("Choose a .kicad_pcb file smaller than 32 MiB".into());
    }
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|error| format!("Could not read routed KiCad board: {error:?}"))?;
    let bytes = Uint8Array::new(&buffer).to_vec();
    let source = String::from_utf8(bytes.clone())
        .map_err(|error| format!("Routed KiCad board is not valid UTF-8: {error}"))?;
    Ok((bytes, source))
}

fn preview_paths(models: &[boardstudio_core::model::PcbModel]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    models
        .iter()
        .filter_map(|model| {
            let path = model.path.as_str();
            (!path.is_empty() && seen.insert(path.to_owned())).then(|| path.to_owned())
        })
        .collect()
}

async fn wait_for_reference_edit(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &super::SelectionAdapter,
    owner: &super::LayoutOwnerIdentity,
    generation: Signal<u64>,
    request_generation: u64,
    reference_id: &str,
    source_asset_id: &str,
    outcome: crate::operation_outcomes::OutcomeSlot,
) -> Result<(), String> {
    loop {
        if generation() != request_generation
            || !super::board_reference_owner_lineage_is_current(runtime, workspace, adapter, owner)
        {
            return Err(
                "The routed-board edit changed before it finished saving. Check the current board."
                    .into(),
            );
        }
        let settled = outcome.borrow().clone();
        if let Some(settled) = settled {
            return match settled {
                TerminalOutcome::Completed
                    if super::board_reference_target_is_current(
                        runtime,
                        workspace,
                        adapter,
                        owner,
                        reference_id,
                        source_asset_id,
                    ) =>
                {
                    Ok(())
                }
                TerminalOutcome::Completed => Err(
                    "The routed-board edit finished, but its reference is no longer active.".into(),
                ),
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::PersistenceFailed(message)
                | TerminalOutcome::BlockedByRecovery(message)
                | TerminalOutcome::ExecutorFailed(message) => Err(message),
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    Err("The routed-board edit was cancelled or superseded.".into())
                }
            };
        }
        TimeoutFuture::new(25).await;
    }
}

async fn discover_stored_paths(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &super::SelectionAdapter,
    owner: &super::LayoutOwnerIdentity,
    reference: &BoardReference,
    generation: Signal<u64>,
    request_generation: u64,
) -> Result<Vec<String>, String> {
    if !super::board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return Err("The active board changed before routed-board model paths were read.".into());
    }
    let model = runtime.model();
    let accepted = model
        .accepted
        .as_ref()
        .ok_or("No accepted project is open.")?;
    let asset = accepted
        .document
        .assets
        .iter()
        .find(|asset| asset.id == reference.asset_id)
        .cloned()
        .ok_or_else(|| "Saved routed-board file is missing from the project.".to_owned())?;
    if asset.sha256.is_empty() {
        return Err("Saved routed-board file has no SHA-256 identity.".into());
    }
    let bytes = runtime
        .store
        .load_asset(asset.sha256.clone())
        .await
        .map_err(|error| format!("Could not load routed-board file: {error}"))?
        .ok_or_else(|| "Saved routed-board file is missing from browser storage.".to_owned())?
        .to_vec();
    if generation() != request_generation
        || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
    {
        return Err("The routed-board source changed while its asset was loading.".into());
    }
    if bytes.is_empty()
        || bytes.len() as f64 > MAX_REFERENCE_BYTES
        || sha256_bytes(&bytes) != asset.sha256
    {
        return Err("Saved routed-board file failed its size or SHA-256 check.".into());
    }
    let source = String::from_utf8(bytes)
        .map_err(|error| format!("Saved routed-board file is not valid UTF-8: {error}"))?;
    let operation = runtime.operation();
    let preview = runtime
        .preview_routed_board_source(
            format!("board-reference-discover-{}", operation.0),
            source,
            owner
                .revision
                .ok_or("Accepted board revision is unavailable.")?,
        )
        .await?;
    if generation() != request_generation
        || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
    {
        return Err("The routed-board source changed while Core discovered model paths.".into());
    }
    if Some(preview.revision) != owner.revision {
        return Err("Core returned model paths for a different routed-board revision.".into());
    }
    Ok(preview_paths(&preview.models))
}

async fn import_routed_board(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &super::SelectionAdapter,
    owner: &super::LayoutOwnerIdentity,
    reference: Option<&BoardReference>,
    file: File,
    generation: Signal<u64>,
    request_generation: u64,
) -> Result<(String, String, Vec<String>), String> {
    if !super::board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return Err("The active project or board changed. Retry with the current board.".into());
    }
    let (bytes, source) = read_kicad_file(&file).await?;
    if generation() != request_generation
        || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
    {
        return Err("The project or board changed while the routed-board file was read.".into());
    }
    let revision = owner
        .revision
        .ok_or("Accepted board revision is unavailable.")?;
    let operation = runtime.operation();
    let preview = runtime
        .preview_routed_board_source(
            format!("board-reference-import-{}", operation.0),
            source,
            revision,
        )
        .await?;
    if generation() != request_generation
        || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
    {
        return Err("The project or board changed while Core previewed the routed board.".into());
    }
    if preview.revision != revision {
        return Err("Core returned a routed-board preview for a different revision.".into());
    }
    let paths = preview_paths(&preview.models);
    let asset_id = crate::runtime::new_project_id()?;
    let sha256 = sha256_bytes(&bytes);
    let asset = Asset {
        id: asset_id.clone(),
        name: file.name(),
        media_type: "application/x-kicad_pcb".into(),
        sha256: sha256.clone(),
        license: None,
        source: None,
    };
    runtime
        .store
        .save_asset(AssetBytes { sha256, bytes })
        .await
        .map_err(|error| format!("Could not store routed-board file: {error}"))?;
    if generation() != request_generation
        || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
    {
        return Err("The project or board changed while the routed-board asset was stored.".into());
    }
    let model = runtime.model();
    let accepted = model
        .accepted
        .as_ref()
        .ok_or("No accepted project is open.")?;
    let scope = owner.scope.as_ref().ok_or("Active board is unavailable.")?;
    let mut proposed = accepted.document.as_ref().clone();
    if proposed.assets.iter().any(|current| current.id == asset.id) {
        return Err("Could not allocate a unique routed-board asset identity.".into());
    }
    proposed.assets.push(asset);
    let reference_id = if let Some(reference) = reference {
        let existing = proposed
            .board_references
            .iter_mut()
            .find(|current| current.id == reference.id && current.board_id == scope.board_id)
            .ok_or_else(|| {
                "The routed-board reference changed. Reopen the panel and retry.".to_owned()
            })?;
        existing.asset_id = asset_id;
        existing.enabled = true;
        existing.id.clone()
    } else {
        let id = crate::runtime::new_project_id()?;
        if proposed
            .board_references
            .iter()
            .any(|current| current.id == id)
        {
            return Err("Could not allocate a unique routed-board reference identity.".into());
        }
        proposed.board_references.push(BoardReference {
            id: id.clone(),
            board_id: scope.board_id.clone(),
            asset_id,
            enabled: true,
            pose: boardstudio_core::model::Pose2 {
                at: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: reference
                .map(|reference| reference.model_assets.clone())
                .unwrap_or_default(),
        });
        id
    };
    let asset_id = proposed
        .board_references
        .iter()
        .find(|current| current.id == reference_id && current.board_id == scope.board_id)
        .map(|current| current.asset_id.clone())
        .ok_or_else(|| "The routed-board reference could not be prepared.".to_owned())?;
    let outcome = super::submit_board_reference_document(
        runtime,
        workspace,
        adapter,
        owner,
        proposed,
        "board-reference-import",
    )?
    .ok_or_else(|| "The routed-board import produced no project change.".to_owned())?;
    wait_for_reference_edit(
        runtime,
        workspace,
        adapter,
        owner,
        generation,
        request_generation,
        &reference_id,
        &asset_id,
        outcome,
    )
    .await?;
    Ok((reference_id, asset_id, paths))
}

enum ModelAttachment {
    Individual { path: String, file: File },
    Directory { files: Vec<File> },
}

fn is_model_filename(filename: &str) -> bool {
    [".step", ".stp", ".stl", ".wrl"]
        .iter()
        .any(|extension| filename.to_ascii_lowercase().ends_with(extension))
}

fn model_basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

async fn attach_model_files(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &super::SelectionAdapter,
    owner: &super::LayoutOwnerIdentity,
    reference: &BoardReference,
    paths: &[String],
    attachment: ModelAttachment,
    generation: Signal<u64>,
    request_generation: u64,
) -> Result<(), String> {
    use super::model_asset_import::read_model_file;

    if !super::board_reference_owner_is_current(runtime, workspace, adapter, owner) {
        return Err("The active project or board changed. Retry with the current board.".into());
    }
    let mut uploads = match attachment {
        ModelAttachment::Individual { path, file } => {
            if !paths.iter().any(|known| known == &path) {
                return Err(
                    "The routed-board model path changed. Reload the reference panel.".into(),
                );
            }
            vec![(path, file)]
        }
        ModelAttachment::Directory { files } => {
            let mut candidates = std::collections::BTreeMap::<String, Vec<File>>::new();
            for file in files
                .into_iter()
                .filter(|file| is_model_filename(&file.name()))
            {
                candidates.entry(file.name()).or_default().push(file);
            }
            let mut matched = Vec::new();
            for path in paths {
                if let Some(files) = candidates.get_mut(model_basename(path))
                    && files.len() == 1
                {
                    matched.push((path.clone(), files.remove(0)));
                }
            }
            if matched.is_empty() {
                return Err(
                    "No unique model filenames matched. Attach files individually below.".into(),
                );
            }
            matched
        }
    };
    let model = runtime.model();
    let accepted = model
        .accepted
        .as_ref()
        .ok_or("No accepted project is open.")?;
    let scope = owner.scope.as_ref().ok_or("Active board is unavailable.")?;
    let current_reference = accepted
        .document
        .board_references
        .iter()
        .find(|current| current.id == reference.id && current.board_id == scope.board_id)
        .ok_or_else(|| {
            "The routed-board reference changed. Reopen the panel and retry.".to_owned()
        })?;
    if current_reference.asset_id != reference.asset_id {
        return Err(
            "The routed-board source changed. Reopen the model attachment controls.".into(),
        );
    }
    let mut proposed = accepted.document.as_ref().clone();
    let mut imported_assets: Vec<(String, Asset)> = Vec::with_capacity(uploads.len());
    for (path, file) in uploads.drain(..) {
        if generation() != request_generation
            || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
        {
            return Err(
                "The project or board changed while model files were being attached.".into(),
            );
        }
        let imported = read_model_file(file).await?;
        if generation() != request_generation
            || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
        {
            return Err("The project or board changed while a model file was read.".into());
        }
        imported
            .store(&runtime.store)
            .await
            .map_err(|error| format!("Could not store model asset: {error}"))?;
        if generation() != request_generation
            || !super::board_reference_owner_is_current(runtime, workspace, adapter, owner)
        {
            return Err("The project or board changed while a model asset was stored.".into());
        }
        let id = crate::runtime::new_project_id()?;
        if proposed.assets.iter().any(|asset| asset.id == id)
            || imported_assets.iter().any(|(_, asset)| asset.id == id)
        {
            return Err("Could not allocate a unique model asset identity.".into());
        }
        imported_assets.push((
            path,
            Asset {
                id,
                name: imported.filename,
                media_type: imported.media_type,
                sha256: imported.sha256,
                license: None,
                source: None,
            },
        ));
    }
    for (path, asset) in imported_assets {
        let Some(current) = proposed
            .board_references
            .iter_mut()
            .find(|current| current.id == reference.id && current.board_id == scope.board_id)
        else {
            return Err(
                "The routed-board reference changed before model attachment completed.".into(),
            );
        };
        current.model_assets.insert(path, asset.id.clone());
        proposed.assets.push(asset);
    }
    let outcome = super::submit_board_reference_document(
        runtime,
        workspace,
        adapter,
        owner,
        proposed,
        "board-reference-model-attach",
    )?
    .ok_or_else(|| "The model attachment produced no project change.".to_owned())?;
    wait_for_reference_edit(
        runtime,
        workspace,
        adapter,
        owner,
        generation,
        request_generation,
        &reference.id,
        &reference.asset_id,
        outcome,
    )
    .await
}

fn begin_request(
    mut request_generation: Signal<u64>,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
) -> Option<u64> {
    if busy() {
        return None;
    }
    let next = request_generation().wrapping_add(1);
    request_generation.set(next);
    busy.set(true);
    error.set(None);
    Some(next)
}

fn begin_board_import(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: super::SelectionAdapter,
    owner: super::LayoutOwnerIdentity,
    reference: Option<BoardReference>,
    file: File,
    request_generation: Signal<u64>,
    busy: Signal<bool>,
    error: Signal<Option<String>>,
    paths: Signal<Vec<String>>,
    paths_asset_id: Signal<Option<String>>,
) {
    let Some(epoch) = begin_request(request_generation, busy, error) else {
        return;
    };
    let mut busy = busy;
    let mut error = error;
    let mut paths = paths;
    let mut paths_asset_id = paths_asset_id;
    let current_epoch = request_generation;
    spawn_local(async move {
        let result = import_routed_board(
            &runtime,
            workspace,
            &adapter,
            &owner,
            reference.as_ref(),
            file,
            current_epoch,
            epoch,
        )
        .await;
        if current_epoch() == epoch {
            busy.set(false);
            match result {
                Ok((_, asset_id, discovered)) => {
                    paths.set(discovered);
                    paths_asset_id.set(Some(asset_id));
                }
                Err(message)
                    if super::board_reference_owner_lineage_is_current(
                        &runtime, workspace, &adapter, &owner,
                    ) =>
                {
                    error.set(Some(message));
                }
                Err(_) => {}
            }
        }
    });
}

fn begin_model_attachment(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: super::SelectionAdapter,
    owner: super::LayoutOwnerIdentity,
    reference: Option<BoardReference>,
    paths: Vec<String>,
    attachment: ModelAttachment,
    request_generation: Signal<u64>,
    busy: Signal<bool>,
    error: Signal<Option<String>>,
) {
    let Some(reference) = reference else {
        return;
    };
    let Some(epoch) = begin_request(request_generation, busy, error) else {
        return;
    };
    let mut busy = busy;
    let mut error = error;
    let current_epoch = request_generation;
    spawn_local(async move {
        let result = attach_model_files(
            &runtime,
            workspace,
            &adapter,
            &owner,
            &reference,
            &paths,
            attachment,
            current_epoch,
            epoch,
        )
        .await;
        if current_epoch() == epoch {
            busy.set(false);
            if let Err(message) = result
                && super::board_reference_owner_lineage_is_current(
                    &runtime, workspace, &adapter, &owner,
                )
            {
                error.set(Some(message));
            }
        }
    });
}

#[component]
pub(super) fn Editor(
    reference: Option<BoardReference>,
    assets: Vec<Asset>,
    disabled: bool,
    runtime: BoardReferenceRuntimeHandle,
    workspace: Signal<&'static str>,
    adapter: BoardReferenceAdapterHandle,
    owner: super::LayoutOwnerIdentity,
) -> Element {
    let runtime = runtime.0;
    let adapter = adapter.0;
    let model_paths = use_signal(Vec::<String>::new);
    let paths_asset_id = use_signal(|| None::<String>);
    let attempted_discovery = use_signal(|| None::<String>);
    let discovery_retry = use_signal(|| 0u64);
    let request_generation = use_signal(|| 0u64);
    let busy = use_signal(|| false);
    let error = use_signal(|| None::<String>);

    use_effect(use_reactive(
        (&reference, &owner, &disabled, &busy(), &discovery_retry()),
        {
            let runtime = runtime.clone();
            let mut model_paths = model_paths;
            let mut paths_asset_id = paths_asset_id;
            let mut attempted_discovery = attempted_discovery;
            let mut request_generation = request_generation;
            let mut busy_signal = busy;
            let mut error = error;
            let workspace = workspace;
            let adapter = adapter.clone();
            move |(reference, owner, disabled, already_busy, _retry)| {
                let Some(reference) = reference.clone() else {
                    if already_busy {
                        return;
                    }
                    model_paths.set(Vec::new());
                    paths_asset_id.set(None);
                    attempted_discovery.set(None);
                    request_generation.set(request_generation().wrapping_add(1));
                    error.set(None);
                    return;
                };
                if disabled || already_busy || paths_asset_id() == Some(reference.asset_id.clone())
                {
                    return;
                }
                let Some(scope) = owner.scope.as_ref() else {
                    return;
                };
                let attempt_key = format!(
                    "{}:{}:{}:{}:{:?}:{}:{}",
                    scope.session_epoch.0,
                    scope.document_id,
                    scope.board_id,
                    reference.id,
                    owner.token,
                    owner.revision.unwrap_or_default(),
                    reference.asset_id,
                );
                if attempted_discovery() == Some(attempt_key.clone()) {
                    return;
                }
                attempted_discovery.set(Some(attempt_key));
                let epoch = request_generation().wrapping_add(1);
                request_generation.set(epoch);
                busy_signal.set(true);
                error.set(None);
                let runtime = runtime.clone();
                let owner = owner.clone();
                let adapter = adapter.clone();
                let workspace = workspace;
                let source_asset_id = reference.asset_id.clone();
                let generation = request_generation;
                let mut busy_signal = busy_signal;
                let mut error = error;
                let mut model_paths = model_paths;
                let mut paths_asset_id = paths_asset_id;
                spawn_local(async move {
                    let result = discover_stored_paths(
                        &runtime, workspace, &adapter, &owner, &reference, generation, epoch,
                    )
                    .await;
                    if generation() == epoch {
                        busy_signal.set(false);
                        match result {
                            Ok(paths)
                                if super::board_reference_owner_lineage_is_current(
                                    &runtime, workspace, &adapter, &owner,
                                ) =>
                            {
                                model_paths.set(paths);
                                paths_asset_id.set(Some(source_asset_id));
                            }
                            Err(message)
                                if super::board_reference_owner_lineage_is_current(
                                    &runtime, workspace, &adapter, &owner,
                                ) =>
                            {
                                error.set(Some(message));
                            }
                            _ => {}
                        }
                    }
                });
            }
        },
    ));

    let on_action = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let reference_id = reference.as_ref().map(|reference| reference.id.clone());
        move |action: Action| {
            let Some(reference_id) = reference_id.as_deref() else {
                return;
            };
            super::dispatch_board_reference_action(
                &runtime,
                workspace,
                &adapter,
                &owner,
                reference_id,
                action,
            );
        }
    });
    let on_import = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let reference = reference.clone();
        move |file: File| {
            if disabled {
                return;
            }
            begin_board_import(
                runtime.clone(),
                workspace,
                adapter.clone(),
                owner.clone(),
                reference.clone(),
                file,
                request_generation,
                busy,
                error,
                model_paths,
                paths_asset_id,
            );
        }
    });
    let on_attach_file = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let reference = reference.clone();
        move |(path, file): (String, File)| {
            if disabled {
                return;
            }
            begin_model_attachment(
                runtime.clone(),
                workspace,
                adapter.clone(),
                owner.clone(),
                reference.clone(),
                model_paths(),
                ModelAttachment::Individual { path, file },
                request_generation,
                busy,
                error,
            );
        }
    });
    let on_attach_directory = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let reference = reference.clone();
        move |files: Vec<File>| {
            if disabled {
                return;
            }
            begin_model_attachment(
                runtime.clone(),
                workspace,
                adapter.clone(),
                owner.clone(),
                reference.clone(),
                model_paths(),
                ModelAttachment::Directory { files },
                request_generation,
                busy,
                error,
            );
        }
    });

    let options = assets
        .iter()
        .filter(|asset| is_model_filename(&asset.name))
        .collect::<Vec<_>>();
    let current_paths = model_paths();
    let busy = busy();
    let mut attempted_discovery_signal = attempted_discovery;
    let mut discovery_retry_signal = discovery_retry;
    let mut error_signal = error;

    rsx! {
        details { class: "m1-case-physical-setup m1-board-reference",
            summary { "Routed PCB reference" }
            p { "Preview a routed KiCad board with the case. Replace this reference after editing routing in KiCad." }
            label {
                if reference.is_some() { "Replace KiCad board" } else { "Import KiCad board" }
                input {
                    r#type: "file",
                    accept: ".kicad_pcb",
                    disabled: disabled || busy,
                    aria_label: if reference.is_some() { "Replace KiCad board" } else { "Import KiCad board" },
                    onchange: move |event: FormEvent| {
                        let Some(input) = event.data().try_as_web_event()
                            .and_then(|event| event.target())
                            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                        else { return; };
                        let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                        input.set_value("");
                        on_import.call(file);
                    }
                }
            }
            if let Some(reference) = reference.as_ref() {
                label { class: "m1-board-reference-enabled",
                    input {
                        r#type: "checkbox",
                        checked: reference.enabled,
                        disabled: disabled || busy,
                        aria_label: "Use routed PCB in assembly",
                        onchange: move |event: FormEvent| on_action.call(Action::SetEnabled(event.checked())),
                    }
                    "Use routed PCB in assembly"
                }
                div { class: "m1-board-reference-transform",
                    label { "X (mm)"
                        input {
                            r#type: "number", step: "0.1", value: "{reference.pose.at.x}",
                            disabled: disabled || busy,
                            oninput: move |event: FormEvent| {
                                if let Ok(value) = event.value().parse::<f64>()
                                    && value.is_finite()
                                { on_action.call(Action::SetPositionX(value)); }
                            }
                        }
                    }
                    label { "Y (mm)"
                        input {
                            r#type: "number", step: "0.1", value: "{reference.pose.at.y}",
                            disabled: disabled || busy,
                            oninput: move |event: FormEvent| {
                                if let Ok(value) = event.value().parse::<f64>()
                                    && value.is_finite()
                                { on_action.call(Action::SetPositionY(value)); }
                            }
                        }
                    }
                    label { "Z (mm)"
                        input {
                            r#type: "number", step: "0.1", value: "{reference.elevation}",
                            disabled: disabled || busy,
                            oninput: move |event: FormEvent| {
                                if let Ok(value) = event.value().parse::<f64>()
                                    && value.is_finite()
                                { on_action.call(Action::SetElevation(value)); }
                            }
                        }
                    }
                }
                label { "Rotation (°)"
                    input {
                        r#type: "number", value: "{reference.pose.rotation}",
                        disabled: disabled || busy,
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f64>()
                                && value.is_finite()
                            { on_action.call(Action::SetRotation(value)); }
                        }
                    }
                }
                if !current_paths.is_empty() {
                    label {
                        "Attach model directory"
                        input {
                            r#type: "file",
                            multiple: true,
                            "webkitdirectory": true,
                            disabled: disabled || busy,
                            aria_label: "Attach model directory",
                            onchange: move |event: FormEvent| {
                                let Some(input) = event.data().try_as_web_event()
                                    .and_then(|event| event.target())
                                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                else { return; };
                                let files = input.files().map(|files| {
                                    (0..files.length()).filter_map(|index| files.get(index)).collect::<Vec<_>>()
                                }).unwrap_or_default();
                                input.set_value("");
                                on_attach_directory.call(files);
                            }
                        }
                    }
                }
                for path in current_paths.iter().cloned() {
                    {
                        let selected_asset_id = reference.model_assets.get(&path).cloned().unwrap_or_default();
                        let current_asset = selected_asset_id.clone();
                        let path_for_select = path.clone();
                        let path_for_file = path.clone();
                        rsx! {
                            label { key: "{path}", "Model asset for {path}"
                                select {
                                    aria_label: "Model asset for {path}",
                                    value: "{selected_asset_id}",
                                    disabled: disabled || busy,
                                    onchange: move |event: FormEvent| {
                                        let selected = event.value();
                                        on_action.call(Action::SetModelAsset {
                                            path: path_for_select.clone(),
                                            asset_id: if selected.is_empty() { None } else { Some(selected) },
                                        });
                                    },
                                    option { value: "", "Resolve bundled model" }
                                    if !current_asset.is_empty() && !options.iter().any(|asset| asset.id == current_asset) {
                                        option { value: "{current_asset}", "Saved model asset is unavailable" }
                                    }
                                    for asset in &options {
                                        option { key: "{asset.id}", value: "{asset.id}", "{asset.name}" }
                                    }
                                }
                                input {
                                    r#type: "file",
                                    accept: ".step,.stp,.stl,.wrl",
                                    disabled: disabled || busy,
                                    aria_label: "Attach model for {path}",
                                    onchange: move |event: FormEvent| {
                                        let Some(input) = event.data().try_as_web_event()
                                            .and_then(|event| event.target())
                                            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                        else { return; };
                                        let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                                        input.set_value("");
                                        on_attach_file.call((path_for_file.clone(), file));
                                    }
                                }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    disabled: disabled || busy,
                    onclick: move |_| on_action.call(Action::Remove),
                    "Remove PCB reference"
                }
            }
            if busy { p { role: "status", "Reading routed board or model assets…" } }
            if let Some(message) = error() { p { role: "alert", "{message}" } }
            if reference.is_some() && error().is_some() && current_paths.is_empty() {
                button {
                    r#type: "button",
                    disabled: disabled || busy,
                    onclick: move |_| {
                        attempted_discovery_signal.set(None);
                        discovery_retry_signal.set(discovery_retry_signal().wrapping_add(1));
                        error_signal.set(None);
                    },
                    "Retry model-path discovery"
                }
            }
        }
    }
}
