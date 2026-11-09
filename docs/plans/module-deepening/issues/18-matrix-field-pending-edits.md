# 18: Matrix fields complete the PendingEdits tracer

Status: resolved
Type: build
Blocked by: 16, 17
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Parent: [PendingEdits and Matrix tracer gate](05-pending-edits-module.md) · Decision: [Pending-edit settlement answer](01-decide-pending-edit-settlement.md#answer), [ADR-0005 amendment](../../../adr/0005-resolve-queued-edits-at-execution.md#amendment-a-retired-edit-is-silent-2026-10-07)

## What to build

Complete the Matrix tracer using the merged Runtime collection and shared Signal
helpers. Own `web/crates/layout/src/objects/matrix_inspector_controller.rs` and
`objects/matrix_inspector.rs`, their field/mounted tests, and only minimal related
module wiring needed to register genuine native interface tests.

Bind fields through the shared field helper and bind the preceding action slice's
presentation through the shared one-shot helper. Rehome its existing logical action
collection under one PendingEditSignals owner and include field keys there; remove the
old direct collection storage. Preserve action-kind key equality and request metadata
for precise Landed follow-ups. Reuse domain admission; do not keep a parallel action
collection or introduce a second settlement policy. Preserve domain
projection, resolver eligibility, catalogue ordering, per-field queueing and exact
post-landing selection. Do not redesign the Runtime/helper interfaces independently;
report any contract gap to the orchestrator and its owner.

Preserve the action slice's tested owner lifetime when binding helpers: mount/editor,
exact selection/context generation and workspace departure retire observation, while
accepted revisions under the same owner keep it live. Retire departed observations
before admitting edits for a new owner; this must never cancel authoritative Session
work. EditTicket owns Runtime Scope liveness, so do not add a second Scope settlement
check. Retain the paired departed-owner/current-owner save-failure regressions and
the exact-selection landing tests.

## Acceptance criteria

- [x] MatrixSubmission and MatrixEditState::Saved are gone; there is no replacement
  per-panel terminal-state enum or settlement mapping.
- [x] All four original settle_pending functions are gone, including the field loop;
  field and action outcomes cross PendingEdits/shared helper interfaces.
- [x] Latest-per-field draft retention, accepted value restoration and inline failure
  work in mounted tests. Retirement never emits the old active-session message.
- [x] Mounted preset/delete/unlink/variant behavior and post-landing selection from
  the preceding action slice remain correct after helper binding.
- [x] Hand-resolved tests use the real native Runtime where registered; shallow
  settlement-only tests are deleted in favor of module interface coverage.
- [x] Search the Matrix files for Submission holders, Saved, direct settlement reads
  and retired-message text. Delete leftovers or document a specific workflow reason;
  never exempt ordinary panel settlement.
- [x] The parent tracer gate's complete original acceptance criteria are demonstrated,
  and the final contracts/examples can be reused by the three panel migration waves.

## Verification

```sh
cargo test -p boardstudio-web-runtime -p boardstudio-web-ui-shared -p boardstudio-web-layout --locked
python3 scripts/check.py lint typecheck test browser
```

Run all affected Matrix mounted tests plus the Layout and shared-helper suites. Do not
start a second browser runner concurrently. Complete parallel Standards/Spec reviews;
report original tracer coverage, executed gates and known baseline limitations to the
orchestrator for the parent gate's Outcome.

## Outcome

Integrated `83b87cb99c62ff09e663b8e6474a5ca6aac61346` through merge
`209c6d14265ba6f4c648f9a4f198b2a0d949861f`. The branch was clean and both
Standards and Spec reviews were pinned to base
`e2f1af89f11c4c8dc945bc71597b470911e5a1db` and that final HEAD. Neither
review found a blocking issue; Standards noted duplicate feedback-update helpers
and fixed test waits as minor follow-ups.

The Inspector now has one `PendingEditSignals<MatrixPendingKey>` for its fields and
ordinary one-shot actions. Field keys identify the field; action equality compares
the action kind while retaining request metadata for exact follow-ups. The controller
owns bound draft/failure Signals so settlement cannot write dropped child Signals.
Field mount initialization refreshes accepted values when no observation is pending;
owner/name-target changes reset fields, while an unrelated accepted revision keeps
the same owner's dirty draft. The consumer and the shared module docs are the
reusable examples for the wider panel migrations.

Actual Inspector mount, editor/context generation, Scope generation and workspace
define panel liveness. EditTicket owns captured Runtime Scope. Helper settlement
restores only an untouched submitted draft, retains newer text and places its
failure inline; retirement clears silently. Callers retain admission and precise
follow-ups. A visible Delete landing drains before its own view unmounts, whereas
an externally hidden Inspector retires even with unchanged selection.

The original submission holders, four settle loops, Saved enum and active-session
retirement message are gone. The duplicate-variant preset's sequential local
`PendingEdits` remains the documented workflow exception: silent retirement is
distinct from genuine failure recovery and guarded clone cleanup. Ordinary field
and action settlement has no second collection or terminal mapping.

The unreachable native field harness (`autotests=false`, removed module path) was
deleted. Its three identity scenarios now execute through the mounted production
controller and real WASM Runtime: equal-baseline changes across two matrices for
five fields, same-owner accepted revision/draft retention, and a real same-Matrix
name-target change. The obsolete Saved-policy assertion is replaced by shared
interface coverage. Mounted tests also cover hidden retirement/failure, a real
hidden Rows landing followed by accepted-value remount, and the existing action
and exact-selection behavior.

Verification: combined native Runtime 110/110, UI-shared 8/8 and Layout 51 passed
(1 ignored); lint and WASM typecheck passed. The final queued-edits browser group
passed 15/15. The complete browser step passed: CAD 15, host 6, Runtime 29,
UI-model 4, UI-shared 16 plus panels 1, Keycaps 27, Library 13, Keymap 21,
Catalogue 12, Case 60, Parts 50, PCB 38, Layout 121 plus setup guide 1, and
all 43 mounted page groups. Chrome was released with no owned runner left.

The combined check passed workspace Rust tests, then stopped at the documented
native CAD baseline: `core_internal_gasket_fixtures_export_connected_positive_regions`,
rotated-concave/bottom expected 80481.2399, actual 80579.55733514718; 49 passed,
1 failed, 4 ignored. Browser was subsequently run as a separate complete step;
the full native test step is not reported as green.

TDD evidence: removing only the Rows completion-status clear made the mounted
regression fail on its own `role=status` Saving indicator beside newer draft `3`
and the inline failure; restoring it passed. The earlier timing-only missing-error
probe is excluded from that evidence. Guarding same-value Signal writes also fixed
an introduced render loop, with mounted Delete and hidden remount regressions green.
GitNexus comparison contained stale/corrupted symbols; source inspection and the
pinned reviews supplied the final scope evidence.
