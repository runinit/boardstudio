//! The synchronous Core entry points the hosting crate injects into Runtime.
//!
//! The page injects typed handlers so the web crates can compile independently of the engine.
use boardstudio_application::OpenValidator;
use boardstudio_model::model::{CoreReply, CoreRequest};

/// The page-owned engine for Session's document and history.
pub type DocumentCore = Box<dyn FnMut(CoreRequest) -> CoreReply>;

/// A throwaway Core engine: requests run in order against its own document and never reach
/// the Session.
pub type ScratchCore = DocumentCore;

/// Creates a fresh engine for the document or a read-only scratch probe (a reviewed link's
/// two-half result or an anchor pick's post-commit poses).
pub type OpenScratchCore = fn() -> ScratchCore;

#[derive(Clone, Copy)]
pub struct CoreServices {
    /// Core's Open admission check, run before editor work is retired.
    pub validate_open: OpenValidator,
    pub open_scratch: OpenScratchCore,
}

#[cfg(test)]
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
