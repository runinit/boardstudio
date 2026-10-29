# 15: Compile-demand tuning caps

Status: claimed
Type: task
Parent: [CI wall time map](../map.md)
Related: [05: Provider-build parallelism](05-provider-parallelism.md)

## Question

What cargo-job and lane-overlap caps minimize the cold build wall without hurting warm builds —
tuning demand down, since 05 proved the box is oversubscribed, not starved?

## Pointers

- 05's finding: overlapped lanes run slower than their serial sums at 2×12 rustc jobs plus
  wasm-opt on 16 cores (lock-wait 0; pure CPU contention). Raising caps is predicted to worsen
  it — measure the downward direction.
- Matrix (local, scratch target, defaults otherwise): cargo jobs {12, 8, 6, 4} × lanes
  {3-lane overlap, serial} on `--fast` cold end-to-end; confirm the winner on shipped cold;
  verify warm stays ~flat. Staggering wasm-opt out of the overlap window is in scope if
  trivially expressible.
- Base: 05's branch `research/provider-parallelism` (`8bf8ebf63`) — check out from there onto
  a new branch `research/tuning-caps`.
- Deliverable: winning config plus walls per cell, committed on the branch (pushed, no PR);
  the winner folds into 05 before its CI receipt.
- Acceptance: green `test-build-web.py`, no new failure modes, numbers not vibes.

## Comments

- Claimed for implementation delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3Abc32678e-0a8d-48cc-88d4-4b0bbd83d61d`
  (jobs × lanes matrix + warm check; branch `research/tuning-caps`, no PR).
