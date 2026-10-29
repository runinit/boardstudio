# 03: Warm-baseline measurement

Status: ready-for-agent
Type: task
Blocked by: [01: Cache-branch readiness](01-cache-readiness.md)
Parent: [CI wall time map](../map.md)

## Question

Land the adopted cache fix on a probe branch from `d650e1ddf`, run full CI, and record the warm
receipt that every later speedup compares against.

## Pointers

- Proceed on 01's verdict: extract the five cache commits onto the probe branch.
- Probe branch: `d650e1ddf` plus the cache fix only — never the `stack/02-split-board` branch
  itself. Pushing the branch alone triggers no CI (pushes fire only for `main`/`dev`), so
  warming it needs a draft PR from the probe branch against `dev` (user approval —
  outward-facing action) followed by a re-run on the same head. Expect cherry-pick conflicts
  in `check.yaml`/`check.py`; resolve in favor of the probe head's tree plus the cache hunks.
- Compare cold versus warm ON THE PROBE HEAD (first PR run cold, re-run warm), not against
  `d650e1ddf` — same-head rule.
- Receipt: web and native walls, cache hit/miss plus bytes plus restore/save times per job,
  executed test counts (107 presentation tests plus all crate launches), green full gate, quota
  inventory.
