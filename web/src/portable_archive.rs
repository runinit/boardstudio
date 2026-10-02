use boardstudio_core::model::{Asset, ProjectDoc};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) struct PreparedArchive {
    pub(crate) metadata: String,
    pub(crate) buffers: Vec<Vec<u8>>,
}

pub(crate) fn prepare_archive(
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
            let sha256 = hex_digest(&bytes);
            packed_document.assets.push(Asset {
                id: model.id.to_owned(),
                name: model.filename.to_owned(),
                media_type: model.media_type.to_owned(),
                sha256: sha256.clone(),
                license: None,
                source: Some("bundled Ergogen library".into()),
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

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Resolves all directly persisted bundled references plus the generated
/// references collected from Ergogen's authoritative `modelAssetIds` export.
/// Generated IDs already owned by the document use its local assets; other
/// generated IDs are validated strictly. Saved metadata keeps
/// React's behavior of including only IDs present in its bundled catalogue.
pub(crate) fn referenced_models(
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
            return Err(format!("Bundled Ergogen model is unavailable: {id}"));
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
    use super::referenced_models;
    use boardstudio_core::model::HardwareSource;
    use boardstudio_core::model::{
        AssemblyDefinition, AssemblyMember, Asset, BoardReference, ModuleBoard, ModuleCircuit,
        ModuleDefinition, ModuleElectrical, ModuleProtocol, PartDefinition, PartKind, PartModel,
        Pose2, ProjectDoc, Side, Vec2, Vec3,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn model(asset_id: &str) -> PartModel {
        PartModel {
            asset_id: asset_id.into(),
            offset: Vec3::default(),
            rotation: Vec3::default(),
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }

    fn definition(id: &str, models: Vec<PartModel>) -> PartDefinition {
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: id.into(),
            name: id.into(),
            kind: PartKind::Custom,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: Some(models),
            generator: None,
            mechanical_profile: None,
        }
    }

    fn module_definition(models: Vec<PartModel>, circuit: ModuleCircuit) -> ModuleDefinition {
        ModuleDefinition {
            id: "module".into(),
            catalogue_row: None,
            name: "module".into(),
            family: "test".into(),
            variant: "test".into(),
            source: HardwareSource {
                repository: "test".into(),
                revision: "test".into(),
                path: "test".into(),
                license: "test".into(),
                sha256: None,
                upstream_status: None,
            },
            board: ModuleBoard {
                contours: vec![],
                holes: vec![],
                thickness: None,
            },
            mounts: vec![],
            volumes: vec![],
            openings: vec![],
            models,
            candidate_models: vec![],
            gates: vec![],
            interfaces: vec![],
            electrical: ModuleElectrical {
                protocol: ModuleProtocol::Nonstandard,
                required_signals: vec![],
                logic_voltage: None,
                current_ma: None,
                i2c_address: None,
                pullup_ohms: None,
                driver: None,
                rotary_profile: None,
            },
            constituents: vec![],
            circuit: Some(circuit),
        }
    }

    #[test]
    fn selected_closure_matches_react_reference_buckets_and_deduplicates() {
        let included = "ergogen:model:foostan/OLED-Module-with-Pins.step";
        let other = "ergogen:model:infused-kim/Nice_Nano_V2.step";
        let unused = "ergogen:model:thqwgd001/THQWGD001 #1.stp";
        let circuit_definition = definition("circuit-definition", vec![model(other)]);
        let circuit = ModuleCircuit {
            definitions: vec![circuit_definition],
            parts: vec![],
            nets: vec![],
            ports: BTreeMap::new(),
            adaptations: vec![],
        };
        let mut document = ProjectDoc::empty("doc", "portable test");
        document
            .definitions
            .push(definition("unplaced", vec![model(included)]));
        document
            .module_definitions
            .push(module_definition(vec![model(other)], circuit));
        document.assemblies.push(AssemblyDefinition {
            id: "assembly".into(),
            name: "assembly".into(),
            members: vec![AssemblyMember {
                parameters: None,
                model_mode: None,
                id: "member".into(),
                definition_id: None,
                pose: Pose2 {
                    at: Vec2::default(),
                    rotation: 0.0,
                },
                side: Side::Front,
                models: vec![model(included)],
            }],
        });
        document.board_references.push(BoardReference {
            id: "board-ref".into(),
            board_id: "board".into(),
            asset_id: "pcb".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: BTreeMap::from([("case".into(), other.into())]),
        });
        document.assets.push(Asset {
            id: included.into(),
            name: "already owned".into(),
            media_type: "model/step".into(),
            sha256: "00".repeat(32),
            license: None,
            source: None,
        });

        let selected = referenced_models(&document, [included.to_owned()]).unwrap();
        let ids = selected.iter().map(|model| model.id).collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "ergogen:model:foostan/OLED-Module-with-Pins.step",
                "ergogen:model:infused-kim/Nice_Nano_V2.step",
            ]
        );
        assert!(!ids.contains(&unused));
    }

    #[test]
    fn generated_document_owned_model_does_not_require_a_bundled_provider() {
        let mut document = ProjectDoc::empty("doc", "local generated model");
        document.assets.push(Asset {
            id: "local-switch".into(),
            name: "local.step".into(),
            media_type: "model/step".into(),
            sha256: "00".repeat(32),
            license: None,
            source: None,
        });
        let models = referenced_models(&document, ["local-switch".to_owned()])
            .expect("document-owned generated model uses local asset bytes");
        assert!(models.is_empty());
    }

    #[test]
    fn generated_unknown_ergogen_model_fails_instead_of_silently_omitting() {
        let error = referenced_models(
            &ProjectDoc::empty("doc", "unknown reference"),
            ["ergogen:model:missing/model.step".to_owned()],
        )
        .unwrap_err();
        assert!(error.contains("ergogen:model:missing/model.step"));
    }

    #[test]
    fn a_disabled_embedding_option_can_keep_local_assets_without_resolving_models() {
        let document = ProjectDoc::empty("doc", "local assets");
        let serialized = serde_json::to_value(&document).unwrap();
        assert_eq!(serialized["assets"], json!([]));
        // The public operation skips `referenced_models` entirely when false;
        // this input documents the boundary before the F8 preference is wired.
        assert!(
            referenced_models(&document, Vec::<String>::new())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn pack_round_trip_keeps_local_bytes_and_optionally_adds_opaque_model_bytes() {
        use boardstudio_core::model::{ArchiveReply, Asset};
        use sha2::{Digest, Sha256};

        let local_bytes = b"existing local asset".to_vec();
        let local_hash = hex_digest(&local_bytes);
        let model =
            super::super::bundled_models::bundled_model("ergogen:model:thqwgd001/THQWGD001 #1.stp")
                .unwrap();
        let opaque_bytes = Vec::new();
        let mut document = ProjectDoc::empty("doc", "archive copy");
        document.assets.push(Asset {
            id: "local-asset".into(),
            name: "local.bin".into(),
            media_type: "application/octet-stream".into(),
            sha256: local_hash.clone(),
            license: None,
            source: None,
        });
        let original = document.clone();
        let local_assets = BTreeMap::from([(local_hash.clone(), local_bytes.clone())]);

        let enabled = super::prepare_archive(
            &document,
            &local_assets,
            vec![(model, opaque_bytes.clone())],
            true,
        )
        .unwrap();
        let (reply, archive_buffers) =
            boardstudio_core::archive::request(&enabled.metadata, &enabled.buffers);
        assert!(matches!(
            serde_json::from_str::<ArchiveReply>(&reply).unwrap(),
            ArchiveReply::Packed
        ));
        let (reply, unpacked_buffers) =
            boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &archive_buffers);
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = serde_json::from_str::<ArchiveReply>(&reply).unwrap()
        else {
            panic!("packed archive should unpack")
        };
        let restored: ProjectDoc = serde_json::from_str(&project_json).unwrap();
        assert_eq!(restored.assets.len(), 2);
        assert_eq!(restored.assets[0], original.assets[0]);
        assert_eq!(restored.assets[1].id, model.id);
        assert_eq!(restored.assets[1].name, model.filename);
        assert_eq!(
            restored.assets[1].source.as_deref(),
            Some("bundled Ergogen library")
        );
        assert_eq!(restored.assets[1].sha256, hex_digest(&opaque_bytes));
        let local = assets
            .iter()
            .find(|asset| asset.sha256 == local_hash)
            .unwrap();
        assert_eq!(unpacked_buffers[local.buffer_index as usize], local_bytes);
        let model_row = assets
            .iter()
            .find(|asset| asset.sha256 == hex_digest(&opaque_bytes))
            .unwrap();
        assert!(unpacked_buffers[model_row.buffer_index as usize].is_empty());
        assert!(archive_option(&enabled.metadata));
        assert_eq!(document, original);

        let disabled =
            super::prepare_archive(&document, &local_assets, vec![(model, opaque_bytes)], false)
                .unwrap();
        let (reply, archive_buffers) =
            boardstudio_core::archive::request(&disabled.metadata, &disabled.buffers);
        assert!(matches!(
            serde_json::from_str::<ArchiveReply>(&reply).unwrap(),
            ArchiveReply::Packed
        ));
        let (reply, _) =
            boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &archive_buffers);
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = serde_json::from_str::<ArchiveReply>(&reply).unwrap()
        else {
            panic!("local-only archive should unpack")
        };
        let restored: ProjectDoc = serde_json::from_str(&project_json).unwrap();
        assert_eq!(restored.assets, original.assets);
        assert_eq!(assets.len(), 1);
        assert!(!archive_option(&disabled.metadata));

        let existing_model_bytes = b"already embedded by the document".to_vec();
        let existing_model_hash = hex_digest(&existing_model_bytes);
        let mut document_with_model = ProjectDoc::empty("doc", "existing model asset");
        document_with_model.assets.push(Asset {
            id: model.id.into(),
            name: model.filename.into(),
            media_type: model.media_type.into(),
            sha256: existing_model_hash.clone(),
            license: None,
            source: Some("document-owned".into()),
        });
        let preserved = super::prepare_archive(
            &document_with_model,
            &BTreeMap::from([(existing_model_hash, existing_model_bytes.clone())]),
            vec![(model, b"must not replace the document-owned copy".to_vec())],
            true,
        )
        .unwrap();
        let (reply, packed_buffers) =
            boardstudio_core::archive::request(&preserved.metadata, &preserved.buffers);
        assert!(matches!(
            serde_json::from_str::<ArchiveReply>(&reply).unwrap(),
            ArchiveReply::Packed
        ));
        let (reply, unpacked_buffers) =
            boardstudio_core::archive::request(r#"{"kind":"unpack-project"}"#, &packed_buffers);
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = serde_json::from_str::<ArchiveReply>(&reply).unwrap()
        else {
            panic!("existing document asset should unpack")
        };
        let restored: ProjectDoc = serde_json::from_str(&project_json).unwrap();
        assert_eq!(restored.assets, document_with_model.assets);
        assert_eq!(assets.len(), 1, "an existing identity is never duplicated");
        assert_eq!(
            unpacked_buffers[assets[0].buffer_index as usize],
            existing_model_bytes
        );

        fn hex_digest(bytes: &[u8]) -> String {
            Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        }

        fn archive_option(metadata: &str) -> bool {
            let value: serde_json::Value = serde_json::from_str(metadata).unwrap();
            let archive_json: serde_json::Value =
                serde_json::from_str(value["archiveJson"].as_str().unwrap()).unwrap();
            archive_json["embedUsedModels"].as_bool().unwrap()
        }
    }
}
