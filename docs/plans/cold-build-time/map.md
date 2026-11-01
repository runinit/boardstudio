# Map: Cold build time, follow-up

## Destination

Cut what a cold build really costs, on the two places cold builds happen: a new worktree on this
machine and a hosted CI run. Every kept change has a measured gain, runs the same tests and keeps
every gate. Nothing here relaxes a test, a count guard or a workflow contract.

Reference point (head `510861b11` plus the uncommitted round one, single run, shared 16-core
machine, 12 cargo jobs): `python3 scripts/check.py` cold took **1273.8 s wall and 3069 CPU-s**.

## Findings that changed the plan

Each is derived from `.evidence/build-time/final.jsonl` (gitignored) and the code at `510861b11`.

1. **CPU and wall are different problems here.** Registry crates take 1694 CPU-s (55%) but only
   303 s of attributed wall, because they compile in parallel early. Workspace and vendored crates
   take 1375 CPU-s (45%) and 607 s of attributed wall, because they form a serial chain. Attributed
   wall splits each moment of rustc activity equally among the running units; it totals the 909 s
   of compile-active time. On a 16-core machine local wall follows the chain. On a 4 vCPU runner
   wall follows CPU, so cite both.
2. **Most of the step wall is sequencing, not compiling.** `build` runs its lanes one after another
   at 1.4 to 4.7x parallelism. `test` waits on the CAD lane, which runs three independent target
   dirs in series (265 s against 153 s for the workspace lane). Three tickets ([04](issues/04-cad-test-lanes.md), [05](issues/05-build-step-lanes.md),
   [06](issues/06-harness-and-tooling-overlap.md)) recover about 250 to 335 s of wall at no CPU cost, more than items 3, 5, 6 and 7 of the brief together.
3. **A real cold build mostly means a new worktree.** `git worktree list` shows 103 worktrees; a
   two-level scan of four worktree roots finds 36 real target dirs, and the ones under
   `~/.t3/worktrees/ts-boardstudio2` total 252 G (`du`, reflinks not deduplicated) against 234 G
   free. All of it is on one btrfs filesystem, so a reflink copy is free and instant, which makes
   [03](issues/03-seed-worktree-targets.md) a candidate to beat `sccache` on effort.
4. **Core is not the place to cut.** 71,300 of its 77,919 production lines form one dependency
   cycle. See [13](issues/13-split-core.md).
5. **The regex cost is mostly not Dioxus.** 130 of the 170 CPU-s is in the CAD trees, reached only
   through `step-p21 -> Inflector`, whose default feature pulls regex that `step-p21` never uses.
   See [07](issues/07-inflector-regex.md).
6. **A `ci-wall-time` plan already exists off-tree** on branch `ci-wall-time-plan`
   (`docs/plans/ci-wall-time/map.md` there, 30 files). It holds the hosted baselines, the selected
   web-split trust design, `research/local-web-builds@0f7c63087` (provider lanes) and the user's
   approximately 10-minute cold target. [10](issues/10-ci-measure-and-fix.md) continues it rather
   than forking it. Its data also corrects the brief: warm, `web` is the critical path, not
   `native`.

## Notes

- **Vocabulary.** *Lane*: an independent chain of commands that can overlap another. *Attributed
  wall*: as in finding 1; a planning estimate, not a measurement. *Cold run*: one
  `check.py` run after wiping this worktree's outputs, with registry and git caches kept. *Cold
  wall*, *CPU*, *warm*: the three gain columns every ticket states separately.
- **Measurement protocol.** Same head, nothing else building (load average under 3 at the start),
  the kit from [02](issues/02-measurement-kit.md). One cold run per arm is enough only when the
  delta is at least 60 s, since earlier matrices in `ci-wall-time` ticket 15 varied by about 15 s
  and had a 101 s outlier under load. Otherwise take two runs per arm and use the median.
- **Stop rule discipline.** Round one reverted two cuts on data. Every ticket states its revert
  threshold, and missing it means revert, not retune.
- **Count floor at this head.** `check: native tests: all 2378 baselined test(s) executed` (27 new,
  not yet recorded), 693 crate launch tests, 112 page tests, 15 drag checks, 364 CAD browser
  checks. A run below any of these is a failure, whatever the time.
- **Contracts.** `scripts/test-ci-workflow.py` and `assert_gate_contract` in `scripts/test-check.py`
  stay as strict as they are. `assert_gate_contract` accepts a `native` job holding only `runs-on`
  and `steps`, and a check step holding only `name`, `id` and `run`, so a native experiment cannot
  add `env:` there.
- **Guardrails.** [AGENTS.md](../../../AGENTS.md) applies: `jobs = 12` stays, no
  `CARGO_INCREMENTAL=0`, no `CARGO_BUILD_JOBS=2`, overlap only through supported scripts. CAD
  review follows the [closure rule](../cad-backend-replacement/map.md#closure-and-workflow-decisions-2026-10-18-user):
  dependency and test-only slices get RED/GREEN evidence and a diff read.
- **Not verified in this plan.** No build or CI run was started. GitHub job logs could not be read
  (`gh` token invalid, HTTP 403); job and step walls came from the public REST API. Every gain below
  is an estimate from one run's logs until its ticket measures it.

## Ranking

Gains are against the reference run and are not additive: overlapping lanes and seeded registry
crates share the same idle time. Stacked, 03 to 09 might take a new worktree from about 1270 s to
about 700 to 800 s; a cold run in an existing worktree to about 900 to 1000 s.

| # | Ticket | Cold wall | CPU-s | Warm | Effort | Verdict |
|---|---|---:|---:|---|---|---|
| [01](issues/01-land-round-one-and-audit.md) | Land round one, then the audit | none new | none new | none | 1 h | Do first |
| [02](issues/02-measurement-kit.md) | Commit the measurement kit, rebaseline | 0 | 0 | 0 | 1 h + one run | Do |
| [03](issues/03-seed-worktree-targets.md) | Reflink-seed new worktree targets | -250 to -300 (new worktree only) | -1694 | none | 1-2 h | Do, prototype first |
| [04](issues/04-cad-test-lanes.md) | Run CAD test trees as lanes | -90 to -110 | 0 | small | 2 h | Do |
| [05](issues/05-build-step-lanes.md) | Overlap page and provider builds | -100 to -135 | 0 | about 0 | 3 h | Do |
| [06](issues/06-harness-and-tooling-overlap.md) | Overlap harness and tooling compiles | -60 to -90 | 0 | small | 3 h | Do |
| [07](issues/07-inflector-regex.md) | Drop Inflector's regex from CAD | -8 to -11 | -130 | none | 1 h | Do |
| [08](issues/08-browser-launch-overhead.md) | Reduce browser launch overhead | -40 to -70 | 0 | same, every run | 1 day | Measure, then decide |
| [09](issues/09-cad-owner-feature-set.md) | One feature set for CAD owner tests | -5 to -30 | -80 to -100 | CI only | 3 h | Do after 04 |
| [10](issues/10-ci-measure-and-fix.md) | Measure and fix CI | unknown until measured | see ticket | see ticket | 1 day + hosted runs | Needs the user |
| [11](issues/11-core-wasm-gating.md) | Gate wasm-bindgen in Core | -4 | -23 | small | 30 min | Low value, cheap |
| [12](issues/12-sccache.md) | `sccache` across worktrees | -290 (new worktree) | -1566 | none | 2 h + install | Only if 03 fails |
| [13](issues/13-split-core.md) | Split `boardstudio_core` | at most -15 to -30 | about 0 | small | weeks | **Drop** |
| [14](issues/14-dioxus-default-features.md) | Remove Dioxus default features | -3 | -41 | none | 1 day + upkeep | **Drop** |
| [15](issues/15-cad-test-dev-deps.md) | Trim CAD test dependencies | -2 to -5 | -20 to -50 | none | half a day | **Drop** |

## Decisions so far

- Keep `jobs = 12`, stable 1.98 and every gate.
- Do not point worktrees at one shared target; see [12](issues/12-sccache.md#shared-target-rejected).
- Items 3, 5 and 7 of the brief do not pay and are recorded as `wontfix` with reopen conditions.
- 2026-10-30: land round one plus the audit (01) from `perf/reduce-cold-build-times`, local
  commits only; no push or hosted run yet.
- 2026-10-30: CI work (10) stays local-only; the `ci-wall-time` hosted-runs hold stands.
  `gh` re-auth is deferred with it.
- 2026-10-30: CI `CARGO_BUILD_JOBS` experiment deferred to last; structural work (code audit,
  tests) comes first. Try `BOARDSTUDIO_CHECK_JOBS` before any exception.
- 2026-10-30: BSD-2-Clause for Inflector (07) may go on the `deny.toml` allow list if needed.

## Waiting on the user

- [01](issues/01-land-round-one-and-audit.md): commit and land round one and the audit.
- [10](issues/10-ci-measure-and-fix.md): re-authenticate `gh`, approve hosted runs, and settle
  whether `ci-wall-time`'s local-first hold still stands.
- [12](issues/12-sccache.md): machine-wide `sccache`, only if 03 fails.

## Not re-planned

From the brief, with no new evidence: `dx --profile dev`, `opt-level=1` for wasm providers and for
CAD tests, one target dir across CAD trees, nightly features, faster linkers.

**Sharing page and test trees** has one new number, which does not reopen it yet. The web crates
compile twice inside `wasm32/debug` because `web/Cargo.toml` turns on `test-support` for the page
tests only: 66 CPU-s, but 46 s of attributed wall since they sit on the serial chain. Reopen if
`browser` compile still dominates after 06 and 08.

## Out of scope

- Runner purchase or size, and the `web` split deployment: owned by `ci-wall-time` tickets 04 and 17.
- Test removal or tiering.
- Converting vendored CAD crates from path to a git source so CI stops recompiling them. It would
  remove the mtime rebuilds that `research/native-warm` reports (not confirmed), but needs a fork
  to host them. [10](issues/10-ci-measure-and-fix.md) measures the cost first.
