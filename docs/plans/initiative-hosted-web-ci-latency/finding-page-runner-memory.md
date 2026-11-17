# Finding: hosted page-runner death is wasm-bindgen memory exhaustion

Unticketed investigation requested by the user on 2026-11-17 after spike 1.3
(epic-browser-launch-latency). This file is evidence for a future ticket, not an
adopted fix. Throwaway branch `investigate/page-runner-death`, built on
`experiment/patched-pair-ab` head `085b5b1b7` (patched pair still selected,
crate batch unchanged). Draft PR [#22](https://github.com/runinit/boardstudio/pull/22),
experiment commit `487942039`. Run [44132437661](https://github.com/runinit/boardstudio/actions/runs/44132437661).

Claim labels: **measured**, **hypothesis**, **unknown** (as in finding-patched-pair-ab.md).

## Cause

`python3 scripts/run-wasm-tests.py --all` submits its three page filters to a
3-worker `ThreadPoolExecutor` (`scripts/run-wasm-tests.py:540-562`). Each filter
is a separate `wasm-pack test … --bin boardstudio-web` whose
`wasm-bindgen-test-runner` runs a full wasm-bindgen pass over the same page-test
binary (~124 MB: ~50 MB code, ~70 MB `name` section, no DWARF). One such pass
peaks at **6.0–6.9 GiB RSS on the hosted runner** (**measured**, `sampler-rss`
lines) and 4.6–4.7 GiB locally (**measured**, `wasm-bindgen --target web` and the
runner itself on a local build of the same binary). Three at once need ~18–20 GiB
on a 15.7–16 GiB runner; available memory collapses to ~0.2 GiB and the runner
agent receives its shutdown signal (exit 143) before any filter completes —
the 5-of-6 failure pattern of A1, B1, B2 and both 1.3 runs.

- Spike 1.3's sampler shows the collapse with only ~295 scheduling entities
  (vs 460–970 when Chrome runs), so no browser had started: the consumer is the
  bindgen phase, not Chrome (**measured** task counts; consistent with the RSS
  attribution below).
- A2 survived the same parallel launch (523.7 s, 263 passed) — **hypothesis**:
  swap absorbed the overshoot that time; its wall is ~4× the serial wall below.

## Intervention (serial page filters)

Experiment change only: `BOARDSTUDIO_WASM_PAGE_JOBS=1` caps the page-filter pool
at one worker (crate batch keeps 4); the sampler adds the top-6 resident
processes every 5 s.

| Run | Web job | Crate batch | Page runner | Peak wasm-bindgen-test-runner RSS | Min mem-available (page phase) | Job |
| --- | --- | --- | --- | ---: | ---: | --- |
| 1 (attempt 1) | 134212849041 | 14/14 ok, 897 tests, 0 slow, span 112 s | **263 passed, 0 failed, 124.9 s** | 6,871 MiB | ~6.7 GiB | success, 16.5 min |
| 2 (attempt 3) | 134238464684 | 14/14 ok, 897 tests, 0 slow, span 180 s | **263 passed, 0 failed, 172.9 s** | 7,040 MiB | 7.1 GiB | success, 19.2 min |

Attempt 2 (web job 134227836619) failed earlier, in
`cad/scripts/test-cad-browser.py`, before the crate batch and page runner ran:
ChromeDriver `session not created: DevToolsActivePort file doesn't exist`
after 60 s, with 14.9 GiB available (**measured**). That script picks its
browser from `CHROME`/`PATH` (`cad/scripts/test-cad-browser.py:23-27`), the
workflow sets only `CHROME_BIN`, so CI drives the image's `/usr/bin/chromium`
with the pinned Chrome-for-Testing chromedriver. Attempt 3 passed the same
script on the same image build (20261111.196.1), so it is an intermittent,
separate defect (unpinned browser), not memory and not this experiment's change.

Page-runner walls for comparison: A2 (parallel, the only prior completion)
523.7 s; run 1 serial 124.9 s (**measured**). Serial is faster, not slower —
**hypothesis**: the parallel run thrashed swap.

## Second risk: the crate batch is near the same edge

With four concurrent crate launches, run 1's crate batch reached
mem-available 321 MiB and 3.0 GiB swap used (17:13:50–17:14:30Z) with three
`wasm-bindgen-test-runner` processes at 4.5/3.4/2.1 GiB (**measured**). It
survived; 1.3's runs dipped to 446/506 MiB. Any growth of the crate test
binaries can tip it over. A fix should bound concurrent wasm-bindgen by memory,
not only for the page runner.

## What this does not show

Two successful attempts of one workflow run (same caveat as 1.3). Why hosted RSS (6–7 GiB)
exceeds local (4.7 GiB) is **unknown** (binary drift since the local build or
allocator/page-size effects). Running bindgen once and reusing the output across
filters was not tried.
