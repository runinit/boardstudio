# Inventory 001 — Layout workspace + Geometry scripts + board/project metadata names

Scope owned by this ticket ONLY (Wayfinder 001). Starting point
`docs/research/2026-11-07-019-draft-safety-change-map.md` (reused, rechecked
against the dirty worktree; counts not copied). All source evidence below was
re-verified in the dirty worktree via native Gortex MCP `search`/`read`
(symbol IDs cited) plus exact line-span reads. No product code changed.

## Owner table

| # | Owner (component / signals) | Class | Admission | Pending route |
|---|---|---|---|---|
| L1 | `LayoutComponentInspector` constraint composite (`offset_x/y`, `rotation`, `source_part_id`, `offset_axes`, `mirror_axis`, `mirror_coordinate`, `mirror_line_x/y/angle`, `constraint_kind`) | **A** | Explicit Save/Add constraint button only | `LayoutComponentInspectorEdits` one-shots (`Constraint`, `RemoveConstraint`, `MirroredHalfState`, `Layout`); post-submission only |
| L2 | `LayoutComponentInspector` component X/Y position drafts (`x`, `y`) | **B** | Blur commits; Enter commits (with Enter→blur dedupe); Escape restores | `LayoutComponentInspectorAction::SetPosition` → `submit_layout_component_edit` → `begin_field` |
| L3 | `LayoutComponentInspector` part-edge margin draft | **B** | Blur parses+commits; Enter blurs (→commit); Escape restores accepted | `SetOutline` action → `submit_layout_component_edit` |
| L4 | `LayoutComponentInspector` anchor numeric drafts (spread/stagger/splay/offset-x/y/rotation) | **B** | Blur parses+commits (invalid keeps draft + sets error); Enter commits (dedupes next blur); Escape restores | `SetPartAnchorValue` action |
| L5 | `LayoutComponentInspector` component-layout select | **C** | `on_value_change` immediately calls `AssignLayout` | `InspectorField::Layout` one-shot via `submit_layout_component_edit` |
| L6 | `LayoutComponentInspector` anchor Follow / Pick / Detach controls | **C** | Immediate submit on change/click (per code comment "Follow, Pick and Detach submit immediately") | `SetPartAnchorFollow` and related actions |
| L7 | `LayoutComponentInspector` outline checkboxes + mirrored-half / part-mirror controls (margin input stays L3) | **C** | `on_checked_change`/`onclick` submit immediately (`set_outline` → `SetOutline`; `mirrored_half_action` → `SetMirroredHalfState`; part-mirror toggle → `SetPartMirrored`) | One-shot `MirroredHalfState`/`PartMirror`/`Outline` bindings via `submit_layout_component_edit` (post-submission only) |
| L8 | Legacy position `Inspector` X/Y drafts (`web/src/presentation/inspector.rs`) | **A** | `oninput` updates draft + live Core preview; commit ONLY via Enter or Apply button; NO blur commit; Escape cancels+restores | `PendingEditSignals<(), (String,String)>` `bind_field` + `begin_value` via `commit_numeric`; invalid parse returns early (no commit, no `aria-invalid`) |
| L9 | `BoardInspector` board-name draft (`web/crates/layout/src/board_inspector.rs`) | **B** | `oninput` sets draft; blur or Enter calls `commit_draft`; Escape restores accepted | `BoardRenameAction` → `use_board_inspector.on_rename` owner checks → `begin_field("board-name")`; empty trims restore accepted, never submit |
| L10 | `GeometryScriptsEditor` name/source/enabled + serialized draft (`web/crates/ui-shared/src/geometry_scripts.rs`) | **A** | `oninput`/`on_checked_change` update locals + serialized draft ONLY; submit ONLY via Apply script button; script picker `on_select_script` swaps active draft without submitting | `PendingEditSignals<ScriptActionKey>` `bind_field(Apply…)` + `bind_one_shot(New/Apply)`; `begin_field` in Apply handler |
| L11 | `MatrixFieldEditor` name/numeric drafts (`web/crates/layout/src/objects/matrix_inspector.rs`) | **B** | `oninput` sets dirty; blur (`commit(None)`) or Enter commits; Escape restores; choice Select/Combobox commits immediately on selection | `MatrixEditRequest` via `on_edit` (controller `begin_field`); `aria-invalid` = `error_text.is_some()` |
| L12 | `NumericTransformField` drafts (`web/crates/layout/src/objects/matrix_transform_inspector.rs`) | **B** | `oninput` sets dirty; blur or Enter parses finite `f64` and calls `on_edit`; Escape restores; untouched blur never submits (formatting guard + `!dirty` guard) | `MatrixTransformRequest` via `on_edit` (controller `begin_field`); error → `aria-invalid` |
| L13 | `OutlineVersionInspector` version-name draft (`web/crates/layout/src/outline/inspector.rs`) | **B** | `oninput` sets draft; blur commits (non-empty, ≤120, changed); Enter blurs (→commit); Escape restores accepted | `OutlineEdit::RenameVersion` → `submit_action` → `begin_field(VersionName)`; `aria-invalid` = empty draft (validity proxy, not full coverage) |
| L14 | `OutlineDimension` / `OutlineCoordinate` drafts (`web/crates/layout/src/outline/overlays.rs`) | **B** | `oninput` sets local field; blur commits if parse succeeds; Enter blurs; Escape restores | `on_commit(f64)` callback → outline submit path; `aria-invalid` = parse validity |
| L15 | `CompanionPlacementInspector` mode/tweak drafts | **A** | Mode Select sets local signal; X/Y/rotation `oninput` set locals; checkbox sets flag; submit ONLY via Apply placement (validates finite tweak, refuses pair+tweak) | `InspectorField::CompanionPlacement` one-shot; `on_apply` → `CompanionPlacementAction` |
| L16 | Setup-guide project-name draft (`ProjectSetupGuide` + `use_setup_project_name`, `presentation.rs`) | **B** | `oninput` → `on_change` (dirty+drafat); blur → `on_commit` (EditTicket post-submission tracking); Enter blurs; empty restores accepted | `EditTicket::begin("setup-project-name")`, `settle` retained per owner; NOT `PendingEditSignals` |
| L17 | Library project-name forms (`library.rs` Current-project + rename disclosure; admission `project_name.rs`) | **B** | `oninput` sets dirty; blur commits (Enter blurs first); Escape restores; empty restores accepted; `ProjectNameCommitAction.commit` rejects unmounted/changed-owner/empty | `PendingEdits.begin` via `begin_field`-equivalent (`project_name_resolver` + `edits.begin`); `aria-invalid` = `name_failure().is_some()` (post-submission only) |
| L18 | Part/component placement actions (`part_placement.rs`, `objects.rs` AddObject) | **D** | Click-to-place actions, no text draft | Post-submission tickets only; no draft owner |
| L19 | Layout transform tool overlays / align toolbar / snap options / key-size controls | **D/E** | Tool/mode/select state; no typed document draft observed (`KeySizeControls` Size/Rotate submit immediately; `LayoutAlignToolbar` reference select is view state, align buttons one-shot) | None (one-shot or view state) |
| L20 | `ExistingHalfSetup` mirror-axis + source-layout drafts (`web/crates/layout/src/objects/existing_half.rs`) | **A** | Axis `oninput` sets local text only; source Select sets local choice only; submit ONLY via form `onsubmit` → `on_create` (invalid axis / empty / duplicate names return silently; `can_create` disables Create) | `ExistingHalfCreateRequest` → controller owner/currency guards → `begin_one_shot(Create)`; pre-submit draft invisible to gate |

Class key: A = explicit-submit draft with NO blur admission and NO gate
visibility (confirmed unprotected). B = blur/focusout/Enter-committing
(ordering risk only). C = immediate commit on change/select. D =
preview-only or post-submission pending only. E = disposable
navigation/search/filter text (excluded as non-draft). U =
unknown/browser-dependent.

Counts: A=5 (L1, L8, L10, L15, L20) · B=8 (L2, L3, L4, L9, L11, L12, L13, L14, L16,
L17 = 10 blur owners; L2/L3/L4 are three sub-owners of one component) ·
C=3 (L5, L6, L7) · D=2 (L18, L19) · E=0 in scope · U=0.
Per-record: A: L1, L8, L10, L15, L20. B: L2, L3, L4, L9, L11, L12, L13, L14, L16,
L17. C: L5, L6, L7. D: L18, L19. U: none.

## Per-owner notes (source-grounded)

### L1 — Layout constraint composite — Class A (confirmed unprotected)
- Production mount: `layout_workspace::inspector` renders
  `LayoutComponentInspector` when a component projection exists
  (`web/src/presentation/layout_workspace.rs:942–963`;
  ID `ts-boardstudio2/web/src/presentation/layout_workspace.rs::inspector`);
  routed from `workspace_composition::inspector`
  (ID `…/workspace_composition.rs::inspector`).
- Scope/lifetime: per-selected-part `LayoutComponentInspectorOwner`
  (scope, snapshot token, revision, context+generation, part + selection);
  `owner_is_live` keyed on
  `(scope, workspace, part_id, selected_part_ids)`; local `use_signal`
  drafts.
- Value + baseline: `offset_drafts()` seeds source/axes/numbers from the
  active accepted `Constraint` (else first other part + zeros); mirror
  signals seed from `Constraint::Mirror`/`MirrorLine` (lines 692–731).
- Input/change: every numeric constraint input uses `oninput` → `set`
  only (e.g. Axis coordinate `mirror_coordinate`, lines ~2410–2420;
  line anchor/angle `mirror_line_*`, offset X/Y/rotation). No `onblur`,
  no `aria-invalid` on these inputs.
- Admission: single `save_constraint` closure (lines 1366–1478) parses all
  fields; any parse failure sets shared `error` and returns (no
  submission); success calls `SetConstraint`.
- Validation: shared `error` signal → one `FieldError`; no per-field
  `aria-invalid`.
- Pending/save: `pending_edits.bind_one_shot(Constraint/RemoveConstraint/
  MirroredHalfState/Layout)`; submission goes through
  `submit_layout_component_edit` (`presentation.rs:2307`,
  ID `…/presentation.rs::submit_layout_component_edit`) → `begin_field`.
  Before Save there is NOTHING in any pending collection — the gate
  cannot see it.
- Unmount/hidden: local signals die with the component; resync-on-accepted-change
  for constraint numbers not line-verified (unknown — does a Core update
  clobber an unsubmitted constraint draft? recorded below).
- Resync (007, source-confirmed CLOBBER): the `accepted_constraint`
  `use_effect` (`layout_component_inspector.rs:~978–1024`;
  ID `…/layout_component_inspector.rs::LayoutComponentInspector`) rewrites
  EVERY constraint draft (`source_part_id`, `offset_axes`, `offset_x/y`,
  `rotation`, `mirror_axis/coordinate`, `mirror_line_*`, `constraint_kind`)
  from the new accepted value on every accepted change, with NO
  clean/dirty check and NO `is_pending` guard — unlike the X/Y/margin
  effects, which skip while dirty-or-pending. An accepted update while a
  constraint draft is open silently replaces the unsubmitted numbers.
  Retention gap, Class A rationale stands.

### L2 — Component X/Y — Class B
- Same mount/owner as L1. `commit_position` closure (lines 1123–1181):
  invalid parse sets `error` ("Enter a finite position coordinate.") and
  returns; Blur carries an Enter-dedupe stamp plus an untouched-blur guard
  (`draft == format!("{current:.2}")` never submits).
- Inputs `X (mm)`/`Y (mm)` (lines 1896/1923): `onblur` → `commit(Blur)`,
  `Enter` → `commit(Enter)`, `Escape` restores accepted text.
- Ordering risk: blur moves focus AND submits; a pending submit races the
  update gate (standard B).

### L3 — Part-edge margin — Class B
- Input at lines 2042+: `oninput` sets `margin`; `onblur` parses
  `margin()` (`f64`, finite, ≥ 0) and calls `SetOutline` on success, sets
  error on failure; `Enter` blurs; `Escape` restores accepted margin.
- Same ordering-only risk as L2.

### L4 — Anchor numerics — Class B
- Helper at lines 1270–1340 builds one numeric field per anchor value:
  `oninput` sets draft; `onblur` commits when `anchor_commit_value`
  parses (else sets "Enter a finite {label}." and keeps the draft);
  `Enter` commits with a dedupe stamp; `Escape` restores.
- Follow checkboxes / Pick / Detach submit immediately (Class C, L6).

### L5/L6 — Immediate commits — Class C
- Component-layout Select/Combobox `on_value_change` (lines 1673–1691)
  immediately calls `AssignLayout` — no draft.
- Anchor Follow/Pick/Detach: immediate per code comment.

### L7 — Outline / mirrored-half section controls — Class C (resolved 007)
- No outline-select draft exists in this file: the select around line 1714
  is the anchor-matrix Pick select (`pick_matrix.set`, local only, submitted
  by the Pick button → `commit_anchor_pick`, immediate L6 family); the
  section around line 2156 is the constraint-type select (local
  `constraint_kind`, submitted by Save → L1).
- Outline checkboxes (`Include in outline`, `Use board margin`,
  `Allow component body overhang`, ~1940–2110): `on_checked_change`
  clones the baseline outline, flips one flag (margin checkbox also resets
  the `margin` text), and calls `set_outline` → `SetOutline` action
  immediately (`set_outline` def lines 1185–1196;
  ID `…/layout_component_inspector.rs::LayoutComponentInspector`).
- Mirrored-half member buttons (Make local / Omit / Rejoin):
  `mirrored_half_action` closure (lines 899–913) calls
  `SetMirroredHalfState` immediately; the independent-part toggle
  (`Mirror on both halves`, lines 1571–1600) calls `SetPartMirrored`
  immediately. Member buttons and toggle share one `MirroredHalfState`/
  `PartMirror` pending slot (comment lines 893–898).
- Margin numeric input in this section is L3 (Class B), unchanged.
- Admission: every L7 control submits on change/click; none holds a typed
  draft. Ordering risk only (standard C). Former unknown #2 closed.

### L8 — Legacy position Inspector — Class A (preview ≠ protection)
- Mount: `layout_workspace::inspector` fallback `show_position_inspector`
  branch (lines 965–966) renders `super::inspector::Inspector {}`.
- `Inspector` (`web/src/presentation/inspector.rs:50–81, 145–267`;
  ID `…/inspector.rs::Inspector`): `oninput` writes `draft` + `dirty_axes`
  and, when both axes parse finite, fires a live position **preview**
  (`submit_position`). `commit_numeric` (lines 293–355;
  ID `…/inspector.rs::commit_numeric`) is called ONLY from Enter keydown
  or the Apply button. No blur handler on either input. Invalid input
  returns early from preview AND reports via `runtime.report` on commit —
  no `aria-invalid` assigned.
- Pending: `PendingEditSignals<(), (String,String)>` bound
  (`bind_field`/`bind_one_shot`); `commit_numeric` → `begin_value` with
  `commit_position_resolver`. Previews are transient Session state, not
  `PendingEdits`; a settled preview leaves the typed draft unprotected.
- Unmount: two `use_drop` blocks — one resubmits a live preview edit,
  one unbinds + settles `false`. Dirty typed text is NOT resubmitted.

### L9 — Board name — Class B
- `use_board_inspector` (`board_inspector.rs:90–260`;
  ID `…/board_inspector.rs::use_board_inspector`) owns draft/failure/
  pending signals; `BoardInspector` render (lines 371–473;
  ID `…/board_inspector.rs::BoardInspector`) wires `oninput` →
  `draft.set`, `onblur` → `commit_draft` (line 399–401, 433), Enter →
  commit, Escape → restore.
- `commit_draft` (lines 454–477): empty trim restores accepted; unchanged
  name restores accepted (no submit); else `on_rename` →
  `BoardRenameAction`. The `on_rename` callback (lines 184–260) rechecks
  workspace/generation/context/scope/lifecycle/epoch/board/selection
  before `pending.begin_field("board-name", …)`.
- `aria-invalid` not set on this input (failure renders `FieldError`
  only); invalid here means empty (restored, not flagged).

### L10 — Geometry scripts — Class A (confirmed unprotected)
- Mount: `layout_workspace::inspector` returns `GeometryScriptsEditor`
  when `geometry_scripts_open` (lines 925–932).
- `GeometryScriptsEditor` (`geometry_scripts.rs:105–438`;
  ID `…/geometry_scripts.rs::GeometryScriptsEditor`): locals
  `name`/`source`/`enabled` + serialized `draft`, bound via
  `pending.bind_field(Apply…, draft, failure)` (lines 105–139).
- Inputs (lines 360–390): name/source `oninput` and enabled
  `on_checked_change` update locals + `draft` ONLY. No blur handler, no
  `aria-invalid` anywhere on these controls.
- Admission: Apply script button (lines 392–428) → `pending.begin_field`
  with `apply_script_resolver(script_id, name(), source(), enabled())`
  (resolver lines 52–79). New-script button uses one-shot `New`.
- `settle(owner_is_live…)` reprojects accepted text; `use_drop` settles
  `false`; switching scripts (`on_select_script`, lines 230–260) calls
  `pending.settle(false, …)` (retires post-submission pending only) then
  overwrites locals + serialized `draft` from the newly selected script
  WITHOUT submitting and WITHOUT any dirty check — an unsubmitted script
  draft is silently replaced in the visible form (retention gap,
  source-confirmed 007). No toast/guard exists in this handler, and no
  `unsaved`/dirty warning was found anywhere in the file (007 text
  search). Section-level Escape calls `on_back` (leaves the editor,
  dropping the draft). Former unknown #4 closed as confirmed gap.

### L11 — Matrix fields — Class B
- `MatrixFieldEditor` (`matrix_inspector.rs:964–1325`;
  ID `…/matrix_inspector.rs::MatrixFieldEditor`): `oninput` sets
  dirty+draft, clears error; blur → `commit(None)` (line ~1270);
  Enter → `commit(None)`; Escape restores.
- `commit` closure (lines ~999–1094): parses per `MatrixFieldKind`
  (positive-int/number/non-negative/choice/name); failure sets inline
  `error` (projected as `aria-invalid` + `FieldError`); unchanged values
  reset without submitting; else builds `MatrixEditRequest` → `on_edit`
  (controller `use_matrix_inspector` `begin_field`, line 675).
- Choice kind: Select/Combobox `on_value_change` commits immediately
  (Class C sub-case, same component).
- Controller guards (007, source-confirmed): `on_edit` in
  `matrix_inspector_controller.rs` (lines ~575–690;
  ID `…/matrix_inspector_controller.rs::use_matrix_inspector`) rejects
  stale requests BEFORE `begin_field` (line 675): `editor_instance_id`,
  `context_generation`, `scope_generation`, `workspace() == "Layout"`,
  `inspector_mounted()`, `request_id` ordering, `current.owner ==
  request.owner`, editable flag, accepted-matrix lookup, and inline
  `build_operation` pre-validation. Former unknown #5 closed for L11.

### L12 — Matrix transform numerics — Class B
- `NumericTransformField` (`matrix_transform_inspector.rs:991–1238`;
  ID `…/matrix_transform_inspector.rs::NumericTransformField`):
  `oninput` sets dirty+baseline; `onfocus`/`pointerenter` show a
  transient amber move-preview (never writes; cleared on leave/blur/
  unmount); `onblur`/`Enter` run `commit` (lines 1040–1110): skips when
  `!editable || !dirty` (formatting can never submit), parses finite
  `f64`, resets when equal to accepted, else `MatrixTransformRequest`
  → `on_edit` (controller `begin_field`, line 1157). Escape restores.
  Error → `aria_invalid`.
- Controller guards (007, source-confirmed): `on_edit` in
  `matrix_transform_controller.rs` (lines ~915–1000;
  ID `…/matrix_transform_controller.rs::use_workspace_matrix_transform`)
  mirrors L11 (`editor_instance_id`, generations, workspace route,
  `request_id` ordering, `current.owner == request.owner`, editable,
  accepted-matrix lookup, `build_operation` pre-validation, plus catalogue
  readiness and topology guards) before `begin_field` (line 1157) /
  `begin_one_shot`. Former unknown #5 closed for L12.

### L13/L14 — Outline version name + dimension/coordinate — Class B
- `OutlineVersionInspector` version-name input (outline/inspector.rs:505):
  `oninput` sets draft; blur trims, restores-or-renames
  (`RenameVersion` only when non-empty, ≤120, changed);
  Enter → `blur()`; Escape → restore. `aria-invalid` = empty draft.
- `OutlineDimension`/`OutlineCoordinate` (outline/overlays.rs:612/672):
  `oninput` sets local `(value, text)`; blur commits parsed finite value
  (dimension also requires ≥ minimum) or restores display; Enter blurs;
  Escape restores. `aria-invalid` = parse validity.
- All submit through `submit_action` (outline/hook.rs:1268;
  ID `…/outline/hook.rs::submit_action`) → `begin_field` or
  `begin_one_shot`; preview `EditPerimeter` path bypasses pending
  (transient preview, D-like sub-case).
- Beyond L13/L14 (007, source-confirmed, no new draft owner): settings
  numerics (Size, Outline margin, Bridge width, cleanup/clearance fields)
  all render `OutlineDimension` with `on_commit` → blur-commit (Class B,
  same pattern as L14); Corners `Select` `on_value_change` submits
  `SetCorners` immediately (`outline/inspector.rs:~600–640`, Class C);
  Keep-gap / Automatic-cleanup checkboxes submit `SetProtectedGap` /
  `SetRepairEnabled` immediately (`outline/inspector.rs:~700–780`,
  Class C); Generate-automatic / gap-focus buttons are one-shot actions
  (Class D); feature add/remove/settle go through canvas + perimeter
  interactions with transient preview (Class D). No typed draft outside
  L13/L14 version-name + dimension/coordinate inputs.

### L15 — Companion placement — Class A
- `CompanionPlacementInspector`
  (`web/src/presentation/inspector/companion_placement.rs:69+`;
  ID `…/companion_placement.rs::CompanionPlacementInspector`): mode Select
  `on_value_change` sets the local `mode` signal only (lines 274–289);
  X/Y/rotation `oninput` set locals (lines ~321–360); checkbox sets
  `pair_default`. NOTHING submits except Apply placement (lines ~190–240),
  which re-parses, refuses non-finite tweaks and pair+tweak, then calls
  `on_apply`. One-shot `CompanionPlacement` binding only disables/tracks
  post-submission; `use_drop` unbinds + settles `false`.

### L16 — Setup-guide project name — Class B
- `ProjectSetupGuide` input (setup_guide.rs:421): `oninput` →
  `on_name_change`; `onblur` → `on_name_commit`; Enter → blur.
- `use_setup_project_name` (presentation.rs:3065+;
  ID `…/presentation.rs::use_setup_project_name`): `on_change` sets
  dirty + draft; `on_commit` no-ops when clean, restores accepted on
  empty, else `EditTicket::begin("setup-project-name")`. Post-submission
  tickets tracked per owner; empty → restore (no invalid marker).

### L17 — Library project names — Class B
- Current-project form (library.rs:854–903) and rename disclosure
  (library.rs:1095–1136), owner `Library`
  (ID `…/library/src/library.rs::Library`): `oninput` clears failure +
  sets dirty; Enter → blur-to-commit; Escape → restore (+close in the
  disclosure); blur with `!dirty` returns; empty restores accepted; else
  `ProjectNameCommitAction.commit(&draft)`.
- Admission (`project_name.rs`;
  `project_name_resolver`): rejects Unmounted / NoProject / OwnerChanged /
  Empty; trims; unchanged resolves `Unchanged` (lands at revision, still
  counted). `aria-invalid` reflects `name_failure` (post-submission
  only). Delete dialogs (library.rs:1195–1310) hold NO typed draft.

### L18/L19 — No-draft owners — Class D/E (extended 007)
- `ComponentPlacementAction` (part_placement.rs:52 re-export; controller
  `use_controller_placement`:1015+) and the objects-panel AddObject
  chooser are click-to-placeoption selections — no typed value, no draft.
- Transform-tool overlays (`layout_transform_toolbar.rs:2126/3936/4632`
  `onblur`s only clear focus/feedback), align toolbar, snap options:
  view/tool state, no document draft.
- `KeySizeControls` (`layout/objects/keycap_size.rs:26–365`;
  ID `…/keycap_size.rs::identity_key` area): `draft`/`draft_dirty` signals
  exist, but Size `Combobox` `on_value_change` (lines ~243–275) and Rotate
  `Checkbox` `on_checked_change` (~321–347) both set the draft AND call
  `submit` immediately (owner/snapshot/revision check + unchanged/duplicate
  suppression, then `KeySizeRequest` → controller). No text input, no blur
  path. Resync effect (~81–101) preserves a dirty draft across accepted
  changes. Immediate-submit controls (Class C equivalent), NO draft owner.
- `LayoutAlignToolbar` (`layout/objects/layout_align.rs:54–140`):
  reference Select/Combobox `on_value_change` → `on_reference` (view
  state); align buttons `onclick` → `on_align` (one-shot `AlignAction`).
  No text input, no draft. Class D/E, NO draft owner.

### L20 — Existing-half mirror dialog — Class A (new 007, confirmed unprotected)
- `ExistingHalfSetup` (`layout/objects/existing_half.rs:568–760`;
  ID `…/existing_half.rs::ExistingHalfSetup`): source-layouts Select
  writes local `default_choice` only; mirror-axis `InputGroupInput`
  (`type=Number`, lines ~708–726) `oninput` sets local `axis` text only.
  No `onblur`, no `aria-invalid`; invalid axis only disables Create
  (`can_create` requires a finite parse).
- Admission: form `onsubmit` ONLY (lines ~585–620) parses axis/names and
  calls `on_create(ExistingHalfCreateRequest)`; invalid input returns
  silently. Nothing enters any pending collection before Create — the gate
  cannot see the typed axis or choice.
- Pending/save: controller `on_create` (`existing_half.rs:228–320`;
  ID `…/existing_half.rs::use_existing_half`) rechecks open/owner,
  `editor_instance_id`, workspace, `scope_generation`, owner currency
  (scope/token/revision), finite axis, option membership, and dry-runs
  `prepare_existing_half` before `begin_one_shot(Create)`. Post-submission
  only. Cancel/Escape discards by design; unmount drops locals.

## Coverage list (denominator support)

Searched WITH a draft-owner verdict (source-confirmed above):
`layout_workspace.rs` (mount/routing), `workspace_composition.rs`
(routing only, no inputs), `inspector.rs` (L8),
`inspector/layout_component_inspector.rs` (L1–L7 incl. `set_outline`,
`mirrored_half_action`, `accepted_constraint` resync effect),
`inspector/companion_placement.rs` (L15),
`layout/board_inspector.rs` (L9),
`layout/objects/matrix_inspector.rs` (L11),
`layout/objects/matrix_transform_inspector.rs` (L12),
`layout/objects/matrix_inspector_controller.rs` `on_edit` guards (L11),
`layout/objects/matrix_transform_controller.rs` `on_edit` guards (L12),
`layout/objects/keycap_size.rs` (no draft owner — immediate C),
`layout/objects/layout_align.rs` (no draft owner — D/E),
`layout/objects/existing_half.rs` `ExistingHalfSetup` + `on_create` (L20),
`layout/outline/inspector.rs` version-name section (L13) + settings /
gap-repair editors (B/C/D sub-cases, no new owner),
`layout/outline/overlays.rs` dimension/coordinate (L14),
`layout/outline/hook.rs` submit path (pending route for L13/L14),
`ui-shared/geometry_scripts.rs` (L10 incl. `on_select_script` no-guard),
`ui-shared/pending_edit_helpers.rs` (seam only — NOT an owner; documents
`bind_field`/`begin_field`/`settle`/`unbind` semantics, lines 1–64),
`library/library.rs` name forms + delete dialogs (L17, no-draft dialogs),
`library/project_name.rs` admission (L17),
`layout/setup_guide.rs` project-name input (L16),
`presentation.rs` `use_setup_project_name` + `submit_layout_component_edit`
+ `dispatch_layout_component_inspector_action` (routes for L1–L7, L16),
`layout/part_placement.rs` (L18 no-draft),
`layout/objects/layout_transform_toolbar.rs` (L19 no-draft).

Searched with NO draft owner found (negative evidence):
`workspace_composition.rs` (pure routing enum, no inputs);
`layout_transform_toolbar.rs` blur handlers (focus/highlight only);
`library.rs` delete dialogs (confirm/cancel/error UI, no typed draft).

NOT fully covered in this pass (gaps, do not count as safe):
`layout/objects/matrix_setup_fields.rs` (dimension sliders — commit
semantics not verified beyond shared-component use in both Add-matrix
modes), `layout/objects/objects.rs` AddObject
row, `layout/outline/*` feature-canvas editors beyond the inspector
sections above, `layout/outline_lifecycle.rs`
beyond `submit_action`,
`presentation/setup_workspace.rs`, matrix relations/key-size mounts in
`layout_workspace.rs` objects section.
Report-only (home is the setup fragment, NOT edited here):
`layout/objects/matrix_setup.rs` + `matrix_setup_controller.rs`
(creation form S2), `layout/objects/mirrored_pair.rs` `MirroredPairForm`
(explicit-submit: left/right-name + gap `oninput`-locals, submit only via
`onsubmit` → `on_preview(MirroredPairRequest)`; classify A-creation-form
for the census, owned by the setup fragment), catalogue
`physical_setup.rs::propose_with_mirror_pair` (pure proposal builder, no
draft). L7 controls resolved above.

## Explicit unknowns (do not treat as safe)

1. RESOLVED 007 (confirmed clobber, recorded under L1): the
   `accepted_constraint` effect rewrites all constraint drafts on every
   accepted change with no dirty/pending guard.
2. RESOLVED 007 as Class C (see L7): outline checkboxes, mirrored-half
   buttons, part-mirror toggle all submit immediately; margin input is L3.
3. L8 preview interplay: how long a live position preview blocks Session,
   and whether the remaining typed draft is protected after it settles —
   needs a mounted trace (map §Editor evidence, same caveat).
4. RESOLVED 007 (confirmed gap, recorded under L10): `on_select_script`
   settles post-submission pending only and replaces locals with no dirty
   check and no toast/guard anywhere in the file; section Escape → `on_back`
   also drops the draft.
5. RESOLVED 007 (guards confirmed, recorded under L11/L12): both
   controllers check instance/context/scope/workspace/mounted/request-id/
   owner/editable/accepted + pre-validate before `begin_field`.
6. L16/L17: blur-commit races (Enter→blur double paths) and
   unmount-during-pending (guide `dirty` vs Library `mounted` flag)
   need a mounted run; source shows guards but not timing.
7. Browser-dependent: native `type=number` empty/invalid string behavior,
   Enter-implies-blur event ordering, focus loss to dialogs/portals —
   source cannot settle these; no browser run performed in this task.
8. Graph gaps: Gortex `relations(callers)` previously returned no incoming
   edge for `LayoutComponentInspector`; production reachability rests on
   source mount reads (`layout_workspace.rs:942–963`), not the graph.

## Validation

- Gortex MCP `search`/`read` used for all source evidence (symbol IDs +
  line spans above); dirty worktree `01a1b3c…` overlay served exact.
- Native file reads used ONLY for exact line spans after Gortex
  truncation (justified fallback, not shell grep; no `grep`/`cat` run).
- No product file edited; no test/build/browser run; no ticket or map.md
  touched. Deliverable is this file only.
- Counts rechecked, not copied: 5×A / 10×B (8 records) / 3×C / 2×D /
  0×U. Three confirmed-unprotected explicit-submit owners in scope: L1
  (constraint composite) and L10 (geometry scripts), plus L8 (legacy
  position, preview-carrying), L15 (companion placement), and new L20
  (existing-half dialog). L7 resolved as immediate-submit (C).
