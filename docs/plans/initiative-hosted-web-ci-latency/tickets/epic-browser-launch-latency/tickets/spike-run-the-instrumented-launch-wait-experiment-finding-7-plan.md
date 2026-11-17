---
title: 'Run the instrumented launch-wait experiment (finding §7)'
type: feature
ticket: 2
created: '2026-11-17'
status: ready-for-dev
baseline_revision: '8e7a6c05ed8b97fc3ca6b2c24e8ef8e59c444533'
route: full
route_source: auto
risk: medium
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: false
context: []
warnings: [oversized]
deferred: []
---

<intent-contract>

## Intent

**Problem:** Finding 1.1 (`finding-launch-wait.md` §7) specifies one instrumented experiment to settle which request stalls in a slow hosted crate launch and what releases it (H1–H4), but its patched §6–§7 passages had no independent re-review and the design lacks executable mechanics.

**Approach:** First re-review the patched §6–§7 passages against the same checksum-verified exact sources (done at planning; verdicts in Design Notes, one amendment folded in below). Then, only after the `plan_checkpoint` is released, build the exact 0.2.129 runner with logging only, smoke it locally, and run up to three hosted 14-launch batches, stopping at the first conclusive one, recording the verdict in `finding-launch-wait-experiment.md`.

## Boundaries & Constraints

**Always:** No fix is applied; the `tiny_http` queue condition stays byte-identical to stock 0.12.0 and the existing patched pair is never built or selected. Crates, filters, and test counts unchanged (`check.py build browser` as hosted runs it). Local cargo/check runs use the AGENTS.md build-lock wrapper (`exec 9>~/.cache/tmp/boardstudio-build.lock; flock 9; export BOARDSTUDIO_BUILD_LOCK_FD=9`), one build per target at a time, `.cargo/config.toml` governs flags (never `CARGO_BUILD_JOBS=2`/`CARGO_INCREMENTAL=0`/`CARGO_PROFILE_DEV_DEBUG=2`). Long output and all downloaded logs go to `.evidence/launch-wait-experiment/` (logs only; no target dirs or source copies inside). Instrumentation lives only on the experiment branch and in the instrumented build's temp source; it never merges as a fix.

**Never:** No timeout/concurrency changes mid-experiment; no batch beyond the third; no push, PR, or hosted run before the checkpoint release explicitly covers it; no touching `epic-ci-progress-feedback/**`, production source, or the three protected untracked files present in this shared checkout (`necessist.db` and the two protected docs — the working tree is "dirty" only with those, as in spike 1.1).

</intent-contract>

## Code Map

- `scripts/build-patched-wasm-bindgen.py` — template to copy for the instrumented build (checksum-verified crate fetch, `--locked` release build of both bins, `--version` assert); its `OLD→NEW` queue rewrite (lines 33–34) must NOT be copied.
- `scripts/wasm_bindgen_tools.py:9-15,26-39` — `BOARDSTUDIO_WASM_BINDGEN_DIR` selection: validates both bins report the lockfile version, prepends the dir to PATH so wasm-pack 0.15 picks the sibling test-runner. This is how the instrumented pair is selected, locally and hosted; no other wiring.
- `scripts/check.py:121-153` (`browser()`), `:279-284` (`check_jobs`, hosted 4 CPU ⇒ 4 concurrent launches), `:1231-1237` (one shared webdriver config/env per plan), `:1269-1278` (pair configure + `WASM_BINDGEN_TEST_TIMEOUT=120`).
- `scripts/run-wasm-tests.py:512-518` — shared `webdriver.json` with `pageLoadStrategy: eager`; unchanged.
- `.github/workflows/check.yaml` `web` job — experiment branch adds: instrumented-pair build step, two env vars on the `check.py build browser` step, an `always()` artifact upload. Nothing else in the workflow changes; a draft PR triggers `native`+`web` only (release jobs never run on PRs — do not use `workflow_dispatch`, which would run release-build/deploy).
- External exact sources (checksums in finding §1; read from `/tmp/bs-spike-1.1` during planning, re-fetched/verified by the build script at execution): `wasm-bindgen-cli` 0.2.129 `src/wasm_bindgen_test_runner/{headless.rs,server.rs,wasm_bindgen_test_runner.rs}`, `tiny_http` 0.12.0 `src/util/task_pool.rs`.

## Tasks & Acceptance

**Execution:** (all tasks after Task 0 wait for the checkpoint release)

- [ ] Task 0 — `plan_checkpoint` gate: stop here. A person releases the checkpoint, choosing hosted vs local-only (Design Notes) and the experiment branch base. Nothing below runs before that.
- [ ] `scripts/build-instrumented-wasm-bindgen.py` — create on experiment branch `experiment/launch-wait-instrumented`, modeled on `build-patched-wasm-bindgen.py`: fetch/verify the same two checksum-pinned crates, assert the stock queue condition `waiting_tasks.load(Ordering::Acquire) == 0` is present exactly once and leave it untouched, apply only the logging edits below (each an exact-match, count-asserted replacement; abort on any mismatch), rewrite the cli lock's `tiny_http` entry to the path patch as the patched script does, `cargo build --release --locked` both bins into a job-local `CARGO_TARGET_DIR` beside the output (never the shared target), assert `--version` output, emit the pair. Logging edits, all lines formatted `wall_ms=<unix epoch ms> mono_ms=<process-relative> pid=<pid> port=<server port> ev=<event> k=v…`, appended to `$BOARDSTUDIO_LAUNCH_LOG_DIR/runner-<pid>-<port>.log` (if the env var is unset, instrumentation is inert and stock behavior is byte-for-byte the log-free path):
  - `wasm_bindgen_test_runner.rs` (`rmain`): `bindgen_start`/`bindgen_end`, `server_start` with bound port.
  - `headless.rs`: `driver_spawn`, `session_sent`/`session_returned`, `navigate_sent`/`navigate_returned` with the raw response body (the legacy `"status"` field, finding §5; `doit` already returns it), `first_poll`, `test_result_seen`; in `Client::new_session` Chrome branch (`headless.rs:603-618`) append switch `log-net-log=$BOARDSTUDIO_LAUNCH_LOG_DIR/netlog-<pid>-<port>.json` (bare style like the existing switches; ChromeDriver `SetUnparsedSwitch` accepts it and a later same-name switch wins); at the driver spawn (`headless.rs:99`, beside `--port=`) append `--verbose --log-path=$BOARDSTUDIO_LAUNCH_LOG_DIR/chromedriver-<pid>-<port>.log` (per-launch path cannot come from the shared `CHROMEDRIVER_ARGS` env).
  - `server.rs` Rouille handler closure: `req_received url=…` at entry and `req_responded` via a drop guard, so no handler restructuring.
  - `tiny_http/src/util/task_pool.rs` `TaskPool::spawn`: assign a sequential `conn=<n>` id, log `spawn conn waiting=<waiting_tasks> queue=<queue.len()> branch=<new_thread|queued>`, and wrap the task closure to log `task_start conn` / `task_end conn` around the unchanged call. No condition, ordering, or timeout changes.
- [ ] Local smoke (not a batch; proves producibility only) — build the pair to `~/.cache/boardstudio-instrumented/wasm-bindgen/` under the build-lock wrapper, then under the same wrapper run `BOARDSTUDIO_WASM_BINDGEN_DIR=… BOARDSTUDIO_LAUNCH_LOG_DIR=/home/chris/.t3/worktrees/ts-boardstudio2/audit-layout-engine-pcb-2e505e394500cb50/.evidence/launch-wait-experiment/smoke python3 scripts/check.py browser --only host ui-shared > .evidence/launch-wait-experiment/smoke.log 2>&1`. Verify: `wasm-bindgen pair:` line printed, per-launch runner/netlog/chromedriver files exist per pid, net-log parses (tolerate a truncated tail if Chrome was killed: repair by trimming to the last complete event), the alignment below yields sane ordering on a fast launch. Also run the §6 amendment check (metadata only, no build): `cargo tree --locked --offline -e features -i dioxus-web --target wasm32-unknown-unknown` for the browser unit set, recording whether `dioxus-web/document` is resolved on; redirect output to `.evidence/launch-wait-experiment/resolved-features.log`.
- [ ] `.github/workflows/check.yaml` (experiment branch only) — in `web`: add a step `python3 scripts/build-instrumented-wasm-bindgen.py --output "$RUNNER_TEMP/wasm-bindgen-instrumented"` (job-local dir per the script's contract), set `BOARDSTUDIO_WASM_BINDGEN_DIR=$RUNNER_TEMP/wasm-bindgen-instrumented` and `BOARDSTUDIO_LAUNCH_LOG_DIR=$RUNNER_TEMP/launch-logs` on the existing `check.py build browser` step, add `actions/upload-artifact` `if: always()` named `launch-wait-logs` uploading `$RUNNER_TEMP/launch-logs` and `web/target/test-results/browser.json`, `retention-days: 7`. Open a **draft** PR from the experiment branch; its first `web` job is batch 1.
- [ ] Batch loop (max three; bookkeeping table appended to this plan's Implementation Notes after each batch: batch #, run id + attempt, slow launches ≥240 s overhead with complete tri-logs?, verdict, next decision): download the batch artifact BEFORE any rerun (`gh run download <run-id> -n launch-wait-logs -D /home/chris/.t3/worktrees/ts-boardstudio2/audit-layout-engine-pcb-2e505e394500cb50/.evidence/launch-wait-experiment/batch-<n>`), compute per-launch overhead as launch wall (check.py timing / runner log span) minus poll-phase test time, pick one slow launch, align its three logs (method in Design Notes), read H1–H4 off the finding §7 table. Conclusive ⇒ stop. No slow launch or incomplete logs ⇒ `gh run rerun <run-id> --job web` for the next batch (web-only, native not repeated). After batch 3 with no conclusive slow launch, stop and report inconclusive; never a fourth.
- [ ] `docs/plans/initiative-hosted-web-ci-latency/tickets/epic-browser-launch-latency/finding-launch-wait-experiment.md` — write the experiment finding: which of H1–H4 settled off one slow launch's aligned logs (or inconclusive after three batches), the stalled request and release event with timestamps, snippet requests actually seen per crate (net-log request lists, settling §6 reachability), the resolved-features result, batch bookkeeping summary, and the §6 correction note from Design Notes. Then the `done_checkpoint`: stop for user acceptance; delete the experiment branch only on the user's word (it is never merged).

**Acceptance Criteria:**

- Given the checkpoint is released for hosted, when the draft PR's `web` job runs, then the job log shows `wasm-bindgen pair: $RUNNER_TEMP/wasm-bindgen-instrumented (version 0.2.129)` and the `launch-wait-logs` artifact holds one runner log, one net-log, and one chromedriver log per launch pid.
- Given a batch with a launch of ≥240 s overhead, when its three logs are aligned, then exactly the finding §7 settling observations decide H1/H2/H3/H4 or declare the capture incomplete — no verdict is read off fewer than the three logs.
- Given three batches with no slow launch, when the loop ends, then the experiment finding reports inconclusive (contention not reproduced) and no further batch is run.
- Given any batch, when its test results are compared to stock, then all fourteen launches and the page tests run with unchanged filters and counts; a pass/fail difference vs the A1/A2 shape is reported as an instrumentation side effect, not a fix.
- Given the instrumented sources, when diffed against the checksum-verified stock crates, then every hunk is logging-only and the `tiny_http` spawn condition is unchanged.

## Design Notes

**Follow-up review of the patched §6–§7 passages (done at planning, fresh context, source-reading only).** Sources re-verified by sha256 against the finding §1 anchors: `wasm-bindgen-cli` 0.2.129 (`5fd044ed…03168c`), `tiny_http` 0.12.0 (`389915df…cdc82`), `rouille` 3.6.2 (`3716fbf5…fcc921`), `wasm-bindgen-cli-support` (`a0724352…cc65a`), Chromium 156.0.8078.73 files from spike 1.1's fetch, registry `dioxus`/`dioxus-web` 0.7.10 (checksums equal `Cargo.lock`'s `48109323…`/`9320593e…`), the vendored interpreter, and local manifests/scripts at HEAD `8e7a6c05e`.

1. **§6 snippet claim — AMENDED.** The rested claim holds: `dioxus-web/Cargo.toml:116-121` depends on `dioxus-interpreter-js` non-optionally with `minimal_bindings`+`webonly`; the vendored `src/lib.rs:28` gates the three externs (`:35,41,56`) exactly on those features; local crate table, `mounted = [web-sys items]` (`dioxus-web/Cargo.toml:67-74`), facade `web:153-160`/`mounted:126`/`minimal:115-121`, `document` gate at `dioxus-web/src/lib.rs:21-22` and `hydrate` `inline_js` at `:96,102` all verify. But the "not enabled by any traced chain" exclusion misses an untraced transitive chain: `dioxus-primitives` (git `9a75825`, wasm32 dep of `ui-shared`, all-target dep of `layout`) and `dioxus-sdk-time` depend on `dioxus` **without** `default-features = false`; facade `default = [launch, devtools, logger, lib]` and `lib` includes `document`, which is `dioxus-web?/document` (`dioxus/Cargo.toml:65-70,79-83,98-107`). With test-floor unification (§6's own caveat), `dioxus-web/document` — hence the `eval.js` externs — is therefore likely resolved ON in the shared browser unit, contrary to the passage's thrust. `hydrate` stays excluded (only `fullstack` enables it). The passage's hedges ("manifests traced here", "full resolved-feature proof not performed", reachability unknown) keep it from being false, but the design must not treat `eval.js` as absent. Amendment folded into the tasks: a `cargo tree -e features` resolved-feature read in the smoke phase settles compilation-level presence, and the net-log (already in §7) settles actual requests per launch; the experiment finding records the correction. The core §6 hypothesis (snippet fan-out only where the interpreter externs are reachable) is unaffected.
2. **§7 H2 — HOLDS.** Eager endpoint verified (`navigation_tracker.cc:281` `Page.domContentEventFired`, `:416-417` `readyState "interactive"`), so a `kTimeout` under eager indeed means `DOMContentLoaded` never fired, not a `load`-only stall. Expiry path verified (`web_view_impl.cc:1678-1698`: `VLOG(0) "Timed out. Stopping navigation..."`, `Page.stopLoading`, up to `kWaitForNavigationStopSeconds = 10` at `:63`), as are the runner's `timeout_global` (`headless.rs:852-853`), HTTP-status-only check (`:818-820`), body `debug!` (`:824`), and empty-struct `goto` (`:680-686`). H2 as patched (ChromeDriver log is the settling record; expiry-of-a-DCL-blocking-stall reading; poll-phase consequence) is consistent with §§3/5.
3. **§7 net-log design — HOLDS, with completions.** Shared config verified (`check.py:1231-1237`, `run-wasm-tests.py:512-518`, `check_jobs` `check.py:279-284`); the runner does mutate `goog:chromeOptions.args` at `headless.rs:607-618`, and ChromeDriver's `ParseSwitches`/`SetUnparsedSwitch` (`capabilities.cc:748-762,1240-1263`) accepts `log-net-log=<path>` in that bare style. Completions the finding left unspecified, now in the tasks: (a) ChromeDriver verbose logging needs its own per-launch `--verbose --log-path=` at the spawn site — the shared `CHROMEDRIVER_ARGS` env (`headless.rs:365-371`) has the same collision the patch fixed for net-logs; (b) per-batch log directories, since pids are reused across batches; (c) net-log tail-truncation tolerance when a browser is killed.
4. **§7/§8 approval wording — HOLDS.** Both now read "up to three … stopping at the first conclusive one", matching the stopping condition.

**Hosted vs local contention — pick: hosted batches (local = smoke only).** The phenomenon was measured hosted (9/14 slow in both attempts, so one hosted batch almost certainly yields a slow launch); local reproduction is weak — this machine has 16 CPUs and faster cores (`check_jobs` default 6, not 4), `taskset -c 0-3` plus `BOARDSTUDIO_CHECK_JOBS=4` would mimic the CPU count but not the slower vCPUs or noisy-neighbor scheduling the §4 interleaving needs, and the shared multi-agent checkout adds uncontrolled load both ways. A local-only run therefore risks spending all three batches on "contention not reproduced". What the checkpoint release must explicitly cover for the hosted route: pushing branch `experiment/launch-wait-instrumented`, opening a draft PR from it, and up to three `web`-job runs (the PR's first run plus up to two `gh run rerun --job web`), with logs retained 7 days as the `launch-wait-logs` artifact. No other hosted trigger is used; `workflow_dispatch` is forbidden (it would run the release jobs).

**Tri-log alignment method.** Normalize every log to Unix epoch ms: runner/server lines carry `wall_ms` directly (with `mono_ms` for exact intra-process durations); ChromeDriver verbose lines carry an epoch-seconds prefix; net-log event times are ticks converted with the file's `constants.timeTickOffset`. Cross-check (and if needed correct a constant offset) with two anchor pairs that must coincide within ~250 ms: runner `navigate_sent` ↔ ChromeDriver Navigate receipt, and net-log first request to the runner port ↔ server first `req_received`. The verdict only reads orderings and gaps ≫ that tolerance (a ≥240 s stall).

**Cost per batch (estimate).** Hosted `web` job = A2's 2146 s `build browser` step plus setup/cache/tool steps and the instrumented release build (~2–5 min on 4 vCPU): **~40–50 min wall per batch**. Batch 1 via draft PR also runs the `native` job in parallel (doesn't gate `web`); batches 2–3 are `web`-only reruns. Worst case all three batches ≈ 2.5 h of `web`-job wall (plus one `native` run). Local smoke ≈ one `browser --only host ui-shared` selection under the build lock, a few minutes plus the instrumented build.

## Implementation Notes

Planning run (this dispatch): ticket resolved as entry `id 2` / ref `1.2` (`tickets.py find 1.2`); its plan file did not exist; `after: [1]` continuity read from spike 1.1's plan. Step-01's clean-tree check is met under the recorded exception: tree dirty only with the three protected untracked files, never opened. Follow-up review verdicts recorded in Design Notes (1 amended, 3 hold). Per the dispatch's HARD STOP at `plan_checkpoint`, execution halts at Task 0: no instrumented runner was built, no browser/test/batch was run, nothing was pushed. Batch bookkeeping table is appended here during execution, one row per batch, hard cap three.

## Plan Change Log

## Review Triage Log

## Verification

**Commands:** (execution phase, after checkpoint release)

- `python3 scripts/build-instrumented-wasm-bindgen.py --output <dir>` — expected: pair built, both `--version` = `0.2.129`; script aborts on any non-logging hunk anchor mismatch.
- `diff -r` of instrumented vs stock crate sources (re-extract the checksum-verified `.crate` archives to a temp dir to compare) — expected: logging-only hunks, queue condition unchanged.
- Smoke and batch commands as in Tasks — expected: tri-log files per launch; acceptance criteria above.
- `cargo tree --locked --offline -e features -i dioxus-web --target wasm32-unknown-unknown` — expected: a definitive `document` on/off answer recorded in `.evidence/launch-wait-experiment/resolved-features.log`.

**Manual checks (if no CLI):**

- One slow launch's aligned tri-log read against the finding §7 H1–H4 table, anchors quoted in `finding-launch-wait-experiment.md`.

## Auto Run Result

- Status: `ready-for-dev` — HALTED after planning at the ticket's `plan_checkpoint` per dispatch. Deliverables of this run: (a) the independent follow-up review of the patched §6–§7 passages of `finding-launch-wait.md` (verdicts with anchors in Design Notes: §6 amended, §7 H2 holds, §7 net-log holds with completions, §7/§8 approval wording holds), and (b) this plan, verified against the READY FOR DEVELOPMENT standard (full route).
- Not done, awaiting checkpoint release: Tasks 1+ (instrumented build script, local smoke, experiment branch + draft PR, up to three hosted batches, `finding-launch-wait-experiment.md`). No instrumented runner was built, no browsers/tests/batches were run, nothing was pushed or triggered in hosted CI.
- Files changed this run: this plan file only. The three protected untracked files remain in the shared checkout, never opened.
- Checkpoint decisions needed: hosted (recommended; release must cover pushing `experiment/launch-wait-instrumented`, a draft PR, and up to three `web`-job runs) vs local-only constrained batches; the experiment branch's base commit; whether the §6 `document`-chain correction is patched into `finding-launch-wait.md` now or recorded only in the experiment finding (plan assumes the latter).
