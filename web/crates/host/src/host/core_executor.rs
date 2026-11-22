use crate::in_process_core::{HostError, InProcessCoreExecutor};
use boardstudio_model::model::{ArtifactReply, ArtifactRequest, CoreReply, CoreRequest};
use js_sys::Uint8Array;
use std::{future::Future, future::ready, pin::Pin};

pub type CoreExecutorFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, HostError>> + 'a>>;

/// The Core execution port: every awaited Core operation the page runtime performs (Session
/// requests, resolvers, archives and artifacts). Production implements it in process
/// ([`InProcessCoreExecutor`]); tests install their own executor, so lifecycle tests can
/// replace or restart the executor without adding a second provider or changing Core's API.
pub trait CoreExecutor {
    fn request<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        request: &'a CoreRequest,
    ) -> CoreExecutorFuture<'a, CoreReply>;

    fn archive<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        metadata: &'a str,
        buffers: Vec<Uint8Array>,
    ) -> CoreExecutorFuture<'a, ArchiveResult>;

    fn artifact<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        request: &'a ArtifactRequest,
    ) -> CoreExecutorFuture<'a, ArtifactReply>;

    fn ready<'a>(&'a self) -> CoreExecutorFuture<'a, ()>;

    fn close(&self);
}

#[derive(Clone, Debug)]
pub struct ArchiveResult {
    pub metadata: String,
    pub buffers: Vec<Uint8Array>,
}

/// Every future is already complete when returned. The executor epoch is not checked here:
/// a reply can no longer arrive after its executor was replaced, so a restart only has to
/// close the old executor, and Runtime's own epoch guards around each await still decide
/// whether a reply is current.
impl CoreExecutor for InProcessCoreExecutor {
    fn request<'a>(
        &'a self,
        request_id: &'a str,
        _executor_epoch: &'a str,
        request: &'a CoreRequest,
    ) -> CoreExecutorFuture<'a, CoreReply> {
        Box::pin(ready(self.run_request(request_id, request)))
    }

    fn archive<'a>(
        &'a self,
        request_id: &'a str,
        _executor_epoch: &'a str,
        metadata: &'a str,
        buffers: Vec<Uint8Array>,
    ) -> CoreExecutorFuture<'a, ArchiveResult> {
        let inputs: Vec<Vec<u8>> = buffers.iter().map(Uint8Array::to_vec).collect();
        let result = self
            .run_archive(request_id, metadata, &inputs)
            .map(|(metadata, outputs)| ArchiveResult {
                metadata,
                buffers: outputs
                    .iter()
                    .map(|bytes| Uint8Array::from(bytes.as_slice()))
                    .collect(),
            });
        Box::pin(ready(result))
    }

    fn artifact<'a>(
        &'a self,
        request_id: &'a str,
        _executor_epoch: &'a str,
        request: &'a ArtifactRequest,
    ) -> CoreExecutorFuture<'a, ArtifactReply> {
        Box::pin(ready(self.run_artifact(request_id, request)))
    }

    fn ready<'a>(&'a self) -> CoreExecutorFuture<'a, ()> {
        Box::pin(ready(self.check_ready()))
    }

    fn close(&self) {
        InProcessCoreExecutor::close(self)
    }
}
