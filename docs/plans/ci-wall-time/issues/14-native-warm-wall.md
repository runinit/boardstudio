# 14: Native warm wall and budgets

Status: resolved
Type: research
Blocked by: [03: Warm-baseline measurement](03-warm-baseline-measure.md)
Parent: [CI wall time map](../map.md)

## Question

Once 03 scores a warm native run (never observed — cold is 30m19s with lint 477s plus test
1095s), break it down and set per-step budgets that keep native off the wall.

## Pointers

- If warm native lands near or above 17 minutes, native becomes the wall and needs its own
  speedup tickets (graduate them from here); the destination's 17 minutes covers the PR-gate
  tier of BOTH jobs.
- Turn the current fog (lint split and clippy overlap, test breakdown) into measured numbers.
- Deliverable: warm breakdown plus budgets per step, and either "native fits" or follow-up
  speedup tickets.

## Answer

**Native fits under 17 minutes**: warm 14m41s (881s), 2m19s under the wall; all steps at
budget would take 16m03s. No follow-up ticket required for 17 — F1 (vendored CAD rebuilds,
~210s est.) and F2 (merge 7 qualification calls, 62s) graduate only if CAD growth trips the
budget or native becomes the slowest job. Findings: `docs/research/2026-10-29-native-warm.md`
on branch `research/native-warm` (pushed, no PR).
Warm breakdown (PR #11 attempt 4; cold = attempt 1): setup+checkout 5s, KiCad 26s, rustup 10s,
cache restore 19s, repo 4.3s, tooling 41.0s (32.9s a dev build of core), lint 132.0s (fmt 6.9 /
native clippy 49.5 / WASM clippy 75.7), test 640.4s (workspace 426.0 / footprints 50.5 / CAD
640.4 — CAD sets the wall: compile 526.8s + tests 110s). Budgets: setup 10 / KiCad 45 /
rustup 15 / restore 25 / repo 8 / tooling 50 / lint 150 / test 660 = 963s total (57s headroom;
+~25s on a key change). Test step has ~20s margin; CAD-lane growth lands straight on the wall.
Over-push risks: toolchain/env change (all-PR cold until re-saved), no `dev` cache yet (until
12 lands — every new PR's first run is cold), lockfile churn (31 manifests in the key),
quota (8.56/10 GB; each miss saves ~2.9 GB), CAD test growth. Native is core-touch-proof in
structure (workspace rebuilds run regardless) — inference, not measurement.
