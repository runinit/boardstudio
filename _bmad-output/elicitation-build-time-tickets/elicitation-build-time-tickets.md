# Build-time backlog tickets

## Round 1 — applied

Reviewed both standalone tickets at `73890746edb04250619aa648a3609718c0d32c11`. The user approved all four changes. The passages below record the accepted revision.

1. Applied — **Story, requirement 2 — Failure Mode Analysis.** Replace the opening of the criterion with: “List the inputs used to build the pair and the inputs used only to validate workspace compatibility. For each build input, show which part of the key covers it. Record why unrelated workspace lock entries can be excluded. Changing the patch, pinned tool sources, compiler identity, target OS or architecture, toolchain configuration or relevant Cargo configuration must change the key.” Retain the genuine-miss check.
2. Applied — **Story, verification — Subtraction.** Replace the verification paragraph with: “From the repository root, run `python3 scripts/test-build-patched-wasm-bindgen.py` and `python3 scripts/test-ci-workflow.py`. Confirm the named controls exercise local hits and misses, independent key-input changes, an incompatible workspace lock with a prospective hit, and damage to each executable. Both suites must pass. Label any elapsed time as a local observation.” Remove the duplicate verification instruction under Notes.
3. Applied — **Spike, description and approach 2 — Map Is Not the Territory and Failure Mode Analysis.** Replace “Schedule it first” with: “The cache story has first priority. This experiment can run independently on the agreed local dev baseline.” Add to approach 2: “Record baseline `a6cfa5183` for both arms and the scheduling-only intervention in the overlap arm. Keep tool setup identical.” Preserve the existing matched conditions.
4. Applied — **Spike, approach 4 and acceptance criterion 2 — Failure Mode Analysis and Subtraction.** Replace approach 4 with: “Run one matched sequential/overlap pair. If saturation, lock waits, a coverage difference or negligible gain rules out adoption, stop. To recommend adoption after a promising first pair, run one matched overlap/sequential pair. Stop after that pair.” Begin criterion 2 with: “Report all four elapsed times if recommending overlap. Both pairs must favor overlap, and the smaller paired saving must exceed the larger difference between repeat runs of the same schedule. Otherwise retain sequential execution. Treat this as a local decision rule.” Keep the existing coverage conditions. Delete the archived timings from criterion 3. Keep their provenance here: 200.6s cold and 36.1s warm were packaged-only on archived `24363c25a`.

### Set aside

- Hard prerequisite on ticket 1: the spike can equalize tool setup on both arms independently. BMad uses `after` for prerequisites.
- Automatic switch to a post-story baseline: the approved experiment pins dev `a6cfa5183`. A new baseline would change the question and requires an explicit decision.
- Delete Cargo timing guidance: retain the primary-source reference for interpreting a dependency wait versus a CPU-slot wait.

### Push files

- round-1-push-1-failure-mode-analysis.md
- round-1-push-2-map-is-not-the-territory.md
- round-1-push-3-subtraction.md
