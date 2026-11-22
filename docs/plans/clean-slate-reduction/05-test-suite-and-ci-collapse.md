# Track 5: Collapse Test Bloat, Retire the 12 Per-Crate Chrome Launches & Slim `scripts/`

**Base:** Merged Wave 2 (after Tracks 1–4 have landed their crate-local test reductions).  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely.

**Owned Surfaces:**
- `core/tests/**` (remaining non-mirror narrative suites — 50,466 LOC on `dev` after Track 1 deletes `mirrored_half` & `native_matrix_exceptions`)
- `web/crates/*` remaining `#[wasm_bindgen_test]` modules & browser launch entries
- `web/tests/**` (3,626 LOC)
- `scripts/**` (26,263 LOC on `dev`)

---

## 1. Why Test & Script Collapse Is Essential

On `dev` (`5aae86ff4`):
- **Rust Test LOC = 248,907** (46% of all Rust code) — burning massive compile time on every `cargo test` and `wasm-pack test`.
- `scripts/check.py` still compiles and boots **12 separate headless-Chrome `wasm-pack test` crate launches** (`host`, `worker-host`, `runtime`, `ui-shared`, `keycaps`, `library`, `keymap`, `case`, `parts`, `pcb`, `layout`, `layout-objects`) PLUS `PAGE_SUITE`, `DRAG_SMOKE`, and `CAD_BROWSER`.
- `scripts/` itself is **26,263 LOC**, dominated by meta-tests testing the test runner (`test-check.py` 1,806 LOC, `test-verification-receipt.py` 1,418 LOC, `test-run-wasm-tests.py` 1,126 LOC, `verification_receipt.py` 810 LOC, plus 10+ UI script-check files).

**Track 5's Goal:**
1. **Retire 10–12 of the 12 per-crate Chrome launches** (target: **≤ 2** crate launches — ONLY `host`/`worker-host` if IndexedDB/WebGL/CAD-worker tests cannot run in `PAGE_SUITE`, ideally **0** crate launches once those run in `PAGE_SUITE`!).
2. **Collapse `core/tests/`** from ~50,400 remaining LOC → **≤ 18,000 LOC** by replacing copy-pasted ticket scenario tests with parameterized tables and property invariants.
3. **Collapse `scripts/`** from 26,263 LOC → **≤ 8,000 LOC** (**−18,000+ LOC**).
4. **Cut full `python3 scripts/check.py` wall time to ≤ 60–75 seconds.**

---

## 2. Concrete Execution Plan

### Part A — Retire Per-Crate Chrome Launches (`12 → ≤ 2` launches)
Audit the 12 entries in `scripts/check.py` `browser()`:
1. **Must retire to 0 browser launches (100% native `#[test]` + packaged `DRAG_SMOKE` / `PAGE_SUITE`):**
   - `ui-shared`, `keycaps`, `library`, `keymap`, `case`, `parts`, `pcb`, `layout`, `layout-objects`, `runtime` (10 crates!).
   - Every pure state/projection/parsing/controller test runs natively under `cargo test --workspace`.
   - Any truly browser-dependent UI regression (real focus, pointer capture, drag-save-undo, Case collision) is covered ONCE in `scripts/run-drag-browser.py` or the slimmed `PAGE_SUITE` (target: ≤ 35 high-signal E2E assertions, down from 115).
2. **Keep ONLY true browser-platform boundary launches (≤ 2 total):**
   - `web/crates/host` (IndexedDB persistence + WebGL2 renderer host lifecycle — ≤ 15 focused tests)
   - `web/crates/worker-host` (CAD Web Worker qualification & collision — ≤ 10 focused tests)
   - *(If both fit cleanly inside `PAGE_SUITE` + `CAD_BROWSER`, retire them too for **0** per-crate Chrome launches!)*
3. Remove all retired crates from `scripts/check.py` `browser()` and `WEB_WASM_PACKAGES` test prebuilds so Cargo stops compiling 10–12 giant test WASM binaries on every `check.py` run.

### Part B — Consolidate `core/tests/` (~50,400 → ≤ 18,000 LOC)
On `dev`, `core/tests/` has **114 files** (50.4k LOC excluding the mirror tests deleted in Track 1), packed with 100–200 line copy-pasted ticket scenarios:
- `core/tests/core/` (4,710 LOC), `support/` (3,388 LOC), `mechanical_integration/` (2,450 LOC), `matrix_anchors/` (1,780 LOC), `mechanical_settings_edits/` (1,636 LOC), `outline_edits/` (1,616 LOC), `matrix_placement/` (1,493 LOC), `module_import/` (1,307 LOC), `outlines.rs` (1,270 LOC), `matrix_kernel.rs` (1,258 LOC), `gasket_features.rs` (1,237 LOC), `document_integrity.rs` (1,235 LOC), `mechanical_identity_monotonic.rs` (1,111 LOC), plus ~75 smaller single-ticket files.
- **Consolidation Rules:**
  - Merge duplicate fixtures into ONE concise builder helper (`core/tests/support.rs`, ≤ 400 LOC).
  - Replace repeated ticket-by-ticket scenarios (e.g., 15 separate tests doing slight variations of the same matrix/outline/mechanical edit) with **table-driven tests or seeded property sweeps** asserting the real domain invariants (geometry bounds, validity, Undo/Redo, export determinism, refusal atomicity).
  - Delete tests that assert obsolete internal field ordering or superseded error string wording.
  - Keep **every genuine domain invariant** (mechanical tolerances, STEP/KiCad/ZMK export accuracy, outline boolean validity, matrix geometry, malformed-input refusal).

### Part C — Slim `scripts/` (`26,263 → ≤ 8,000 LOC`)
Delete over-engineered script infrastructure and meta-tests while keeping the real build/gate/E2E commands rock-solid:
- **Delete completely:**
  - `scripts/verification_receipt.py` *(810 LOC)* & `scripts/test-verification-receipt.py` *(1,418 LOC)* — replace with plain 30-line JSON/stdout summary inside `check.py` (update `AGENTS.md` / repo checks cleanly).
  - Static regex/text UI-grep script tests that duplicate compiler/test checks: `scripts/test-command-palette.py` (639), `scripts/test-case-settings.py` (588), `scripts/test-layout-toolbar.py` (512), `scripts/test-dock-layout.py` (464), `scripts/test-parts-library.py` (369), `scripts/test-header.py` (330), `scripts/test-layout-review-regressions.py` (327) — **−3,229 LOC** of brittle source-regex tests immediately!
  - `scripts/test-parallel-interrupt.py` (625 LOC) & `scripts/test-check-wasm-tests.py` (once per-crate WASM runner complexity is gone).
- **Slim aggressively:**
  - `scripts/check.py` (`1,755 → ≤ 500 LOC`) & `scripts/test-check.py` (`1,806 → ≤ 300 LOC`): With 10–12 per-crate Chrome launches and receipt plumbing gone, `check.py` becomes a clean, fast orchestrator (`repo`, `build`, `lint`, `test`, `browser`).
  - `scripts/run-wasm-tests.py` (`866 → ≤ 350 LOC`) & `scripts/test-run-wasm-tests.py` (`1,126 → ≤ 200 LOC`).
  - `scripts/build-web.py` (`571 → ≤ 350 LOC`) & `scripts/test-build-web.py` (`690 → ≤ 200 LOC`).
  - Keep `scripts/run-drag-browser.py` (packaged 2×2 + 4×6 E2E), `scripts/browser_tools.py`, `cad/scripts/test-cad*.py`, and `scripts/ergogen-parity/`.

---

## 3. Hard LOC Budget (Whole-Repo `check_reduction.py` Gate vs. Post-Wave-2 Base)

- **Mandatory Net Production Rust Delta:** **`<= 0` LOC** (no production bloat added)
- **Mandatory Net Test Rust Delta:** **`<= -50,000` LOC** (across `core/tests/`, `web/tests/`, and retired WASM crate tests)
- **Mandatory Net Scripts Delta:** **`<= -15,000` LOC** (`scripts/` finishes **≤ 8,000 LOC** total)
- **Mandatory Net Other Delta:** **`<= 0` LOC**
- **Mandatory Browser Launch Cap:** `scripts/check.py` launches **≤ 2** per-crate Chrome processes (down from 12).
- **Mandatory Wall-Time Target:** Full `python3 scripts/check.py` finishes in **≤ 75 seconds** (warm target dir).

---

## 4. Mandatory Deletion Manifest

- `scripts/verification_receipt.py`
- `scripts/test-verification-receipt.py`
- `scripts/test-command-palette.py`
- `scripts/test-case-settings.py`
- `scripts/test-layout-toolbar.py`
- `scripts/test-dock-layout.py`
- `scripts/test-parts-library.py`
- `scripts/test-header.py`
- `scripts/test-layout-review-regressions.py`
- `scripts/test-parallel-interrupt.py`
- All `crate_launch(...)` entries for `ui-shared`, `keycaps`, `library`, `keymap`, `case`, `parts`, `pcb`, `layout`, `layout-objects`, `runtime` inside `scripts/check.py`.

---

## 5. Banned Patterns (Automatic Rejection)

- **DO NOT** write a 1,400-line markdown "coverage inventory" in `.evidence/` before deleting tests. Just audit, keep the genuine invariant natively or in packaged E2E, and delete the duplicate launch.
- **DO NOT** disable tests with `#[ignore]` to claim speedups — delete redundant tests cleanly and keep active, passing assertions.
- **DO NOT** drop `scripts/run-drag-browser.py` (2×2 & 4×6 collision), `cad/scripts/test-cad-browser.py`, or `cad/scripts/test-cad-provider.py`.

---

## 6. Verification & Mechanical Gate Commands

```bash
# 1. Mechanical Reduction Gate (MUST exit 0 against <WAVE2_SHA>)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base <WAVE2_SHA> \
  --head HEAD \
  --max-prod-delta 0 \
  --max-test-delta -50000 \
  --max-script-delta -15000 \
  --max-other-delta 0 \
  --must-delete scripts/verification_receipt.py \
  --must-delete scripts/test-verification-receipt.py \
  --must-delete scripts/test-command-palette.py \
  --must-delete scripts/test-case-settings.py \
  --must-delete scripts/test-layout-toolbar.py \
  --must-delete scripts/test-dock-layout.py \
  --must-delete scripts/test-parts-library.py \
  --must-delete scripts/test-header.py \
  --must-delete scripts/test-layout-review-regressions.py

# 2. Full gate & wall-time check (under shared flock)
time python3 scripts/check.py
```
