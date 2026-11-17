---
title: 'Codebase research: web CI latency'
type: codebase
ask: |-
  Understand why the hosted `web` job waits so long and shows so little useful
  progress while it does: how build, binding generation, browser startup and
  execution add up; where scheduling and locks wait; how buffering hides
  activity; how caching, timeouts and shutdown work and why; and what tests
  cover these mechanisms.
repository: '.'
commit: '46d758d3fe8fc049084c87f077083dbe3eae137b'
branch: 'audit/layout-engine-pcb-layout'
status: complete
validation: 'normal'
claims: {verified: 12}
created: '2026-11-17'
updated: '2026-11-17'
---

# Codebase research: web CI latency

## Questions

1. `how does X work` — How do build, binding generation, browser startup and test execution contribute to the elapsed time of the `web` job's `check.py build browser` step?
2. `where does X live` — Where do scheduling, concurrency limits and package or build locks introduce waiting in that step?
3. `how does X work` — How do output buffering and progress reporting determine what the job log shows while the step runs?
4. `why is X this way` — How do caching, timeout and shutdown handling work in that step, and how did they come to be?
5. `what touches X` — Which tests cover these mechanisms?

One researcher took every role (locator, analyzer, historian, verifier) in sequence, knowing the ask, so the research firewall was not in force. Code was read through native Gortex on the exact worktree route at `46d758d`; the fallbacks are listed in `digests/locator-and-analyzer.md`.

Each claim is labelled by class. A **measurement** comes from the two hosted logs or the run API and is detailed in `digests/measurements-logs.md`; "A1" is attempt 1 (failed) and "A2" is attempt 2 (passed) of run 44018165934, both on this commit. A **mechanic** comes from code at the commit. A **history** claim comes from a commit. A **hypothesis** fits the evidence but is not shown by it. An **unknown** is named as such.

## Summary of what was verified

The four statements the brief asked to verify:

| Statement | Verdict |
| --- | --- |
| A2 step total 2146.1 s: build 417 s, browser 1729.1 s | Verified (measurement; build is 417.0 s). |
| Keymap launch about 290 s for about 8 s of tests | Verified and wider than stated. A2: 290.7 s for 5.70 s of tests. A1: 302.1 s for 7.81 s. Nine of fourteen launches in each attempt show the same 285 to 322 s of overhead. |
| A1 had cold caches and buffered timestamps | Verified, and A2 was cold too: all three caches missed in both attempts, because A1 saved nothing. |
| A1 shutdown with exit 143, then delayed finalization | Verified. The step ended at 05:09:01; the job completed at 05:53:14, 2653 s later, annotated "The hosted runner lost communication with the server". |

## 1. How do build, binding generation, browser startup and test execution contribute to elapsed time?

### What the step runs

- **Mechanic.** The `web` job restores three caches, then runs one command, `python3 scripts/check.py build browser`, with `CARGO_PROFILE_DEV_DEBUG=false`. `.github/workflows/check.yaml:102-106 (jobs.web) @ 46d758d`
- **Mechanic.** `build` is the single command `build-web.py --fast`. `scripts/check.py:203 (STEPS) @ 46d758d`
- **Mechanic.** `browser` is 24 commands in order: six gate scripts, the prebuild, `test-cad-browser.py`, fourteen `wasm-pack test` crate launches, a page `--list`, and `run-wasm-tests.py --all --depth 1`. `scripts/check.py:121-153 (browser) @ 46d758d`

### Measured shares (A2, the complete run)

| Segment | Seconds | Share of 2146.1 |
| --- | --- | --- |
| `build` | 417.0 | 19% |
| Gates beside the prebuild | 228.5 | 11% |
| `test-cad-browser.py` | 26.9 | 1% |
| Fourteen crate launches | 916.9 | 43% |
| Page `--list` | 33.1 | 2% |
| `run-wasm-tests.py --all` | 523.7 | 24% |

**Measurement.** The five browser segments sum to 1729.1 s. A1 was slower in every completed segment (build 555.5 s, gates 300.9 s, launches 973.1 s) and was killed 106.7 s into the page runner.

### Build (417 s in A2, 556 s in A1)

- **Mechanic.** `--fast` adds `--no-opt` to the four `wasm-pack build` provider commands and to the CAD wasm build, so the checked build skips wasm-opt. `scripts/build-web.py:61-70 (provider_commands) @ 46d758d`
- **Mechanic.** The build runs three lanes at once: the four provider `wasm-pack` builds in series, the CAD wasm build, and the `dx build` page lane. `scripts/build-web.py:288-292 (prepare_runtime_providers) @ 46d758d`
- **Mechanic.** The page lane joins the pool only for a fast, single-route build with providers. `scripts/build-web.py:257 (build) @ 46d758d`
- **Measurement.** With no rust-cache, the provider lane dominates: cargo reported the core worker at 5 m 45 s (A2) and 7 m 32 s (A1), then 8.74, 16.47 and 26.77 s for the next three in A2. The CAD lane reported 3 m 14 s (A2) and 4 m 10 s (A1).
- **Mechanic.** All provider and CAD builds are `--release`. `scripts/build-web.py:49-60 (provider_commands) @ 46d758d`

### Binding generation and test-binary compilation

- **Mechanic.** One `cargo test --no-run` for the wasm target compiles the test binaries of twelve web crates before any launch. `scripts/check.py:98-104 (BROWSER_PREBUILD) @ 46d758d`
- **Measurement.** That prebuild took 3 m 48 s (A2) and 5 m 00 s (A1), cold. It ran beside the gates, so it added 8.9 s (A2) and 62.8 s (A1) beyond the gates' own time.
- **Measurement.** After it, each launch's two cargo invocations report 0.15 to 0.68 s: launches do not recompile.
- **Measurement.** The page test binary is not in the prebuild; the `--list` command compiled `boardstudio-web` in 23.70 s (A2) and 30.35 s (A1).
- **Mechanic.** The web tools (wasm-pack 0.15.0, dx 0.7.10) are downloaded and checksum-verified on every job, not cached. `.github/actions/web-tools/action.yml:11-21 (runs) @ 46d758d`
- **Measurement.** That download step took about 1 s; installing Chrome for Testing on a cache miss took 4 to 7 s. Neither is material.
- **Unknown.** How long wasm-bindgen's own post-processing of each test binary takes inside a launch. The runner prints no timing for it.

### Browser startup and execution: the 300-second overhead

- **Measurement.** Tests are a small part of launch time. Summed over fourteen launches, the runner's `finished in` figures are 148 s (A2) and 169 s (A1); summed launch wall time is 3233 s and 3347 s.
- **Measurement.** Launches fall into two groups. Five carry 11 to 47 s of overhead beyond their tests. Nine carry 285 to 322 s. No launch falls between 47 and 285 s.
- **Measurement.** The slow group is not fixed. `parts` took 73.5 s in A1 and 336.8 s in A2; `keycaps` took 300.2 s in A1 and 20.5 s in A2. `host`, `runtime`, `ui-model` and `catalogue` were fast in both.
- **Measurement.** Slow launches pass. A two-test launch (`ui-shared`, `panels::`) took 291.9 s for 0.40 s of tests in A2.
- **Mechanic.** The code already names a 300 s wait in the runner: "The runner reads no test output until navigation returns, and allows that 300 s, so a page slow to reach `load` otherwise stalls a passing launch silently." The generated WebDriver config therefore sets `pageLoadStrategy: eager`. `scripts/run-wasm-tests.py:512-514 (desktop_webdriver_config) @ 46d758d`
- **Mechanic.** That config reaches the crate launches as well as the page runner: `run_plan` builds it once and the browser step's environment takes it. `scripts/check.py:1231-1237 (run_plan) @ 46d758d`, `scripts/check.py:1269-1271 (run_step) @ 46d758d`
- **Mechanic.** With no `web/webdriver.json` or root `webdriver.json` at the commit, the generated config starts empty and gets the window size, the Chrome binary and the eager strategy. `scripts/run-wasm-tests.py:488-514 (desktop_webdriver_config) @ 46d758d`
- **Hypothesis.** The nine slow launches each waited out a wait of about 300 s in the runner or the WebDriver session before tests began, and the eager strategy did not prevent it on the hosted runner. The evidence is the bimodal overhead, its closeness to 300 s, and the code comment. Neither log shows where inside a launch the time went.
- **Mechanic.** The repository carries an opt-in build of a patched runner pair that changes one condition in `tiny_http`'s task pool, the HTTP server library inside the runner. `scripts/build-patched-wasm-bindgen.py:33-34 (NEW) @ 46d758d`
- **Mechanic.** The pair is used only when `BOARDSTUDIO_WASM_BINDGEN_DIR` is set. `scripts/wasm_bindgen_tools.py:12-15 (configure) @ 46d758d`
- **Measurement.** The workflow does not set that variable and neither log prints the `wasm-bindgen pair:` line, so both attempts ran the unpatched runner that wasm-pack installed.
- **Unknown.** Whether the `tiny_http` condition the patch changes is what stalls a launch. No commit followed says why the patch exists.

### Page runner (524 s in A2)

- **Mechanic.** The runner lists the tests, groups them by first module segment, and runs one `wasm-pack test` per group plus one per isolated prefix. `scripts/run-wasm-tests.py:422-453 (all_module_selection) @ 46d758d`
- **Measurement.** A2 ran three filters and 263 tests. The first report line arrived 454.2 s after selection and the next two 59.9 s later.
- **Unknown.** Each filter's own duration. The runner records `duration_ms` per filter in `browser.json`, which was not uploaded in either attempt. `scripts/run-wasm-tests.py:586 (run) @ 46d758d`
- **Hypothesis.** The 454 s before the first report includes the same 300 s class of wait, since the first filter holds one test. The log cannot show it.

## 2. Where do scheduling, concurrency limits and locks introduce waiting?

### Concurrency limits

- **Mechanic.** `check_jobs()` is `BOARDSTUDIO_CHECK_JOBS` if set, else `min(6, cpu_count)`. `scripts/check.py:279-284 (check_jobs) @ 46d758d`
- **Mechanic.** The module docstring states a different default: "up to BOARDSTUDIO_CHECK_JOBS (default 4) at once". `scripts/check.py:32-33 (check.py) @ 46d758d`
- **Measurement.** On the hosted runner four launches started together and each later launch started as one ended, so the effective limit was four. The workflow does not set the variable.
- **Mechanic.** Consecutive crate launches form one batch on a thread pool of `check_jobs()` workers. `scripts/check.py:987-993 (run_commands) @ 46d758d`, `scripts/check.py:671 (run_parallel) @ 46d758d`
- **Mechanic.** The batch is ordered so that `layout`, `case`, `pcb` and `parts` start first. `scripts/check.py:220-221 (HEAVY_CRATES) @ 46d758d`, `scripts/check.py:664 (run_parallel) @ 46d758d`
- **Measurement.** With four slots and nine launches near 300 s, the batch needed three rounds of about 300 s: 916.9 s (A2) and 973.1 s (A1). Launch wall time summed to 3233 s, about 3.5 slots busy on average.
- **Mechanic.** The page runner caps itself at three filters at once, or one when `check_jobs()` is 1. `scripts/run-wasm-tests.py:549-557 (results) @ 46d758d`

### Serial stretches

- **Mechanic.** Outside the launch batch, the browser step is serial: each gate, `test-cad-browser.py`, the page `--list` and the page runner run one after another. `scripts/check.py:985-1036 (run_commands) @ 46d758d`
- **Mechanic.** The one overlap is the prebuild, which runs on its own thread beside the six gates and is joined before the launches. `scripts/check.py:947-954 (_run_browser_overlap) @ 46d758d`
- **Mechanic.** `build` and `browser` never overlap on the hosted runner. Step overlap needs `lint`, `test` and `build` together in the plan and at least eight CPUs. `scripts/check.py:1115-1137 (_overlap_split) @ 46d758d`
- **Mechanic.** The `web` and `native` jobs run side by side; `release-build` waits for both. `.github/workflows/check.yaml:119-120 (jobs.release-build) @ 46d758d`
- **Measurement.** The gates took 219.6 s (A2) in series. `test-cad-capture-controls.py` alone took 116.6 s (A2) and 146.2 s (A1), which includes building the Rust browser harness while the prebuild competes for the same four CPUs.
- **Mechanic.** That harness build is a separate `cargo build` with its own manifest, run under `flock` on the shared lock file when no lock descriptor is inherited. `scripts/browser_tools.py:98-103 (_build_harness) @ 46d758d`

### Locks

- **Mechanic.** The four provider `wasm-pack` builds are kept serial because they share one target directory, "where Cargo's single target-dir lock would serialize them anyway". `scripts/build-web.py:74-80 (provider_lanes) @ 46d758d`
- **Measurement.** Cargo printed `Blocking waiting for file lock on package cache` 31 times (A2) and 36 times (A1), and `... on build directory` twice in each. The lines sit in the prebuild output and in the launches that start together.
- **Unknown.** How long those waits lasted. Cargo prints the line without a duration, and the affected launches' cargo steps still report under 0.7 s.
- **Mechanic.** The cross-session build lock (`BOARDSTUDIO_BUILD_LOCK_FD`) applies only when the variable is set; otherwise `_inherited_build_lock` returns `None`. `scripts/browser_tools.py:67-71 (_inherited_build_lock) @ 46d758d`
- **Measurement.** The workflow does not set it, so that lock adds no wait on the hosted runner.
- **Mechanic.** Cargo's job count is fixed at 12 for every cargo, wasm-pack and dx build. `.cargo/config.toml:3-4 (build) @ 46d758d`
- **Hypothesis.** On a four-CPU runner, three build lanes at 12 jobs each, and later four browsers plus their servers, oversubscribe the CPUs. No CPU figure appears in either log.

### Workflow-level waiting

- **Mechanic.** Runs for one pull request share a concurrency group, and a newer push cancels the run in progress. `.github/workflows/check.yaml:9-14 (concurrency) @ 46d758d`
- **Measurement.** Neither attempt was cancelled that way: A1 concluded `failure`, not `cancelled`, and A2 is a rerun of the same run.

## 3. How do output buffering and progress reporting determine what the log shows?

### Three places hold output until a command ends

- **Mechanic.** `build-web.py` captures each lane command's output, and prints lane output only after the pool's `with` block has ended, which is after every lane has finished. `scripts/build-web.py:100-108 (_run_captured) @ 46d758d`, `scripts/build-web.py:124-132 (run_pool) @ 46d758d`
- **Measurement.** The log shows nothing for 415.9 s (A2) and 554.6 s (A1) during `build`. All build output, about 600 lines, then arrives stamped with one second.
- **Mechanic.** A parallel launch's output is read to end of stream into memory and returned only when the process has closed its output. `scripts/check.py:584-590 (_parallel_process) @ 46d758d`
- **Mechanic.** The collected text is printed whole, after the command line and before the elapsed time. `scripts/check.py:721-723 (emit) @ 46d758d`
- **Mechanic.** The prebuild is also captured and printed whole, so that it "must not interleave" with the gates. `scripts/check.py:527-532 (run_serial) @ 46d758d`
- **Mechanic.** The page runner captures each filter's output and prints at most the last 40 lines of it, and only for a filter with a problem. `scripts/run-wasm-tests.py:256 (run_filter) @ 46d758d`, `scripts/run-wasm-tests.py:600-604 (append_filter_log) @ 46d758d`

### What does print as it happens

- **Mechanic.** Each parallel worker prints `check: start <time> $ <command>` before launching and `check: end <time> exit <code> $ <command>` when the process ends. Both are flushed. `scripts/check.py:636 (run_worker) @ 46d758d`, `scripts/check.py:655 (run_worker) @ 46d758d`
- **Mechanic.** Serial commands print the same start and end lines, and their output streams straight through unless captured. `scripts/check.py:515 (run_serial) @ 46d758d`, `scripts/check.py:545-549 (run_serial) @ 46d758d`
- **Measurement.** During the launch batch those start and end lines are the only output. The longest runs without any line were 320.7, 230.6 and 232.2 s (A2) and 256.4, 281.3 and 277.5 s (A1).
- **Mechanic.** Nothing prints on a timer. No watcher thread reports "still running" for a launch, a lane or a filter.

### Timestamps and order

- **Measurement.** A line inside a captured block carries the time the block was printed. Timestamps in the log therefore say when a command ended, not when a line was produced.
- **Mechanic.** The start and end lines embed their own wall-clock time, which stays correct whatever the log viewer stamps. `scripts/check.py:635-636 (run_worker) @ 46d758d`, `scripts/check.py:652-655 (run_worker) @ 46d758d`
- **Mechanic.** Launch blocks print in completion order unless `in_order` is set; the launch batch does not set it. `scripts/check.py:736-739 (run_parallel) @ 46d758d`
- **Measurement.** The end line for a launch and its output block are separated by the next worker's start line, as at success log lines 1654 to 1656.

### The page runner reports in submission order

- **Mechanic.** Filter results are consumed in the order they were submitted, and `[i/n] <filter>` prints only once that filter's result is in hand. `scripts/run-wasm-tests.py:553-557 (results) @ 46d758d`, `scripts/run-wasm-tests.py:560-562 (run) @ 46d758d`
- **Mechanic.** So a later filter that finishes first stays unreported until every earlier one has finished.
- **Measurement.** A2 printed nothing for 454.2 s after the selection lines. A1 printed the selection lines and then nothing for 92.4 s until the shutdown, so the log holds no trace of which filter was running or how far it had got.
- **Mechanic.** `check.py` does not capture `run-wasm-tests.py`; its lines reach the log as it prints them. The silence comes from the runner's own capture. `scripts/check.py:538-546 (run_serial) @ 46d758d`

### What a failure adds

- **Mechanic.** When a launch fails, `check.py` prints a one-line summary extracted from the captured text after the block. `scripts/check.py:856-859 (_report_launch_failure) @ 46d758d`
- **Mechanic.** The failure artifact is uploaded only by a step that runs after a failed step. `.github/workflows/check.yaml:107-112 (jobs.web) @ 46d758d`
- **Measurement.** In A1 that upload step was skipped along with every post step, so no artifact exists for the failed attempt.

## 4. How do caching, timeout and shutdown handling work, and how did they come to be?

### Caching

- **Mechanic.** The `web` job has three caches: wasm-pack's tool cache keyed by tool versions, Chrome for Testing keyed by its pinned version, and rust-cache with the shared key `web` over two target directories. `.github/workflows/check.yaml:71-81 (jobs.web) @ 46d758d`, `.github/workflows/check.yaml:94-101 (jobs.web) @ 46d758d`
- **Mechanic.** rust-cache saves on `main`, `dev` and same-repository pull requests that are not Dependabot's. `.github/workflows/check.yaml:101 (jobs.web) @ 46d758d`
- **Mechanic.** `cache-on-failure` is not set, so a failed or cancelled job saves no rust-cache. `.github/workflows/check.yaml:44-47 (jobs.native) @ 46d758d`
- **History.** `7f787b9cb` "ci: let same-repo pull requests save their own rust-cache scope" gives the reason: the save "also runs when cancel-in-progress cancels a superseded run, and a partial target saved under the immutable key would be exact-hit, and never overwritten, by every later run."
- **History.** The same commit explains pull-request scope: GitHub writes a pull request's cache to `refs/pull/N/merge`, "which only later pushes to that pull request can restore", so that stacked pull requests "stop starting cold".
- **Measurement.** All three caches missed in A1. A1 then saved nothing, because its post steps were skipped. A2 missed all three again and ran as cold as A1. A2 saved the Chrome and wasm-pack caches; its log shows the rust-cache post step running for 13 s.
- **Unknown.** Why the first attempt found no rust-cache for this pull request's scope or its base. The log shows the miss, not the keys tried against existing caches.
- **Mechanic.** `CARGO_PROFILE_DEV_DEBUG=false` is set on the step rather than the job "so the rust-cache key stays shared with release-build". `.github/workflows/check.yaml:103-106 (jobs.web) @ 46d758d`
- **History.** `62a274623` "Compile every browser crate launch in one prebuild" replaced per-launch compiles: launches "recompiled Core and the web crates while queued on the build lock".
- **History.** `7102a22d0` "Overlap page and provider builds (ticket 05)" introduced the build lanes: "Cold build step 282s -> 197s".

### Timeouts

- **Mechanic.** The browser step sets `WASM_BINDGEN_TEST_TIMEOUT=120` unless the caller set it; the page runner and Cargo's config set the same default. `scripts/check.py:1277-1278 (run_step) @ 46d758d`, `scripts/run-wasm-tests.py:209-210 (runner_environment) @ 46d758d`, `.cargo/config.toml:10-11 (env) @ 46d758d`
- **Measurement.** Every launch echoes `Set WASM_BINDGEN_TEST_TIMEOUT to 120 seconds...`, and launches that took about 300 s still passed. Whatever that 120 s bounds, it did not bound the launch.
- **Mechanic.** No command launch in `check.py`, `run-wasm-tests.py` or `build-web.py` passes a timeout. The only timeouts in `check.py` bound cleanup waits after cancellation. `scripts/check.py:770-778 (cancel_workers) @ 46d758d`
- **Mechanic.** The workflow sets no `timeout-minutes` on the job or the step, so GitHub's default job limit applies.
- **History.** `bf6ba7288` "Stop browser launches waiting for the full page load" added the eager strategy: "wasm-bindgen-test-runner reads no test output until navigation returns and allows that 300 s, so a page slow to reach load stalled passing launches silently. Eager navigation returns at DOMContentLoaded, leaving the wait under the test timeout."
- **Measurement.** The stall that commit describes matches what both hosted attempts still show at this commit: passing launches near 300 s.
- **Mechanic.** The Rust browser harness the CAD gates use has its own bounds: 60 s each for closing, waiting on and killing the browser. `tools/browser-harness/src/shutdown.rs:13-25 (finish_browser) @ 46d758d`

### Shutdown

- **Mechanic.** `check.py` installs SIGINT and SIGTERM handlers only while a parallel batch runs, and restores the previous handlers afterwards. `scripts/check.py:681-691 (interrupt) @ 46d758d`, `scripts/check.py:816-820 (run_parallel) @ 46d758d`
- **Mechanic.** Inside a batch, a signal cancels the workers: SIGTERM to each child's process group, SIGKILL 0.5 s later, then up to 2 s for workers to finish. SIGTERM becomes `SystemExit(143)`. `scripts/check.py:759-779 (cancel_workers) @ 46d758d`, `scripts/check.py:849-850 (run_parallel) @ 46d758d`
- **Mechanic.** Each parallel child starts in its own session, which is what makes the group kill reach its descendants. `scripts/check.py:575-576 (_parallel_process) @ 46d758d`
- **Mechanic.** A serial command runs through plain `subprocess.run` with no handler installed. `scripts/check.py:484 (launch_command) @ 46d758d`
- **Mechanic.** `run-wasm-tests.py` installs no signal handler; its imports do not include `signal`. `scripts/run-wasm-tests.py:17-35 (run-wasm-tests.py) @ 46d758d`
- **Measurement.** A1 was in the serial `run-wasm-tests.py` command when the signal arrived, so none of the batch cleanup applied. The runner reported exit code 143 and then "Cleaning up orphan processes".
- **Measurement.** The job carries the annotation "The hosted runner lost communication with the server. Anything in your workflow that terminates the runner process, starves it for CPU/Memory, or blocks its network access can cause this error." GitHub marked the job complete 2653 s after the step ended.
- **Unknown.** What stopped the runner. The annotation lists three classes of cause and the log shows none of them directly.
- **Hypothesis.** Three concurrent headless Chrome sessions loading the page test binary starved the four-CPU runner of CPU or memory. The timing fits (the signal came 92 s after the three filters started); nothing measures it.
- **History.** `f8a0b8fbd` "Handle interrupted parallel checks" added the signal handling, with no message body. The reason for its design is not in the history followed.

## 5. Which tests cover these mechanisms?

All are Python `unittest` scripts that the `tooling` step runs in the `native` job. `scripts/check.py:78-88 (TOOLING_TESTS) @ 46d758d` Test names were read; test bodies were not, except one.

### Scheduling, ordering and progress lines

- `scripts/test-check.py:959 (test_parallel_heartbeat_tracks_started_and_completed_workers_before_blocked_first) @ 46d758d` — asserts the start and end lines of later workers print while the first is still blocked. Its body was read: it patches `_parallel_process` and checks for `exit 0 $ second` and `$ third`.
- `scripts/test-check.py:810 (test_browser_prebuild_overlaps_gates_but_finishes_before_browser_deadlines) @ 46d758d` — the prebuild overlap.
- `scripts/test-check.py:861 (test_browser_prebuild_and_gates_remain_serial_with_one_job) @ 46d758d` — the single-job path.
- `scripts/test-check.py:325 (test_browser_prebuild_covers_every_crate_launch) @ 46d758d` — the prebuild's crate list.
- `scripts/test-check.py:1258 (test_hosted_ci_plans_stay_serial_with_unchanged_command_order) @ 46d758d` — hosted plans do not overlap steps.
- `scripts/test-check.py:1231 (test_overlap_groups_lint_test_build_and_keeps_everything_else_serial) @ 46d758d` — step overlap rules.
- `scripts/test-check.py:1425 (test_overlapped_output_prints_whole_in_plan_order) @ 46d758d` — whole, ordered output of overlapped steps.
- `scripts/test-check.py:1466 (test_overlapped_total_is_wall_not_a_sum) @ 46d758d` — step-time arithmetic.
- `scripts/test-check.py:1083 (test_browser_keep_going_runs_remaining_launches_serially_and_exits_nonzero) @ 46d758d` — `--keep-going`.
- `scripts/test-run-wasm-tests.py:103 (test_parallel_filters_report_in_original_order_even_when_they_finish_out_of_order) @ 46d758d` — the page runner's ordered reports.
- `scripts/test-run-wasm-tests.py:144 (test_parallel_failing_filter_fails_the_run) @ 46d758d` — a failing parallel filter.
- `scripts/test-build-web.py:107 (test_provider_lanes_keep_wasm_pack_serial_and_cad_alone) @ 46d758d` — lane shape.
- `scripts/test-build-web.py:117 (test_provider_lanes_overlap_but_stage_only_after_all) @ 46d758d` — lane overlap.
- `scripts/test-build-web.py:152 (test_dx_lane_overlaps_providers_only_for_fast_single_route) @ 46d758d` — the page lane condition.

### Timeouts and WebDriver config

- `scripts/test-check.py:671 (test_browser_checks_supply_a_bounded_timeout_without_changing_other_steps) @ 46d758d`
- `scripts/test-check.py:686 (test_browser_checks_preserve_an_explicit_timeout) @ 46d758d`
- `scripts/test-check.py:773 (test_a_harness_timeout_before_any_test_output_fails_closed_on_one_launch) @ 46d758d`
- `scripts/test-run-wasm-tests.py:27 (test_batch_timeout_is_bounded_and_respects_explicit_override) @ 46d758d`
- `scripts/test-run-wasm-tests.py:759 (test_desktop_webdriver_config_is_wide_and_preserves_explicit_override) @ 46d758d`
- `scripts/test-run-wasm-tests.py:778 (test_first_failure_summary_surfaces_raw_wasm_trap_before_harness_timeout) @ 46d758d`

### Shutdown and interruption

- `scripts/test-parallel-interrupt.py:13 (ParallelInterruptTest) @ 46d758d` — eighteen tests of `run_parallel` under signals, among them:
- `scripts/test-parallel-interrupt.py:571 (test_sigterm_terminates_owned_child) @ 46d758d`
- `scripts/test-parallel-interrupt.py:592 (test_two_interrupts_terminate_owned_process_group) @ 46d758d`
- `scripts/test-parallel-interrupt.py:222 (test_interrupt_cleans_descendant_after_command_completed) @ 46d758d`
- `scripts/test-parallel-interrupt.py:456 (test_cleanup_returns_while_escaped_descendant_holds_stdout) @ 46d758d`
- `scripts/test-check.py:594 (test_ordered_batch_interrupt_prints_finished_output) @ 46d758d`
- `scripts/test-check.py:1492 (test_overlap_interrupt_ends_every_child_and_removes_the_fifo) @ 46d758d`
- `scripts/test-browser-tools.py:48 (test_cancelled_launcher_keeps_build_lock_until_child_exits) @ 46d758d`

### Caching, concurrency group and locks

- `scripts/test-ci-workflow.py:58 (test_rust_cache_save_matrix) @ 46d758d`
- `scripts/test-ci-workflow.py:80 (test_cache_on_failure_stays_off) @ 46d758d`
- `scripts/test-ci-workflow.py:104 (test_tool_caches_are_version_keyed_and_release_only_restores) @ 46d758d`
- `scripts/test-ci-workflow.py:113 (test_cache_key_sees_only_the_pinned_toolchain) @ 46d758d`
- `scripts/test-ci-workflow.py:138 (test_only_pull_request_runs_are_cancelled) @ 46d758d`
- `scripts/test-browser-tools.py:206 (test_build_lock_across_scheduler_and_standalone) @ 46d758d`
- `scripts/test-check.py:34 (test_preflight_rejects_invalid_lock_for_focused_browser_and_quick) @ 46d758d`
- `scripts/test-build-patched-wasm-bindgen.py:41 (test_build_patches_and_builds_both_locked_offline_then_installs) @ 46d758d`

### Looked for and not found

Searched the test-name lists of the seven files above at `46d758d`:

- No test name mentions a signal reaching a serial command or reaching `run-wasm-tests.py`.
- No test name mentions launch duration, a wait near 300 s, or the eager page-load strategy by name; whether `test_desktop_webdriver_config_is_wide_and_preserves_explicit_override` asserts `pageLoadStrategy` was not read.
- No test name mentions the silent interval of `build-web.py` lanes or a periodic progress line.
- No test name mentions the `check_jobs()` default against the docstring's stated default.
- All of these tests use fakes or local processes; none runs on the hosted runner's browser path.

## Where the answers meet

- **`run_parallel`**, reached by questions 2, 3 and 4. It sets the launch concurrency, holds each launch's output until the launch ends, and owns the only signal handling. `scripts/check.py:671 (run_parallel) @ 46d758d`, `scripts/check.py:721-723 (emit) @ 46d758d`, `scripts/check.py:681-691 (interrupt) @ 46d758d`
  - Raises `SystemExit(143)` on SIGTERM. `scripts/check.py:849-850 (run_parallel) @ 46d758d`
- **`desktop_webdriver_config`**, reached by questions 1 and 4. It is where the 300 s navigation allowance is named and the eager strategy set, for crate launches and page filters alike. `scripts/run-wasm-tests.py:512-514 (desktop_webdriver_config) @ 46d758d`
  - Raises `RunnerError` when a caller-supplied config names a different browser. `scripts/run-wasm-tests.py:473-474 (desktop_webdriver_config) @ 46d758d`
- **`run_filter` and `run`** in the page runner, reached by questions 1, 2, 3 and 4. They capture output, cap concurrency at three, report in order, and run with no handler and no timeout. `scripts/run-wasm-tests.py:256 (run_filter) @ 46d758d`, `scripts/run-wasm-tests.py:553-557 (results) @ 46d758d`
  - Records an invoked test with no terminal line as `INCOMPLETE`, which fails the run. `scripts/run-wasm-tests.py:264-265 (run_filter) @ 46d758d`, `scripts/run-wasm-tests.py:611-614 (run) @ 46d758d`
- **`run_pool`** in the build, reached by questions 1, 2 and 3. It overlaps the lanes and prints them only after all have ended. `scripts/build-web.py:124-132 (run_pool) @ 46d758d`
  - Re-raises the first lane failure after printing that lane's output. `scripts/build-web.py:129-131 (run_pool) @ 46d758d`

## Open questions

Each is where research stopped, with the next factual question.

1. **Inside a slow launch.** The logs give one block per launch. Where does a launch spend its time between `Running headless tests in Chrome` and `running N tests`? This needs per-phase timestamps from a run, or the runner's source.
2. **The runner's 300 s.** The source of `wasm-bindgen-test-runner` 0.2.129 was not available. Which wait in that runner or in ChromeDriver lasts 300 s, and does `pageLoadStrategy: eager` apply to it?
3. **The patched pair.** What does the `tiny_http` task-pool condition change in the runner's server, and why was the patch written? No commit followed explains it.
4. **Why some launches are never slow.** `host`, `runtime`, `ui-model` and `catalogue` were fast in both attempts. What do those four crates' test pages lack that the other eight load?
5. **Per-filter page durations.** What `duration_ms` does `browser.json` record for the three filters on a hosted run?
6. **The lost runner.** What were CPU and memory use on the runner at 05:09 in attempt 1? Neither log records them.
7. **The first cold start.** Which rust-cache keys existed for this pull request's scope and its base branch when attempt 1 began?
8. **History not found.** Why three page-runner workers, why `min(6, cpu)`, and why signal handling only inside the batch? The commits carry no message body; their pull requests were not looked up because the history budget was spent.

## Prior documents

Listed by title only; none was opened, since the ask names none.

- Research folders beside this one: `research-editor-workspace-ownership`.
- Decision records: the index at `docs/adr/README.md`.
- Not evidence and not read: every plan, ticket and retrospective under `docs/plans/`.
