# 09: Split half frame and stored gap (contract prefactor)

Status: ready-for-agent
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
