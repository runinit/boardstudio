# Track 3: Collapse Monolithic Workspace Inspectors into Declarative `rust-ui` Specs

**Base:** Merged Wave 1 (`dev` + Track 1 + Track 2).  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely. Track 2's synchronous `runtime.commit(op)` MUST be landed first so inspectors require zero async settlement plumbing.

---

## 1. Why Wave 1 Failed Here (And What Track 3 Does Differently)

In `refactor/simplify-implementation-wave`, `web/crates/case/src/mechanical_settings*` shrank by only **−753 LOC out of 12,975 LOC (−5.8%)**. Why? Because it swapped individual HTML `<input>` tags for `rust-ui` inputs while **preserving all 4 hand-wired indirection layers** (`mechanical_settings_mount.rs` → `mechanical_settings_controller.rs` → `mechanical_settings_mount/projection.rs` → `mechanical_settings/*.rs` + 4,500 LOC of per-field mounted browser tests).

**Track 3's Rule:** Delete the 4-layer indirection stack in every workspace crate. Build **4 tiny, stateless declarative `rust-ui` schema molecules** in `ui-shared` (≤ 400 Prod LOC total), then rewrite every inspector panel as a **compact declarative field table** calling `runtime.commit(op)` directly.

---

## 2. Sub-Track Partition (100% File-Disjoint After 3A Lands)

Execute **3A first** (tiny 1-commit foundation), then fan out **3B, 3C, 3D, 3E, 3F** in up to 4 parallel worktrees:

| Sub-Track | Owned Paths | `dev` LOC (Prod / Test) | Hard Ending Ceiling (Total) | Min Net Reduction |
|---|---|---:|---:|---:|
| **3A** `ui-shared` molecules | `web/crates/ui-shared/src/inspector_fields.rs`, `choice_field.rs`, `footprint_forms.rs` | 14,406 *(7,549 / 6,857)* | ≤ 10,500 *(delete bloat tests)* | **−3,900** |
| **3B** `case` inspector | `web/crates/case/src/mechanical_settings*`, `mechanical_rules.rs`, `mechanical_pair*`, `mechanical_session_tests.rs`, `case_bodies.rs`, `case_controller.rs` *(exclude `shared_viewer`/`case_viewer` → Track 6)* | ~24,400 *(13,800 / 10,600)* | ≤ 6,000 | **−18,400** |
| **3C** `parts` inspector | `web/crates/parts/**` | 23,559 *(14,197 / 9,362)* | ≤ 8,000 | **−15,500** |
| **3D** `pcb` inspector | `web/crates/pcb/**` | 16,068 *(9,172 / 6,896)* | ≤ 6,000 | **−10,000** |
| **3E** `keycaps` + `keymap` + `library` | `web/crates/keycaps/**`, `web/crates/keymap/**`, `web/crates/library/**` | 22,614 *(11,963 / 10,651)* | ≤ 8,500 | **−14,100** |
| **3F** `web/src/presentation` inspectors & shells | `web/src/presentation/{inspector*,case_workspace*,export_workspace.rs,command_palette.rs,keycaps_workspace.rs,keymap_workspace.rs,parts_workspace.rs,setup_*}` *(exclude `editor*`/`layout*` → Track 4)* | ~19,000 *(12,500 / 6,500)* | ≤ 7,500 | **−11,500** |
| **Track 3 Total** | — | **~120,000** | **≤ 46,500** | **≥ −73,400** |

*Note: Exact `check_reduction.py` deltas for 3B–3E are measured against the post-Wave-1 base (where Track 2 already deleted ~4.3k of queued/settlement test files in these crates; adjust each sub-track's `--max-test-delta` accordingly so the cumulative Track 3 reduction hits ≥ −73,000 LOC).*

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

## 4. Per-Crate Collapse Rules & Mandatory Deletion Manifests

### Sub-Track 3B: `web/crates/case` (`mechanical_settings*`: 12,975 → ≤ 2,500 LOC)
- **Delete completely:**
  - `web/crates/case/src/mechanical_settings_mount.rs` *(841 LOC)* & `mechanical_settings_mount/` *(882 LOC)*
  - `web/crates/case/src/mechanical_settings_controller.rs` *(763 LOC)* & `mechanical_settings_controller/` *(1,162 LOC)*
  - `web/crates/case/src/mechanical_settings/tests_mounted.rs`, `tests_flows.rs`, `tests_contextual.rs` *(~3,400 LOC)*
  - `web/crates/case/src/mechanical_session_tests.rs` *(1,388 LOC)*
  - Split `mechanical_settings/` 11-file maze (9,219 LOC) → replace with **3 clean files** (≤ 1,800 Prod LOC + ≤ 600 Native Test LOC):
    - `mechanical_settings.rs` — declarative sections (Construction, Dimensions, Clearances, Mounts, Hardware, Openings, Battery) using `NumericField`/`SelectField`/`ToggleField` directly on `&MechanicalSettings` → `runtime.commit(EditOperation::UpdateMechanicalSettings(...))`.
    - `pair_controls.rs` — mirrored-pair actions (Make Local / Rejoin / Unlink).
    - `case_controller.rs` — slimmed body/export/CAD trigger (≤ 600 LOC).
- **Hard Sub-Track 3B Budget:** Prod `<= -8,500`, Test `<= -8,000`.

### Sub-Track 3C: `web/crates/parts` (23,559 → ≤ 8,000 LOC)
- **Delete completely:**
  - `web/crates/parts/src/parts_definition_name/` *(1,881 LOC — replace with a 40-line `TextField` + rename commit)*
  - `web/crates/parts/src/parts/generator_settings/tests.rs` *(1,282 LOC → replace with ≤ 250 LOC native schema tests)*
  - Separate `*_editor.rs` / `*_ui.rs` duplicate splits (`mechanical_profile.rs` + `mechanical_profile_editor.rs` + `mechanical_profile_ui` → ONE file ≤ 700 LOC; `assembly_editor.rs` + `assembly_presets.rs` → ONE file ≤ 700 LOC; `parts_custom_definition.rs` + `parts_custom_definition/` 2,069 LOC → ≤ 700 LOC).
  - Keep specialized workflows intact (STEP/footprint import, generator parameter schema rendering, preview, placement) — just collapse form boilerplate and per-field mounted DOM tests.
- **Hard Sub-Track 3C Budget:** Prod `<= -7,500`, Test `<= -6,500`.

### Sub-Track 3D: `web/crates/pcb` (16,068 → ≤ 6,000 LOC)
- **Collapse:**
  - `pcb_wiring/` (16 files, 6,832 LOC) + `pcb_wiring.rs` (1,244 LOC) → ≤ 2,500 LOC (keep net/pin assignment & destructive remap confirmation; delete multi-layer draft/controller indirection and duplicated tests).
  - `pcb_physical_setup/` (1,669 LOC) + `pcb_module_inspector*` (1,882 LOC) + `pcb_board_reference*` (1,680 LOC) → declarative `ui-shared` fields (≤ 1,800 LOC combined).
  - Keep `pcb_scene.rs` SVG rendering clean and compact (≤ 600 LOC).
- **Hard Sub-Track 3D Budget:** Prod `<= -5,000`, Test `<= -4,000`.

### Sub-Track 3E: `web/crates/keycaps` + `keymap` + `library` (22,614 → ≤ 8,500 LOC)
- `keycaps`: Collapse `keycaps_settings.rs` + `keycaps_settings/` (3,061 → ≤ 800 LOC) using declarative `NumericField`/`SelectField` with inherited placeholder support; slim `keycaps_fit*` (2,104 → ≤ 900 LOC) and `keycaps_navigation.rs` (1,236 → ≤ 500 LOC).
- `keymap`: Collapse `keymap/` (13 files, 7,742 → ≤ 2,500 LOC) — keep layer list, binding variant picker, encoder targets, and macro step editor; delete redundant wrapper controllers and mounted DOM boilerplate.
- `library`: Collapse `library/mounted_tests/` (2,067 LOC!) and `library.rs` (1,320 → ≤ 700 LOC) — keep import/open/rename/delete/export commands and ≤ 300 LOC of focused tests.
- **Hard Sub-Track 3E Budget:** Prod `<= -6,000`, Test `<= -6,500`.

### Sub-Track 3F: `web/src/presentation/` Non-Layout Inspectors & Shells
- Remove redundant inspector wrapper glue in `web/src/presentation/inspector*` (3,085 → ≤ 800 LOC), `case_workspace*` (2,097 → ≤ 800 LOC), `export_workspace.rs` (793 → ≤ 400 LOC), and `command_palette.rs` (817 → ≤ 400 LOC).
- Delete all remaining raw HTML `<button>`, `<input>`, `<select>`, `<textarea>` stragglers in favor of `ui-shared::ui::*`.
- **Hard Sub-Track 3F Budget:** Prod `<= -5,500`, Test `<= -4,500`.

---

## 5. Banned Patterns (Automatic Rejection)

- **DO NOT** keep `*_mount.rs` + `*_controller.rs` + `*_projection.rs` + `*_view.rs` 4-file stacks for simple forms. Every form section MUST go directly from `&ResolvedDoc` → `NumericField`/`SelectField`/`ToggleField` → `runtime.commit(op)`.
- **DO NOT** add new `#[wasm_bindgen_test]` mounted tests for field parsing, clamping, or enable/disable rules — test those natively in `#[test]`.
- **DO NOT** leave unused `ui-shared` wrappers behind after migrating callers.

---

## 6. Verification & Mechanical Gate Commands

Run `check_reduction.py` per sub-track against the post-Wave-1 base commit (`<WAVE1_SHA>`):

```bash
# Example for Track 3 combined gate against <WAVE1_SHA>:
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base <WAVE1_SHA> \
  --head HEAD \
  --max-prod-delta -35000 \
  --max-test-delta -33000 \
  --max-script-delta 0 \
  --max-other-delta 200 \
  --must-delete web/crates/case/src/mechanical_settings_mount.rs \
  --must-delete web/crates/case/src/mechanical_settings_mount \
  --must-delete web/crates/case/src/mechanical_settings_controller.rs \
  --must-delete web/crates/case/src/mechanical_settings_controller \
  --must-delete web/crates/case/src/mechanical_session_tests.rs \
  --must-delete web/crates/parts/src/parts_definition_name \
  --must-delete web/crates/ui-shared/src/choice_field.rs \
  --must-delete web/crates/ui-shared/src/footprint_forms.rs

# Focused checks per crate:
cargo fmt --all --check
cargo test -p boardstudio-web-ui-shared -p boardstudio-web-case -p boardstudio-web-parts \
  -p boardstudio-web-pcb -p boardstudio-web-keycaps -p boardstudio-web-keymap -p boardstudio-web-library
cargo check -p boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page
```
