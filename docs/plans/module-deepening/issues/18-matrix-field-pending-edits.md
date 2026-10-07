# 18: Matrix fields complete the PendingEdits tracer

Status: ready-for-agent
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

## Acceptance criteria

- [ ] MatrixSubmission and MatrixEditState::Saved are gone; there is no replacement
  per-panel terminal-state enum or settlement mapping.
- [ ] All four original settle_pending functions are gone, including the field loop;
  field and action outcomes cross PendingEdits/shared helper interfaces.
- [ ] Latest-per-field draft retention, accepted value restoration and inline failure
  work in mounted tests. Retirement never emits the old active-session message.
- [ ] Mounted preset/delete/unlink/variant behavior and post-landing selection from
  the preceding action slice remain correct after helper binding.
- [ ] Hand-resolved tests use the real native Runtime where registered; shallow
  settlement-only tests are deleted in favor of module interface coverage.
- [ ] Search the Matrix files for Submission holders, Saved, direct settlement reads
  and retired-message text. Delete leftovers or document a specific workflow reason;
  never exempt ordinary panel settlement.
- [ ] The parent tracer gate's complete original acceptance criteria are demonstrated,
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
