# 03: Warm-baseline measurement

Status: resolved
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

## Answer

Warm baseline scored on draft PR #11 (head `008cb78f`, open draft, never to merge).
Cold (transition, red cache, all green): native 29m22s (rust-cache MISS, saved 1.47 GiB in
~17s), web 42m17s (MISS, saved 1.41 GiB in ~19s; wasm-pack HIT), audit 6m34s.
Warm (attempt 4, same head, GREEN steady state): native **14m41s** (exact-key HIT, 1.47 GiB
in ~5.6s, no re-upload), web **26m03s** (HIT, 1.41 GiB in ~6.9s; build ~6.1 min, browser
~19.2 min), audit 6m28s. Tests identical cold and warm: 2870 native passed / 0 failed /
18 ignored; 364 browser checks, 659 wasm unit, 5/5 drag-smoke. Quota unchanged (~8.56 GiB,
10 caches); warm runs downloaded ~3.0 GiB, uploaded nothing.
Web 26m03s lands inside the predicted 22–27 min (upper end) — 17 is NOT met by caching alone;
web needs ~9 more minutes from split/workload. Attempts 2–3 red on the same head with
timing-sensitive flakes (native `test-serve-web.py` 0.2s localhost timeout; web drag-smoke
translate diff); cold passed both, failed runs saved no caches, warm receipt stands.
Full receipt: `.evidence/warm-baseline-probe/receipt.md` (delegate worktree, gitignored —
key numbers preserved here).
- Probe ready 2026-10-29: `ci/probe-warm-baseline` pushed at `008cb78f` — `d650e1ddf` plus
  exactly the five cache commits (`git log d650e1ddf..HEAD` shows only those five; 4 files
  touched). One conflict in `scripts/check.py` (probe head's `TOOLING_TESTS` lacks the
  stack-product entries): resolved probe-tree plus the cache hunk. Wiring verified:
  `test-ci-workflow.py` exists and is in `TOOLING_TESTS`; save-if scopes, per-PR cancellation,
  `RUSTUP_HOME`, `ImageOS`, qualification target all present. Next: draft PR against `dev`
  (user approval), then re-run for cold→warm on the probe head; the transition run will be
  cold and is not steady state.
- CI delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3Afd270442-2586-442f-af93-6db38e28c3bd`
  (draft-PR approval granted 2026-10-29; open draft, cold run, re-run warm, record receipt;
  never merge).
