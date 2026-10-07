# 05: `PendingEdits`, with the Matrix Inspector as tracer bullet

Status: ready-for-agent
Type: build
Blocked by: 01, 03, 04
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [01 answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

The `PendingEdits` module, the edit ticket's lineage liveness, the `ui-shared` helpers,
and the Matrix Inspector (`web/crates/layout/src/objects/matrix_inspector_controller.rs`)
moved onto them. The Matrix Inspector is the tracer because it has every case: queued
field edits, one-shot preset and delete actions, an unlink with no owner beyond its
ticket, post-landing selection on delete, and the live-owner retired message that the
amendment removes. Line numbers at `915d305c0`, orientation only.

## Interface

A sketch from the decisions; exact names are the implementer's, checked with the
`codebase-design` skill.

- **Edit ticket** (`web/crates/runtime/src/edit_ticket.rs`): `begin` captures the
  current `Scope` (the port gains a scope accessor). The ticket retires itself once the
  current scope differs; the caller's `owner_is_live` now means only the panel's own
  owner (selection generation, mounted target).
- **`PendingEdits<K>`** (`web/crates/runtime/src/pending_edits.rs`, no Dioxus):
  - `begin(port, key, label, feature, resolver)`: starts a ticket; a newer ticket for the
    same key replaces the observed one (latest per key wins; the older still runs in
    Session);
  - `is_pending(&key)`: one-shot controls disable while true;
  - `settle(owner_is_live: impl Fn(&K) -> bool) -> Vec<(K, Settled)>`: drops terminal
    tickets and reports, per key, `Landed { revision }`, `Failed { message }` or
    `Retired`. There is no `Saved` state; `Landed` exists for one-shot follow-ups such
    as post-landing selection.
- **`ui-shared` helpers** (Signal-bound): a text-field helper that keeps the draft
  while pending, and on settle restores the accepted value, showing a failure inline
  and a retirement silently; a one-shot helper that disables while pending and returns
  the failure message for the panel to place.

## Acceptance criteria

- [ ] Native tests at `PendingEdits`' interface cover: latest ticket per key; one-shot
  pending until settled; failure message; silent retirement on scope change, on a
  departed owner and on `Superseded`/`Cancelled`/`Closed`; `Landed` revision.
- [ ] Edit ticket tests cover scope-captured retirement.
- [ ] `MatrixSubmission`, `MatrixPresetSubmission`, `MatrixDeletionSubmission`,
  `MatrixEditState::Saved` and the four `settle_pending*` functions (~1338-1480) are
  gone; "did not complete in the active session" is gone from the Matrix Inspector.
- [ ] Matrix Inspector mounted tests pass, with any asserting the removed message or
  "Saved" status updated; native panel tests that only restated settlement are deleted.

## Verification

```sh
cargo test -p boardstudio-web-runtime -p boardstudio-web-ui-shared -p boardstudio-web-layout --locked
python3 scripts/check.py lint typecheck test
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```
