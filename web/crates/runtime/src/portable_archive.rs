use boardstudio_core_kernel::hash::sha256_hex;
#[cfg(any(test, target_arch = "wasm32"))]
use boardstudio_model::model::ArchiveReply;
#[cfg(target_arch = "wasm32")]
use boardstudio_model::model::ArchiveEntry;
use boardstudio_model::model::{ArchiveRequest, Asset, ProjectDoc};
use std::collections::{BTreeMap, HashMap, HashSet};

#[cfg(target_arch = "wasm32")]
pub(crate) fn pack_files(
    files: impl IntoIterator<Item = (String, Vec<u8>)>,
) -> Result<Vec<u8>, String> {
    let mut entries = Vec::new();
    let mut buffers = Vec::new();
    for (path, bytes) in files {
        let buffer_index = u32::try_from(buffers.len()).map_err(|_| "Too many archive files.")?;
        entries.push(ArchiveEntry { path, buffer_index });
        buffers.push(bytes);
    }
    let request = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
        .map_err(|error| error.to_string())?;
    let (metadata, buffers) = boardstudio_core::archive::request(&request, &buffers);
    match serde_json::from_str(&metadata).map_err(|error| error.to_string())? {
        ArchiveReply::Packed => buffers
            .into_iter()
            .next()
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| "Archive returned no ZIP bytes.".into()),
        ArchiveReply::Error { message } => Err(message),
        ArchiveReply::Unpacked { .. } => Err("Archive returned an unpacked project.".into()),
    }
}

pub struct PreparedArchive {
    pub metadata: String,
    pub buffers: Vec<Vec<u8>>,
}

pub fn prepare_archive(
    document: &ProjectDoc,
    local_asset_bytes: &BTreeMap<String, Vec<u8>>,
    bundled_bytes: Vec<(&'static crate::bundled_models::BundledModel, Vec<u8>)>,
    embed_used_models: bool,
) -> Result<PreparedArchive, String> {
    let mut packed_document = document.clone();
    let mut buffers = Vec::<Vec<u8>>::new();
    let mut archive_entries = Vec::new();
    let mut path_indices = HashMap::<String, u32>::new();
    let mut asset_ids = document
        .assets
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<HashSet<_>>();

    fn add_file(
        path: String,
        bytes: Vec<u8>,
        buffers: &mut Vec<Vec<u8>>,
        entries: &mut Vec<serde_json::Value>,
        indices: &mut HashMap<String, u32>,
    ) -> Result<(), String> {
        if let Some(index) = indices.get(&path) {
            if buffers[*index as usize] != bytes {
                return Err(format!("Conflicting project archive payload for {path}"));
            }
            return Ok(());
        }
        let index = u32::try_from(buffers.len())
            .map_err(|_| "Too many project archive buffers".to_owned())?;
        entries.push(serde_json::json!({"path": path, "bufferIndex": index}));
        buffers.push(bytes);
        indices.insert(path, index);
        Ok(())
    }

    if embed_used_models {
        for (model, bytes) in bundled_bytes {
            if !asset_ids.insert(model.id.to_owned()) {
                continue;
            }
            let sha256 = sha256_hex(&bytes);
            packed_document.assets.push(Asset {
                id: model.id.to_owned(),
                name: model.filename.to_owned(),
                media_type: model.media_type.to_owned(),
                sha256: sha256.clone(),
                license: None,
                source: Some("bundled footprint library".into()),
            });
            add_file(
                format!("assets/{sha256}"),
                bytes,
                &mut buffers,
                &mut archive_entries,
                &mut path_indices,
            )?;
        }
    }

    for asset in &document.assets {
        let bytes = local_asset_bytes
            .get(&asset.sha256)
            .cloned()
            .ok_or_else(|| format!("Missing asset: {}", asset.name))?;
        add_file(
            format!("assets/{}", asset.sha256),
            bytes,
            &mut buffers,
            &mut archive_entries,
            &mut path_indices,
        )?;
    }

    let project_json =
        serde_json::to_string(&packed_document).map_err(|error| error.to_string())?;
    let archive_json = serde_json::json!({"embedUsedModels": embed_used_models}).to_string();
    let metadata = serde_json::json!({
        "kind": "pack-project",
        "projectJson": project_json,
        "archiveJson": archive_json,
        "assets": archive_entries,
    })
    .to_string();
    Ok(PreparedArchive { metadata, buffers })
}

/// Resolves all directly persisted bundled references plus the generated
/// references collected from the generators' authoritative model asset IDs.
/// Generated IDs already owned by the document use its local assets; other
/// generated IDs are validated strictly. Saved metadata keeps
/// React's behavior of including only IDs present in its bundled catalogue.
pub fn referenced_models(
    document: &ProjectDoc,
    generated_ids: impl IntoIterator<Item = String>,
) -> Result<Vec<&'static crate::bundled_models::BundledModel>, String> {
    use std::collections::HashSet;

    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    let mut add_known = |id: &str| {
        if crate::bundled_models::bundled_model(id).is_some() && seen.insert(id.to_owned()) {
            ids.push(id.to_owned());
        }
    };

    // Existing document assets own their bytes even when a generator names them.
    // Only generated references without a local owner require the bundled provider.
    for id in generated_ids {
        if document.assets.iter().any(|asset| asset.id == id) {
            continue;
        }
        if crate::bundled_models::bundled_model(&id).is_none() {
            return Err(format!("Bundled model is unavailable: {id}"));
        }
        add_known(&id);
    }

    // Saved metadata references are filtered through the same catalogue
    // lookup as storage.ts. This intentionally includes unplaced definitions.
    for definition in &document.definitions {
        for model in definition.models.iter().flatten() {
            add_known(&model.asset_id);
        }
    }
    for definition in &document.module_definitions {
        for model in &definition.models {
            add_known(&model.asset_id);
        }
        if let Some(circuit) = &definition.circuit {
            for part_definition in &circuit.definitions {
                for model in part_definition.models.iter().flatten() {
                    add_known(&model.asset_id);
                }
            }
        }
    }
    for assembly in &document.assemblies {
        for member in &assembly.members {
            for model in &member.models {
                add_known(&model.asset_id);
            }
        }
    }
    for reference in &document.board_references {
        for id in reference.model_assets.values() {
            add_known(id);
        }
    }

    Ok(ids
        .iter()
        .filter_map(|id| crate::bundled_models::bundled_model(id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_document_owned_bytes_and_optionally_embeds_models() {
        let model =
            crate::bundled_models::bundled_model("bundled-model:thqwgd001/THQWGD001 #1.stp")
                .unwrap();
        let local = b"local asset".to_vec();
        let hash = sha256_hex(&local);
        let mut document = ProjectDoc::empty("doc", "Archive");
        document.assets.push(Asset {
            id: "local".into(),
            name: "local.bin".into(),
            sha256: hash.clone(),
            media_type: "application/octet-stream".into(),
            source: None,
            license: None,
        });
        for embed in [false, true] {
            let prepared = prepare_archive(
                &document,
                &BTreeMap::from([(hash.clone(), local.clone())]),
                vec![(model, vec![])],
                embed,
            )
            .unwrap();
            let (reply, packed) =
                boardstudio_core::archive::request(&prepared.metadata, &prepared.buffers);
            assert_eq!(
                serde_json::from_str::<ArchiveReply>(&reply).unwrap(),
                ArchiveReply::Packed
            );
            let (reply, buffers) =
                boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &packed);
            let ArchiveReply::Unpacked {
                project_json,
                assets,
            } = serde_json::from_str(&reply).unwrap()
            else {
                panic!("packed archive should unpack");
            };
            let restored: ProjectDoc = serde_json::from_str(&project_json).unwrap();
            assert_eq!(restored.assets.len(), 1 + usize::from(embed));
            assert_eq!(restored.assets[0], document.assets[0]);
            let row = assets.iter().find(|row| row.sha256 == hash).unwrap();
            assert_eq!(buffers[row.buffer_index as usize], local);
            if embed {
                assert_eq!(restored.assets[1].id, model.id);
            }
        }
    }
}
