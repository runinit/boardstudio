# 14: Native warm wall and budgets

Status: needs-info
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
