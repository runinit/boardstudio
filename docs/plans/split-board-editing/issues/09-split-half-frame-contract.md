# 09: Split half frame and stored gap (contract prefactor)

Status: resolved
Type: build
Blocked by: none (decisions in [04](04-decide-adaptive-split-gap.md) and [07](07-prototype-split-editing-view.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

A mirrored pair gets a stored, editable **Split gap**. Each half's own contents (matrix origins
and parts) are placed in that half's own frame, offset from the split axis by the gap. So
changing the spacing moves whole halves symmetrically and never rewrites their coordinates.
Today the gap only exists at creation: the source matrix origin is written in board coordinates
and the partner reflects about the link's axis. This ticket makes the change easy for 10–13. It
needs an ADR (saved-document contract) and GLOSSARY updates (Split gap, half frame) via
`domain-modeling`.

Existing saved boards must open with identical geometry and no migration. A board saved without
the stored gap keeps its board-coordinate placement until its first committed geometry edit, and
that edit adopts the half frame without moving anything.

## First RED / GREEN slice

Fail a public Core open/edit test. Create a mirrored pair, change the stored Split gap, and
assert that both halves move apart symmetrically, the source matrix origin's half-frame value is
unchanged, and the partner still reflects exactly.

## Acceptance

- [ ] ADR records the half frame, the stored gap and the no-migration-on-open rule.
- [ ] Changing the Split gap is one Undo entry; matrix origins and part poses keep their
  half-frame values.
- [ ] A fixture saved before this change reopens with byte-identical evaluated geometry.
- [ ] PCB, Case and exports that consume half geometry see identical output for an unchanged gap.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

Built 2026-10-25 on the half-frame contract. A mirrored pair stores its Split gap on the link (`splitGap`, optional); each half's unanchored matrix origins are half-frame values offset from the split axis by half the gap at evaluation, anchored matrices inherit through their anchors, and the mirror keeps the negated half-frame Origin/Rotation so the axis reflection holds for any gap.

- Legacy documents (no stored gap) open with byte-identical evaluated geometry and no migration; the first committed matrix-geometry edit adopts the frame (stores the implied gap, re-expresses unanchored origins, moves nothing). Fully anchored pairs stay legacy until 10.
- `set-split-gap` (or an equivalent `SetLayout`) moves both halves symmetrically in one Undo entry without rewriting stored origins; member part poses follow through the refresh. Unlinking or clearing a stored gap restores board coordinates the same pose-preserving way.
- Record: [ADR-0035](../../adr/0035-split-half-frame-and-stored-gap.md) and the GLOSSARY Half frame term. Display and spacing policy stay with 10/11: the Inspector still shows within-half origins, and independent parts stay board-absolute until 12.
- Checks: new `core/tests/split_half_frame.rs` (5 tests: legacy open identical, adopt-without-moving, symmetric gap change + undo, validation, creation stores gap) RED first then GREEN; full core suite 871 integration + 772 unit green; scoped clippy clean; `cargo fmt` clean; `check.py repo` and `tooling` pass. The workspace `lint`/`browser` steps currently fail on sibling in-progress presentation work (tickets 14–16), not on these files.
