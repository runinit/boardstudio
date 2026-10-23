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
└── 09 Prototype the Inspector and toolbar (prototype, also after 01) ──────────────────────────┴── 10 Slice the build
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

## Decisions so far

<!-- one line per resolved ticket: linked title, then the gist -->

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

## Not yet specified

- **Relations and canvas highlight.** How Relations explains carried chains (which rows and
  keys inherit a carried shift or rotation) and dependencies on anchors between matrices.
  Waits on the parameter names, the kernel and the anchor rules.
- **Mirrored halves.** The reconcile, inverse-edit and exception rules for each new field and
  for anchors, in `layout::reflected`, `mirrored_half/{inverse,reconcile}`. Graduates once the
  kernel and anchor tickets resolve.
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
