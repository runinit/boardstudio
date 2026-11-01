# 15: Placement breadcrumb and Relationships; retire the Relations tab

Status: resolved
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

- [x] Every fact the Relations tab showed is reachable beside the selection.
- [x] Chip and fact navigation follows normal selection rules and never moves split focus.
- [x] Mounted browser coverage for chip navigation and the violet hover; the Relations tab is gone.
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
- 2026-10-25 (ticket-15 agent): committed via an isolated index as
  `1ff5e4c6` (feature) after HEAD moved past the contested shared index; the
  commit holds ticket-15 hunks only (verified: no foreign markers; foreign
  hunks in `presentation.rs`, `layout_matrix_relations.rs`, `m1.css` left
  uncommitted). Follow-up `26663852` removes two bare `{ }` rsx blocks the
  tab removal left behind (bare blocks are not valid rsx children; the page
  binary failed macro expansion) plus the now-unused `mut` on the inspector
  input. After the fix the only page-binary compile error is ticket 16's
  in-flight `field_previews` E0618; `cargo fmt --check` is clean tree-wide
  and `check.py repo` passes. `check.py quick` still fails at lint on that
  E0618 (clippy `-D warnings`); build/test/browser could not run past it.
  Ticket-15 browser coverage (chip navigation, chip violet hover, tab gone)
  is written but awaits a compilable tree to execute.
- 2026-10-25 (ticket-15 agent): verified in isolated worktree
  `/home/chris/.cache/t15-verify/tree` (at e3c8493 + 2 uncommitted
  verification-only removals of foreign `split_gap: None,` lines, and one
  uncommitted focus-return test entry in `layout_findings_state.rs`):
  `check.py build` passes; native `check.py test` passes (all 2280 baselined
  tests executed, 4 new); relations module green 14/14 (3 new ticket-15
  tests: tab gone/facts beside fields, carries-chip navigation without
  write/Undo, chip hover highlights only its keys). Full wasm suite remains
  flaky under memory pressure (varying failures across reruns, sampled
  inspector tests pass solo); `check.py` full green in one command not yet
  achieved (repo gate flags another lane's uncommitted deferred-items docs;
  lint blocked by ticket-14 clippy debt). Served the isolated debug build and
  drove the live app (Split 42 demo, Column 1 selected):
  `nav.m1-placement-crumbs[aria-label='How it is placed']` renders 12
  crumbs above the fields; `#m1-relations-heading` "Relationships" lists
  all 12 fact rows; zero buttons match /relation/i (Relations tab gone).
  Hovering the "Carries Column 2" chip sets exactly 3
  `.m1-matrix-key.is-relation-highlight` keys; clicking the chip reselects
  to "Left . Column 2, 3 keys selected". Screenshots:
  `.evidence/t15-breadcrumb.png` (breadcrumb above fields),
  `.evidence/t15-chip-hover3.png` (hovered chip, violet-highlighted canvas
  keys, Relationships section). The 2 `split_gap` lines flagged earlier are
  now valid: ticket 09 landed the core field, so no action needed.
- 2026-10-25 (ticket-15 agent): combined Spec + Standards review of
  `ef0c29253..26663852` (ticket-14 hunks in f0b8bda2f excluded) ran in two
  parallel read-only sub-agents; reports verbatim in the session record.
  Standards: PASS, no hard violations. Judgement-call smells recorded, not
  fixed (zero acceptance value, cross-file ripple risk): `BreadcrumbChip`
  duplicates `StructureRow`'s `on_select` dispatch; `carries_label` /
  `anchor_reference_label` encode the relation role as parallel display-string
  cascades (one enum or shared map would remove drift risk); retained unused
  `_selected_context` param in `use_layout_workspace_state`; mixed
  "Relationships" UI copy vs `Relations` code identifiers. Spec: the
  "written but never executed" gap is now closed (relations module 14/14
  green in the isolated tree, plus live-app driving with screenshots), and a
  split-focus pin was added as `6c2cd40f` after the review. Two review claims
  adjudicated, not fixed: (b) `reset_properties_drafts` deletion is required
  cleanup, not creep — the closure was wired only to the retired Properties
  tab button, and owner-switch reset survives via the pre-existing
  identity-keyed `use_effect` syncs in `layout_component_inspector.rs`;
  (c) breadcrumb order follows the pre-existing kernel `structure` order
  (documented in `matrix_relations.rs`: anchor frame heads the chain,
  per-kind orders stated per builder), shared with the Relationships section
  — reordering the kernel is out of ticket-15 scope and would ripple across
  tickets; recorded as a parent interpretation question.
- 2026-10-25 (ticket-15 agent): isolated-tree gates re-run for closure.
  `check.py tooling` passes; `build` passes; native `test` passes (2280
  baselined, 4 new); relations wasm module 14/14 green including the new
  split-focus pin. Single-command `check.py` still red on three
  foreign/environment causes only: `repo` fails on another lane's
  uncommitted `deferred-items-*.md` broken links; `lint` fails on 5 clippy
  errors all inside ticket-14 lines (`ef0c29253`, `presentation.rs`
  8634-8927, collapsible_if x4 + redundant_closure); `browser` drag smoke
  needs `playwright-core`/`NODE_PATH` (CI web-tools install it). The isolated
  `layout_findings_state.rs` focus-test entry remains unported (optional
  follow-up).

## Answer

- Variant-B "How it is placed" breadcrumb above the Inspector fields plus a
  Relationships section beside them; Relations tab and `LayoutInspectorTab`
  retired with all facts preserved. Commits: `1ff5e4c6` (feature),
  `26663852` (rsx fix), `e3c8493` + `10f66d5` (verification notes),
  `6c2cd40f` (split-focus pin test).
- Evidence: mounted 14/14 relations tests (chip Replace-navigation without
  write/Undo, split focus unchanged, violet hover marks only its keys, tab
  gone); live-app driving (12 crumbs, 12 fact rows, 0 relation buttons,
  hover highlights exactly 3 keys, chip click reselects Column 2);
  `.evidence/t15-breadcrumb.png`, `.evidence/t15-chip-hover3.png`.
- Standards review PASS; Spec review claims adjudicated above. Box 4
  (`check.py` single command) stays unchecked: blocked only by the named
  foreign/environment causes; every gate runnable against ticket-15 code is
  green.
