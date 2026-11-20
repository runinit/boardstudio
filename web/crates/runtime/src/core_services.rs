//! The synchronous Core entry points the hosting crate injects into Runtime.
//!
//! The page owns the document engine; everything below it reaches Core through these function
//! pointers (or the asynchronous `CoreExecutor`), so the web crates compile without waiting
//! for the engine. This is the same injection the engine uses for its `ScriptRunner` and
//! Session uses for its `OpenValidator`.
use boardstudio_application::OpenValidator;
use boardstudio_model::model::{CoreReply, CoreRequest};

/// A throwaway Core engine: requests run in order against its own document and never reach
/// the Session or the worker executor.
pub type ScratchCore = Box<dyn FnMut(CoreRequest) -> CoreReply>;

/// Opens a fresh [`ScratchCore`] for a read-only probe (a reviewed link's two-half result or
/// an anchor pick's post-commit poses).
pub type OpenScratchCore = fn() -> ScratchCore;

#[derive(Clone, Copy)]
pub struct CoreServices {
    /// Core's Open admission check, run before editor work is retired.
    pub validate_open: OpenValidator,
    pub open_scratch: OpenScratchCore,
}

#[cfg(any(test, feature = "test-support"))]
impl CoreServices {
    /// The real engine, for tests (the page builds the same services from its own engine edge).
    pub fn engine() -> Self {
        Self {
            validate_open: boardstudio_core::CoreEngine::validate_open,
            open_scratch: || {
                let mut engine = boardstudio_core::CoreEngine::new();
                Box::new(move |request| engine.handle(request))
            },
        }
    }
}
