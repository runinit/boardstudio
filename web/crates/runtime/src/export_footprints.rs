//! Private standalone KiCad footprint ZIP adapter for the Export workspace.
//!
//! Rust Core plans, renders and serializes the footprints in one `ExportPcb`
//! request; the existing Core archive worker packs the resulting files.

use boardstudio_application::{AcceptedSnapshot, OperationId};
use boardstudio_core::model::{
    ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, ExportTarget,
    PrepareExportRequest,
};
use boardstudio_web_host::host::{BrowserStore, CoreExecutor};
use js_sys::Uint8Array;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub struct ExportSource<'a> {
    pub operation_id: OperationId,
    pub snapshot: &'a AcceptedSnapshot,
    pub core: &'a dyn CoreExecutor,
    pub store: &'a BrowserStore,
    pub executor_epoch: u64,
}

pub async fn build_zip(
    source: ExportSource<'_>,
    is_current: impl Fn() -> Result<(), String>,
) -> Result<Vec<u8>, String> {
    let ExportSource {
        operation_id,
        snapshot,
        core,
        store,
        executor_epoch,
    } = source;
    let document = snapshot.document.as_ref();
    is_current()?;
    if document.definitions.is_empty() {
        return Err("Add a component definition before exporting footprints.".into());
    }

    let model_ids = crate::bundled_models::footprint_export_model_ids(document).await;
    is_current()?;
    let model_ids = model_ids?;
    let model_files = load_model_files(document, store, &model_ids, &is_current).await;
    is_current()?;
    let (model_paths, mut files) = model_files?;

    let definition_ids = document
        .definitions
        .iter()
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    let target = ExportTarget::StandaloneFootprints { definition_ids };
    let request_id = format!("footprints-{}", operation_id.0);
    let request = ArtifactRequest::ExportPcb {
        id: request_id.clone(),
        request: PrepareExportRequest {
            snapshot_token: snapshot.token.0.to_string(),
            expected_revision: snapshot.document.revision,
            document: document.clone(),
            target,
            contours: vec![],
            model_paths,
        },
    };
    is_current()?;
    let reply = core
        .artifact(&request_id, &executor_epoch.to_string(), &request)
        .await;
    is_current()?;
    let reply = reply.map_err(|error| format!("Footprint export failed: {error}"))?;
    let exported = match reply {
        ArtifactReply::ExportPcb { id, result } if id == request_id => result,
        ArtifactReply::Error { id, error } if id == request_id => {
            return Err(format!("Core rejected footprint export: {}", error.message));
        }
        ArtifactReply::ExportPcb { .. } | ArtifactReply::Error { .. } => {
            return Err("Core returned footprint files for another request.".into());
        }
        _ => return Err("Core returned an unexpected footprint export reply.".into()),
    };
    if exported.snapshot_token != snapshot.token.0.to_string()
        || exported.revision != snapshot.document.revision
    {
        return Err("Core returned footprints for another accepted revision.".into());
    }

    for file in exported.files {
        push_file(
            &mut files,
            format!("BoardStudio.pretty/{}", file.filename),
            file.content.into_bytes(),
        )?;
    }
    push_file(
        &mut files,
        "fp-lib-table".into(),
        b"(fp_lib_table (lib (name \"BoardStudio\") (type \"KiCad\") (uri \"${KIPRJMOD}/BoardStudio.pretty\") (options \"\") (descr \"\")))\n".to_vec(),
    )?;
    let mut utility_notice = vec![
        "Standalone footprint libraries contain footprints only.".to_owned(),
        "Generated board routing and graphics are included by placed board export.".to_owned(),
    ];
    if !exported.skipped_utilities.is_empty() {
        utility_notice.push(String::new());
        utility_notice.push("Generators skipped from this standalone footprint library:".into());
        utility_notice.extend(
            exported
                .skipped_utilities
                .iter()
                .map(|name| format!("- {name}")),
        );
    }
    utility_notice.push(String::new());
    push_file(
        &mut files,
        "BOARD-UTILITIES.txt".into(),
        utility_notice.join("\n").into_bytes(),
    )?;

    is_current()?;
    let entries = files
        .iter()
        .enumerate()
        .map(|(index, (path, _))| {
            Ok(ArchiveEntry {
                path: path.clone(),
                buffer_index: u32::try_from(index)
                    .map_err(|_| "Too many footprint archive files.".to_owned())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
        .map_err(|error| format!("Could not prepare footprint ZIP request: {error}"))?;
    let buffers = files
        .iter()
        .map(|(_, contents)| Uint8Array::from(contents.as_slice()))
        .collect();
    let archive_id = format!("footprints-{}-archive", operation_id.0);
    let packed = core
        .archive(&archive_id, &executor_epoch.to_string(), &metadata, buffers)
        .await;
    is_current()?;
    let packed = packed.map_err(|error| format!("Footprint ZIP packaging failed: {error}"))?;
    match serde_json::from_str::<ArchiveReply>(&packed.metadata)
        .map_err(|error| format!("Could not read footprint ZIP result: {error}"))?
    {
        ArchiveReply::Packed => packed
            .buffers
            .first()
            .map(Uint8Array::to_vec)
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| "Footprint ZIP provider returned no bytes.".into()),
        ArchiveReply::Error { message } => {
            Err(format!("Footprint ZIP packaging failed: {message}"))
        }
        ArchiveReply::Unpacked { .. } => {
            Err("Core returned an unpacked project for footprint export.".into())
        }
    }
}

async fn load_model_files(
    document: &boardstudio_core::model::ProjectDoc,
    store: &BrowserStore,
    model_ids: &[String],
    is_current: &impl Fn() -> Result<(), String>,
) -> Result<(BTreeMap<String, String>, Vec<(String, Vec<u8>)>), String> {
    let mut model_paths = BTreeMap::new();
    let mut files = Vec::<(String, Vec<u8>)>::new();
    for id in model_ids {
        is_current()?;
        let asset = document.assets.iter().find(|asset| asset.id == *id);
        let bundled = asset
            .is_none()
            .then(|| crate::bundled_models::bundled_model(id))
            .flatten();
        let filename = asset
            .map(|asset| asset.name.as_str())
            .or_else(|| bundled.map(|model| model.filename))
            .ok_or_else(|| format!("Model asset {id} needs a STEP, STP, STL, or WRL filename"))?;
        let extension = model_extension(filename).ok_or_else(|| {
            format!("Model asset {filename} needs a STEP, STP, STL, or WRL filename")
        })?;
        let (digest, bytes) = if let Some(asset) = asset {
            let result = store.load_asset(asset.sha256.clone()).await;
            is_current()?;
            let bytes = result
                .map_err(|error| format!("Could not load model asset {}: {error}", asset.name))?
                .ok_or_else(|| format!("Model asset {} is missing", asset.name))?
                .to_vec();
            (asset.sha256.clone(), bytes)
        } else {
            let model =
                bundled.ok_or_else(|| format!("Bundled model asset {id} is unavailable"))?;
            let bytes = crate::bundled_models::bundled_model_bytes(id).await;
            is_current()?;
            let bytes = bytes?;
            (model.sha256.to_owned(), bytes)
        };
        let actual_digest = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual_digest != digest.to_ascii_lowercase() {
            return Err(format!(
                "Model asset {filename} failed its content hash check"
            ));
        }
        let path = format!("models/{digest}.{extension}");
        model_paths.insert(id.clone(), path.clone());
        if let Some((_, existing)) = files
            .iter()
            .find(|(existing_path, _)| existing_path == &path)
        {
            if existing != &bytes {
                return Err(format!("Conflicting model files resolve to {path}"));
            }
        } else {
            files.push((path, bytes));
        }
    }
    Ok((model_paths, files))
}

fn model_extension(filename: &str) -> Option<&'static str> {
    match filename.rsplit_once('.')?.1.to_ascii_lowercase().as_str() {
        "step" => Some("step"),
        "stp" => Some("stp"),
        "stl" => Some("stl"),
        "wrl" => Some("wrl"),
        _ => None,
    }
}

fn push_file(
    files: &mut Vec<(String, Vec<u8>)>,
    path: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if files.iter().any(|(existing, _)| existing == &path) {
        return Err(format!("Footprint export path repeats: {path}"));
    }
    files.push((path, bytes));
    Ok(())
}
