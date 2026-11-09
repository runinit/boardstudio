# Spec: Module deepening

Source: [architecture review 2026-10-07](../../investigations/architecture-review-2026-10-07.html)
(all six candidates) · Decisions in force: [ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md),
[ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md)

## Problem

Edit settlement left one deep module, the edit ticket, but the behaviour around it is
shallow and copied:

- About fifteen panels each rebuild settlement above the ticket: a `*Submission`
  holder, a `{Pending, Saved, Failed}` feedback enum, an owner-liveness check and a
  settle loop. They disagree about a ticket that settles `Retired` while its owner is
  still live (silent in Keycap size, a failure message in the Matrix controllers).
  Resolvers hand-write `base_revision: 0`, an empty `transaction_id` and an epoch check
  that Session's own epoch rejection already makes unreachable.
- Natively, `web/crates/runtime/src/lib.rs` swaps the Runtime for a stub that never
  resolves an edit, so panel tests call resolvers by hand or use `HookPort`, a second
  copy of Session's resolution rules. Only the edit ticket's own tests reach a real
  Session natively.
- `runtime.rs` (8,377 lines) repeats one export capture and currency rule per export
  kind, and carries mechanical package formatting that needs no Runtime.
- The page shell's `Editor` (`web/src/presentation.rs`, ~6,000 lines) holds every
  workspace's state, reachable only through mounted tests.
- Mechanical settings defaults and validation live in the Case crate and copy Core.
- Outline version planning shares one 3,574-line file with its Inspector and overlays.

## Goal

Each concept has one deep module with a small interface that tests cross directly:

- `PendingEdits` owns ADR-0005's settlement rules for every panel;
- native tests drive the real Session and Core through the same Runtime surface panels
  use;
- one export-lease module owns export capture and currency;
- `Editor` composes per-workspace state modules;
- mechanical settings and outline version rules move behind typed Core intents
  ([typed Core edits](../typed-core-edits/map.md) slices).

## Constraints

- ADR-0005 and ADR-0006 stand. Field edits queue and the latest value wins; one-shot
  actions disable while pending; export commits, the captured gesture commit and the
  protected electrical remap keep strict captured-revision checks.
- No user-visible behaviour change, except where a decision ticket settles a policy
  that panels disagree on today (the `Retired` message).
- Crate layering in [architecture](../../architecture.md) holds: `runtime` stays
  Dioxus-free; Signal-bound helpers live in `ui-shared` or above.
- Replace, don't layer: when a test at a deepened module's interface covers a
  behaviour, the shallow tests it replaces are deleted.
- Shared files (`application/src/session.rs`, `runtime.rs`, `presentation.rs`) change in
  small commits of their own, so concurrent tickets rebase cleanly.

## Out of scope

- Splitting `runtime.rs` beyond exports (preview pipelines, project lifecycle): a later
  review once exports and the native Runtime have landed.
- Enforcing ADR-0006, and finer-grained Undo.
- Stale direct-edit previews (`docs/backlog.md`).
