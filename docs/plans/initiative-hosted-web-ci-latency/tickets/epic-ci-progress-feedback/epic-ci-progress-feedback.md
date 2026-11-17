---
type: epic
title: "Web CI progress feedback and diagnostic retention"
parent: initiative-hosted-web-ci-latency
covers: [R3, R4]
sources:
  - docs/plans/research-web-ci-latency/research-web-ci-latency.md
companions: []
after: []
refined: true
risk: medium
---

# Web CI progress feedback and diagnostic retention

## Description

Three buffering layers hide what the hosted `web` job is doing: build-lane output prints only after every lane ends (415.9 s of silence in the passing attempt), each crate launch's output prints whole at its end, and the page runner reports filters in submission order (454.2 s before its first report). The per-filter durations that would explain the page runner are recorded in `browser.json`, written only at the end of the run, and the existing artifact step uploads them on failure only. When this epic is done, each segment reports useful progress while it runs and those durations survive the run. Source: the research under References.

## Outcome

Anyone watching the hosted `web` job can tell what is running, how long it has been running, and afterwards how long each page filter took — without waiting for the job to finish to learn any of it.

## Requirements

- R1 (parent R3): The build lanes (`scripts/build-web.py`), the crate launch batch (`scripts/check.py`), and the page runner (`scripts/run-wasm-tests.py`) each report still-running work at least every 60 s in one shared format, `<tool>: still running <elapsed>s - <what>`. The page runner also announces each filter's start and prints its already-measured duration on the `[i/n]` report line. Captured output still prints whole and uninterleaved, and page reports stay in submission order.
- R2 (parent R4): `browser.json` is uploaded as a CI artifact on success and on failure, and the page runner rewrites it after each filter so partial per-filter durations survive a killed page run whenever the runner itself survives. (A runner that loses communication with GitHub, as in attempt 1, still uploads nothing — a platform limit, not this epic's to fix.)

## Constraints

- Heartbeat lines only, never streamed captured output: existing tests pin whole, uninterleaved blocks (`test_parallel_heartbeat_tracks_started_and_completed_workers_before_blocked_first`, `test_overlapped_output_prints_whole_in_plan_order`; bodies read at this refinement).
- The interval is a fixed 60 s with no new environment knob; the shared format above is settled here so the three scripts share no progress code.
- Which commands run, their order where tests pin it, and every pass criterion are unchanged; the page runner's submission-order reporting (pinned by `test_parallel_filters_report_in_original_order_even_when_they_finish_out_of_order`) stays.

## Done when

- A hosted run's log shows a still-running line at least every 60 s in each of the build, crate batch, and page runner segments, delivered through the normal CI path.
- A command's failure output still prints whole and uninterleaved, and page filters still report in submission order with their durations.
- `browser.json` is uploaded as an artifact on a passing and on a failing hosted run.
- The full project check passes on the delivered branch.

## Boundaries

- Out: removing the ~300 s launch wait itself — epic-browser-launch-latency owns the cause and any fix; this epic makes waits visible, not shorter.
- Out: cold-build and cache optimization, the lost-runner event, and progress for the serial gate/prebuild segment (its silences were 63–125 s and it is not one of the three segments R1 names).
- Must not change: which commands run, their order where tests pin it, or any pass criteria.

## References

- parent — docs/plans/initiative-hosted-web-ci-latency/initiative-hosted-web-ci-latency.md, section Requirements
- research — docs/plans/research-web-ci-latency/research-web-ci-latency.md, section 3, and section 5 for the test files whose bodies refinement must read

## Notes

- Decision: 2026-11-17 — refined with user-approved picks: heartbeat-only mechanism at a fixed 60 s in the shared format in R1; page runner keeps submission-order reports plus start lines and durations; the existing artifact step (`.github/workflows/check.yaml`, `failure()` today) becomes `if: always()`; incremental `browser.json` writing folds into the page-runner story so the tracer stays a one-line workflow change.
- Decision: 2026-11-17 — tracer bullet is entry 1 (artifact on every run; visible on the run page). Entries 2–4 are parallel lanes in separate files sharing only R1's settled format, so none waits on another; entry 5, the refactor sweep, waits on all four.
- Decision: 2026-11-17 — entries verify their heartbeat locally (test suites plus a long local log); the hosted-run observation in Done when is this epic's closure check, run once against a hosted `web` job on the delivered branch, not repeated per story.
