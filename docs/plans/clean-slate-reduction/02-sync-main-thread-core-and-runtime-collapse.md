# Track 2: Retire Core Worker, Delete Async Edit Ceremony & Slim `runtime`/`host`/`application`

**Base:** `dev` (`5aae86ff4`) — can run in parallel with Track 1 in an isolated worktree.  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely.  
**Reference Commit:** Inspect `phase4/sync-core` (`5f698fd52`) as a proven starting point for retiring `CoreWorker`, then push all the way through `runtime`, `application`, and `draft_edits` deletion.

**Owned Surfaces:**
- `web/src/core_worker.rs`, `web/generated/core-worker-entry.js`, `web/Cargo.toml` (`core-worker` feature)
- `web/crates/host/**` (specifically `core_protocol.rs`, `core_executor.rs`, worker transport)
- `web/crates/runtime/**`
- `application/**`
- `web/crates/ui-shared/src/draft_edits.rs` & `web/crates/ui-shared/src/draft_edits/**`
- `*_queued_edit_tests.rs`, `*_queued_settings_tests.rs`, `*_settlement_tests.rs` across workspace crates
- Build script wiring for `core-worker` in `scripts/build-web.py` (minimal removal of the deleted worker target ONLY)

---

## 1. Baseline vs. Hard LOC Budget

| Surface on `dev` (`5aae86ff4`) | Prod LOC | Test LOC | Total LOC |
|---|---:|---:|---:|
| `web/crates/runtime/` (60 files) | 15,829 | 11,635 | 27,464 |
| `web/crates/host/` (14 files) | 3,198 | 3,028 | 6,226 |
| `application/` (14 files) | 3,337 | 4,604 | 7,941 |
| `web/src/core_worker.rs` + `core-worker-entry.js` | ~260 | 0 | ~260 |
| `web/crates/ui-shared/src/draft_edits*` (3 files) | 355 | 1,393 | 1,748 |
| Queued/settlement test files across `keycaps`, `keymap`, `library`, `parts`, `pcb`, `layout-objects` | 69 | ~5,500 | ~5,569 |
| **Total Track 2 Footprint** | **~23,048** | **~26,160** | **~49,208** |

### Hard Budget (Whole-Repo `check_reduction.py` Gate vs. `dev`)
- **Replacement Ceiling:** Combined `runtime` + `host` + `application` MUST be **≤ 18,000 LOC total** (≤ 11,000 Prod + ≤ 7,000 Test).
- **Mandatory Net Production Rust Delta:** **`<= -10,000` LOC**
- **Mandatory Net Test Rust Delta:** **`<= -18,000` LOC**
- **Mandatory Net Scripts Delta:** **`<= 0` LOC** (retiring `core-worker` build step shrinks `scripts/build-web.py` slightly)
- **Mandatory Net Other Delta:** **`<= +200` LOC**
- **WASM Win:** Eliminates `core-worker.wasm` (**−11.6 MB** removed from packaged output and cold build!).

---

## 2. Why `runtime` & `application` Are 35k+ LOC Today (And How to Collapse Them)

On `dev`, document edits already execute synchronously in-process on the main thread, **yet the codebase still carries two full layers of legacy async-worker ceremony**:
1. **A still-compiled `core-worker` WASM binary (`11.6 MB`)** kept around solely for ZIP archive packing/unpacking and firmware/PCB artifact export — dragging `boardstudio_core` into a second WASM build, maintaining `CoreExecutor` traits, `core_protocol.rs`, `RestartCoreExecutor`, epoch checks, and 1,000+ lines of worker-restart tests.
2. **Queued-edit / multi-phase settlement plumbing** (`draft_edits.rs` 355 LOC + 1,393 LOC of tests, `queued_edits` across every workspace, `runtime/effects.rs` 727 LOC, `runtime/projects.rs` 885 LOC, `application/src/session/` completion/retry state machines, plus ~5,500 LOC of `*_queued_edit_tests.rs` and `*_settlement_tests.rs`).

### Target Architecture (Clean, Synchronous Main-Thread Document State)

1. **Retire `core-worker` 100%:**
   - Delete `web/src/core_worker.rs`, `web/generated/core-worker-entry.js`, `web/crates/host/src/core_protocol.rs`, and the `core-worker` Cargo feature + `scripts/build-web.py` step.
   - Run document edits, ZIP archive pack/unpack, and artifact generation directly in-process via `CoreEngine` (fast synchronous calls; for large ZIPs, keep only `async` Blob/IndexedDB I/O).
   - Keep **ONLY the CAD Web Worker** (`worker-host` / `cad_worker`) for 3D meshing, STEP import, and 3D collision checks.
2. **Collapse `application` + `runtime` Edit Pipeline to Direct Synchronous Commit:**
   - `runtime.commit(op: EditOperation) -> Result<AcceptedSnapshot, EditError>` runs **synchronously** on the main thread (`<0.2 ms`), immediately updating the Dioxus `Signal<ProjectView>`.
   - **Persistence Model (Simple & Safe, ≤ 150 LOC):**
     - Immediately after synchronous in-memory commit, schedule an asynchronous IndexedDB write on a single-slot coalescing saver (`SaveState`: `Saved` | `Saving(rev)` | `Error(msg)` with one Retry button and `beforeunload` warning when dirty).
     - **If save-before-publication (ADR-0013) is explicitly required by the user**, implement it as a **single ≤ 120-LOC `async fn commit_and_save`** directly on `Runtime` without multi-slot queues, tickets, or per-workspace settlement controllers. **Either way, DELETE `draft_edits.rs`, all per-workspace `queued_edits` state machines, and all `*_queued_edit_tests.rs`.**
3. **Collapse `runtime` Adapters & Preview Modules:**
   - Merge redundant adapter boilerplate across `runtime/src/runtime/` (`effects.rs`, `sources.rs`, `layout_preview.rs`, `scene_preview.rs`, `case_preview.rs`, `parts_preview.rs`, `pcb_handoff.rs`, `firmware_request_adapter.rs`, `firmware_position_projection.rs`, `mechanical_package.rs`, `portable_archive.rs`, `exports.rs`, `export_bytes.rs` — currently **11,000+ Prod LOC**!) into clean, direct functions reading the `ResolvedDoc` snapshot (**≤ 4,500 Prod LOC** total).
   - Collapse `model_delivery.rs` + `model_delivery/*` (3,440 LOC!) into a simple asset cache + fetch loader (**≤ 800 LOC**).

---

## 3. Mandatory Deletion Manifest

Every single path below MUST be **completely deleted** (`git ls-files` count = 0):

### Worker & Async Protocol Deletions
- `web/src/core_worker.rs` *(199 LOC)*
- `web/generated/core-worker-entry.js`
- `web/crates/host/src/core_protocol.rs` *(29 LOC)*
- `web/crates/ui-shared/src/draft_edits.rs` *(355 LOC)*
- `web/crates/ui-shared/src/draft_edits/` *(1,393 LOC)*

### Superseded Queued-Edit & Settlement Test Suites to Delete
- `web/crates/keycaps/src/keycaps_settings/queued_settings_tests.rs` *(894 LOC)*
- `web/crates/keymap/src/keymap/queued_edit_tests.rs` *(1,158 LOC)*
- `web/crates/keymap/src/keymap/queued_edit_tests/` *(409 LOC)*
- `web/crates/library/src/library/queued_rename_tests.rs` *(91 LOC)*
- `web/crates/parts/src/parts/component_model_editor/settlement_tests.rs` *(658 LOC)*
- `web/crates/pcb/src/pcb_wiring/queued_edit_tests.rs` *(1,041 LOC)*
- `web/crates/layout-objects/src/objects/matrix_transform_controller/queued_edits.rs` *(69 LOC)*
- `web/crates/runtime/src/runtime/in_process_adapter_tests.rs` *(212 LOC)*
- `web/crates/runtime/src/runtime/document_core_tests/` *(525 LOC — replace with ≤150 LOC native tests)*

### Forbidden Symbols (Must Have 0 Matches in `*.rs`)
- `CoreWorker`
- `CORE_WORKER_FRAME_VERSION`
- `RestartCoreExecutor`
- `encode_core_request`
- `DraftEditController` *(or equivalent `draft_edits` settlement hook)*
- `PendingEdit`

---

## 4. Replacement Verification Suite (≤ 5,000 Test LOC Across `runtime` + `host` + `application`)

Keep high-signal tests only (native `#[test]` wherever DOM/IndexedDB is not required):
1. **Native (`cargo test`):** Synchronous `Runtime::commit` happy path, refusal leaves state intact, Undo/Redo, stale-revision guard, background save success/failure/retry, ZIP export/import round-trip, CAD job cancellation on newer edit.
2. **Browser (`host` + `runtime` slimmed):** Real IndexedDB save/reopen/delete/quota-error handling, CAD Web Worker round-trip & 4×6 collision completion, offline safety.

---

## 5. Banned Patterns (Automatic Rejection)

- **DO NOT** keep `core-worker` "just for ZIP" or "just for exports".
- **DO NOT** replace `draft_edits.rs` with a renamed 600-LOC `field_lifecycle.rs` hook — synchronous edits do not need async field-settlement state machines!
- **DO NOT** port 5,500 lines of `queued_edit_tests` by renaming them to `sync_edit_tests` — delete them and test synchronous commit once at the `runtime` boundary.

---

## 6. Verification & Mechanical Gate Commands

```bash
# 1. Mechanical Reduction Gate (MUST exit 0)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base 5aae86ff4 \
  --head HEAD \
  --max-prod-delta -10000 \
  --max-test-delta -18000 \
  --max-script-delta 0 \
  --max-other-delta 200 \
  --must-delete web/src/core_worker.rs \
  --must-delete web/crates/host/src/core_protocol.rs \
  --must-delete web/crates/ui-shared/src/draft_edits.rs \
  --must-delete web/crates/ui-shared/src/draft_edits \
  --must-delete web/crates/keycaps/src/keycaps_settings/queued_settings_tests.rs \
  --must-delete web/crates/keymap/src/keymap/queued_edit_tests.rs \
  --must-delete web/crates/pcb/src/pcb_wiring/queued_edit_tests.rs \
  --must-delete web/crates/parts/src/parts/component_model_editor/settlement_tests.rs \
  --forbid-symbol CoreWorker \
  --forbid-symbol CORE_WORKER_FRAME_VERSION \
  --forbid-symbol encode_core_request

# 2. Focused checks (inside worktree)
cargo fmt --all --check
cargo test -p boardstudio-application -p boardstudio-web-host -p boardstudio-web-runtime -p boardstudio-web-ui-shared
cargo check -p boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page
```
