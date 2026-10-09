# 02: Restore closure mounts in the shipped split keyboards

Status: ready-for-agent
Type: task
Blocked by: 01
Map: [map.md](../map.md) · Related: [typed-core-edits 04](../../typed-core-edits/issues/04-refresh-included-keyboards.md)

## Request

Keyboard ticket 04's recipes set an empty closure collection after enabling each half
(`core/examples/demo_projects/mechanical.rs:~244-255`) because of the 01 defect, so the
ten shipped splits have no closure screws.

## What to do

- Remove the `no-closures-*` edits; keep Core's proposed closure mounts.
- Regenerate fixtures (`build`), then re-record
  `core/tests/fixtures/demo-projects-baseline.json` for the ten paired fixtures only
  (`revision`, `hardware`, plus the generated `case-closure/*` parts and
  `assembly-closure/definition/*` definitions). Unibody, review and archive fixtures do
  not change.
- For each keyboard, record the closure count, any overlap with authored mounting holes
  (Sofle has measured holes; closure holes are de-duplicated only against each other) and
  any new fit findings, since proposed positions can land on switches. Do not hide a
  conflict by moving mounts without a source fact.
- Extend `core/tests/included_keyboards.rs` to export each pair (KiCad) with the closure
  holes present.

## Acceptance

- [ ] All ten splits ship with closure mounts on both halves; exports succeed.
- [ ] Baseline changes limited to the listed fields; no key moved (1e-9 comparison).
- [ ] Per-keyboard closure and fit record in the Outcome.

## Verification

`build`, Core tests, `quick`. Keyboard 04 is still `claimed` on the same recipe,
baseline and fixtures: land this before 04's mounted/live checks and Standards/Spec
reviews so they review the final data once. This Outcome must correct 04's claim that
no part values changed. No ADR.
