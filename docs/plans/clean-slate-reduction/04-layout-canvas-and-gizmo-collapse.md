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

## 2. Baseline vs. Hard LOC Budget

| Surface on `dev` (`5aae86ff4`) | Prod LOC | Test LOC | Total LOC | Target Ceiling |
|---|---:|---:|---:|---:|
| `web/crates/layout-objects/` (84 files) | 24,688 | 13,794 | 38,482 | ≤ 9,500 |
| `web/crates/layout/` (31 files) | 8,825 | 7,941 | 16,766 | ≤ 5,500 |
| `web/crates/layout-core/` (39 files) | 7,797 | 6,010 | 13,807 | ≤ 6,500 |
| `web/src/presentation/{editor*,layout_*}` | ~13,500 | ~6,500 | ~20,000 | ≤ 6,500 |
| **Total Track 4 Footprint** | **~54,810** | **~34,245** | **~89,055** | **≤ 28,000** |

### Hard Budget (Whole-Repo `check_reduction.py` Gate vs. Post-Wave-1 Base)
- **Mandatory Net Production Rust Delta:** **`<= -32,000` LOC**
- **Mandatory Net Test Rust Delta:** **`<= -25,000` LOC**
- **Mandatory Net Scripts Delta:** **`<= 0` LOC**
- **Mandatory Net Other Delta:** **`<= +200` LOC**

---

## 3. Target Replacement Architecture

### A. One Pure-Rust `CanvasInteraction` State Machine (`web/crates/layout-core/src/interaction.rs`, ≤ 1,500 Prod LOC)
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
- **Invariants pinned in ≤ 500 LOC of native `#[test]`:**
  - Second pointer ignored cleanly without corrupting active drag.
  - Lost pointer capture, window blur, `Escape`, and scope/mode switch cancel preview cleanly with zero stuck state.
  - Commit produces a single synchronous `EditOperation` with exact Undo.
  - Snapping, grid alignment, and keyboard nudge (arrow keys + Shift) run through the exact same math.

### B. One Mounted SVG Surface Adapter (`web/src/presentation/editor/canvas_adapter.rs`, ≤ 500 Prod LOC)
- Single `<svg>` container listening to `onpointerdown`, `onpointermove`, `onpointerup`, `onpointercancel`, `onwheel`, `onkeydown`, `onblur`.
- Performs pointer capture (`set_pointer_capture` / `release_pointer_capture`), converts client coordinates to board `mm` via the active `Camera`, resolves `data-pick-id` hit target once, and feeds `CanvasInteraction::handle(...)`.
- Renders transient visual overlays (marquee rectangle, snap guides, rotation pivot, active drag highlight) directly from `CanvasInteraction` state.
- **Zero** `web_sys::Document` `Closure::wrap` listener leaks, **zero** `thread_local!` mount registries, **zero** per-tool pointer handlers.

### C. Collapsed Declarative Layout Inspectors (`≤ 6,000 Prod LOC` across `layout-objects` + `layout`)
Replace the fragmented 4-layer controllers with concise declarative `ui-shared` components:
- `matrix_inspector.rs` (≤ 1,200 LOC): Matrix, column, row, key properties, splay, stagger, and anchor-follow using `NumericField`/`SelectField`/`ToggleField` directly against `&ResolvedDoc`.
- `part_inspector.rs` (≤ 800 LOC): Selected part/companion pose, rotation, mirror ownership actions (`Make Local` / `Rejoin` / `Omit` from Track 1).
- `outline_inspector.rs` (≤ 1,000 LOC): Outline & gap parameters + contour editing hooks (replaces `layout/src/outline/` 5,168 LOC + `outline_lifecycle_*tests` 3,475 LOC).
- `keycap_size_inspector.rs` (≤ 500 LOC): Replaces `keycap_resize*` + `keycap_size*` (4,154 LOC).
- `align_and_setup.rs` (≤ 800 LOC): Replaces `layout_align*` (1,769 LOC) + `matrix_setup*` (1,216 LOC) + `mirrored_pair*` (1,730 LOC) + `existing_half.rs` (926 LOC).

*(Optional bonus in Track 4C: merge `boardstudio-web-layout-objects` into `boardstudio-web-layout` if compile-graph timings remain flat or faster, eliminating cross-crate re-export glue.)*

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
# 1. Mechanical Reduction Gate (MUST exit 0 against <WAVE1_SHA>)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base <WAVE1_SHA> \
  --head HEAD \
  --max-prod-delta -32000 \
  --max-test-delta -25000 \
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
