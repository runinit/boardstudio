//! Consumer-owned headless application session and interaction policy.
//!
//! Edit resolvers run synchronously against the accepted snapshot.
pub mod interactions;
pub mod session;
pub use session::*;
