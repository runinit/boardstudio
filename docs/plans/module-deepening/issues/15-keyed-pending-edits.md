# 15: PendingEdits owns keyed settlement

Status: claimed
Type: build
Blocked by: 14
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

Add the Dioxus-free keyed collection in `web/crates/runtime/src/pending_edits.rs`,
with its export in `runtime/src/lib.rs`. Use the scope-aware ticket from
[Edit tickets own captured Scope liveness](14-edit-ticket-scope-lineage.md).
Keep this slice inside Runtime; do not migrate the Matrix controller or UI helpers.

## Interface contract

Exact Rust names and generic bounds are the implementer's choice, reviewed with
`codebase-design`; publish the tested contract in this ticket's Outcome before the
parallel consumers start.

- `begin(port, key, label, feature, resolver)` starts and observes an edit. The latest
  ticket per key replaces the previous observation; previous Session edits still run.
- `is_pending(&key)` supports one-shot admission until the observed edit settles or
  retires. One key does not block another field key.
- `settle(owner_is_live)` drains terminal observations and returns keyed `Landed`
  (with revision), `Failed` (with message), or `Retired` results. Pending observations
  remain tracked; a terminal result is returned once. The owner predicate answers
  only panel lifetime; the ticket owns Scope liveness.
- There is no Saved status, no presentation state and no automatic retry. Retirement
  carries no failure message. Cloning or Signal storage must not acknowledge an old
  ticket as the replacement ticket's result.

## Acceptance criteria

- [ ] Native tests use real Runtime/Core/save gates at the collection's interface.
- [ ] Cover latest-per-key replacement with both Session operations still executing,
  independent keys, and one-shot pending until settled.
- [ ] Cover Core/save failure wording, Landed revision, Unchanged landing, and
  exactly-once draining of terminal results.
- [ ] Cover Scope change, departed owner, and Superseded/Cancelled/Closed retirement
  through actual Session sequencing; do not hand-settle Runtime.
- [ ] Runtime remains Dioxus-free; the new module compiles natively and for WASM.
- [ ] Outcome contains a small consumer example and the final interface/invariants;
  shared UI helpers and Matrix actions can start without designing another policy.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py lint typecheck test
```

Run affected Runtime browser tests if the shared ticket/port implementation changes.
Complete the handoff's parallel reviews. Later interface changes require coordination
with both consumer owners rather than independent edits to Runtime.
