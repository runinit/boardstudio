# Map: Rust best practices hardening

Label: wayfinder:map

## Destination

Core, renderer, contracts, archive, and web workspace crates reject malformed or overflowing inputs with clean errors instead of panicking or silently corrupting document state, preserve monotonic feature identities across removal and undo, keep parallel test receipts clean of uncaptured drop output, and pass strict Clippy linting across both native and WASM targets.

## Notes

- Domain terms: [CONTEXT.md](../../../CONTEXT.md) (accepted document, pending edit, field edit, one-shot action).
- Skills: `rust-best-practices`, `tdd` for build tickets, `code-review` before resolving.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Out of scope (by design): sweeping `Result<_, String>` to `thiserror` rewrites where callers only display error text; removing transactional whole-document clones on edit paths; refactoring single-threaded `RefCell` caches in `mirrored_half::reconcile`; repo-wide `#![deny(missing_docs)]`.

## Order

```text
01 (duplicate IDs & revision overflow) ──┬── 02 (monotonic mechanical IDs) ──┐
03 (constraint recursion depth)          │                                    ├── 08 (defensive unwrap/unreachable cleanup)
04 (WRL primitives, colors & normals)    │                                    │
05 (support floats & firmware GPIO)      │                                    │
06 (ZipWriter drop stderr in receipts)   │                                    │
07 (WASM Clippy & lint gate) ────────────┴────────────────────────────────────┘
```

## Tickets

- [01: Reject duplicate document identities and overflowing revisions on open and commit](issues/01-reject-duplicate-identities-and-revision-overflow.md) — `ready-for-agent`.
- [02: Preserve monotonic mechanical feature identities across removal, undo, and overflow](issues/02-preserve-monotonic-mechanical-identities.md) — `ready-for-agent` (blocked by 01).
- [03: Bound constraint solver recursion depth](issues/03-bound-constraint-solver-recursion-depth.md) — `ready-for-agent`.
- [04: Fix WRL primitive tessellation, per-face color indexing, and box normal winding](issues/04-fix-wrl-primitives-colors-and-box-normals.md) — `ready-for-agent`.
- [05: Handle untrusted support-polygon floats and malformed firmware GPIO pins without panicking](issues/05-reject-non-finite-support-coords-and-bad-gpio-pins.md) — `ready-for-agent`.
- [06: Prevent ZipWriter drop stderr output from corrupting parallel test receipts](issues/06-silence-zipwriter-drop-stderr-on-limit-errors.md) — `ready-for-agent`.
- [07: Fix WASM Clippy warnings across workspace crates and widen the lint gate](issues/07-fix-wasm-clippy-warnings-and-widen-lint-gate.md) — `ready-for-agent`.
- [08: Replace check-then-unwrap and defensive unreachable calls in fallible production paths](issues/08-replace-fragile-unwraps-and-unreachables.md) — `ready-for-agent` (blocked by 01, 02, 07).
