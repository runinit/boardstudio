# Planning 001 — PCB + Parts draft-owner census

Source-grounded census of live **project-editing** draft owners in `web/crates/pcb/**`
and `web/crates/parts/**`, read against the dirty worktree overlay (exact Gortex
views; dirty generations 62–66 observed across reads). No product source was changed.
Starting point `docs/research/2026-11-07-019-draft-safety-change-map.md` did not census
PCB/Parts; every row below was rechecked in dirty source via native Gortex
`search`/`read` (no shell grep/cat for source).

Conventions: all paths repo-relative. Symbol IDs are exact Gortex IDs with the
`ts-boardstudio2/` prefix trimmed in prose (full IDs in the table). Line spans are the
component/function extent unless narrowed to the control. "Gate visibility" = whether
the update-safety gate (`UpdateSafetyInputs`/invalid-field query/pending-work signals)
can observe the unsubmitted value — in all A/B rows below the answer is **no** by
construction (local signals only; `PendingEditSignals` binding alone does not report).

Class legend: **A** explicit-submit draft, no blur admission, no gate visibility
(confirmed unprotected) · **B** blur/focusout/Enter-committing (ordering risk only) ·
**C** immediate commit on change/select · **D** preview-only or post-submission pending
only · **E** disposable navigation/search/filter/view state (excluded) · **U**
unknown / browser-dependent (reason stated).

## Owner table

| # | Owner (component · file) | Class | Value + baseline | Input/change behavior | Admission route | Validation / aria-invalid | Pending-edit route | Unmount/hidden retention |
|---|---|---|---|---|---|---|---|---|
| A1 | `GeneratorSettingsEditor` · `web/crates/parts/src/parts/generator_settings.rs:785–1363` · ID `.../generator_settings.rs::GeneratorSettingsEditor` | **A** | Local `edits: BTreeMap<String,Value>` + `changed_parameters: BTreeSet` (815–820); accepted baseline = definition generator parameters (`initial_parameters`) | Every parameter control calls `on_change` → `change_parameter` (1023, 1312), which only updates the local map and fires a preview (`request_preview` → `GeneratorDraftStore`); nothing is submitted | **Apply generator settings** button → `begin_one_shot(GeneratorKey::Apply, …, generator_apply_resolver…)`; model upload → `begin_one_shot(GeneratorKey::Upload, …)`. No blur/key handler on any parameter field (`GeneratorParameterField`, 1366–1497, wires `oninput` straight to `on_change`) | Preview failure / resolver refusal → `ScopedFeedback` Alert only; parameter fields carry **no** `aria-invalid` (the 3d-model row renders `FieldError` text without setting the input's error prop). Invalid JSON/numbers stay local until Apply | `PendingEditSignals<GeneratorKey>`; one-shot Apply + Upload; settle retires/clears on owner change | Owner change **clears** drafts (`use_effect` on owner resets `edits`/`changed_parameters`/store). Unmount drops everything — nothing retained |
| A2 | `ModuleProfileEditor` · `web/crates/parts/src/parts/module_profile_editor.rs:295–813` · ID `...::ModuleProfileEditor` | **A** | `draft: ModuleProfileDraft` + `rotary: ModuleRotaryDraft` + `shape: ModuleVolumeDraft` + `reviewed` checkbox (317–322); baselines `ModuleProfileDraft::from_definition` / `from_profile` | All controls (`EditorTextField/NumberField/Select/Checkbox`, pulses/triggers, volume X/Y/width/depth/Z/height/source, model vectors, mounts/openings add-remove) mutate local signals only (`oninput` → `with_mut`, no blur props passed) | **Save project module profile** button → `submit_profile` → `profile_edits.begin_value(ModuleProfileKey::Save, …)` + `rotary_edits.begin_value(…)` (608/617). No blur/Enter admission anywhere in the render | Add-volume validates at click (`ModuleProfileError` Alert, no field-level marker); submit-time resolver errors → `visible_error` Alert; **no `aria-invalid`** on any profile input | Typed `PendingEditSignals<ModuleProfileKey, ModuleProfileDraft>` + `<…, ModuleRotaryDraft>`, bound `Save`, `bind_field` (333–335) | New owner retires observations and resets all drafts (`settle(false)` + resets); `use_drop` unbinds. Close/navigation drops the draft |
| A3 | `ManualProfileEditor` + `contour_editor` · `web/crates/parts/src/parts/mechanical_profile_editor.rs:86–475, 807–905` · IDs `...::ManualProfileEditor`, `...::contour_editor` | **A** | `draft: MechanicalPartProfile` signal (88); `purposes` map; plate-gap number; contour vertex arrays; standard-load / extraction async merges write into the same draft | `plate_to_pcb` oninput parses and ignores unparseable text; contour vertex `oninput` parses `f64`, silently keeping the old value on failure; purpose selects / family select / checkboxes write local state; "Use standard cutout" / "Read KiCad layers" load **into the draft**, not into the document | **Save fit profile** button → `on_save.call(draft())` → owner's `begin_one_shot(ProfileKey::Save, …, mechanical_profile_resolver…)` (`mechanical_profile_ui.rs` `save_profile`). No blur/Enter admission | Load/extraction failures → `Alert`; add-volume-style checks → owner error Alert; **no `aria-invalid`** on plate-gap or vertex inputs | Owner shell (`PartsMechanicalProfileWorkspace`, `mechanical_profile_ui.rs:43–282`) holds the one-shot `ProfileKey::Save`; `PendingEditSignals<ProfileKey>` + `bind_one_shot` | `editing=false` (Cancel/close/save) unmounts the editor and the draft signal is dropped. Reopen restarts from accepted/existing profile. Nothing retained |
| A4 | Assembly draft (`SavedAssembliesEditor` + `AssemblyDraftFields` + `AssemblyMemberFields` + `AssemblyModelFields` + `ModelVectorFields`) · `web/crates/parts/src/parts/assembly_editor.rs:331–774, 1065–1243, 1246–1633, 1636–1673, 1676–1709` · IDs `...::SavedAssembliesEditor`, `...::AssemblyDraftFields`, `...::AssemblyMemberFields`, `...::AssemblyModelFields`, `...::ModelVectorFields` | **A** | `editing: Option<AssemblyDraft>` (`base/value/assets/document_id/session_epoch/scope`, 32–39) in the parent; per-field `value: Signal<AssemblyDefinition>` + `origin_x/origin_y` string signals in the fields components | Name/member/choice/select/number/file controls call `update_member` → `MemberPatch::apply` (1769–1856; X/Y/rotation/vectors parse `f64` and **silently ignore** unparseable input) → `publish_draft` → parent `editing` signal. No blur/key handlers on any assembly field | **Save assembly** → `begin_one_shot(AssemblyKey::Save, …)`; **Place on selected board** → `begin_place_assembly_on_board` (777–930: parses origin strings, `feedback` text on non-finite, then `begin_one_shot(AssemblyKey::Place, …)`); **Apply to selected matrix** → `begin_apply_assembly_to_matrix` → `begin_one_shot(AssemblyKey::Matrix, …)`. No blur admission | Origin/parse/proposal failures → `feedback` status text; save refusal surfaces via settle `feedback`; **no `aria-invalid`** on assembly inputs | `PendingEditSignals<AssemblyKey>` (Save/Place/Matrix one-shots) + `pending: Vec<AssemblyAction>` intents; settle retires silently on owner change and clears `pending` | Owner change retires + clears; **Close editor drops the draft** (`editing.set(None)`). Origin fields reset per mount (`use_signal "0"`). Nothing retained |
| A5 | `PcbMountedModuleInspector` · `web/crates/pcb/src/pcb_module_inspector.rs:88–933` · ID `...::PcbMountedModuleInspector` | **A** | Single `draft: MountedModule` + `draft_dirty` (117–119); accepted baseline = `accepted_instance`; support sub-draft `SupportDraft` (120, 656–732) staged into the same draft via the Add control; `joins` connection map staged likewise | **Every** control (placement X/Y/rotation/gap/service-clearance numbers 587–641, host-instance/face/attachment selects 546–616, detached checkbox 738, connection selects + bus/terminal/voltage inputs 747–829, joins selects 842+, support fields) writes via `edit_module_draft` (937–947, value-equality dirty marker). `PcbInput`/`PcbChoice` forward **only `oninput`/`on_change`** (`pcb_controls.rs`); no blur props exist | **Save placement** button (921, `onclick: save`) → `save_edits.begin_value(ModuleEditKey::Placement, …)` (313). No blur/Enter admission | Placement binds `placement_failure` (bound-field inline failure); action failures → panel `feedback`; numeric controls parse-guard on input (invalid keystrokes ignored, control re-renders accepted/draft text). **`PcbInput` has no error/`aria-invalid` prop at all** — no per-field invalid marker exists on this owner | Typed `PendingEditSignals<ModuleEditKey, MountedModule>` (`Placement` bound field) + one-shot Remove/Embed/RemoveCircuit; settle restores submitted values, clears `draft_dirty` only when draft == accepted | Component keyed by module (`owner_key`); `use_drop` unbinds all keys; selection/owner change retires. Draft lives exactly as long as the mount — hidden retained panels keep it (Collapsible `keep_mounted`), genuine unmount drops it |
| A6 | `ModuleAttachment` options · `web/crates/parts/src/parts/modules_catalogue/module_attachment.rs:43–402` · ID `...::ModuleAttachment` | **A** | `connector_selection` (defaults to automatic) + `host_connection_enabled` checkbox; validation-only `feedback` string; `attached_module_id` post-submit navigation | Choice/checkbox write local signals (`on_value_change` / `on_checked_change`); nothing is submitted on change | **Attach module to {board}** button → `begin_one_shot(AttachKey::Attach, …, attachment_resolver…)` (after async connector load + re-validation). No blur admission | Missing-connector / stale-owner problems → `feedback` Alert text; **no `aria-invalid`** | `PendingEditSignals<AttachKey>` one-shot; settle lands → toast + `on_attached` navigation, fail → message, retire → silent | Signals re-initialize per mount (automatic connector default); unmount/remount resets. No draft retention |
| A7 | New-net-name field · `generic_part_wiring` in `web/crates/pcb/src/pcb_wiring.rs:526–637` (`PcbInput` 617–622, submit 606–616) | **A** | `new_net_name: Signal<String>` (prop-passed); accepted baseline = nets list (empty = no-op; submit disabled while blank) | `oninput` writes the signal only; no blur/key handler | Form `onsubmit` / **Add net** button → `PartNetEditRequest{CreateNet{name}}` → `on_edit` → controller `begin` (one-shot-style submit). Cleared on submit | Empty name: button disabled + resolver `Err("Enter a name…")` (`part_connections.rs` `propose`); Net feedback → panel `Alert`; **no `aria-invalid`** on the input | Wiring `PendingEdits` keyed tickets via `use_part_net_edits` (controller); submitted name is post-submit pending only | Signal owned by `PcbWiringInspector` (`pcb_wiring.rs:253` `use_signal`, passed at :317; IDs `.../pcb_wiring.rs::PcbWiringInspector`, `...::GenericPartWiringProps`, `...::generic_part_wiring`). Cleared only on successful submit (`add_name.set(String::new())` after `on_edit.call`); no reset on selection change — single mount without `key` (`workspace_composition.rs:112`, props built `presentation.rs:8274`) so text survives part switches and collapse (`keep_mounted`), drops on workspace switch/unmount. Stale text can carry to another part's Add — still Class A (explicit submit, no gate visibility) |
| B1 | `DefinitionNameEditor` · `web/crates/parts/src/parts_definition_name.rs:37–242` · ID `...::DefinitionNameEditor` | **B** | `draft: String` + `draft_dirty` (44–45); baseline = `definition.name`; `failure` message | `oninput` → local draft + dirty (222–225); no immediate submit | **Blur** → `on_blur` → ownership check → `begin_field(NameFieldKey::Name, …, definition_name_resolver…)` (150–180). **Enter** → native `blur()` (keydown 184–196); **Escape** restores accepted + clears dirty | `aria_invalid: name_error.is_some()` on the input + `FieldError`; failure Alert path also sets `failure` on settle | `PendingEditSignals<NameFieldKey>`, `bind_field(Name, draft, failure)`; owner change → unbind + `settle(false)` + reset draft; `use_drop` same; accepted-equality clears dirty | Retained while mounted (bound field survives rerenders/settlement); owner change or unmount drops it. Hidden (`keep_mounted` collapsible) retains |
| B2 | Courtyard W/H · `CustomDefinitionFields` in `web/crates/parts/src/parts_custom_definition.rs:253–643` (fields ~520–560, blurs ~500–518) · ID `...::CustomDefinitionFields` | **B** | `width/height: String` + `width_dirty/height_dirty` (courtyard strings of the accepted definition); shared `failure` | `oninput` → local + dirty (602/610); `validate_definition_edit` runs at submit, not at typing | **Blur** → `submit(DefinitionEdit::CourtyardWidth/Height)` → validate → `begin_field` (else dirty-clear fast path). **Enter** → blur, **Escape** → restore (`draft_keydown`, 1044–1063) | No `error` prop is passed to these `EditorNumberField`s → **no `aria-invalid`**; validation failure → shared panel `Alert` (`failure`) | `PendingEditSignals<DefinitionFieldKey>` (`CourtyardWidth/Height` bound fields); owner change → unbind both + `retain_rows(&[])` + `settle(false)` + reset; `use_drop` same | Same pattern as B1: retained while mounted incl. hidden; dropped on owner change/unmount |
| B3 | Pad row text/number cells · `PadFields` in `web/crates/parts/src/parts_custom_definition.rs:781–983` (cells 958–978) · ID `...::PadFields` | **B** | Per-row `RowDrafts` (7 text signals + dirties) owned by the parent `PanelRows` (so settlement never touches an unmounted row); baseline = accepted pad text | `oninput` → row draft + dirty; no immediate submit | **Blur** per cell → `submit(PadId/PadNumber/PadCoordinate/PadSize/PadDrill)` → validate → `begin_field` or `begin_one_shot` (Remove only). **Enter** → blur, **Escape** → restore (`draft_keydown`) | No `error` prop on the cells → **no `aria-invalid`**; failures → shared panel `Alert` | Same helper/keys (`DefinitionFieldKey::Pad{row,field}` bound per visible row each render; `retain_rows` prunes vanished rows but keeps their terminal failure at the panel) | Row handles pruned when the pad leaves the accepted list; owner change wipes all rows (`retain_rows(&[])`); unmount unbinds + settles(false) |
| B4 | Model alignment axes · `ModelVectorEditor` in `web/crates/parts/src/parts/component_model_editor.rs:729–894` · ID `...::ModelVectorEditor` | **B** | Per-axis `drafts[3]: String` + `dirty[3]` + `submitted[3]` admission memory; baseline = accepted `Vec3` per field | `oninput` → draft + dirty (typing stays a draft even if it re-equals accepted while settling) | **Blur** → parse + finite/positive check → inline error **or** `on_commit` → parent resolver → `begin_field(ModelKey::Transform{field,axis}, …, &raw)` (858). **Enter** → blur; **Escape** → restore submitted-or-accepted | Local `error` per axis → `ModelAxisField` sets **`aria_invalid` + `aria_describedby`** (`model_axis.rs:16–64`) — the one B owner with a real field-level invalid marker; helper `failure` → `FieldError` | `PendingEditSignals<ModelKey>` bound per axis; settle uses `settle(true, …)` (always-live, intentional — see U4) and clears `submitted` per settled axis | Remounted per model owner (parent keys by owner); revision/undo follow accepted only when clean. Unmount drops axes |
| C1 | Definition kind select · `CustomDefinitionFields` (~470–495) | **C** | No draft: `on_value_change` → `on_kind` → `submit(Kind)` immediately | Select commits on change | Immediate `begin_field` (or fast-path no-op if unchanged) | Resolver refusal → panel Alert | Same `DefinitionFieldKey` helper | n/a (no pre-submit state) |
| C2 | Pad shape select · `PadFields` (~930–955) | **C** | No draft: `on_shape` → `submit(PadShape)` immediately | Select commits on change | Immediate submit path | Same panel Alert | Same helper | n/a |
| C3 | Routed-board reference controls · `Editor` in `web/crates/pcb/src/pcb_board_reference.rs:607–1038` · ID `...::Editor` | **C** | **No local text draft at all**: X/Y/Z/rotation `PcbInput.oninput` parses and calls `on_action` only when finite (922–956); checkbox/select/file controls call `on_action` directly | Every valid keystroke/selection dispatches immediately (`dispatch_board_reference_action` → `PendingEdits::begin`, `board_reference_owner.rs`); **unparseable keystrokes are discarded** (control re-renders the pending/accepted value) | Immediate per-change `begin` (keyed `BoardReferenceKey`); pending value shown via `reference_with_drafts` + `submissions` memory | Failures → panel `Alert` + `error` signal; **no `aria-invalid`** anywhere on these controls | `PendingEdits<BoardReferenceKey>` + `PendingEditSignals` one-shot Remove; settle-on-version with `place_reference_results` | No unsubmitted state exists to retain; unmount retires observations via `use_drop`-adjacent guards. File-replacement flows are async pre-submit reads (D-like) with stale-owner discard |
| C4 | Board wiring mode select · `use_board_wiring_mode_edits` in `web/crates/pcb/src/pcb_wiring/mode.rs:69–194` (rendered `board_wiring` ~792) | **C** | `draft` mirrors the submitted text for pending projection only; no pre-submit draft | Select → `on_change` → `begin_field` immediately ("Submit the exact text the user chose") | Immediate `begin_field` | Failure → inline failure text; no pre-submit invalid state | `PendingEditSignals<BoardWiringModeFeedbackTarget>` bound field | Departing drops only the observation; the Session edit can still land (documented) |
| C5 | Wiring pin selects · `use_pcb_wiring_pin_edits` in `web/crates/pcb/src/pcb_wiring/pins.rs` (rendered `pin_assignment_row`, `pcb_wiring.rs:962`) | **C** | `submissions` memory = pending projection only | Select → `on_change` → `begin` immediately | Immediate `begin` | Failure → attributed failure text; nothing pre-submit | `PendingEdits<PcbWiringPinKey>` + `PinOwner` submissions context | Same retire-on-depart pattern |
| C6 | Part-net assignment selects · `generic_part_wiring` (`pcb_wiring.rs:594–616`) via `use_part_net_edits` (`controller.rs`) | **C** | `submissions`-style keyed intents are post-submit pending only | Net select → `on_edit(AssignPads)` immediately | Immediate `begin` | Refusal → panel `Alert` (`actions.feedback`); availability pre-checked (`pin_edit_is_available` analogue in `part_connections.rs::propose`) | `PendingEdits` keyed tickets | Retire-on-depart; queue drains serially |
| C7 | Part press/generator settings · `PartInputInspector` in `web/crates/pcb/src/pcb_wiring/part_input_settings.rs:133–282` + `owner.rs::use_part_net_edits` | **C** | No pre-submit draft: scan-mode select, generator-net selects, anchor X/Y number inputs all call `on_edit` directly (anchor `oninput` parses, ignores unparseable/empty→None immediately) | Change/select/keystroke (valid) dispatches immediately | Immediate `begin` via the part-input queue (`owner.rs`); committed intents re-projected over accepted via `apply_drafts` (display of pending, not a draft) | Resolver-level `validate_parameter_value` retires with message → panel Alert; **no `aria-invalid`** on these inputs | `PendingEdits<PartInputKey>` + `InputDrafts` context; serial queue + `preparing` gate | Departure retires per-ticket scope; committed queue items still land |
| C8 | Firmware keymap selects · `FirmwareKeymapPanel`/`FirmwareChoice` in `web/crates/pcb/src/firmware_positions.rs` (+ `controller.rs::use_firmware_position_edits`) | **C** | `submissions` = pending binding projection only | Combobox select → `on_change(FirmwarePositionEditRequest)` immediately | Immediate `begin` | Pending/Failed → inline status/Alert; pre-submit has no invalid state (choice list is closed) | `PendingEdits<FirmwarePositionFeedbackTarget>` + `FirmwareOwner` submissions | Scope-departed tickets retire |
| C9 | Physical-setup toggles/selects · `use_controller` in `web/crates/pcb/src/pcb_physical_setup/controller.rs` (rendered by Case/Project-Guide owners via `PhysicalSetupMount::submit`) | **C** | No draft: intent enums only | Toggle/select → `mount.submit(intent)` → `on_intent` → async prepare → `PendingEdits::begin` | Immediate (one in-flight gate via `busy`) | Failure → operation feedback text; landing is silent (accepted document is the confirmation) | Task-local `PendingEdits<OwnerIdentity>` per operation (explicitly *not* `PendingEditSignals` — "workflow orchestration, not field settlement") | n/a |
| D1 | 3D-model upload / remove · `ComponentModelEditor` (`component_model_editor.rs:270–729`) | **D** | No typed draft: `uploading` flag + `submit_owner` capture; file bytes read then `begin_one_shot(ModelKey::Upload, …)`; **Remove attached model** → `begin_one_shot(ModelKey::Remove, …)` | Button/file-picker only | Explicit click/pick → one-shot submit | Read/store failures → `ModelEditorMessage` Alert; owner-scoped visibility | `PendingEditSignals<ModelKey>` one-shots; `upload_pending`/`remove_pending` disable | Async reads guarded by `alive` + generation + `current_snapshot`; stale completions discarded, never applied |
| D2 | Add/Remove pad · `CustomDefinitionFields`/`PadFields` | **D** | No draft: `begin_one_shot(AddPad / Pad{Remove})`, buttons disable while pending | Button only | Explicit click | Panel Alert | Same definition helper one-shots | n/a |
| D3 | New custom component · `NewCustomComponentAction` (`parts_new_component.rs:46–189`) | **D** | No draft: `waiting: Option<CreationIntent>` post-submit only | Button → `begin_one_shot(CreateKey::NewComponent, …)` | Explicit click (guards: mounted, workspace, scope) | Failure → scoped Alert; land → selects the created definition | `PendingEditSignals<CreateKey>` + `waiting` intent; settle by owner liveness | `use_drop` flips `owner_is_mounted`; unmount retires |
| D4 | Import KiCad footprint · `ImportKiCadFootprintAction` (`parts_import_footprint.rs:253–712`) | **D** | No draft: `pending: Option<ImportPhase>` (Preparing→Committed) post-pick only | File pick → async read/compile → `begin_one_shot(ImportKey::Footprint, …)` | Explicit pick (+Cancel button drops the phase) | Read/compile/owner failures → scoped Alert/notice; stale-read discards with retry text | `PendingEditSignals<ImportKey>`; settle lands/selects or fails/retires | `use_drop` bumps epoch + unmounts; stale async completions return early |
| D5 | Assembly create/duplicate/customize · `SavedAssembliesEditor` | **D** | Drafts constructed programmatically (`AssemblyDraft{base,value,…}`), not typed | Button only | Explicit click opens the A4 editor | Construction failures (id allocation, scope change) → `feedback` status | Subsequent save/place/apply are A4 one-shots | n/a (creation, then A4 retention) |
| D6 | Module Embed / Remove module / Remove circuit · `PcbMountedModuleInspector` | **D** | No draft: one-shot keys with bound disabled signals | Buttons only | Explicit click → one-shot `begin` | Failure → attributed panel `feedback` | `bind_one_shot` (Remove/Embed/RemoveCircuit) | n/a |
| D7 | Wiring Apply / Release-reviewed-connections · `use_board_wiring_apply` (`apply.rs:38–174`) | **D** | No draft: one-shot keys, disabled-signal binding | Buttons only | Explicit click → `begin_one_shot(Apply / ReleaseReviewedConnections, …)` | Failure → attributed failure text | `PendingEditSignals<BoardWiringApplyKey>` one-shots | n/a |
| D8 | Protected remap review · `use_protected_remap_review` (`remap.rs`) + review button (`pcb_wiring.rs:838`) | **D** | No draft: `pending: Option<PendingProtectedRemap>` holds the *proposal*, not user typing | Button → `Event::ReviewElectricalRemap` → outcome → Saved/Failed feedback | Explicit click | `ProtectedRemapFeedback::{Pending, Saved, Failed}` status | Outcome-slot observation, fingerprint-checked | n/a |
| D9 | Previews · `PartsPreviewPanel` (`preview.rs`, header: "Read-only, source-backed 2D preview"), assembly preview (`AssemblyDraftFields`), generator preview (`GeneratorPreviewStatus`) | **D** | Rendered from draft/accepted inputs; never written back except through the owners' explicit submits | n/a (read-only) | n/a — a mounted test asserts preview failure "must not submit a project edit" | Load failures → Alert/status | Preview lease/owner generations (not edits) | n/a |
| D10 | Mechanical workspace shell · `PartsMechanicalProfileWorkspace` (`mechanical_profile_ui.rs`) | **D** | `editing` flag + `pending: Option<SubmittedProfile>` (post-submit record for settlement attribution) | Edit/Define button opens A3; Save delegates to A3's draft | Save → `begin_one_shot(ProfileKey::Save, …)` | Failure → Alert | One-shot Save + `save_disabled` binding | Owner/generation change retires + clears `pending` |
| E1 | Catalogue/library search boxes · `PartsLibraryPanel` (`parts.rs:1001`), `AddObjectComponentChooser` (`parts.rs:489`); `NewCustomComponentAction`/`ImportKiCadFootprintAction` clear `query` on success | **E** | Filter text only; never enters any edit path | Typing filters lists | n/a | n/a | none | n/a |
| E2 | Layer/scene visibility · `PcbLayerControls`/`CanvasLayers` (`pcb_layers.rs`), `PcbScene` toggles (`pcb_scene.rs`), preview layer toggles + 2D/3D switch (`preview.rs`), module layer `hidden` sets (`pcb_module_footprints.rs`) | **E** | View visibility only; `LayerVisibility` context, not project data | Toggles | n/a | n/a | none | n/a |
| E3 | Selection/navigation · `ModuleSourceFootprints` (`pcb_module_footprints.rs`, SVG `role=button` select only), `ModuleInspector` variant select + "Edit selected mounted placement" (`modules_catalogue.rs`), `SelectedDefinition` (`details.rs`, read-only), firmware disclosure `open`, PCB/Parts `Collapsible` states | **E** | Navigation/selection/disclosure only | Click/select navigates | n/a | n/a | none | n/a |
| U1 | New-net-name signal lifetime — RESOLVED (retention = A7 as written) | **A** | Owner traced: `PcbWiringInspector` creates `new_net_name` (`pcb_wiring.rs:253`) and passes it to `generic_part_wiring` (`:317`); single mount without `key` (`workspace_composition.rs:112`) | Typing is local and submit-cleared; no `use_effect`/`key` reset — text survives part-selection change and panel collapse, drops only on workspace switch/unmount (workspace `Pcb(Some)` → other arm) | Form submit (verified, A7) | No aria (verified) | Keyed part-net tickets (verified) | **Resolved — mount lifetime wider than one part; no new Class A (field already A7); stale-carry quirk noted in A7** |
| U2 | Mounted-module constituent footprint rows — RESOLVED (not A5) | **D** | `ConstituentAction` (`pcb_module_inspector.rs:62–70`) rows built at :446; per-row `Place` button only — no draft signal, no `edit_module_draft` anywhere in the constituent block (rows :903, handler :908) | `onclick` → `on_place_component.call(ComponentPlacementAction::AddSourceObject {…})` (:908) — starts a canvas placement session (`use_controller_placement`, `part_placement.rs:432–1399`, `AddSourceObject` arms :1019/:1064), committed later via `PlacementPendingKey::Placement` one-shot | Explicit Place click (placement session), never Save placement | n/a at row level; placement-session errors surface in the placement host | Placement-session pending (`PlacementPendingKey`), not `ModuleEditKey::Placement` | Rows unmount with the inspector; no staged value to retain |
| U3 | Native number-input sanitization (all B/C numerics) | **U** | Dioxus `InputType::Number` fields: browsers may refuse non-numeric keystrokes before `oninput` ever fires, and Enter-to-blur relies on the web `HtmlInputElement.blur()` path | Varies by browser/engine | Blur/Enter paths verified in source; **actual key behavior is browser-dependent** and needs a mounted run (cf. map.md §5 leg plan) | `aria-invalid` where present (B4) vs absent elsewhere is source-verified; whether AT exposes in-progress invalid text depends on rerender timing — **browser-dependent** | n/a | n/a |
| U4 | `ModelVectorEditor` always-live settlement — RESOLVED (intentional) | **B** (B4 stands) | `pending.settle(true, …)` (`component_model_editor.rs:787`) with per-axis accepted projection; helper chain `PendingEditSignals::settle` (`pending_edit_helpers.rs:371–394`) → `PendingEdits::settle` (`pending_edits.rs:87–104`) → `EditTicket::settlement` (`edit_ticket.rs:127–140`): `true` drains terminal outcomes while each ticket's own Scope lineage still retires scope-dead tickets; `false` would unconditionally retire live tickets | Intended: child remounts per model owner (parent keys `"{owner:?}-Offset/Rotation/Scale"`, :683–687) and owns a private `PendingEditSignals` collection, so while mounted the owner is live by construction — inline comment states this (:737–740). Siblings (B1/B2/`CustomDefinitionFields` :482, `parts_definition_name.rs:123`) persist across owner changes in one mount and must gate on `owner_live` + `settle(false)` reset; this child gets a fresh store per owner instead | Blur submit (verified, B4) | n/a | Helper-bound (verified); axes clear `submitted` per settled axis | **Resolved — not an anomaly; B4 unchanged. Residual: no `use_drop` unbind in the child (parent `ComponentModelEditor` settles only Upload/Remove via `settle(live)`); harmless — unmount drops the private store with its bound drafts** |

## Per-owner notes

- **A1 vs D9 (generator):** the preview pipeline is deliberately *not* an edit path —
  `change_parameter` only touches `edits`/`GeneratorDraftStore` and fires an async
  render. The only document mutations are Apply (full candidate definition) and model
  upload. A valid unsaved parameter set is therefore invisible to every gate while the
  editor stays open — the archetypal class-A shape (explicit Apply, no blur, no marker).
  Mitigating facts: owner change *clears* the draft (cannot go stale across selection),
  and Apply is disabled until the preview is `Ready`.
- **A2 (module profile):** the largest local surface in scope — profile + rotary +
  staged volume (`shape`) + `reviewed` gate + candidate-model attach all funnel into one
  Save. `submit_profile` is the sole admission; nothing commits on blur/change. Newer
  typing is protected against older accepted values by the `profile_dirty`/`rotary_dirty`
  markers, but that protection is *editor-internal* — no gate observes it.
- **A3 (mechanical fit):** two-layer shape — the workspace shell owns the one-shot
  ticket while the modal-ish editor owns the draft. Cancel/close discards without
  confirmation; the draft never touches `PendingEditSignals` until Save (binding is
  one-shot-only). Contour vertex inputs *look* immediate (value renders from draft) but
  invalid text is dropped, never committed — still A, not C, because valid values also
  wait for Save.
- **A4 (assembly):** one draft tree, three admissions (Save / Place-on-board /
  Apply-to-matrix). `origin_x/origin_y` are the only text fields in scope whose invalid
  state produces an inline message *before* submission ("Enter finite X and Y…"), yet
  still no `aria-invalid` and no gate visibility. `Close editor` discards silently.
- **A5 (mounted module):** the PCB counterpart to A4 — one `MountedModule` draft with
  ~20 staged controls and a single Save. Notable: `PcbInput` *cannot* express invalid
  state (no error prop), so the whole owner is marker-less by construction. The support
  sub-form (mount id/diameters/Z/height) is a draft-within-a-draft with its own Add
  button but no separate admission — it lands only via Save placement.
- **A6 (module attach):** smallest A surface (one select + one checkbox), but
  structurally identical: local state, single one-shot admission, no markers, reset on
  remount. Kept as its own row because its lifetime owner (catalogue selection) differs
  from A4/A5.
- **B1–B4:** all four share the blur→`begin_field` shape with Enter→blur and
  Escape→restore, but differ in markers: B4 alone sets `aria-invalid`; B2/B3 rely on a
  shared panel Alert; B1 sets both input `aria-invalid` and `FieldError`. None is
  observable pre-submission. Ordering risk only: focus movement dispatches work, so an
  update check racing blur may run before or interleave with the admission.
- **C3 (board reference):** the sharpest C in scope — not even a local draft exists for
  numeric fields; each valid keystroke is a dispatched edit and each invalid keystroke
  vanishes. There is literally nothing to retain, lose, or gate; staleness is handled
  per-ticket inside `PendingEdits`.
- **U1 (resolved):** `new_net_name` lives in `PcbWiringInspector` (`pcb_wiring.rs:253`,
  passed at :317; single mount without `key` at `workspace_composition.rs:112`), so typed
  text survives part-selection changes and panel collapse and is cleared only on
  successful submit — a stale-carry quirk across parts, but still Class A (explicit
  submit, no blur, no gate visibility). No new Class A promoted: the field was
  already A7; only the retention cell changed.
- **U2 (resolved):** constituent `Place` rows are not staged — each row calls
  `on_place_component` → `AddSourceObject` → a canvas placement session committed
  later under `PlacementPendingKey`, never `edit_module_draft` / Save placement.
  Reclassed **D** (button-only initiator, cf. D5); A5's draft boundary is unchanged.
- **U4 (resolved):** `settle(true)` is intentional, not anomalous — the child remounts
  per model owner (keyed `"{owner:?}-…"`), owns a private helper collection, and the
  inline comment (:737–740) states the lifetime equivalence; per-ticket Scope checks
  still guard cross-scope landings. B4 stands as written.

## Coverage list (denominator)

Files/dirs in scope, each searched; disposition (owner rows above or reason for none).
Test/example/support files are listed for completeness, not counted as owners.

**`web/crates/pcb` (29 files):**

| File | Disposition |
|---|---|
| `src/pcb_board_reference.rs` | C3 (+ D file flows) |
| `src/pcb_board_reference/matching.rs` | none — filename matching logic |
| `src/board_reference_owner.rs` | none — keys/resolvers/admission checks (cited as C3 evidence) |
| `src/pcb_module_inspector.rs` | A5 (+ D6) |
| `src/pcb_module_footprints.rs` | E3 (read-only SVG + selection) |
| `src/pcb_layers.rs` | E2 (visibility inventory) |
| `src/pcb_scene.rs` | none/E2 — canvas + layer toggles, no `PcbInput`/`PcbChoice`, no draft signals (verified via control search) |
| `src/pcb_controls.rs` | none — shared presentational controls (cited: `PcbInput` has no blur/error props) |
| `src/pcb_physical_setup.rs` + `controller.rs` + `tests.rs` | C9 (controller) |
| `src/pcb_wiring.rs` | A7 (net name) + C6 (net selects) + render for C4/C5/D7/D8 |
| `src/pcb_wiring/mode.rs` | C4 (+ `mode_owner_tests.rs` test-only) |
| `src/pcb_wiring/pins.rs` | C5 |
| `src/pcb_wiring/part_connections.rs` | none — pure proposal logic (cited: CreateNet name validation) |
| `src/pcb_wiring/part_input_settings.rs` + `owner.rs` | C7 |
| `src/pcb_wiring/part_net_admission.rs` | none — lifetime helper |
| `src/pcb_wiring/apply.rs` | D7 (+ resolver logic) |
| `src/pcb_wiring/connections.rs` | none — `existing_connection_review` logic |
| `src/pcb_wiring/remap.rs` | D8 |
| `src/pcb_wiring/controller.rs` | none new — immediate dispatchers for C6/C8 (`use_firmware_position_edits`, `use_part_net_edits`, resolution owner) |
| `src/pcb_wiring/queued_edit_tests.rs` | test-only |
| `src/pcb_wiring_remap_operation.rs` | none — proposal logic |
| `src/firmware_positions.rs` | C8 (+ E3 disclosure) |
| `src/firmware_position_choices.rs` | none — static choice list |
| `src/lib.rs`, `Cargo.toml` | none — wiring only |

**`web/crates/parts` (27 files):**

| File | Disposition |
|---|---|
| `src/parts/assembly_editor.rs` | A4 (+ D5) |
| `src/parts/component_model_editor.rs` | B4 + D1 |
| `src/parts/generator_settings.rs` | A1 |
| `src/parts/mechanical_profile_ui.rs` | D10 shell (editor = A3) |
| `src/parts/mechanical_profile_editor.rs` | A3 |
| `src/parts/mechanical_profile.rs` | none — resolver/owner logic (no UI) |
| `src/parts/standard_profile_lifetime.rs` | none — mount-lifetime helper (no inputs; verified: no `oninput`/editor controls reference it as a control) |
| `src/parts/module_profile_editor.rs` | A2 |
| `src/parts/modules_catalogue.rs` | E3 (catalogue + `ModuleInspector` navigation; mounts A2/A6) |
| `src/parts/modules_catalogue/module_attachment.rs` | A6 |
| `src/parts_custom_definition.rs` | B2 + B3 + C1 + C2 + D2 |
| `src/parts_definition_name.rs` | B1 |
| `src/parts_import_footprint.rs` | D4 |
| `src/parts_new_component.rs` | D3 |
| `src/parts/assembly_presets.rs` | none — recipe/proposal logic (no UI) |
| `src/parts_assembly_preset_draft.rs` | none — draft construction from recipe (no UI) |
| `src/parts/details.rs` | none — read-only definition facts |
| `src/parts/preview.rs` | D9 (+ E2 view toggles) |
| `src/parts_view_generation.rs` | none — preview generation (no inputs; no `oninput`/submit controls found) |
| `src/parts/editor_fields.rs` | none — shared presentational inputs (cited: forward `onblur`/`aria-invalid`, hold no drafts) |
| `src/parts/model_axis.rs` | none — shared axis input (cited: B4's `aria-invalid` source) |
| `src/parts.rs` | E1 (search boxes; mounts all Parts owners) |
| `src/parts/controls_tests.rs`, `placement_action_tests.rs` | test-only |
| `src/lib.rs`, `Cargo.toml`, `examples/inspector_020.rs` | none — wiring/example |

**Denominator: 56 files** (29 PCB + 27 parts). Production UI files with **no** draft
owner: 21 (logic/support/read-only/view-only as noted). Files owning ≥1 draft input:
14. Test/example/wiring files: 9 (no owners by construction). Former gaps **U1**
(net-name lifetime), **U2** (constituent block) and **U4** (`settle(true)` rationale)
are resolved in their rows; the only remaining unknown is **U3** (browser-dependent).

## Graph gaps vs source-confirmed vs browser-dependent

- **Source-confirmed:** every A/B/C/D/E row — control text, handler, admission call,
  and helper key were read in the exact dirty-worktree view (citations above).
- **Graph gaps closed (007):** U1 provider trace (`PcbWiringInspector::new_net_name`,
  `pcb_wiring.rs:253` → `generic_part_wiring` :317, mount `workspace_composition.rs:112`);
  U2 constituent block handlers (`pcb_module_inspector.rs:903`/`908` → `AddSourceObject`,
  `part_placement.rs:1019`/:1064); U4 `settle(true)` rationale (comment :737–740, parent
  keys :683–687, helper `pending_edit_helpers.rs:371–394` → `pending_edits.rs:87–104` →
  `edit_ticket.rs:127–140`). No open source-readable gap remains.
- **Browser-dependent:** U3 only — whether invalid text can even sit in a native
  number input, Enter/blur timing, and AT exposure of in-progress invalid text. The
  map.md legs 1–2 (real valid-draft RED/GREEN, real invalid-owner observation) remain
  the required next proof for A1–A7/B1–B4; this census does not substitute for them.

## Validation

- No product code changed, no tests run, no builds run (census task; validation =
  source re-reads only).
- All Gortex reads reported `exact:true` against worktree overlay
  `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378` (dirty generations 62→66 across the session —
  expected: overlay tracks the dirty worktree; reads were disk-verified).
- Pre-existing dirty edits were not touched; the sole file write of this task is this
  inventory.
- Checkpoint: notes `planning-001-pcb-parts-census` (subagent-local).
- 007 gap-closure (U1/U2/U4): source traces only via native Gortex
  `search`/`read` (exact worktree views, dirty generation 76); no builds, no tests,
  no browser. Sole file write remains this inventory.
