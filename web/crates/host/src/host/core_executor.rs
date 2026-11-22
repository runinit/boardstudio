use crate::in_process_core::{HostError, InProcessCoreExecutor};
use boardstudio_model::model::{ArtifactReply, ArtifactRequest, CoreReply, CoreRequest};
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

    fn artifact<'a>(
        &'a self,
        request_id: &'a str,
        executor_epoch: &'a str,
        request: &'a ArtifactRequest,
    ) -> CoreExecutorFuture<'a, ArtifactReply>;

    fn ready<'a>(&'a self) -> CoreExecutorFuture<'a, ()>;

    fn close(&self);
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
