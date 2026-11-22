# Track 3: Collapse Monolithic Workspace Inspectors into Declarative `rust-ui` Specs

**Base:** Merged Wave 1 (`dev` + Track 1 + Track 2).  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely. Track 2's synchronous `runtime.commit(op)` MUST be landed first so inspectors require zero async settlement plumbing.

---

## 1. Why Wave 1 Failed Here (And What Track 3 Does Differently)

In `refactor/simplify-implementation-wave`, `web/crates/case/src/mechanical_settings*` shrank by only **−753 LOC out of 12,975 LOC (−5.8%)**. Why? Because it swapped individual HTML `<input>` tags for `rust-ui` inputs while **preserving all 4 hand-wired indirection layers** (`mechanical_settings_mount.rs` → `mechanical_settings_controller.rs` → `mechanical_settings_mount/projection.rs` → `mechanical_settings/*.rs` + 4,500 LOC of per-field mounted browser tests).

**Track 3's Rule:** Delete the 4-layer indirection stack in every workspace crate. Build **4 tiny, stateless declarative `rust-ui` schema molecules** in `ui-shared` (≤ 400 Prod LOC total), then rewrite every inspector panel as a **compact declarative field table** calling `runtime.commit(op)` directly.

---

## 2. Sub-Track Partition (100% File-Disjoint After 3A Lands)

Execute **3A first** (tiny foundation), then fan out **3B, 3C+3D, 3E+3F** in parallel worktrees alongside Track 4 (never exceeding 4 active worktrees).

**Gate Model (Updated):**
- **Hard Gate:** Delete every Mandatory Deletion Manifest path, remove all `Draft*` compat helpers and 4-layer mount/controller/projection stacks, keep `net Production Rust < 0` and `net Test Rust < 0` vs. the track base, and **preserve 100% of user-facing [PRODUCT.md](../../../PRODUCT.md) features, specialist controls, saved parameters, and pure-Rust domain unit tests**.
- **Advisory Targets:** The ceilings and deltas below are stretch benchmarks to verify boilerplate elimination — **never** drop user-facing features or valid domain tests solely to cross a line-count number.

| Sub-Track | Owned Paths | `dev` LOC (Prod / Test) | Advisory Target Total | Advisory Net Delta |
|---|---|---:|---:|---:|
| **3A** `ui-shared` molecules | `web/crates/ui-shared/src/inspector_fields.rs`, `choice_field.rs`, `footprint_forms.rs` | 14,406 *(7,549 / 6,857)* | ≤ 10,500 *(delete DOM mount bloat)* | **−3,900** |
| **3B** `case` inspector | `web/crates/case/src/{mechanical_settings*,mechanical_rules.rs,mechanical_pair*,mechanical_session_tests.rs,case_bodies.rs,case_controller.rs}` *(exclude `shared_viewer`/`case_viewer`/`cad_presentation.rs` → Track 6)* | ~24,400 *(13,800 / 10,600)* | ≤ 9,000 | **−15,400** |
| **3C** `parts` inspector | `web/crates/parts/**` | 23,559 *(14,197 / 9,362)* | ≤ 10,000 | **−13,500** |
| **3D** `pcb` inspector | `web/crates/pcb/**` | 16,068 *(9,172 / 6,896)* | ≤ 7,500 | **−8,500** |
| **3E** `keycaps` + `keymap` + `library` | `web/crates/keycaps/**`, `web/crates/keymap/**`, `web/crates/library/**` | 22,614 *(11,963 / 10,651)* | ≤ 10,500 | **−12,100** |
| **3F** `web/src/presentation` inspectors & shells | Non-layout files in `web/src/presentation/**` *(exclude `presentation.rs`, `editor*`, `layout_*`, `inspector/layout_component_inspector*` → Track 4)* | ~19,000 *(12,500 / 6,500)* | ≤ 9,500 | **−9,500** |
| **Track 3 Total** | — | **~120,000** | **≤ 57,000** | **~ −63,000** |

*Note: Check `check_reduction.py` deltas for 3B–3F against the Wave 2 post-3A base commit (`e5035558`). Track 2 already deleted queued/settlement boilerplate in Wave 1.*

---

## 3. Sub-Track 3A: Declarative `rust-ui` Schema Molecules (`≤ 400 Prod LOC`)

In `web/crates/ui-shared/src/inspector_fields.rs`, provide **exactly four** composable, cross-target (host + `wasm32`) `rust-ui` components built on `ui::{Field, InputGroup, Input, Select, Checkbox, Collapsible, Tooltip}`:

1. `NumericField` — Props: `label`, `unit` (`"mm"`, `"°"`, `"u"`), `value: f64`, `min`, `max`, `step`, `precision`, `placeholder` (for inherited/mixed), `disabled_reason: Option<String>`, `on_commit: EventHandler<f64>`.
   - Handles local text buffer while focused, Enter/blur parse + clamp + synchronous `on_commit`, Escape reverts to `value`. Total implementation: **~110 LOC**.
2. `SelectField<T>` — Props: `label`, `value: T`, `options: Vec<(T, String)>`, `disabled_reason`, `on_commit: EventHandler<T>`. Total: **~80 LOC**.
3. `ToggleField` — Props: `label`, `checked: bool`, `disabled_reason`, `on_commit: EventHandler<bool>`. Total: **~50 LOC**.
4. `InspectorSection` — Collapsible `rust-ui` card/section header with optional badge & reset button. Total: **~60 LOC**.

**Delete:**
- `web/crates/ui-shared/src/choice_field.rs` (fold into `SelectField`)
- `web/crates/ui-shared/src/footprint_forms.rs` (replace callers with `NumericField`/`SelectField`)
- Redundant browser test boilerplate in `ui-shared` (`toolbar_toggle_slider_toast_tests.rs`, `action_menu_tests.rs` → move pure checks to concise native `VirtualDom` tests, **−2,500+ Test LOC**).

---

## 4. Per-Crate Collapse Rules, Feature Preservation & Mandatory Deletions

### Sub-Track 3B: `web/crates/case` (Declarative Collapse With Full Feature Parity)
- **Delete completely (`--must-delete`):**
  - `web/crates/case/src/mechanical_settings_mount.rs` & `mechanical_settings_mount/`
  - `web/crates/case/src/mechanical_settings_controller.rs` & `mechanical_settings_controller/`
  - `web/crates/case/src/mechanical_session_tests.rs`, `mechanical_feedback.rs`, `examples/settings.rs`
  - Mounted DOM boilerplate (`tests_mounted.rs`, `tests_flows.rs`, `tests_contextual.rs`)
- **Rearchitect declaratively (ALL user-facing capabilities preserved):**
  - `mechanical_settings.rs` — build directly on `ui_shared::inspector_fields::{NumericField, TextCommitField, SelectField, ToggleField, InspectorSection}` + synchronous `Runtime::commit`, preserving ALL user-facing sections (Construction, Dimensions, Clearances, Mounts, Closure mounts & closure hardware, Hardware, Openings, Battery, Profile geometry & manufacturing editors, Stabilizer fits, Process targets, Critical fits, Layer/stack inspector, Gasket support editing & body-level gasket placement previews, Findings).
  - `pair_controls.rs` — full mirrored-pair lifecycle (reviewed **Link** flow alongside Make Local, Rejoin, and Unlink).
  - `case_controller.rs` / `case_bodies.rs` / `case_collision.rs` — direct declarative field bindings preserving `[data-case-collision-findings]`, `[aria-label="Collision check"]`, and `[aria-label="Witness identity"]`.
  - Keep concise pure-Rust unit tests covering mechanical rules, pair projection/link flow, and case body validation.
- **Anti-bloat floor & Advisory Target:** Hard `--max-prod-delta -1 --max-test-delta -1`; advisory stretch `--target-prod-delta -7500 --target-test-delta -4800`.

### Sub-Track 3C: `web/crates/parts`
- **Delete completely (`--must-delete`):**
  - `web/crates/parts/src/parts_definition_name` (fold rename directly into `TextCommitField` + commit)
  - Separate `*_editor.rs` / `*_ui.rs` duplicate layers, internal controller/mount/state-bag boilerplate, and bloated DOM mounted tests (`parts/generator_settings/tests.rs`, etc.).
- **Feature Preservation (NON-NEGOTIABLE):** Rearchitect ALL user-facing Parts workflows declaratively on `ui-shared::inspector_fields` — assembly selector & presets, mechanical profile parameters, footprint catalog, custom footprint editor, STEP/footprint import, generator parameter schema rendering, pinout/module inspector, preview, and placement — plus pure-Rust domain unit tests.
- **Anti-bloat floor & Advisory Target:** Hard `--max-prod-delta -1 --max-test-delta -1`; combined 3C+3D advisory stretch `--target-prod-delta -12500 --target-test-delta -7500`.

### Sub-Track 3D: `web/crates/pcb`
- **Collapse & Feature Preservation:** Flatten multi-layer draft/controller/settlement-polling stacks across `pcb_wiring*`, `pcb_physical_setup*`, `pcb_module_inspector*`, and `pcb_board_reference*` into clean declarative `ui-shared` field sections calling `Runtime::commit` directly, while keeping `pcb_scene.rs` SVG rendering crisp and preserving **100% of PCB capabilities** (physical setup, board reference, wiring editor, net/pin assignment & destructive remap confirmation, module/pinout inspector, ZMK/KiCad controls) and pure-Rust wiring/net/pin unit tests.

### Sub-Track 3E: `web/crates/keycaps` + `keymap` + `library`
- **Collapse & Feature Preservation:** Collapse `keycaps_settings*`, `keycaps_fit*`, `keycaps_navigation.rs`, `keymap/` wrapper controllers (`*_controller.rs`, redundant `*_editor.rs`), and `library/mounted_tests/` into declarative `ui-shared` components with pure-Rust domain tests, preserving **every user-facing workflow**: keycap profile/inherited fit & settings editing, navigation/scene overlays, keymap layer list, binding variant picker, encoder targets, macro step editor, library import/open/rename/delete/export actions, and demo projects.
- **Anti-bloat floor & Advisory Target:** Hard `--max-prod-delta -1 --max-test-delta -1`; combined 3E+3F advisory stretch `--target-prod-delta -11500 --target-test-delta -7000`.

### Sub-Track 3F: `web/src/presentation/` Non-Layout Inspectors & Shells
- **Collapse & Feature Preservation:** Remove prop-forwarding glue and legacy wrapper layers in non-layout `web/src/presentation/` files (`inspector*`, `case_workspace*`, `export_workspace.rs`, `zmk_firmware_export.rs`, `command_palette.rs`, `keycaps_workspace.rs`, `keymap_workspace.rs`, `parts_workspace.rs`, `pcb_workspace.rs`, `setup_*`, `header.rs`, `workbench.rs`) using `ui-shared::ui::*` directly — **preserving every user-facing workspace panel, command palette action, ZMK/KiCad/STEP export option, header save indicator (`.header-save`), `[aria-label="Case assembly"]`, and `[aria-label="Witness identity"]`**.

---

## 5. Banned Patterns (Automatic Rejection)

- **DO NOT** keep `*_mount.rs` + `*_controller.rs` + `*_projection.rs` + `*_view.rs` 4-file stacks for simple forms. Every form section MUST go directly from `&ResolvedDoc` → `NumericField`/`TextCommitField`/`SelectField`/`ToggleField` → `runtime.commit(op)`.
- **DO NOT** silently drop user-facing features, specialist sections, or saved-parameter controls to hit an advisory LOC number.
- **DO NOT** add new `#[wasm_bindgen_test]` mounted tests for field parsing, clamping, or enable/disable rules — test those natively in `#[test]`.
- **DO NOT** leave temporary `Draft*` compat wrappers (`DraftNumericField`, `DraftSelectField`, etc.) behind in `ui-shared` after migrating callers.

---

## 6. Verification & Mechanical Gate Commands

Run `check_reduction.py` against the post-Wave-1 base commit (`0c41526d0`):

```bash
# Track 3 combined gate against 0c41526d0 (hard anti-bloat floor + advisory targets):
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base 0c41526d0 \
  --head HEAD \
  --max-prod-delta -1 \
  --max-test-delta -1 \
  --target-prod-delta -35000 \
  --target-test-delta -33000 \
  --max-script-delta 0 \
  --max-other-delta 200 \
  --must-delete web/crates/case/src/mechanical_settings_mount.rs \
  --must-delete web/crates/case/src/mechanical_settings_mount \
  --must-delete web/crates/case/src/mechanical_settings_controller.rs \
  --must-delete web/crates/parts/src/parts_definition_name \
  --must-delete web/crates/ui-shared/src/choice_field.rs \
  --must-delete web/crates/ui-shared/src/footprint_forms.rs \
  --forbid-symbol DraftNumericField \
  --forbid-symbol DraftSelectField \
  --forbid-symbol mechanical_settings_mount

# Focused checks per crate:
cargo fmt --all --check
cargo test -p boardstudio-web-ui-shared -p boardstudio-web-case -p boardstudio-web-parts \
  -p boardstudio-web-pcb -p boardstudio-web-keycaps -p boardstudio-web-keymap -p boardstudio-web-library
cargo check -p boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page
```
