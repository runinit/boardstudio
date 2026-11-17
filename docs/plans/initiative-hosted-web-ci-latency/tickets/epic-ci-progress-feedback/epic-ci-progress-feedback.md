---
type: epic
title: "Web CI progress feedback and diagnostic retention"
parent: initiative-hosted-web-ci-latency
covers: [R3, R4]
sources:
  - docs/plans/research-web-ci-latency/research-web-ci-latency.md
companions: []
after: []
refined: false
risk: medium
---

# Web CI progress feedback and diagnostic retention

## Description

Three buffering layers hide what the hosted `web` job is doing: build-lane output prints only after every lane ends (415.9 s of silence in the passing attempt), each crate launch's output prints whole at its end, and the page runner reports filters in submission order (454.2 s before its first report). The per-filter durations that would explain the page runner are recorded in `browser.json` and never uploaded. When this epic is done, each segment reports useful progress while it runs and those durations survive the run. Source: the research under References.

## Outcome

Anyone watching the hosted `web` job can tell what is running, how long it has been running, and afterwards how long each page filter took — without waiting for the job to finish to learn any of it.

## Done when

- A hosted run's log shows progress in the build, crate batch, and page runner segments within the interval set at this epic's refinement, delivered through the normal CI path.
- A command's failure output still prints whole and uninterleaved.
- `browser.json` is uploaded as an artifact on a passing and on a failing hosted run.
- The full project check passes on the delivered branch.

## Boundaries

- Out: removing the ~300 s launch wait itself — epic-browser-launch-latency owns the cause and any fix; this epic makes waits visible, not shorter.
- Out: cold-build and cache optimization, and the lost-runner event.
- Must not change: which commands run, their order where tests pin it, or any pass criteria.

## References

- parent — docs/plans/initiative-hosted-web-ci-latency/initiative-hosted-web-ci-latency.md, section Requirements
- research — docs/plans/research-web-ci-latency/research-web-ci-latency.md, section 3, and section 5 for the test files whose bodies refinement must read

## Notes

- Open question: the progress interval and mechanism (heartbeat lines vs streaming captured output), and whether page filters report on completion or on start; settled at refinement after reading the relevant test bodies in `scripts/test-check.py`, `scripts/test-run-wasm-tests.py`, and `scripts/test-build-web.py` — the research read their names and one body only. Sofia's pick: a heartbeat at most every ~60 s.
