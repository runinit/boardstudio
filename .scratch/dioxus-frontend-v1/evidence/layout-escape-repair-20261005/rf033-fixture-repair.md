# RF-033 Layout component draft fixture repair

The prior known-failure entry in `scripts/wasm-known-failures.json` records the failing assertion as `left "4.00", right "7.25"`. The runner's earlier `executed 25, failed 0` summary excluded that known failure; it is not baseline GREEN evidence, and no raw terminal transcript for that earlier failure was retained.

Diagnosis: the mounted test fixture intercepts emitted Layout editor Events and returns before Core/Session applies them. The original test blurred X while moving to Y, then switched from Relations back to Properties. Production intentionally resets property drafts on that tab return. Since the fake sink had not accepted the X edit, the reset correctly restored 4.00. This was a fixture-path mismatch; no production behavior was changed.

The repair keeps each observation attributable: a focused X draft survives a direct unrelated accepted-state refresh while clean Y follows the refreshed value; X blur dispatches against the refreshed owner. A separate margin test verifies Enter dispatches a `ReplaceDocument` against the latest accepted revision while preserving the unrelated parameter and changed Y. A third test independently verifies Relations-tab retention. The fixture rerenders through its mounted test signal without moving focus or manufacturing accepted edit state.

Exact verification:

- Command: `python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs --result-json .scratch/dioxus-frontend-v1/evidence/layout-escape-repair-20261005/rf033-fixed-results.json`
- Terminal summary: `run-wasm-tests: executed 15, failed 0 (filters: presentation::layout_component_inspector_tests::)`
- Structured result: `complete=true`, 15 passed, 0 failed, 0 incomplete, no problems. The three new named mounted tests each have an explicit `passed` terminal outcome in the JSON.
- `rustfmt --edition 2024 --check web/src/presentation/layout_component_inspector_tests.rs` and `git diff --check -- web/src/presentation/layout_component_inspector_tests.rs` passed.
- Full current test-file SHA-256: `877cd4d45d18b811e9c90299225fb490130439a4d108871ea1cf3a275b651dfc`. The file also contains the concurrently prepared group-position test changes; those were preserved.
