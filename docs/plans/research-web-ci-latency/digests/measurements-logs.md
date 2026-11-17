# Measurements digest: hosted logs and run API

Sources: `/tmp/pr19-web-failure.log` (attempt 1, 3147 lines), `/tmp/pr19-web-success.log` (attempt 2, 3251 lines), `gh api repos/runinit/boardstudio/actions/runs/44018165934/attempts/{1,2}[/jobs]` and `check-runs/133821090974/annotations`, read 2026-11-17. Both attempts ran head `46d758d3fe8fc049084c87f077083dbe3eae137b`, event `pull_request`, image `ubuntu-26.04` 20261111.196.1. Times are UTC. "Log line" is the line number in the named log file.

## Job and step times (run API)

| | Attempt 1 (failure) | Attempt 2 (success) |
| --- | --- | --- |
| `web` job started | 04:35:11 | 05:54:46 |
| Setup steps before `check.py` | 33 s (04:35:11 to 04:35:44) | 23 s (05:54:46 to 05:55:09) |
| `check.py build browser` step | 04:35:44 to 05:09:01, failure (1997 s) | 05:55:09 to 06:30:55, success (2146 s) |
| Post steps | all skipped; `Complete job` has no conclusion | 18 s; caches saved |
| `web` job completed | 05:53:14 (2653 s after the step ended) | 06:31:15 |
| `native` job | 04:35:10 to 04:48:35, success | not rerun; attempt 1 result carried |

Attempt 1 `web` annotation, level `failure`: "The hosted runner lost communication with the server. Anything in your workflow that terminates the runner process, starves it for CPU/Memory, or blocks its network access can cause this error."

## Caches

Both attempts: `Cache not found for input keys: wasm-pack-0.15.0-wasm-bindgen-0.2.129` (log line 203), `Cache not found for input keys: cft-156.0.8078.73` (224), rust-cache `No cache found.` (338). Attempt 2 saved `cft-156.0.8078.73` (success log line 3214) and `wasm-pack-0.15.0-wasm-bindgen-0.2.129` (3218). Attempt 1 saved none: every post step skipped.

## Step breakdown (seconds)

| Segment | Attempt 1 | Attempt 2 | Source |
| --- | --- | --- | --- |
| `build` (`build-web.py --fast`) | 555.5 | 417.0 | `check: end` lines 965 / 962 |
| Gates beside prebuild (wall = prebuild) | 300.9 | 228.5 | lines 1284 / 1281 |
| ... prebuild `cargo test --no-run` compile | 5 m 00 s | 3 m 48 s | lines 1271 / 1268 |
| ... six gates summed | 238.1 | 219.6 | `check: end` lines |
| `test-cad-browser.py` | 16.0 | 26.9 | lines 1652 / 1649 |
| 14 crate launches, wall | 973.1 | 916.9 | first `check: start` to last `check: end` |
| Page `--list` (compiles the page test binary) | 44.7 (30.35 s compile) | 33.1 (23.70 s compile) | lines 3138 / 3130 |
| `run-wasm-tests.py --all` | killed after 106.7 | 523.7 | lines 3145 / 3141 |
| `browser` total | not reached | 1729.1 | success log line 3142 |
| Step total printed by `check.py` | not printed | 2146.1 | success log line 3142 |

Attempt 2 segments sum to 228.5 + 26.9 + 916.9 + 33.1 + 523.7 = 1729.1 s.

Inside `build`, cargo's own `Finished` lines: core worker 7 m 32 s / 5 m 45 s; CAD worker 12.32 s / 8.74 s; offline worker 27.63 s / 16.47 s; renderer 37.80 s / 26.77 s; CAD wasm 4 m 10 s / 3 m 14 s (attempt 1 / attempt 2, log lines 449, 462, 482, 520, 673).

## Crate launches

`wall` is the `(N s)` line `check.py` prints; `tests` is the runner's `finished in`; overhead is the difference.

| Launch | A1 wall | A1 tests | A1 overhead | A2 wall | A2 tests | A2 overhead |
| --- | --- | --- | --- | --- | --- | --- |
| pcb (66 tests) | 330.1 | 14.39 | 315.7 | 320.7 | 13.06 | 307.6 |
| case (100) | 334.4 | 18.87 | 315.5 | 324.3 | 16.41 | 307.9 |
| parts (108) | 73.5 | 26.21 | 47.3 | 336.8 | 26.85 | 309.9 |
| layout, skip `setup_guide::` (281) | 389.5 | 67.79 | 321.7 | 375.1 | 61.54 | 313.6 |
| layout, `setup_guide::` (2) | 300.2 | 1.14 | 299.1 | 305.4 | 0.68 | 304.7 |
| ui-shared, skip `panels::` (94) | 308.6 | 4.93 | 303.7 | 296.8 | 3.82 | 293.0 |
| ui-shared, `panels::` (2) | 302.8 | 0.49 | 302.3 | 291.9 | 0.40 | 291.5 |
| library (22) | 303.5 | 7.34 | 296.2 | 289.1 | 3.82 | 285.3 |
| keymap (48) | 302.1 | 7.81 | 294.3 | 290.7 | 5.70 | 285.0 |
| keycaps (34) | 300.2 | 3.86 | 296.3 | 20.5 | 2.12 | 18.4 |
| host (50) | 32.1 | 10.82 | 21.3 | 29.7 | 8.93 | 20.8 |
| runtime (69) | 28.1 | 4.76 | 23.3 | 24.9 | 4.67 | 20.2 |
| ui-model (9) | 22.8 | 0.08 | 22.7 | 15.8 | 0.06 | 15.7 |
| catalogue (12) | 19.1 | 0.67 | 18.4 | 11.1 | 0.28 | 10.8 |

- Nine of fourteen launches in each attempt carry 285 to 322 s of overhead; the other five carry 11 to 47 s.
- The slow set differs: parts is fast in attempt 1 and slow in attempt 2; keycaps is the reverse.
- Test time summed over 14 launches: 169 s (attempt 1), 148 s (attempt 2). Launch wall summed: 3347 s and 3233 s.
- Four `check: start` lines share one second at the start of the batch (success log lines 1650 to 1653), then one start follows each end.
- Every launch prints `Set WASM_BINDGEN_TEST_TIMEOUT to 120 seconds...` and `Try find webdriver.json ... Ok`; neither log has a `wasm-bindgen pair:` line.
- `Blocking waiting for file lock on package cache`: 36 lines (attempt 1), 31 (attempt 2); `... on build directory`: 2 and 2. They sit in the prebuild output and in the first launches (6 to 9 lines each for pcb, case, parts, layout).

## Silence (gaps between consecutive log lines, 45 s or more, inside the step)

| Attempt 1 | Attempt 2 | What was running |
| --- | --- | --- |
| 554.6 s | 415.9 s | `build-web.py --fast` lanes |
| 125.2 s | 104.3 s | `test-cad-capture-controls.py` beside the prebuild |
| 62.9 s | none | prebuild after the last gate |
| 73.5, 256.4, 281.3, 277.5 s | 320.7, 230.6, 232.2 s | crate launches |
| 92.4 s, then the shutdown | 454.2 s, then 59.9 s | `run-wasm-tests.py` filters |

Lines inside a captured block carry the time the block was printed, not the time they were produced: all build output (log lines 359 to 961) is stamped 06:02:05 in attempt 2 and 04:44:59 in attempt 1.

## Page runner

- Attempt 2: selection printed 06:22:21; `[1/3] model_decode_boundary_tests::` at 06:29:55; `[2/3] presentation::` at 06:30:55; `[3/3]` 7 ms later; `executed 263, failed 0`.
- Attempt 1: selection printed 05:07:28; no `[n/3]` line; 05:09:01.31 `##[error]The runner has received a shutdown signal`; `##[error]Process completed with exit code 143`; 05:09:01.57 `Cleaning up orphan processes`.
- Per-filter `duration_ms` lives in `web/target/test-results/browser.json`, which neither attempt uploaded.

## Not found

- No per-test or per-phase timestamps inside a launch: output arrives as one block.
- No CPU, memory or load figures in either log.
- No line naming who or what sent the shutdown signal.
