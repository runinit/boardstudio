//! The page's in-process Core executor. Core requests, archives and artifacts run
//! synchronously on the page thread through entry points the composition root injects, so
//! this crate never depends on the engine and no request crosses a `postMessage` boundary.
use boardstudio_model::model::{ArtifactReply, ArtifactRequest, CoreReply, CoreRequest};
use std::{
    cell::{Cell, RefCell},
    fmt,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostError(pub String);

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HostError {}

/// One executor's own Core engine. It never holds Session's document: requests that reach it
/// carry their own inputs.
pub type CoreHandler = Box<dyn FnMut(CoreRequest) -> CoreReply>;

/// Core's archive entry point: JSON metadata and input buffers in, reply metadata and output
/// buffers out.
pub type ArchiveHandler = fn(&str, &[Vec<u8>]) -> (String, Vec<Vec<u8>>);

/// Core's stateless artifact entry point over its JSON wire frames.
pub type ArtifactHandler = fn(&str) -> String;

/// Runs each request to completion before returning. Restarting the executor closes it and
/// builds a new one with a fresh engine, so a closed executor refuses all further work.
pub struct InProcessCoreExecutor {
    core: RefCell<CoreHandler>,
    archive: ArchiveHandler,
    artifact: ArtifactHandler,
    closed: Cell<bool>,
}

impl InProcessCoreExecutor {
    pub fn new(core: CoreHandler, archive: ArchiveHandler, artifact: ArtifactHandler) -> Self {
        Self {
            core: RefCell::new(core),
            archive,
            artifact,
            closed: Cell::new(false),
        }
    }

    pub fn check_ready(&self) -> Result<(), HostError> {
        if self.closed.get() {
            return Err(HostError("core executor is closed".into()));
        }
        Ok(())
    }

    pub fn close(&self) {
        self.closed.set(true);
    }

    pub fn run_request(
        &self,
        request_id: &str,
        request: &CoreRequest,
    ) -> Result<CoreReply, HostError> {
        self.admit(request_id)?;
        if core_request_id(request) != request_id {
            return Err(HostError("core request id mismatch".into()));
        }
        let reply = (self.core.borrow_mut())(request.clone());
        if core_reply_id(&reply) != request_id {
            return Err(HostError("core reply id mismatch".into()));
        }
        Ok(reply)
    }

    pub fn run_archive(
        &self,
        request_id: &str,
        metadata: &str,
        inputs: &[Vec<u8>],
    ) -> Result<(String, Vec<Vec<u8>>), HostError> {
        self.admit(request_id)?;
        Ok((self.archive)(metadata, inputs))
    }

    pub fn run_artifact(
        &self,
        request_id: &str,
        request: &ArtifactRequest,
    ) -> Result<ArtifactReply, HostError> {
        self.admit(request_id)?;
        if artifact_request_id(request) != request_id {
            return Err(HostError("artifact request id mismatch".into()));
        }
        let frame = serde_json::to_string(request)
            .map_err(|error| HostError(format!("could not encode artifact request: {error}")))?;
        let reply: ArtifactReply = serde_json::from_str(&(self.artifact)(&frame))
            .map_err(|error| HostError(format!("invalid artifact reply: {error}")))?;
        if artifact_reply_id(&reply) != request_id {
            return Err(HostError("artifact reply id mismatch".into()));
        }
        Ok(reply)
    }

    fn admit(&self, request_id: &str) -> Result<(), HostError> {
        if request_id.is_empty() {
            return Err(HostError(
                "core executor request id must not be empty".into(),
            ));
        }
        self.check_ready()
    }
}

fn core_request_id(request: &CoreRequest) -> &str {
    match request {
        CoreRequest::ResolveModules { id, .. }
        | CoreRequest::ResolveKeycaps { id, .. }
        | CoreRequest::GenerateFirmware { id, .. }
        | CoreRequest::ResolveElectrical { id, .. }
        | CoreRequest::ApplyElectrical { id, .. }
        | CoreRequest::ReviewElectricalRemap { id, .. }
        | CoreRequest::ProtectElectricalHandoff { id, .. }
        | CoreRequest::MechanicalProfile { id, .. }
        | CoreRequest::ResolveMechanical { id, .. }
        | CoreRequest::Open { id, .. }
        | CoreRequest::Edit { id, .. }
        | CoreRequest::Undo { id }
        | CoreRequest::Redo { id }
        | CoreRequest::Snapshot { id }
        | CoreRequest::ProjectMatrices { id, .. }
        | CoreRequest::PrepareCase { id, .. } => id,
    }
}

fn core_reply_id(reply: &CoreReply) -> &str {
    match reply {
        CoreReply::ModulesResolved { id, .. }
        | CoreReply::KeycapsResolved { id, .. }
        | CoreReply::FirmwareGenerated { id, .. }
        | CoreReply::ElectricalResolved { id, .. }
        | CoreReply::ElectricalApplied { id, .. }
        | CoreReply::ElectricalHandoffProtected { id, .. }
        | CoreReply::MechanicalProfile { id, .. }
        | CoreReply::MechanicalResolved { id, .. }
        | CoreReply::Preview { id, .. }
        | CoreReply::Scene { id, .. }
        | CoreReply::MatrixProjections { id, .. }
        | CoreReply::Error { id, .. }
        | CoreReply::CasePrepared { id, .. } => id,
    }
}

fn artifact_request_id(request: &ArtifactRequest) -> &str {
    match request {
        ArtifactRequest::ReadBoardSource { id, .. }
        | ArtifactRequest::ImportModuleBoard { id, .. }
        | ArtifactRequest::ExportMechanicalPlate { id, .. }
        | ArtifactRequest::ExtractMechanical { id, .. }
        | ArtifactRequest::PreviewBoard { id, .. }
        | ArtifactRequest::CompileFootprints { id, .. }
        | ArtifactRequest::ImportFootprint { id, .. }
        | ArtifactRequest::PreviewPcb { id, .. }
        | ArtifactRequest::ExportPcb { id, .. }
        | ArtifactRequest::ExportOutline { id, .. } => id,
    }
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

#[cfg(test)]
mod tests {
    use super::{HostError, InProcessCoreExecutor};
    use boardstudio_core::CoreEngine;
    use boardstudio_model::model::{ArtifactReply, ArtifactRequest, CoreReply, CoreRequest};

    fn executor() -> InProcessCoreExecutor {
        let mut engine = CoreEngine::new();
        InProcessCoreExecutor::new(
            Box::new(move |request| engine.handle(request)),
            boardstudio_core::archive::request,
            boardstudio_core::artifact_request,
        )
    }

    fn snapshot(id: &str) -> CoreRequest {
        CoreRequest::Snapshot { id: id.into() }
    }

    #[test]
    fn core_request_reaches_the_engine_and_keeps_its_id() {
        let reply = executor()
            .run_request("m1-1", &snapshot("m1-1"))
            .expect("Core replies in process");
        assert_eq!(super::core_reply_id(&reply), "m1-1");
    }

    #[test]
    fn requests_share_the_executor_engine() {
        let mut replies = Vec::new();
        let executor = InProcessCoreExecutor::new(
            Box::new(move |request| {
                let id = super::core_request_id(&request).to_owned();
                replies.push(id.clone());
                CoreReply::Error {
                    id,
                    message: String::new(),
                    revision: replies.len() as u64,
                }
            }),
            boardstudio_core::archive::request,
            boardstudio_core::artifact_request,
        );
        executor.run_request("a", &snapshot("a")).unwrap();
        let reply = executor.run_request("b", &snapshot("b")).unwrap();
        assert!(matches!(reply, CoreReply::Error { revision: 2, .. }));
    }

    #[test]
    fn request_id_must_be_present_and_match_the_frame() {
        let executor = executor();
        assert_eq!(
            executor.run_request("", &snapshot("")),
            Err(HostError(
                "core executor request id must not be empty".into()
            ))
        );
        assert_eq!(
            executor.run_request("m1-1", &snapshot("m1-2")),
            Err(HostError("core request id mismatch".into()))
        );
    }

    #[test]
    fn reply_id_must_match_the_request() {
        let executor = InProcessCoreExecutor::new(
            Box::new(|_| CoreReply::Error {
                id: "other".into(),
                message: String::new(),
                revision: 0,
            }),
            boardstudio_core::archive::request,
            boardstudio_core::artifact_request,
        );
        assert_eq!(
            executor.run_request("m1-1", &snapshot("m1-1")),
            Err(HostError("core reply id mismatch".into()))
        );
    }

    #[test]
    fn artifact_request_round_trips_through_core() {
        let executor = executor();
        let reply = executor
            .run_artifact(
                "artifact-1",
                &ArtifactRequest::CompileFootprints {
                    id: "artifact-1".into(),
                    jobs: vec![],
                },
            )
            .expect("artifact replies in process");
        assert!(matches!(
            reply,
            ArtifactReply::CompileFootprints { ref id, .. } if id == "artifact-1"
        ));
        assert_eq!(
            executor.run_artifact(
                "artifact-2",
                &ArtifactRequest::CompileFootprints {
                    id: "artifact-3".into(),
                    jobs: vec![],
                },
            ),
            Err(HostError("artifact request id mismatch".into()))
        );
    }

    #[test]
    fn artifact_reply_id_must_match_the_request() {
        let executor = InProcessCoreExecutor::new(
            Box::new(|_| unreachable!("artifacts never reach the document engine")),
            boardstudio_core::archive::request,
            |_| r#"{"kind":"compile-footprints","id":"other","result":[]}"#.into(),
        );
        assert_eq!(
            executor.run_artifact(
                "artifact-1",
                &ArtifactRequest::CompileFootprints {
                    id: "artifact-1".into(),
                    jobs: vec![],
                },
            ),
            Err(HostError("artifact reply id mismatch".into()))
        );
    }

    #[test]
    fn archive_passes_metadata_and_buffers_to_core() {
        let executor = InProcessCoreExecutor::new(
            Box::new(|_| unreachable!("archives never reach the document engine")),
            |metadata, inputs| (format!("{metadata}:{}", inputs.len()), inputs.to_vec()),
            boardstudio_core::artifact_request,
        );
        assert_eq!(
            executor.run_archive("archive-1", "meta", &[vec![1, 2], vec![3]]),
            Ok(("meta:2".into(), vec![vec![1, 2], vec![3]]))
        );
        assert!(executor.run_archive("", "meta", &[]).is_err());
    }

    #[test]
    fn closed_executor_refuses_all_work() {
        let executor = executor();
        assert_eq!(executor.check_ready(), Ok(()));
        executor.close();
        let closed = HostError("core executor is closed".into());
        assert_eq!(executor.check_ready(), Err(closed.clone()));
        assert_eq!(
            executor.run_request("m1-1", &snapshot("m1-1")),
            Err(closed.clone())
        );
        assert_eq!(
            executor.run_archive("archive-1", "{}", &[]),
            Err(closed.clone())
        );
        assert_eq!(
            executor.run_artifact(
                "artifact-1",
                &ArtifactRequest::CompileFootprints {
                    id: "artifact-1".into(),
                    jobs: vec![],
                },
            ),
            Err(closed)
        );
    }
}
