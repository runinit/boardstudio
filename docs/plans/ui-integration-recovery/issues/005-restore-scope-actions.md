# 005: Restore nonduplicated scope actions

Status: claimed
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A3/A4](../../../research/2026-11-06-dev-ui-integration-audit.md#a3a4--duplicate-actions-explicitly-expected-by-the-new-test)

## What to build

All five scope choices (Matrix, Row, Column, Key and Part) select or retain the
active scope without opening an action or hint popup. One inline surface shows
the intended contextual actions, or useful empty-selection guidance until an
applicable object is selected. Keep the current shared controls and existing
Inspector access; do not introduce a second editor owner.

Align and Transform sit beside the scope choices when space permits and wrap
directly underneath otherwise, without an empty reserved row between them.
This is the user's approved 2026-11-09 scope/placement decision, shared with
[011 D10/D16](011-populated-workspace-acceptance.md#second-batch-correction-execution-plan--2026-11-09).

## Acceptance criteria

- [ ] For Matrix, Row, Column, Key and Part, choosing or reselecting scope
  preserves the intended active state and action availability without opening
  an action or hint popup, including when no applicable object is selected.
- [ ] With no applicable selection, useful selection guidance appears inline.
  Selecting an applicable object replaces the guidance with its inline actions;
  the old hint popup and its redundant controls do not remain interactive.
- [ ] Each intended action appears once across the competing toolbar/popover
  surfaces, including scope changes and reselection. Hidden duplicate labels
  alone are not sufficient if duplicate interactive controls remain.
- [ ] Keyboard scope navigation, action access and focus remain correct. Escape
  still dismisses actual action menus and restores their trigger focus, but scope
  presses themselves do not create a popup or move focus into one. Preserved
  canvas handles and Command palette actions are not unwanted duplicates.
- [ ] Align and Transform are beside the scope choices when they fit and wrap
  directly underneath when necessary, with no empty reserved row separating
  them. A selection change does not displace this compact scope/menu grouping.
- [ ] Restore and strengthen the original Matrix/Row/Column assertions (A4),
  covering equivalent Key/Part inputs in the same stories. Combine focused
  toolbar fixtures with production Layout checks for real selection and focus
  transitions, recording actual RED/GREEN rather than source-presence proof.
- [ ] Update relevant design and recovery records alongside the behavior (A8).
  Preserve the shared control migration and avoid changes to transform semantics
  or unrelated command surfaces.

## Verification and completion

Run focused mounted toolbar and production Layout selection/focus tests, recording
exact failing and passing invocations and source identity. Run applicable
compile/style checks for the touched modules. All five scopes remain one slice
because they share the scope-control implementation. Keep the parent
specification and original audit unchanged. The coordinator owns this ticket,
final combined Spec + Standards review, independent sensitivity checks and the
full gate; the implementation owner returns commands, logs and changed files.

### Approved proof map

These rows preserve the approved claims; their statuses distinguish executed
proof from remaining gates. Reuse the existing toolbar and production Layout
mounting seams; no new test-only production API is approved.

| Requirement — precondition → action → independent oracle | Tier | Status |
| --- | --- | --- |
| Scope choice/reselection — applicable selection or empty scope → choose/rechoose Matrix, Row, Column, Key or Part → intended active scope retained, no action/hint popup, intended inline actions occur once or useful inline guidance appears | Mounted toolbar; equivalent inputs table-driven | producer RED/GREEN: `.evidence/011-round2-D0/red-scope.log`, `green-layout-checks.log`; fresh combined Standards PASS / Spec PASS, independent sensitivity pending |
| Guidance handoff — real Layout with no applicable selection and inline guidance → select the relevant object on the canvas → guidance replaced by its usable inline actions, no duplicate popup/editor owner | Production Layout mounted | initial persistent Matrix-clear RED corrected privately in the page; unchanged all-five-scope story now GREEN, including clear→guidance and Inspector reopening, after correcting the existing host's Close teardown; 23 affected browser checks GREEN in `.evidence/011-round2-D0-cleanup/green-affected.log`; fresh combined Standards PASS / Spec PASS; both independent page-guard mutants killed with restored GREEN, remaining toolbar sensitivity pending |
| Keyboard/focus — scope strip and actual Align/Transform menus → navigate scopes, activate/reselect, enter an action menu and Escape → focus follows the existing toolbar pattern, no scope popup, real menu returns focus to its trigger | Mounted toolbar plus existing production focus scenario | producer GREEN: `.evidence/011-round2-D0/green-layout-checks.log`, `green-existing-page-focus-selection.log`, `packaged-proof.log`; fresh combined Standards PASS / Spec PASS, independent sensitivity pending |
| Compact grouping — wide and constrained canvas widths, before/after selection → render/resize/select → Align/Transform beside the scope strip or immediately beneath it, normal control gap rather than an empty action-row gap, no horizontal clipping | Mounted measured layout; packaged Theme/Appearance checks | producer RED/GREEN: `.evidence/011-round2-D0/red-grouping-measured.log`, `green-layout-checks.log`, `packaged-proof.log`; 12 combinations; fresh combined Standards PASS / Spec PASS, independent sensitivity pending |

## Comments

- 2026-11-06: Cancelled by the user. This ticket will not be actioned; its
  acceptance criteria are retained as historical context, not completed evidence.
- 2026-11-06: Published from the accepted breakdown; independent of MCU diagnosis
  and the other UI behaviors, with shared-file changes coordinated as necessary.
- 2026-11-08: REOPENED by user during 011 human acceptance review (see 011
  D10 dual scope menus). Original cancellation preserved above as history.
  Scoping to follow after 011 defect slices; do not treat the 2026-11-06
  cancellation rationale as current.
- 2026-11-09: Required `/grill-with-docs` decision completed during the second
  011 correction batch. User answered **“yes, yes”** to (1) inline actions or
  empty-selection guidance for all five scopes, with no scope popup even on
  reselection, and (2) adjacent Align/Transform controls wrapping immediately
  below when needed. This explicitly supersedes the old mandatory empty-selection
  hint-panel criterion, not the useful guidance or selection/focus proofs. Claimed
  for Pi Sol medium implementation; no passing result or human acceptance yet.
- 2026-11-09: D0 returned only `layout_toolbar.rs` / `layout-toolbar.css`:
  13 toolbar checks, 268 general Layout plus 2 setup-guide checks, 168 native
  checks, rustfmt and WASM Clippy reported GREEN. Packaged five-scope canvas
  handoffs and 12 Theme/Appearance/width combinations passed. After D1's completed
  PASS/PASS review, a fresh regression owner receives only the existing page-test
  file for persistent mounted guidance coverage. The earlier page run hit D1's
  in-flight gutter RED, not a passing integrated result.
  The initial production-mounted story exposed a real RED: Matrix guidance
  became usable actions and reopened Inspector, but Ctrl-click cleared Runtime
  selection without restoring inline guidance. Four scopes were then unreached;
  the 7 passed / 1 failed receipt remains in `.evidence/011-round2-D0-mounted/`.
- 2026-11-09: The private page correction returned only `presentation.rs`
  (+16/−5): retained Inspector context no longer admits actions or reselects
  objects when Runtime selection is empty. All five unchanged scope cycles
  then passed, reaching a separate test-host Close cleanup RED. Independent
  evidence: `Session::finish_close` retains accepted state and sets lifecycle
  Closed; this host mounted Editor unconditionally. The coordinator changed
  only that existing host to observe its existing version signal and unmount
  Editor at Closed, before restoring pane preferences. No production lifecycle,
  API or scope oracle changed. The coordinator reproduced the cleanup RED,
  then 23 affected browser checks passed, including the full five-scope story,
  field/split-preview/guide, spacing/focus and relation checks; formatting, WASM
  Clippy and whitespace passed. Logs: `.evidence/011-round2-D0-cleanup/`.
  Fresh combined review returned Standards **PASS** / Spec **PASS**, no required
  or optional findings, all 71 baseline/current hunks read; the bounded report,
  source pins and limitations are in [011's D0 execution record](011-populated-workspace-acceptance.md#d0-fresh-combined-review--2026-11-09).
  Coordinator sensitivity, design update, integrated gates and human acceptance
  remain open; no acceptance checkbox is completed.

### Retired guard mapping

The approved inline-only contract replaces popup-opening expectations, not the
selection, guidance, keyboard or focus proof. The fresh review checked:

| Previous guard | Surviving guard and retained proof |
| --- | --- |
| `direct_selection_choices_open_scope_panels_and_support_keyboard` | `direct_selection_choices_stay_inline_and_support_keyboard`: all-five active/theme/keyboard assertions, now also focus retention and no-popup reselection |
| `scope_popover_keeps_hint_when_nothing_selected` | `scope_choices_keep_inline_guidance_when_nothing_selected`: useful guidance, expanded to all five scopes and both presses |
| `column_scope_trigger_opens_migration_actions_when_tools_visible` | `scope_actions_appear_once_on_choice_reselection_and_transform_menu`: Column actions, expanded scope/action uniqueness and actual-menu Escape focus |
| `scope_panel_updates_when_column_selection_arrives_and_can_close` | `inline_guidance_yields_to_actions_without_displacing_compact_scope_menus`: guidance→actions and retained scope, plus measured compact/stable grouping |
| `column_tools_appear_once_when_transform_popover_opens` | Retained; duplicate selector also counts menu-item roles |

The removed `scope_reselect_probe_host` is covered by the existing pill fixture
and the real mounted five-scope story; no replacement mount framework was added.

Coordinator page-guard sensitivity: the unchanged real Editor story caught removal
of the empty-selection toolbar filter at Matrix clear→guidance, and independently
caught removal of the scope-handler return at the next scope's initial guidance
assertion. Both built successfully, restored the exact reviewed page bytes, and
the restored five-scope story passed. Canonical logs/results:
`.evidence/011-round2-D0/coordinator-page-sensitivity/`. Remaining toolbar
sensitivity, DESIGN, integrated/optimized and human gates are not waived.

- 2026-11-10: A8 design/record update recorded: DESIGN.md now carries the
  approved inline-only scope actions, compact Align/Transform grouping,
  guide-overlay frame and uniform-canvas-paint contracts, and the 011/D0
  execution records are current. No acceptance checkbox is closed; remaining
  toolbar sensitivity, integrated/optimized and human gates stay open.
