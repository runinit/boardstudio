# Map: Ergogen layout parity

Label: wayfinder:map

## Destination

This planning map is finished when two things exist. One is an implementation-ready spec for
the accepted subset of ergogen's point-layout model, covering:

- the parameters and their order of operations;
- how each parameter carries to later rows and columns;
- anchoring one matrix to another;
- Inspector, toolbar and Relations behaviour;
- the format reset.

The other is a set of build tickets, sliced in this plan and ready for TDD work, that keep every
BoardStudio extended layout feature working.

## Planning status

Resolved on 2026-10-23 by [10](issues/10-slice-the-build.md#answer): the consolidated
[spec](spec.md) and [dependent TDD build tickets](#build-tickets) now exist. This closes
planning, not implementation. No shipped version-7 product behavior is claimed; [27](issues/27-end-to-end-acceptance.md)
is the implementation release gate. Tooling [15](issues/15-generate-parity-fixtures.md#answer)
is complete; the unshipped [17 cutover](issues/17-cut-over-format-and-model.md#answer)
and all five subtickets are resolved. The next build frontier is [27](issues/27-end-to-end-acceptance.md) (READY-FOR-AGENT),
now unblocked by resolved [26](issues/26-rebuild-examples-and-fixtures.md#answer);
parent-owned handoff remains pending.
The first-build parity claim excludes aggregate anchors explicitly, not silently.

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
├── 12 Decide how Relations explains chains and anchors (grilling, also after 01, 05, 07) ──────┤
├── 13 Decide format reset and recovery (grilling, also after 07, 11) ──────────────────────────┤
└── 14 Decide starters and demos (grilling, also after 07, 08, 09, 11, 12, 13) ─────────────────┴── 10 Slice the build
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
- [13: Decide the format reset and recovery behaviour](issues/13-format-reset-and-recovery.md)
- [14: Decide how starters and demos are rebuilt](issues/14-starters-and-demos.md)

## Build tickets

Unresolved tickets are `ready-for-agent` subject to their dependency headers. Use the coordinated,
unshipped cutover lane in the spec; do not promote intermediate version-7 writers.

| Ticket | Depends on | Outcome |
| --- | --- | --- |
| [15: Pinned parity fixtures](issues/15-generate-parity-fixtures.md) | 10 | Resolved: 40 upstream cases, 4 worked cases, pinned tooling/provenance/notices. |
| [16: Pure kernel](issues/16-build-layout-kernel.md) | 15 | Resolved: staged kernel, 381 fixture poses, no live activation. |
| [17: Model/format cutover](issues/17-cut-over-format-and-model.md#answer) | 16 | Resolved: grouped records, Layout membership, removed-field callers and strict gates; unshipped lane only. |
| [17a: Grouped model and Core callers](issues/17a-grouped-model-and-core-callers.md) | 16 | Resolved: grouped records, `matrixIds`, explicit correspondences, five failing tests fixed, web callers migrated. |
| [17b: Validation and ownership](issues/17b-validation-and-ownership.md) | 17a | Resolved: dimension bound runs before any row/column walk, grouped validation, step policy, Layout ownership, no anchor-as-origin. |
| [17c: Kernel production routing](issues/17c-kernel-production-routing.md#answer) | 17a | Resolved: projection and parts use the kernel; ordinary saved-pose/member-id oracle coverage replaces the retired test. |
| [17d: Entry-path gates](issues/17d-entry-path-gates.md) | 17a | Resolved: archive unpack gated before the asset table; Core Open, replace and browser list/load gates pinned by tests. |
| [17e: Fixtures and final gate](issues/17e-fixtures-and-final-gate.md#answer) | 17b, 17c, 17d | Resolved: minimum fixture repairs, quick/full gates and independent Standards/Spec re-reviews passed. |
| [18: Core placement edits](issues/18-core-placement-edits.md#answer) | 17 | Resolved: named edits, exact inverses, origin compensation and rigid Column Align; linked constraint-driven Align retains its existing refusal. |
| [19: Anchor/Layout groups](issues/19-anchor-layout-groups.md#answer) | 18 | Resolved: dependency-order evaluation, typed anchor Add/change/Detach with Undo, atomic refusals and frame-preserving lifecycle. |
| [20: Linked group reflection](issues/20-reflect-linked-layouts.md#answer) | 19 | Resolved: linked Add with transported Follow, anchor-by-identity, actual Local, column-0 hold-still, Omitted live pose and Y-mirror refusal proof. |
| [21: Exception lifecycle](issues/21-exception-lifecycle.md#answer) | 20 | Resolved: Local/Omitted resize, Rejoin adoption, confirmed detach and source-removal lifecycle; native matrix-level transitions deferred. |
| [22: Consumers](issues/22-update-layout-consumers.md#answer) | 21 | Resolved: selected-pitch snap, exact Core Align routing with rigid Column proof and Follow refusal; remaining consumers verified with no production change. |
| [23: Inspector/tools/handles](issues/23-inspector-toolbar-and-handles.md#answer) | 22 | Resolved: variant-D Inspector/toolbar/handles through real controls and trusted pointer drags; both re-reviews passed. |
| [24: Relations/highlights](issues/24-relations-and-highlights.md#answer) | 23 | Resolved: Core-backed placement chains, Follow-subset/Local/Omitted anchor overlays, reference navigation and highlight lifecycle; both re-reviews passed. |
| [25: Project recovery](issues/25-project-format-recovery.md#answer) | 17 | Resolved: safe unsupported-project recovery preserves records, drafts, saves and history; full merged gate passed. |
| [26: Examples/fixture finish](issues/26-rebuild-examples-and-fixtures.md#answer) | 24, 25 | Resolved: Studio 60, Macro 12 and Split 42 rebuilt through Core matrix materialization with approved poses; both re-reviews passed. |
| [27: End-to-end acceptance](issues/27-end-to-end-acceptance.md) | 26 | Full production-path proof and release gate, not planning evidence. |

## Ownership

- Fixtures and final gate: [17e](issues/17e-fixtures-and-final-gate.md) owns fixture regeneration and the cutover gates; [26](issues/26-rebuild-examples-and-fixtures.md) owns only the example/starter rebuilds and the goldens they directly cause; [27](issues/27-end-to-end-acceptance.md) owns only the end-to-end checklist and release gate.
- Same-board Local/Omitted/Rejoin: [20](issues/20-reflect-linked-layouts.md) owns linked group reflection; [21](issues/21-exception-lifecycle.md) owns the exception lifecycle.
- Confirmed exception changes: [21](issues/21-exception-lifecycle.md) owns the Core behaviour; [23](issues/23-inspector-toolbar-and-handles.md) owns only the UI confirmation and its trusted-pointer tests.
- Align through anchors: [19](issues/19-anchor-layout-groups.md) owns the Core edit; [22](issues/22-update-layout-consumers.md) owns only the consumer wiring that calls it.

## Decisions so far

<!-- one line per resolved ticket: linked title, then the gist -->
- [17: Model/format cutover](issues/17-cut-over-format-and-model.md#answer): The coordinated unshipped cutover and all five subtickets are accepted; 18 and 25 form the next independent lanes. Release remains gated by 27.
- [17e: Fixtures and final gate](issues/17e-fixtures-and-final-gate.md#answer): All 119 paths passed fresh quick/full gates, native 2,150, page 77/77 and packaged drag/collision 13/13; both final review findings were fixed and independent Standards/Spec re-reviews passed.
- [17c: Kernel production routing](issues/17c-kernel-production-routing.md#answer): Draft/actual projection and parts route through the kernel; nine ordinary fixture cases retain saved-pose/member-id coverage, and the reviewed merged quick passes.
- [25: Project recovery](issues/25-project-format-recovery.md#answer): Incompatible Open/import is validated before switching; owned commits settle and replacements publish only after persistence. Raw download is JSON-only, New is fresh, Delete is named and guarded; merged quick/full and independent Spec/Standards review passed. Conservative confirmation may appear for an already-open editor because there is no shared dirty-state registry. This does not authorize release or user-data writers.
- [18: Core placement edits](issues/18-core-placement-edits.md#answer): Named placement edits, inverse moves, exact origin compensation and rigid Column Align passed the independent Spec/Standards review and fresh merged quick/full gates. The existing linked-pair constraint-driven Align refusal remains for ticket 20; no constrained Align support or release approval is claimed.

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
  - Rust compares poses by `(matrix, row, column)`; ticket 04 qualifies the initial
    tolerance to `1e-6` upstream and `1e-9` for BoardStudio-only regressions.
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
    prototype is not production implementation. Relations semantics are recorded in ticket 12.
- [11: Decide mirrored-half rules](issues/11-mirrored-halves-rules.md#answer):
  - Both linking systems share per-matrix/key Local, Omitted and Rejoin behaviour. New local
    layout values copy unchanged; frames and anchor identities reflect through the pair map.
  - Partial Follow uses transported source-world axes; dependants follow actual Local anchor
    keys. Omitted anchors retain logical poses.
  - Local keys keep world poses; Local matrices keep independent dimensions/layout. Make Local
    detaches a matrix and keeps its pose. Source removal retains a complete Local matrix.
  - Unrepresentable hold-still/Align refuses explicitly; Follow constrains matrix drags.
    Asymmetric keep-pose detach/removal needs confirmation of any additional Local exception.
  - Rejoin preserves child exceptions and validates anchors atomically. Real prepared-route
    lifecycle acceptance is required; no runtime support is claimed by the decision.
- [12: Decide how Relations explains chains and anchors](issues/12-relations-and-highlight.md#answer):
  - One **How it is placed** chain, **Independent adjustments** and **Authored constraints**.
    Core supplies accurate cumulative contributions; Relations supplies presentation, not
    duplicate layout maths. Mixed values say **Varies**.
  - Distinguish matrix carry from direct anchor dependencies and Local/Omitted effects.
    Invisible contributors remain explained without restoring geometry or hidden layers.
  - Violet dashed references highlight named contributors/receivers; amber edit previews show
    actual movement. Navigation opens the appropriate context/tool without editing values;
    Reset/Convert remain explicit actions with accessible blocked reasons.
- [13: Decide format reset and recovery](issues/13-format-reset-and-recovery.md#answer):
  - Planned document version **7**, keeping `boardstudio/v2`, IndexedDB version 1 and archive
    metadata. Refuse older/unversioned input without migration at every entry path.
  - Preserve incompatible projects/assets, with JSON-only raw download, fresh New project
    and guarded confirmed per-project Delete. No global wipe or legacy converter/exporter.
  - Distinguish older/newer/invalid input. Validate before switching; protect drafts, pending
    edits/saves and current history on refusal/cancellation. Publish only after persistence;
    successful replacement starts a fresh editor session/history under existing recovery rules.
  - This is the planned contract, not a running-format bump or permission to erase data.
- [14: Decide starters and demos](issues/14-starters-and-demos.md#answer):
  - Rebuild the same three synthetic geometry-only entries with real matrix/Layout ownership,
    19.05 mm pitch, 1.05 mm edge gap and 18×18 mm keycaps; saved user copies stay untouched.
  - Studio 60 (5×12) and Macro 12 (3×4) preserve geometry using Row 2's carried Shift only.
  - Split 42 uses two linked Layouts with Main/anchored Thumb matrices. Preserve left/thumb
    poses; approve right-main Row 2 X −5 mm and Row 3 X −10 mm to become exact reflection.
  - Keep other parameter/exception stress cases in focused fixtures. Rebuild affected schema,
    scene and pose goldens without retaining old-engine maths or weakening other contracts.
- [10: Slice the build](issues/10-slice-the-build.md#answer): consolidated the accepted
  contract in [spec.md](spec.md), split 13 dependent TDD build tickets (15–27), and
  explicitly deferred aggregate anchors with a qualified first-build parity claim.
  Source checks and planning validation are not implementation acceptance.

## Deferred beyond this build

- **Aggregate anchors.** The user explicitly deferred average/intersection anchors on
  2026-10-23. This build covers accepted parameters and single-key anchors, not complete
  Ergogen points coverage. Future aggregate support needs its own representation,
  evaluation/degenerate-case and UI decisions; see [ADR-0034](../../adr/0034-matrices-follow-ergogen-point-layout.md).

## Out of scope

- **Ergogen YAML import and export.** The data model should still map onto it cleanly
  (ADR-0034).
- **Ergogen's override precedence between levels.** Additive offsets are a deliberate
  difference.
- **Ergogen's `orient`, column-level compounding `shift`, and zone or global rotation about
  world (0, 0).** Each is expressible with the chosen parameters.
- **Changing the default pitch from 19.05 mm.**
