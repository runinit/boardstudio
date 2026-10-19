# 01: Reject duplicate document identities and overflowing revisions on open and commit

Status: ready-for-agent
Type: build
Blocked by: None (can start immediately)

**What to build:** Opening or replacing a project document validates that top-level entity identities (`parts`, `definitions`, `boards`, `matrices`, `nets`, `constraints`) are unique rather than allowing duplicate part IDs to silently overwrite earlier poses during constraint resolution and turn subsequent `MoveParts` edits into silent no-ops. Opening a document at `u64::MAX` revision or advancing a document revision at `u64::MAX` across any commit, undo/redo, or electrical remap review returns a clean error instead of panicking on integer overflow.

## Acceptance criteria

- [ ] Opening or replacing a document with duplicate `parts`, `definitions`, `boards`, `matrices`, `nets`, or `constraints` identities is rejected with a clear error message, leaving any previously accepted document intact.
- [ ] A document with unique part IDs continues to open and move parts normally.
- [ ] Opening a document with `revision == u64::MAX` or advancing a revision when at `u64::MAX` (on edit commit, undo/redo, and electrical remap review) returns a clean `CoreReply::Error` instead of panicking on `+ 1`.
- [ ] Native unit/integration tests cover duplicate part ID rejection on open/replace and `u64::MAX` revision handling across open, commit, undo/redo, and remap review.
