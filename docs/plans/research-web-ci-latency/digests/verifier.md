# Verifier digest (role taken inline)

The verifier was the same researcher, not a fresh context: each cited range was re-read from the numbered Gortex content of the file at `46d758d`, and each history claim against `git show -s` of its commit. This is weaker than an independent check. `recon_kit.py anchors --at 46d758d` re-found all 110 code citations (0 moved, 0 missing, 0 unparsed, exit 0).

| Ref | Citation as it should read | Verdict |
| --- | --- | --- |
| 1 | `.github/workflows/check.yaml:102-106 (jobs.web) @ 46d758d` | verified |
| 2 | `scripts/check.py:671 (run_parallel) @ 46d758d`; `scripts/check.py:279-284 (check_jobs) @ 46d758d` | verified |
| 3 | `scripts/check.py:584-590 (_parallel_process) @ 46d758d`; `scripts/check.py:721-723 (emit) @ 46d758d`; `scripts/check.py:636 (run_worker) @ 46d758d` | verified |
| 4 | `scripts/build-web.py:124-132 (run_pool) @ 46d758d` | verified: results print after the `with` block |
| 5 | `scripts/run-wasm-tests.py:256 (run_filter) @ 46d758d`; `scripts/run-wasm-tests.py:553-557 (results) @ 46d758d`; `scripts/run-wasm-tests.py:560-562 (run) @ 46d758d` | verified |
| 6 | `scripts/run-wasm-tests.py:512-514 (desktop_webdriver_config) @ 46d758d` | verified: the comment and `setdefault("pageLoadStrategy", "eager")` |
| 7 | `scripts/check.py:681-691 (interrupt) @ 46d758d`; `scripts/check.py:816-820 (run_parallel) @ 46d758d` | verified; `anchors` reports `symbol_found` false for line 816 only because `def run_parallel` is at line 600, beyond its 80-line window |
| 8 | `scripts/check.py:849-850 (run_parallel) @ 46d758d` | verified; same `symbol_found` note |
| 9 | `scripts/check.py:736-739 (run_parallel) @ 46d758d` | verified; same `symbol_found` note |
| 10 | `.github/workflows/check.yaml:44-47 (jobs.native) @ 46d758d` | verified for the comment; that the `web` job's rust-cache block (lines 94-101) also omits `cache-on-failure` was read directly |
| 11 | `bf6ba7288 "Stop browser launches waiting for the full page load"` | verified against the message; the 300 s figure is the message's claim, not read in the runner's source |
| 12 | `7f787b9cb "ci: let same-repo pull requests save their own rust-cache scope"` | verified against the message |

Not ledgered at `normal`: the other code claims, the test list in question 5 (names and line numbers only, re-found by `anchors`), and all measurements, which were recomputed once from the logs (sums in `measurements-logs.md` check to 0.1 s).
