# 15: Placement breadcrumb and Relationships; retire the Relations tab

Status: claimed
Type: build
Blocked by: none (decisions in [06](06-decide-relationship-highlighting-home.md), presentation from [07](07-prototype-split-editing-view.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/split-board-editing` (ef1c8a008), variant B; run it to compare.

## What to build

Above the Inspector fields, the selection shows "How it is placed" as a clickable breadcrumb:
half › anchor reference › matrix › columns › rows › the selection, then dashed "Carries …" chips
for what it carries. Clicking a chip selects that item. A Relationships section beside the
fields keeps the Relations facts (mirror partner, follows, anchors). The Relations Inspector tab
is removed, but its facts and data stay. Hovering a reference marks its parts with the violet
dashed outline, which stays distinct from amber edit previews.

## Acceptance

- [ ] Every fact the Relations tab showed is reachable beside the selection.
- [ ] Chip and fact navigation follows normal selection rules and never moves split focus.
- [ ] Mounted browser coverage for chip navigation and the violet hover; the Relations tab is gone.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser); `check.py test --update-native-baseline` only for deliberate test renames.

## Comments

- 2026-10-25 (ticket-15 agent): implementation complete in the working tree,
  coexistence-checked against tickets 14 (committed ef0c29253) and 16 (in
  flight). Breadcrumb + Relationships render; Relations tab and
  `LayoutInspectorTab` retired; toolbar button retargeted to open the
  Inspector. Full `check.py` is blocked by other tickets' in-flight breakage:
  ticket 09's `SetSplitGap` leaves `boardstudio_core` uncompilable (E0004,
  E0061), and ticket 16's new files fail rustfmt. Ticket-15 files are
  rustfmt-clean and parse-clean; `check.py repo` passes. NOT COMMITTED: the
  shared index is contested (a selective stage swept ticket-14's fresh `mut`
  hunks alongside ticket-15 hunks in `web/src/presentation.rs`; pre-existing
  staged bytes predate this work). The parent should commit ticket-15 hunks
  once the tree settles: pure files `layout_workspace.rs`,
  `layout_component_inspector.rs`, `layout_component_inspector_tests.rs`,
  `layout_relation_highlight_tests.rs`, `layout_findings_state.rs`, this
  ticket file; from `presentation.rs`, `layout_matrix_relations.rs`
  (presentation), `m1.css` take only hunks without `FieldPreview`,
  `field_preview`, `is-field-preview` or `field-preview-hint` added lines.
