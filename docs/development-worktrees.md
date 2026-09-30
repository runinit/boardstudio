# Development worktrees

The 2026-09-30 cleanup preserved unfinished work before returning dev to its
committed baseline:

- `boardstudio-outline-wip`: pending outline versions, repair/clearance tools,
  electrical/mechanical changes, generated contracts, and browser tests.
- `boardstudio-performance`: experimental CAD implementations, historical raw
  reports, and proposed performance/preview architecture documents.

Use `git worktree list` to locate these checkouts. Their pending files remain
editable and have not been represented as tested production changes. An additional
hash-verified snapshot lives in the local Codex cleanup-snapshots directory.

Dev retains accepted production optimizations, executable benchmark harnesses,
frozen budgets, baseline controls, and regression tests. Historical performance
reports and experiments belong in the performance checkout. Local agent memory
journals and newly generated measurements are ignored.

The preserved outline snapshot has been reconciled with current dev. See
[outline recovery integration](outline-recovery-integration.md) for preservation,
conflict decisions, and verification. The original pending source files remain
in the outline checkout as recovery evidence; they are not a second integration
queue to apply again. Performance and other independent experiments remain
separate.
