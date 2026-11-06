# 17e: Fixtures and final gate

Status: resolved
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

### Supported provider preparation and final quick gate — 2026-10-23

Verified branch `docs/ergogen-layout-comparison` at source HEAD
`8323a6293c4d92da02f3b5f66f3a80bf879c9151`; fixed base remains
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`. `BOUNDARY_BLESS` was unset. Tracked
files were clean before generated preparation; the only unrelated untracked path,
`web/crates/layout/prototype/`, was not touched or passed to quick. The exact
116-path base-to-HEAD inventory was retained (the 114 original paths plus
`scripts/check_quick.py` and `scripts/test-check-quick.py`).

Before preparation, the focused browser run reproduced the existing worker failure:
all direct module/geometry checks passed, then `worker.onerror` surfaced at
`cad/scripts/browser-smoke/qualification.mjs:22:63`. The required worker JS and
WASM were absent at `web/target/providers/cad-worker/`, while the CAD package itself
was present. No source defect was demonstrated.

The documented provider-only preparation passed:

```sh
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/build-web.py --providers-only --fast
```

Generated `m1_cad_worker.js` and `m1_cad_worker_bg.wasm` now exist under
`web/target/providers/cad-worker/` and are ignored by Git. The focused browser run
then passed **364 browser checks**, including actual worker operations and restart
behavior. Logs: `worker-missing-browser.log`, `worker-providers-build.log`, and
`worker-ready-browser.log` under `.evidence/ergogen-17e/`.

Fresh all-caller quick used the exact 116 tracked paths at source HEAD above, with
the shared lock, 22 GiB/no-swap scope, 3600-second timeout, and no selector,
baseline, or skip overrides. Receipt: `worker-ready-quick-receipt.json`; log:
`worker-ready-quick.log`, both under `.evidence/ergogen-17e/`. Repo/tooling checks,
lint, page WASM typecheck/build, WASM guard, and native tests passed. The native
baseline matched **2,149/2,149 names**; tests reported 2,149 passed, 12 ignored,
zero failed/incomplete. CAD provider checks passed, CAD browser checks passed, all
14 per-crate WASM browser launches passed, and the full-page WASM test inventory
was listed. The paired-rejoin fixture visibly selected the Rust fallback
(`not a web/src Rust file; every Rust check applies`).

Quick is **not accepted**: 53 of 54 scheduled commands passed; the only failure was
`python3 scripts/run-wasm-tests.py --all --depth 1 --isolate=presentation::case_workspace::`.
The page batch executed 77 tests and failed 21. Its first reported failure was
`presentation::layout_component_inspector_tests::mounted_rapid_xy_queued_edits_keep_both_coordinates_and_undo_removes_only_y`
at `web/src/presentation/layout_component_inspector_tests.rs:2050:28` (expected
`X mm` input to render); 20 later failures were reported as panic-hook fallout.
The separate canvas wait also timed out at 5 seconds. These runtime failures are
unresolved; no source or test fix was made. The product drag smoke and full gate
were not run; full remains blocked by quick. Earlier failed receipts remain intact.

No baseline, fixture, source, configuration, or other ticket status changed. 17e
and parent 17 remain claimed pending final independent Standards+Spec reviews;
provider preparation and focused success alone are not acceptance.

### First page failure diagnosis — 2026-10-23 (partial; no fix)

Source pin: `f3e7818a8a1838d3d318b3ac2ef8bc371d0bc70a`, branch
`docs/ergogen-layout-comparison`. No production/test/fixture correction was applied.
The unrelated untracked `web/crates/layout/prototype/` remains untouched.

Two fresh exact-test diagnostic launches used the existing `run_filter` helper in
`scripts/run-wasm-tests.py`, with `runner_environment()` and
`desktop_webdriver_config(...)`, selecting
`presentation::layout_component_inspector_tests::mounted_rapid_xy_queued_edits_keep_both_coordinates_and_undo_removes_only_y`.
Both returned **0 passed, 1 failed, 76 filtered**, with the original `X mm input is
rendered` assertion at `layout_component_inspector_tests.rs:2050:28`.
Logs: `.evidence/ergogen-17e/page-diagnosis-exact-red.log` and
`.evidence/ergogen-17e/page-diagnosis-state.log`. These are exact-filter partial
runs, not page acceptance. Each used the shared outer flock, 22 GiB/no-swap
systemd scope and 900-second timeout; `TMPDIR=/home/chris/.cache/tmp` and
`RUST_TEST_THREADS=2` were exported, with normal runner/Cargo flags.

The supported file-level command was:

```sh
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs --result-json .evidence/ergogen-17e/page-diagnosis-module.json > .evidence/ergogen-17e/page-diagnosis-module.log 2>&1
```

It completed **25 tests: 11 passed, 14 failed, zero incomplete**. The same first
panic was followed by 13 panic-hook failures; no additional independent origin
was established. The original quick's 20 later failures remain fallout, not 20
independently diagnosed bugs. Full raw module output is preserved at
`.evidence/ergogen-17e/page-diagnosis-module.json.logs/filter-001.log`.

Temporary `[DEBUG-17e-page]` probes captured the actual Core reply and missing-DOM
state. Core returned `Error`: `Linked Layout requires an explicit correspondence
for every source matrix`. The settled host had `accepted=false`, lifecycle
`Empty`, no scope, empty selected IDs, no selection anchor/context, and no
Inspector projection. Its DOM contained fixture buttons, an Inspector placeholder
and render-generation output `1`, with no position inputs. Thus mount delay,
Inspector conditional rendering and prior-test state are not required to explain
this failure: the fixture's Open was refused before selection could exist.

High-confidence cause: incomplete grouped-layout fixture cutover.
`fixture_document` supplies an empty `LayoutMirrorLink.correspondences`; current
`matrix::layout::validate_link` requires a Matrix correspondence for each source
matrix. Compared with fixed pre-cutover `a58a2bf187bd6d08b652dc739cd7c391709ff5c8`,
the fixture gained plural matrix IDs plus the empty list, while the validator gained
this explicit-correspondence requirement. The old base was inspected with Git;
it was not executed, so no historical runtime pass is claimed.

Proposed next action, requiring the parent's decision: correct the shared fixture
in `web/src/presentation/layout_component_inspector_tests.rs` with the explicit
Matrix pair `matrix` -> `paired-matrix` (no companion override/tweak), preserving
the intended linked-layout semantics. Add an Open-success assertion after the real
Session/Core open in `mounted_runtime_probe`, before selection/mount, so invalid
fixtures fail at admission. Preserve the existing mounted X/Y queue and Undo test
as the regression seam; rerun exact then file-level checks. No Core validation or
production Inspector change is supported by this evidence; the proposed correction
has not been applied or verified and could expose further failures.

All temporary probes were removed; the two instrumented source files returned to
HEAD with no diff. No quick/full or product drag gate was run in this diagnosis.
Failed receipts remain intact. 17e and parent 17 remain claimed; final acceptance,
reviews and later feature work remain pending.

### Positive Inspector fixture correction — 2026-10-23 (partial checkpoint)

Started at authorized HEAD `8baee0c416f20aa6f56d196220b57395c0a0f993` on
`docs/ergogen-layout-comparison`, with clean tracked files and only the preexisting
untracked `web/crates/layout/prototype/`. Inspected every fixture constructor and
caller in `web/src/presentation/layout_component_inspector_tests.rs`. Its only two
`LayoutMirrorLink` constructors lacked the Matrix correspondence required by
[spec §2](../spec.md#2-persisted-shape-and-validation) and [17b](17b-validation-and-ownership.md).
Added typed `MirroredCorrespondence` records with `kind: Matrix`, no companion
override/tweak, and these source/mirror identities:

- Shared `fixture_document`: `matrix` → `paired-matrix`.
- `mounted_matrix_key_stays_key_until_explicit_finding_opens_component_inspector`:
  `component-matrix` → `component-paired-matrix`.

Synthetic ReadModel fixtures retain their projection setup. `runtime_document`
still supplies the two base matrices. `configure_mirrored_part_pair` still clears
layouts/matrices and uses its physical Part pair without extra metadata.

Added an accepted-snapshot assertion in `mounted_runtime_probe`, immediately after
the real Session/Core Open effects settle via `run_pending().await`, before
SelectParts or DOM mounting. Each probe starts with a fresh empty runtime, so no
previously accepted snapshot can satisfy admission. No raw Core-only replacement,
new production helper, diagnostic probe or runner option was introduced.

**RED → focused GREEN.** Before changing either mapping, the exact existing
`mounted_rapid_xy_queued_edits_keep_both_coordinates_and_undo_removes_only_y` test
failed at the new admission assertion: `the mounted Inspector fixture must be
accepted by Session/Core Open before selection` (**0 passed, 1 failed, 76 filtered**).
After both mappings, a fresh exact launch passed (**1 passed, 0 failed, 76 filtered**),
including both coordinates and Undo removing only Y. Both launches reused
`run_filter`, `runner_environment()` and `desktop_webdriver_config(...)` from
`scripts/run-wasm-tests.py`, as in the diagnosis. Logs/results:
`.evidence/ergogen-17e/inspector-fixture-red.{log,json}` and
`.evidence/ergogen-17e/inspector-fixture-exact.{log,json}`. The initial rustfmt check
identified only formatting of the new import/assertion; applied rustfmt before
the file run.

**STOP: new first independent failure; no semantic expansion.** Executed:

```sh
export TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2
unset BOUNDARY_BLESS
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 3600 python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs --result-json .evidence/ergogen-17e/inspector-fixture-file.json
```

File selection included all **25** Inspector and nested Matrix Relations scenarios:
**12 passed, 13 failed, 0 incomplete**, 52 filtered. The queued X/Y regression
passed again. First independent panic:
`mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_the_reset`,
`layout_component_inspector_tests.rs:2575`, **`converted X draft 1`**. The unchanged
assertion expects converted X approximately `2.866025403784439`; no cause or fix
for this separate conversion failure is claimed. The runner reports **12 subsequent
panic-hook cascade failures**, not 12 additional independent bugs. In particular,
the matrix-key fixture's mounted scenario occurs after the first panic, so its
mapping has no clean runtime success receipt in this batch. All terminal scenario
names/outcomes are retained in `inspector-fixture-file.json`; raw output is
`inspector-fixture-file.json.logs/filter-001.log`, with scheduler output in
`inspector-fixture-file.log`, all under `.evidence/ergogen-17e/`.

Exact runs used the same outer lock, 22 GiB/no-swap scope and environment, with
`timeout 900`; file used `timeout 3600`. No heavy commands overlapped. Earlier
failure logs remain intact. Page `--all --depth 1 --isolate=presentation::case_workspace::`
was **not run**, because the file prerequisite failed; no page result is claimed.
All 116-caller quick/full and product drag checks remain outside this round.

This is a focused-GREEN, file-blocked fixture checkpoint for parent review, not
17e/17 closure or full acceptance. No production code, validation, baselines,
allowlists, timeouts, skips, UI labels, existing assertions or test names changed;
no fixture blessing or later feature work. Parent 17 and 17e remain **claimed**.
Verification: final diff read; `rustfmt --edition 2024 --check` on the test file
and `git diff --check` passed. `scripts/check-doc-links.py` passed for 554 Markdown
files under the same resource guard with `timeout 900`; log:
`.evidence/ergogen-17e/inspector-fixture-doc-links.log`. No temporary diagnostic
probes remain. Only the authorized test file and this Progress entry are changed.

### Part-offset axes fixture diagnosis and correction — 2026-10-23

Started at `b065ec0dea865bf9022164e218698c532fd4ec3b` on
`docs/ergogen-layout-comparison`, with clean tracked state and only the untouched
preexisting `web/crates/layout/prototype/`. Scope is this Progress entry and
`web/src/presentation/layout_component_inspector_tests.rs`; prior explicit Matrix
correspondences and Session/Core admission assertions are retained. 17e and parent
17 remain **claimed**.

**Runtime RED and cause.** A fresh exact launch of
`mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_the_reset`
reproduced `converted X draft 1`: **0 passed, 1 failed, 76 filtered**, 0.20s.
Temporary `[DEBUG-17e-axes]` probes showed immediately after real Session/Core Open:
accepted source key heading **0 degrees**, authored cell rotation **None**, and
accepted `PartOffset { offset: (1, -4), rotation: 5 }`. After the real board-axes
edit, accepted revision **1** contained `Offset { offset: (1, -4), rotation: 5 }`;
the form showed `("1", "board", "matrix/component-matrix/r0c0")`. Thus H1 is
confirmed: Open refresh replaced the materialized-only 30-degree pose. H2 (edit
not submitted/rejected) and H3 (accepted conversion with stale form) do not explain
this runtime state. Evidence: `axes-fixture-runtime-red.{log,json}` under
`.evidence/ergogen-17e/`.

Removed all diagnostic probes, then added the permanent accepted source-heading
assertion before conversion. A separate fresh exact launch failed at that new seam:
**left 0.0, right 30.0; 0 passed, 1 failed, 76 filtered**, 0.17s
(`axes-fixture-heading-red.{log,json}`). Only after observing that RED, corrected
this constructor's `MatrixCell.rotation` from `None` to `Some(30.0)`.
[Spec §3](../spec.md#3-one-core-geometry-module) defines the final key pose with
`turn(keyRotation[r,c])`; ordinary `refresh_existing` materializes that kernel pose
unless Local/override applies. No Local/override shortcut is used. The fixed base
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8` already had saved pose 30 but cell None;
that source comparison is not a historical runtime receipt. The fixture no longer
relies on a materialized pose surviving the kernel cutover.

This is PartOffset part-to-board axes conversion, not column offset-to-Stagger or
the zero-Splay conversion rule. Its existing board-X assertion stays
`2.866025403784439` (`cos(30°) * 1 - sin(30°) * -4`), with the same tolerance.
The intended source heading is 30 degrees and the original part-axes Y is -4.
Existing source/axes choices, selection switches, conversion, Undo, reset, and
genuine key-versus-Component mode assertions are unchanged. No production,
validation, baseline, allowlist, timeout, label, skip or blessing change occurred.

All exact launches reuse `run_filter`, `runner_environment()` and
`desktop_webdriver_config(...)` from `scripts/run-wasm-tests.py`, each in a fresh
browser runtime. Every heavy launch takes outer
`flock /home/chris/.cache/tmp/boardstudio-build.lock`, then
`systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- timeout 900`
(exact) or `timeout 3600` (file/page), with
`TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2` and `BOUNDARY_BLESS` unset.
Normal build jobs/flags and existing runner timeouts remain in force; no heavy
builds overlap.

**Focused and file GREEN.** Fresh exact conversion: **1 passed, 0 failed,
76 filtered**, 0.23s, including the accepted 30-degree heading and unchanged
conversion/Undo/reset assertions (`axes-fixture-conversion-green.{log,json}`).
Separate fresh exact
`mounted_matrix_key_stays_key_until_explicit_finding_opens_component_inspector`:
**1 passed, 0 failed, 76 filtered**, 0.15s, proving the second explicit Matrix
mapping and key/Component behavior without preceding panic fallout
(`axes-fixture-matrix-key-green.{log,json}`).

Supported file command:

```sh
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs --result-json .evidence/ergogen-17e/axes-fixture-file.json
```

Result: **25 passed, 0 failed, 0 incomplete**, 52 filtered, including nested Matrix
Relations. Receipt `axes-fixture-file.json`, scheduler log `axes-fixture-file.log`,
raw runtime `axes-fixture-file.json.logs/filter-001.log`. The previous conversion
panic and its 12 subsequent panic-hook failures do not recur in this fresh batch.

**Page GREEN.** Executed the requested supported page command under the same guard:

```sh
python3 scripts/run-wasm-tests.py --all --depth 1 --isolate=presentation::case_workspace:: --result-json .evidence/ergogen-17e/axes-fixture-page.json
```

Result: **77 passed, 0 failed, 0 incomplete** across
`model_decode_boundary_tests::`, `presentation::` and isolated
`presentation::case_workspace::`; receipt is complete with no problems or known
failure exclusions. Receipt `axes-fixture-page.json`, scheduler log
`axes-fixture-page.log`, raw filter logs in `axes-fixture-page.json.logs/`, all
under `.evidence/ergogen-17e/`. No new independent panic or panic-hook fallout
occurred. This page receipt supersedes neither the failed historical quick receipts
nor the outstanding full/product gates.

Final verification: read the complete two-file diff; `rustfmt --edition 2024
--check web/src/presentation/layout_component_inspector_tests.rs`,
`scripts/check-doc-links.py` (554 Markdown files, same guard, `timeout 900`) and
`git diff --check` passed. Logs: `axes-fixture-rustfmt.log`,
`axes-fixture-doc-links.log`, `axes-fixture-diff-check.log`. No diagnostic probes
remain. Only this Progress entry and the authorized fixture file changed; the
unrelated prototype is untouched. No subagents, stash, push, amend, other worktrees
or broad staging. All-116-caller quick, full and product drag gates were **not run**
in this round. Stop for parent review; no ticket/map closure or full acceptance.

### Fresh acceptance gates after Inspector fixture corrections — 2026-10-23

The parent reviewed both exact Inspector fixture corrections (explicit Matrix
admission metadata and the intended heading in `MatrixCell`) and the runtime
diagnosis. These changes are test-fixture metadata only; the saved-part pose
conversion values, tolerance, Undo and reset expectations remain unchanged. The
latest correction retains its accepted-heading assertion. No production or
assertion changes were made during this gate round.

Verified source pin: branch `docs/ergogen-layout-comparison`, tested source HEAD
`b35e44c5efdbd60b6c01e400a15a543bac5d0ec5`; fixed coordinated base
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`. The Git diff from that base to the
tested HEAD contains all **116** paths enumerated above, including JSON, Core/web,
tests, native baseline, docs and the two quick-scheduler files. Every path was
passed to the fresh quick command. `BOUNDARY_BLESS` was unset. Tracked state was
clean before the gates; preexisting untracked `web/crates/layout/prototype/` remains
untouched.

Fresh quick command (all 116 paths supplied from the pinned base-to-HEAD diff):

```sh
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 timeout 3600 python3 scripts/check.py quick --files $(git diff --name-only a58a2bf187bd6d08b652dc739cd7c391709ff5c8..b35e44c5efdbd60b6c01e400a15a543bac5d0ec5) --receipt .evidence/ergogen-17e/final-quick-receipt.json > .evidence/ergogen-17e/final-quick.log 2>&1
```

Quick passed: `check: all steps passed: repo, tooling, lint, typecheck, test,
layout-test-css, browser`. Receipt accepted, 54/54 scheduled commands. The native
baseline matched all 2,149/2,149 names with no missing or new names; workspace tests
were 2,052 passed/9 ignored and footprints 97 passed/3 ignored. Quick's page suite
ran 77/77 successfully. This is scoped quick evidence, not full acceptance.

The first full preflight stopped before running checks because `playwright-core`
was not on `NODE_PATH`. The pinned package existed at
`/home/chris/.cache/boardstudio-web-tools/node/node_modules`; the supported full
command was rerun with that required path:

```sh
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules timeout 3600 python3 scripts/check.py --receipt .evidence/ergogen-17e/final-full-receipt.json > .evidence/ergogen-17e/final-full.log 2>&1
```

Full stopped at its first actual failure. Final line: `check: step 'browser' failed`.
Receipt: 36/55 commands ran; repo, tooling, lint, build and test passed. Native
baseline again matched 2,149/2,149 names; workspace tests were 2,052 passed and
footprints 97 passed. Packaged-page drag smoke passed part drag and column body
drag with exact Undo, then failed at the stagger handle:
`AssertionError [ERR_ASSERTION]: Stagger handle: changed stagger` in
`scripts/test-drag-interactions.cjs:254` (`actual: 0`, `expected: 0`). No repair or
narrow rerun was made. Full therefore did not reach its WASM crate compile/runtime
checks, CAD provider or CAD browser checks, or its full 77-test page suite. Quick's
page run does not cover those full gates.

Receipts/logs: `.evidence/ergogen-17e/final-quick-receipt.json`,
`.evidence/ergogen-17e/final-quick.log`,
`.evidence/ergogen-17e/final-full-receipt.json`,
`.evidence/ergogen-17e/final-full.log`. Final documentation checks also passed:
`.evidence/ergogen-17e/final-progress-doc-links.log` reports 554 Markdown files;
`.evidence/ergogen-17e/final-progress-diff-check.log` is empty with exit 0. Earlier
failure logs and receipts remain intact. Acceptance remains incomplete. 17e and
parent 17 remain **claimed** pending the full gate and final independent
Standards+Spec reviews; this records no closure, shipping or user-data approval.


### Conditional drag test-reader migration — 2026-10-23

Verified branch `docs/ergogen-layout-comparison` at
`a8c8e0033dea69181ff129dc72a77e9d9688ede4`, with clean tracked state and only
preexisting untracked `web/crates/layout/prototype/`, untouched. Explicit conditional
authorization covers this Progress and `scripts/test-drag-interactions.cjs` only.
Read AGENTS, coding standards, README drag guidance, spec §§2/6/7, relevant glossary
and ADRs 0007/0034; used diagnosing-bugs/TDD at the existing real-page smoke seam.
No production fix or fixture blessing is authorized or performed. The checked site
from the full build is reused directly by the supported runner; production sources
are unchanged since `b35e44c5`. No rebuild, copied site or forged build receipt.

**Predictions before the probe:** H1: grouped Stagger persists but legacy reads yield
zero; H2: grouped Stagger never changes/cancels; H3: poll and final reader target
different matrices/revisions. Temporary `[DEBUG-17e-drag]` probes in the test script
captured selected identity, sparse grouped record, legacy fields, revision, predicate
and selected scene before/poll/after. The existing full RED remains at
`.evidence/ergogen-17e/final-full.log:3801` and
`web/target/test-results/drag-browser/20261023T214730Z-292cba25aa73/run.log`.

**Fresh actual RED / H1 confirmed:** `.evidence/ergogen-17e/drag-schema-red.log`;
standard run log:
`web/target/test-results/drag-browser/20261023T215103Z-d98c381d1da9/run.log`.
The selected identity was `matrix-1`, row 4, column 2 throughout. Before Stagger:
revision 6, grouped column record absent (sparse zero), both legacy fields absent,
reader Stagger 0. Poll and after: revision 7, `columnLayout[2] = {stagger: -9.525}`,
both legacy fields still absent, reader Stagger still 0. Selected left/right scene
transforms changed from `translate(±67.125,76.2) rotate(0)` to
`translate(±67.125,66.675) rotate(0)`. The trusted gesture/live preview and durable
full-document change passed, followed by the original `changed stagger` assertion
failing with actual 0 / expected 0. Part and Column body drag and their exact Undo
passed first; their grouped column record stayed absent. This is positive persisted
grouped-state evidence, excluding H2 and the observed H3 identity/timing alternative.

**Poll mismatch:** the probe logged `fields: ["stagger"]`, expected/current legacy
Stagger 0 and `changed: false`, yet execution reached the after-read. Local installed
`playwright-core` 1.62.1 `lib/coreBundle.js:23886` calls `predicate()` synchronously,
checks the returned Promise for truthiness and fulfills with that Promise; subsequent
awaiting unwraps its false result without repeating the predicate. This explains the
apparent contradiction; it is not evidence of a different matrix or a successful
changed-field poll. Read-only implementation excerpt:
`.evidence/ergogen-17e/drag-schema-poll-implementation.log`. Wait implementation is
unchanged under this authorization. The durable field-change and exact-Undo
assertions still check actual state, but async polling has this existing limitation;
a separate authorized wait/tooling correction can address it.

**Minimal migration:** inventoried all four legacy reads (Node `matrixState` and
page-context `runDrag` poll), `vectorAt`'s sole caller, `partRead`/`columnRead` and
all four `runDrag` callers. Both readers now use `matrix.columnLayout[column]`'s
optional `offset`/`stagger`, with existing zero vector/scalar defaults required by
spec §2 for missing records/fields. No legacy fallback or alias. Removed only the
now-unused `vectorAt`. No serializers or production callers changed. Original
expected/allowed fields, gestures, waits, tolerances, cancellation, trusted input,
pointer capture/live preview, save-cycle/revision/full-document signature, unchanged
fields, exact document/scene Undo, qualified collision checks and count guards remain.
The existing real smoke supplies RED/GREEN regression coverage for these field reads;
no new nontrivial logic or test harness was added. All temporary probes were removed
before the focused post-migration run.

Both focused runs use the same command, redirected respectively to
`.evidence/ergogen-17e/drag-schema-red.log` and `drag-schema-green.log`:

```sh
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env -u BOUNDARY_BLESS TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules timeout 900 python3 scripts/run-drag-browser.py
```

**Focused GREEN:** supported runner exited 0, validated its receipt and reported
`PASS drag browser smoke: 13 checks`. Receipt:
`web/target/test-results/drag-browser/20261023T215124Z-6e35d0250bfc/receipt.json`;
raw browser log is adjacent `run.log`; outer log is
`.evidence/ergogen-17e/drag-schema-green.log`. All original checks completed: Part,
Column body, Stagger handle and Ctrl Offset handle with exact document/scene Undo;
Stagger cancel with no saved revision; Default collision PCB pair; Case viewer with
overflowing collision findings; Qualified collision product fixture; Unavailable
physical PCB stays uncertain. No new independent failure. Default collisions
observed four PCB pairs, with findings scrolling beside the visible viewer.

Final checks: complete two-file diff read; `node --check
scripts/test-drag-interactions.cjs`, documentation links (554 Markdown files) and
`git diff --check` passed. Logs under `.evidence/ergogen-17e/`:
`drag-schema-syntax.log`, `drag-schema-doc-links.log`, `drag-schema-diff-check.log`.
No legacy reads or diagnostic probes remain in the script. Only the two authorized
files are staged for this minimal increment; the unrelated prototype remains
untracked and untouched. No subagents, stash, push, amend, broad staging or other
worktrees. Stop for parent review before acceptance reruns.

**Scope/gates:** the script was absent from the original 116-path inventory, so its
addition expands the next changed-file scope to 117 paths. No all-117 quick or full
run was executed in this round. The prior all-116 quick receipt remains scoped prior
evidence; the failed full receipt remains failed (36/55), and its later WASM/CAD/page
stages remain unreached by that full run. This focused green does not accept the
cutover. 17e and parent 17 remain **claimed**, with final independent Standards+Spec
reviews and the resumed full gate outstanding. No map/closure or shipping approval.

### Fresh post-reader-correction acceptance gates — 2026-10-23

The parent reviewed the focused reader-only drag smoke correction: both readers now
use grouped `columnLayout.offset` / `stagger`; original gestures, waits, tolerance,
cancel and exact Undo assertions remain. No source, test, tooling, fixture or
baseline edits were authorized or made in this gate round. The prior full failure
and all earlier receipts remain preserved.

Tested source pin: branch `docs/ergogen-layout-comparison`, HEAD
`47aeb001dc29c9cf9d9f8ea26806f91e6a8addc9`; fixed coordinated base
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`. The complete base-to-HEAD tracked
diff contains **117 paths**, including JSON, Core/web, tests, native baseline,
docs, `scripts/check_quick.py`, `scripts/test-check-quick.py` and
`scripts/test-drag-interactions.cjs`; all 117 were supplied to quick. No paths
were omitted or deleted. `BOUNDARY_BLESS` was unset. Tracked state was clean
before the gate; only the preexisting untracked `web/crates/layout/prototype/`
remains. Supported builds regenerated their ordinary provider/site outputs; no
hand-copied or forged outputs were used.

Fresh quick command, with all 117 paths from the pinned base-to-HEAD diff:

```sh
mapfile -t changed_paths < <(git diff --name-only a58a2bf187bd6d08b652dc739cd7c391709ff5c8 47aeb001dc29c9cf9d9f8ea26806f91e6a8addc9)
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env -u BOUNDARY_BLESS TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 timeout 3600 python3 scripts/check.py quick --files "${changed_paths[@]}" --receipt .evidence/ergogen-17e/post-drag-quick-receipt.json > .evidence/ergogen-17e/post-drag-quick.log 2>&1
```

Quick passed. Final line: `check: all steps passed: repo, tooling, lint, typecheck,
test, layout-test-css, browser`. Receipt accepted, **54/54** scheduled commands;
native name guard **2,149/2,149** matched with none missing/new (2,149 passed,
12 ignored). All **14 WASM launches** passed; CAD provider and CAD browser checks
passed; the page suite passed **77/77**. This quick receipt is partial by design.

Because quick passed, ran the unfiltered full pre-PR command:

```sh
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env -u BOUNDARY_BLESS TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules timeout 3600 python3 scripts/check.py --receipt .evidence/ergogen-17e/post-drag-full-receipt.json > .evidence/ergogen-17e/post-drag-full.log 2>&1
```

Full passed. Final line: `check: all steps passed: repo, tooling, lint, build, test,
browser`. Receipt complete, **55/55** scheduled commands, no failures or not-run
commands. Native guard again matched **2,149/2,149**; all **14 WASM launches** and
the packaged page suite **77/77** passed. Packaged-page trusted drag smoke passed
**13/13** with receipt
`web/target/test-results/drag-browser/20261023T215933Z-0eb5bebe2115/receipt.json`:
Stagger handle and cancel, Ctrl Offset, exact document/scene Undo, collision PCB
and qualified product fixtures all passed. CAD provider and browser checks passed.
No scope was unreached.

Gate receipts/logs: `.evidence/ergogen-17e/post-drag-quick-receipt.json`,
`.evidence/ergogen-17e/post-drag-quick.log`,
`.evidence/ergogen-17e/post-drag-full-receipt.json`,
`.evidence/ergogen-17e/post-drag-full.log`. Receipt source identity is the tested
HEAD above; the following docs-only progress commit does not change that pin.
17e and parent 17 remain **claimed** pending their final independent
Standards+Spec reviews. This checkpoint makes no closure, release or user-data
writer approval.

### Fresh gates after merged final-review fixes — 2026-10-23

At `6838c68ae`, the independent final reviews recorded two historical requests:
Standards P2 for persisted companion arithmetic regrouping and Spec P2 because
This column read Splay while writing rotation. These findings are not yet
independently rereviewed. The geometry correction `1824e7d9d32160ee0e3632600e552ee9ed4961fa`
was merged as `81854b28b04a0637da5346584f2f4c48ff391b65`: private shared
`companion_pose` materialization/refresh preserves the original grouping and
angular sum. Its exact-bit drift RED and exact refresh plus serialized Core reopen
GREEN evidence is in
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-384cad19/.evidence/ergogen-17-review-fixes/`;
the focused runs covered 806 library and 757 integration tests.

The UI correction `e06cc125c02de1b25320aa5844242413a858b889` was merged as
`bbf1a1a8546ca2520d90f9226ce79689f7def3f8`: This column uses rotation consistently
through display, preview, nudge, drag delta, queued resolution, equality and draft
restoration; a scope change resets only the angle draft. Its five new WASM
regressions and 157+1 layout browser tests plus 80 native tests are recorded in
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-c2a06b48/.evidence/ergogen-17-review-fixes/`.
These child-worktree receipts are prior fix evidence, not substitutes for the
fresh gates below.

The parent reviewed the native baseline candidate updater's exact one addition and
zero removals, then committed
`matrix::projection_tests::following_companion_preserves_materialized_pose_exactly_on_refresh_and_reopen`
at `47ce298020ef72903bab4032d5ee5980b7779215`. The WASM baseline was left
unchanged because it ratchets wasm-only plain `#[test]` debt, not runtime test
names. The five new `#[wasm_bindgen_test]` regressions add no such debt; its guard passed.

Fresh gate pin: branch `docs/ergogen-layout-comparison`, source HEAD
`47ce298020ef72903bab4032d5ee5980b7779215`, fixed base
`a58a2bf187bd6d08b652dc739cd7c391709ff5c8`. The exact base-to-HEAD diff has
119 existing paths: the prior 117 plus
`web/crates/layout/src/objects/layout_canvas_controls.rs` and
`web/crates/layout/src/objects/matrix_transform_inspector.rs`. All 119 exist and
were passed to quick, including JSON, Core, web, tests, fixtures, baseline and
docs. No exclusions or deletions; the baseline was already in scope. Tracked
state was clean. The preexisting untracked
`web/crates/layout/prototype/` remained untouched and was not passed to quick.
`BOUNDARY_BLESS` was unset; no baseline updater, skip or selector override was
used. The receipt's local-dirty marker reflects that untracked prototype.

Fresh quick command (complete output in `.evidence/ergogen-17e/review-fix-quick.log`):

```sh
mapfile -d '' -t paths < <(git diff --name-only -z a58a2bf187bd6d08b652dc739cd7c391709ff5c8 47ce298020ef72903bab4032d5ee5980b7779215)
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env -u BOUNDARY_BLESS TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 timeout 3600 python3 scripts/check.py quick --files "${paths[@]}" --receipt .evidence/ergogen-17e/review-fix-quick-receipt.json > .evidence/ergogen-17e/review-fix-quick.log 2>&1
```

Quick passed; final line: `check: all steps passed: repo, tooling, lint, typecheck,
test, layout-test-css, browser`. Receipt `.evidence/ergogen-17e/review-fix-quick-receipt.json`
is complete and accepted: 54/54 commands, native guard 2,150/2,150 matched
(2,150 passed, 12 ignored), all 14 WASM launches passed, and page tests passed
77/77. The five new WASM tests passed by name:
`column_angle_scope_switches_display_and_discards_the_other_fields_draft`,
`column_angle_equality_uses_the_requested_field_in_the_accepted_snapshot`,
`column_angle_nudges_compose_without_overwriting_splay`,
`column_angle_drag_delta_composes_with_the_latest_rotation`, and
`column_angle_pointer_preview_starts_at_rotation_and_preserves_mirror_direction`.
The layout launch passed 157 tests, its separate setup-guide launch passed 1, and
CAD provider and browser checks passed (364 browser checks).

Only after quick passed, the unfiltered README full check ran with the same lock,
22 GiB/no-swap cap, timeout and test environment, plus the pinned Playwright
`NODE_PATH` required by browser tooling:

```sh
flock /home/chris/.cache/tmp/boardstudio-build.lock systemd-run --user --scope -q -p MemoryMax=22G -p MemorySwapMax=0 -- env -u BOUNDARY_BLESS TMPDIR=/home/chris/.cache/tmp RUST_TEST_THREADS=2 NODE_PATH=/home/chris/.cache/boardstudio-web-tools/node/node_modules timeout 3600 python3 scripts/check.py --receipt .evidence/ergogen-17e/review-fix-full-receipt.json > .evidence/ergogen-17e/review-fix-full.log 2>&1
```

Full passed; final line: `check: all steps passed: repo, tooling, lint, build, test,
browser`. Receipt `.evidence/ergogen-17e/review-fix-full-receipt.json` is complete
and accepted: 55/55 commands, native guard 2,150/2,150 matched (2,150 passed,
12 ignored), all 14 WASM launches, all five named regressions, and page tests
77/77 passed. Packaged build, CAD provider/browser checks and trusted drag/collision
smoke all ran; drag smoke passed 13/13, including exact Undo, Stagger/Offset drags,
qualified collision and product checks. Column rotation is covered by the five
WASM regressions above. No scheduled scope was unreached.

These are fresh whole-gate receipts for this source pin, not final ticket reviews.
17e and parent 17 remain **claimed** pending independent Standards+Spec rereviews;
no closure, release or user-data writer approval is recorded. Older success and
failure receipts above remain historical and unchanged.

## Answer

Resolved on 2026-10-23 after fresh gates and independent final re-reviews of the
full coordinated cutover. Earlier Progress checkpoints retain their historical
blocked/pending state; this Answer records final acceptance.

- Minimum fixture repairs preserve the intended boundary success/refusal scenarios,
  explicit Matrix correspondence admission and authored source-key rotation. Drag
  smoke reads the grouped persisted fields without weakening its gesture, durable
  save, cancellation or exact Undo assertions. Examples/starters remain owned by 26.
- Both final-review P2 findings are resolved: materialization and refresh share the
  original companion arithmetic order with exact refresh/reopen coverage; Column
  rotation and Splay use scope-correct reads throughout existing controls, with five
  failing-first WASM regressions. The native baseline contains 2,150 names; the WASM
  debt baseline is unchanged.
- Tested source: `47ce298020ef72903bab4032d5ee5980b7779215`, compared with fixed base
  `a58a2bf187bd6d08b652dc739cd7c391709ff5c8`. Quick covered all 119 paths and passed
  **54/54** commands; the full pre-PR check passed **55/55**. Both matched all
  **2,150** native names, with zero missing/new and 12 existing ignored. All 14 WASM
  launches passed, including layout **157+1**; the separate page suite passed
  **77/77**. CAD checks and the
  packaged trusted drag/collision smoke **13/13** passed. Evidence is in
  `.evidence/ergogen-17e/review-fix-{quick,full}-receipt.json` and the matching logs.
- Independent Codex Sol medium/fast re-reviews at
  `f67cd2b48217c1206d1c187cdfc13644da365cd2` returned **Standards: PASS** and
  **Spec: PASS**, with no remaining blocking or should-fix findings. Review rounds:
  `ergogen-17-final-standards-r2-sol` and `ergogen-17-final-spec-r2-sol`.
  Reviewers inspected source, fixtures and receipts without running builds/tests.
  Changes after the gate source are progress/closure documentation only.

The preexisting untracked prototype remains untouched. Before/after test-count
comparison is unknown because no base run was supplied. This closes the final gate
for parent 17's unshipped cutover, not tickets 18–27, release gate 27 or permission
to promote a user-data writer.
