use std::collections::BTreeMap;

use boardstudio_core::{archive, model::ArchiveReply};
use serde::Serialize;

#[derive(Debug, PartialEq)]
pub enum ArchiveError {
    InvalidReply(String),
    Core(String),
}

#[derive(Debug, PartialEq)]
pub struct UnpackedProject {
    pub project_json: String,
    pub assets: BTreeMap<String, Vec<u8>>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ProbeRequest<'a> {
    UnpackProject,
    PackProject {
        #[serde(rename = "projectJson")]
        project_json: &'a str,
        assets: Vec<AssetEntry>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetEntry {
    path: String,
    buffer_index: u32,
}

pub fn unpack_project(bytes: &[u8]) -> Result<UnpackedProject, ArchiveError> {
    let metadata = serde_json::to_string(&ProbeRequest::UnpackProject).expect("request serializes");
    let (reply_json, buffers) = archive::request(&metadata, &[bytes.to_vec()]);
    let reply: ArchiveReply = serde_json::from_str(&reply_json)
        .map_err(|error| ArchiveError::InvalidReply(error.to_string()))?;
    match reply {
        ArchiveReply::Unpacked {
            project_json,
            assets,
        } => {
            let mut unpacked = BTreeMap::new();
            for asset in assets {
                let bytes = buffers.get(asset.buffer_index as usize).ok_or_else(|| {
                    ArchiveError::InvalidReply("Missing archive asset buffer".into())
                })?;
                unpacked.insert(asset.sha256, bytes.clone());
            }
            Ok(UnpackedProject {
                project_json,
                assets: unpacked,
            })
        }
        ArchiveReply::Error { message } => Err(ArchiveError::Core(message)),
        other => Err(ArchiveError::InvalidReply(format!(
            "Expected unpacked archive, got {other:?}"
        ))),
    }
}

pub fn pack_project(project: &UnpackedProject) -> Result<Vec<u8>, ArchiveError> {
    let entries: Vec<_> = project
        .assets
        .keys()
        .enumerate()
        .map(|(index, hash)| AssetEntry {
            path: format!("assets/{hash}"),
            buffer_index: index as u32,
        })
        .collect();
    let metadata = serde_json::to_string(&ProbeRequest::PackProject {
        project_json: &project.project_json,
        assets: entries,
    })
    .expect("request serializes");
    let buffers: Vec<_> = project.assets.values().cloned().collect();
    let (reply_json, outputs) = archive::request(&metadata, &buffers);
    let reply: ArchiveReply = serde_json::from_str(&reply_json)
        .map_err(|error| ArchiveError::InvalidReply(error.to_string()))?;
    match reply {
        ArchiveReply::Packed => outputs
            .into_iter()
            .next()
            .ok_or_else(|| ArchiveError::InvalidReply("Packed archive has no bytes".into())),
        ArchiveReply::Error { message } => Err(ArchiveError::Core(message)),
        other => Err(ArchiveError::InvalidReply(format!(
            "Expected packed archive, got {other:?}"
        ))),
    }
}
