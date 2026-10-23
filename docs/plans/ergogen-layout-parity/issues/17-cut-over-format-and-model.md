# 17: Cut over the persisted layout model and every removed-field caller

Status: resolved
Type: build
Blocked by: [16](16-build-layout-kernel.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Split into: [17a](17a-grouped-model-and-core-callers.md) · [17b](17b-validation-and-ownership.md) · [17c](17c-kernel-production-routing.md) · [17d](17d-entry-path-gates.md) · [17e](17e-fixtures-and-final-gate.md)

## Outcome

Perform the coordinated model/caller cutover in the unshipped integration lane.
Replace parallel layout vectors and singular Layout ownership, route production math
to the kernel, and introduce the controlled version-7 gate. Do not promote a user-data
writer before ticket 27. Tests use isolated stores; no compatibility aliases/migration.

## First RED / GREEN slice

Fail a public Core Open/serialize/projection case for one valid grouped-layout v7
matrix plus refusal of the old input. Introduce the smallest complete schema/reader
path to pass, then add the next frame/default/ownership case. A constant-only bump
must not reach the old migration loop's `unreachable!`.

## Coordinated acceptance

- Persist column `rotation` as well as Spread/Stagger/Splay/origin/offset, row records,
  cell Shift/Bend and optional single-key anchor; replace Layout with `matrixIds`.
  Extend the same-board follower link's explicit correspondence/exception owner as
  specified; do not fake a physical pair or infer matrix pairs from list order.
- Remove fields to inventory compiler errors, then update every caller together:
  Core model/validation/materialization, matrix layout/splay/deletion, mirror inverse/
  reconcile, geometry/outline ownership, boundary projection, web tree/selection,
  setup/Inspector/transform consumers and their constructors/fixtures. No old formula
  survives as a fallback. No silent acceptance of an unresolved anchor as world origin.
- Apply finite/default/list-bound checks, sparse defaults, negative/zero step policy,
  growth/truncation and nonempty logical Layout ownership. Preserve unrelated data.
- Gate browser list/load, archive unpack and direct Core Open before migration or
  replacement effects. Keep `boardstudio/v2`, DB version 1 and archive metadata.
  Missing/older/newer/malformed versions fail through the specified controlled paths.
- Route draft/actual projection and materialized parts through the staged kernel.
  Retire the old TypeScript oracle only with replacement coverage active. Regenerate
  the minimum affected baseline schema/scene fixtures deliberately; do not weaken tests.
- Keep current storage transaction, identity, companion, scan/keymap and history
  contracts intact. Remaining feature tickets must complete before this lane ships.

## Checks

Source anchors: `core/src/model.rs`, `migrate.rs`, `matrix.rs`, `matrix/layout.rs`,
`validate.rs`, `geometry.rs`, `outline_versions.rs`, `mirrored_half/`,
`web/crates/host/src/host/storage.rs`, `web/crates/ui-model/src/tree.rs`.
Use compiler-driven inventory, focused public tests and `check.py quick --files` for
all changed callers. Require native/contract and WASM compilation, not just serde tests.

## Progress

### Implementation checkpoints — 2026-10-23

- [x] A — grouped-v7 records and `matrixIds` reach the real Core Open,
  serialization and projection path. The retained
  `grouped_v7_records_and_layout_membership_round_trip_through_core` tracer failed
  with `missing field matrixId` (`06-tracer-red.log`) and passes in
  `.evidence/ergogen-17/15-tracer-green.log`. Its authored numeric literals now use
  floating-point JSON numbers so exact serialization comparisons test the schema,
  not serde_json's integer/float representation distinction. Shift/Bend and anchor
  records are persisted; document evaluation refuses unresolved anchor contexts.
- [ ] B — caller cutover checkpointed, not accepted: native workspace compiled
  (`af85a2a9`); production callers/oracle replacement are in `570bb2672` and the
  subsequent WIP checkpoint. `.evidence/ergogen-17/42-core.log` has **801 unit
  passed**, four ignored, and **729 integration passed / five failed**. Linked
  column deletion and four linked-outline cases must turn GREEN before more
  caller work. WASM/browser verification remains pending.
- [ ] C — partial validation checkpoint (`8b367d8c7`, `eb82b77db` and subsequent
  WIP): all **15 document-integrity tests pass** in `40-document-followup.log`,
  including invalid lists, unresolved anchors, duplicate correspondences, explicit
  steps and logical Omitted anchors with Undo/Redo. Linked compaction metadata
  still fails the broader integration run; resize/ownership closure is pending.
- [ ] D — strict raw browser/archive/direct-Open admission is implemented in the
  WIP checkpoint; direct refusal/history and archive tests passed in focused runs.
  Real BrowserStore mixed-version preservation coverage is added but unrun.
- [ ] E — production replacement covers all **381 committed parity poses** through
  actual scenes, draft projections and materialized parts (`33-production-parity.log`).
  Boundary transcript rebuilt deliberately (`36-boundary-rebuild.log`); final
  changed-file quick receipt and final non-WIP commit are still missing.

The WIP checkpoint preserves the accumulated implementation and passing focused
evidence, not a green whole-ticket state.

- [x] B/C linked-split regression follow-up: `4db0c548` checkpoints the accumulated
  cutover; `98cad6d0` maintains explicit member identities through compaction and
  resize. The five failures from `42-core.log` subsequently pass (734 integration
  passed in `43-integration.log`). The new public resize/reopen case failed on stale
  ownership in `44-resize-red.log`, then passed in `45-resize-green.log`.
- [x] B grouped-outline follow-up: a public anchored two-matrix-per-Layout fixture
  exposed automatic bridging across the split when a child's relative Origin was
  used to classify the entire group. `48-outline-red.log` observes one contour
  instead of two; `49-outline-green.log` passes after classifying by unanchored
  logical roots. Admission is not weakened and logical Omitted roots remain usable.
- [ ] Verification closure: `46-quick.log` was interrupted; `47-quick.log` passes
  native lint but stops at WASM lint on a redundant String conversion in the new
  BrowserStore test, now corrected. Neither is an acceptance receipt. Replayed
  manual launches interleaved some earlier logs; require a fresh scheduler receipt
  rather than treating those logs as final evidence.

Checkpoint success is not ticket acceptance or approval to use user-data stores.

The acceptance is a multi-session coordinated cutover. Planned green increments:

1. Public Core Open/serialization/projection tracer: grouped records, kernel-backed
   production selection, all removed-field callers, and the controlled v7 reader.
   Remove fields and use compiler errors as the caller inventory; no compatibility
   aliases or alternate production geometry engine.
2. Extend frame/default/ownership and same-board correspondence coverage, including
   sparse records, axis growth/removal, invisible logical contributors and unresolved
   anchor refusal. Preserve existing identity/history/companion contracts.
3. Close browser list/load and archive strict gates and deliberately rebuild the
   minimum affected fixtures; verify native/contract and WASM compilation through
   the changed-file quick scheduler.

Each implementation increment requires observed behavioral RED/GREEN, a local
commit and changed-file quick evidence. Status remains claimed for orchestrator
review; no intermediate writer is approved for user-data stores.

### 2026-10-23 — prerequisite checkpoint, not the model cutover

- Temporarily removed the five Matrix axis vectors and `Layout.matrix_id` to
  inventory callers. `cargo check -p boardstudio_core` reported 162 expected
  compiler errors. Read the inventory at
  `.evidence/ergogen-17/removed-fields-inventory.log`; it reaches `matrix.rs`,
  `matrix/{deletion,layout,placement,splay}.rs`, `mirrored_half.rs`,
  `mirrored_half/{inverse,inverse/layout,matrix_addition,reconcile}.rs`,
  `geometry.rs`, `outline_cleanup.rs`, `mechanical/gasket.rs`, `model.rs` and
  `lib.rs`. Restored only this temporary edit exactly. This is a first Core
  inventory, not an exhaustive web/test caller inventory or behavioral TDD RED.
- Chose a smaller prerequisite checkpoint before the planned grouped-v7 tracer:
  direct Core Open and replacement now distinguish older/no-conversion,
  newer/compatible-build and invalid-zero versions instead of instructing callers
  to migrate. Reused one crate-private version admission function; its callers
  retain their existing format-identifier gates and check before replacement effects.
  No new public interface, glossary term, dependency or geometry engine.
- Public JSON Core regression first failed on the old Open message, `Document
  must be migrated before it is opened` (expected RED), then passed. It exercises
  Open and committed replacement, older/newer/zero/malformed versions, accepted
  state preservation and functioning Undo/Redo. Existing current-schema and legacy
  direct-Open assertions now require the distinct refusal explanations.
  Logs: `.evidence/ergogen-17/01-core-refusal-red.log` and
  `.evidence/ergogen-17/02-core-refusal-green.log` (one passed test);
  `.evidence/ergogen-17/03-document-integrity-green.log` is the focused module run.
- The running format remains **6**. The existing browser/archive migration routes
  are unchanged. This checkpoint does **not** satisfy the first grouped-v7 tracer,
  activate grouped records or anchors, prove production invisible contributors,
  implement Layout membership/correspondences, or deliver the v7 strict gate.
  All three planned increments above remain open. The prerequisite's narrower
  ordering is a session-scope judgment, not a relaxation of coordinated acceptance.
- Verification for this checkpoint: focused document-integrity run **9 passed**,
  zero failed; changed-file quick **1,960 native passed** (nine existing ignored)
  and **587 WASM passed**, zero failed, including native/WASM lint and typecheck.
  Receipt: `.evidence/ergogen-17/quick-receipt.json` and `.json.md`.
  Log: `.evidence/ergogen-17/quick.log`, ending
  `check: all steps passed: repo, lint, typecheck, test, layout-test-css, browser`.
  This is changed-file verification, not full ticket acceptance, the page suite or
  provider/product smoke. Fixtures and the untracked prototype were untouched;
  no npm command, storage write, stash or push was performed.

## Answer

Resolved on 2026-10-23 in the coordinated, unshipped integration lane. All five
subtickets are resolved: [17a](17a-grouped-model-and-core-callers.md#answer),
[17b](17b-validation-and-ownership.md#answer),
[17c](17c-kernel-production-routing.md#answer),
[17d](17d-entry-path-gates.md#answer) and
[17e](17e-fixtures-and-final-gate.md#answer). Earlier Progress subsections are
historical checkpoints, superseded for acceptance by this Answer.

The grouped v7 records, plural Layout membership and explicit correspondences reach
the removed-field Core/web callers. Draft/actual projection and materialized parts
use the kernel; validation bounds run before dimension walks, unresolved anchors
are refused, and browser/archive/direct Core entry paths enforce controlled format
admission without migration. Replacement oracle and invisible-contributor coverage
are active; minimum affected fixtures retain their intended scenarios. Existing
identity, companion, scan/keymap, storage and history contracts remain covered.
The final companion arithmetic and scoped Column rotation findings were corrected
with failing-first regressions rather than weakening those contracts.

[17e's final gate](17e-fixtures-and-final-gate.md#answer) records all 119 paths since
fixed base `a58a2bf187bd6d08b652dc739cd7c391709ff5c8`, tested source
`47ce298020ef72903bab4032d5ee5980b7779215`, quick **54/54**, full **55/55**, native
**2,150 matched**, 14 WASM launches, page **77/77**, CAD checks and packaged
drag/collision **13/13**. Independent final Standards and Spec re-reviews at
`f67cd2b48217c1206d1c187cdfc13644da365cd2` both passed with no remaining findings.
Subsequent closure changes are documentation only; the untracked prototype is
untouched.

This closes the model/caller cutover, not the remaining feature/recovery/example
work or release gate. [18](18-core-placement-edits.md) and
[25](25-project-format-recovery.md) are now unblocked and may run in separate lanes;
neither is claimed here. Tickets 18–27 must finish before this lane ships. No release
or user-data writer approval is granted.
