# 36: Committing test for aggregate anchors

Status: claimed
Type: build
Blocked by: None (all dependencies resolved; aggregate evaluation and the Inspector aggregate surface already exist)
Parent: [map.md](../map.md)
Map: [map.md](../map.md)

## Goal

Lock the aggregate anchor surface with a mounted test that commits a real
aggregate change, so the existing navigate-only coverage gains a regression
proof that method and parts edits write through and undo cleanly.

## First RED / GREEN slice

Fail a mounted test: committing an aggregate method change through the
mounted Inspector surface writes the change with one Undo entry, and Undo
restores the prior method with poses unchanged.

## Subsequent acceptance

- [ ] A mounted test commits an aggregate method switch end to end and proves
      the single Undo and Redo round-trip.
- [ ] A mounted test commits a parts change end to end (add or remove a part)
      and proves dependant poses update and Undo restores them.
- [ ] Refusals surface the published reason inline with nothing written; the
      existing navigate-only coverage keeps passing unchanged.
- [ ] No client-side geometry is asserted: expected poses come from
      Core-published values only.

## Checks

Mounted browser tests for the committing aggregate coverage; affected
quick/browser checks.

## Progress

- New tests (no baseline edits):
  - `aggregate_method_switch_commits_with_redo_and_pose_round_trip`
  - `aggregate_part_remove_updates_dependant_pose_with_undo_round_trip`
- Proving tests: 8/8 `aggregate_*` layout browser tests pass, including the 2
  new tests plus the existing navigate-only, method-switch, part-remove,
  multi-pick commit and self-part refusal tests.
- Counts: targeted 2/2 new tests pass; aggregate suite 8/8 pass.
- Quick: `cargo fmt --all --check` clean for the changed test file; workspace
  `repo` (pre-existing broken `../deferred-items-plan.md` links in 28-32) and
  unrelated `mirror_existing_half`/`mirrored_pair_geometry`/`existing_half`
  rustfmt drift fail on base too, so full `check.py quick` cannot go green
  from this ticket alone.
