# 02: Preserve monotonic mechanical feature identities across removal, undo, and overflow

Status: ready-for-agent
Type: build
Blocked by: [01: Reject duplicate document identities and overflowing revisions on open and commit](01-reject-duplicate-identities-and-revision-overflow.md)

**What to build:** Mechanical feature additions (`openings`, `vertices`, `criticalFits`, `hardware`, and `mounts`) mint monotonically increasing identities that are never re-issued after removal or after undoing an addition within the session, so a queued non-strict field edit targeting a removed or undone item is rejected rather than mutating a newly created replacement item. Imported documents containing `u64::MAX` numeric identity suffixes return an error when minting a new identity instead of panicking on overflow.

## Acceptance criteria

- [ ] Removing a critical fit, hardware entry, or mechanical mount and adding a new one assigns a fresh monotonic identity rather than reusing the removed `*-1` identity, so a stale edit targeting the removed ID is rejected.
- [ ] Adding an opening (or other mechanical feature), undoing that addition, and adding another one assigns a fresh identity rather than re-issuing the undone identity, so a stale edit targeting the undone ID is rejected.
- [ ] Adding an opening, vertex, critical fit, hardware entry, or mount when an imported document already carries a `u64::MAX` numeric suffix returns a clean error instead of panicking with arithmetic overflow.
- [ ] Native tests verify non-reuse across remove+add, non-reuse across add+undo+add, and clean refusal on `u64::MAX` suffix exhaustion.
