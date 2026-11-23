# Track 4: Collapse Layout Canvas, Transform Toolbar & Matrix Controllers

**Base:** Merged Wave 1 (`dev` + Track 1 + Track 2); can run in parallel with Track 3 (100% file-disjoint after Track 3A `ui-shared` lands).  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely.

**Owned Surfaces:**
- `web/crates/layout-objects/**`
- `web/crates/layout/**`
- `web/crates/layout-core/**`
- `web/src/presentation/editor/**`
- `web/src/presentation/layout_*` (`layout_workspace.rs`, `layout_canvas*`, `layout_component_*`, `layout_key_selection_actions*`, `layout_matrix_relations.rs`, `layout_*_tests.rs`)

---

## 1. Why Wave 1 Failed Here (And What Track 4 Does Differently)

In `refactor/simplify-implementation-wave`:
- `web/crates/layout-core` **grew +1,244 LOC** (adding `canvas_pointer.rs`),
- `web/crates/layout-objects` **grew +565 LOC** (`layout_transform_toolbar/` stayed at **10,919 LOC**, *+22*; `matrix_transform*` **grew +413 LOC**),
- `scripts/` gained ~1,200 LOC of Canvas2D trial scripts only to conclude SVG was already fast enough.

**Track 4's Rule:**
1. **Keep SVG** (4.55 µs hit queries, 16.67 ms frame pacing — **ZERO** Canvas2D or WebGL2 2D trials!).
2. **Delete the entire 23-file `layout_transform_toolbar/` directory (10,735 LOC)** and replace it with ONE pure-Rust `CanvasInteraction` state machine in `layout-core` (**≤ 1,500 Prod LOC**) + ONE SVG pointer/keyboard adapter in `web/src/presentation/editor/` (**≤ 500 Prod LOC**).
3. **Delete the 4-layer matrix controller/inspector stack** (`matrix_transform_controller*` + `matrix_transform_inspector*` + `matrix_inspector_controller*` + `matrix_inspector*` = ~14,000 LOC) and replace with declarative Track-3A `ui-shared` fields + ONE pure matrix transform helper (**≤ 2,500 LOC**).

---

## 2. Baseline, Hard Architectural Gate & Advisory LOC Targets

| Surface on `dev` (`5aae86ff4`) | Prod LOC | Test LOC | Total LOC | Advisory Stretch Ceiling |
|---|---:|---:|---:|---:|
| `web/crates/layout-objects/` (84 files) | 24,688 | 13,794 | 38,482 | ≤ 11,500 |
| `web/crates/layout/` (31 files) | 8,825 | 7,941 | 16,766 | ≤ 6,500 |
| `web/crates/layout-core/` (39 files) | 7,797 | 6,010 | 13,807 | ≤ 6,500 |
| `web/src/presentation/{editor*,layout_*}` | ~13,500 | ~6,500 | ~20,000 | ≤ 8,000 |
| **Total Track 4 Footprint** | **~54,810** | **~34,245** | **~89,055** | **≤ 32,500** |

### Hard Gate + Advisory Stretch Targets (Whole-Repo `check_reduction.py` Gate vs. Wave Base)
- **Hard Anti-Bloat Floor:** `--max-prod-delta -1 --max-test-delta -1 --max-script-delta 0 --max-other-delta 200` (plus ALL Mandatory Deletion Manifest paths deleted, zero `Draft*` / 4-layer controller stacks, single `CanvasInteraction` state machine + single SVG canvas adapter, and full Layout feature parity).
- **Advisory Stretch Targets:** `--target-prod-delta -31000 --target-test-delta -22000` (report actual deltas alongside feature parity; never drop Layout capabilities or domain invariants to hit the stretch numbers).

---

## 3. Target Replacement Architecture & Feature Preservation

### A. One Pure-Rust `CanvasInteraction` State Machine (`web/crates/layout-core/src/interaction.rs`)
Handles **every** Layout canvas gesture in pure, host-testable Rust with **zero** `web_sys` or Dioxus dependency:
- **Normalized Input Events:** `PointerDown { id, pos_mm, screen_px, button, modifiers, hit }`, `PointerMove`, `PointerUp`, `PointerCancel`, `KeyDown { key, modifiers }`, `Blur`, `ScopeChanged`.
- **Single Explicit State Enum:**
  - `Idle`
  - `Panning { origin_px, start_camera }`
  - `MarqueeSelecting { origin_mm, current_mm, additive }`
  - `DraggingParts { ids, origin_mm, snap, preview_deltas }`
  - `DraggingMatrixRail { matrix_id, axis, origin_mm, preview }`
  - `Rotating { targets, pivot_mm, start_angle, preview_angle }`
  - `PickingAnchor { source_id }`
  - `PlacingPart { def_id, cursor_mm }`
  - `DraggingOutlineHandle { outline_id, handle, origin_mm, preview }`
- **Pure Outputs:** `InteractionEffect` (`SetCamera`, `SetSelection`, `PreviewOp(EditOperation)`, `CommitOp(EditOperation)`, `CancelPreview`, `CapturePointer(id)` / `ReleasePointer(id)`).
- **Preserved Pure-Rust Invariants (`#[test]`):**
  - Second pointer ignored cleanly without corrupting active drag.
  - Lost pointer capture, window blur, `Escape`, and scope/mode switch cancel preview cleanly with zero stuck state.
  - Commit produces a single synchronous `EditOperation` with exact Undo.
  - Snapping, grid alignment, keyboard nudge (arrow keys + Shift), matrix transform math, outline editing, and placement invariants remain thoroughly covered by pure-Rust unit tests (no arbitrary LOC cap stripping valid invariant tests).

### B. One Mounted SVG Surface Adapter (`web/src/presentation/editor/canvas_adapter.rs`)
- Single `<svg>` container listening to `onpointerdown`, `onpointermove`, `onpointerup`, `onpointercancel`, `onwheel`, `onkeydown`, `onblur`.
- Performs pointer capture (`set_pointer_capture` / `release_pointer_capture`), converts client coordinates to board `mm` via the active `Camera`, resolves `data-pick-id` hit target once, and feeds `CanvasInteraction::handle(...)`.
- Renders transient visual overlays (marquee rectangle, snap guides, rotation pivot, active drag highlight) directly from `CanvasInteraction` state.
- **Zero** `web_sys::Document` `Closure::wrap` listener leaks, **zero** `thread_local!` mount registries, **zero** per-tool pointer handlers.

### C. Collapsed Declarative Layout Inspectors & Feature Parity
Replace the fragmented 4-layer controllers with concise declarative `ui-shared` components while **preserving ALL user-facing Layout capabilities** (matrix/row/col/key properties, splay/stagger/column-offset handles, anchor follow & anchor picking/badges, part & companion pose/rotation/mirror actions, outline & gap parameters + contour/overlay editing, keycap size/resize controls, alignment tools, setup guide, findings navigation, selection/undo/Escape workflows, and every `scripts/run-drag-browser.py` selector):
- `matrix_inspector.rs`: Matrix, column, row, key properties, splay, stagger, and anchor-follow using `NumericField`/`TextCommitField`/`SelectField`/`ToggleField` directly against `&ResolvedDoc`.
- `part_inspector.rs`: Selected part/companion pose, rotation, mirror ownership actions (`Make Local` / `Rejoin` / `Omit` / companion placement).
- `outline_inspector.rs`: Outline & gap parameters + contour editing hooks & overlays.
- `keycap_size_inspector.rs`: Keycap size & interactive resize controls.
- `align_and_setup.rs`: Alignment actions, matrix setup presets, and mirrored-pair actions.

---

## 4. Mandatory Deletion Manifest

Every single path below MUST be **completely deleted** (`git ls-files` count = 0):

- `web/crates/layout-objects/src/objects/layout_transform_toolbar/` *(all 23 files, 10,735 LOC)*
- `web/crates/layout-objects/src/objects/layout_transform_toolbar.rs` *(162 LOC)*
- `web/crates/layout-objects/src/objects/matrix_transform_controller.rs` *(1,218 LOC)* & `matrix_transform_controller/` *(668 LOC)*
- `web/crates/layout-objects/src/objects/matrix_transform_inspector.rs` *(1,005 LOC)* & `matrix_transform_inspector/` *(1,705 LOC)*
- `web/crates/layout-objects/src/objects/matrix_transform_inspector_tests.rs` *(1,131 LOC)* & `matrix_transform_inspector_tests/` *(1,465 LOC)*
- `web/crates/layout-objects/src/objects/matrix_inspector_controller.rs` *(1,144 LOC)* & `matrix_inspector_controller/` *(2,045 LOC)*
- `web/crates/layout-objects/src/objects/keycap_size_tests.rs` *(1,101 LOC)*
- `web/crates/layout/src/outline_lifecycle_browser_tests.rs` *(1,368 LOC)* & `outline_lifecycle_browser_tests/` *(981 LOC)*
- `web/crates/layout/src/outline_lifecycle_tests.rs` *(1,125 LOC)*
- `web/src/presentation/layout_component_inspector_tests/` *(2,369 LOC)* & `layout_component_inspector_tests.rs` *(772 LOC)*
- `web/src/presentation/layout_relation_highlight_tests.rs` *(1,192 LOC)*

---

## 5. Banned Patterns (Automatic Rejection)

- **DO NOT** build Canvas2D or WebGL2 2D renderer prototypes, observers, or benchmark scripts.
- **DO NOT** add `canvas_pointer.rs` while leaving `layout_transform_toolbar/` alive beside it.
- **DO NOT** keep separate `*_controller.rs`, `*_logic.rs`, `*_projection.rs`, and `*_inspector.rs` files for a 5-field UI panel.
- **DO NOT** break the packaged trusted drag + Undo + Escape + Case collision smoke (`scripts/run-drag-browser.py`).

---

## 6. Verification & Mechanical Gate Commands

```bash
# 1. Mechanical Reduction Gate (MUST exit 0 against <WAVE_BASE_SHA>)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base <WAVE_BASE_SHA> \
  --head HEAD \
  --max-prod-delta -1 \
  --max-test-delta -1 \
  --target-prod-delta -31000 \
  --target-test-delta -22000 \
  --max-script-delta 0 \
  --max-other-delta 200 \
  --must-delete web/crates/layout-objects/src/objects/layout_transform_toolbar \
  --must-delete web/crates/layout-objects/src/objects/layout_transform_toolbar.rs \
  --must-delete web/crates/layout-objects/src/objects/matrix_transform_controller.rs \
  --must-delete web/crates/layout-objects/src/objects/matrix_transform_inspector \
  --must-delete web/crates/layout-objects/src/objects/matrix_transform_inspector_tests \
  --must-delete web/crates/layout-objects/src/objects/matrix_inspector_controller \
  --must-delete web/crates/layout/src/outline_lifecycle_browser_tests \
  --must-delete web/src/presentation/layout_component_inspector_tests

# 2. Focused checks (inside worktree)
cargo fmt --all --check
cargo test -p boardstudio-web-layout-core -p boardstudio-web-layout -p boardstudio-web-layout-objects
cargo check -p boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page
```
*(Integration runs full `scripts/check.py` including `scripts/run-drag-browser.py` on both the 2×2 default smoke and saved 4×6 collision fixture.)*
