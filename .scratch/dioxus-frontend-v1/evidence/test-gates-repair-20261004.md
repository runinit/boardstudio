# Native and wasm test gate repair — 2026-10-04

Scope: four independently reproduced gate defects in `scripts/run-wasm-tests.py` and `scripts/migration-deliver.py`. Regression tests were added before the implementation changes. No application builds or browser runs were needed.

## RED evidence

Focused pre-fix commands:

```text
python3 scripts/test-run-wasm-tests.py RunWasmTestsTests.test_allowlisted_invoked_but_unreported_test_still_fails RunWasmTestsTests.test_path_attribute_modules_ignore_aliases_inactive_for_wasm
python3 scripts/test-migration-deliver.py MigrationDeliverTests.test_changed_native_presentation_helper_does_not_require_wasm_tests MigrationDeliverTests.test_native_library_test_failure_blocks_commit MigrationDeliverTests.test_native_commit_rejects_zero_test_summary MigrationDeliverTests.test_native_commit_rejects_all_ignored_summary
```

The wasm cases were both RED. The allowlisted invocation returned success despite lacking a terminal result:

```text
AssertionError: 0 != 1 : run-wasm-tests: executed 1, failed 0 (filters: presentation::x::)
```

The target-specific alias selected the inactive native name:

```text
AssertionError: Lists differ: ['case_display::'] != ['presentation::case_display::']
```

The migration regressions were RED as follows:

- A changed native presentation helper ran a wasm filter with `running 0 tests` and the commit was rejected (`zero tests executed for filter case_display::`).
- A fake library-only failure did not block the commit (`AssertionError: 0 == 0`); the recorded Cargo invocation lacked `--lib`.
- Both the zero-test summary (`0 passed; 0 failed; 0 ignored`) and all-ignored summary (`0 passed; 0 failed; 3 ignored`) were accepted (`AssertionError: 0 == 0`).

## Changes and GREEN evidence

- Interrupted invocations now retain an `INCOMPLETE` status and add a fatal incomplete-execution problem, independent of the known-failure allowlist. Explicit reported assertion failures remain eligible for the existing allowlist.
- `#[path]` aliases are resolved from the wasm cfg-resolved module tree, excluding inactive native declarations.
- Changed wasm-relevant files still receive the wasm compile check. Headless wasm tests run for wasm-only files and files containing wasm-bindgen tests; a native-covered shared helper no longer fails on a meaningless zero-test wasm filter. A wasm-only file with zero wasm tests remains rejected.
- The web native command includes both `--lib` and `--bin boardstudio-web`; successful native checks require a summary with at least one passed test.

Full GREEN checks after the repair:

```text
python3 scripts/test-run-wasm-tests.py
Ran 14 tests ... OK

python3 scripts/test-migration-deliver.py
Ran 26 tests ... OK
```

The test suites use fake Cargo and wasm runners. No actual Cargo build, Chrome session, app behavior, commit, or history change was part of this packet.

## Relative-root follow-up

Regression command before the fix:

```text
python3 scripts/test-run-wasm-tests.py RunWasmTestsTests.test_path_attribute_modules_accept_dot_root
```

It errored under `--root .` while converting the resolved active wasm source to a repo-relative path:

```text
ValueError: '/tmp/run-wasm-tests-tdsay56w/web/src/main.rs' is not in the subpath of '.'
```

`path_attr_modules` now resolves the root once to an absolute `Path` before scanning and relativizing. After the fix:

```text
python3 scripts/test-run-wasm-tests.py
Ran 15 tests ... OK

python3 scripts/test-migration-deliver.py
Ran 26 tests ... OK
```

`git diff --check` also passed. This follow-up changed only the runner's root normalization and its focused regression test; existing `--depth` and allowlist updates in the committed gate work were preserved.

## `--files` module-isolation follow-up

New pre-fix regressions were run with:

```text
python3 scripts/test-run-wasm-tests.py RunWasmTestsTests.test_files_mode_expands_parent_prefix_to_isolated_matching_modules RunWasmTestsTests.test_files_mode_rejects_nonzero_partial_test_listing RunWasmTestsTests.test_files_mode_rejects_listed_tests_omitted_after_known_failure
```

All three were RED. A `presentation.rs` edit ran the raw `presentation::` filter and returned zero tests rather than isolating the listed presentation modules. A partial test listing with exit 1 was accepted, and an allowlisted terminal failure with a second listed-but-unreported test returned success.

`--files` now expands source prefixes against successful wasm test listings and runs listed module filters independently. It rejects empty or failed listings, refuses unrelated substring matches, checks that every listed test under a selected module reached a terminal result, and records bounded runner output for failure diagnostics. A deep source prefix without listed tests may widen only to exact listed modules under its parent; no raw substring filter is run. The existing `--all` path and its `--depth` option remain intact. The delivery test's fake runner now answers `--list`.

GREEN checks:

```text
python3 scripts/test-run-wasm-tests.py
Ran 20 tests ... OK

python3 scripts/test-migration-deliver.py
Ran 26 tests ... OK
```
