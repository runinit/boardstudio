# 05: PendingEdits and Matrix tracer integration gate

Status: resolved
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

- [x] Every implementation slice is merged, reviewed and resolved.
- [x] Native collection tests cover latest ticket per key, one-shot pending until
  settled, failure message, silent retirement on Scope/owner departure and Closed, and Landed revision.
  Superseded/Cancelled mapping is covered at EditTicket; those outcomes are unreachable
  through ResolveEdit-only collection submission (see the keyed collection Outcome).
- [x] Edit ticket tests cover captured-scope retirement using the real Runtime.
- [x] MatrixSubmission, MatrixPresetSubmission, MatrixDeletionSubmission,
  MatrixEditState::Saved and all four original settle_pending functions are gone;
  the Matrix active-session retirement message is gone.
- [x] Matrix mounted tests and the shared-helper interface tests pass; shallow
  settlement tests have been replaced rather than retained as another policy layer.
- [x] Contracts and examples are recorded for the downstream panel implementers;
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

## Outcome

The complete tracer is merged and the wider panel migrations are unblocked. Its
implementation receipts remain in their reviewed, resolved slices:
[captured Scope liveness](14-edit-ticket-scope-lineage.md#outcome),
[keyed collection](15-keyed-pending-edits.md#outcome),
[Signal helpers](16-pending-edit-ui-helpers.md#outcome),
[Matrix actions](17-matrix-one-shot-pending-edits.md#outcome), and
[Matrix fields](18-matrix-field-pending-edits.md#outcome).

The combined source matches the tested final Matrix commit
`83b87cb99c62ff09e663b8e6474a5ca6aac61346`; merge
`209c6d14265ba6f4c648f9a4f198b2a0d949861f` added it without source conflicts.
Root confirmed the five resolved slices, removed legacy Matrix holders/enum/loops,
and reused their pinned reviews and fresh combined verification. Matrix has one
helper-owned collection for ordinary fields/actions; its precise selection, actual
mount/owner retirement and accepted-revision behavior are demonstrated. The sequential
duplicate-variant workflow exception and its guarded recovery are recorded in the
Matrix Outcomes.

The collection and helper Outcomes publish their interfaces and examples; the final
Matrix consumer demonstrates safe bound-Signal lifetimes and mounted owner tracking.
ResolveEdit-only submission cannot emit Superseded or Cancelled, so their existing
EditTicket mapping coverage is retained rather than introducing a substitute Session.
Actual Closed retirement and real captured-Scope retirement are covered as described
in the collection and ticket Outcomes.

Fresh combined native checks passed Runtime 110/110, UI-shared 8/8 and Layout 51
(1 ignored). Lint and WASM typecheck passed. The complete browser step passed,
including Runtime 29, UI-shared 16+1, Layout 121+1 and all 43 mounted page groups;
the full receipt is in the Matrix fields Outcome. Both final Matrix reviews found
no blocking findings. Source is unchanged since those checks, so root did not repeat
the suites for tracker-only commits; repository documentation checks passed.

The full native test step remains limited by the documented unrelated CAD fixture
volume failure (49 passed, 1 failed, 4 ignored), with exact values in the Matrix
fields Outcome. Its short-circuit did not skip browser evidence: the entire browser
step was run separately and passed. The earlier helper-app WASM fixture compile
blocker is repaired in the collection's native-cfg follow-up and verified by the
complete Runtime browser suite.
