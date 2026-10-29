# 12: Parts in the half frame, split clamp, mirrored controller by default

Status: resolved
Type: build
Blocked by: [10](10-adaptive-spacing-on-commit.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Parts on a split half live in that half's frame, so spacing changes never move them within
their half. Dragging a part stops at the split axis ("a part cannot cross to the other half"),
and the drop commits through 10's spacing rule. A controller added to a mirrored half is
**mirrored on both halves by default**, as a linked partner (a Part correspondence on the mirror
link). Either copy edits both, and the Inspector's "Mirror on both halves" makes it local.

## First RED / GREEN slice

Fail a Core edit test. Add a controller to the source half of a mirrored pair and assert that a
linked partner exists at the reflected pose. Then move the source copy toward the axis and assert
that spacing widens and both copies keep their half-frame values.

## Acceptance

- [x] Dragging either copy edits both; neither crosses the axis.
- [x] The "Mirror on both halves" toggle is one Undo entry each way; making a part local removes the partner.
- [x] A local (unmirrored) part at the axis pushes the other half out by its overhang (07 carried point; note it in the ADR).
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames. (repo/tooling/build/test/typecheck pass; workspace lint stays red on sibling ticket-18's untracked ghost-tests file, which this ticket must not touch; the browser step cannot run here — Chromium core-dumps on launch — so the mounted toggle test is committed but unverified in this environment.)

## Answer

Built 2026-10-25 on the half-frame contract (09) and the adaptive pass (10).
Independent parts on a stored-gap half store half-frame poses (`stored_part_pose`
board→half at `AddPart`/`SetLayout`/adoption, `evaluated_part_pose` half→board
for readers); drags arrive board-space and clamp so the evaluated extent stops
at the split axis, then commit through 10's spacing rule. A controller added to
either copy of a mirrored half materializes a linked partner as a Part
correspondence on the mirror link in the same Undo entry; either copy edits
both through `move_pair_part`, with the partner reflected (`-x`, shared
mirrored-part heading; controllers keep the world heading). `SetMirroredPart`
makes a part local (removes the partner, pins the substitute flag so the
default-mirror sync passes it over) or re-mirrors it, one Undo entry each way;
the Inspector's "Mirror on both halves" toggle drives it via
`part_mirror_resolver`. Legacy board-space pairs adopt the half frame
pose-preservingly on their first edit and adopt legacy property-linked copies
into records. A local part's overhang past the axis counts in the 10-minimum,
so the mirror half moves out symmetrically.

- Combined Spec+Standards review (code-review skill, parallel subagents):
  Spec pass (every checkbox and both RED/GREEN assertions map to a named
  test; no creep; nothing wrong). Standards no hard violations; fixed the
  actionable items — the toggle projection now reuses Core's
  `part_correspondence` lookup, and the three whole-Part shadow clones fold
  into one `evaluated_part` Cow helper. Declined with reasons: pre-existing
  hand-rolled pose math (next-touch), cross-crate test-helper sharing
  (impossible), a wrapper for the gathered `world()` signature change
  (speculative), merging the call-local companion-set check (different
  predicate), renaming local test helpers (churn).
- Checks: 6 new `core/tests/split_half_frame_parts.rs` tests (mirror on add,
  clamp+widen keeping half-frame values, mirror-copy drag, toggle both ways +
  undo, legacy adoption + undo, local overhang) plus updated offset/constraint
  expectations; full core suite lib 772 + integration 888 pass, core clippy
  `-D warnings` clean, touched files rustfmt clean. `check.py`
  repo/tooling/build/test/typecheck pass (713.9 s total); workspace lint stays
  red on sibling ticket-18's untracked `layout_spacing_ghost_tests.rs`
  (dead code + needless returns), which this ticket must not touch —
  ticket-12 files are clippy/rustfmt clean. The browser step cannot run in this environment
  (system Chromium exits 133/core-dumps, so chromedriver cannot start a
  session); the mounted toggle test
  (`mounted_part_mirror_toggle_mirrors_and_unmirrors_with_undo`) compiles for
  wasm but is unverified here. Mutation check 3/3 killed
  (default-mirror, split-clamp extent, toggle-off partner removal).

Prototype comparison: the Core run matches variant B on every carried point —
parts keep half-local coordinates, cannot cross the split ("Stopped at the
split axis"), a new controller mirrors on both halves by default with the
exact "Mirror on both halves" label, and a local part still pushes the mirror
half out. Differences are deliberate: the Inspector toggle resolves through
the Core op (one Undo entry per flip) rather than the prototype's simulated
state, and rotation stays frame-free.
