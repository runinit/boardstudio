//! Shared accepted-owner/snapshot guard for preview and viewer freshness checks.
//!
//! Several presenters scope an async preview or viewer signal to the accepted
//! snapshot and scope that produced it; those guards share exactly the
//! scope-plus-snapshot-token comparison below. Guards that also cover
//! generations, leases, revisions or workflow state stay with their owners.
use boardstudio_application::{Scope, SnapshotToken};

/// Whether `scope`/`snapshot_token` still name the accepted `owner` source.
/// Lease liveness, generations and revisions are separate identity domains and
/// are checked by the callers that own them.
pub fn owner_is_current(
    owner_scope: &Scope,
    owner_token: SnapshotToken,
    scope: &Scope,
    snapshot_token: SnapshotToken,
) -> bool {
    owner_scope == scope && owner_token == snapshot_token
}
