# 162: Bound STEP occurrence expansion before geometry work

Status: claimed
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [CAD 007](../../cad-backend-replacement/issues/007-strict-step-import.md), [CAD 010](../../cad-backend-replacement/issues/010-production-cache-worker-browser.md)

## What to build

Importing a small STEP file with many nested mapped copies must either deliver a
complete model within a finite work budget or return a clear resource-limit error.
The CAD worker must remain usable after refusal. Keep ordinary mapped assemblies
and their source occurrence identities intact.

The 2026-10-18 audit at `f955050c3` reproduced 2, 16 and 256 occurrences at mapping
depths 1, 4 and 8. The depth-eight file was only 4,066 bytes. Each level references
the preceding representation twice, so file-size and nesting-depth guards do not
bound the total expanded geometry. The probe demonstrated expansion; it did not
attempt to exhaust memory.

## Acceptance criteria

- [ ] A regression fails on the current importer because an over-budget mapped
  representation is accepted. Use a bounded synthetic fixture and a deterministic
  refusal oracle rather than attempting memory exhaustion or asserting elapsed time.
- [ ] A finite import-wide budget limits traversal work and emitted occurrences
  before excess instances or their geometry are materialized. It covers branching,
  multiple roots and assembly paths; starting another root does not reset it.
- [ ] Record the chosen limits and their units. Keep enforcement private to import
  admission and preserve existing cycle/depth refusals and completeness checks.
- [ ] Refusal names the exceeded limit and reaches the caller through the normal
  import error path. No partial model is returned as successful geometry.
- [ ] Supported repeated and nested mapped assemblies retain their counts, placement,
  source provenance and mesh coverage below the limits.
- [ ] An actual CAD-worker regression submits an over-budget asset, observes failure,
  then imports an ordinary asset successfully using the same worker.
- [ ] Run affected native STEP/import tests in release mode and the focused worker
  check. Complete one combined Standards+Spec review of the finished behavior under
  the CAD review policy, and record exact results and remaining limits.

## Comments

Created from audit finding 1 at the user's request. This ticket is independent of
repair-movement certification and the other three audit follow-ups.


### CURRENT — 2026-10-18 implementation stream

- Implemented private admission shared by all standalone roots and assembly paths:
  **1,024 solid/curve-set occurrences** and **16,384 traversal steps** per import.
  One step is a representation visit, item visit, assembly path or assembly edge.
  These conservative preview limits stop branching before excess occurrence tuples
  or derived geometry; they are not allocator/process-memory certification.
  Existing depth/cycle, completeness, placement and source-identity checks remain.
- Added the 4,436-byte `mapped-over-budget.step` fixture (eleven binary mapping
  levels, 2,048 tetrahedron occurrences), reconstructed from committed
  `mapped-occurrence.step`, plus six public-import regressions. Coverage includes
  placement-only branching, shared budgets across roots and assembly paths, and
  exact admission at 1,024 with original bytes, source faces, placement, triangle
  coverage and Unknown accuracy preserved.
- RED: `cargo test --release --manifest-path cad/monstertruck/Cargo.toml --test step_occurrence_budget`
  failed because the original importer **accepted** the over-budget asset.
  `cargo test --release --locked --manifest-path cad/monstertruck/Cargo.toml --test step_occurrence_budget branching_placement_only`
  also failed by accepting placement-only branching before traversal accounting
  was enabled. Logs: `.evidence/ticket-162/red-occurrences.log` and `red-traversal.log`.
- GREEN: `cargo test --release --locked --manifest-path cad/monstertruck/Cargo.toml --test step_occurrence_budget`
  **6 passed**; same command with `--test step_import` **61 passed, 1 existing
  ignored** (`corpus_faces_without_a_uv_chart_are_meshed_from_shared_samples`,
  vendor corpus by repo path). No tests removed or renamed. Logs: `green-budget.log`
  and `green-step-import.log` in `.evidence/ticket-162/`.
- Added an actual Host CAD-worker regression: authenticated over-budget import
  fails with the named resource limit and no successful result, then the same
  worker imports the ordinary mapped fixture with source SHA, face coverage and
  Unknown accuracy retained. It also runs in the regular CAD browser smoke.
  Focused command: `python3 cad/scripts/test-cad-browser.py --occurrence-budget`.
  Attempt blocked at prerequisite: **`cad/wasm/pkg is missing; run python3 cad/scripts/build-cad-wasm.py first`**
  (`.evidence/ticket-162/worker.log`). Parent must run it with freshly built CAD WEB
  bindings and `web/target/providers/cad-worker/{m1_cad_worker.js,m1_cad_worker_bg.wasm}`.
- `cargo clippy --release --locked --manifest-path cad/monstertruck/Cargo.toml --lib --test step_occurrence_budget -- -D warnings`
  passed (`.evidence/ticket-162/clippy.log`). Cargo reported the existing
  `proc-macro-error2 v2.0.1` future-incompatibility notice.
- `node --check cad/scripts/browser-smoke/qualification.mjs`,
  `python3 -m py_compile cad/scripts/test-cad-browser.py`, and `git diff --check` passed.
- Target inspection: root `target` absent (no root Cargo/wasm-pack invocation);
  CAD native target resolves inside this isolated worktree. No shared target builds.
- Combined Standards+Spec review and final full suite remain **pending with parent**;
  no reviewer spawned in this stream. Status stays claimed; acceptance is not closed.
