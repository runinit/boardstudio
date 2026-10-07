# 14: Edit tickets own captured Scope liveness

Status: claimed
Type: build
Blocked by: 03, 04
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

Make `EditTicket` answer document/board/instance lineage liveness itself. This is the
first slice after the resolution constructors merge; it does not migrate panel
settlement or add the keyed collection yet.

Own `web/crates/runtime/src/edit_ticket.rs` and the Runtime scope-port adapter there.
Change Runtime support files only if the read-only scope source requires it; keep any
`runtime.rs` change in its own small first commit. Keep the existing panel call surface
usable so every intermediate branch compiles without a sweep of workspace files.

## Interface and invariants

- The ticket captures the current `Scope` before submission, and can read the current
  scope through its port at observation. The implementation chooses the ownership of
  that read-only source; callers do not carry a second captured scope for settlement.
- A scope mismatch retires the observation, even when its authoritative operation is
  still pending or has landed. A retired observation cannot resume as live after the
  panel returns. Dropping/retiring observation does not cancel the Session operation.
- `owner_is_live` describes only the panel's selection/mount lifetime. Accepted revision
  and token changes inside the same Scope do not retire a queued edit.
- Observe before submit, preserve failure wording and landing revision, and use the
  real native Runtime from the native Runtime ticket. Do not fabricate scope changes,
  replace a Session under an active gate, or add a production native Runtime.

## Acceptance criteria

- [ ] Native interface tests fail first for captured-scope retirement, then pass.
- [ ] Held Core/save operations demonstrate retirement on real navigation or Open;
  use actual Session sequencing, including Open/Close draining active work.
- [ ] Revision changes in the same Scope remain live; departed panel owners retire.
- [ ] Existing ticket outcomes and Runtime consumers continue to compile and pass.
- [ ] The port/lifetime contract is documented for the keyed collection implementer.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py lint typecheck test
```

Run the affected Runtime browser suite for changes to the WASM port. Record known
baseline gate failures separately; test-only native changes need no unrelated browser
rerun. Complete the handoff's parallel Standards and Spec reviews before integration.
