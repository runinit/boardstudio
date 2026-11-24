//! Concrete JSON codecs for the worker wire protocols.
//!
//! The request and reply enums share most of the document graph. Keeping their
//! `serde_json` entry points in this crate gives the final link one set of
//! deserializer and serializer monomorphizations to retain, instead of one set
//! per crate that happens to speak a protocol.

use crate::model::{
    ArchiveReply, ArchiveRequest, ArtifactError, ArtifactErrorCode, ArtifactReply, ArtifactRequest,
};
#[cfg(feature = "wire-json")]
use crate::model::{CoreReply, CoreRequest};

pub fn decode_archive_request(json: &str) -> Result<ArchiveRequest, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn encode_archive_reply(reply: &ArchiveReply) -> String {
    serde_json::to_string(reply).unwrap_or_else(|err| {
        serde_json::to_string(&ArchiveReply::Error {
            message: format!("Failed to serialize archive reply: {err}"),
        })
        .unwrap_or_else(|_| {
            r#"{"kind":"error","message":"Failed to serialize archive reply"}"#.to_owned()
        })
    })
}

#[cfg(feature = "wire-json")]
pub fn decode_core_request(json: &str) -> Result<CoreRequest, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(feature = "wire-json")]
fn core_reply_identity(reply: &CoreReply) -> (&str, u64) {
    match reply {
        CoreReply::ModulesResolved { id, .. }
        | CoreReply::KeycapsResolved { id, .. }
        | CoreReply::FirmwareGenerated { id, .. }
        | CoreReply::ElectricalResolved { id, .. }
        | CoreReply::MechanicalProfile { id, .. }
        | CoreReply::MechanicalResolved { id, .. }
        | CoreReply::CasePrepared { id, .. } => (id, 0),
        CoreReply::Preview { id, scene } | CoreReply::Scene { id, scene, .. } => {
            (id, scene.revision)
        }
        CoreReply::ElectricalApplied { id, revision, .. }
        | CoreReply::ElectricalHandoffProtected { id, revision, .. }
        | CoreReply::MatrixProjections { id, revision, .. }
        | CoreReply::Error { id, revision, .. } => (id, *revision),
    }
}

#[cfg(feature = "wire-json")]
pub fn encode_core_reply(reply: &CoreReply) -> String {
    serde_json::to_string(reply).unwrap_or_else(|err| {
        let (id, revision) = core_reply_identity(reply);
        serde_json::to_string(&CoreReply::Error {
            id: id.to_owned(),
            message: format!("Failed to serialize core reply: {err}"),
            revision,
        })
        .unwrap_or_else(|_| {
            r#"{"kind":"error","id":"","message":"Failed to serialize core reply","revision":0}"#
                .to_owned()
        })
    })
}

pub fn decode_artifact_request(json: &str) -> Result<ArtifactRequest, serde_json::Error> {
    serde_json::from_str(json)
}

fn artifact_reply_id(reply: &ArtifactReply) -> &str {
    match reply {
        ArtifactReply::ReadBoardSource { id, .. }
        | ArtifactReply::ImportModuleBoard { id, .. }
        | ArtifactReply::ExportMechanicalPlate { id, .. }
        | ArtifactReply::ExtractMechanical { id, .. }
        | ArtifactReply::PreviewBoard { id, .. }
        | ArtifactReply::CompileFootprints { id, .. }
        | ArtifactReply::ImportFootprint { id, .. }
        | ArtifactReply::ExportPcb { id, .. }
        | ArtifactReply::ExportOutline { id, .. }
        | ArtifactReply::Error { id, .. } => id,
    }
}

pub fn encode_artifact_reply(reply: &ArtifactReply) -> String {
    serde_json::to_string(reply).unwrap_or_else(|err| {
        serde_json::to_string(&ArtifactReply::Error {
            id: artifact_reply_id(reply).to_owned(),
            error: ArtifactError::new(
                ArtifactErrorCode::Validation,
                format!("Failed to serialize artifact reply: {err}"),
            ),
        })
        .unwrap_or_else(|_| {
            r#"{"kind":"error","id":"","error":{"code":"validation","message":"Failed to serialize artifact reply","diagnostics":[]}}"#.to_owned()
        })
    })
}
