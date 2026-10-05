# F7.6 test packet preparation receipt

Prepared against `bda42cae76b342147330f971c108711b80549038`.

| File | Baseline SHA-256 |
|---|---|
| `web/src/runtime.rs` | `5d94109e52827ee05831c154329ec4af96a30bd6286c20447f543fcf77fc410f` |
| `web/src/presentation.rs` | `fae911b0f35c6d935c6ce512319b2ef8b83b70e8e5a3a34c3a63f7defacc7b5c` |
| `web/src/cad_presentation.rs` | `d7009cad80c088584ced010b3735220d0b2092801a5bcf473e5411216f2e9495` |

Prepared copies are under `copies/web/src/`. The applicable unified diff is `F76_REQUIRED_TESTS_READY.patch` (three paths only). It contains private cfg(test) fixtures/bridge and the two required mounted Case tests; no production behavior is changed. `git apply --check F76_REQUIRED_TESTS_READY.patch` passed on the clean owned paths. No live source was edited; no tests, compiler, package, or browser action was run. The prior `F76_REQUIRED_TESTS.patch` was preserved unchanged.

## Applied run on HEAD `cc728fc644d57b985feed9128ffdb40f9c75a709`

The patch was applied after confirming the three original baseline hashes still matched exactly and the worktree paths were clean. Only the three listed paths changed.

| Final source | SHA-256 |
|---|---|
| `web/src/cad_presentation.rs` | `4794fee20ac55784228ceef05d3433a08d0a18ddf7a54347f563fbbb1eec50f3` |
| `web/src/runtime.rs` | `33e568d43357bda42fdcb825d30e6ad71364aa43c405ab051939d73b08df92b6` |
| `web/src/presentation.rs` | `abae011da90fb4dcbe7d5ecfa957e084517eb23a5088957fed98b81c2f96519d` |

`git diff --check` passed. Focused browser tests, through `scripts/migration-deliver.py focused-test`, both passed 1/1:

- `cad_presentation::mounted_tests::mounted_failed_current_generation_keeps_previous_geometry_and_retries_owner` — `failed-current-mounted.log`.
- `cad_presentation::mounted_tests::mounted_read_model_preview_and_session_gesture_block_export_until_cancelled` — `preview-gesture-mounted.log`.

The wasm page/test graph compiled successfully during the first focused run. Existing unrelated unused/dead-code warnings appeared. No native test, broad suite, package, browser workflow, or public ZIP generation was run. Existing public ZIP/signature comparisons remain the output journey evidence.

## Reviewer-requested grouped DOM cleanup

After the superseded combined gate was explicitly stopped (its headless result was incomplete, not passed), applied `F76_DOM_CLEANUP.patch` to `cad_presentation.rs`. It scopes the existing exact-export test's refresh to its own root and removes all three export-test roots after assertions. No export assertions were weakened.

Final frozen source hashes:

| Source | SHA-256 |
|---|---|
| `web/src/cad_presentation.rs` | `2d42e194d135e1330c89ada46a4f6d798ffd4f3f2fc203c23149cba5b00e32ef` |
| `web/src/runtime.rs` | `33e568d43357bda42fdcb825d30e6ad71364aa43c405ab051939d73b08df92b6` |
| `web/src/presentation.rs` | `abae011da90fb4dcbe7d5ecfa957e084517eb23a5088957fed98b81c2f96519d` |

`git diff --check` passed. Full `cad_presentation::mounted_tests` module passed 4/4, 0 failed, 315 filtered, through `migration-deliver focused-test`; complete output is `cad-mounted-module-final.log`. The two new focused cases also passed individually before this cleanup; those logs remain retained. No additional source/tests/package/public output actions were performed.
