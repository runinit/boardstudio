# Map: Rust best practices hardening

Label: wayfinder:map

## Destination

Core rejects ambiguous document identities, overflowing revisions and over-depth constraint traversal without changing accepted state. Mechanical identities are not reused within a document session. The CAD worker rejects missing prepared input, archive limit failures preserve bounded memory without noisy Drop output, renderer geometry is correct, and native/WASM lint gates stay clean. Existing support-polygon and firmware admission guards remain intact.

## Notes

- Domain terms: [GLOSSARY.md](../../../GLOSSARY.md) (accepted document, pending edit, field edit, one-shot action).
- Skills: `rust-best-practices`, `tdd` for build tickets, `code-review` before resolving.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Out of scope (by design): sweeping `Result<_, String>` to `thiserror` rewrites where callers only display error text; removing transactional whole-document clones on edit paths; refactoring single-threaded `RefCell` caches in `mirrored_half::reconcile`; repo-wide `#![deny(missing_docs)]`.
- Best-practices heuristics identify places to inspect, not mandatory rewrites. A guarded invariant is not a demonstrated user-input bug; retain fail-closed validation and fix reachable defects first.

## Order

No hard dependencies remain among the open tickets. Prefer 01 (document integrity), 02 (identity reuse), 03 (bounded constraints), then 08 (worker input boundary); 04 (WRL correctness), 06 (archive teardown) and 07 (lint coverage) are independent slices. This is recommended sequencing, not a reason to block a smaller reproducible fix. Ticket 05 is `wontfix`.

## Research decisions (2026-10-19)

[Primary-source validation of 01, 02, 03, 05, 06 and 08](../../research/rust-best-practices-ticket-validation.md) distinguishes reachable defects from defended invariants. No implementation or runtime reproduction was performed during this research.

- **01 — keep:** validate identities before normalization; require atomic revision refusal, including history and protected handoff.
- **02 — rewrite:** preserve session high-water reservations across history/lifecycle changes; remove inaccurate claims that every current allocator overflows on a sparse max suffix.
- **03 — keep:** bound all active dependency edges; fixtures must force deep traversal despite memoization, and validate the chosen budget on WASM too.
- **05 — wontfix:** public polygon/firmware guards already reject the proposed malformed inputs. Do not weaken GPIO admission to emit placeholder firmware.
- **06 — keep:** locked zip 9.0.1 confirms direct stderr on failed Drop finalization. Preserve memory limits and prove teardown behavior with child-process stderr capture and supported receipts.
- **08 — narrow:** fix raw worker missing-prepared input, not repository-wide guarded unwraps. No dependency on 01, 02 or 07 is necessary.
- **04/07 — unchanged:** outside this research follow-up; the earlier ticket review found no blocking specification issue, not runtime proof of correctness.

## Tickets

- [01: Reject duplicate document identities and overflowing revisions on open and commit](issues/01-reject-duplicate-identities-and-revision-overflow.md) — `resolved`.
- [02: Preserve monotonic mechanical feature identities across removal, undo, and overflow](issues/02-preserve-monotonic-mechanical-identities.md) — `resolved`.
- [03: Bound constraint solver recursion depth](issues/03-bound-constraint-solver-recursion-depth.md) — `resolved`.
- [04: Fix WRL primitive tessellation, per-face color indexing, and box normal winding](issues/04-fix-wrl-primitives-colors-and-box-normals.md) — `resolved`.
- [05: Handle untrusted support-polygon floats and malformed firmware GPIO pins without panicking](issues/05-reject-non-finite-support-coords-and-bad-gpio-pins.md) — `wontfix` (existing public guards; see Answer).
- [06: Prevent ZipWriter drop stderr output from corrupting parallel test receipts](issues/06-silence-zipwriter-drop-stderr-on-limit-errors.md) — `resolved`.
- [07: Fix WASM Clippy warnings across workspace crates and widen the lint gate](issues/07-fix-wasm-clippy-warnings-and-widen-lint-gate.md) — `resolved`.
- [08: Reject missing prepared input at the CAD worker receiver boundary](issues/08-replace-fragile-unwraps-and-unreachables.md) — `resolved`.

## Implementation outcomes (2026-10-19)

Tickets [01](issues/01-reject-duplicate-identities-and-revision-overflow.md),
[02](issues/02-preserve-monotonic-mechanical-identities.md),
[03](issues/03-bound-constraint-solver-recursion-depth.md),
[04](issues/04-fix-wrl-primitives-colors-and-box-normals.md),
[06](issues/06-silence-zipwriter-drop-stderr-on-limit-errors.md),
[07](issues/07-fix-wasm-clippy-warnings-and-widen-lint-gate.md), and
[08](issues/08-replace-fragile-unwraps-and-unreachables.md) are `resolved` (with [05](issues/05-reject-non-finite-support-coords-and-bad-gpio-pins.md) `wontfix`). Each ticket's Answer records the
public seam and focused evidence.
