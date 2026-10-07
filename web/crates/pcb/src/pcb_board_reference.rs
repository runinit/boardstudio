//! Scoped controls and file-backed editing for routed-board references.
use boardstudio_core::model::{Asset, BoardReference};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_timers::future::TimeoutFuture;
use js_sys::Uint8Array;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{File, HtmlInputElement};

use boardstudio_web_host::host::AssetBytes;

use crate::runtime::Runtime;

#[path = "pcb_board_reference/matching.rs"]
mod matching;

const MAX_REFERENCE_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone)]
pub struct BoardReferenceRuntimeHandle(Rc<Runtime>);

impl BoardReferenceRuntimeHandle {
    pub fn new(runtime: Rc<Runtime>) -> Self {
        Self(runtime)
    }
}

impl PartialEq for BoardReferenceRuntimeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct BoardReferenceAdapterHandle(super::SelectionAdapter);

impl BoardReferenceAdapterHandle {
    pub fn new(adapter: super::SelectionAdapter) -> Self {
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
pub enum Action {
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

fn removal_pending(tickets: Signal<Vec<(Action, EditTicket)>>) -> bool {
    tickets
        .read()
        .iter()
        .any(|(action, ticket)| *action == Action::Remove && ticket.is_pending())
}

fn reference_with_drafts(
    mut reference: BoardReference,
    tickets: &[(Action, EditTicket)],
) -> BoardReference {
    for (action, ticket) in tickets {
        if !ticket.is_pending() {
            continue;
        }
        match action {
            Action::SetEnabled(value) => reference.enabled = *value,
            Action::SetPositionX(value) => reference.pose.at.x = *value,
            Action::SetPositionY(value) => reference.pose.at.y = *value,
            Action::SetRotation(value) => reference.pose.rotation = *value,
            Action::SetElevation(value) => reference.elevation = *value,
            Action::SetModelAsset { path, asset_id } => match asset_id {
                Some(id) => {
                    reference.model_assets.insert(path.clone(), id.clone());
                }
                None => {
                    reference.model_assets.remove(path);
                }
            },
            Action::Remove => {}
        }
    }
    reference
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

async fn wait_for_reference_ticket(
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: &super::SelectionAdapter,
    owner: &super::LayoutOwnerIdentity,
    generation: Signal<u64>,
    request_generation: u64,
    ticket: EditTicket,
) -> Result<(), String> {
    loop {
        let live = generation() == request_generation
            && super::board_reference_owner_lineage_is_current(runtime, workspace, adapter, owner);
        match ticket.settlement(live) {
            Settlement::Pending => {}
            Settlement::Landed { .. } => return Ok(()),
            Settlement::Failed { message } => return Err(message),
            Settlement::Retired => return Err(String::new()),
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
    #[cfg(all(test, target_arch = "wasm32"))]
    test_support::pause_import_after_file_read().await;
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
    wait_for_reference_ticket(
        runtime,
        workspace,
        adapter,
        owner,
        generation,
        request_generation,
        outcome,
    )
    .await?;
    Ok((reference_id, asset_id, paths))
}

enum ModelAttachment {
    Individual { path: String, file: File },
    Directory { files: Vec<File> },
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
            let filenames = files.iter().map(File::name).collect::<Vec<_>>();
            let matched = matching::unique_directory_matches(paths, &filenames)
                .into_iter()
                .map(|(path, index)| (path, files[index].clone()))
                .collect::<Vec<_>>();
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
    wait_for_reference_ticket(
        runtime,
        workspace,
        adapter,
        owner,
        generation,
        request_generation,
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
                Err(message) if !message.is_empty() => {
                    error.set(Some(super::board_reference_effect::retryable_error(
                        message,
                        super::board_reference_owner_lineage_is_current(
                            &runtime, workspace, &adapter, &owner,
                        ),
                    )))
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
                && !message.is_empty()
            {
                error.set(Some(super::board_reference_effect::retryable_error(
                    message,
                    super::board_reference_owner_lineage_is_current(
                        &runtime, workspace, &adapter, &owner,
                    ),
                )));
            }
        }
    });
}

#[component]
pub fn Editor(
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
            let model_paths = model_paths;
            let paths_asset_id = paths_asset_id;
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
                    super::board_reference_effect::clear_missing_reference(
                        model_paths,
                        paths_asset_id,
                        attempted_discovery,
                        request_generation,
                        error,
                    );
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
                            Err(message) => {
                                error.set(Some(super::board_reference_effect::retryable_error(
                                    message,
                                    super::board_reference_owner_lineage_is_current(
                                        &runtime, workspace, &adapter, &owner,
                                    ),
                                )))
                            }
                            _ => {}
                        }
                    }
                });
            }
        },
    ));

    let action_tickets = use_signal(Vec::<(Action, EditTicket)>::new);
    let action_latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let version = use_context::<Signal<u64>>()();
    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let mut action_tickets = action_tickets;
        let mut error = error;
        move |_| {
            let mut entries = action_tickets.peek().clone();
            let before = entries.len();
            entries.retain(|(_, ticket)| {
                let message =
                    match ticket.settlement(super::board_reference_owner_lineage_is_current(
                        &runtime, workspace, &adapter, &owner,
                    )) {
                        Settlement::Pending => return true,
                        Settlement::Failed { message } => Some(message),
                        _ => None,
                    };
                if *action_latest.peek() == Some(ticket.operation()) {
                    error.set(message);
                }
                false
            });
            if entries.len() != before {
                action_tickets.set(entries);
            }
        }
    }));
    let on_action = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let reference_id = reference.as_ref().map(|reference| reference.id.clone());
        let mut action_tickets = action_tickets;
        let mut action_latest = action_latest;
        let mut error = error;
        move |action: Action| {
            let Some(reference_id) = reference_id.as_deref() else {
                return;
            };
            if action == Action::Remove && removal_pending(action_tickets) {
                return;
            }
            if let Some(ticket) = super::dispatch_board_reference_action(
                &runtime,
                workspace,
                &adapter,
                &owner,
                reference_id,
                action.clone(),
            ) {
                action_latest.set(Some(ticket.operation()));
                error.set(None);
                action_tickets.write().push((action, ticket));
            }
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

    let reference =
        reference.map(|reference| reference_with_drafts(reference, &action_tickets.read()));
    let options = assets
        .iter()
        .filter(|asset| matching::is_model_filename(&asset.name))
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
                        disabled,
                        aria_label: "Use routed PCB in assembly",
                        onchange: move |event: FormEvent| on_action.call(Action::SetEnabled(event.checked())),
                    }
                    "Use routed PCB in assembly"
                }
                div { class: "m1-board-reference-transform",
                    label { "X (mm)"
                        input {
                            r#type: "number", step: "0.1", value: "{reference.pose.at.x}",
                            disabled,
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
                            disabled,
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
                            disabled,
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
                        disabled,
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
                        let available_selection = matching::selected_available_asset(
                            &selected_asset_id,
                            options.iter().map(|asset| asset.id.as_str()),
                        );
                        let saved_asset_unavailable = !selected_asset_id.is_empty()
                            && available_selection.is_none();
                        let path_for_select = path.clone();
                        let path_for_file = path.clone();
                        rsx! {
                            label { key: "{path}", "Model asset for {path}"
                                select {
                                    aria_label: "Model asset for {path}",
                                    disabled,
                                    onchange: move |event: FormEvent| {
                                        let selected = event.value();
                                        on_action.call(Action::SetModelAsset {
                                            path: path_for_select.clone(),
                                            asset_id: if selected.is_empty() { None } else { Some(selected) },
                                        });
                                    },
                                    option { value: "", selected: selected_asset_id.is_empty(), "Resolve bundled model" }
                                    if saved_asset_unavailable {
                                        option { value: "{selected_asset_id}", selected: true, "Saved model asset is unavailable" }
                                    }
                                    for asset in &options {
                                        option { key: "{asset.id}", value: "{asset.id}", selected: available_selection == Some(asset.id.as_str()), "{asset.name}" }
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
                    disabled: disabled || busy || removal_pending(action_tickets),
                    onclick: move |_| on_action.call(Action::Remove),
                    "Remove PCB reference"
                }
            }

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

#[cfg(all(test, target_arch = "wasm32"))]
mod test_support {
    use futures_channel::oneshot;
    use std::cell::RefCell;

    struct ImportGate {
        entered: oneshot::Sender<()>,
        release: oneshot::Receiver<()>,
    }

    thread_local! {
        static IMPORT_GATE: RefCell<Option<ImportGate>> = const { RefCell::new(None) };
    }

    pub fn install_import_gate() -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        IMPORT_GATE.with(|gate| {
            *gate.borrow_mut() = Some(ImportGate {
                entered: entered_tx,
                release: release_rx,
            });
        });
        (entered_rx, release_tx)
    }

    pub async fn pause_import_after_file_read() {
        let gate = IMPORT_GATE.with(|gate| gate.borrow_mut().take());
        if let Some(gate) = gate {
            let _ = gate.entered.send(());
            let _ = gate.release.await;
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_async_tests {
    use super::*;
    use crate::runtime::{firmware_export_test_support, project_name_test_support as support};
    use boardstudio_application::{AcceptedSnapshot, Event, Scope};
    use boardstudio_core::model::{Asset, Board, BoardReference, Pose2, Vec2};
    use js_sys::{Array, Uint8Array};
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{File, HtmlElement};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct Fixture {
        runtime: Rc<Runtime>,
        accepted: AcceptedSnapshot,
        scope: Scope,
        reference: BoardReference,
        asset: Asset,
    }

    impl PartialEq for Fixture {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.runtime, &other.runtime)
                && self.accepted.token == other.accepted.token
                && self.scope == other.scope
                && self.reference == other.reference
                && self.asset == other.asset
        }
    }

    async fn fixture() -> Fixture {
        let runtime = support::new_runtime();
        let missing_sha = sha256_bytes(b"absent routed board browser-store fixture");
        let asset = Asset {
            id: "routed-board-asset".into(),
            name: "Left_PCB.kicad_pcb".into(),
            media_type: "application/x-kicad_pcb".into(),
            sha256: missing_sha,
            license: None,
            source: None,
        };
        let mut document = firmware_export_test_support::board_document();
        // A second board for the owner-change test to navigate to.
        document.boards.push(Board {
            id: "changed-routed-board-target".into(),
            name: "Changed target board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        let reference = BoardReference {
            id: "routed-board-reference".into(),
            board_id: document.boards[0].id.clone(),
            asset_id: asset.id.clone(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: Default::default(),
        };
        document.assets.push(asset.clone());
        document.board_references.push(reference.clone());
        support::open_document(&runtime, document).await;
        let accepted = runtime
            .model()
            .accepted
            .expect("the routed-board project is accepted");
        let scope = runtime
            .scope()
            .expect("the accepted routed-board project has a scope");
        Fixture {
            runtime,
            accepted,
            scope,
            reference,
            asset,
        }
    }

    #[component]
    fn missing_asset_host(fixture: Fixture) -> Element {
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        let workspace = use_signal(|| "Layout");
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| None);
        let generation = use_signal(|| 1u64);
        let adapter = use_hook(|| {
            super::super::SelectionAdapter::new(selected_context, anchor_scope, generation)
        });
        let owner = super::super::current_layout_owner(&fixture.runtime, workspace, &adapter);
        rsx! {
            Editor {
                reference: Some(fixture.reference.clone()),
                assets: vec![fixture.asset.clone()],
                disabled: false,
                runtime: BoardReferenceRuntimeHandle::new(fixture.runtime.clone()),
                workspace,
                adapter: BoardReferenceAdapterHandle::new(adapter),
                owner,
            }
        }
    }

    #[component]
    fn field_edit_host(fixture: Fixture) -> Element {
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        use_hook(|| {
            fixture.runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version.set(version() + 1);
            }));
        });
        let _ = version();
        let workspace = use_signal(|| "Layout");
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| None);
        let generation = use_signal(|| 1u64);
        let adapter = use_hook(|| {
            super::super::SelectionAdapter::new(selected_context, anchor_scope, generation)
        });
        let owner = super::super::current_layout_owner(&fixture.runtime, workspace, &adapter);
        let disabled = !super::super::board_reference_owner_is_current(
            &fixture.runtime,
            workspace,
            &adapter,
            &owner,
        );
        let snapshot = fixture.runtime.model().accepted.unwrap();
        rsx! {
            Editor {
                reference: snapshot.document.board_references.first().cloned(),
                assets: snapshot.document.assets.clone(),
                disabled,
                runtime: BoardReferenceRuntimeHandle::new(fixture.runtime.clone()),
                workspace,
                adapter: BoardReferenceAdapterHandle::new(adapter),
                owner,
            }
        }
    }

    #[wasm_bindgen_test]
    async fn reference_fields_queue_two_commits_keep_the_draft_and_undo_in_order() {
        let fixture = fixture().await;
        let runtime = &fixture.runtime;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new_with_props(
            field_edit_host,
            field_edit_hostProps {
                fixture: fixture.clone(),
            },
        );
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        TimeoutFuture::new(100).await;
        let x = root
            .query_selector("input[type='number']")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let (entered, release) = support::gate_next_core_reply(runtime);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        x.set_value("7.25");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        TimeoutFuture::new(80).await;
        support::drive_pending(runtime);
        entered.await.unwrap();
        TimeoutFuture::new(80).await;
        assert!(
            !x.disabled(),
            "coordinate fields stay enabled while pending"
        );
        assert_eq!(
            x.value(),
            "7.25",
            "the pending coordinate draft stays visible"
        );
        x.set_value("9.5");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        TimeoutFuture::new(80).await;
        assert_eq!(x.value(), "9.5");
        release.send(()).unwrap();
        for _ in 0..12 {
            support::run_pending(runtime).await;
            TimeoutFuture::new(30).await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.board_references[0]
                .pose
                .at
                .x,
            9.5
        );
        assert_eq!(x.value(), "9.5");
        for expected in [7.25, 0.0] {
            runtime.submit(Event::Undo {
                operation_id: runtime.operation(),
            });
            for _ in 0..8 {
                support::run_pending(runtime).await;
                TimeoutFuture::new(30).await;
            }
            assert_eq!(
                runtime.model().accepted.unwrap().document.board_references[0]
                    .pose
                    .at
                    .x,
                expected
            );
            assert_eq!(x.value(), expected.to_string());
        }
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_missing_stored_board_blob_reports_retry_and_keeps_reference() {
        let fixture = fixture().await;
        assert!(
            fixture
                .runtime
                .store
                .load_asset(fixture.asset.sha256.clone())
                .await
                .expect("IndexedDB asset lookup succeeds")
                .is_none(),
            "fixture SHA must be absent from browser storage"
        );
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("pcb-reference-missing-asset-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new_with_props(
            missing_asset_host,
            missing_asset_hostProps {
                fixture: fixture.clone(),
            },
        );
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );

        for _ in 0..100 {
            if root.query_selector("[role=alert]").unwrap().is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let alert = root
            .query_selector("[role=alert]")
            .unwrap()
            .expect("missing stored blob produces a mounted alert");
        assert!(
            alert
                .text_content()
                .unwrap()
                .contains("Saved routed-board file is missing from browser storage")
        );
        let buttons = root.query_selector_all("button").unwrap();
        let retry = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .find(|button| button.text_content().as_deref() == Some("Retry model-path discovery"))
            .expect("failed discovery offers retry")
            .dyn_into::<HtmlElement>()
            .unwrap();
        assert!(retry.get_attribute("disabled").is_none());
        retry.click();
        for _ in 0..100 {
            if root.query_selector("[role=alert]").unwrap().is_none() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        for _ in 0..100 {
            if root.query_selector("[role=alert]").unwrap().is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            root.query_selector("[role=alert]")
                .unwrap()
                .expect("retry runs the real missing-asset lookup again")
                .text_content()
                .unwrap()
                .contains("Saved routed-board file is missing from browser storage")
        );
        assert!(support::take_held_effects(&fixture.runtime).is_empty());
        assert!(
            fixture
                .runtime
                .model()
                .accepted
                .unwrap()
                .document
                .board_references
                .iter()
                .any(|reference| reference == &fixture.reference)
        );
        root.remove();
    }

    #[derive(Clone)]
    struct ReplaceHostProbe {
        fixture: Fixture,
        file: File,
        file_sha: String,
        busy: Rc<RefCell<Option<Signal<bool>>>>,
        error: Rc<RefCell<Option<Signal<Option<String>>>>>,
    }

    impl PartialEq for ReplaceHostProbe {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.busy, &other.busy)
        }
    }

    #[component]
    fn replace_host(probe: ReplaceHostProbe) -> Element {
        let workspace = use_signal(|| "Layout");
        let selected_context = use_signal(|| None);
        let anchor_scope = use_signal(|| None);
        let generation = use_signal(|| 1u64);
        let adapter = use_hook(|| {
            super::super::SelectionAdapter::new(selected_context, anchor_scope, generation)
        });
        let owner = super::super::current_layout_owner(&probe.fixture.runtime, workspace, &adapter);
        let request_generation = use_signal(|| 0u64);
        let busy = use_signal(|| false);
        let error = use_signal(|| None::<String>);
        let paths = use_signal(Vec::<String>::new);
        let paths_asset_id = use_signal(|| None::<String>);
        *probe.busy.borrow_mut() = Some(busy);
        *probe.error.borrow_mut() = Some(error);
        let runtime = probe.fixture.runtime.clone();
        let adapter_for_import = adapter.clone();
        let reference = probe.fixture.reference.clone();
        let file = probe.file.clone();
        rsx! {
            button {
                id: "pcb-reference-test-begin-replace",
                disabled: busy(),
                onclick: move |_| begin_board_import(
                    runtime.clone(),
                    workspace,
                    adapter_for_import.clone(),
                    owner.clone(),
                    Some(reference.clone()),
                    file.clone(),
                    request_generation,
                    busy,
                    error,
                    paths,
                    paths_asset_id,
                ),
                "Begin routed-board replacement"
            }
            if busy() { p { role: "status", "Reading routed board" } }
            if let Some(message) = error() { p { role: "alert", "{message}" } }
        }
    }

    #[derive(Clone, Copy)]
    enum OwnerChange {
        Project,
        Board,
        Revision,
    }

    /// Change the accepted owner through the real Session: open another project, navigate to
    /// another board, or land another revision of the same board.
    async fn changed_owner(fixture: &Fixture, change: OwnerChange) {
        let runtime = &fixture.runtime;
        match change {
            OwnerChange::Project => {
                let mut replacement = (*fixture.accepted.document).clone();
                replacement.id = "changed-routed-board-project".into();
                runtime.submit(Event::Open {
                    operation_id: runtime.operation(),
                    document: replacement,
                });
                support::run_pending(runtime).await;
            }
            OwnerChange::Board => support::navigate(runtime, "changed-routed-board-target").await,
            OwnerChange::Revision => {
                let mut document = (*fixture.accepted.document).clone();
                document.name.push_str(" revised");
                support::replace_document(
                    runtime,
                    "routed-board-revision-change",
                    &fixture.scope.board_id,
                    document,
                )
                .await;
            }
        }
    }

    fn replacement_file(salt: usize) -> (File, String) {
        let mut bytes = include_bytes!("../../../tests/fixtures/replacement.kicad_pcb").to_vec();
        // Trailing whitespace preserves the fixture as KiCad text while giving
        // each stale-owner case a unique BrowserStore identity.
        bytes.extend(std::iter::repeat_n(b'\n', salt));
        let sha256 = sha256_bytes(&bytes);
        let contents = Uint8Array::from(bytes.as_slice());
        let file = File::new_with_u8_array_sequence(
            &Array::of1(&contents.into()),
            "Replacement_PCB.kicad_pcb",
        )
        .expect("supported routed-board File fixture constructs");
        (file, sha256)
    }

    #[wasm_bindgen_test]
    async fn replace_discards_async_file_read_after_project_board_or_revision_changes() {
        for (index, change) in [
            OwnerChange::Project,
            OwnerChange::Board,
            OwnerChange::Revision,
        ]
        .into_iter()
        .enumerate()
        {
            let fixture = fixture().await;
            let (file, file_sha) = replacement_file(index + 1);
            let probe = ReplaceHostProbe {
                fixture: fixture.clone(),
                file,
                file_sha,
                busy: Rc::new(RefCell::new(None)),
                error: Rc::new(RefCell::new(None)),
            };
            let document = web_sys::window().unwrap().document().unwrap();
            let root = document.create_element("div").unwrap();
            root.set_id(&format!("pcb-reference-replace-stale-test-{index}"));
            document.body().unwrap().append_child(&root).unwrap();
            dioxus_web::launch::launch_virtual_dom(
                VirtualDom::new_with_props(
                    replace_host,
                    replace_hostProps {
                        probe: probe.clone(),
                    },
                ),
                dioxus_web::Config::new().rootnode(root.clone().into()),
            );
            let (entered, release) = test_support::install_import_gate();
            for _ in 0..100 {
                if root
                    .query_selector("#pcb-reference-test-begin-replace")
                    .unwrap()
                    .is_some()
                {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(10).await;
            }
            root.query_selector("#pcb-reference-test-begin-replace")
                .unwrap()
                .unwrap()
                .dyn_into::<HtmlElement>()
                .unwrap()
                .click();
            entered
                .await
                .expect("real File.array_buffer read reaches gate");
            changed_owner(&fixture, change).await;
            let accepted_after_change = fixture
                .runtime
                .model()
                .accepted
                .expect("project stays accepted");
            release.send(()).expect("release file-read admission gate");

            let busy = probe.busy.borrow().expect("mounted busy signal");
            let error = probe.error.borrow().expect("mounted error signal");
            for _ in 0..100 {
                if !busy() && error().is_some() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(10).await;
            }
            assert!(
                !busy(),
                "stale operation settles and re-enables replacement"
            );
            let message = error().expect("changed owner reports a retryable failure");
            match change {
                OwnerChange::Project | OwnerChange::Board => {
                    assert!(message.contains("result was discarded"));
                }
                OwnerChange::Revision => {
                    assert!(message.contains("changed while the routed-board file was read"));
                }
            }
            assert!(
                root.query_selector("#pcb-reference-test-begin-replace:not([disabled])")
                    .unwrap()
                    .is_some(),
                "replacement action remains usable for retry"
            );
            assert!(
                support::take_held_effects(&fixture.runtime).is_empty(),
                "no project edit may be submitted after the stale file read"
            );
            let accepted_after_read = fixture
                .runtime
                .model()
                .accepted
                .expect("project stays accepted");
            assert_eq!(accepted_after_read.token, accepted_after_change.token);
            assert_eq!(
                accepted_after_read.document.revision, accepted_after_change.document.revision,
                "the stale file read changed no accepted document"
            );
            let stored = fixture
                .runtime
                .store
                .load_asset(probe.file_sha.clone())
                .await
                .expect("IndexedDB asset lookup succeeds");
            assert!(
                stored.is_none(),
                "stale read must not store replacement bytes"
            );
            root.remove();
        }
    }
}
