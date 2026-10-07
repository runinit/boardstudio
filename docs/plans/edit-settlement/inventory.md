# Edit-submission inventory: what changes Session documents and which edits can overwrite each other

Status: reference (taken at 3368825; line numbers drift)
Source: inventory for [the edit settlement plan](map.md).


This covers every user action in `web` that sends a document edit (`Event::Edit`) to the Session, checked against the current tree (HEAD `3368825`, after the crate split) with `grep -rn "Event::Edit" web --include=*.rs`. It was read-only: I edited nothing and ran no builds.

**Bottom line:**
- There are 73 actions. 57 send a whole document or a whole entity copied from an earlier snapshot ("ABS"); 16 send only the change ("FIELD").
- 13 actions have no guard against a second submission; 11 of those are ABS, so a second queued edit silently replaces the first.
- Every per-panel guard only blocks a second edit from the *same* panel. The only thing that stops an edit queued behind another panel's edit is an admission check that the session is idle (`lifecycle == Ready`, usually plus "saved"); I call this the idle gate.
- No test anywhere queues two edits back-to-back against a real Session.

**How to read the tables:**
- Crate paths below are relative to `web/crates/<crate>/src/`; root rows are in `web/src/`.
- **Guard: yes** means the panel refuses a new submit while its own previous edit is pending. **partial** means it only has the idle gate, only checks at drag start, or only disables a button.
- **Settle** names the pending record and how the panel decides the edit actually landed.
- **Std-landed** is the shared check: token changed, revision went up, session `Ready` and `Saved` at the new revision.
- In the Settle column, "whole-document equality" means the accepted document equals the submitted one with the revision field ignored.
- Every location was re-checked after the split. Most files moved into crates kept their old line numbers. The exceptions are `web/src/presentation.rs`, `layout/src/part_placement.rs` (about 14 lines earlier), and board reference, now in `pcb/src/board_reference_owner.rs`.

## 1. Edit actions

### Root `web/src` (8 actions)
| # | Action | Submit location | Operation | Payload | Guard | Settle |
|---|---|---|---|---|---|---|
|1|Layout Component Inspector SetPosition X/Y (blur/Enter)|`presentation.rs:2235` → helper `submit_layout_component_edit` :2121; its owner check (:2078) has no lifecycle check|MoveParts|ABS|**no**|none|
|2|Layout Component Inspector AssignLayout|`presentation.rs:2276`|ReplaceDocument|ABS|**no**|none|
|3|Layout Component Inspector SetOutline (margin etc.)|`presentation.rs:2295`|ReplaceDocument|ABS|**no**|none|
|4|Layout Component Inspector SetConstraint|`presentation.rs:2350`|SetConstraint|FIELD|no|none|
|5|Layout Component Inspector RemoveConstraint|`presentation.rs:2366`|RemoveConstraint|FIELD|no|none|
|6|Old position Inspector (live preview while typing, then Enter/Apply)|`presentation/inspector.rs:171`|MoveParts|ABS|**no**|none|
|7|Object-tree keyboard nudge|`presentation.rs:4776`|MoveParts|ABS|**no**|none|
|8|Setup-guide project rename|`presentation.rs:8243` (copies the document as it was when the panel rendered)|ReplaceDocument|ABS|**no**|none|

### `layout` crate (27 actions)
| # | Action | Submit location | Operation | Payload | Guard | Settle |
|---|---|---|---|---|---|---|
|9|Canvas Align|`objects/layout_align_controller.rs:357`|SetMatrix / MoveParts|ABS|yes :218, idle gate|`PendingAlign` :21; `pending_settlement_gate` (`objects/layout_align_geometry.rs:38`) then `expected_applied` :1017|
|10|Component placement commit|`part_placement.rs:1217`|ReplaceDocument|ABS|yes :1163, idle gate|`PendingCommit` :176; `completion_is_accepted` :1909 (revision = base+1)|
|11|Apply component to the selected key|`part_placement.rs:748`|SetMatrix|ABS|yes :587, idle gate|`PendingKeyEdit` :183; outcome only|
|12|Matrix setup create|`objects/matrix_setup_controller.rs:331`|SetMatrix|ABS (new matrix)|yes :158, idle gate|`PendingSetup` :21; settle at :455|
|13|Matrix placement|`objects/matrix_placement_controller.rs:498`|SetMatrix|ABS (new matrix)|yes :444, idle gate|`PendingPlacement` :22|
|14|Matrix Inspector fields (name, rows, columns, pitch, switch, diode, edge gap, add row/column)|`objects/matrix_inspector_controller.rs:365`|SetMatrix / SetLayout|ABS|yes :219, idle gate|`PendingMatrixEdit` :25; settle at :1327 (Std-landed + the field value matches)|
|15|Matrix preset|same file :570|SetMatrix|ABS|yes :396|`PendingMatrixPreset` :34; settle at :1418|
|16|Matrix delete|same file :658|RemoveMatrix|FIELD|yes :602|`PendingMatrixDelete` :44; settle at :1507|
|17|Unlink mirrored halves|same file :925|SetLayout|ABS|yes :865|none|
|18|Duplicate design variant|same file :2147|SetMatrix|ABS|yes :684|`wait_for_variant_edit` :2242|
|19|Matrix Transform Inspector|`objects/matrix_transform_controller.rs:465`|SetMatrix (ABS) and SetMatrixSplay (FIELD)|mixed|yes :251|`PendingTransformEdit` :24; settle at :799|
|20|Transform tool drag (preview, then commit)|`objects/layout_transform_toolbar.rs:1119`|SetMatrix / SetMatrixSplay|ABS|partial: idle gate at drag start only (:845)|none|
|21|Transform handle keyboard nudge|`objects/layout_transform_toolbar.rs:682`|SetMatrix (matrix copied when the panel rendered) / SetMatrixSplay|ABS|**no**|none|
|22|Mirrored pair create|`objects/mirrored_pair_controller.rs:704`|CreateMirroredPair|FIELD|yes :644, idle gate|`PendingPair` :23; transaction-id match (`mirrored_pair_lifecycle.rs:46`)|
|23|Existing-half mirror|`objects/existing_half.rs:258`|ReplaceDocument|ABS|yes :181|`PendingExistingHalf` :57 (revision = base+1)|
|24|Add board|`objects/board_setup_controller.rs:159`|ReplaceDocument|ABS|yes :103|`PendingBoard` :28 (revision = base+1)|
|25|Keycap size|`objects/keycap_size_controller.rs:272`|ReplaceDocument|ABS|yes :171|`PendingResize` :81; settle at :575|
|26|Board rename|`board_inspector.rs:141`|ReplaceDocument|ABS|partial: `Ready` + owner token check only|none|
|27–35|Outline actions, all through `outline_lifecycle.rs:2071` (helper `submit_action` :1339): Activate, Copy, Delete, AddFeature, AddConnection, SetFeature, RemoveFeature, settings Update (`outline_settings.rs:118`), EditPerimeter (its preview edits at :2044 are not tracked)|see left|SelectOutline, CopyOutline, RemoveOutline, ReplaceDocument, SetOutline, RenameOutline|Activate, Copy and Delete are FIELD; the other six are ABS|yes :1346|`Pending` :325; settle effect at :795 (Std-landed + a per-action content check)|

### `keymap` crate (3) and `keycaps` crate (1)
| # | Action | Submit location | Operation | Payload | Guard | Settle |
|---|---|---|---|---|---|---|
|36|Macro add/edit/remove|`keymap/macro_controller.rs:344`|EditKeymap|FIELD|yes :284, idle gate|`PendingMacroEdit` :40; `intent_applied` :775|
|37|Key/encoder binding edit|`keymap/binding_controller.rs:924`|EditKeymap|FIELD|yes :825, idle gate|`PendingBindingEdit` :82|
|38|Layer add/rename/remove|`keymap/layer_controller.rs:492`|EditKeymap|FIELD|yes :292, idle gate|`PendingLayerEdit` :86; `is_applied_to` :48|
|39|Keycap settings (board, matrix, key)|`keycaps_settings.rs:1340` (keycaps crate)|SetKeycapBoard / SetMatrixKeycaps / SetKeycapKey|FIELD|yes :471|`PendingEdit` :173; `change_is_applied` :1127|

### `pcb` crate (14 actions)
| # | Action | Submit location | Operation | Payload | Guard | Settle |
|---|---|---|---|---|---|---|
|40|Board reference actions|`board_reference_owner.rs:128` (owner check :13 requires `Ready` + `Saved`)|ReplaceDocument|ABS|partial|observed, but the result is discarded|
|41|Wiring mode|`pcb_wiring/mode.rs:187`|ReplaceDocument|ABS|yes :145|`PendingModeEdit` :48; whole-document equality|
|42|Pin lock|`pcb_wiring/pins.rs:204`|ReplaceDocument|ABS|yes :152|`PendingPinEdit` :56; whole-document equality|
|43|Apply wiring plan|`pcb_wiring/apply.rs:174`|ReplaceDocument|ABS|yes :141|`PendingApply` :38; whole-document equality|
|44|Release reviewed connections|`pcb_wiring/apply.rs:218`|ReplaceDocument|ABS|partial|none|
|45|Firmware position|`pcb_wiring/controller.rs:238`|SetKeyBinding|FIELD|partial: no pending check, idle gate only|`PendingFirmwarePositionEdit` :27 → `settle_edit` (`runtime/src/firmware_position_projection.rs:135`)|
|46|Part net assign/create|`pcb_wiring/controller.rs:524`|ReplaceDocument|ABS|yes :360|`PendingPartNetEdit` :277; whole-document equality|
|47|Part scan mode|`pcb_wiring/part_input_settings/owner.rs:259`|SetInputScanMode|FIELD|yes :190|`PendingEdit` :65|
|48|Part generator parameter|same location|ReplaceDocument|ABS|yes :190|`PendingEdit` :65|
|49|Physical setup|`pcb_physical_setup/controller.rs:329`|ReplaceDocument|ABS|yes :213|`SubmittedSetup` :152 → `accepted_matches_proposal` (`catalogue/src/physical_setup.rs:28`)|
|50|Module save placement|`pcb_module_inspector.rs:226`|SetMountedModule|ABS|yes :125|outcome only|
|51|Module remove|`pcb_module_inspector.rs:300`|RemoveMountedModule|FIELD|yes :282|outcome only|
|52|Embed module circuit|`pcb_module_inspector.rs:470`|EmbedModuleCircuit|FIELD|partial (`Ready` only)|none|
|53|Remove embedded circuit|`pcb_module_inspector.rs:829`|RemoveEmbeddedCircuit|FIELD|partial (`Ready` only)|none|

### `case`, `ui-shared` and `library` crates (5 actions)
| # | Action | Submit location | Operation | Payload | Guard | Settle |
|---|---|---|---|---|---|---|
|54|Case body edits and mount drag|`case/src/case_controller.rs:470`|SetCase|ABS (whole body)|yes :367, idle gate|`PendingBodyEdit` :15|
|55|Mechanical settings|`case/src/mechanical_settings_mount.rs:180`|ReplaceDocument|ABS|yes|`expected_matches` (`case/src/mechanical_settings_controller.rs:893`)|
|56|Geometry scripts: New|`ui-shared/src/geometry_scripts.rs:122` → :275|ReplaceDocument|ABS|partial (`Ready` only)|none|
|57|Geometry scripts: Apply|`ui-shared/src/geometry_scripts.rs:214` → :275|ReplaceDocument|ABS|partial (`Ready` only)|none|
|58|Project rename (library menu)|`library/src/library.rs:990`|ReplaceDocument|ABS|**no**|none|

### `parts` crate (15 actions)
| # | Action | Submit location | Operation | Payload | Guard | Settle |
|---|---|---|---|---|---|---|
|59|New custom component|`parts_new_component.rs:136`|ReplaceDocument|ABS|yes :94|`PendingCreate` :17; `reconciliation_is_current` :318|
|60|Custom definition fields (each commits on blur)|`parts_custom_definition.rs:~139` → :881|ReplaceDocument|ABS|**no**|none|
|61|Definition name (on blur)|`parts_definition_name.rs:103` → :226|ReplaceDocument|ABS|**no**|none|
|62|Import KiCad footprint|`parts_import_footprint.rs:701`|ReplaceDocument|ABS|yes :423|`PendingImport` :228; `accepted_import_is_current` :151|
|63–65|Component model: transform / remove / upload|`parts/component_model_editor.rs:249` / :308 / :658|ReplaceDocument|ABS|yes :686 / :732 / :471|`PendingModelEdit` :37; outcome only|
|66|Generator Apply|`parts/generator_settings.rs:1235`|ReplaceDocument|ABS|partial (button disabled only)|`PendingApply` :68|
|67|Generator asset upload|`parts/generator_settings.rs:1180`|ReplaceDocument|ABS|yes :956|`PendingApply` :68|
|68|Mechanical profile save|`parts/mechanical_profile_ui.rs:130`|ReplaceDocument|ABS|**no**|`PendingProfileEdit` :207; outcome only|
|69|Module profile save|`parts/module_profile_editor.rs:593`|SetModuleDefinition|ABS|yes :553|outcome only|
|70|Module attach|`parts/modules_catalogue/module_attachment.rs:254`|SetMountedModule|ABS|yes :88|revision = base+1|
|71–73|Assembly: save / place on board / apply to matrix|`parts/assembly_editor.rs:466` / :712 / :853|ReplaceDocument, ReplaceDocument, SetMatrix|ABS|yes :421 / :543 / :740|`PendingSave` :28|

## 2. Other events
- **Gestures:** GestureBegin `web/src/presentation.rs:6130`, GestureEnd `:6322`. The Session treats a gesture commit as strict: it is rejected if the revision moved.
- **ExportCommit:** `web/crates/runtime/src/runtime.rs:5257`. Strict.
- **ReviewElectricalRemap:** `web/crates/pcb/src/pcb_wiring/remap.rs:177`. Strict and guarded.
- **Undo/Redo:** `web/src/presentation/workbench_shortcuts.rs:24/27/30` and `presentation.rs:6587/6591/8880/8881`. Not guarded and not observed.

## 3. Totals and grouping
- **73 actions by crate:** layout 27, parts 15, pcb 14, root `web/src` 8, keymap 3, case 2, ui-shared 2, keycaps 1, library 1.
- **Payload:** 57 ABS (two of them mixed, counted as ABS) and 16 FIELD.
- **Guard:** 45 yes, 15 partial, 13 none. 11 of the 13 unguarded actions are ABS.
- **ReplaceDocument:** 36 actions use it.
- **Grouping for tickets:**
  - Layout: #1–11 and #56–57.
  - Objects/Matrix: #12–25.
  - Outline: #27–35.
  - Keymap: #36–38.
  - Keycaps: #39.
  - PCB wiring, modules and physical setup: #40–53.
  - Mechanical/Case: #54–55.
  - Parts, library and definitions: #59–73.
  - Project and board metadata: #8, #26 and #58.

## 4. Top 10 risks (unguarded ABS)
1. **#60 Custom definition fields.** Tabbing from courtyard width to height sends two whole-document replacements built from the same snapshot; the height commit silently reverts the width.
2. **#1 Layout Component Inspector SetPosition.** The Y commit carries the old X as an absolute position, so a just-committed X edit is lost.
3. **#2 and #3 Layout Component Inspector AssignLayout / SetOutline.** A whole-document replacement queued behind any other edit undoes it.
4. **#21 Transform handle keyboard nudge.** Holding an arrow key repeats a full-matrix replacement copied when the panel rendered.
5. **#7 Tree keyboard nudge.** Every repeat is computed from the same accepted position, so nudges are lost.
6. **#6 Old position Inspector.** Sends a live preview on every keystroke plus an absolute commit.
7. **#61 Definition name.** Commits on blur, usually right next to #60 in the same panel.
8. **#8 Setup-guide project rename.** Built from the document as it was when the panel rendered.
9. **#58 Library project rename.** Whole-document replacement with no idle check.
10. **#68 Mechanical profile save.** Rebuilt from the latest saved document, but ignores edits still queued.

Next tier, protected only by the idle or `Ready` check: #26 board rename, #40 board reference, #56–57 geometry scripts, #44 release reviewed connections, and #66 generator Apply.

## 5. How panels decide an edit "landed"
- **Std-landed:** the matrix, outline, keymap, keycaps and mechanical-settings panels. Align uses the same logic through the helper `pending_settlement_gate`. The case panel uses a weaker version (saved and token changed only).
- **Revision is exactly base+1:** add board, existing half, `completion_is_accepted`, module attach.
- **Whole-document equality:** the PCB wiring panels and `accepted_matches_proposal`.
- **Content checks:** `expected_matches`, `intent_applied`, `is_applied_to`, `change_is_applied`, `expected_applied`, field-value and id-exists checks, and `settle_edit`.
- **Transaction-id match:** only `mirrored_pair_lifecycle.rs:46`. It is the only check that ties the landed snapshot to this specific edit.
- **Outcome only (success taken at face value):** the model editor, module profile, mechanical profile, module inspector and apply-to-key.
- **No check at all:** every row whose Settle column says "none", plus #40, whose observed result is thrown away.

## 6. Tests
I didn't re-count assertions after the split; the harness kinds below are as I classified them before it.
- **Browser tests that capture the submitted events instead of reaching a Session:**
  - `layout_component_inspector_tests` covers #1–3.
  - `matrix_transform_inspector_tests` covers #19.
  - `outline_lifecycle_browser_tests` covers the outline actions and touches the board-name input (#26).
- **Native tests on a stub runtime that records events:** `outline_lifecycle_tests`, `pcb_wiring/mode_owner_tests`, `pcb_physical_setup/tests`.
- **Other intercepting harnesses:** the definition-name and generator tests use `set_definition_name_test_state`; the `part_placement` tests use a fake runtime.
- **Real Session + Core:**
  - Library rename and PCB module save run in the browser through `project_name_test_support::install`.
  - Native tests in `parts_new_component`, `parts_definition_name`, `mechanical_profile`, `matrix_transform_lifecycle`, `firmware_position_projection` and `pcb_handoff`.
- **`web/tests/*`** are native copies of the logic, not the production panels.

Each real-Session test covers a single edit plus undo/redo or persistence. None queues two edits to check for last-writer-wins.