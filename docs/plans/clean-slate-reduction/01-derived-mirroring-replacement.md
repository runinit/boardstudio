# Track 1: Replace `mirrored-half` Reconciler with Pure Derived Symmetry

**Base:** `dev` (`5aae86ff4`)  
**Prerequisites:** Read [00-agent-guardrails-and-gate.md](00-agent-guardrails-and-gate.md) completely.  
**Owned Surfaces:**
- `core/crates/mirrored-half/**`
- `core/crates/model/src/model/document.rs` (mirror types) & `core/crates/model/src/mirrored_half.rs`
- `core/crates/engine/src/**` & `core/crates/engine/tests/**` (mirror dispatch, archive format 8, open/save)
- `core/crates/kernel/src/matrix/layout/components.rs` & `core/crates/kernel/src/migrate.rs`
- `core/tests/mirrored_half/**`, `core/tests/mirrored_half.rs`, `core/tests/native_matrix_exceptions/**`
- In-repo example/demo `.boardstudio` / JSON fixtures (updated in-place to canonical format 8)
- Minimal call-site compile fixes in `application/` and `web/` ONLY where Core API signatures change (leave Runtime/UI refactors to Tracks 2–4).

---

## 1. Baseline vs. Hard LOC Budget

| Surface on `dev` (`5aae86ff4`) | Prod LOC | Test LOC | Total LOC |
|---|---:|---:|---:|
| `core/crates/mirrored-half/` (47 files) | 19,217 | 7,543 | 26,760 |
| `core/crates/engine/tests/mirrored_*` (22 files) | 0 | 10,450 | 10,450 |
| `core/tests/mirrored_half*` + `native_matrix_exceptions` (18 files) | 0 | 12,328 | 12,328 |
| `core/crates/kernel/src/migrate.rs` + `matrix/layout/components.rs` | ~1,570 | ~100 | ~1,670 |
| Mirror dispatch/reconcile glue in `core/crates/engine/src/` | ~800 | ~200 | ~1,000 |
| **Total Track 1 Footprint** | **~21,587** | **~30,621** | **~52,208** |

### Hard Budget (Whole-Repo `check_reduction.py` Gate vs. `dev`)
- **Replacement Ceiling:** New `core/crates/mirrored-half` MUST be **≤ 3,000 Prod LOC** and **≤ 2,000 Test LOC** (**≤ 5,000 LOC total**).
- **Mandatory Net Production Rust Delta:** **`<= -16,000` LOC**
- **Mandatory Net Test Rust Delta:** **`<= -27,000` LOC**
- **Mandatory Net Scripts Delta:** **`<= 0` LOC**
- **Mandatory Net Other Delta:** **`<= +200` LOC**

---

## 2. Explicit Clean-Slate Product Decisions (Non-Negotiable)

These decisions are **final**. Do NOT soften them or add compatibility shims:

1. **Canonical-Only Document Model (Format 8).**
   - `ProjectDoc` stores **ONLY** authored source entities, the `MirroredPair` definitions (both two-board split pairs and same-board mirrored halves), and a sparse `MirrorOverrides` map (`Local(Payload)` | `Omitted` | `CompanionPose(Vec2, f64)`).
   - **NEVER store materialized mirror copies inside `ProjectDoc` or saved archives.**
   - **Zero v7 migration, zero staged conversion, zero drift-repair dialogs, zero original-file recovery.** Reject any file with `formatVersion != 8` at admission with a single clear error string. Update bundled example/demo JSONs in-place.
2. **Source-Only Editing (Delete 100% of `inverse/`).**
   - Edits always target authored entities.
   - If an `EditOperation` targets a derived **Linked** mirror entity ID, Core returns a typed refusal (`MirrorEditRefused::LinkedTarget { target_id, source_id, pair_id }`) telling the caller to either edit `source_id` or invoke `MakeLocal`.
   - **ZERO inverse edit translation.** Do not translate mirror coordinates back onto the source half.
   - Explicit lifecycle commands supported on mirror targets: `MakeLocal`, `Omit`, `Rejoin`, `Unlink`, `SetCompanionPose`, and `DeleteSource`.
3. **Single Pure Deterministic Projector.**
   - One pure function: `project_document(canonical: &ProjectDoc) -> ResolvedDoc` (or `ResolvedSnapshot`).
   - Takes `canonical` + reflection axis/mode and produces the complete read-only resolved geometry (parts, matrices, keys, companions, outlines, mechanical features, assemblies, keycaps, electrical boards/nets) in **one deterministic pass** with **zero before/after tree diffing** and **zero multi-pass reconciliation**.
   - Both **two-board pairs** (e.g. Left/Right split halves) and **same-board halves** use the exact same projector with a single `MirrorDestination` enum (`SecondBoard(BoardId)` | `SameBoardLayout(LayoutId)`).
   - Reversible single-board assemblies continue to reference one board via two physical instances without duplicating board geometry.
4. **Explicitly Dropped Legacy Quirks (DO NOT RE-IMPLEMENT).**
   - No before/after `ProjectDoc` structural diffing (`reconcile/*`).
   - No multi-pass convergence or `stable_placement` loops.
   - No false-parent decoy ID heuristics (ownership comes strictly from structured IDs / `PairKey(PairId, ItemKind, SourceId)` — never string prefix parsing).
   - No complex tombstone compaction/reallocation ledgers beyond a simple `BTreeSet<OmittedKey>`.
   - No legacy `changed_document_ids` structural diff scan (return direct affected IDs from the operation + pair projector).
   - Undo/Redo stores canonical `ProjectDoc` snapshots (or diffs) — **zero** special mirror undo/redo machinery because re-projecting an undone canonical doc is automatically exact.

---

## 3. Target Replacement Architecture (≤ 3,000 Prod LOC)

Replace the entire 47-file `core/crates/mirrored-half` crate with **at most 6 focused files**:

```text
core/crates/mirrored-half/src/
├── lib.rs              (~60 LOC)   Public API exports
├── model.rs            (~350 LOC)  MirroredPair, MirrorDestination, MirrorKey, MirrorOverride, MirrorOwnership
├── reflect.rs          (~300 LOC)  Pure 2D/3D pose, polygon, outline, rotation & companion reflection math
├── project.rs          (~1,200 LOC) Single-pass pure projector: canonical ProjectDoc -> ResolvedDoc
│                                    (parts, matrices/keys/companions, outlines, mechanical, electrical, keycaps)
├── ops.rs              (~650 LOC)  Canonical lifecycle mutations & admission:
│                                    create_pair, unlink_pair, make_local, omit, rejoin,
│                                    set_companion_pose, admit_edit (source-only guard), dependency refusal
└── invariants_tests.rs (~1,500 LOC) Seeded property/operation-sequence state-machine test suite
```

### Core Engine & Snapshot Contract (`core/crates/engine/src/`)
- `CoreEngine` owns `canonical: ProjectDoc` and caches `resolved: Arc<ResolvedDoc>` (recomputed via `project_document(&canonical)` whenever `canonical` changes).
- `apply_edit(op)`:
  1. Run `admit_edit(&canonical, &op)` — rejects writes to Linked mirror IDs immediately with `MirrorEditRefused`.
  2. Apply `op` directly to `canonical` (or to `ops::*` for mirror lifecycle commands).
  3. Re-derive `resolved = project_document(&canonical)`.
  4. Validate `resolved` (and canonical integrity).
  5. Push previous `canonical` onto Undo stack; return accepted snapshot carrying both `canonical` (for save/archive) and `resolved` (for UI/CAD/PCB/firmware consumers).
- **Archive & Persistence:** Serialize/deserialize `canonical` ONLY with `formatVersion: 8`.

---

## 4. Mandatory Deletion Manifest

Every single path below MUST be **completely deleted** (`git ls-files` count = 0):

### Production Files to Delete
- `core/crates/mirrored-half/src/mirrored_half/reconcile/` *(all 27 files, 14,888 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/reconcile.rs` *(714 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/inverse/` *(all 12 files, 5,828 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/inverse.rs` *(1,428 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/matrix_addition.rs` *(1,093 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/mirror_build.rs` *(725 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/derive.rs` *(410 LOC)*
- `core/crates/mirrored-half/src/mirrored_half.rs` *(1,388 LOC)*
- `core/crates/mirrored-half/src/mirrored_half/tests.rs` *(273 LOC)*
- `core/crates/kernel/src/migrate.rs` *(~867 LOC — pre-format-8 migrations)*
- `core/crates/kernel/src/matrix/layout/components.rs` *(712 LOC — duplicate same-board layout sync; replaced by unified `project.rs`)*

### Legacy Narrative Test Suites to Delete
- `core/crates/engine/tests/mirrored_reconcile_a/` & `mirrored_reconcile_a.rs` *(2,903 LOC)*
- `core/crates/engine/tests/mirrored_reconcile_b/` & `mirrored_reconcile_b.rs` *(732 LOC)*
- `core/crates/engine/tests/mirrored_prepare/` & `mirrored_prepare.rs` *(3,400 LOC)*
- `core/crates/engine/tests/mirrored_lifecycle/` & `mirrored_lifecycle.rs` *(2,557 LOC)*
- `core/crates/engine/tests/mirrored_matrix_addition.rs` *(864 LOC)*
- `core/tests/mirrored_half/` & `core/tests/mirrored_half.rs` *(11,164 LOC)*
- `core/tests/native_matrix_exceptions/` *(1,164 LOC)*

### Forbidden Symbols (Must Have 0 Matches in `*.rs`)
- `reconcile_pairs`
- `reconcile_canonical`
- `prepare_admitted_transaction`
- `prepare_mirror_transaction`
- `canonical_request`
- `materialize_fixed_matrix`
- `fixed_membership_view`
- `restore_local_matrix_companion_parts`
- `changed_document_ids`

---

## 5. Replacement Verification Suite (≤ 2,000 Test LOC Total)

Replace the deleted 30,600 LOC of narrative mirror tests with **one clean invariant suite** in `core/crates/mirrored-half/src/invariants_tests.rs` + **one focused integration test** in `core/crates/engine/tests/derived_mirror.rs` proving:

1. **Pure Reflection Determinism:** For any sequence of source edits (parts, matrices, rows/cols, disabled cells, outlines, mechanical settings, hosted modules, keycaps), every Linked mirror entity in `project_document(doc)` equals the exact geometric reflection of its source.
2. **Idempotence & Save/Reopen Exactness:** `serialize(canonical) -> deserialize -> project_document` produces byte-for-byte identical canonical JSON and identical resolved geometry; saved JSON contains zero materialized Linked mirror items.
3. **Format-8 Admission:** Any `formatVersion != 8` document is rejected cleanly before mutating engine state.
4. **Source-Only Guard:** Any edit targeting a Linked mirror ID fails atomically (document & Undo history unchanged) and returns `MirrorEditRefused::LinkedTarget` naming the source ID.
5. **Local / Omit / Rejoin / Companion Tweak Lifecycle:**
   - `MakeLocal` captures an independent override that survives subsequent source moves AND source deletion.
   - `Omit` removes the mirror entity from `ResolvedDoc` (and refuses cleanly if active routed copper/constraints reference it, naming the blocker).
   - `Rejoin` deletes the `Local`/`Omitted` override so the entity immediately re-projects from source.
   - `Unlink` materializes remaining mirror items into ordinary independent entities and removes the pair.
   - `SetCompanionPose` overrides only that companion's local pose while keeping parent key/matrix linked.
6. **Same-Board & Two-Board Parity:** Same invariants hold for both `MirrorDestination::SecondBoard` and `MirrorDestination::SameBoardLayout`, plus reversible single-board assemblies.
7. **Exact Undo / Redo:** Random seeded operation sequences (e.g., 32 seeds × 40 steps = 1,280 steps in <2s) verify Undo/Redo restores exact `canonical` and `resolved` state at every step.

---

## 6. Banned Patterns (Automatic Rejection)

- **DO NOT** keep expanded mirror items in `ProjectDoc` and strip them only in `archive.rs`. That is the exact bug that caused `refactor/simplify-implementation-wave` to fail.
- **DO NOT** keep any file under `reconcile/` or `inverse/` "just for Open" or "just for ReplaceDocument".
- **DO NOT** copy-paste deleted tests from `core/tests/mirrored_half/` into new files.
- **DO NOT** add v7 migration, repair dialogs, or recovery stores.

---

## 7. Verification & Mechanical Gate Commands

Run under the shared build lock (`~/.cache/tmp/boardstudio-build.lock`, `BOARDSTUDIO_BUILD_LOCK_FD=9`, `TMPDIR="$HOME/.cache/tmp"`):

```bash
# 1. Mechanical Reduction Gate (MUST exit 0)
python3 docs/plans/clean-slate-reduction/check_reduction.py \
  --base 5aae86ff4 \
  --head HEAD \
  --max-prod-delta -16000 \
  --max-test-delta -27000 \
  --max-script-delta 0 \
  --max-other-delta 200 \
  --must-delete core/crates/mirrored-half/src/mirrored_half/reconcile \
  --must-delete core/crates/mirrored-half/src/mirrored_half/reconcile.rs \
  --must-delete core/crates/mirrored-half/src/mirrored_half/inverse \
  --must-delete core/crates/mirrored-half/src/mirrored_half/inverse.rs \
  --must-delete core/crates/mirrored-half/src/mirrored_half/matrix_addition.rs \
  --must-delete core/crates/mirrored-half/src/mirrored_half/mirror_build.rs \
  --must-delete core/crates/kernel/src/migrate.rs \
  --must-delete core/crates/kernel/src/matrix/layout/components.rs \
  --must-delete core/crates/engine/tests/mirrored_reconcile_a \
  --must-delete core/crates/engine/tests/mirrored_reconcile_b \
  --must-delete core/crates/engine/tests/mirrored_prepare \
  --must-delete core/crates/engine/tests/mirrored_lifecycle \
  --must-delete core/tests/mirrored_half \
  --must-delete core/tests/native_matrix_exceptions \
  --forbid-symbol reconcile_pairs \
  --forbid-symbol reconcile_canonical \
  --forbid-symbol canonical_request \
  --forbid-symbol materialize_fixed_matrix \
  --forbid-symbol restore_local_matrix_companion_parts \
  --forbid-symbol changed_document_ids

# 2. Focused native checks (inside worktree)
cargo fmt --all --check
cargo test -p boardstudio-core-mirrored-half -p boardstudio-core-engine -p boardstudio-core -p boardstudio-application
cargo check --workspace --exclude boardstudio-web
cargo check -p boardstudio-web --target wasm32-unknown-unknown --no-default-features --features page
```
