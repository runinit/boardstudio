//! Consumer-owned headless application session and interaction policy.
//!
//! Edit resolvers run synchronously against the accepted snapshot.
pub mod interactions;
pub mod session;
pub use session::*;

/// Returns `document` with mirror-link derived halves materialized for read-only consumers.
pub fn resolved_document(
    document: &boardstudio_core::model::ProjectDoc,
) -> std::borrow::Cow<'_, boardstudio_core::model::ProjectDoc> {
    if document.mirrored_pairs.is_empty() {
        std::borrow::Cow::Borrowed(document)
    } else {
        std::borrow::Cow::Owned(
            boardstudio_core::project_document(document)
                .geometry()
                .clone(),
        )
    }
}
