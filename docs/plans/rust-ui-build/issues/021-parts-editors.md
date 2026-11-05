# 021: Parts editors

Status: resolved
Owner: lane/021-parts-editors (.worktrees/lane-021-parts-editors)
Type: build
Blocked by: [006: Accessible Select and Combobox](006-accessible-select-combobox.md), [008: Accessible Toolbar, Toggle Group, Slider and Toast](008-accessible-toolbar-toggle-slider-toast.md), [011: Docks and narrow-screen sheet](011-docks-and-narrow-sheet.md)
Parent: [Spec](../../rust-ui-migration/spec.md)
Related: [inventory](../../rust-ui-migration/inventory.md), [shared contract](../map.md#shared-contract)

## What to build

The Parts editors: assembly, model, profile and generator editors, custom definition and footprint import. Applies the shared contract in the plan map: raw controls and `Board*` become the mapped components, inline messages are classified, icon-only buttons get Tooltips, selects follow the 12-option rule, tests find controls by role and name, and the lane deletes only the old CSS its files used.

## Acceptance criteria

- [x] The editor rows of the inventory for Parts have zero raw controls and zero `Board*` uses, with precise numeric entry unchanged
- [x] Existing tests pass after the rewrite rule
- [x] The shared contract in the [plan map](../map.md#shared-contract) holds for everything this ticket touches

## Answer

Migrated all 11 Parts-editor files on `lane/021-parts-editors` (15 commits,
`169eff5` claim through `d30391b` fixes; base `feat/rust-ui-migration` tip
`6fc3956f5` unchanged, merge reports already-up-to-date). Zero `Board*` and
zero raw controls remain in owned production code except the mapped patterns:
Button-triggered hidden native file inputs (4 sites), the test-only native
selects in the persisted-selection fixture, and one themed native textarea
(see decision 1). All drafts, parsing, blur commits, key handling, pending
settlement, resolvers and accessible names are unchanged; precise numeric
entry (steps, mins, commit-on-blur/Enter, Escape restore) is preserved
verbatim at every site.

What changed, per vertical slice (each green on host check, native lib
tests, wasm clippy `-D warnings` and fmt before commit):

- `parts/editor_fields.rs` (new, `pub(crate)`): `EditorTextField`,
  `EditorNumberField`, `EditorTextarea`, `EditorSelect` (short lists),
  `EditorChoiceField` (12-option Select/Combobox rule). Composition owners
  keep all state; explicit `aria_label` preserved for existing test names.
- `model_axis.rs`: X/Y/Z `BoardNumberField` becomes the number composition.
- `parts_new_component.rs`: raw button becomes Button Secondary; error
  becomes Alert Destructive; test uses role+name.
- `parts_import_footprint.rs`: Button + hidden file input, Spinner status,
  Alert error, Callout Info notice; test waits on the Button.
- `parts_definition_name.rs`: controlled Collapsible + Field text input +
  FieldError; trigger `aria-expanded` replaces `details[open]`; tests by role.
- `parts_custom_definition.rs`: Kind/Shape Selects, courtyard/pad editor
  fields (fieldset/disabled kept), Button/Callout/Alert messages; pad tests
  by textbox/spinbutton/button role+name.
- `mechanical_profile_editor.rs` + `mechanical_profile_ui.rs`: EditorSelect
  family/purpose pickers, EditorNumberField gap/contour inputs, Button
  actions (Save stays Primary-mapped Default), Spinner/Alert feedback;
  tests by role+name with aria-scoped absence assertions.
- `component_model_editor.rs`: Collapsible + Button + hidden attach input +
  Spinner/Callout/Alert; offset tests by spinbutton role+name.
- `module_profile_editor.rs`: rotary/volume/model editor fields,
  EditorSelect pickers, 12-rule candidate picker (attached options
  disabled), Checkbox reviews with scope ids, Default apply save.
- `generator_settings.rs`: group Collapsibles, parameter editor fields
  (readonly filename, Button + hidden attach, Checkbox booleans,
  EditorTextarea arrays, EditorNumberField/EditorTextField, EditorSelect
  side), Default apply save, Spinner/Alert feedback; tests by role+name
  with aria-checked state reads.
- `assembly_editor.rs`: saved-list Item rows (Ghost open + Secondary
  Duplicate), Callout guidance rows, editor/member/model fields via the
  shared helpers, `EditorChoiceField` Component/Asset pickers fed by pure
  row builders keeping the Visual-model-only and unavailable fallbacks,
  hotswap Checkbox, Button + hidden model import; fixture renders the row
  builders with identical assertions; resolution tests by role+name.
- CSS: deleted 71 dead m1.css lines (old-CSS count 619 -> 590 selectors;
  `--wb-*` untouched at 82 for 024); narrowed the two shared rules whose
  other selectors stay live (`.m1-parts-empty`, `.m1-case-measures`). The
  manual Graphite script now waits on the Assembly editor region by
  role+name.

Decisions (no source-doc updates needed; all within the mapped patterns):

1. `EditorTextarea` is Field + themed native textarea, not a copied
   Textarea: no Textarea copy exists in `ui/` and legacy CSS has no global
   textarea rule, so this is the faithful mapped target.
2. Remove/cancel/secondary actions stay Button Secondary (their old
   default look); only accent-filled apply/save actions map to Default.
3. Multi-part Collapsible trigger names get explicit `aria_label`: RSX
   concatenates adjacent element text without whitespace.

RED/GREEN evidence (all three REDs found by the parts browser launch,
fixed, then green): (1) generator upload scoping — the schema renders
three `*3dmodel_filename` file inputs and the root query hit the wrong
one (`8f02f73`); (2) trigger-name concatenation above (`a001c4b`);
(3) two-pad Remove-button ambiguity, first-match kept (`d30391b`).
Later-run contamination panics were follow-on symptoms, not counted.

Commands + results (logs in lane `.evidence/021/`):

- `check.py repo lint test`: PASS (lint 84s incl. wasm clippy; native
  2104 baselined executed, 0 missing; the 6 new natives are feat-tip
  drift from other lanes — this ticket adds/renames no native tests, so
  no baseline update rides here).
- `check.py build`: PASS (433s).
- `check.py browser --only parts`: 71/71 PASS; `--only case`: 85/85 PASS.
- `check.py quick`: gated by a PRE-EXISTING tooling failure (20 test
  scripts from other lanes unregistered in `check.py`; `check.py` and
  `test-check.py` are byte-identical to the feat tip, and this ticket
  adds no scripts). Full `browser` gate, screen-reader, human 026 and
  final-merge gates remain open for the coordinator/merger.

Follow-ups/risks: none owned. The merger lands this branch; the lane
worktree, branch and evidence stay in place for review.
