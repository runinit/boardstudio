# 10: Measure and fix CI

Status: ready-for-human
Type: task
Blocked by: [01](01-land-round-one-and-audit.md)
Parent: [map](../map.md)
Related: `ci-wall-time` plan on branch `ci-wall-time-plan`, [research audit](../../../research/2026-10-28-ci-build-time-audit.md)

## Question

What does a PR run record, what does it currently cost, and which change is proven to reduce it?

## Evidence

**A plan already exists.** Branch `ci-wall-time-plan` has `docs/plans/ci-wall-time/` with 26 tickets, a
user-set target of about 10 minutes cold and a selected web-split design. It is not on this head.
Experiments exist at `ci/experimental-web-gate-topology@54e58b336` and `perf/native-ci-topology@f3f6c893a`.
Its map says no hosted runs, PRs or merges until the user returns to GitHub work. Continue it; do not
fork it. This ticket adds only what is new since 2026-10-29.

**Job walls from the public REST API** (not logs; check step in brackets where read):

| Run | Event | native | web | State |
|---|---|---:|---:|---|
| [40791505198](https://github.com/runinit/boardstudio/actions/runs/40791505198) | PR | 30m19s | 33m37s | cold, from `ci-wall-time` baseline |
| [40806171071](https://github.com/runinit/boardstudio/actions/runs/40806171071) | PR, attempt 4 | 14m41s (818 s) | 26m03s (1510 s) | warm |
| [40821892659](https://github.com/runinit/boardstudio/actions/runs/40821892659) | PR 13 | 15m10s (828 s) | 22m01s (1252 s) | 41 s restore, cache state unread |
| [40822500650](https://github.com/runinit/boardstudio/actions/runs/40822500650) | PR 12 | 13m45s (744 s) | 36m56s (2151 s) | cache state unread |
| [40826089013](https://github.com/runinit/boardstudio/actions/runs/40826089013) | push `dev` | 31m50s (1848 s) | 37m36s (2206 s) | new key on `dev`, inferred cold |

So: **warm, `web` is the critical path**, not native. The brief's "native about 32 min" is the cold
figure, and web was as slow or slower. Reducing native alone saves nothing when warm. Web's 1252 s to
2206 s spread on the same step is unexplained. Cache state for two rows needs logs.

**Compile is dominated by what a cache cannot help.** `rust-cache` has `cache-workspace-crates` false
(input at the pinned SHA, default "false"), and `research/native-warm` reports all path crates recompiling
when warm: CAD lane compile only falls from 903 s to 527 s. Cause is not confirmed; it suspects mtime
freshness after a fresh checkout. By the local unit log, path crates are 45% of CPU, so warm CI should
not be expected to fall below about half of cold.

**Others.**

- Cache usage: 10,587,247,763 bytes in 8 entries, at the 10 GB limit, including `refs/pull/*` entries that
  can evict `dev`'s. Each native or web entry is 1.3 to 1.5 GB.
- `jobs = 12` on 4 vCPU, with `BOARDSTUDIO_CHECK_JOBS=4`, allows 48 rustc at once. `ci-wall-time` 15's
  interim local data did not show lower job counts helping (noisy, cancelled, not hosted). No hosted
  measurement exists.
- `security.yaml` compiles cargo-audit and cargo-deny on each run (3m25s + 2m12s). The job takes 6.5 min
  and ends long before `web`, so fixing it saves runner minutes and no PR wall time.
- Round one's step-level `CARGO_PROFILE_DEV_DEBUG=false` on `web` is unmeasured on CI. `ci-wall-time` 13's
  local A/B found 20 to 30% per launch, cold 55 s to 93 s (catalogue), and one timeout on line tables.

**Access gap.** `gh` here has an invalid token. Log download returns HTTP 403, so nothing below can be
done until the user re-authenticates.

## Gain

Not estimated as a total: hosted and local behaviour differ too much to project. What each change can
reach, from the data above:

| Change | Reach | Confidence |
|---|---|---|
| Round one on `web` (already written) | Part of the browser step, 988-1363 s | Measured locally only |
| [05](05-build-step-lanes.md) on the web build step | Part of about 326-624 s | Estimated |
| [07](07-inflector-regex.md), [09](09-cad-owner-feature-set.md) on native | -130 and -100 CPU-s, about -55 s on 4 vCPU; only if native is critical | Estimated |
| Prebuilt audit tools | 0 PR wall; 5m37s of runner time per run | Measured interval |
| `ci-wall-time` 04 web split | Largest known; the deployment owner is that plan | Not measured |

Effort: 1 day of analysis plus hosted runs. Risk: hosted runs and cache writes are outward-facing, and a
new key causes a transition cost.

## What a PR run must record

Same head for before and after, in `.evidence/` and summarized here:

- Run and job URLs, head SHA, attempt, runner image, and vCPU count.
- For each `rust-cache` step: exact key and restore key used, ref scope, hit, partial or miss, compressed
  size, restore time, save time and cache usage after saving.
- `check: step times` line for each job, and each child's start and end from the heartbeat logs (PR 12).
- Critical path: `max(native, web)` including setup and restore; the same for security separately.
- Test counts: native baselined and new, crate launch, page, drag, CAD browser, each equal to the count
  floor or higher.
- The first failure, if any, and whether the job was infra-killed (exit 143 with no failed test).

## Acceptance

A change counts when, on the same head and required check set, it has one cold and one warm hosted pair,
every required check is green, counts are unchanged, and `max(native, web)` falls by at least 60 s in the
state it claims. Cold and warm are reported separately. Local timings never substitute.

## Work

1. User: `gh auth login`, approve the hosted runs, decide the questions below.
2. Baseline without extra cost: read logs of the landing runs from [01](01-land-round-one-and-audit.md), as
   they are the first after round one. Take cold from the push to `dev`, warm from the next PR.
3. Explain web's 1252-2206 s spread from those logs before optimizing anything.
4. Land 04 to 07 locally, then do one hosted pair covering them, rather than one pair per change.
5. Only then run these one at a time, each one key transition:
   - `BOARDSTUDIO_CHECK_JOBS` of 2 and 3, which changes no key.
   - Workflow-level `CARGO_BUILD_JOBS` of 4 and 6. Adding `env:` to `native` or its check step breaks
     `assert_gate_contract` and must not be done. Workflow-level `CARGO_*` changes the `rust-cache` key.
     `AGENTS.md`'s rule is against `CARGO_BUILD_JOBS=2` and overriding `jobs = 12`, so this needs the user's
     approval as a CI-only exception.
   - `cache-workspace-crates: true`, only if the fingerprint trace shows it would stay fresh. Rewriting
     mtimes stays rejected, as `ci-wall-time`'s policy says.
6. Feed the results into `ci-wall-time`'s tickets 04 and 17 instead of building a second topology.

## Verification

```sh
python3 -B scripts/test-ci-workflow.py && python3 -B scripts/test-check.py   # unchanged assertions
gh run view <id> --repo runinit/boardstudio --json jobs > .evidence/ci-<id>.json
gh run view <id> --repo runinit/boardstudio --log > .evidence/ci-<id>.log
grep -E 'Cache (hit|restored|not found)|check: step times|native tests:' .evidence/ci-<id>.log
gh api repos/runinit/boardstudio/actions/cache/usage
```

## Stop rule

Revert any workflow change that misses the acceptance bar, makes a job flakier across two runs, or pushes
`dev`'s cache out through eviction. Stop tuning job counts after the first round if no setting moves
`max(native, web)` by 60 s.
