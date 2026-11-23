//! Scoped controls and file-backed editing for routed-board references.
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_model::model::{Asset, BoardReference};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{
    FilePickerButton, InspectorSection, NumericField, SelectField, TOUCH_CLS, ToggleField,
};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use js_sys::Uint8Array;
use std::{collections::BTreeSet, future::Future, rc::Rc};
use wasm_bindgen_futures::JsFuture;
use web_sys::File;

use boardstudio_web_host::host::AssetBytes;

use crate::runtime::Runtime;

#[path = "pcb_board_reference/matching.rs"]
mod matching;

const MAX_REFERENCE_BYTES: f64 = 32.0 * 1024.0 * 1024.0;

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

fn preview_paths(models: &[boardstudio_model::model::PcbModel]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    models
        .iter()
        .filter_map(|model| {
            let path = model.path.as_str();
            (!path.is_empty() && seen.insert(path.to_owned())).then(|| path.to_owned())
        })
        .collect()
}

#[derive(Clone)]
struct BoardReferenceContext {
    runtime: Rc<Runtime>,
    workspace: Signal<Workspace>,
    adapter: super::SelectionAdapter,
    owner: super::LayoutOwnerIdentity,
    generation: Signal<u64>,
    request_generation: u64,
}

impl BoardReferenceContext {
    fn new(
        runtime: Rc<Runtime>,
        workspace: Signal<Workspace>,
        adapter: super::SelectionAdapter,
        owner: super::LayoutOwnerIdentity,
        generation: Signal<u64>,
        request_generation: u64,
    ) -> Self {
        Self {
            runtime,
            workspace,
            adapter,
            owner,
            generation,
            request_generation,
        }
    }

    fn is_current(&self) -> bool {
        super::board_reference_owner_is_current(
            &self.runtime,
            self.workspace,
            &self.adapter,
            &self.owner,
        )
    }

    fn is_live(&self) -> bool {
        self.generation.cloned() == self.request_generation && self.is_current()
    }

    fn current_or(&self, message: &str) -> Result<(), String> {
        self.is_current()
            .then_some(())
            .ok_or_else(|| message.to_owned())
    }

    fn live_or(&self, message: &str) -> Result<(), String> {
        self.is_live()
            .then_some(())
            .ok_or_else(|| message.to_owned())
    }

    fn accepted(&self) -> Result<boardstudio_application::AcceptedSnapshot, String> {
        self.runtime
            .model()
            .accepted
            .ok_or_else(|| "No accepted project is open.".to_owned())
    }

    fn lineage_is_current(&self) -> bool {
        super::board_reference_owner_lineage_is_current(
            &self.runtime,
            self.workspace,
            &self.adapter,
            &self.owner,
        )
    }

    /// Request context for a user-initiated board/model task.
    fn for_request(
        runtime: Rc<Runtime>,
        workspace: Signal<Workspace>,
        adapter: &super::SelectionAdapter,
        owner: &super::LayoutOwnerIdentity,
        generation: Signal<u64>,
    ) -> Self {
        Self::new(
            runtime,
            workspace,
            adapter.clone(),
            owner.clone(),
            generation,
            0,
        )
    }

    /// Run `task` under the next request epoch (unless busy) and settle
    /// busy/error via `on_ok`, keeping the `request_generation` epoch gating.
    fn spawn_request<T: 'static>(
        mut self,
        mut busy: Signal<bool>,
        mut error: Signal<Option<String>>,
        task: impl Future<Output = Result<T, String>> + 'static,
        on_ok: impl FnOnce(T) + 'static,
    ) {
        if busy() {
            return;
        }
        let epoch = (self.generation)().wrapping_add(1);
        self.generation.set(epoch);
        busy.set(true);
        error.set(None);
        self.request_generation = epoch;
        spawn(async move {
            let result = task.await;
            if (self.generation)() != epoch {
                return;
            }
            busy.set(false);
            match result {
                Ok(value) => on_ok(value),
                Err(message) if !message.is_empty() => {
                    error.set(Some(super::board_reference_effect::retryable_error(
                        message,
                        self.lineage_is_current(),
                    )));
                }
                Err(_) => {}
            }
        });
    }
}

/// Push `asset` onto the proposed document, rejecting identity collisions.
fn push_unique_asset(
    proposed: &mut boardstudio_model::model::ProjectDoc,
    asset: Asset,
    conflict: &str,
) -> Result<(), String> {
    if proposed.assets.iter().any(|current| current.id == asset.id) {
        return Err(conflict.into());
    }
    proposed.assets.push(asset);
    Ok(())
}

async fn discover_stored_paths(
    ctx: &BoardReferenceContext,
    reference: &BoardReference,
) -> Result<Vec<String>, String> {
    ctx.current_or("The active board changed before routed-board model paths were read.")?;
    let accepted = ctx.accepted()?;
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
    let bytes = ctx
        .runtime
        .store
        .load_asset(asset.sha256.clone())
        .await
        .map_err(|error| format!("Could not load routed-board file: {error}"))?
        .ok_or_else(|| "Saved routed-board file is missing from browser storage.".to_owned())?
        .to_vec();
    ctx.live_or("The routed-board source changed while its asset was loading.")?;
    if bytes.is_empty()
        || bytes.len() as f64 > MAX_REFERENCE_BYTES
        || sha256_hex(&bytes) != asset.sha256
    {
        return Err("Saved routed-board file failed its size or SHA-256 check.".into());
    }
    let source = String::from_utf8(bytes)
        .map_err(|error| format!("Saved routed-board file is not valid UTF-8: {error}"))?;
    let operation = ctx.runtime.operation();
    let preview = ctx
        .runtime
        .preview_routed_board_source(
            format!("board-reference-discover-{}", operation.0),
            source,
            ctx.owner
                .revision
                .ok_or("Accepted board revision is unavailable.")?,
        )
        .await?;
    ctx.live_or("The routed-board source changed while Core discovered model paths.")?;
    if Some(preview.revision) != ctx.owner.revision {
        return Err("Core returned model paths for a different routed-board revision.".into());
    }
    Ok(preview_paths(&preview.models))
}

async fn import_routed_board(
    ctx: &BoardReferenceContext,
    reference: Option<&BoardReference>,
    file: File,
) -> Result<(String, Vec<String>), String> {
    ctx.current_or("The active project or board changed. Retry with the current board.")?;
    let (bytes, source) = read_kicad_file(&file).await?;
    ctx.live_or("The project or board changed while the routed-board file was read.")?;
    let revision = ctx
        .owner
        .revision
        .ok_or("Accepted board revision is unavailable.")?;
    let operation = ctx.runtime.operation();
    let preview = ctx
        .runtime
        .preview_routed_board_source(
            format!("board-reference-import-{}", operation.0),
            source,
            revision,
        )
        .await?;
    ctx.live_or("The project or board changed while Core previewed the routed board.")?;
    if preview.revision != revision {
        return Err("Core returned a routed-board preview for a different revision.".into());
    }
    let paths = preview_paths(&preview.models);
    let asset_id = crate::runtime::new_project_id()?;
    let sha256 = sha256_hex(&bytes);
    let asset = Asset {
        id: asset_id.clone(),
        name: file.name(),
        media_type: "application/x-kicad_pcb".into(),
        sha256: sha256.clone(),
        license: None,
        source: None,
    };
    ctx.runtime
        .store
        .save_asset(AssetBytes { sha256, bytes })
        .await
        .map_err(|error| format!("Could not store routed-board file: {error}"))?;
    ctx.live_or("The project or board changed while the routed-board asset was stored.")?;
    let accepted = ctx.accepted()?;
    let scope = ctx
        .owner
        .scope
        .as_ref()
        .ok_or("Active board is unavailable.")?;
    let mut proposed = accepted.document.as_ref().clone();
    push_unique_asset(
        &mut proposed,
        asset,
        "Could not allocate a unique routed-board asset identity.",
    )?;
    if let Some(reference) = reference {
        let existing = proposed
            .board_references
            .iter_mut()
            .find(|current| current.id == reference.id && current.board_id == scope.board_id)
            .ok_or("The routed-board reference changed. Reopen the panel and retry.")?;
        existing.asset_id = asset_id.clone();
        existing.enabled = true;
    } else {
        proposed.board_references.push(BoardReference {
            id: crate::runtime::new_project_id()?,
            board_id: scope.board_id.clone(),
            asset_id: asset_id.clone(),
            enabled: true,
            pose: boardstudio_model::model::Pose2 {
                at: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: Default::default(),
        });
    }
    super::submit_board_reference_document(
        &ctx.runtime,
        ctx.is_live(),
        &ctx.owner,
        proposed,
        "board-reference-import",
    )?;
    Ok((asset_id, paths))
}

enum ModelAttachment {
    Individual { path: String, file: File },
    Directory { files: Vec<File> },
}

async fn attach_model_files(
    ctx: &BoardReferenceContext,
    reference: &BoardReference,
    paths: &[String],
    attachment: ModelAttachment,
) -> Result<(), String> {
    use super::model_asset_import::read_model_file;

    ctx.current_or("The active project or board changed. Retry with the current board.")?;
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
    let accepted = ctx.accepted()?;
    let scope = ctx
        .owner
        .scope
        .as_ref()
        .ok_or("Active board is unavailable.")?;
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
    // Staged attachments accumulate in the proposed document, so the helper
    // rejects collisions with both saved and earlier staged assets.
    let mut imported_assets: Vec<(String, String)> = Vec::with_capacity(uploads.len());
    for (path, file) in uploads.drain(..) {
        ctx.live_or("The project or board changed while model files were being attached.")?;
        let imported = read_model_file(file).await?;
        ctx.live_or("The project or board changed while a model file was read.")?;
        imported
            .store(&ctx.runtime.store)
            .await
            .map_err(|error| format!("Could not store model asset: {error}"))?;
        ctx.live_or("The project or board changed while a model asset was stored.")?;
        let asset = Asset {
            id: crate::runtime::new_project_id()?,
            name: imported.filename,
            media_type: imported.media_type,
            sha256: imported.sha256,
            license: None,
            source: None,
        };
        push_unique_asset(
            &mut proposed,
            asset.clone(),
            "Could not allocate a unique model asset identity.",
        )?;
        imported_assets.push((path, asset.id));
    }
    for (path, asset_id) in imported_assets {
        let Some(current) = proposed
            .board_references
            .iter_mut()
            .find(|current| current.id == reference.id && current.board_id == scope.board_id)
        else {
            return Err(
                "The routed-board reference changed before model attachment completed.".into(),
            );
        };
        current.model_assets.insert(path, asset_id);
    }
    super::submit_board_reference_document(
        &ctx.runtime,
        ctx.is_live(),
        &ctx.owner,
        proposed,
        "board-reference-model-attach",
    )
}

fn begin_board_import(
    ctx: BoardReferenceContext,
    reference: Option<BoardReference>,
    file: File,
    busy: Signal<bool>,
    error: Signal<Option<String>>,
    mut paths: Signal<Vec<String>>,
    mut paths_asset_id: Signal<Option<String>>,
) {
    let task_ctx = ctx.clone();
    ctx.spawn_request(
        busy,
        error,
        async move { import_routed_board(&task_ctx, reference.as_ref(), file).await },
        move |(asset_id, discovered)| {
            paths.set(discovered);
            paths_asset_id.set(Some(asset_id));
        },
    );
}

fn begin_model_attachment(
    ctx: BoardReferenceContext,
    reference: Option<BoardReference>,
    paths: Vec<String>,
    attachment: ModelAttachment,
    busy: Signal<bool>,
    error: Signal<Option<String>>,
) {
    let Some(reference) = reference else {
        return;
    };
    let task_ctx = ctx.clone();
    ctx.spawn_request(
        busy,
        error,
        async move { attach_model_files(&task_ctx, &reference, &paths, attachment).await },
        |()| {},
    );
}

#[component]
pub fn Editor(
    reference: Option<BoardReference>,
    assets: Vec<Asset>,
    disabled: bool,
    runtime: BoardReferenceRuntimeHandle,
    workspace: Signal<Workspace>,
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
            let mut attempted_discovery = attempted_discovery;
            let busy_signal = busy;
            let mut model_paths = model_paths;
            let mut paths_asset_id = paths_asset_id;
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
                let ctx = BoardReferenceContext::for_request(
                    runtime.clone(),
                    workspace,
                    &adapter,
                    &owner,
                    request_generation,
                );
                let source_asset_id = reference.asset_id.clone();
                let task_ctx = ctx.clone();
                let done_ctx = ctx.clone();
                ctx.spawn_request(
                    busy_signal,
                    error,
                    async move { discover_stored_paths(&task_ctx, &reference).await },
                    move |paths| {
                        if done_ctx.lineage_is_current() {
                            model_paths.set(paths);
                            paths_asset_id.set(Some(source_asset_id));
                        }
                    },
                );
            }
        },
    ));

    let _ = use_context::<Signal<u64>>()();
    let on_action = use_callback({
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = owner.clone();
        let reference_id = reference.as_ref().map(|reference| reference.id.clone());
        let mut error = error;
        move |action: Action| {
            let Some(reference_id) = reference_id.as_deref() else {
                return;
            };
            if let Some(result) = super::dispatch_board_reference_action(
                &runtime,
                workspace,
                &adapter,
                &owner,
                reference_id,
                action,
            ) {
                error.set(result.err());
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
                BoardReferenceContext::for_request(
                    runtime.clone(),
                    workspace,
                    &adapter,
                    &owner,
                    request_generation,
                ),
                reference.clone(),
                file,
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
                BoardReferenceContext::for_request(
                    runtime.clone(),
                    workspace,
                    &adapter,
                    &owner,
                    request_generation,
                ),
                reference.clone(),
                model_paths(),
                ModelAttachment::Individual { path, file },
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
                BoardReferenceContext::for_request(
                    runtime.clone(),
                    workspace,
                    &adapter,
                    &owner,
                    request_generation,
                ),
                reference.clone(),
                model_paths(),
                ModelAttachment::Directory { files },
                busy,
                error,
            );
        }
    });

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
        InspectorSection { title: "Routed PCB reference", default_open: false,
            p { "Preview a routed KiCad board with the case. Replace this reference after editing routing in KiCad." }
            FilePickerButton {
                    label: if reference.is_some() { "Replace KiCad board" } else { "Import KiCad board" },
                    accept: ".kicad_pcb",
                    disabled: disabled || busy,
                    on_files: move |files: Vec<File>| if let Some(file) = files.into_iter().next() { on_import.call(file); },
            }
            if let Some(reference) = reference.as_ref() {
                ToggleField { label: "Use routed PCB in assembly", checked: reference.enabled,
                    disabled_reason: disabled.then(|| "Routed PCB reference is read-only".to_owned()),
                    on_commit: move |enabled| on_action.call(Action::SetEnabled(enabled)),
                }
                div {
                    for (label, value, make, step) in [
                        ("X (mm)", reference.pose.at.x, Action::SetPositionX as fn(f64) -> Action, 0.1),
                        ("Y (mm)", reference.pose.at.y, Action::SetPositionY as fn(f64) -> Action, 0.1),
                        ("Z (mm)", reference.elevation, Action::SetElevation as fn(f64) -> Action, 0.1),
                        ("Rotation (°)", reference.pose.rotation, Action::SetRotation as fn(f64) -> Action, 1.0),
                    ] {
                        NumericField { key: "{label}", label: label.to_owned(), value, step: Some(step),
                            disabled_reason: disabled.then(|| "Routed PCB reference is read-only".to_owned()),
                            on_commit: move |value| on_action.call(make(value)),
                        }
                    }
                }
                if !current_paths.is_empty() {
                    FilePickerButton { label: "Attach model directory", directory: true,
                            disabled: disabled || busy,
                            on_files: move |files: Vec<File>| on_attach_directory.call(files),
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
                            div { key: "{path}", class: "grid gap-2",
                                SelectField::<String> {
                                    label: format!("Model asset for {path}"), aria_label: format!("Model asset for {path}"), value: selected_asset_id.clone(),
                                    disabled,
                                    on_commit: move |selected: String| {
                                        on_action.call(Action::SetModelAsset {
                                            path: path_for_select.clone(),
                                            asset_id: if selected.is_empty() { None } else { Some(selected) },
                                        });
                                    },
                                    options: std::iter::once((String::new(), "Resolve bundled model".into()))
                                        .chain(saved_asset_unavailable.then(|| (selected_asset_id.clone(), "Saved model asset is unavailable".into())))
                                        .chain(options.iter().map(|asset| (asset.id.clone(), asset.name.clone()))).collect(),
                                }
                                FilePickerButton { label: format!("Attach model for {path}"), accept: ".step,.stp,.stl,.wrl",
                                    disabled: disabled || busy,
                                    on_files: move |files: Vec<File>| if let Some(file) = files.into_iter().next() {
                                        on_attach_file.call((path_for_file.clone(), file));
                                    },
                                }
                            }
                        }
                    }
                }
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS,
                    disabled: disabled || busy,
                    onclick: move |_| on_action.call(Action::Remove),
                    "Remove PCB reference"
                }
            }

            if let Some(message) = error() { Alert { variant: AlertVariant::Destructive, "{message}" } }
            if reference.is_some() && error().is_some() && current_paths.is_empty() {
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS,
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
