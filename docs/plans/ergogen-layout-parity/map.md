# Map: Ergogen layout parity

Label: wayfinder:map

## Destination

This map is finished when two things exist. One is an implementation-ready spec for matrix
layout that matches ergogen's point model, covering:

- the parameters and their order of operations;
- how each parameter carries to later rows and columns;
- anchoring one matrix to another;
- Inspector, toolbar and Relations behaviour;
- the format reset.

The other is a set of build tickets, sliced in this plan and ready for TDD work, that keep every
BoardStudio extended layout feature working.

## Notes

- Decision record: [ADR-0034](../../adr/0034-matrices-follow-ergogen-point-layout.md). Earlier
  contract: [ADR-0007](../../adr/0007-matrix-gestures-edit-named-parameters.md).
- Evidence:
  - [Ergogen comparison](../../research/2026-10-22-ergogen-layout-engine-comparison.md), with
    measured scenarios S1–S15. Harness: `.evidence/ergogen-layout-comparison/compare.js`, run
    against upstream ergogen `8286e18f` (v4.2.1).
  - [Dependents inventory](../../research/2026-10-22-matrix-layout-dependents-inventory.md),
    which lists extended features and compatibility traps.
- Terms: [GLOSSARY.md, Matrix layout](../../../GLOSSARY.md#matrix-layout). Use the
  `domain-modeling` skill whenever a term changes. Use `grilling` + `domain-modeling` for
  grilling tickets, `prototype` for prototype tickets, `research` for research tickets, and
  `codebase-design` where a ticket shapes a Core interface.
- Tracker: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md), Wayfinding
  operations.
- Standing decisions (2026-10-22). Detail lives in ADR-0034; this is the gist:
  - Parity means behavioural, and is proven by golden fixtures. Ergogen YAML import is out.
  - The existing `Matrix` evolves; there is no new zone tree.
  - Offsets stay additive and act like ergogen `adjust`. They do not move the splay origin.
  - New parameters:
    - column spread (inherited);
    - row padding;
    - a carried shift and a carried rotation at row and key level, with no compounding at
      column level and no `orient`;
    - a splay origin relative to the column base;
    - matrix anchors.
  - Key drags keep writing the key's independent offset.
  - Spacing parameters move key centres only. Key, keycap and plate-opening sizes stay tied to
    pitch minus edge gap.
  - Linked halves' Local and Omitted exceptions cover ergogen `asym`. The Y mirror stays.
  - Extended features keep their behaviour, though their storage may change. The full list is
    in the inventory.
  - There is no migration: the format version is bumped, older documents are refused, and
    starters and fixtures are rebuilt. The default pitch stays 19.05 mm.
  - Electrical scan already pools every matrix on a board into its own scan grid
    (`core/src/electrical.rs::discover_keys`), so extra matrices need no wiring decision.

## Tickets

```text
01 Name the carried parameters (grilling) ──────────────┐
02 Pin ergogen's anchor and mirror semantics (research) ── 07 Define matrix anchors (grilling) ──┤
03 Design the parity fixture pipeline (research) ───────────────────────────────────────────────┤
04 Specify the layout kernel's order of operations (grilling)                                   │
├── 05 Represent "This column" splay as its own intent (grilling) ─────────────────────────────┤
├── 06 Decide how origin edits hold keys still (grilling) ─────────────────────────────────────┤
├── 08 Adapt consumers that assume uniform pitch (grilling) ───────────────────────────────────┤
├── 09 Prototype the Inspector and toolbar (prototype, also after 01) ──────────────────────────┤
├── 11 Decide mirrored-half rules (grilling, also after 05, 06, 07) ────────────────────────────┤
└── 12 Decide how Relations explains chains and anchors (grilling, also after 01, 05, 07) ──────┴── 10 Slice the build
```

- [01: Name the carried parameters](issues/01-name-the-carried-parameters.md)
- [02: Pin ergogen's anchor and mirror semantics](issues/02-ergogen-anchor-and-mirror-semantics.md)
- [03: Design the parity fixture pipeline](issues/03-parity-fixture-pipeline.md)
- [04: Specify the layout kernel's order of operations](issues/04-layout-kernel-order-of-operations.md)
- [05: Represent "This column" splay as its own intent](issues/05-this-column-splay-representation.md)
- [06: Decide how origin edits hold keys still](issues/06-origin-edits-hold-keys-still.md)
- [07: Define matrix anchors and several matrices per layout](issues/07-matrix-anchors.md)
- [08: Adapt consumers that assume uniform pitch](issues/08-uniform-pitch-consumers.md)
- [09: Prototype the Inspector and toolbar for the new parameters](issues/09-inspector-and-toolbar-prototype.md)
- [10: Slice the build into tickets](issues/10-slice-the-build.md)
- [11: Decide mirrored-half rules for the new layout fields and anchors](issues/11-mirrored-halves-rules.md)
- [12: Decide how Relations explains carried chains and anchors](issues/12-relations-and-highlight.md)

## Decisions so far

<!-- one line per resolved ticket: linked title, then the gist -->

- [01: Name the carried parameters](issues/01-name-the-carried-parameters.md#answer): **Shift** and **Bend**
  are the carried move and turn (ergogen `shift` and `rotate`). Offsets and Key rotation keep their
  names. Hints say "later rows". Tools are named after parameters. Relations follows the stagger
  pattern and says "origin", not "pivot". There are no ergogen names in the UI.
- [02: Pin ergogen's anchor and mirror semantics](issues/02-ergogen-anchor-and-mirror-semantics.md#answer):
  - Anchors apply orient → shift → rotate, then `affect`.
  - Mirrored references negate relative x and rotation unless `resist` is set.
  - Anchors see only points generated earlier, which gives the evaluation order.
  - `skip` and `asym` filter keys after layout.
  - Width, height and autobind are outside layout parity.
  - Fixture scenarios are listed.
- [03: Design the parity fixture pipeline](issues/03-parity-fixture-pipeline.md#answer):
  - An opt-in `scripts/generate-ergogen-parity-fixtures.cjs` runs ergogen pinned at
    `8286e18f`, with exact Maker.js and Math.js versions and a lockfile.
  - It writes committed JSON goldens that carry provenance and ergogen key names.
  - Rust compares poses by `(matrix, row, column)` within `1e-9`.
  - Licence notices travel with the tooling.
  - The old projection oracle is replaced by regenerated BoardStudio-only regressions (see the
    ticket's comment).
- [04: Specify the layout kernel's order of operations](issues/04-layout-kernel-order-of-operations.md#answer):
  - Each column steps out by spread (a full step; absent means pitch) and stagger, then splays
    about its base plus a relative origin.
  - Each row steps by padding, then applies the summed row and key Shift, then the summed Bend.
  - Offsets add up and Key rotation follows, both in the key's carried frame.
  - The mirror negates the whole local heading; the anchor frame wraps origin and rotation.
  - Storage is grouped as `columnLayout`/`rowLayout` records. One Core kernel feeds the scene
    (`shiftFrame`, `offsetFrame`, `spreadAxis`, `base`) and drag inverses.
  - Ergogen goldens compare within `1e-6`.
- [05: Represent "This column" splay as its own intent](issues/05-this-column-splay-representation.md#answer):
  - A stored, non-inherited **Column rotation** turns one column about its splay origin; there
    is no counter-splay and no compensation.
  - The Inspector shows Splay and Column rotation as two fields. The Splay drag's "This column"
    choice selects which field it writes. Switching never changes values.
  - Linked halves copy it unchanged.
- [06: Decide how origin edits hold keys still](issues/06-origin-edits-hold-keys-still.md#answer):
  - Hold-still compensation goes into this column's Spread and Stagger. The next column's
    Spread and Stagger change too only when Column rotation ≠ 0. Column 0 writes the matrix
    Origin. It is exact under Bend.
  - Ctrl at pointer-down (or Ctrl-click Pick) swings keys ergogen-style. Nudges and Inspector
    fields always hold still.
  - The Inspector shows the relative origin, and a status line names the changed fields. Each
    edit is one Undo entry.
  - A "Reset to column base" button holds keys still. The base is a snap target.
- [07: Define matrix anchors and several matrices per layout](issues/07-matrix-anchors.md#answer):
  - A Layout lists one or more matrices, and anchors stay inside one Layout.
  - `Matrix.anchor` is `{ matrixId, row, column, follow }`. Origin and Rotation are the shift
    and rotation in the anchor key's final pose. `follow` is ergogen `affect`. There is no
    `orient` and no `resist`.
  - Matrices are evaluated in dependency order, and cycles are refused. Compaction remaps the
    anchor. Removing the anchor key detaches the matrix and keeps its pose.
  - Halves anchor to the partner key. Anchoring to a constraint-driven key is refused.
  - A Key selection offers "Add anchored matrix". Objects lists a Layout's matrices flat.
- [08: Adapt consumers that assume uniform pitch](issues/08-uniform-pitch-consumers.md#answer):
  - Pitch is the default step, the key and keycap size (minus edge gap) and the snap unit, never
    a key position. Keycap fit, previews, draft and resize need only kernel poses.
  - Align on a Column becomes a Core "move this column" edit that writes Spread and Stagger, the
    same function as ticket 06.
  - Convert to stagger is offered only when the column's keys are unturned.
- [09: Prototype the Inspector and toolbar](issues/09-inspector-and-toolbar-prototype.md#answer):
  - Accepted variant D: A's parameter-specific toolbar and grouped Inspector, C's canvas
    handles and collapsible **How it is placed** chain.
  - Handles have a view-only visibility toggle. Spread and Stagger have separate horizontal
    and vertical handles; Alt keeps disabling snapping.
  - Affected-key previews, changed-field status and one Undo entry make carried edits visible.
  - Primary source is archived on local branch `prototype/ergogen-parity-inspector`; the
    prototype is not production implementation. Relations remains ticket 12.

## Not yet specified

- **Format reset.** The user-facing message when an older document is refused, how stored
  browser projects and Undo history are cleared, and the archive import message.
- **Starters.** Rebuilding the three starters and the demos under the new rules, for example
  whether the split starter's thumb keys become an anchored thumb matrix.
- **Aggregate anchors.** Ergogen's `aggregate` anchors (average or intersection of keys), if
  parity work shows they are needed.

## Out of scope

- **Ergogen YAML import and export.** The data model should still map onto it cleanly
  (ADR-0034).
- **Ergogen's override precedence between levels.** Additive offsets are a deliberate
  difference.
- **Ergogen's `orient`, column-level compounding `shift`, and zone or global rotation about
  world (0, 0).** Each is expressible with the chosen parameters.
- **Changing the default pitch from 19.05 mm.**
