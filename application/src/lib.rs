//! Consumer-owned headless application session and interaction policy.
//!
//! Pending edits are submitted as intent with [`session::EditResolver`]: Session calls the
//! resolver exactly once, when the edit reaches the head of the queue with nothing else
//! running, and hands it the accepted snapshot the next command will apply to. A resolver
//! is a pure function of that snapshot and the values its owner captured at submit time —
//! it must not read UI state, signals or Runtime.
pub mod interactions;
pub mod session;
pub use session::*;
