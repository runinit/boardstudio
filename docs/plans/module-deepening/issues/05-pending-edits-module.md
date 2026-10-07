# 05: PendingEdits and Matrix tracer integration gate

Status: ready-for-agent
Type: task
Blocked by: 14, 15, 16, 17, 18
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to verify

The user requested smaller implementation slices on 2026-10-07. This ticket retains
its original role as the prerequisite for the three wider panel migration tickets;
it is now an orchestrator-owned integration gate, with no separate coding worktree.
The approved settlement decisions and overall scope are unchanged.

| Implementation slice | Waits for | Owns |
| --- | --- | --- |
| [Edit tickets own captured Scope liveness](14-edit-ticket-scope-lineage.md) | Native Runtime and resolution constructors | Ticket scope port and native tests |
| [PendingEdits owns keyed settlement](15-keyed-pending-edits.md) | Captured Scope liveness | Dioxus-free collection and interface tests |
| [Shared UI helpers present pending edits](16-pending-edit-ui-helpers.md) | Keyed settlement | Signal helpers and mounted helper tests |
| [Matrix one-shot actions use PendingEdits](17-matrix-one-shot-pending-edits.md) | Keyed settlement | Preset/delete/unlink and variant observations |
| [Matrix fields complete the PendingEdits tracer](18-matrix-field-pending-edits.md) | UI helpers and Matrix actions | Fields, helper binding and complete mounted tracer |

Shared helpers and Matrix actions run concurrently after the keyed interface merges.
Matrix fields follow both, so agents never edit the Matrix controller concurrently or
work against unfinished helper code. The exact collection contract lives in its
slice and Outcome; the Signal contract lives in the shared-helper slice and Outcome.

## Acceptance criteria

- [ ] Every implementation slice is merged, reviewed and resolved.
- [ ] Native collection tests cover latest ticket per key, one-shot pending until
  settled, failure message, silent retirement on Scope/owner departure and on
  Superseded/Cancelled/Closed, and Landed revision.
- [ ] Edit ticket tests cover captured-scope retirement using the real Runtime.
- [ ] MatrixSubmission, MatrixPresetSubmission, MatrixDeletionSubmission,
  MatrixEditState::Saved and all four original settle_pending functions are gone;
  the Matrix active-session retirement message is gone.
- [ ] Matrix mounted tests and the shared-helper interface tests pass; shallow
  settlement tests have been replaced rather than retained as another policy layer.
- [ ] Contracts and examples are recorded for the downstream panel implementers;
  known baseline failures and unexecuted tests are reported accurately.

## Verification

The orchestrator reuses sufficient fresh evidence from the implementation slices and
checks their combined tree. Run additional affected checks only for integration
changes or unresolved risks; do not claim known baseline gates are green.

```sh
cargo test -p boardstudio-web-runtime -p boardstudio-web-ui-shared -p boardstudio-web-layout --locked
python3 scripts/check.py lint typecheck test browser
```

Resolve this gate only when the complete tracer is demonstrated. Layout, Parts/PCB
and Case/Keymap/Keycaps/Library migrations still wait for this gate, rather than an
unfinished Runtime interface or helper-only checkpoint.
