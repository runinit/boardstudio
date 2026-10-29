# 03: Warm-baseline measurement

Status: claimed
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

## Comments

- Claimed for probe-branch prep delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A8f05dfbe-b82d-498b-9096-38c1c357cfc3`
  (work stops before the draft PR — that needs user approval).
- Probe ready 2026-10-29: `ci/probe-warm-baseline` pushed at `008cb78f` — `d650e1ddf` plus
  exactly the five cache commits (`git log d650e1ddf..HEAD` shows only those five; 4 files
  touched). One conflict in `scripts/check.py` (probe head's `TOOLING_TESTS` lacks the
  stack-product entries): resolved probe-tree plus the cache hunk. Wiring verified:
  `test-ci-workflow.py` exists and is in `TOOLING_TESTS`; save-if scopes, per-PR cancellation,
  `RUSTUP_HOME`, `ImageOS`, qualification target all present. Next: draft PR against `dev`
  (user approval), then re-run for cold→warm on the probe head; the transition run will be
  cold and is not steady state.
