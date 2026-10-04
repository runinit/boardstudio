# Native/wasm commit gate review — 2026-10-04

Verdict: changes required. Reviewed commit `97926b0e3351e3259e02ee61f81613a9777cbe1f` against `9d4d70e3`; the pending tooling change landed during initial inspection. Scope: `AGENTS.md`, `scripts/migration-deliver.py`, `scripts/test-migration-deliver.py`, `scripts/run-wasm-tests.py`, `scripts/test-run-wasm-tests.py`, and `scripts/wasm-known-failures.json`.

Read `AGENTS.md`, `handoff.md`, the Codex route in `CONSTRAINTS.md`, and issue/domain guidance. The worktree has no `.codegraph/` directory despite the new AGENTS statement; targeted source reads were used. This is an independent correctness review of the delivery tooling, not a parent acceptance review.

## 1. P1 — An interrupted known test makes the wasm gate pass

Location: `scripts/run-wasm-tests.py:137-140`, `177-188`.

`run_filter` converts an `Invoking test:` line without a terminal result into `FAILED`. `run` then treats the presence of that synthetic failure as an explanation for a nonzero runner exit, and the known-failure allowlist removes it. If Chrome dies while executing a listed test, the runner reports success even though that test never completed and subsequent selected tests may never have run. This contradicts the source comment that an invoked but unreported test is a failure rather than a pass.

Deterministic probe: use an override runner that prints only `Invoking test: presentation::layout_component_inspector_tests::mounted_component_drafts_survive_unrelated_acceptance_and_blur_uses_latest_owner` and exits 1. The existing parser produces one synthetic `FAILED`; passing that result to `run` with the checked-in allowlist returns `executed=1`, `failed=[]`, `problems=[]`, so `main` returns 0. I reproduced the classification with `unittest.mock`, without starting Chrome or cargo.

Required regression: run an allowlisted test whose invocation has no terminal result; require failure. Preserve the distinction between explicit assertion failure and incomplete execution, and never allowlist an incomplete execution or a harness failure.

## 2. P2 — Valid native presentation helpers cannot pass the new wasm gate

Location: `scripts/migration-deliver.py:354-357`, `381-385`; `scripts/run-wasm-tests.py:40-66`, `172-175`.

Every changed Rust file beneath `web/src/presentation/` is sent to the wasm runner, even if its tests are correctly included in the native binary. Existing `case_display.rs`, `footprint_forms.rs`, and `parts/mechanical_profile.rs` have plain `#[test]` tests and native inclusion in `web/src/main.rs`; they have no wasm-bindgen tests. Their changed-module wasm runs therefore execute zero tests and reject an otherwise valid commit after native verification succeeds. The alias resolver additionally ignores cfg conditions: `case_display.rs` resolves to the native-only root alias `case_display::`, although its wasm source module is `presentation::case_display`.

Actual `filters_for_files` probes returned:

| Source | Selected filter |
| --- | --- |
| `web/src/presentation/case_display.rs` | `case_display::` |
| `web/src/presentation/footprint_forms.rs` | `footprint_forms::` |
| `web/src/presentation/parts/mechanical_profile.rs` | `parts_mechanical_profile::` |

With a runner result of exit 0 and `running 0 tests`, each produces a `zero tests executed for filter ...` problem. Source inspection establishes that these helpers already have native tests; no Rust build was needed.

Required regressions: edits to a helper covered only by valid native tests should pass; edits to an actually wasm-only module with zero wasm tests should still fail. Resolve module paths for the active wasm target rather than choosing an arbitrary `#[path]` alias found during a filesystem scan.

## 3. P2 — The native web gate omits the library test target

Location: `scripts/migration-deliver.py:320`.

The new native web command explicitly selects `--bin boardstudio-web`, excluding the library unit tests. `web/src/lib.rs` uniquely includes `offline` and, with the default `core-worker` feature, `layout_align_geometry_tests`. For example, changing `web/src/offline.rs` triggers the native binary command but neither the offline library tests nor a wasm test invocation. A regression in its cache-name or manifest validation tests can therefore pass this commit gate. Existing tests in `web/src/offline.rs:76-91` are concrete omitted coverage, not hypothetical future targets.

Required regression: make a library-only test fail while binary tests pass; require commit rejection. Include the native library target with the binary suite (or use the appropriate complete cargo target selection).

## 4. P2 — Native command success is accepted with zero executed tests

Location: `scripts/migration-deliver.py:325-338`.

`_run_native_tests` checks the exit status only. Unlike the existing `focused-test` wrapper, it accepts an all-ignored or zero-test summary as verification. `CONSTRAINTS.md` requires that a successful command with zero executed tests cannot count as verification. A direct mock returning exit 0 and `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` was accepted without a `GuardError`. The newly added green fixtures also use `test result: ok.` without proving any tests executed, so the tooling suite currently does not guard this condition.

Required regression: both zero-test and all-ignored native suite results must reject the commit; a positive executed count must pass. Reuse the existing Rust summary parser where appropriate.

## Verification and limits

The coordinator supplied existing passing tooling-suite evidence: 21 migration delivery tests and 12 wasm runner tests. This review reused that evidence and performed focused import/mock probes for the uncovered cases. No source scripts, staged files, browser state, builds, or Git history were changed; the only added worktree file is this report. No application tests or browser journeys were run under this review assignment.

## Independent repair re-review

Verdict: the four original correctness findings are resolved in the exact script versions below. No further actionable defect remains in the repaired gate logic after the relative-root follow-up. This verdict does not approve the externally added failure exclusions or establish application/release acceptance.

The four-script repairs were first inspected as a diff against `97926b0e`. During re-review an external coordinator committed them together with its `--depth` option and expanded allowlist in `8b9f83355dd6fd4c73b69978836721cfd340653b`. History was preserved. The final state reviewed here is that HEAD plus the two-file uncommitted relative-root follow-up.

Verified resolution:

- Allowlisted invoked-but-unreported tests now retain `INCOMPLETE` and create a fatal problem. A subprocess-output mock using the original allowlisted mounted-component test was rejected independently of any added exclusions.
- Native-covered presentation helpers retain the wasm compile check but avoid an unnecessary zero-test wasm invocation; actually wasm-only modules still require executable wasm tests. The cfg-aware resolver now selects `presentation::case_display::` rather than the inactive native alias.
- The native web command explicitly includes both `--lib` and `--bin boardstudio-web`, covering the previously omitted library unit tests. The library-failure fixture now rejects the commit.
- Native zero-test and all-ignored summaries reject the commit; a positive passed count succeeds. Independent summary mocks confirmed all three outcomes.

A new P2 regression was found during this re-review: absolute cfg-resolved module paths were made relative to an unresolved root, so `--root .` raised an uncaught `ValueError`. The baseline version accepted the same input. The author added a failing relative-root regression and normalized the root with `Path(root).resolve()`; the final probe now returns `['presentation::panels::scroll_tests::']`, and `case_display` resolves correctly with the same relative root.

Executed independently:

```text
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test-migration-deliver.py
Ran 26 tests ... OK

PYTHONDONTWRITEBYTECODE=1 python3 scripts/test-run-wasm-tests.py
Ran 15 tests ... OK
```

The migration-suite files remained at the same hashes after that test run; its evidence was reused while the runner-only relative-root follow-up settled. All checks used fake cargo/wasm runners or mocks. No actual build or browser session was run.

Exact reviewed SHA-256 hashes:

| File | SHA-256 |
| --- | --- |
| `scripts/migration-deliver.py` | `206a439bb414627e7c266f3f8b9450c2b59ad2941f7f646d758fb5c36fced186` |
| `scripts/test-migration-deliver.py` | `ec9572f4d09e6059f2863a22fa5bea655857237ec2340518069a3fe5cf1314c1` |
| `scripts/run-wasm-tests.py` | `9d9217b4e2fee14be24af6369d10c056e99e936e01b2d3c22b5a7feaa866c00f` |
| `scripts/test-run-wasm-tests.py` | `8df240fa33f3df429d25cc50e8a31f819080fce7b9f52294d39de550d654b0a9` |
| `scripts/wasm-known-failures.json` (observed external state) | `559692801be55cb442651b8d6251e3d893cc9c5e50292cfc9c0af25ed99d5e03` |

External change boundary: the checked-in allowlist now contains 13 entries versus four at the original review baseline. The nine new exclusions suppress additional assertion failures and were not assessed or accepted by this repair review. They must not be counted as passing test evidence or as reviewed changes. The externally introduced `--depth` option was preserved; the repair verdict covers the gate fixes rather than authorization of that extra scope.

Only this report was edited by the reviewer. No source edits, staging, commits, resets, or history changes were performed by this review.

## Runtime gate follow-up: isolate modules selected by files

Verdict: CLEAR for the frozen runner repair identified below. The prior four-finding source review remains clear. This is a tooling source/mock verdict; the coordinator owns the actual enforced wasm run and its result. No application acceptance or allowlist review is implied.

Trigger supplied by the coordinator: the actual commit gate selected the single broad `presentation::` filter for an edit to `presentation.rs`, so all selected presentation tests shared one browser page. That run reported 140 executions, two failures, and an incomplete mechanical contextual test. The existing `--all` path already divided tests by module. This follow-up extends listed-module selection to `--files` so a broad source prefix is resolved into narrower module invocations.

Reviewed against `faa10f38` and checked the exact frozen versions after the author settled:

| File | Reviewed SHA-256 |
| --- | --- |
| `scripts/run-wasm-tests.py` | `5bcdd888554876a367ac7adbf4874767cea76cfdd00c42dda9cf871b6752e1a4` |
| `scripts/test-run-wasm-tests.py` | `326ba282364a3b64669833201034c79a40cfad10cd1c52a39f52f6c7523bb362` |
| `scripts/test-migration-deliver.py` | `a4baac50548c384f0ea4f423cfd97118ef0d6d746fa61a1b2cd37547c9b948da` |

Correctness checks:

- Requested modules are selected with anchored `startswith` prefixes from a successful test listing. `presentation::` does not select `cad_presentation::`. If the substring-based underlying runner nevertheless executes an unexpected test, its name now creates a fatal problem rather than silently broadening coverage.
- A successful listing with no matching selected module creates a deterministic zero-test problem before execution; no unmatched raw filter is passed to the runner. Empty listings and nonzero listing exits are rejected, including exits that printed partial names.
- Each selected filter records all expected descendant test names, including descendants of a parent filter that collapsed nested modules. Every expected name must produce a terminal `ok` or `FAILED` result. Listed-but-uninvoked tests remain missing, and invoked-but-unreported tests remain fatal `INCOMPLETE` outcomes independently of any tolerated assertion failure.
- A listed module that unexpectedly executes zero tests cannot widen to another module to satisfy its expected coverage. The existing bounded parent widening during initial source selection considers only listed modules.
- The delivery integration fixtures now answer `--list`, preserving the existing native/library/zero-test gate regressions.

During draft review I independently reproduced two false successes: an unmatched `presentation::` prefix accepted output from `cad_presentation::`, and a collapsed parent filter accepted a completed direct test while omitting its listed nested test. Both are resolved in the frozen hashes. Final independent probes returned:

| Probe | Final result |
| --- | --- |
| Only `cad_presentation::tests::a` listed for requested `presentation::` | Failure; zero runner invocations |
| Direct and nested tests listed under one collapsed parent, only direct result returned | Failure; nested test reported missing |
| Two matching presentation modules plus a cad presentation module listed | Two separate matching invocations succeed; cad module excluded |

Executed independently after the source settled:

```text
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test-run-wasm-tests.py
Ran 20 tests ... OK

PYTHONDONTWRITEBYTECODE=1 python3 scripts/test-migration-deliver.py
Ran 26 tests ... OK
```

Hashes were checked again after the probes and match the author's frozen packet. This review edited only this report and ran Python/mock checks; no builds, browser sessions, application source edits, commits, or allowlist edits were performed. The external allowlist additions remain outside the review verdict.
