# Map: Matrix layout consistency

Label: wayfinder:map
Spec: [spec.md](spec.md) · Source: [handoff](handoff-opus.md)

## Destination

The Inspector, toolbar drags, previews and keyboard nudges all mean the same thing by
each matrix parameter: Stagger is the relative column stagger inherited by later
columns, and offsets are named independent adjustments. Relations shows the inherited
structure of any matrix selection immediately, separate from authored constraints.
Saved projects keep their geometry.

## Notes

- Domain terms: [CONTEXT.md](../../../CONTEXT.md#matrix-layout).
- Edit lifecycle: [ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md) and
  its amendments. Typed Core edit direction:
  [ADR-0006](../../adr/0006-replace-document-for-import-and-recovery.md).
- Skills: `grilling` + `domain-modeling` for 01; `tdd` for build tickets;
  `codebase-design` for 02's interface; `code-review` before resolving a build ticket.
  Use GitNexus `impact` before each symbol edit and `detect-changes` before each
  commit.
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Preserve the fixes from `39c9bbac2` (bulk key actions, deletion, layers, assembly
  reflow) and `f65ed01a6` (parametric key moves, assembly dependency graph, explicit
  companion constraints and authored offsets).
- The working tree holds unrelated uncommitted edits to `AGENTS.md`, `DESIGN.md`,
  `PRODUCT.md`, `docs/README.md` and `docs/design/project-context.md`. Tickets 06 and
  10, which touch `DESIGN.md`, stage partially.

## Dependency order and file ownership

```text
01 decide contract (resolved)
└── 02 Core SetMatrixPlacement + staggerAxis
    ├── 03 Core companion modes ── 08 relations projection ──┐
    └── 04 web module + Inspector                             │
        ├── 05 toolbar Stagger + Shift quick offset ── 06 Offset tool ──┐
        └── 07 Convert offset to stagger ── 09 Relations panel ◄────────┤
                                            └── 10 canvas highlight ────┴── 11 verify
```

- Core files (`model.rs`, `lib.rs`, `matrix*.rs`, the transcript fixture) belong to
  one ticket at a time: 02, then 03. Ticket 07's Core part rebases on 03 if 03 is
  still open.
- `matrix_transform_operation.rs` and the Inspector belong to 04, then to 07 for the
  Convert button. `layout_transform_toolbar.rs` belongs to 05, then 06.
- After 02, the 03 → 08 line and the 04 → 05/06/07 line run in parallel.
- 06 and 09 both edit `web/src/presentation.rs`, in non-overlapping regions (tool
  activation; Relations). Rebase before committing.

## Decisions so far

- Stagger means the inherited column stagger, and Offset means an independent
  adjustment. The toolbar has a separate Offset tool plus a Shift quick offset. Core
  owns the rules through `SetMatrixPlacement`, `staggerAxis` and companion modes.
  Relations is a web projection over them. Snapping applies to the resulting value.
  Convert is offered only when exact. Highlight is in scope:
  [ADR-0007](../../adr/0007-matrix-gestures-edit-named-parameters.md),
  [ticket 01](issues/01-decide-matrix-transform-contract.md#answer), terms in
  [CONTEXT.md](../../../CONTEXT.md#matrix-layout).

## Progress

<!-- one line per resolved build ticket -->

- 02 `SetMatrixPlacement` and the published stagger axis:
  [Outcome](issues/02-core-set-matrix-placement.md#outcome).
- 03 Core publishes companion placement modes:
  [Outcome](issues/03-core-companion-placement-modes.md#outcome).
- 04 one web module builds every placement edit and the Inspector uses it:
  [Outcome](issues/04-web-placement-module-and-inspector.md#outcome).
- 08 the pure Relations projection lands:
  [Outcome](issues/08-matrix-relations-projection.md#outcome).
- 05 the toolbar Stagger and Shift quick offset use the placement module:
  [Outcome](issues/05-toolbar-stagger-and-quick-offset.md#outcome).
- 07 Core converts an offset to stagger when geometry is preserved:
  [Outcome](issues/07-convert-offset-to-stagger.md#outcome).
- 06 the Offset tool, with activation rules and real-pointer coverage:
  [Outcome](issues/06-toolbar-offset-tool.md#outcome).
- 09 the Relations panel renders the projection with Reset and Convert:
  [Outcome](issues/09-relations-panel.md#outcome).
- 10 Relations references highlight their parts on the canvas without changing selection:
  [Outcome](issues/10-relations-canvas-highlight.md#outcome).
- 11 integrated checks, save/reopen test and a real product gesture pass (with gaps):
  [Outcome](issues/11-integrated-verification.md#outcome).
- 12 corrects two Inspector labels; independent review and the E12+F135 integration cohort passed:
  [Outcome](issues/12-inspector-copy.md#outcome).
- 12 corrects two Inspector labels to match the constraint domain and Properties navigation:
  [ticket](issues/12-inspector-copy.md).

## Still open

- Ticket 07 exports `convert_offset_available` from Core so the UI and Core share the
  rule. That is one more public function than ADR-0007 names; confirm it in 07's
  review.
- Remaining Inspector visual observation/polish belongs to a later `impeccable` pass. Five objective
  source/acceptance packets are complete, but no live visual observation occurred and no rendered CSS
  defect was established. Ticket 12 only covers two copy corrections.

## Limitations of this plan

- The plan rests on source reading at `f65ed01a6`. It ran no regression test and no
  browser reproduction of the stagger mismatch. Earlier fix checks for `39c9bbac2` and
  `f65ed01a6` do not verify the new behaviour.
- The stagger-axis formula and the Convert exactness condition (spec) are derived by
  hand from `matrix::location`. Core computes the axis by differencing `location`,
  and tickets 02 and 07 test both against Core before anything depends on them.
- `build_operation` has HIGH upstream impact (23 symbols). `apply` and
  `EditOperation` (ticket 02) were not impact-checked during planning, so expect wide
  reach. Rerun impact at implementation time.
- Line numbers cited in tickets are for orientation at `f65ed01a6`.
