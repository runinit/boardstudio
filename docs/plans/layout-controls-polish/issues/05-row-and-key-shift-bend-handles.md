# 05: Row and key Shift / Bend handles

Status: resolved
Type: build
Blocked by: [03](03-handle-rail.md)
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D`.
Review: web-only behaviour on existing Core ops; RED/GREEN evidence and a diff read.

## What to build

The rail gains the handles that variant D adds. A row offers **Shift** and **Bend** next to its shipped Offset. A key offers
**Shift** and **Bend**. Each drag edits only its own field through the existing placement change
(`PlacementParameter::RowBend` / `KeyBend` and the matching Shift parameters in
`layout_transform_toolbar.rs`), and each commit is one Undo entry. Hover marks the keys that
would move in amber, using the same key sets as the Inspector field preview
(`matrix_relations::field_preview`: row Shift/Bend carry downward, key Shift/Bend carry upward).
`transform_tools` gains the row and key cases. Hidden-handles mode keeps only the active tool.

## Seam and first RED / GREEN slice

Seam: `transform_tools` (pure) plus the existing drag-to-placement-change mapping.

RED: a native test that `transform_tools` for a Row context returns `[Offset, Shift, Bend]` and for a
Key context returns `[Shift, Bend]`. Today a row returns `[Offset]` and a key returns nothing.
Then a mounted test drags Row Bend by a known amount and asserts one revision with the row's
`bend` changed by the expected angle.

## Acceptance

- [x] Native `transform_tools` and drag-mapping tests for row and key, including hidden handles.
- [x] Mounted tests: row Bend drag, key Shift drag, the hover amber set matching
  `field_preview` for the same field, and one Undo entry each.
- [x] Mirrored keys follow Core's sync rules (Local keys stay put), as with the field preview.
- [x] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser).
  `check.py test --update-native-baseline` only for deliberate test renames.
  (Partial: `check.py test` green incl. the recorded baseline; `cargo fmt
  --check` and `cargo clippy -D warnings` green natively and for wasm32 on both
  touched crates; `cargo check` green natively and for wasm32; scoped
  `wasm-pack` browser suites green — `tool_tests` 21/21, `nudge_tests` 17/17,
  `layout_canvas` page-bin 13/13. Full-workspace `fmt --all`, the full `browser`
  step and the drag smoke were not run to green: the full suite degrades in
  this environment (driver SIGKILL + cascade timeouts, documented above) and
  the smoke is waived by the user for this round. See Answer.)

## Answer

Implemented on branch `docs/ergogen-layout-comparison` (prototype ref
`prototype/layout-controls-iconography` `e91865f43`, variant D).

What changed (ticket-05 files only):

- `web/src/presentation/layout_canvas.rs`: `transform_tools` returns
  `[Offset, Shift, Bend]` for a Row and `[Shift, Bend]` for a Key, keeping the
  shipped Offset first per the carried decision. Hidden-handles mode keeps only
  the active tool per context (Row+Shift/Bend, Key+Shift/Bend added).
- `web/crates/layout/src/matrix_transform_operation.rs`: `gesture_value` gains
  the Bend arm — horizontal world travel turns the running frame at the
  prototype's rate (0.5°/mm, snapped to 0.5°, Alt keeps the raw angle; the grid
  step never reaches a bend). New `BEND_DRAG_DEGREES_PER_MM` /
  `BEND_DRAG_SNAP_DEGREES` consts. The stale "Bend has no drag gesture" comments
  now name Padding only.
- `web/crates/layout/src/objects/layout_transform_toolbar.rs`: new
  `LayoutTransformTool::Shift` / `::Bend` (rail order Offset, Shift, Bend);
  `TransformGesture::Bend`; `PlacementDelta::Bend`; Shift drags reuse the
  Offset gesture's shift axes, Bend drags flow through the standard
  preview/commit/compose path; hover previews the same `field_preview` sets as
  the Inspector fields (row carry downward, key carry upward); new
  `TransformNudge::Bend` mirroring the Splay angle nudge (arrows ±1°, Shift
  ±5°; Shift nudges step like offsets); Shift always shows the four-way glyph,
  Bend the prototype curved arrow; row/key aria-labels
  ("Drag to shift/bend row/key"); Shift/Bend tokens hide on contexts without a
  row or key instead of editing the wrong field; key selections centre their
  rail on the key's cell. Menu popover lists the tools automatically via `ALL`;
  the Row/Key context bars are untouched (rail-only ticket).
- `DESIGN.md`: canvas paragraph now names Shift/Bend and the row/key rails.
- `scripts/check-native-tests-baseline.json`: one sorted insertion for the new
  native bend-drag test (deliberate addition).

Carried point held: the row rail is Offset, Shift, Bend — no shipped tool
removed. Menu/context-bar listing for Shift/Bend beyond the popover is a
follow-up, not this ticket.

Evidence:

- RED then GREEN: new native `bend_drag_turns_horizontal_travel_into_snapped_degrees`
  failed first with "a carried Bend has no drag gesture", passes after.
- Native: `cargo test --workspace` gate green
  ("2367 baselined executed; 1 new", then recorded → 2368).
- Mounted (scoped, `wasm-pack test --headless --chrome web/crates/layout`):
  `nudge_tests` 17/17 (row/key shift drag-mapping, row bend drag + nudge,
  right-half row-shift mirror), `tool_tests` 21/21 (row Bend drag −5° with one
  Undo, key Shift drag +9.525 mm with one Undo, hover parity with
  `field_preview` for all five row/key fields, activation kinds), and the
  `layout_canvas` page-bin suite 13/13 (Row `[Offset, Shift, Bend]`, Key
  `[Shift, Bend]`, hidden handles).
- Full-suite runs degrade in this environment (driver SIGKILL, cascade
  timeouts): the pre-existing rail canary, the key-shift drag, and both
  Inspector bend/shift tests all pass in isolation on this tree. The Inspector
  module's first standalone failure is a detach Core-pose assertion in
  `matrix_transform_inspector_tests.rs` (detach territory, as baselined by 03);
  the inspector never calls `gesture_value`/`nudge_value`, so the new Bend arm
  is unreachable from it. Local-stays-put rides the same Core sync op as every
  other placement param plus the `field_preview` Local exclusion pinned by
  existing `matrix_relations` native tests; no new Local-specific drag test
  (no seam exists short of hand-building hardware pairs).
- Gates: `cargo fmt --check` clean on both touched packages; `cargo clippy`
  `-D warnings` green natively and for wasm32 on both touched crates.

Two-axis review (inline, no subagents per the run's delegation rule): Spec —
Row `[Offset, Shift, Bend]`, Key `[Shift, Bend]`, per-field edits through
existing placement changes, one Undo each, amber hover sets equal to
`field_preview`, hidden-handles parity, mirror sync, drag math/Ctrl/nudges/Undo
preserved, desktop rail only, no 07/08 work. Standards — exact ticket-05 file
list (plus the one-line native baseline insertion), verbatim prototype
rate/snap/glyphs, no dead code, `cargo fmt` + Clippy `-D warnings` green on
both targets.

## Carried point

- Row Offset (`PlacementParameter::RowOffset`) and Row Shift (`RowShift`) are separate fields
  (`web/crates/layout/src/matrix_transform_operation.rs`). Variant D's row rail shows only Shift
  and Bend. Whether to drop the shipped Offset token is undecided. By default the row rail is
  Offset, Shift, Bend; ask before removing a shipped tool.
