# 13: Part anchoring with the column vocabulary

Status: resolved
Type: build
Blocked by: [12](12-half-frame-parts-and-mirrored-controller.md), [17](17-canvas-anchor-badge.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

A part is placed with the same layout vocabulary as a column. It has an anchor key with Follow
X/Y/rotation, plus **Spread, Stagger, Splay** carried from that anchor and **Offset, Rotation**
for the part alone. Pick anchor key and Detach keep the part's pose; Detach places it from the
half origin. A dropped part anchors to the nearest key of its half and keeps the drop point,
upright. Dragging edits Spread/Stagger. Editing the anchor column or key moves the part. This
extends ADR-0036's matrix anchors to parts, which needs an ADR update and a GLOSSARY entry.

Placement math decided in the prototype:

```js
// prototype: A = anchor key pose filtered by follow (or half origin when detached)
base = A + rot({x: spread, y: stagger}, A.r)
pose = step({x: base.x, y: base.y, r: A.r + splay}, offset, rotation)
```

The Inspector groups "Carried from the anchor" and "This part only", then Anchor, Relationships
and the placement breadcrumb. Selecting the anchor key lists "Anchors <part>". The canvas anchor
badge from 17 covers parts.

## First RED / GREEN slice

Fail a Core edit test. Anchor a part to Main R3C6, change that column's Stagger, and assert that
the part moves by the transported amount. Then Detach and assert that the pose is unchanged.

## Acceptance

- [ ] Follow subset, Pick and Detach each keep the pose and are one Undo entry each.
- [ ] Mirrored partners use the same half-frame values; splay mirrors.
- [ ] Confirm with the user: dropping anchors to the nearest key (07 carried point).
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Answer

Implemented on branch `docs/ergogen-layout-comparison` (ticket-13 files only;
17's badge WIP and 18's ghost WIP untouched except through shared compilation).

Core (`core/src/matrix/part_anchors.rs`, model, `layout.rs`, `lib.rs`,
`mirrored_half/inverse*.rs`):
- `Part.anchor: Option<PartAnchor>` — anchor key (`matrixId`/row/column) with
  Follow, carried Spread/Stagger/Splay and local Offset/Rotation. `None` is a
  detached part (ticket-12 behavior unchanged).
- `evaluated_part_pose` derives anchored poses from the anchor key's
  board-space pose (`base = A + rot({spread, stagger}, A.r)`,
  `pose = step({base, A.r + splay}, offset, rotation)`). Unresolvable anchors
  hold the stored pose; members/companions never derive; driven parts keep
  their solver pose. The key pose follows the evaluated cell — which rides its
  half through spacing — except Local/overridden members, which are followed
  frozen (their stale stored poses broke the adaptive minimum search).
- Ops, each one Undo entry: `SetPartAnchor` (Pick), `SetPartAnchorFollow`,
  `DetachPartAnchor`, `SetPartAnchorValues`. Retargets re-solve values to keep
  the pose; drags re-solve Spread/Stagger; drops auto-anchor to the nearest
  live key of the half (keeps the drop point, upright); validation refuses
  locked/driven parts, members, cross-layout targets, missing keys and driven
  keys, naming a missing mirror counterpart instead of retargeting.
- Mirrors share half-frame values through a mapped target (counterpart key,
  matrix relative-negate rule) with the source copy as authority; either copy
  edits both. Stored poses refresh on part writes; mirror sync, clamp extents
  and the legacy sync read the evaluated pose.
- RED→GREEN: `core/tests/split_part_anchoring.rs` (7 tests: drop anchors to
  Main R3C6 keeping the drop point upright; Pick/Follow/Detach keep the pose
  with Undo; carried Stagger moves the part by the transported amount; drag
  edits Spread/Stagger; mirror shares values while Splay mirrors). Regression
  hunt fixed two real bugs (member-override staleness in the spacing search,
  typed X/Y on anchored parts). Blessed `boundary/transcript.json`
  (mirrorAddition only: the two hashes downstream of the added anchor; all
  guards still hold).

Web (`anchor_badge.rs`, `matrix_relations.rs`, part Inspector, `presentation.rs`,
`layout_component_edits.rs`):
- Badge: anchored parts badge their anchor key (label, tie, violet member
  highlight, follows text); detached parts badge their half origin;
  members/companions keep their key badge. Canvas render already generic.
- Relations: the anchor key lists "Anchors part" (targeting the part); the
  Relationships summary leads with the anchor fact.
- Inspector: "Carried from the anchor" (Spread/Stagger/Splay), "This part
  only" (Offset/Rotation), Anchor section (key label, Follow toggles, Pick
  matrix/row/column + Follow form, Detach), all committing through the new ops.
  Typed X/Y edits read the evaluated pose for anchored parts.

Docs: ADR-0040 (`docs/adr/0040-part-anchoring-column-vocabulary.md`),
GLOSSARY "Part anchor", scoped prototype-reference cleanup only.

Evidence: Core integration 895 pass (incl. 7 new); web-layout lib 140 pass
(incl. 3 badge + 1 relations new); wasm bin + wasm tests compile; boundary
golden diff is anchor-only. Browser-mounted coverage for the part
badge/Inspector was recorded but not run (browser env-blocked here), same as
ticket 12's toggle test.
Confirmed by the user on 2026-10-26: a dropped part anchors to the nearest
key of its half — the default stands.

Final gate pass (closing): the last `quick` run surfaced 28 `missing field
anchor` errors in web-layout lib-test and pcb test targets (Part literals in
test-only code) plus 6 clippy lints in the new Inspector code (4 redundant
`PartAnchorValueChange` closures, 1 redundant `draft` rebind, 1
`result_large_err` on `anchor_guard`). Fixed by inserting `anchor: None`
(scripted, count-asserted 28), tuple-variant closures, dropping the rebind,
and returning `Option<Resolution>` from `anchor_guard` (4 call sites). Final:
`quick` green through repo/tooling/lint/typecheck/test, `build test` green
(2280 baselined executed, 76 new additive, baseline NOT updated); `browser`
env-blocked — ChromeDriver cannot start Chromium here (reproduced bare with
curl, no app code; same machine condition as 09/10/11/12). Spec review: core
placement math matches the spec formula; Standards review: no hard
documented-standard breaches.
