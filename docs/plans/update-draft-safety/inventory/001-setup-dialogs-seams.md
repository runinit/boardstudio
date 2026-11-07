# Inventory 001 — Setup, dialogs/settings, presentation wrappers, and shared seams

Orchestrator scope for Wayfinder 001 (complements `001-layout-metadata.md`,
`001-pcb-parts.md`, `001-keymap-keycaps-case.md`). Evidence: native Gortex MCP
`search`/`read` (symbol IDs + spans cited); `rg` used only to discover
candidate `use_signal`/`oninput` sites where Gortex text search returned no
match for parenthesised patterns, each finding below verified by a file read.
Rechecked against the dirty worktree. No product code changed.

Class key (shared with the other fragments): A = explicit-submit draft, no
blur admission, no gate visibility · B = blur/focusout/Enter-committing ·
C = immediate commit · D = preview-only / post-submission pending only ·
E = disposable navigation/search/filter/preference text (excluded by charter) ·
U = unknown/browser-dependent.

## Owner table

| # | Owner | Class | Admission | Pending route |
|---|---|---|---|---|
| S1 | Setup-guide project name (`ProjectSetupGuide` / `use_setup_project_name`) | B | Blur commits, Enter blurs, empty restores | `EditTicket::begin("setup-project-name")` — counted as L16 in the Layout fragment; not recounted here |
| S2 | New matrix form (`MatrixSetup`, setup guide + objects panel) | **A (creation form)** | Sliders/toggles write local signals only; submit only via **Create matrix** (`onsubmit`); Cancel discards by design | `MatrixSetupCreateRequest` → controller one-shot at create time; nothing pending before Create |
| S2b | Mirrored-pair Add-matrix form (`MirroredPairForm`, `web/crates/layout/src/objects/mirrored_pair.rs:126–286`) — classified 007 | **A (creation form)** | Left/right-name + gap `oninput` write local signals only; submit only via form `onsubmit` → `on_preview(MirroredPairRequest)` (Preview placement); invalid names/gap return silently | Preview request → controller proposal (`catalogue::physical_setup::propose_with_mirror_pair`, pure builder, no draft); nothing pending before Preview |
| S3 | Setup-guide preferences (stage/open, per project) | E | Stage/open changes persist immediately to local preferences (`write_preferences`) | None — UI preference, not project data |
| S4 | New board action (`use_board_setup`) | D | Click only, no typed value | `PendingEditSignals<BoardAction>::begin_one_shot(Add)` post-click |
| S5 | Catalogue physical/matrix setup proposals (`catalogue::physical_setup`, `matrix_setup_operation`) | — (no UI owner) | Pure proposal functions, no signals/inputs | n/a |
| S6 | Export workspace / ZMK firmware export | — (no draft owner) | Export actions fire immediately; "Export draft" is an export variant, not an editor draft | Post-action runtime work only |
| S7 | Search/filter/navigation text: command-palette query, Library project search (`library.rs:981`), Parts search query, workspace/panel/appearance signals | E | n/a | None |
| S8 | Dialog primitives (`ui-shared` Dialog/AlertDialog) and `UpdateWarningDialog` | — (no draft owner) | Confirm/cancel only; the update warning itself holds no editable value | n/a |

## Per-owner notes

### S2 — New matrix form — Class A (creation form, distinct sub-case)
- Production mount: rendered inline in the setup guide
  (`presentation.rs:8648–8660`, `inline_matrix_setup`) and in the objects
  panel; component `MatrixSetup`
  (`web/crates/layout/src/objects/matrix_setup.rs:105–220`).
- Local signals `rows`/`columns` (text form "5"/"6"), sliders and assembly
  toggles (`matrix_setup_fields.rs:54–160`, IDs
  `…/matrix_setup_fields.rs::MatrixDimensionSliders`,
  `…/matrix_setup_fields.rs::MatrixAssemblyPicker`) write only those
  signals. No blur/Enter commit, no `aria-invalid`.
- Admission: form `onsubmit` (lines 131–146) parses and calls
  `on_create(MatrixSetupCreateRequest)`; the **Create matrix** submit
  button is the only admission. Before that, nothing reaches Session,
  `PendingEdits`, or the update gate.
- Baseline difference from L1/L10: there is no accepted document value —
  the baseline is form defaults for a not-yet-created matrix. Reload loss
  would lose form choices, not an edit of accepted data. Whether creation
  forms are in the protected set is a question for
  [Choose the draft-safety interface and lifetime contract](../issues/004-choose-draft-safety-interface.md),
  recorded here as a classification, not decided.
- Hidden retention: the form is an overlay panel; switching workspace
  unmounts it and the choices die with the component (no retention
  signal observed).

### S2b — Mirrored-pair form — Class A (creation form, classified 007)
- `MirroredPairForm` (ID `…/mirrored_pair.rs::MirroredPairForm`): local
  `left_name`/`right_name`/`gap` signals seeded from projection defaults;
  dimension sliders and preset pickers write the same locals. The only
  admission is the form `onsubmit` → `on_preview(MirroredPairRequest)`;
  no blur/Enter commit, no `aria-invalid`. Same creation-form sub-case as
  S2: baseline is form defaults, reload loses the choices; whether
  creation forms are in the protected set stays with ticket 004.

### S1/S3 — Setup guide
- `ProjectSetupGuide` (`web/crates/layout/src/setup_guide.rs:281`,
  ID `…/setup_guide.rs::ProjectSetupGuide`) has exactly one text input:
  the project name, owned by `use_setup_project_name`
  (`presentation.rs:3065`, counted as L16). Its other stages embed
  `MechanicalSetupBasics` (Case — see the Keymap/Keycaps/Case fragment)
  and `ControllerSetupPicker` (Parts — see the PCB/Parts fragment),
  mounted at `presentation.rs:8662–8686`.
- Guide stage/open state lives in `SetupWorkspaceState`
  (`web/src/presentation/setup_workspace.rs:10–30`) and is written to
  local preferences on change (`write_preferences`,
  `setup_guide.rs:596`). It survives reload by design and is not project
  data: Class E.

### S5–S8 — Negative results (searched, no draft owner)
- `web/crates/catalogue/src/physical_setup.rs` and
  `matrix_setup_operation.rs`: pure functions (`propose`, `propose_with_mirror_pair`, …);
  no Dioxus signals or inputs.
- `web/src/presentation/export_workspace.rs`, `zmk_firmware_export.rs`:
  no `oninput`/local text signals in production code; export buttons act
  immediately.
- `web/src/presentation/command_palette.rs:287` query signal,
  `library.rs:981` search input, `parts_workspace.rs:25` query: Class E
  under the charter's disposable-text exclusion.
- `web/crates/ui-shared/src/ui/dialog.rs::Dialog`,
  `alert_dialog.rs::AlertDialog`: presentation primitives with no value
  state. Library delete dialogs (no typed draft) are covered in the
  Layout fragment (L17 notes).
- `web/src/presentation/notifications.rs::UpdateWarningDialog` (line 151):
  the gate's own cancel-only warning; holds no draft.

## Shared seams — what the gate can and cannot see

| Seam | Source | What it actually provides | What it does not provide |
|---|---|---|---|
| `PendingEditSignals<K, D>` | `web/crates/ui-shared/src/pending_edit_helpers.rs` (ID `…/pending_edit_helpers.rs::PendingEditSignals`); module doc lines 1–64 | Binds a field's live draft Signal + failure Signal per key with binding epochs; `begin_field`/`begin_value` submit an exact draft; `settle` writes back only while the live draft still equals the submitted value, so an older settlement never overwrites newer typing | No accepted-baseline at bind time, no draft≠accepted query, no enumeration of bound keys for an outside reader, no cross-component registry. A bound-but-never-submitted draft (Geometry L10) is invisible |
| `OwnedEdits<K>` | `web/crates/ui-shared/src/owned_edits.rs:1–109` (ID `…/owned_edits.rs::OwnedEdits`) | Per-controller owner lifetime: `follow_owner(scope, generation)` retires observations and rebinds still-mounted field Signals; `bound_draft(key)` reads a mounted field's current text at submission time; pending-key tracking/pruning | Scope is one controller/owner, not the project; no global inventory, no baseline comparison |
| Runtime `PendingEdits` collection | via `PendingEditSignals` above | Post-submission queue/active state feeding `Session::has_pending_work` | Pre-submission drafts never enter it |
| Update gate inputs | `Runtime::update_safety_inputs` (`web/crates/runtime/src/runtime.rs:930–946`); `visible_invalid_draft` (281–293); decision `update_safety` (`web/crates/runtime/src/update_safety.rs:139–174`) | Lifecycle/recovery, storage init/error, Session pending work, durability vs accepted revision, started/retained save, save conflict, and one DOM query for `aria-invalid=true` on input/textarea/select/combobox/textbox/spinbutton | No general dirty-draft input. The module doc itself records the gap (`update_safety.rs:25–27`): a component holding an uncommitted draft without blur-commit and without `aria-invalid` "is not visible". Both checkpoints (initial `begin_update`, post-takeover `reload_when_safe`) consume this same input set |

## Explicit unknowns in this scope

1. Whether creation-form state (S2 and the mirrored-pair variant S2b,
   classified A-creation-form in 007) belongs in the protected set — a
   product classification for ticket 004, not settled here.
2. Hidden/portal retention of the New matrix overlay across workspace
   switches is unmount-by-source-reading; no mounted trace was run.
3. No browser proof in this fragment: focus/blur ordering at the actual
   Update Now entry points belongs to
   [Establish a real-editor draft-loss probe](../issues/002-establish-real-editor-draft-loss-probe.md).

## Validation

- Gortex MCP `search`/`read` for cited symbols/spans; `rg` discovery only
  where Gortex text search returned no parenthesised-pattern matches,
  verified by reads. No product file edited; no tests/builds run.
