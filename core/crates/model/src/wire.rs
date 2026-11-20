//! Concrete JSON codecs for the worker wire protocols.
//!
//! The request and reply enums share most of the document graph. Keeping their
//! `serde_json` entry points in this crate gives the final link one set of
//! deserializer and serializer monomorphizations to retain, instead of one set
//! per crate that happens to speak a protocol.

use crate::model::{
    ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, CoreReply, CoreRequest,
};

pub fn decode_archive_request(json: &str) -> Result<ArchiveRequest, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn encode_archive_reply(reply: &ArchiveReply) -> String {
    serde_json::to_string(reply).expect("archive reply serializes")
}

pub fn decode_core_request(json: &str) -> Result<CoreRequest, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn encode_core_reply(reply: &CoreReply) -> String {
    serde_json::to_string(reply).expect("reply serializes")
}

pub fn decode_artifact_request(json: &str) -> Result<ArtifactRequest, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn encode_artifact_reply(reply: &ArtifactReply) -> String {
    serde_json::to_string(reply).expect("artifact reply serializes")
}
