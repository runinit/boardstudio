# 01: Reject duplicate document identities and overflowing revisions on open and commit

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Opening or replacing a project document validates that top-level entity identities (`parts`, `definitions`, `boards`, `matrices`, `nets`, `constraints`) are unique rather than allowing duplicate part IDs to silently overwrite earlier poses during constraint resolution and turn subsequent `MoveParts` edits into silent no-ops. Opening a document at `u64::MAX` revision or advancing a document revision at `u64::MAX` across any commit, undo/redo, or electrical remap review returns a clean error instead of panicking on integer overflow.

## Research decision (2026-10-19)

Keep: duplicate parts collapse during identity-keyed constraint resolution, and revision increments are unchecked. Constraint IDs already have a solver guard; preserve it rather than inventing a second policy. Validate incoming identities before normalization can hide duplicates, and preserve uniqueness when publishing generated output. Uniqueness is within each named collection, not across entity kinds. See [research 01](../../../research/rust-best-practices-ticket-validation.md#01--duplicate-identities-and-revision-overflow).

Compute a checked next revision before mutating history or clearing a protected handoff. Rejecting max-revision Open alone is insufficient: a document accepted at `MAX-1` can reach `MAX`. Do not change successful previews, no-ops, or existing earlier validation errors when no revision advances.

## Acceptance criteria

- [ ] Opening or replacing a document with duplicate `parts`, `definitions`, `boards`, `matrices`, `nets`, or `constraints` identities is rejected with a clear error message, leaving any previously accepted document intact.
- [ ] A document with unique part IDs continues to open and move parts normally.
- [ ] Opening a document with `revision == u64::MAX` or advancing a revision when at `u64::MAX` (on edit commit, undo/redo, and electrical remap review) returns a clean `CoreReply::Error` instead of panicking on `+ 1`.
- [ ] Every refusal preserves the accepted document, revision, outline cache, undo/redo contents and protected handoff. Tests exercise nonempty history and a matching remap fingerprint, not earlier guards that avoid the increment.
- [ ] Native unit/integration tests cover each named collection's duplicate rejection on Open/replacement, and max-revision handling across Open, commit, undo/redo and remap review. Include successful `MAX-1` to `MAX` transitions followed by atomic rejection of the next advancing operation.

## Answer

Resolved on `t3/implement-rust-best-practices` (base `c071dedcf`).

- `core::validate::duplicate_top_level_identity` rejects duplicates within each of the six
  collections (`parts`, `definitions`, `boards`, `matrices`, `nets`, `constraints`). Open runs
  it before `prepare_open` and again on the prepared document; a wholesale `ReplaceDocument` is
  checked before phase dispatch and once more at publication.
- Revision advance is computed with `checked_add` before any mutation on edit commit,
  undo/redo and `ReviewElectricalRemap`. Open refuses `revision == u64::MAX`. A refusal keeps
  the accepted document, revision, outline cache, undo/redo stacks and protected handoff.
- Tests: `core/tests/document_integrity.rs` covers every collection on Open and replacement,
  unique-control move, MAX Open, MAX-1 -> MAX -> atomic refusal, undo/redo at MAX, matching
  remap fingerprint at MAX, and failed-Open history preservation.

Evidence: `cargo test -p boardstudio_core --locked --lib` (794 passed) and
`cargo test -p boardstudio_core --locked --test integration` (691 passed; 13 pre-existing
KiCad-environment failures, see final report), `check.py test --receipt` (Core 794 passed).
