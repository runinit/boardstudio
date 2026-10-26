# 04: Re-anchoring moves the item

Status: ready-for-agent
Type: build
Blocked by: none (decision in [map.md](../map.md#decisions-so-far), 2026-10-26)
Map: [map.md](../map.md)
Supersedes: the pose-preserving Pick in [split-board-editing 13](../../split-board-editing/issues/13-part-anchoring-column-vocabulary.md) and [ADR-0038](../../../adr/0038-part-anchoring-column-vocabulary.md)
Review: **combined Spec + Standards review** (AGENTS.md). This changes the semantics of a public Core edit op (`SetPartAnchor`) and an ADR.

## What to build

A behaviour change in Core. Picking a new anchor (`EditOperation::SetPartAnchor`) **keeps the
part's current carried and local values** (Spread, Stagger, Splay, Offset, Rotation) and places
the part freshly from the new anchor key. The part moves. Today
`core/src/matrix/part_anchors.rs::set_anchor` re-solves those values through `solve_for_pose`,
so the evaluated pose does not change.

- Matrices already behave this way: `matrix::anchors::set_anchor` keeps the matrix's stored
  frame and the matrix moves. Pin that with a test; do not change it.
- Mirrored partners map to the counterpart key and move symmetrically. The existing refusals
  stay: locked, driven, member, cross-layout, missing key, missing mirror counterpart, and
  anchor loops.
- The Inspector's Pick (part Anchor section) and its help text stop promising "keeps position".
- Records: amend ADR-0038's "Pose-preserving retargets" bullet for Pick, with a pointer to this
  decision. Update the GLOSSARY "Part anchor" entry if it says Pick keeps the pose.
- Out of scope: Follow-subset changes, Detach (part and matrix) and drop auto-anchor keep their
  current pose-preserving behaviour (see the carried points).

## Seam and first RED / GREEN slice

Seam: the public Core edit interface (`CoreEngine` apply of `SetPartAnchor`), the same seam as
`core/tests/split_part_anchoring.rs`.

RED: in that file, anchor a part to Main R3C6 with Spread 4, then Pick Main R2C5. Assert that
the stored carried/local values are unchanged and that the evaluated pose equals the placement
formula from ADR-0038 applied to R2C5's key pose, not the old pose. It is one Undo entry, and
Undo restores the old anchor and pose. The existing `pick_keeps_the_pose_and_is_one_undo_entry`
fails against the new rule: rename and rewrite it as a deliberate rename.

## Acceptance

- [ ] RED/GREEN as above, plus a mirrored-pair case (the partner moves by the mirrored amount)
  and a matrix pin test (re-anchoring the thumb matrix moves it and keeps its stored frame).
- [ ] `follow_subset_change_keeps_the_pose` and `detach_keeps_the_pose_from_the_half_origin` still
  pass unchanged.
- [ ] `scripts/mutation-check.py` on the changed `set_anchor` path: a mutant that restores
  `solve_for_pose` is killed. The log goes to `.evidence/`.
- [ ] The boundary transcript (`boundary/transcript.json`) is re-blessed only if hashes downstream
  of a Pick change, and the diff is explained in the Answer.
- [ ] A mounted Inspector check: Pick on a part moves it on the canvas and the Inspector values
  stay as they were.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` for the deliberate rename only.
- [ ] The combined Spec + Standards review has run, with findings adjudicated in the Answer.

## Decided (2026-10-26)

- **Undo and status wording:** YES — one Undo entry, and the status/toast reads
  "<item> anchored to <new anchor> (was <old anchor>) · 1 Undo step". The toast itself
  is [06](06-value-popup-and-commit-toast.md).
- **Scope of "re-anchor":** NO — a Follow-subset change or a Detach does NOT move the
  item; both still hold the pose. Only re-anchoring (Pick/new anchor) moves.
