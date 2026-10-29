# 15: Compile-demand tuning caps

Status: claimed
Type: task
Parent: [CI wall time map](../map.md)
Related: [05: Provider-build parallelism](05-provider-parallelism.md)

> **Suspended 2026-10-29:** No further tuning matrix or promotion under the current local-first
> [direction](../map.md#near-term-focus--2026-10-29). The claim and progress below are preserved;
> the recorded delegate's current activity is unknown. Check with the owner before changing the
> claim or treating the work as stopped or complete.

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

- Claimed for implementation delegate (moved to Opus after two provider 403s on the default key)
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3Ae2d2f3ae-4b70-478b-a0b3-5c1ffa2041ac`
  (jobs × lanes matrix + warm check; branch `research/tuning-caps`, no PR).
- Progress 2026-10-29: 10-cell cold `--fast` matrix done but noisy — re-running provider-only,
  two interleaved rounds in opposite orders (~25 min). Interim: overlap beats serial at every
  jobs level (12: 272.3/135.8 vs 294.2/155.3; 8: 286.6/139.8 vs 300.5/158.0; 6: 287.4/146.5 vs
  312.4/161.7; 4: 281.6/140.1 vs 338.6/191.8). Lowering jobs has NOT helped — 12-job default
  holds the best provider time (core-worker wants the full core count). Load interfered
  (load 2 → 6–10, swap activity; 12-job repeat came in 101s slower; dx varied 136–161s —
  treat ±15s as noise). Not done: shipped cold, warm runs, test-build-web.py, commit, push.
- Progress 2026-10-29: 10-cell cold `--fast` matrix running (~40 min): jobs 12/8/6/4 ×
  overlap/serial, plus a CAD-lane-capped cell and a 12-job baseline repeat for noise. First
  cell (12-job overlap): 272.3s end-to-end — provider phase 135.8s matches 05's 133.6s, so
  05's figure was provider-phase only; the other ~136s is the cold `dx` page build. Lane A
  (4× wasm-pack) is the critical path at 135.7s, 99s of it core-worker; lane B overlaps only
  its first 46s. Knobs temporary until the winner is picked.
