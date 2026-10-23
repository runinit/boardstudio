# 17e: Fixtures and final gate

Status: claimed
Type: build
Blocked by: [17b](17b-validation-and-ownership.md), [17c](17c-kernel-production-routing.md), [17d](17d-entry-path-gates.md)
Parent: [17](17-cut-over-format-and-model.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns fixture regeneration and the changed-file and full gates for the cutover; the example/starter rebuilds are left to [26](26-rebuild-examples-and-fixtures.md) and the end-to-end checklist and release gate to [27](27-end-to-end-acceptance.md).

## Outcome

Close ticket 17: regenerate the minimum affected fixtures, run the gates and review.

## Acceptance

- Regenerate the minimum affected baseline schema/scene fixtures deliberately; do not weaken tests.
- Changed-file quick gate and the full check gate pass.
- Standards and Spec review before closure.
- Keep current storage transaction, identity, companion, scan/keymap and history contracts intact. Remaining feature tickets must complete before this lane ships.

## Checks

Use `check.py quick --files` for all changed callers and the full check gate; require native/contract and WASM compilation, not just serde tests.

## Progress

### Implementation claim — 2026-10-23

- Verified branch `docs/ergogen-layout-comparison` at pinned
  `42abce93c3854f3f76de43f0a0b26cc6e71f3e81`; 17a–17d headers are resolved.
  Only preexisting untracked `web/crates/layout/prototype/` is present; it is excluded.
- Plan: fix the review base before grouped-field replacement using Git history and
  17a's Answer; inventory the entire coordinated range, audit schema/scene/boundary
  fixtures and reuse deliberate 17a rebuilds where sufficient. Change only fixtures
  with a spec justification; leave examples/starters to 26 and release/E2E to 27.
- Then run fresh all-caller changed-file quick followed, only if green, by the
  supported full pre-PR scheduler. Every heavy command uses the shared flock,
  22G memory/no-swap scope, 3600-second timeout, absolute TMPDIR and two test threads.
  Keep native/WASM baselines unchanged; stop on first failure or uncertain drift.
- Record exact inventory, commands, receipts and observed/unreached scopes here.
  Leave this ticket and parent 17 claimed for independent Standards+Spec reviews;
  no source fixes, later claims, shipping or user-data writer approval.

### Audit checkpoint — blocked before gates, 2026-10-23

Claim commit: `3259619`. Fixed coordinated review base:
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`; audited implementation head:
`42abce93c3854f3f76de43f0a0b26cc6e71f3e81`. The base is the direct parent
of `13e086e40db1eba2c47ba1dfef2bee609c27b040`, the first persisted grouped-field
replacement (`git show --stat 13e086e40`, `git rev-parse 13e086e40^`).
17a's Answer starts at `b159d3d5f`, the later ticket split; Git first-parent history
shows the model/caller cutover already preceding it. Using that Answer's start or
17c's diff alone would omit coordinated callers. No `.codegraph/` directory exists.

**STOP: uncertain boundary fixture drift / lost scenario coverage.** Read-only
semantic comparison of the fixed base and head JSON found the first success/refusal
drift at `core[61:convert-preview]` in
`core/tests/fixtures/boundary/transcript.json`: `kind: preview` became `kind: error`,
message `Converting requires zero Splay, Column rotation and carried Bend`, revision
43. `core[62:convert-commit]` likewise changed `scene` to that error. Spec §6 supports
the stricter refusal for nonzero Splay; it does not justify losing later scenarios.
`core/tests/support/boundary.rs:840` still describes column 1 as convertible despite
its Splay; requests at lines 860–880 still assume the conversion advanced revision.
Consequently `convert-offset-2`, `convert-refused`, `mirrored-half-new-board` and
`mirrored-half-reuse-board` now record `Stale base revision`. The latter two previously
reached `Physical instance "board" is unavailable.` The golden comparison can pass
while these intended seam scenarios are no longer exercised. This is source/fixture
inspection evidence, not an executed regression or a new runtime failure.

Minimal proposed next action (not implemented): authorize a fixture-support correction
in `core/tests/support/boundary.rs`, preserving a representable zero-turn successful
conversion preview/commit and the explicit nonzero-turn refusal, assert each intended
reply kind/reason before blessing, and keep subsequent requests at valid revisions so
both mirrored-half cases reach their physical-instance refusal. Then use the existing
`BOUNDARY_BLESS=1 cargo test -p boardstudio_core --test boundary_parity
native_transport_reproduces_the_golden_transcript` rebuild under the required cap,
inspect its semantic delta, replay without blessing, and resume the all-caller gates.
Do not change production code or the native/WASM baselines to hide this drift.

Fixture decisions before the stop:

- `matrix-projections.json`: retain 17a's 90 field corrections, justified by spec
  §§2/7/10 and ticket 04's carried custom origins, whole mirrored headings and logical
  contributors without rendered member IDs. 17c restored direct ordinary coverage of
  all 9 cases/81 cells alongside public Local and 381-pose production coverage. No
  tolerance/case/assertion changes or further regeneration performed here.
- `synthetic-outline.json`: prior grouped `matrixIds` and format 7 satisfy §§2/9;
  explicit cell rotation 27 preserves the authored rotated key after Open refresh.
  Retain pending the fresh gates; no new blessing.
- `paired-rejoin-preview.json`: prior single-member `matrixIds` lists and format 7
  satisfy §§2/9; no new pose expectation or rebuild here.
- `adoption-reachability.json`: prior positive documents changed format 6 to 7 (§9);
  Unicode escaping and pretty-print changes are JSON-semantic no-ops. No CAD bounds,
  material probe or hardware expectation changes found; no regeneration here.
- Boundary rebuilds already exist in `f59e3a856` and `4db0c5484`. Added cell frames,
  column bases/axes/relative origins/rotation follow §7; grouped records/version and
  archive digests follow §§2/9. The conversion/revision cascade above prevents
  accepting the entire rebuilt golden as semantically justified.
- Intentional legacy-refusal inputs remain (including `legacy_archive_migration.rs`'s
  unversioned negative archive). Example/starter rebuilds and their goldens remain
  ticket 26; release/E2E remains 27. No generator/dependency install or upgrade run.

Verification state: quick **not run**; full **not run**, because the mandatory audit
stop occurred first. No native tests (including the 2,149 baseline names), WASM
compilation/runtimes or browser stages executed in this task. Prior 17c receipts
remain prior evidence only. No `quick.log`, `quick-receipt.json`, `full.log` or
`full-receipt.json` was generated; no success final lines exist for these gates.
No baseline or fixture was edited. Parent 17 remains claimed; 17e remains claimed
for resolution of this blocker followed by gates and independent Standards+Spec
review. No later ticket was claimed and no shipping/user-data writer is approved.

Documentation verification on this progress increment:

```sh
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/check-doc-links.py
git diff --check
```

Observed: `Documentation links check passed: 554 Markdown files.`; `git diff --check`
exited 0 with no output. These are documentation checks only, not quick/full acceptance.

### Complete coordinated changed-path inventory

Derived with `git diff --name-only` between the exact fixed base/head above; includes
all changed source/test/fixture/baseline/docs paths, including Core and web. The
future quick command must pass **every existing path below** to `--files` (plus this
ticket's progress changes), not only 17e's documentation. No deleted paths occur.

```text
README.md
application/tests/durable_session.rs
application/tests/outline_queue_orders.rs
core/src/archive.rs
core/src/geometry.rs
core/src/lib.rs
core/src/matrix.rs
core/src/matrix/deletion.rs
core/src/matrix/evaluation.rs
core/src/matrix/kernel.rs
core/src/matrix/layout.rs
core/src/matrix/placement.rs
core/src/matrix/splay.rs
core/src/mechanical/clearance.rs
core/src/mechanical/gasket.rs
core/src/mechanical/halves.rs
core/src/migrate.rs
core/src/mirrored_half.rs
core/src/mirrored_half/inverse.rs
core/src/mirrored_half/inverse/layout.rs
core/src/mirrored_half/inverse/tests.rs
core/src/mirrored_half/lifecycle/tests.rs
core/src/mirrored_half/matrix_addition.rs
core/src/mirrored_half/matrix_addition/tests.rs
core/src/mirrored_half/reconcile.rs
core/src/mirrored_half/reconcile/prepare.rs
core/src/mirrored_half/reconcile/tests.rs
core/src/model.rs
core/src/outline_cleanup.rs
core/src/validate.rs
core/tests/closure_generator_parameters.rs
core/tests/contracts.rs
core/tests/core.rs
core/tests/document_integrity.rs
core/tests/electrical_wiring.rs
core/tests/fixtures/adoption-reachability.json
core/tests/fixtures/boundary/transcript.json
core/tests/fixtures/matrix-projections.json
core/tests/fixtures/synthetic-outline.json
core/tests/gasket_features.rs
core/tests/generator_parameters.rs
core/tests/key_move_assemblies.rs
core/tests/legacy_archive_migration.rs
core/tests/matrix_companions.rs
core/tests/matrix_kernel.rs
core/tests/matrix_performance.rs
core/tests/matrix_placement.rs
core/tests/mirrored_half.rs
core/tests/outline_linked_resolution.rs
core/tests/outline_persistence.rs
core/tests/outline_repairs.rs
core/tests/outlines.rs
core/tests/part_offset_constraint.rs
core/tests/splay_companions.rs
core/tests/splay_reconciliation.rs
core/tests/support/boundary.rs
core/tests/support/synthetic_half.rs
docs/plans/ergogen-layout-parity/issues/17-cut-over-format-and-model.md
docs/plans/ergogen-layout-parity/issues/17a-grouped-model-and-core-callers.md
docs/plans/ergogen-layout-parity/issues/17b-validation-and-ownership.md
docs/plans/ergogen-layout-parity/issues/17c-kernel-production-routing.md
docs/plans/ergogen-layout-parity/issues/17d-entry-path-gates.md
docs/plans/ergogen-layout-parity/issues/17e-fixtures-and-final-gate.md
docs/plans/ergogen-layout-parity/issues/18-core-placement-edits.md
docs/plans/ergogen-layout-parity/issues/19-anchor-layout-groups.md
docs/plans/ergogen-layout-parity/issues/20-reflect-linked-layouts.md
docs/plans/ergogen-layout-parity/issues/21-exception-lifecycle.md
docs/plans/ergogen-layout-parity/issues/22-update-layout-consumers.md
docs/plans/ergogen-layout-parity/issues/23-inspector-toolbar-and-handles.md
docs/plans/ergogen-layout-parity/issues/25-project-format-recovery.md
docs/plans/ergogen-layout-parity/issues/26-rebuild-examples-and-fixtures.md
docs/plans/ergogen-layout-parity/issues/27-end-to-end-acceptance.md
docs/plans/ergogen-layout-parity/map.md
scripts/check-native-tests-baseline.json
web/crates/case/src/case_selection.rs
web/crates/case/src/mechanical_session_tests.rs
web/crates/case/src/mechanical_settings_controller.rs
web/crates/catalogue/src/matrix_setup_operation.rs
web/crates/catalogue/src/physical_setup.rs
web/crates/host/src/host/storage_lifetime_tests.rs
web/crates/layout/src/matrix_relations.rs
web/crates/layout/src/matrix_relations/tests.rs
web/crates/layout/src/matrix_transform_lifecycle.rs
web/crates/layout/src/matrix_transform_operation.rs
web/crates/layout/src/mirrored_pair_geometry.rs
web/crates/layout/src/objects.rs
web/crates/layout/src/objects/existing_half.rs
web/crates/layout/src/objects/keycap_resize.rs
web/crates/layout/src/objects/keycap_size_tests.rs
web/crates/layout/src/objects/layout_align.rs
web/crates/layout/src/objects/layout_align_controller.rs
web/crates/layout/src/objects/layout_align_geometry.rs
web/crates/layout/src/objects/layout_transform_toolbar.rs
web/crates/layout/src/objects/matrix_inspector_controller.rs
web/crates/layout/src/objects/matrix_placement_controller.rs
web/crates/layout/src/objects/matrix_setup_controller.rs
web/crates/layout/src/objects/matrix_transform_controller.rs
web/crates/layout/src/objects/matrix_transform_inspector_tests.rs
web/crates/layout/src/outline_lifecycle_browser_tests.rs
web/crates/layout/src/outline_lifecycle_tests.rs
web/crates/layout/src/part_placement.rs
web/crates/parts/src/parts/assembly_presets.rs
web/crates/pcb/src/pcb_wiring.rs
web/crates/pcb/src/pcb_wiring/mode_owner_tests.rs
web/crates/pcb/src/pcb_wiring/part_input_settings.rs
web/crates/runtime/src/runtime.rs
web/crates/ui-model/src/tree.rs
web/crates/ui-model/src/tree_tests.rs
web/src/presentation.rs
web/src/presentation/context_summary.rs
web/src/presentation/layout_canvas.rs
web/src/presentation/layout_component_inspector_tests.rs
web/src/presentation/layout_key_selection_actions/tests.rs
web/src/presentation/layout_layers_fixtures/paired-rejoin-preview.json
```
