# Finding: patched-pair A/B (spike 1.3)

Ticket: 1.3, epic-browser-launch-latency. Trial only — the patched pair was
selected on a throwaway branch; nothing here adopts it.
Branch `experiment/patched-pair-ab` at base
`fba9ddac15c9b16f978c253ff7f5ae3e2bbb1dcb`; draft PR
[#21](https://github.com/runinit/boardstudio/pull/21) targeting
`audit/layout-engine-pcb-layout`; experiment commit
`da590d550a2a1189c76c30c80d49a5a76e6492a4` (workflow), plan commit
`7a99d1995a415f8105652d482e03413f0d22e8bc`.
Runs: [44119032280](https://github.com/runinit/boardstudio/actions/runs/44119032280)
attempt 1 (web job 134164964892) = run 1; its web-only rerun, attempt 2
(web job 134180142688) = run 2. Baselines: A1/A2 = run 44018165934 attempts
1/2 (measurements digest); B1/B2 = run 44100203460 attempts 1/2 (entry 1.2's
logging-only instrumented pair, `tiny_http` unpatched), extracted from the
web-job logs for this finding (jobs 134097855020 / 134142672874).

Claim labels: **measured** (read off the named streamed job log),
**hypothesis** (fits, not shown), **unknown**.

Method (same as the measurements digest): per launch, overhead =
`check.py`'s printed `(N s)` wall − the runner's `finished in` test time.
Walls were cross-checked against the `check: start`/`check: end` UTC
timestamps (equal in every parsed launch). A launch is "slow" at
≥240 s overhead. The parser reproduces B1's previously recorded 6/14,
which validates it against an independent count (**measured**).

## Verdict

**PASS.** Both A/B runs have **0 of 14** launches ≥240 s overhead (maxima
89.5 s and 96.7 s), all fourteen launches exit 0, and every launch's test
count is unchanged (897 passed in total, identical per launch to the
baselines). Against baselines that carried 9, 9, 6 and 5 slow launches,
the slow group is absent in both patched runs. The page runner did not
complete in either A/B run — the runner received a shutdown signal ~42 s
and ~48 s in, the same death as A1/B1/B2 — so "263 page tests" is
unverified under the patched pair; per the ticket's protocol that clause
applies only if it completes, and the crate-batch read is unaffected
because every launch block had already streamed.

## Per-launch results, A/B runs (measured)

Wall and test time in seconds; overhead = wall − tests.

| Launch | Tests | Run 1 wall | Run 1 tests | Run 1 overhead | Run 2 wall | Run 2 tests | Run 2 overhead |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pcb | 66 | 51.0 | 14.39 | 36.6 | 52.3 | 19.13 | 33.2 |
| case | 100 | 103.0 | 21.72 | 81.3 | 118.9 | 26.40 | 92.5 |
| parts | 108 | 123.0 | 33.55 | 89.5 | 130.0 | 41.84 | 88.2 |
| layout, skip `setup_guide::` | 281 | 154.2 | 68.73 | 85.5 | 172.8 | 76.13 | 96.7 |
| layout, `setup_guide::` | 2 | 21.8 | 0.84 | 21.0 | 25.7 | 0.93 | 24.8 |
| ui-shared, skip `panels::` | 94 | 27.1 | 8.60 | 18.5 | 31.5 | 11.31 | 20.2 |
| ui-shared, `panels::` | 2 | 23.3 | 0.57 | 22.7 | 23.0 | 0.60 | 22.4 |
| library | 22 | 23.6 | 5.49 | 18.1 | 27.9 | 8.08 | 19.8 |
| keymap | 48 | 23.6 | 7.21 | 16.4 | 27.2 | 8.13 | 19.1 |
| keycaps | 34 | 20.9 | 3.76 | 17.1 | 25.9 | 5.32 | 20.6 |
| host | 50 | 24.3 | 13.39 | 10.9 | 27.1 | 15.29 | 11.8 |
| runtime | 69 | 21.4 | 5.68 | 15.7 | 25.3 | 7.13 | 18.2 |
| ui-model | 9 | 13.5 | 0.07 | 13.4 | 15.3 | 0.07 | 15.2 |
| catalogue | 12 | 13.1 | 0.48 | 12.6 | 15.9 | 0.71 | 15.2 |

- Slow (≥240 s): **run 1: 0/14; run 2: 0/14.**
- Crate-batch span (first `check: start` → last `check: end`):
  run 1 **167.2 s**, run 2 **187.3 s**.
- The patched pair was actually selected in both runs:
  `wasm-bindgen pair: /home/runner/work/_temp/wasm-bindgen-patched
  (version 0.2.129)` appears twice per log (crate batch and page runner),
  and the build step printed `built patched pair: …`. Building it cost
  77 s (run 1) and 95 s (run 2) of job time.
- Three launches per run sit at 81–97 s overhead (`case`, `parts`,
  layout-skip) — above the baselines' fast band (≤62 s) but 2.5× below the
  slow threshold and with no 240–330 s cluster. **Measured** as values;
  why these three run warm under 4-way concurrency is **unknown** and not
  needed for the verdict.

## Baselines (measured; A from the digest, B extracted here)

Overhead seconds per launch; bold = slow (≥240 s).

| Launch | A1 | A2 | B1 | B2 |
| --- | ---: | ---: | ---: | ---: |
| pcb | **315.7** | **307.6** | 33.0 | **300.6** |
| case | **315.5** | **307.9** | **295.8** | **331.4** |
| parts | 47.3 | **309.9** | **319.6** | 58.2 |
| layout, skip `setup_guide::` | **321.7** | **313.6** | 42.5 | 62.3 |
| layout, `setup_guide::` | **299.1** | **304.7** | **292.6** | 20.2 |
| ui-shared, skip `panels::` | **303.7** | **293.0** | 9.7 | **285.5** |
| ui-shared, `panels::` | **302.3** | **291.5** | 9.3 | **285.3** |
| library | **296.2** | **285.3** | **278.6** | 11.7 |
| keymap | **294.3** | **285.0** | **280.3** | 12.1 |
| keycaps | **296.3** | 18.4 | **278.1** | **280.7** |
| host | 21.3 | 20.8 | 7.4 | 11.6 |
| runtime | 23.3 | 20.2 | 8.1 | 13.6 |
| ui-model | 22.7 | 15.7 | 6.9 | 11.5 |
| catalogue | 18.4 | 10.8 | 7.7 | 8.8 |
| **Slow count** | **9/14** | **9/14** | **6/14** | **5/14** |
| Crate-batch span | 973.1 s | 916.9 s | 626 s | 596 s |

B2's count (5/14) is new with this finding; B1's extraction matches the
6/14 recorded when entry 1.2 paused. Every baseline batch carried a
278–331 s overhead cluster; neither patched batch has any launch above
96.7 s, and the batch span fell from 596–973 s to 167–187 s (**measured**).

## Page runner (measured)

| Run | `--list` wall | `run-wasm-tests.py --all` outcome |
| --- | ---: | --- |
| A1 | 44.7 s | runner shutdown signal; killed after 106.7 s; no counts |
| A2 | 33.1 s | completed 523.7 s; `executed 263, failed 0` |
| B1 | 25.6 s | shutdown signal ~49 s after start; no counts |
| B2 | 38.1 s | shutdown signal ~50 s after start; no counts |
| Run 1 (patched) | 31.7 s | shutdown signal ~42 s after start (16:08:35→16:09:17Z, exit 143); no counts |
| Run 2 (patched) | 37.1 s | shutdown signal ~48 s after start (16:33:47→16:34:35Z, exit 143); no counts |

The page-runner death occurs with and without the patched pair (3 of 4
baseline attempts, both A/B attempts), always shortly after the page
runner starts and before any `[n/3]` filter line. It is the already-known
lost-runner event, **not** something the patch introduced (**measured**
co-occurrence; the death's cause remains **unknown**, see sampler below).

## Sampler evidence (measured values; interpretation labelled)

The sampler (UTC, `free -m`, loadavg every 5 s into the streamed log;
195 lines in run 1, 225 in run 2) gives the first resource data around the
death. Available memory around page-runner start (15,983/15,987 MiB total):

Run 1 (page runner started 16:08:35Z, shutdown 16:09:17Z):

| UTC | mem-available | load 1m |
| --- | ---: | ---: |
| 16:08:43 | 11,016 MiB | 6.45 |
| 16:08:48 | 1,547 MiB | 6.97 |
| 16:08:54 | 314 MiB | 6.73 |
| 16:08:59–16:09:14 | 184–271 MiB | 6.52–8.66 |

Run 2 (started 16:33:47Z, shutdown 16:34:35Z):

| UTC | mem-available | load 1m |
| --- | ---: | ---: |
| 16:33:56 | 13,116 MiB | 6.72 |
| 16:34:01 | 6,251 MiB | 6.50 |
| 16:34:06 | 416 MiB | 7.33 |
| 16:34:11–16:34:31 | 169–242 MiB | 6.98–8.63 |

- In both runs, ~11–13 GiB of available memory disappears within ~10 s of
  the page runner's selection lines printing, stays at 169–416 MiB for
  20–35 s, and then the runner receives its shutdown signal. **Measured.**
- Memory also dipped during the crate batch (minima 446 MiB run 1,
  506 MiB run 2) but recovered to >10 GiB before the page runner; load
  peaked at 23.2 / 23.6 during the build phase without incident.
  **Measured.**
- That the death is memory-exhaustion-related (runner agent starved or
  OOM-adjacent pressure when the page test binary's Chrome starts) is a
  **hypothesis** the sampler now supports but does not prove: the sampler
  cannot name the consuming process, and no OOM-kill line is in the job
  log. Identifying the consumer needs per-process sampling, which this
  ticket did not run.

## What this does and does not prove

Does prove (**measured**, intervention at batch level): selecting the
existing patched pair for the unchanged hosted batch removes the
278–331 s overhead group in two consecutive representative runs
(0/14 + 0/14 vs 9/9/6/5 of 14 across four unpatched batches), with all
launch counts and pass criteria unchanged. Per-launch independence within
a batch is not claimed; the batch-level contrast (slow group present in
4/4 unpatched batches, absent in 2/2 patched) is the evidence.

Strengthens but does not demonstrate (**hypothesis**): finding 1.1 §4's
mechanism — the `tiny_http` TaskPool lost-queue stranding as the hosted
trigger. The patch changes exactly that one spawn condition, so the
intervention result is what §4 predicts; but no run here observed a
stranded request, and which request stalls / what releases it unpatched
(§5) remains **unknown**. Entry 1.2's tri-log experiment is still the
only designed path to request-level proof; its ticket note says resume
only if this A/B failed, which it did not.

Does not prove: that the page runner's 263 tests pass under the patched
pair (it never got to run them — the death precedes test execution and
also occurs unpatched); why three launches run at 81–97 s overhead; what
consumes the memory at page-runner start; anything about local runs.

## Recommended next step (user decision)

1. **Accept or reject this finding** (this ticket's `done_checkpoint`).
2. On acceptance, slice the deferred R2 fix entry: adopt the patched pair
   in the `web` job permanently, deciding there how the pair is produced
   (per-job build measured at 77–95 s here, vs a cache keyed on the
   script + version pins). That entry's acceptance still needs the 263
   page tests passing in a representative run, which neither A/B run
   could show.
3. Decide the lost-runner scope question now that sampler evidence
   exists: the page-runner memory collapse killed 5 of the last 6 hosted
   web attempts regardless of runner patch. It needs its own scoped
   spike/fix (per-process memory attribution at page-runner start) or an
   explicit decision to leave it — it will otherwise keep failing the
   `web` job even with launch latency fixed.
4. Entry 1.2 stays paused; per its recorded resume condition (resume only
   if the A/B fails), the user may close it rather than resume it.
