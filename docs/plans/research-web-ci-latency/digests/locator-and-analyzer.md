# Locator and analyzer digest (roles taken inline)

One researcher took the locator and the five analyzer roles in sequence, knowing the ask. The research firewall was not in force. Code was read through native Gortex `read.file` on the exact worktree route (`worktree:01a1e162-10eb-7118-bffd-92e655700b20`, `exact: true`, head `46d758d`); the tracked tree equals the commit (only three untracked protected paths differ, none opened). Line numbers were derived from the Gortex content. Native fallbacks: `git ls-tree` and `git grep -l` at the commit for the area listing and file location; `git show` at the commit for the two `webdriver.json` files.

## Locator: files per question

| File | Lines | Questions |
| --- | --- | --- |
| `.github/workflows/check.yaml` | 162 | 1, 2, 4 |
| `.github/actions/web-tools/action.yml` | 24 | 1, 4 |
| `scripts/check.py` | 1494 | 1, 2, 3, 4 |
| `scripts/run-wasm-tests.py` | 777 | 1, 2, 3, 4 |
| `scripts/build-web.py` | 334 | 1, 2, 3 |
| `scripts/browser_tools.py` | 139 | 2, 4 |
| `scripts/wasm_bindgen_tools.py` | 40 | 1 |
| `scripts/build-patched-wasm-bindgen.py` | 120 | 1, 4 |
| `scripts/install-chrome-for-testing.sh` | 65 | 1, 4 |
| `.cargo/config.toml` | 11 | 2, 4 |
| `web/crates/layout/webdriver.json`, `web/crates/ui-shared/webdriver.json` | 3 each | 1 |
| `tools/browser-harness/src/shutdown.rs` | 70 (first 25 read) | 4 |
| `scripts/test-check.py`, `test-run-wasm-tests.py`, `test-parallel-interrupt.py`, `test-ci-workflow.py`, `test-build-web.py`, `test-browser-tools.py`, `test-build-patched-wasm-bindgen.py` | test names only | 5 |

Files opened per question: Q1 9, Q2 6, Q3 3, Q4 8, Q5 7 (names only). No budget was reached.

## Analyzer: load-bearing claims

Each is carried into the document with its citation; the ledger refs are in `.memlog.md`.

1. The `web` job runs one step, `python3 scripts/check.py build browser`, after three cache restores. `.github/workflows/check.yaml:102 (jobs.web) @ 46d758d`.
2. `build` is one command, `build-web.py --fast`; `browser` is 24 commands. `scripts/check.py:203 (STEPS) @ 46d758d`, `scripts/check.py:121-153 (browser) @ 46d758d`.
3. Crate launches run in a thread pool of `check_jobs()` workers, default `min(6, cpu_count)`. `scripts/check.py:671 (run_parallel) @ 46d758d`, `scripts/check.py:279-284 (check_jobs) @ 46d758d`.
4. A parallel launch's output is read to end of stream and printed whole afterwards; only `check: start` and `check: end` print as they happen. `scripts/check.py:587-590 (_parallel_process) @ 46d758d`, `scripts/check.py:636 (run_worker) @ 46d758d`, `scripts/check.py:655 (run_worker) @ 46d758d`, `scripts/check.py:721-723 (emit) @ 46d758d`.
5. `build-web.py` prints lane output only after every lane has finished. `scripts/build-web.py:124-132 (run_pool) @ 46d758d`.
6. The page runner captures each filter's output, runs at most three filters at once and prints `[i/n]` in submission order after the result arrives. `scripts/run-wasm-tests.py:256 (run_filter) @ 46d758d`, `scripts/run-wasm-tests.py:553-557 (results) @ 46d758d`, `scripts/run-wasm-tests.py:562 (run) @ 46d758d`.
7. The only bounded wait the step sets is `WASM_BINDGEN_TEST_TIMEOUT=120`; no subprocess call passes a timeout and the workflow sets no `timeout-minutes`. `scripts/check.py:1278 (run_step) @ 46d758d`, `scripts/run-wasm-tests.py:210 (runner_environment) @ 46d758d`.
8. `check.py` handles SIGINT and SIGTERM only inside `run_parallel`. `scripts/check.py:681-691 (interrupt) @ 46d758d`, `scripts/check.py:816-820 (run_parallel) @ 46d758d`.
9. rust-cache saves only when the job succeeds: `cache-on-failure` is not set. `.github/workflows/check.yaml:44-47 (jobs.native) @ 46d758d`, `.github/workflows/check.yaml:94-101 (jobs.web) @ 46d758d`.
10. The generated WebDriver config sets `pageLoadStrategy: eager`, with a comment naming a 300 s navigation allowance in the runner. `scripts/run-wasm-tests.py:512-514 (desktop_webdriver_config) @ 46d758d`.

## Looked for and not found (at `46d758d`)

- `timeout-minutes`, `BOARDSTUDIO_WASM_BINDGEN_DIR`, `BOARDSTUDIO_CHECK_JOBS` under `.github/`: no match.
- `signal.` in `scripts/run-wasm-tests.py`: no match.
- `web/webdriver.json`, `webdriver.json` at the root: absent; only the two crate files exist.
- A periodic progress line (timer or heartbeat thread) in `check.py`, `run-wasm-tests.py` or `build-web.py`: none; the test named `heartbeat` asserts the start and end lines.
- The source of `wasm-bindgen-test-runner` 0.2.129: not in the repository and not in the local Cargo registry, so the 300 s the comment names was not read in code.
