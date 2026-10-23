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

### Authorized boundary test-support correction — 2026-10-23

Narrow continuation from `e0fbe74a1ad5e29b3c40f075caba5983477802e3`, with
explicit authorization to edit only shared boundary support, its golden and this
ticket. Branch/HEAD and clean tracked state were verified first; preexisting
untracked `web/crates/layout/prototype/` remains untouched. Parent 17 and 17e stay
**claimed**. This clears the identified lost-scenario blocker for independent
review of test intent; it does not accept the cutover or resume expensive gates.

**Feedback loop and real RED.** Read the audit checkpoint, parent 17, spec §6,
README gate guidance, coding standards, relevant glossary terms and ADRs 0005,
0010 and 0034. The diagnosing-bugs/TDD seam is the already-authorized shared
`run` → `check_core` path, used by native and WASM transports before compaction
and before blessing. Added literal outcome/revision guards before changing any
request setup or golden. Separate minimization, repeated reproduction and a
multi-hypothesis search were omitted: the prior deterministic request-dependency
audit already isolates the failure, and retaining the chain is necessary to catch
the downstream lost scenarios. Runtime evidence confirmed that diagnosis.

The requested `--test boundary_parity` command failed at target selection:
`error: no test target named boundary_parity in boardstudio_core package`.
This is **not** RED. `core/Cargo.toml` has `autotests = false`; README documents
`--test integration`. Used that existing target and the exact existing test name,
without editing configuration or test names/baselines. Compiled with `--no-run`
first, then the unblessed focused run failed at the new intended-outcome assertion:

```text
assertion left == right failed: convert-preview:
{"id":"convert-preview","kind":"error","message":"Converting requires zero Splay, Column rotation and carried Bend","revision":43}
left: String("error")
right: "preview"
FAILED. 0 passed; 1 failed; 756 filtered out
```

This preceded golden comparison, so it is a real lost-scenario RED, not an existing
snapshot mismatch. Evidence: `.evidence/ergogen-17e/boundary-red.log` and
`boundary-red-compile-supported.log`. The invalid target attempt is retained in
`boundary-red-compile.log`.

**Smallest shared correction.** Reuse already-zero-turn column 0 for the first
Y-offset and its preview/commit conversion, instead of splayed column 1. Keep
column 2's nonzero Splay and separate refusal. No new requests or IDs. Keep fixed
base revisions 42/43/44/45, rather than deriving every request from current state;
assert each prerequisite success and revision. Spec §6 requires zero Splay,
Column rotation and carried Bend and a Y-only Stagger transfer with next-column
compensation. The committed result asserts column 0 Stagger `0 → 3`, its independent
Y-offset `3 → 0` (zero vector omitted by sparse serialization), and column 1 Stagger
`2 → -1`. All existing assertions remain; `stale-draft` additionally pins the exact
`Stale base revision` reason. New outcome assertions run in both shared transports.

Two intermediate pre-blessing assertion failures were corrected before writing the
golden: `boundary-bless.log` expected a serialized zero offset, but sparse output
omits it; `boundary-bless-supported.log` rejected empty key poses, but the existing
outline-prune deliberately removes all keys before this block. The speculative pose
assertion was removed; this slice proves conversion outcomes and persisted transfer,
not nonempty key-pose preservation. Neither attempt rewrote the fixture. These were
assertion-authoring corrections, not competing production failures or accepted drift.

Observed final outcomes (scene revisions for success; accepted revision for errors):

| ID | Intended result now observed |
|---|---|
| `convert-offset-1` | `scene`, revision 43; column 0 Y-offset 3 setup |
| `convert-preview` | `preview`, revision 43; zero-turn conversion allowed |
| `convert-commit` | `scene`, revision 44; real Stagger transfer, cleared offset |
| `convert-offset-2` | `scene`, revision 45; column 2 Y-offset 1 accepted |
| `convert-refused` | `error`, revision 45; exactly `Converting requires zero Splay, Column rotation and carried Bend` |
| `mirrored-half-new-board` | `error`, revision 45; exactly `Physical instance "board" is unavailable.` |
| `mirrored-half-reuse-board` | `error`, revision 45; exactly `Physical instance "board" is unavailable.` |

**Full semantic golden delta.** Compared every JSON section/entry against the prior
pinned golden and inspected the affected entries against fixed pre-cutover base
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`:

- Counts/order/IDs remain **67 Core + 11 archive** entries, with the same four
  auxiliary scenario blocks. Exactly **seven** Core entries change against HEAD;
  all other Core entries, archive content and auxiliary blocks are identical.
- `convert-offset-1`: only compact body digest changes, because the offset now
  belongs to column 0. It still accepts revision 42 and publishes 43.
- `convert-preview` and `convert-commit`: erroneous zero-turn refusal at accepted
  revision 43 becomes the original intended `preview` / `scene` outcome. Successful
  conversion advances the accepted document to 44; preview stays at 43. Their
  body digests and scene/document key summaries now describe successful replies.
- `convert-offset-2`: stale error at 43 becomes accepted `scene` at 45, consuming
  revision 44 after the real conversion. Its digest includes the column 2 offset
  and the new column 0/1 Stagger values; no request-revision constants changed.
- `convert-refused`: stale at 43 becomes the deliberate exact conversion refusal
  at 45. Relative to the pre-cutover base, the reason changes from the older
  default-origin pivot restriction to spec §6's zero-turn restriction.
- Both mirrored-half entries: stale at 43 becomes physical-instance refusal at 45.
  Their entire compact JSON, including body digests, exactly matches pre-cutover
  base. `stale-draft` remains identical across base/prior/new at revision 1.
- The successful conversion reply digests differ from pre-cutover base because of
  the grouped-v7/kernel cutover already audited above and this intentional column
  selection/transfer. No unrelated new output delta was found or blessed.

Complete entry-by-entry old/base/new comparison:
`.evidence/ergogen-17e/boundary-semantic-delta.log`.

**Supported focused commands and GREEN.** All Cargo commands took the shared lock,
used absolute TMPDIR and `RUST_TEST_THREADS=2`, and ran with a 900-second timeout.
Compile-only used the authorized 22G cap; focused executions used 6G, zero swap.
No concurrency/debug flags, overlapping builds or baseline updates were introduced.

```sh
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 900 cargo test -p boardstudio_core --test integration boundary_parity::native_transport_reproduces_the_golden_transcript --no-run
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 -- timeout 900 env BOUNDARY_BLESS=1 cargo test -p boardstudio_core --test integration boundary_parity::native_transport_reproduces_the_golden_transcript -- --exact --test-threads=2
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0 -- timeout 900 cargo test -p boardstudio_core --test integration boundary_parity::native_transport_reproduces_the_golden_transcript -- --exact --test-threads=2
```

Final compile: `boundary-green-compile-final.log`, exit 0. Supported blessing:
`boundary-bless-green.log`, **1 passed**, 756 filtered out, 1.63s. Unblessed replay
(`BOUNDARY_BLESS` verified unset): `boundary-replay-green.log`, **1 passed**,
756 filtered out, 1.63s. All logs are under `.evidence/ergogen-17e/`.

Production source, native 2,149-name and WASM baselines remain untouched. No tests
were deleted/skipped, tolerances weakened, new test names added, or production/API
changes made. No subagents, stash, other worktrees, pushes, amends or broad staging.
Examples/starters 26 and release/E2E 27 remain excluded. Quick/full, WASM runtime,
browser gates and independent review remain **pending**; stop here before resuming
the original complete all-caller gate plan. Shared assertions are active in the
WASM path by source composition, not a fresh WASM execution receipt.

Documentation links: `boundary-doc-links.log` reports
`Documentation links check passed: 554 Markdown files.` (shared lock, 6G/no-swap
scope, `timeout 900 python3 scripts/check-doc-links.py`). `git diff --check`
passed with no output (`boundary-diff-check.log`). The full three-file diff was
read before the narrow commit; no unrelated semantic fixture changes were accepted.

### Focused independent review and acceptance gate attempt — 2026-10-23

The focused boundary correction independently passed Standards + Spec review.
The zero-turn column-0 conversion succeeds and transfers only Y Stagger
(`0 → 3`, next column `2 → -1`); the separately splayed column-2 conversion is
refused. Intended chain/refusal assertions run before golden comparison/blessing.
The fixture does not prove populated key-pose preservation; the existing
`matrix_placement.rs` pose/history cases remain part of native verification.
This focused pass is not the final ticket review; 17e and parent 17 remain claimed.

Reverified branch `docs/ergogen-layout-comparison`, tested HEAD
`c9a0699c3189ca899d81ad97ddd4c59bbada6455`, and only the preexisting untracked
`web/crates/layout/prototype/`. The committed fixed-base inventory exactly matches
`git diff --name-only a58a2bf187bd6d08b652dc739cd7c391709ff5c8
c9a0699c3189ca899d81ad97ddd4c59bbada6455`: **114/114 existing paths**, zero
deleted/missing paths. Those 114 paths (including Core, web, tests, fixtures,
baseline and docs) were passed to `quick --files`; the ignored prototype was not.
`BOUNDARY_BLESS` was unset; no golden/baseline updater flags were supplied.

Fresh quick command, serialized with the shared build lock, 22 GiB/no-swap scope,
3600-second timeout, `TMPDIR=/home/chris/.cache/tmp` and `RUST_TEST_THREADS=2`:

```sh
base=a58a2bf187bd6d08b652dc739cd7c391709ff5c8
mapfile -t paths < <(git diff --name-only "$base" c9a0699c3189ca899d81ad97ddd4c59bbada6455)
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
unset BOUNDARY_BLESS
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/check.py quick --files "${paths[@]}" --receipt .evidence/ergogen-17e/quick-receipt.json
```

Quick reached repo, tooling, lint, WASM page typecheck, native tests,
`layout-test-css`, and browser. Repo/docs, lint, typecheck, native tests,
layout-test-css and every scheduled crate browser launch passed. The log reports
all **2,149 baseline test names executed**; the receipt records 1,993 native
passed, 9 existing ignored, 0 failed/incomplete, 0 missing baseline names (1,993
matched). Fourteen WASM test launches passed, **589 passed, 0 failed** across
host, runtime, ui-model, ui-shared (two isolated groups), keycaps, library,
keymap, catalogue, case, parts, pcb and layout (including setup guide).

Quick is **not accepted**: its final line is
`check: step 'browser' failed`. The page-suite command rejected the supplied JSON
fixture as `run-wasm-tests: not a web/src Rust file:
web/src/presentation/layout_layers_fixtures/paired-rejoin-preview.json`; the
page suite is incomplete (0/0 observed), and provider/product browser smoke was
not run. First failure log: `.evidence/ergogen-17e/quick.log`; receipt and summary:
`.evidence/ergogen-17e/quick-receipt.json` and `.evidence/ergogen-17e/quick-receipt.json.md`.
Per the stop rule, full was **not run**; no full receipt/log exists. This is a
scheduler scope failure, not a source/fixture failure. No source, fixture,
baseline or configuration change was made. Final independent ticket reviews remain
pending; no closure or shipping approval is implied.

### Narrow quick-selector repair — 2026-10-23

Parent-authorized tooling scope, starting at
`7e64c131841114b6ee38d9f69875d9d37a9816c8` on
`docs/ergogen-layout-comparison`; only the untouched preexisting untracked
`web/crates/layout/prototype/` was present. Inspected all `Selection`,
`page_files` and `full_rust` callers: the selector, quick planner/output and
their tests. The runner accepts only `web/src/**/*.rs` for `--files`, but the
selector supplied every `web/src/` and `web/tests/` path. The actual failed
command at `quick.log:3406` includes the paired-rejoin JSON fixture.

The parent's minimal shared-selector assertion was reproduced locally: it
failed with that JSON path in `page_files`. This deterministic caller/contract
mismatch already supplies the minimized reproduction, so no bisection,
speculative hypotheses, instrumentation or expensive gate rerun was needed.
The agreed regression seam is real `select` → `quick_plan`, checking both
runner arguments and preservation of consuming-test coverage.

Before changing the selector, added one regression test covering fixture-only,
valid Rust + fixture in both orders, and the same three cases for
`web/tests/page.rs`. `python3 -B scripts/test-check-quick.py` ran **16 tests,
6 expected subtest failures**, each finding the unsupported path in `page_files`
(`.evidence/ergogen-17e/scheduler-red.log`).

The smallest fix keeps valid `web/src/*.rs` file filtering and reuses
`full_rust` for other affected page paths, with a visible note naming the path
and `not a web/src Rust file; every Rust check applies`. This selects every
package and the existing full native/browser plan, including page `--all`,
even when no valid Rust file was supplied. It does not silently drop fixture
coverage or guess an owning module. Quick still excludes the packaged drag
smoke; no runner, production source, fixture, baseline or configuration changed.

Executed with `TMPDIR=/home/chris/.cache/tmp` (fast Python tests; fake/mocked
build runners, no Cargo/WASM builds):

- `python3 -B scripts/test-check-quick.py`: **16 passed**, including all six
  regression cases and unchanged valid-Rust `--files` planning
  (`scheduler-green.log`).
- `python3 -B scripts/test-check.py`: **31 passed** (`scheduler-check.log`).
- `python3 -B scripts/test-run-wasm-tests.py`: **41 passed**
  (`scheduler-run-wasm.log`).
- Original shared-selector assertion plus `full_rust` assertion: passed,
  printing the fixture-specific fallback note (`scheduler-repro-green.log`).
- Documentation links: **554 Markdown files passed** (`scheduler-doc-links.log`);
  `git diff --check`: passed (`scheduler-diff-check.log`). Full diff read.

All log names above are under `.evidence/ergogen-17e/`. Historical failed quick
logs/receipts are preserved. Changed files are `scripts/check_quick.py`,
`scripts/test-check-quick.py` and this Progress entry. Independent focused review
and fresh all-caller quick/full gates remain pending; **17e and parent 17 remain
claimed**. These planning/test receipts do not establish runtime acceptance.

### Selector review and fresh acceptance attempt — 2026-10-23

The parent completed the focused independent selector diff review: the planner
contract matches the runner, all `page_files`/`full_rust` consumers were checked,
and the parent's independent rerun of all **88 selector, scheduler and runner
tests passed**. This is the focused selector review only; the final Standards+Spec
reviews for 17e and parent 17 remain pending.

Before gates, verified branch `docs/ergogen-layout-comparison`, tested source
HEAD `84cc5a1a158c30552c6fd571279df31170b830a8`, clean tracked status, and only
the untouched preexisting untracked `web/crates/layout/prototype/`. With fixed
base `a58a2bf187bd6d08b652dc739cd7c391709ff5c8`, the prior 114-path inventory
plus `scripts/check_quick.py` and `scripts/test-check-quick.py` exactly matches
the **116/116 existing paths** passed to quick; no paths were deleted or
excluded, including JSON, docs, tests, Core and web. `BOUNDARY_BLESS` was unset;
no baseline-update or skip flags were supplied. The historical `quick.log` and
receipt from the 114-path invocation remain untouched.

Fresh quick command, under the shared lock, 22 GiB/no-swap scope and 3600-second
timeout with the required environment:

```sh
base=a58a2bf187bd6d08b652dc739cd7c391709ff5c8
mapfile -d '' -t paths < <(git diff --name-only -z "$base" 84cc5a1a158c30552c6fd571279df31170b830a8)
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
unset BOUNDARY_BLESS
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/check.py quick --files "${paths[@]}" --receipt .evidence/ergogen-17e/selector-fixed-quick-receipt.json > .evidence/ergogen-17e/selector-fixed-quick.log 2>&1
```

The selector fallback appeared in the log for
`web/src/presentation/layout_layers_fixtures/paired-rejoin-preview.json`.
Repo (6 commands), tooling (22), lint (3), page typecheck, native tests (2
commands) and `layout-test-css` passed. The native guard reports all **2,149
baselined names executed**; receipt counts are 2,149 passed, 12 ignored, zero
failed/incomplete, with all baseline names matched. WASM-target Clippy, the web
page `cargo check`, and the layout CSS page build passed. `check-wasm-tests.py`
passed its guard. However, `cad/scripts/test-cad-provider.py` failed immediately
because the CAD WEB package is missing (`RuntimeError: CAD WEB package is
missing; run python3 cad/scripts/build-cad-wasm.py --no-opt`). The quick final
line is **`check: step 'browser' failed`**. Of 54 receipt commands, 37 ran and
17 were not run. The page-suite `scripts/run-wasm-tests.py --all` invocation,
per-crate WASM browser launches, CAD browser suite and provider/product browser
smoke were not reached; the printed fallback is selector planning evidence, not
page runtime coverage.

Quick receipt: `.evidence/ergogen-17e/selector-fixed-quick-receipt.json`; log:
`.evidence/ergogen-17e/selector-fixed-quick.log`. **Full was not run** because
quick failed; there is no full receipt/log or full-gate result. No source,
fixture, tooling or baseline correction was made. The source tested remains
`84cc5a1a158c30552c6fd571279df31170b830a8`; this progress update is a separate
documentation-only commit. 17e and parent 17 remain claimed pending the
unreached acceptance stages and final Standards+Spec reviews; no shipping or
user-data writer approval is implied.

### CAD package preparation and final quick attempt — 2026-10-23

Verified branch `docs/ergogen-layout-comparison`, source HEAD
`f4a657d3ef13cd941f042ada35d0d2350477c7eb`, fixed base
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`, `BOUNDARY_BLESS` unset, and the
exact 116 existing tracked paths from the selector-fixed inventory (the prior
114-path set plus `scripts/check_quick.py` and `scripts/test-check-quick.py`).
The untracked `web/crates/layout/prototype/` was not passed. No paths were deleted.

Prepared the missing ignored CAD WEB package using the supported command under
the shared build lock, 22 GiB/no-swap scope, 3600-second timeout, and required
environment:

```sh
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 cad/scripts/build-cad-wasm.py --no-opt
```

Build passed (`Finished release profile`; wasm-pack `Done` and package ready).
Generated JS and WASM files are present and ignored. The quick gate's existing
`cad/scripts/test-cad-provider.py` current-build-stamp/package check ran and
passed. Build log: `.evidence/ergogen-17e/cad-package-build.log`.

Fresh all-caller quick ran against the exact HEAD and all 116 paths, with the
same environment and resource guard:

```sh
base=a58a2bf187bd6d08b652dc739cd7c391709ff5c8
head=f4a657d3ef13cd941f042ada35d0d2350477c7eb
mapfile -d '' -t paths < <(git diff --name-only -z "$base" "$head")
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
unset BOUNDARY_BLESS
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/check.py quick --files "${paths[@]}" --receipt .evidence/ergogen-17e/cad-ready-quick-receipt.json
```

The selector fallback was visible for
`web/src/presentation/layout_layers_fixtures/paired-rejoin-preview.json`:
`not a web/src Rust file; every Rust check applies`. Repo, tooling, lint, page
WASM typecheck, native tests and `layout-test-css` passed. All **2,149 native
baseline names** ran (2,149 passed, 12 ignored, 0 failed/incomplete; no missing
names). WASM-target Clippy, page `cargo check`, page/layout CSS build and the
WASM-test guard passed. These are compile/guard checks, not WASM browser runtime
coverage.

Quick is **not accepted**: receipt `.evidence/ergogen-17e/cad-ready-quick-receipt.json`
records 38/54 commands run, exit 1, with 16 commands not run. The CAD provider
suite passed. The CAD browser suite reached `worker.onerror` in
`cad/scripts/browser-smoke/qualification.mjs:22:63` after its preceding checks;
final line: **`check: step 'browser' failed`**. The 14 per-crate WASM browser
launches and `scripts/run-wasm-tests.py --all` page suite were not run; product
browser smoke was not reached. The full gate was not run because quick failed.
Quick log: `.evidence/ergogen-17e/cad-ready-quick.log`. Earlier
`quick.log`/receipt and `selector-fixed-quick.*` remain historical evidence.

No source, fixture, baseline, or configuration change occurred; tracked files
remain clean apart from this progress update, and the only untracked path remains
the prototype. 17e and parent 17 remain claimed. Final independent Standards+Spec
reviews are pending; this gate attempt grants no closure or shipping approval.
