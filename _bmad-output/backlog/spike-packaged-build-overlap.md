---
id: 2
type: spike
title: "Does overlapping one producer shorten packaged validation?"
covers: []
after: []
refined: true
hitl: false
risk: low
---

# Does overlapping one producer shorten packaged validation?

## Description

The build maintainer needs evidence before changing the local packaged schedule. Cargo already schedules work across cores. A second command may use idle capacity, compete for CPU or wait on the same target lock. The cache story has first priority. This experiment can run independently on the agreed local dev baseline.

## Approach

1. On the local dev baseline `a6cfa5183`, identify when page compilation, bundling and browser checks first read each generated asset. Choose one producer that can finish before its first consumer. If none qualifies, record the dependency and close the spike.
2. Compare the existing sequence with that single overlap on the same machine, revision, toolchain, core budget, starting cache state and target layout. Respect the repository's single-build-per-target guard. Include any extra cold compilation caused by separate targets. Record baseline `a6cfa5183` for both arms and the scheduling-only intervention in the overlap arm. Keep tool setup identical.
3. Capture monotonic start and finish times through the last required packaged check, CPU utilization over time, and Cargo lock waits. Compare elapsed wall time. Preserve the exact names and outcomes of browser tests and gate receipts in both runs.
4. Run one matched sequential/overlap pair. If saturation, lock waits, a coverage difference or negligible gain rules out adoption, stop. To recommend adoption after a promising first pair, run one matched overlap/sequential pair. Stop after that pair.

## Acceptance Criteria

1. The spike's result names the producer and its first consumer, the tested conditions, and the critical-path wall time for each schedule, or records why no safe overlap exists.
2. Report all four elapsed times if recommending overlap. Both pairs must favor overlap, and the smaller paired saving must exceed the larger difference between repeat runs of the same schedule. Otherwise retain sequential execution. Treat this as a local decision rule. Both schedules must pass equivalent dev checks, including staged renderer assets, drag checks, the model-lookup gate and exact page test inventory. Preserve existing browser isolation and release behavior.
3. Record the decision and its evidence in the spike's plan, with paths to the local logs. Recommend a separate implementation ticket only if the overlap meets these criteria.

## References

- Dev gate inventory — `.evidence/build-audit/commands.log`, browser-packaged.
- Cargo guidance — https://doc.rust-lang.org/cargo/reference/timings.html, reading the graphs.
