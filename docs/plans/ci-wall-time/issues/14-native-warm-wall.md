# 14: Native warm wall and budgets

Status: claimed
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

## Comments

- Claimed for read-only research delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A39d5dcc9-9550-4e99-9739-0a0ff441b55f`
  (findings go to `docs/research/2026-10-29-native-warm.md` on branch `research/native-warm`,
  pushed, no PR).
