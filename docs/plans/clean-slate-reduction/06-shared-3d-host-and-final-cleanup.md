# Track 6: Unified 3D Host, Core Mechanical/Kernel Trim & Final Release Audit

**Base:** Merged Wave 2 (can run in parallel with Track 5 in a file-disjoint worktree, or immediately after Track 5).  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely.

**Owned Surfaces:**
- 3D Viewer & Host: `web/crates/host/src/renderer_host.rs`, `web/crates/case/src/shared_viewer*`, `web/crates/case/src/case_viewer*`, `web/crates/case/src/cad_presentation.rs`, `web/crates/runtime/src/renderer_host_page.rs` (if present)
- Core non-mirror trim: `core/crates/mechanical/**`, `core/crates/kernel/**`, `core/crates/artifact/**`, `core/crates/model/**`
- Repo & worktree hygiene cleanup

---

## 1. Scope & Reduction Targets

After Tracks 1–5 remove the 5 largest structural monoliths, Track 6 finishes the cleanup and verifies the whole-repo end-state:

### Part A — Unify the 3D WebGL2 Viewport & Loader (`~8,500 → ≤ 2,000 LOC`, **−6,500 LOC**)
On `dev`, 3D viewing is split across:
- `web/crates/case/src/shared_viewer/` (3,390 LOC) + `shared_viewer.rs` (1,201 LOC) = **4,591 LOC**
- `web/crates/case/src/case_viewer/` (1,741 LOC) + `case_viewer.rs` (875 LOC) = **2,616 LOC**
- `web/crates/case/src/cad_presentation.rs` (986 LOC)
- `web/crates/host/src/renderer_host.rs` (1,263 LOC)
- Duplicate renderer module mount & decode paths across `host` and `runtime`

**Replacement:**
- **ONE** shared renderer module initializer in `web/crates/host/src/renderer_host.rs` (≤ 500 Prod LOC) used by both mount and decode (single cache, retry on failure, clean teardown, resize & context-loss handling).
- **ONE** shared Dioxus `<Viewport3D />` component (≤ 900 Prod LOC) used by Case, Parts, Layout 3D, and Keycaps 3D — deleting duplicate orbit/pan/zoom gesture wrappers and redundant `case_viewer` / `shared_viewer` splits.
- Concise native orbit/camera math tests + 1 browser WebGL smoke check (≤ 500 Test LOC).

### Part B — Trim Dead Paths in `core/crates/{mechanical,kernel,artifact}` (`~52,600 → ≤ 34,000 LOC`, **−18,600 LOC**)
Once Track 1 removes legacy mirror reconciliation and format-7 compatibility, audit and delete dead/duplicated code in:
- `core/crates/mechanical/` (25,442 LOC: 19,406 Prod + 6,036 Test → target **≤ 16,500 LOC**):
  - Delete legacy mirror-compatibility branches in `mechanical/halves.rs` (1,380 LOC) and `mechanical/edit/changes.rs` (1,347 LOC).
  - Deduplicate repeated case-body/clearance/opening validation helpers and collapse copy-pasted mechanical unit tests.
- `core/crates/kernel/` (20,526 LOC: 16,395 Prod + 4,131 Test → target **≤ 13,500 LOC**):
  - Remove dead pre-format-8 deserializers, unused compatibility helpers, and duplicate outline/matrix utilities.
- `core/crates/artifact/` (6,615 LOC → target **≤ 4,000 LOC**):
  - Simplify KiCad/ZMK/BOM artifact emitters now that they read one clean `ResolvedDoc`.

### Part C — Prune Stale Worktrees & Final Program Audit
1. Audit `git worktree list` (currently **37 worktrees** on disk) and remove all merged/abandoned worktrees (never delete active/unmerged user branches without checking with the user first). Confirm `df -h .` has `> 50 GiB` free.
2. Run the **Final Program Audit** against `dev` (`5aae86ff4`):
   - Total tracked `.rs` + `scripts/*.{py,sh}` LOC **≤ 230,000** (down from 565,286 on `dev` and 721,916 on baseline `28e08ac7f`).
   - Full `python3 scripts/check.py` passes cleanly in **≤ 75s**.
   - Packaged release build (`scripts/build-web.py`) succeeds with:
     - Total shipped WASM bytes **strictly smaller than `dev`** (at least **−11.6 MB** from retiring `core-worker.wasm` plus page WASM shrinkage).
     - `scripts/run-drag-browser.py` passes on both 2×2 default and 4×6 saved collision fixtures.
     - Cold startup and drag-to-save latency **≤ `dev`** (`≤ 1,116 ms` cold startup, `≤ 64 ms` drag-to-saved).

---

## 2. Hard LOC Budget (Whole-Repo `check_reduction.py` Gate vs. Post-Wave-2/Track-5 Base)

- **Mandatory Net Production Rust Delta:** **`<= -16,000` LOC**
- **Mandatory Net Test Rust Delta:** **`<= -9,000` LOC**
- **Mandatory Net Scripts Delta:** **`<= 0` LOC**
- **Mandatory Net Other Delta:** **`<= +100` LOC**

### Cumulative End-of-Program Gate (Against `dev` `5aae86ff4`)
- **Cumulative Net Production Rust Delta:** **`<= -120,000` LOC** (290,116 → ≤ 170,000)
- **Cumulative Net Test Rust Delta:** **`<= -170,000` LOC** (248,907 → ≤ 79,000)
- **Cumulative Net Scripts Delta:** **`<= -18,000` LOC** (26,263 → ≤ 8,200)
- **Total Tracked Rust + Scripts:** **≤ 230,000 LOC** *(conservative hard ceiling: ≤ 255,000 LOC if `footprints`/`cad`/`renderer` remain untouched)*.

---

## 3. Mandatory Deletion Manifest

- `web/crates/case/src/case_viewer/` *(folded into unified `<Viewport3D />`)*
- `web/crates/case/src/case_viewer.rs`
- `web/crates/case/src/shared_viewer/tests.rs` *(superseded by concise native camera tests + 1 WebGL smoke)*
- Any remaining duplicate renderer loader/init functions outside `web/crates/host/src/renderer_host.rs`

---

## 4. Verification & Final Gate Commands

```bash
# 1. Track 6 Mechanical Reduction Gate (against <PRE_TRACK6_SHA>)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base <PRE_TRACK6_SHA> \
  --head HEAD \
  --max-prod-delta -16000 \
  --max-test-delta -9000 \
  --max-script-delta 0 \
  --max-other-delta 100 \
  --must-delete web/crates/case/src/case_viewer \
  --must-delete web/crates/case/src/case_viewer.rs

# 2. Cumulative Program Gate (against original dev 5aae86ff4)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base 5aae86ff4 \
  --head HEAD \
  --max-prod-delta -120000 \
  --max-test-delta -170000 \
  --max-script-delta -18000 \
  --max-other-delta 500

# 3. Full Verification Gate (under shared flock)
cargo fmt --all --check
time python3 scripts/check.py
```
