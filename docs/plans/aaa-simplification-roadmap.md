# BoardStudio AAA Simplification & Clean-Slate Roadmap

**Baseline:** `dev` (`28e08ac7f`, plus Wave 4 `refactor/cold-build-dependency-graph`)  
**Target State:** 100% Rust + Dioxus (`rust-ui.com` component architecture), zero Monstertruck, synchronous main-thread Core edits, derived bilateral symmetry, native-first testing, and ~400,000+ fewer lines of code.

## Implementation checkpoint, 2026-11-21

The wave tables below record the original estimates. Use this checkpoint to choose the next work.

- **CAD.** Generated case and PCB geometry use the native mesher and analytical STEP exporter. Imported STEP uses patched cadmpeg in `cad/core`. Monstertruck is gone. The release corpus admitted all 75 former successes and two additional models. `KS33.stp` and `SK6812MINI-E v1.step` still refuse. The 49 bundled previews total 2,277,862 compressed bytes against a 2,883,584-byte budget. The CAD workspace has a separate lockfile. Applying the repository license policy to it still rejects Inflector through step-p21 and clipper2-rust through manifold-rust. Cold-build [ticket 07](cold-build-time/issues/07-inflector-regex.md) records approval for Inflector’s BSD-2-Clause license. The Boost license for clipper2-rust needs a policy decision.
- **Document edits.** The page already gives Runtime a main-thread document Core. The ticket and pending-edit registry files are gone. Preserve save-before-publication, save-only retry, and explicit recovery on unknown outcomes under [ADR-0013](../adr/0013-save-before-publication-and-explicit-recovery.md). Matching-preview commits met the original latency target in the paired run. Fresh previews retain the cost of the exact outline union. Further work should remove duplicated routing while keeping expensive archive and CAD work off the UI thread. The experimental worker-retirement branch moves ZIP operations onto the page.
- **Web and verification.** Catalogue and UI-model no longer need dedicated Chrome launches. The Space-pan listener runs in the page suite. Twelve crate launches remain alongside the page, drag and CAD checks. The latest shared-control sweep changed 20 fixture buttons and two production file inputs. For the next native batch, extract the pure Parts add-menu and PCB reassignment decisions from their WASM-only parents, verify native discovery by test name, and keep mounted interactions in Chrome.
- **Packaged smoke.** Level 3A uses the shared Python WebDriver client and one project for trusted drag, Undo, Escape and Case collision findings. The committed smoke uses a 2×2 matrix. The planned 4×6 scenario passed the drag checks and timed out after 120 seconds in the collision check. A worker trace recorded 1,941 collision replies and 1.69 GB of outgoing position and tag buffers before timeout. Measure the cost of sending both meshes for each pair before changing the worker protocol. Keep the collision observation, findings scroll and viewport assertions.
- **Mirrored halves.** Hosted edits, matrix membership and sparse exception transitions share more production logic. The before/after reconciler is still live. Before integrating the companion fix from 92030e7cd, derive key ownership from the paired matrix and slot. An independent Part can have a key-like ID prefix. Pin that decoy, the omitted-target refusal, and the exact Rejoin pose, no-op snapshot and exception count in the replacement tests. The next deletion should consolidate the exception lookup used by forward projection and inverse editing for keys and companions. Verify Local/Omitted/Linked from both authoring halves, exact Local payload, changed IDs, reopen and Undo/Redo before removing a duplicate implementation. Preserve separate input checks for Open and document replacement.

---

## 1. Architectural Pillars

1. **100% Rust + Dioxus Built on `rust-ui.com` Composition:**
   - Enable `dioxus-primitives` across both host and `wasm32` targets in `web/crates/ui-shared` so all 35 `rust-ui.com` components (`Select`, `Combobox`, `Dialog`, `DropdownMenu`, `Popover`, `Tabs`, `Toolbar`, `ToggleGroup`, `Collapsible`, `Checkbox`, `Slider`, `Tooltip`, `Field`, `InputGroup`, etc.) compile natively and in WASM.
   - Replace the 99 raw `button { ... }` / `input { ... }` / `select { ... }` stragglers across workspace crates with `boardstudio_web_ui_shared::ui::*`.
   - Build 4 higher-level composed `rust-ui` inspector molecules in `ui-shared` (`NumericField`, `EnumSelectField`, `BooleanToggleField`, `InspectorSection`) so 9,000-LOC hand-wired forms (`mechanical_settings.rs`, `matrix_transform_inspector.rs`, `generator_settings.rs`, `keycaps_settings.rs`) collapse into declarative property specs (~80% LOC reduction).
   - Move 2D SVG transform gizmos out of Dioxus VDOM component lifecycles (`layout_transform_toolbar.rs` 13.3k LOC) into a pure-Rust `CanvasController` state machine.

2. **Complete Replacement of Monstertruck (`-165,902 LOC`):**
   - **Phase 1 (Evict Qualification & Unused B-Rep Crates):** Remove `cad/monstertruck-qualification` (133,880 LOC of qualification harnesses, `monstertruck-solid`, T-splines, and marching-ssi) from the active workspace and `check.py` loop.
   - **Phase 2 (Direct 2.5D Case Mesher & STEP AP214 Writer):** `cad/monstertruck/src/layered.rs` already computes case and PCB bodies as 2.5D horizontal Z-bands (`Band { low, high, lower: Profile, upper: Profile }`) using 2D planar booleans. Replace Monstertruck's `Solid` builder, NURBS tessellator, and STEP serializer with:
     - Direct 2.5D Z-band triangulation (`i_overlay` / `lyon_tessellation` caps + analytical quad/cylinder/cone walls with exact face provenance, ~350 LOC).
     - Direct analytical ISO-10303-21 (`AP214`) STEP writer emitting `MANIFOLD_SOLID_BREP` (`PLANE`, `CYLINDRICAL_SURFACE`, `CONICAL_SURFACE`, `LINE`, `CIRCLE`) directly from `layered::Band` (~500 LOC).
   - **Phase 3 (Vendor STEP Import Replacement):** Pre-bake bundled `ergogen/library/` 3D component models at content-build time into compact binary mesh + `parry3d` collision assets, and replace `cad/monstertruck/src/step/` (and its 12,000 LOC of Monstertruck repair hacks) with a clean, standalone STEP AP214 reader/triangulator on top of `step-p21`. Delete `cad/monstertruck` and all `monstertruck-*` vendor crates completely.

3. **Synchronous Main-Thread `CoreEngine` (`-45,000 LOC`):**
   - Execute `CoreEngine::apply_edit` synchronously on the main thread (`<0.2 ms`).
   - Keep Web Workers strictly for heavy asynchronous jobs (3D CAD generation, 3D collision checks, IndexedDB persistence, ZIP export).
   - Delete `PendingEdits`, `PendingEditSignals`, `EditTicket`, and queued-field-settlement ceremony across all 7 workspaces.

4. **Derived Bilateral Symmetry Instead of Post-Hoc Tree Diffing (`-45,000 LOC`):**
   - Replace `core/crates/mirrored-half`'s 52,000-LOC post-hoc `before`/`after` `ProjectDoc` tree-diff reconciler with operation-level mirror forwarding and derived mirror projection over a sparse `MirrorOverride` map.

5. **Native-First Testing & Retiring the Baseline Floor Ratchet (`-150,000 LOC`):**
   - Remove the one-way test-count ratchet (`scripts/check-native-tests-baseline.json`) so redundant tests can be consolidated.
   - Move pure controller, view-model, and `rust-ui` component tree tests to fast native `#[test]` runs (using native Dioxus `VirtualDom` where needed), retiring the 14 per-crate headless-Chrome `wasm-bindgen-test` launches while keeping the packaged-page browser E2E smoke suite (`drag_smoke`, CAD browser, page E2E).
   - Replace 160-line copy-pasted ticket scenario tests in `core/` with `proptest` state-machine invariant suites.

---

## 2. Multi-LLM & Multi-Worktree Execution Waves (Quick Wins First)

Each wave is partitioned into **100% file-disjoint tracks** (`Worktree A`, `Worktree B`, `Worktree C`, ...) so multiple LLM delegates can execute in parallel worktrees and merge to `dev` with zero git conflicts.

---

### Wave 0: Land In-Flight Wave 4 (`refactor/cold-build-dependency-graph`)
- **Status:** All 4 slices (`W4-1`..`W4-4`) committed in worktree `t3-eff9fcab-5d94b7dd1e21ea50`; final `check.py` gate completing now.
- **Action:** Fast-forward `dev` to `refactor/cold-build-dependency-graph` as soon as the delegated run reports completion.

---

### Wave 1: Instant Dead-Weight Eviction (3 Parallel Worktrees)
*Goal: Delete ~140,000+ LOC of dead product scope, vendored qualification bloat, and test-ratchet friction in one parallel burst.*

| Track | Branch / Worktree | Owned Files (Disjoint) | Scope & Done Condition | Target Model | Est. Delta |
|---|---|---|---|---|---:|
| **W1-A: Delete Out-of-Scope Scripting (`rhai`)** | `cleanup/remove-rhai-scripting` | `core/crates/script/**`, `web/crates/ui-shared/src/geometry_scripts.rs`, `core/crates/engine/**`, `core/crates/mirrored-half/src/mirrored_half/reconcile/prepare.rs` | Per ADR-0029, user geometry scripting is out of product scope. Remove `boardstudio-script` and `rhai` from the workspace; strip `geometry_scripts.rs` UI from `ui-shared`; make legacy stored script fields in `ProjectDoc` deserialize cleanly as ignored/no-op during migration. | **Terra 6.1 High** (or **Spark 1.3 Contributor xhigh**) | **-3,500 LOC**, drops `rhai` from WASM & release `wasm-opt` |
| **W1-B: Evict `monstertruck-qualification` Harness** | `cleanup/evict-monstertruck-qualification` | `cad/Cargo.toml`, `cad/monstertruck-qualification/src/**`, `cad/monstertruck-qualification/tests/**`, `cad/monstertruck-qualification/Cargo.toml`, `cad/scripts/**` | Remove `monstertruck-qualification` and unused vendor crates (`monstertruck-solid`, `monstertruck-derive`) from `cad/Cargo.toml` members and `cad/scripts/test-cad.py`. Keep only the minimal vendor crates referenced by `[patch.crates-io]` for `boardstudio-cad` until Wave 2 replaces them. | **Terra 6.1 High** | **-40,000+ LOC** immediately removed from active CAD builds/tests |
| **W1-C: Retire Test-Floor Ratchet & Prune Redundant Script Meta-Tests** | `cleanup/retire-test-ratchet` | `scripts/check_native_tests.py`, `scripts/check-native-tests-baseline.json`, `scripts/test-check-native-tests.py`, `scripts/verification_receipt.py`, `scripts/test-verification-receipt.py` | Replace the strict 2,454 test-name floor check with a zero-failure `cargo test --workspace` execution check so subsequent waves can delete/consolidate redundant tests without baseline-json conflicts. Slim `verification_receipt.py` and its 1,487-LOC self-test. | **Terra 6.1 High** | **-4,500 LOC**, unlocks test consolidation across all waves |

---

### Wave 2: Replace Monstertruck & Consolidate 2D/3D Geometry (2 Parallel Worktrees)
*Goal: Completely eliminate `monstertruck-*` (all 133,880 vendored lines + 32,022 adapter lines) and `cavalier_contours`.*

| Track | Branch / Worktree | Owned Files (Disjoint) | Scope & Done Condition | Target Model | Est. Delta |
|---|---|---|---|---|---:|
| **W2-A: Direct 2.5D Case Mesher & Analytical STEP AP214 Writer** | `feat/cad-direct-25d-kernel` | `cad/monstertruck/src/layered/**`, `cad/monstertruck/src/profile.rs`, `cad/monstertruck/src/export.rs`, `cad/monstertruck/src/mesh.rs` | `layered` already computes horizontal Z-bands (`Band`, `Cap`) in 2D. Replace `monstertruck_modeling::Solid` construction, `monstertruck_meshing`, and `monstertruck_io::step::save` with:<br>1. Direct 2.5D triangulation from `Cap` + `Band` rings (planar caps via `i_overlay`/`lyon_tessellation` + analytical quad/cylinder/cone walls with exact `GeneratedSourceFace` provenance).<br>2. Direct ISO-10303-21 (`AP214`) STEP writer directly from `Cap` + `Band` rings (`MANIFOLD_SOLID_BREP`, `PLANE`, `CYLINDRICAL_SURFACE`, `CONICAL_SURFACE`, `LINE`, `CIRCLE`). | **Opus 5.5 High** (or **Wonder Max**) | Replaces ~90,000 LOC of B-Rep modeling/meshing/export dependencies with **~850 LOC** of fast, deterministic Rust |
| **W2-B: Replace `cavalier_contours` with `i_overlay` in `kicad-source`** | `refactor/unify-2d-polygons` | `core/crates/kicad-source/**` | Replace `cavalier_contours` usage in `core/crates/kicad-source` with `i_overlay` (already in `core/crates/kernel`), leaving only one 2D boolean/offset engine in Core. | **Terra 6.1 High** | Drops duplicate 2D boolean dependency from Core |
| **W2-C (Follow-on after W2-A): Replace Vendor STEP Reader & Delete `monstertruck-qualification/vendor/**`** | `feat/replace-vendor-step-reader` | `cad/monstertruck/src/step/**`, `cad/monstertruck/Cargo.toml`, `cad/monstertruck-qualification/vendor/**` | Pre-bake bundled component STEP files into compact binary meshes + collision hulls at content build time, replace `cad/monstertruck/src/step/` (and its 12k LOC of Monstertruck repair workarounds) with a focused `step-p21` analytical face/triangulation reader, and `git rm -rf cad/monstertruck-qualification/vendor`. | **Opus 5.5 High** | **-125,000+ LOC** (`git rm` of all vendored `monstertruck-*` crates) |

---

### Wave 3: `rust-ui.com` Unification & Native UI Testing (3 Parallel Worktrees)
*Goal: Make all 35 `rust-ui.com` components cross-target, introduce composed `rust-ui` inspector molecules, and convert slow headless-Chrome crate tests into instant native tests.*

| Track | Branch / Worktree | Owned Files (Disjoint) | Scope & Done Condition | Target Model | Est. Delta |
|---|---|---|---|---|---:|
| **W3-A: Cross-Target `rust-ui` & Composed Inspector Molecules** | `feat/rust-ui-inspector-molecules` | `web/crates/ui-shared/**` | 1. Move `dioxus-primitives` from `[target.'cfg(target_arch = "wasm32")'.dependencies]` to `[dependencies]` in `ui-shared/Cargo.toml` and remove `#[cfg(target_arch = "wasm32")]` gates in `ui-shared/src/ui/mod.rs` so all 35 `rust-ui` components compile on native host targets.<br>2. Add composed `rust-ui` inspector molecules (`NumericPropertyField`, `SelectPropertyField`, `TogglePropertyField`, `PropertySection`) built strictly from `rust-ui` primitives (`Field`, `InputGroup`, `Select`, `Combobox`, `Checkbox`, `Collapsible`, `Tooltip`, `Alert`). | **Terra 6.1 High** | Unlocks native compilation & testing of all Dioxus UI components |
| **W3-B: Migrate `case`, `parts`, `keycaps`, `keymap`, `pcb`, `library` to `rust-ui` Molecules** | `refactor/workspaces-rust-ui` | `web/crates/case/**`, `web/crates/parts/**`, `web/crates/keycaps/**`, `web/crates/keymap/**`, `web/crates/pcb/**`, `web/crates/library/**` | Replace the 85 hand-wired `rsx!` field blocks in `mechanical_settings.rs` (9,205 LOC), `generator_settings.rs` (2,802 LOC), `keycaps_settings.rs` (3,115 LOC), and raw HTML `<button>`/`<input>`/`<select>` tags with the new `rust-ui` inspector molecules. | **Terra 6.1 High** (sharded per crate if desired) | **-15,000+ LOC** of repetitive `rsx!` form boilerplate |
| **W3-C: Extract Pure-Rust 2D Canvas Gizmo Controller from `layout-objects`** | `refactor/canvas-gizmo-controller` | `web/crates/layout-objects/**`, `web/crates/layout-core/**` | Extract the 2D transform rail & gizmo state machine out of `layout_transform_toolbar.rs` (13,313 LOC), eliminating the `thread_local! { static RAIL_MOUNTS }` hack and per-tool `web_sys::Document` `Closure::wrap` listeners in favor of one pure-Rust controller and `rust-ui` `Toolbar`/`DropdownMenu`/`Tooltip` chrome. | **Opus 5.5 High** | **-8,000+ LOC** from `layout_transform_toolbar.rs` |

---

### Wave 4: Synchronous Main-Thread `CoreEngine` & Native Test Conversion (2 Parallel Worktrees)
*Goal: Instant 120fps edits with zero `PendingEditSignals` ceremony, and 3–4x faster `check.py` runs.*

| Track | Branch / Worktree | Owned Files (Disjoint) | Scope & Done Condition | Target Model | Est. Delta |
|---|---|---|---|---|---:|
| **W4-A: Synchronous Main-Thread Document Edits** | `arch/sync-main-thread-core` | `application/**`, `web/crates/runtime/**`, `web/crates/ui-shared/src/pending_edit*` | Execute `CoreEngine::apply_edit` synchronously on the main thread (`<0.2 ms`); keep async workers strictly for 3D CAD, 3D collision, IndexedDB save, and ZIP export. Retire `PendingEdits`, `PendingEditSignals`, and queued-field-settlement plumbing across inspector controllers. | **Opus 5.5 High** | **-35,000+ LOC** across `runtime`, `application`, and inspector controllers + instant 120fps input scrubbing |
| **W4-B: Convert Per-Crate Browser Tests to Fast Native Tests** | `perf/native-controller-tests` | `scripts/check.py`, `web/crates/*/src/*tests*.rs` | With `rust-ui` components compiling natively (W3-A) and synchronous edits (W4-A), convert the 799 per-crate `#[wasm_bindgen_test]` controller/inspector tests to native `#[test]` runs. Keep `browser` step focused on packaged-page E2E (`PAGE_SUITE`, `DRAG_SMOKE`, `CAD_BROWSER`). | **Terra 6.1 High** | **-50,000+ LOC** of bespoke browser-mount harnesses; cuts `check.py browser` by ~90s |

---

### Wave 5: Replace `mirrored-half` Reconciliation with Derived Symmetry + `proptest` Invariants
*Goal: Collapse the 52,000-LOC `mirrored-half` subsystem into ~2,000 LOC of pure math and property tests.*

| Track | Branch / Worktree | Owned Files (Disjoint) | Scope & Done Condition | Target Model | Est. Delta |
|---|---|---|---|---|---:|
| **W5-A: Operation-Level / Derived Bilateral Symmetry** | `arch/derived-mirrored-half` | `core/crates/mirrored-half/**`, `core/crates/engine/tests/mirrored_*.rs`, `core/tests/mirrored_half.rs` | Replace post-hoc `before`/`after` `ProjectDoc` tree-diff reconciliation (`reconcile.rs` 7.5k LOC, `lifecycle.rs` 4.2k LOC) with deterministic mirror projection over `MirroredHalves` + sparse `MirroredException` overrides, validated by a compact `proptest` operation-sequence invariant suite. | **Opus 5.5 High** | **-45,000+ LOC** (`20k` prod + `25k` narrative tests replaced by `proptest` invariants) |

---

## 3. Cumulative Impact Summary

| Metric | Current (`dev`) | After Waves 1–5 |
|---|---:|---:|
| **Total Rust + Script LOC** | ~722,000 LOC | **~240,000 LOC** (and <100k excluding bundled content/footprints) |
| **Vendored Monstertruck / B-Rep Kernel LOC** | 165,902 LOC | **0 LOC** (replaced by ~1,500 LOC direct 2.5D mesher + STEP AP214 stack) |
| **Out-of-Scope Scripting (`rhai`)** | Compiled in every Core & WASM build | **0 LOC** (completely removed) |
| **UI Component System** | Mixed `rust-ui` (`wasm32`-only) + 99 raw HTML controls + 85-macro monolithic forms | **100% `rust-ui.com` cross-target primitives + composed inspector molecules** |
| **Edit Latency (Main Thread)** | Async ticket -> Worker -> `PendingEditSignals` settlement | **< 0.2 ms synchronous main-thread mutation (120fps)** |
| **Full `check.py` Wall Time** | ~227s | **~60–75s** |
