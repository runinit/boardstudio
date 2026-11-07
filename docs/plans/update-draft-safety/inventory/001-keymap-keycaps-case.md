# 001 inventory: Keymap / Keycaps / Case draft owners

Scope owned by this note ONLY: `web/crates/keymap/**`, `web/crates/keycaps/**`,
`web/crates/case/**`. Researched 2026-11-07 against the **dirty** automatic
worktree overlay `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378` (commit generation 2,
dirty generation 62). Reuses and rechecks
`docs/research/2026-11-07-019-draft-safety-change-map.md` against live source;
no counts copied blindly — every row below was re-read in this task via native
Gortex `read`/`search`/`relations`. No product file was changed; the only file
write owned by this task is this inventory note. The 001 ticket and map.md were
not edited.

Method note: Gortex text search in this repo does not support alternation
(`a|b` returns nothing), so unbound-draft hunting used the single token
`use_signal` plus targeted symbol searches (`TextDraft`, `NumberDraft`,
`KeycodeField`, `BindingChoice`, `SettingsInput`, `SettingsChoice`,
`CaseNumberField`, `DimensionField`, `SetMountPosition`, `gesture_field`,
`legend_blur_requires_reconciliation`, `Sett
ingsSwitch`, `MethodChoice`,
`KeymapLayerControls`, `SelectedKeySettings`, …). Symbol IDs below are exact
Gortex IDs; paths are repo-relative; line spans are the component/function
bodies as served.

## Census table

| # | Logical owner (component) | Mount / scope / lifetime | Class |
|---|---|---|---|
| K1 | Layer name — `KeymapLayerControls` | Keymap Inspector (`KeymapPanel` → `KeymapLayersSection`); controller `use_layer_operations`, Editor-lifetime, owner = workspace `Keymap` + scope generation | **B** |
| K2 | Binding selects — Behavior / Hold modifier / Layer / Macro (`BindingChoice` in `BindingEditor`) | Same Keymap Inspector mount; per-key `BindingEditor`, keyed by scope/layer/target; no local draft | **C** |
| K3 | Binding keycode/tap combobox — `KeycodeField` | Same mount; local `draft`+`dirty`, container `focusout` admission | **B** (invalid-input path: see unknowns) |
| K4 | Macro name / step keycode text — `TextDraft` | `MacroCard` / `StepEditor`; bound via `use_bound_macro_field` to `MacroPending` (`OwnedEdits`); blur admission | **B** |
| K5 | Macro tap/wait/delay numbers — `NumberDraft` | `MacroCard` (`tap_ms`, `wait_ms`) / `StepEditor` (`StepDelay`); bound to `MacroPending`; blur parses `u32` (empty→0), commits only on success+dirty | **B** (unparseable path: **U**, A-candidate) |
| K6 | Macro step kind select + step/macro add/remove buttons (`StepEditor`, `MacroCard`, `MacroEditor`) | Same mount; `on_value_change`/`onclick` → typed request immediately | **C** |
| K7 | Keymap search / key picker / layer tabs / disclosures / editor toggle (`KeymapPanel`, `EncoderSection` etc.) | Panel-local signals; `EncoderSection`/`Collapsible` `keep_mounted: true` | **E** (excluded) |
| C1 | Keycap legend text — `KeycapsSettingsEditor` legend field | Keycaps Inspector; `legend_draft`+`dirty` (+ controller `KeycapsEditsContext.legend_draft`), bound to `PendingEditSignals` key helper; owner retires on scope/selection/workspace change | **B** |
| C2 | Keycaps immediate settings — board colors, clearance, matrix profile/mount/first-row/wall, key color/profile/mount/row/width/depth, inherit/blank buttons (`SettingsInput` stateless + `SettingsChoice`) | Same mount; `oninput`/`on_change` → `emit_settings_change` immediately; `SettingsInput` holds NO draft signal | **C** |
| C3 | Keycaps retry drafts (`settings_feedback` Retry/Discard alerts) | Same mount; rendered from `retry_drafts` after settlement failure | **D** |
| C4 | Keycaps finding navigation (`keycaps_navigation.rs`) + finding marker | Route effects / pending layout fit / focus queue; no document write | **E** (excluded) |
| S1 | Case body numbers — `CaseNumberField` (thickness, clearance, z, wall h/t, mount x/y/hole/boss/height, gasket inset/width/depth) | Case Inspector (`CaseBodies`); bound via `use_bound_case_field` to `BodyEditPending` (`OwnedEdits`); controller `CaseBodyInspector` Editor-lifetime | **B** |
| S2 | Case body selects/actions — body-type select, mount-kind select, Add body/mount, Remove mount, gasket set, tabs/disclosures | Same mount; select `on_value_change` / button `onclick` → `CaseBodyRequest` immediately; tabs/disclosures are view state | **C** selects+actions / **E** tabs |
| S3 | Mechanical dimensions — `DimensionField` (plate/pcb/bottom/foam thicknesses, wall, clearance, mount/opening/critical-fit/hardware dims, process thickness) | Case Inspector (`MechanicalSettings`); bound via `use_bound_mechanical_field` to `MechanicalSettingsController`; mount `use_mechanical_settings_mount` Editor-lifetime, survives hidden Case | **B** |
| S4 | Mechanical text fields — `TextDraftField` | Same mount/controller as S3; patch-based, blur/Enter commit, Escape restore | **B** |
| S5 | Mechanical select/switch-type controls — method/mount/bottom-style/stabilizer/profile/family pickers (`MechanicalSetupBasics` 3433–3496, `ConstructionControls` 3545–3629, `ManufacturingControls` ~3328, `ProfileGuidance` 3760–3952, `ProfileTargetField` 3652–3678; helpers `method_id` 5277–5284, `parse_method` 5286–5294, `family_id`/`parse_family` 5336–5359) | Same mount/controller staging as S3/S4 (`send_request` → `MechanicalSettingsPatch`); every picker `on_change`/`on_value_change` → `send_request` immediately, no `draft`/`dirty` signal on any commit path | **C** (library-assign two-step staging `selected_family`/`selected_stabilizer` is chooser state, not a document draft — see S5 note) |
| S6 | Case viewer drag placement (mount/gasket `CaseGestureDraft` in `handle_case_gesture`) | `CaseViewer`; transient drag state, preview-only mid-drag, submit on release | **C** commit-on-release / **D** mid-drag preview |
| S7 | Collision-check clearance input (`CaseCollisionChecks`) | Collapsible panel; local signal only, explicit Check buttons, never enters document | **D** |
| S8 | Case assembly-layer display/selection/disclosure; viewer status signals (`gesture_field` message etc.) | View-only / gesture status | **E** |

Counts: **A = 0 confirmed** · **B = 8** (K1, K3, K4, K5, C1, S1, S3, S4) ·
**C = 7 groups** (K2, K6, C2, S2-selects/actions, S5-selects, S6-release, + keymap add/remove-layer actions inside K1's panel) ·
**D = 4** (C3, post-submit pending projections incl. `draft_binding`/`draft_step`/`pending_changes` display, S7, S6-mid-drag) ·
**E** excluded (listed, incl. keycaps `KeycapsKeyList` search query + `KeycapsFitInspector` disclosure, `ProfileGuidance` library-assign chooser staging) · **U = 2** (see unknowns: K5-invalid, K3-focusout).

## Per-owner notes (admission-relevant detail)

### K1 — Layer name (B)
- Source: `ts-boardstudio2/web/crates/keymap/src/keymap/panel.rs::KeymapLayerControls`,
  lines 365–467. Controller:
  `ts-boardstudio2/web/crates/keymap/src/keymap/layer_controller.rs::use_layer_operations`
  with `ts-boardstudio2/web/crates/keymap/src/keymap/layer_controller.rs::LayerEditsContext`
  (line 51), `::LayerPending` (line 38, `OwnedEdits<LayerKey>`),
  `::accepted_layer_name` (line 76).
- Value/baseline: controller-owned `name_draft`/`name_failure` signals; accepted
  baseline from `accepted_layer_name` (accepted document, `base`→`Base` fallback).
  Only the displayed layer's field is bound (`bind_field` on display change,
  unbind old, layer_controller re-read).
- Input: `oninput` sets `name_failure=None`, `name_draft`, `dirty=true`. No
  `Escape` handler on this input (source shows `oninput` + `onblur` only).
- Admission: `onblur` → if `dirty && enabled` → `KeymapLayerOperation::Rename`.
  Test-confirmed:
  `ts-boardstudio2/web/crates/keymap/src/keymap/panel.rs::layer_name_keeps_its_length_limit_and_commits_on_blur`
  (line 1024; also `maxlength 32`).
- Validation: `field_error` (local `name_failure` or controller `Failed`
  feedback) drives `aria_invalid` + `FieldError`. No gate-visible dirty query.
- Pending/save route: `on_operation` → `use_layer_operations` handler →
  `LayerKey::Rename` → `helper.begin_field(…, "keymap-layer", …, resolver,
  &submitted)` → `remember(key)`; feedback `Pending`→terminal via settle/prune
  effect. Post-submission pending is observed (D), not a draft.
- Unmount/hidden: controller is Editor-lifetime; the effect `follow_owner`
  (workspace `Keymap` + scope generation) retires observations when leaving the
  workspace while Session edits continue. `name_draft` lives in the controller
  context, so a hidden panel retains the text; a *displayed-layer change*
  rebinds (unsubmitted text for the old layer is dropped from the field —
  blur fires first on real focus move, so B holds; programmatic layer switch
  without blur is a graph/browser gap, listed in unknowns).

### K2 — Binding selects (C)
- Source:
  `ts-boardstudio2/web/crates/keymap/src/keymap/binding_editor.rs::BindingEditor`
  (465–655), `::BindingChoice` (659–697).
- No `draft`/`dirty` signal: `BindingChoice` memoizes `value` from props;
  `Select`/`Combobox` `on_value_change` → `emit_change` (immediate
  `BindingEditRequest` with bumped `request_sequence`, gated only on `enabled`).
- Baseline: `draft_binding(scope, layer, target, accepted)` projects
  post-submission pending over accepted (`binding_controller.rs::draft_binding`,
  `::BindingKey`, `::BindingPending` line 105, `OwnedEdits`). Invalid/failed
  fields surface via `feedback`/`code_error`/`error` alerts; `aria_invalid`
  only for post-submission failure (`show_error = !dirty && error`), i.e. no
  pre-submission gate mark — fine for C (nothing to gate).
- Unmount/hidden: projection is per-render from controller `OwnedEdits`;
  nothing typed is held locally, so hiding loses nothing.

### K3 — KeycodeField (B, invalid path U)
- Source:
  `ts-boardstudio2/web/crates/keymap/src/keymap/binding_editor.rs::KeycodeField`
  (715–785).
- Local `draft` (init `accepted_value`) + `dirty`; `oninput` and combobox
  `on_value_change` set both. Effect re-syncs `draft := accepted` only when
  `!dirty && !pending` — a dirty draft survives accepted churn (newer-wins).
- Admission: wrapping `div onfocusout` → `with_keycode(...)` conversion; if
  `dirty` and conversion succeeds → `dirty=false`, `emit_change`. Empty/unknown
  text that `with_keycode` rejects leaves the draft dirty with NO commit and NO
  `aria_invalid` (`show_error = !dirty() && props.error.is_some()`).
- Classification rationale: valid path is blur-equivalent (B); the rejected
  path is A-shaped (unsubmitted + gate-invisible) but unconfirmed in a browser.
  Map.md already flags focusout/portal/conversion as needing a mounted trace —
  recheck, do not reclassify to A without one. Browser-dependent: combobox
  popup focus transitions may or may not fire container `focusout`.

### K4 — Macro TextDraft: name + step keycode (B)
- Source:
  `ts-boardstudio2/web/crates/keymap/src/keymap/macro_editor.rs::TextDraft`
  (521–566). Bound:
  `ts-boardstudio2/web/crates/keymap/src/keymap/macro_controller.rs::use_bound_macro_field`
  (line 59) into `::MacroPending` (line 51, `OwnedEdits<MacroKey>`).
- `oninput` clears `failure`, sets `draft`+`dirty`. `onblur` commits when
  `dirty || commit_if_unchanged`. No `Escape`/restore handler on this control.
- Mounts: macro name (`MacroCard`, line 320 `NumberDraft` siblings context —
  name uses `TextDraft`), step keycode (`StepEditor`, 383–498,
  `trim_on_commit: true`, `commit_if_unchanged =
  step_keycode_requires_normalization(&accepted_step)` so blurring an unchanged
  legacy non-`KeyPress` binding normalizes it — source-confirmed at StepEditor).
  Macro-name `trim_on_commit` RESOLVED this task: the `MacroCard` name instance
  (`macro_editor.rs::MacroCard`, 245–364; `TextDraft` instantiation ~314–318) does
  NOT pass `trim_on_commit` or `commit_if_unchanged` → both default `false`
  (`TextDraftProps`, 501–518, `#[props(default = false)]`). `TextDraft` (521–566)
  has NO `Escape`/key handler — `oninput` + `onblur` only. With `trim=false`,
  blur commits the raw value when `dirty`; clean blur is a no-op for the name
  instance. Contrast the step-keycode instance (`StepEditor`, 467ff) which passes
  `trim_on_commit: true` + conditional `commit_if_unchanged`. Classification stays
  **B**; unknown U-4 (macro trim/Escape) is closed.
- Validation: `failure` (controller settlement failure) → `aria_invalid` +
  `FieldError`. Mounted test
  `unchanged_valid_step_keycode_blur_does_not_submit_a_macro_edit` proves clean
  blur is a no-op (no spurious submission).
- Pending route: `on_commit` → typed `MacroEditRequest` → controller admission
  (`use_macro_operations`), `field_feedback`/`action_pending` projection;
  `draft_step` overlays post-submit pending (D).

### K5 — Macro NumberDraft: tap_ms / wait_ms / step delay (B, invalid path U)
- Source:
  `ts-boardstudio2/web/crates/keymap/src/keymap/macro_editor.rs::NumberDraft`
  (583–623). Re-verified verbatim in this task (disk-verified read).
- `oninput` clears failure, sets string `draft`+`dirty`. `onblur`: empty→0,
  else `raw.parse::<u32>().ok()`; commits ONLY on `Some && dirty`.
  `aria_invalid = message.is_some()` where message is the bound helper failure —
  a locally unparseable string sets NO failure and NO marker while staying
  dirty. Matches map.md; kept B (valid path commits on blur) with the invalid
  path as an A-candidate pending browser proof: native `type=number` inputs
  sanitize keystrokes, so unparseable retention may be untypable in practice
  (U-3).

### K6 — Step kind + add/remove actions (C)
- `StepEditor` kind `Select on_value_change` → `MacroEditRequest(StepKind)`
  immediately (macro_editor.rs 383–498). Add/Remove step, Add/Remove macro
  buttons → typed requests on click (MacroEditor/MacroCard/StepEditor).
- `action_pending` guards double-submit; no typed draft anywhere on these
  paths. Post-submit pending display only (D).

### K7 — Keymap non-draft UI (E, excluded with retention note)
- `KeymapPanel`: `query` search signal (filters key list only), `selectedEditor`
  toggle, `keys/macros/encoders_section_open`, key-picker combobox
  (`on_select_key` selection, not a project edit), layer tabs/roving focus.
- Retention note for the safety argument: `EncoderSection`
  (`encoder_editor.rs::EncoderSection`, line 167) and panel `Collapsible`s use
  `keep_mounted: true` — collapsed sections RETAIN mounted `BindingEditor`
  drafts (K2/K3 state stays live). This supports, not weakens, the census:
  hidden ≠ discarded here.

### C1 — Keycap legend (B)
- Source:
  `ts-boardstudio2/web/crates/keycaps/src/keycaps_settings.rs::KeycapsSettingsEditor`
  (1610–1845); helpers `::legend_change_required` (1893),
  `::legend_blur_requires_reconciliation` (1897).
- Local `legend_draft` (+ controller-provided draft via `KeycapsEditsContext`
  when mounted under the controller), `legend_dirty`; `oninput` sets dirty.
  `bind_legend` binds draft/failure to the `PendingEditSignals` key helper
  (`KeycapsEdits::bind_legend`, `KeycapsPendingKey::bound(Key, Legend)`).
- Admission: `onblur` commits when `dirty || reconciliation-needed`
  (change vs accepted OR retry pending). `maxlength 12`. Mounted tests
  `type_legend` + `input.blur()` emit the edit (test helpers at lines
  2692/2791). Owner-scoped: effect resets draft on legend-owner change
  (editor/instance/scope/selection/key); `follow_owner` replaces helpers on
  owner change, silently retiring observations while Session edits continue.
- Failed legend edits become `KeycapsRetryDraft` + inline `legend_failure`
  (`aria_invalid`) → post-submit (D), with explicit Retry/Discard.

### C2 — Keycaps immediate settings (C)
- `KeycapsBoardSettingsEditor` (1464–1510): color inputs `oninput` →
  `emit_settings_change` per keystroke; clearance number parses-guarded
  `oninput` → immediate. `KeycapsMatrixSettingsEditor` (1521–1580): profile /
  socket `SettingsChoice on_change` → immediate; first-row / wall `oninput`
  parse-guarded → immediate. Selected-key editor (in C1 body): color/profile/
  mount/row/width/depth same pattern.
- `SettingsInput` (1435–1455) is stateless: NO `use_signal` draft, `value`
  from props, pure `oninput` forwarder. Nothing to retain across blur/hide.
  Parse-guard detail: unparseable keystrokes are dropped (no draft, no marker)
  — acceptable for C since the accepted value visibly never changes; still,
  rapid Law-of-demonstration caveat: a dropped keystroke looks like lost input
  but is not a document draft (excluded by definition, noted for UX only).

### C3 — Keycaps retry drafts (D)
- `settings_feedback` renders `KeycapsRetryDraft`s (failed post-submission)
  with Retry/Discard. Explicitly post-submission pending, not a live draft;
  counted for the denominator so a gate aggregator does not double-count them
  as unsubmitted work. `draft_owner_mismatch` / `request_owner_is_current` /
  `retry_identity_is_current` scope them to the live owner.

### C4 — Keycaps navigation (E, excluded)
- `keycaps_navigation.rs`: `NavigationOwner`, `route_effects`,
  `use_pending_layout_fit`, `queue_owner_focus` — workspace routing + camera
  fit + focus scheduling. No project-document write path; finding marker
  publication follows the admitted route. Navigation/selection state only.

### S1 — Case body numbers (B)
- Source: `ts-boardstudio2/web/crates/case/src/case_bodies.rs::CaseNumberField`
  (677–818). Controller:
  `ts-boardstudio2/web/crates/case/src/case_controller.rs::use_bound_case_field`
  (line 112), `::BodyEditPending` (line 47, `OwnedEdits<BodyEditKey>`),
  `::CaseBodyInspector` (Editor-lifetime; body-edit portal dispatch).
- Local `draft`/`error`/`dirty` + bound `field_failure`; effect follows
  accepted while `!pending` and restores canonical text appropriately.
- Admission: `commit()` on `blur` AND on `Enter` (`skip_enter_blur` prevents
  double admission); `Escape` restores accepted text + clears dirty/error.
  Rule validation (`Finite`/`Nonnegative`/`Positive`) → local `error` +
  `aria_invalid` + `aria_describedby`; mount test
  `thickness_rejects_invalid_draft_with_described_error` proves the invalid
  draft is owned AND marked (contrast with K5).
- Save route: `on_commit(CaseNumberCommit)` → `CaseBodyRequest` with stable
  `field_id` → controller `submit_body_edit` → helper observation; settlement
  tests prove: held edit survives body-tab switch (`…keeps_its_held_edit`),
  one Enter+blur admits once (`repeated_enter_and_native_blur…`), newer draft
  survives older failure.
- Unmount: field signals release on unmount (`use_drop` unbind + fields entry
  removal). Tab switch moves focus → blur commits first in real interaction;
  programmatic unmount without blur is a gap (see unknowns).

### S2 — Case body selects/actions (C) + tabs (E)
- Body-type `Select` → `SetKind` on change (test
  `body_type_select_uses_custom_trigger_and_commits_kind`); mount-kind select
  likewise; `+ New case body` / Add/Remove mount / gasket buttons emit
  `CaseBodyEdit` actions immediately (`emit_edit`/`emit_edit_with_field`,
  one-shot `action_id` field ids for feedback correlation only).
- Body tabs + mounting/gasket disclosures (`CaseBodyDisclosureState`) are
  selection/view state (E); tab switch swaps the active editor (retention
  covered in S1).

### S3 — Mechanical dimensions (B)
- Source:
  `ts-boardstudio2/web/crates/case/src/mechanical_settings.rs::DimensionField`
  (4766–4978). Bound:
  `ts-boardstudio2/web/crates/case/src/mechanical_settings_controller.rs::use_bound_mechanical_field`
  (line 105) → `MechanicalSettingsController` staging
  (`::request_field` lines 219–235, `::bind_field` line 182).
- Same admission shape as S1: `oninput` sets draft+dirty; `onblur` commits;
  `Enter` blurs (native commit path); `Escape` restores accepted. Rule errors
  (`NumberRule`) → local error + `aria_invalid`. Covers all
  `MechanicalDimension` patch targets (support/mount/profile-volume/opening/
  critical-fit/hardware/process).
- Lifetime: `use_mechanical_settings_mount` (mechanical_settings_mount.rs) is
  explicitly Editor-lifetime and keeps the controller + sequence alive even
  when Case/workspace/panel is hidden — hidden retains pending deliberately.

### S4 — Mechanical text fields (B)
- Source: `.../mechanical_settings.rs::TextDraftField` (4995–5159).
- `draft`/`dirty` + local `error` + bound `failure`; `intent.patch(value)`
  parse step; `onblur` commits, `Enter` blurs-then-commits, `Escape` restores
  accepted. Multiline variant commits on blur only (no Enter-blur). Same
  controller staging as S3.

### S5 — Mechanical select/switch controls (C — read this task)
- Sources (all `ts-boardstudio2/web/crates/case/src/mechanical_settings.rs`):
  `MechanicalSetupBasics` (3433–3496: Mounting-style + Case-process `SettingsSelect`
  → `send_request(SetMount)` / `send_request(SetMethod)` on change); `ConstructionControls`
  (3545–3629: Plate-method / Mount-style / Bottom-construction `SettingsSelect` →
  `send_request(SetMethod/SetMount/SetBottomStyle)`; stabilizer `SettingsSelect` →
  `send_request(SetStabilizerKind)`); `ManufacturingControls` (~3300–3350, `method_id`
  use at 3328: per-part process-method `SettingsSelect` →
  `send_request(SetProcessMethod)`); `ProfileGuidance` (3760–3952: per-profile
  family `SettingsSelect` → `send_request(SetSwitchFamily)`; paired library-assign
  selects → `send_request(AssignSwitchProfile/AssignStabilizerProfile/
  AssignImportedGeometryProfile)`); `ProfileTargetField` (3652–3678: ≤12 options
  → `SettingsSelect`, else `Combobox` with constant `selected = use_memo(None)` and
  `on_value_change` → immediate `send_request`); helpers `method_id` (5277–5284),
  `parse_method` (5286–5294), `family_id`/`family_label`/`parse_family` (5336–5359).
- No `draft`/`dirty`/`use_signal` text-draft state on any commit path; `value` comes
  from accepted props. Chooser staging only: `ProfileGuidance` holds local
  `selected_family`/`selected_stabilizer` (`use_signal(String::new)`, ~3763–3764)
  recording the first half of a two-step library assign; they gate the paired
  `ProfileTargetField` (`disabled … || selected.is_empty()`) and never enter the
  document themselves — chooser/view state (E-adjacent), not a document draft.
- Combobox text-sub-input check: the only `Combobox` in this file is inside
  `ProfileTargetField`; its `value` prop is a constant-`None` memo and the change
  commits immediately on select. No `Combobox` holds a document draft in scope
  (inner popup text state, if any, lives in the shared UI crate, out of scope).
  S5 = **C**. Unknown S5-bodies item is closed; no new Class A.

### S6 — Viewer drag placement (C on release / D mid-drag)
- Source:
  `ts-boardstudio2/web/crates/case/src/case_viewer.rs::handle_case_gesture`
  (913–1274); signals `CaseViewer` `gesture_field`/`gesture_message`
  (lines 395–397).
- `Start` captures `CaseGestureDraft::{Gasket,Mount}` (before+pending, no
  document write). `Move` updates in-memory pending + viewer preview handles
  and (optionally) a transient `update_case_gesture_preview` document — D.
  `End` submits exactly once: mechanical patch via `submit_settings_patch`
  or authored-body edit via `body_edit_portal.dispatch`; unchanged move
  submits nothing (`supports_equal_positions` / at-equality checks);
  `identity_current` false → drop without submit. `Cancel` clears.
- No typed-text draft exists on this path; pointer-held, not focus-held. The
  `gesture_field`/`gesture_message` signals are status text (E-adjacent, kept
  under S6 for completeness).

### S7 — Collision clearance (D)
- Source:
  `ts-boardstudio2/web/crates/case/src/case_collision.rs::CaseCollisionChecks`
  (35–190). `clearance` is a LOCAL check parameter: `oninput` cancels running
  checks + resets state; only explicit `Check collisions` / `Check at
  0.01 mm` buttons consume the parsed threshold. Never enters the document;
  invalid text disables the buttons via `threshold.is_none()`. Explicit-submit
  *check*, not a project draft → D.

### S8 — Case view-only/navigation state (E)
- `case_assembly_layers.rs`: `open`/`display`/`selected_layer` signals —
  disclosure/display/selection only. `direct_gesture_message`,
  `CaseKeycapsState`, preview handles — status/preview. `cad_presentation.rs`
  host signals seen in `use_signal` sweep are test-harness signals.

## Coverage list (50 files enumerated; what was searched, what was not)

Files-scale denominator: keymap 18 + keycaps 8 + case 24 = **50 files**
(Gortex `files` enumeration, exact worktree view).

Searched with a draft owner FOUND (this note): `keymap/macro_editor.rs`,
`keymap/binding_editor.rs`, `keymap/binding_controller.rs`,
`keymap/layer_controller.rs`, `keymap/macro_controller.rs`,
`keymap/panel.rs`, `keymap/encoder_editor.rs`, `keycaps/keycaps_settings.rs`,
`keycaps/keycaps_navigation.rs` (E), `case/case_controller.rs`,
`case/case_bodies.rs`, `case/mechanical_settings.rs`,
`case/mechanical_settings_controller.rs` (partial: `request_field`,
`bind_field` regions), `case/mechanical_settings_mount.rs`,
`case/case_collision.rs` (D), `case/case_viewer.rs` (S6 regions),
`keymap/layer_edit.rs` + `keymap/macro_edit.rs` (resolvers only — no UI
draft), `keymap/lib.rs`, `case/lib.rs` (module manifests only),
`presentation/keymap_workspace.rs` + `presentation/keycaps_workspace.rs`(files
enum) + `presentation/case_workspace.rs` (files enum; mounts cited for K-mount
evidence: `keymap_workspace.rs::inspector` mounts `BindingEditor`,
`MacroEditor`, `EncoderEditor`, `KeymapPanel`).

Searched with NO draft owner found (source-confirmed, citeable as no-owner):
- `keymap/canvas.rs`, `keymap/view.rs`, `keymap/view_controls.rs`
  (`FitCamera` math only), `keymap/encoder_inputs.rs` (`choice()` identity
  helper — no signals), `keymap/queued_edit_tests.rs`,
  `keymap/macro_accessible_names.rs` (label strings), keymap `Cargo.toml`;
- keycaps `Cargo.toml`, `keycaps_finding_marker.rs` (no owner — `FocusedFindingMarker`
  53–90 pure render, `use_retire_stale_finding` 24–50 stale-identity retire; E, extends
  C4), `keycaps_fit.rs` (no owner — `use_keycaps_fit` 184–242 async preview
  bookkeeping, `KeycapsFitInspector` 245–348 disclosure-only signals; E),
  `keycaps_scene.rs` (no owner — `KeycapsKeyList` 376–442 search `query` filter +
  picker selection = E like K7, `KeycapsCanvas` 279–362 immediate select; E),
  `keycaps_settings.rs` test pages (excluded as tests), `keycaps/lib.rs`
  (module manifest only — 6 module decls, no components/signals);
- case `Cargo.toml`, `case_display.rs`, `case_generation_admission.rs`,
  `case_preview_lifecycle.rs`, `case_selection.rs`, `shared_viewer.rs`,
  `mechanical_feedback.rs` (only `field_feedback` symbol seen),
  `mechanical_pair.rs`, `cad_presentation.rs` (only test-host signals seen in
  sweep), `case_assembly_layers.rs` (E signals seen), `test_contexts.rs`,
  `mechanical_session_tests.rs`, `shared_viewer_020_tests.rs`,
  `mechanical_settings_migration_tests.rs`, `examples/settings.rs`
  (example/test fixture only).

Remaining graph gaps (enumerated but NOT opened — must be covered by a follow-up,
not assumed clean): case `case_display.rs`,
`case_generation_admission.rs`, `case_preview_lifecycle.rs`,
`case_selection.rs`, `shared_viewer.rs`, `mechanical_feedback.rs`,
`mechanical_pair.rs`, `mechanical_settings.rs` S5-adjacent remaining regions
(`ManufacturingControls` outside the process-method select,
`ProfileGeometryEditor`/`DimensionControls` bodies — S5 pickers themselves read),
`case_collision.rs` below line 190, `case_viewer.rs` outside the gesture
region, `case_controller.rs` outside the bind/submit regions,
`mechanical_settings_controller.rs` outside `request_field`/`bind_field`,
`panel.rs` `MacroCard` name-field props (trim flag), `binding_controller.rs`
submit path (only projection/resolver regions re-read),
`macro_controller.rs` submit path (only bind/project regions re-read).
Closed by this task (007): S5 select/switch bodies → C; `MacroCard` name-field
props (trim flag = false, no Escape) → K4 stays B; keycaps `lib.rs` /
`keycaps_scene.rs` / `keycaps_fit.rs` / `keycaps_finding_marker.rs` → no owner (E).
Rationale for proceeding anyway: the repo-wide `use_signal` sweep surfaced no
other `draft`-shaped signals in scope besides the owners above and test
fixtures; the gaps are bounded to named regions.

## Explicit unknowns (U + browser-dependent)

1. **NumberDraft unparseable path (K5).** Source proves a dirty, marker-less,
   uncommitted state is reachable *if* unparseable text can sit in the input.
   Native `type=number` sanitization may make it untypable — browser proof
   required before A-classification or dismissal.
2. **KeycodeField focusout topology (K3).** Container-`div onfocusout` vs
   combobox popup portal focus moves: does selecting/cancelling always fire
   the container handler exactly once? Mounted trace required (map.md agrees).
3. **Macro-name trim/Escape (K4) — CLOSED this task.** `MacroCard` name instance
   (`macro_editor.rs::MacroCard` 245–364, instantiation ~314–318) passes neither
   `trim_on_commit` nor `commit_if_unchanged` → both default `false`
   (`TextDraftProps` 501–518); `TextDraft` (521–566) has `oninput` + `onblur` only,
   no `Escape` handler. Blur commits raw value when dirty. Stays **B**.
4. **Mechanical select/switch bodies (S5) — CLOSED this task.** All picker bodies
   read (`MechanicalSetupBasics` 3433–3496, `ConstructionControls` 3545–3629,
   `ManufacturingControls` ~3328, `ProfileGuidance` 3760–3952, `ProfileTargetField`
   3652–3678; helpers 5277–5359): every `on_change` → immediate `send_request`,
   no draft signals; the only `Combobox` uses a constant-`None` value with
   immediate commit; `selected_family`/`selected_stabilizer` are chooser staging
   (E-adjacent), not document drafts. **C**.
5. **Programmatic unmount without blur (K1/S1).** Layer switch, body-tab
   switch, or workspace switch driven without a focus move: does Dioxus fire
   `onblur` on unmount? If not, an unsubmitted dirty draft is silently dropped
   (data loss of a different kind — not a stale-gate issue). Real-mount check
   needed; tab-switch tests only cover submitted (held) edits.
6. **Gortex `callers` gap.** As in map.md, `relations(callers)` for editor
   components returned inferred/empty edges; production mounts cited here are
   source-read mounts (`keymap_workspace.rs::inspector`, controller
   owner-scope comments), not graph-proven call chains.

## Validation (none) / product unchanged

- No product source, ticket, map, ADR, baseline, or historical log was
  changed. No test, build, mutation, or browser run was executed.
- `change(detect/tests/guards/contract)` were NOT run (artifact-scope checks
  on this worktree would also enumerate the pre-existing 23 dirty files and
  prove nothing about this note).
- Dirty-worktree caveat (inherited from map.md): all reads were disk-verified
  against overlay `01a1b3cf` at dirty generation 62; later reindexing may
  shift line spans — symbol IDs are the stable citation.
