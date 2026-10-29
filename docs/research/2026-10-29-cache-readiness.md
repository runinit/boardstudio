# Ticket 01 readiness: is `ci/improve-cache-speed` landable as the stack-PR cache fix?

Date: 2026-10-29. Author: research sub-agent (ticket 01). Read-only on product code; this file is the only change.

## Verdict: DO NOT LAND the branch as-is — land its cache fix only, extracted as a standalone change

`ci/improve-cache-speed` (head `3505f294b`) is **not landable as-is** as "the stack-PR cache fix"
because the branch is the whole stack, not a cache fix. `git log --oneline dev..ci/improve-cache-speed`
returns **41 commits**, and `git diff dev..ci/improve-cache-speed --stat` is **473 files, ~118,404
insertions / 8,838 deletions** — the stack's product work (`stack/01-core-anchors` anchor domain,
`stack/02-split-board` half-frames/part anchoring, `stack/03-toolbar-polish` 10-tool pill/rails,
`stack/04-browser-migration` Rust harness, layout canvas/inspector rewrites) plus the cache work.
Merging it would land ~118k lines of unreviewed-for-this-ticket product code under a cache-fix label.

The actual cache fix inside it is small, coherent, and well-evidenced. It should be landed **with changes**:
cherry-pick / re-apply exactly these 5 commits onto `dev` as a standalone CI-only change:

| Commit | Subject | Files touched |
|---|---|---|
| `2874034e2` | stable rust-cache key + cache the qualification target | `.github/workflows/check.yaml`, `scripts/check.py` (1 line: register new test), `scripts/test-ci-workflow.py` |
| `281b7a286` | cancel superseded `pull_request` validation runs | `.github/workflows/check.yaml`, `scripts/test-ci-workflow.py` |
| `8e647283a` | same-repo PRs save their own rust-cache scope | `.github/workflows/check.yaml`, `scripts/test-ci-workflow.py` |
| `b5b964a80` | dependabot PRs do not save (quota) | `.github/workflows/check.yaml`, `scripts/test-ci-workflow.py` |
| `3505f294b` | audit doc | `docs/research/2026-10-28-ci-build-time-audit.md` (43 lines, doc-only) |

What each cache commit does (all line refs are to
`git show ci/improve-cache-speed:.github/workflows/check.yaml` unless noted):

1. **Stable rust-cache key** (`2874034e2`): workflow-level `RUSTUP_HOME: /home/runner/.rustup-pinned`
   (check.yaml:19-23) so rust-cache hashes only the `rust-toolchain.toml`-pinned toolchain
   (`channel = "1.98.0"`, per `git show ci/improve-cache-speed:rust-toolchain.toml`) instead of the
   runner image's moving preinstalled stable; `rustup toolchain install` runs *before* rust-cache in
   all three jobs (check.yaml:41, 65, 128; ordering pinned by
   `test_toolchain_is_installed_before_the_cache_hashes_rustc` in
   `git show ci/improve-cache-speed:scripts/test-ci-workflow.py:93-100`); `env-vars: ImageOS`
   keeps image families apart in the key (check.yaml:55, 99, 143; pinned by same file:111-117).
2. **Qualification-target coverage** (`2874034e2`): native `workspaces` gains
   `cad/monstertruck-qualification -> target` (check.yaml:53). The before-state on `dev`
   (`git show dev:.github/workflows/check.yaml:33-36`) lists only `.`, `cad/monstertruck`,
   `cad/wasm` — the `cad_build.py` owner-test target was built but never cached (commit message of
   `2874034e2`; audit doc §"Cache accessibility and gaps"). Coverage is pinned by
   `test_native_caches_every_cad_target_dir` (test-ci-workflow.py:124-129).
3. **PR-scope saves** (`8e647283a`, tightened by `b5b964a80`): native and web `save-if` becomes
   `main || dev || (pull_request && same-repo && actor != 'dependabot[bot]')` (check.yaml:56, 100).
   Before-state on `dev` is `main || dev` only (`git show dev:.github/workflows/check.yaml:37, 63`),
   so every stack PR targeting a feature branch (push triggers are `main, dev` only — check.yaml:4-5)
   started fully cold: the cold-green baseline receipt records `Swatinem/rust-cache: "No cache found"`
   and `save-if: false` on PR scope. GitHub scopes a `pull_request` cache write to
   `refs/pull/N/merge`, restorable only by later pushes to that same PR (commit message of
   `8e647283a`) — i.e. stacked PRs stop starting cold from their *second* push on, while
   `main`/`dev`/sibling scopes are unreadable and unwritten by it.
4. **Run cancellation** (`281b7a286`): concurrency group becomes per-PR-number for `pull_request`
   events with `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` (check.yaml:11, 14).
   Before-state on `dev` uses a run-unique group with `cancel-in-progress: false`
   (`git show dev:.github/workflows/check.yaml:11-12`), so a new push never cancels the stale run.
   `dev`/`main` pushes and dispatches keep run-unique/`pages` groups and are never cancelled
   (check.yaml:11-14; pinned by `ConcurrencyTests`, test-ci-workflow.py:132-140).
5. **Audit doc** (`3505f294b`): `docs/research/2026-10-28-ci-build-time-audit.md` lives only on the
   branch (`git show dev:<that path>` → `fatal: path ... does not exist in 'dev'`). It is the
   evidentiary basis for the fix and for the wall-time reasoning below; landing it with the fix
   preserves the rationale next to the code.

Deliberately **excluded** from "the cache fix" (stack product work that happens to touch CI files on
this branch — do not cherry-pick as part of ticket 01): the `ubuntu-24.04` native pin
(check.yaml:28-29, from stack commit `f7642d3ee` for the KiCad 10 PPA) — fine to keep when the stack
lands, but it is not a cache change; the Chrome-for-Testing cache + pinned-Chrome steps
(check.yaml:74-92, from stack commits `707e87c70`/`aa30da07b`); and the `web-tools/action.yml`
playwright-core removal (from stack commit `132b61a8c`). None of these is required for cache
reachability, and each deserves its own ticket's receipt.

`scripts/test-ci-workflow.py` on the branch hermetically pins the whole policy — save matrix
(same-repo PR saves; fork/dependabot/other-branch/dispatch do not, :57-71), release-build never
saves (:73-76), cache-on-failure stays off (:78-84), no trigger can write the default-branch scope
(:86-91, `permissions: contents: read` + triggers limited to `pull_request`/`push`/`workflow_dispatch`),
tool caches version-keyed (:102-109) — and `scripts/check.py` registers it in `TOOLING_TESTS`
(commit `2874034e2`, `scripts/check.py` hunk adding `"scripts/test-ci-workflow.py"`). That test
coverage is why the extracted change is low-risk to land: the policy is machine-checked in CI itself.

## Predicted warm-wall range: ~22–27 min wall (≈7–12 min saved vs the 33m37s cold baseline)

Baseline (no re-measurement; receipt on the parent checkout at
`.evidence/web-ci-green-baseline-d650e1dd/receipt.md`):
cold green run https://github.com/runinit/boardstudio/actions/runs/40791505198 on head `d650e1ddf`
(`stack/02-split-board`): **web 33m37s** (`build 624.3s + browser 1363.4s`, both cold — rust-cache
miss, wasm-pack miss), **native 30m19s** (cold), web is the PR wall by ~3m18s. Previous red run
40776517605 (47m02s) is inflated by timeouts/cascades and is not a baseline.
Warm reference: PR 7 run https://github.com/runinit/boardstudio/actions/runs/40730153041 —
**web 22m52s with a dev-cache restore** (31s restore, wasm-pack hit; build 326.4s) and native 32m50s
on a native miss (audit doc table; receipt "Implication" section).

Reasoning (all comparisons same-head-controlled unless noted — and the one available warm-vs-cold
pair is *not* same-head, so treat it as a bound, not a measurement; the audit doc itself warns the
warm 326.4s vs cold 718.8s difference "is **not** a controlled cache experiment", §"Inside the jobs"):
- **Web build (~10m24s cold → ~5.5m warm):** the only true cache datum is build 624.3s cold
  (receipt) vs 326.4s warm-with-restore (audit doc, PR 7 job 122963956010) → ~5m bound on build
  savings. Expect ~4–5m.
- **Web browser (~22m43s cold → ~16–21m warm):** cold browser 1363.4s ran fully cold compiles
  (wasm-pack miss) plus 107 `run-wasm-tests` in 314.2s and a 35.8s page `--list` inflated by cold
  compile (receipt). PR 7's whole warm web job was 22m52s total (build 326.4s + browser 988.4s).
  Expect browser ~1000–1250s warm → ~2–6m saving; the test-execution floor (~300s+ of real browser
  tests) does not compress with cache.
- **Native (30m19s cold → ~20–28m warm, the uncertainty):** there is *no* observed warm native run —
  PR 7 native (32m50s) was also a miss, and the cold-green native (30m19s) is the only green datum.
  The fix adds two things for native: a stable key (previously every image bump invalidated it) and
  the qualification target. Native lint alone is ~515s and test ~1040–1160s per the audit's PR 10/PR 7
  job logs, so even a partial restore should take several minutes off; but with no warm sample the
  honest interval is wide.
- **Wall = max(native, web), not the sum** (audit doc §"What is actually on the critical path"):
  making an already-faster job faster saves zero wall. Cold wall is web-bound (33m37s vs 30m19s). If
  warm web lands ~22–24m and warm native lands anywhere in 20–28m, the warm wall is **~22–27m**,
  i.e. **~7–12m saved**, with the bottleneck possibly flipping to native. Do not promise more: the
  first push after landing still runs **cold** (key change via `RUSTUP_HOME`/`ImageOS` invalidates
  existing `dev` caches → one cold transition that *pays* save time), and only the *second* push on
  a PR restores that PR's own scope. wasm-pack (~9 MB, audit doc) and CFT caches are seconds-scale
  and immaterial to the wall.

## Trust / quota risks (all bounded by the branch's own policy + tests)

- **Untrusted-PR cache writes:** same-repo PRs write only their own `refs/pull/N/merge` scope, which
  no other ref can read (commit message `8e647283a`; enforced by
  `test_no_trigger_can_write_the_default_branch_cache_scope`, test-ci-workflow.py:86-91, plus
  top-level `permissions: contents: read`, check.yaml:7-8). Fork PRs never save (check.yaml:56, 100
  `head.repo.full_name == github.repository` gate; matrix case "fork pull request" → False,
  test-ci-workflow.py:67). Dependabot is explicitly excluded from saving by `b5b964a80`
  (`github.actor != 'dependabot[bot]'`; matrix case :66). The residual trust surface is a PR
  poisoning *its own* scope — self-inflicted, invisible to `main`/`dev`/siblings. Acceptable.
- **10 GB quota / LRU eviction:** the audit doc's inventory showed ~1.36 GB native + 2×~1.47 GB web
  on `dev` against the 10 GB quota (§"Cache accessibility and gaps"). Every active same-repo PR now
  keeps its own multi-GB `refs/pull/N/merge` scope (native ~2.8 GB-class per the `8e647283a` message).
  A handful of concurrent stack PRs can therefore LRU-evict the trusted `dev` caches — the exact
  failure mode being fixed would re-appear as cold `dev`-adjacent runs. Mitigations already in the
  branch: rust-cache skips the save on an exact key hit (so lockfile-stable PRs save nothing after
  the first push — commit message `8e647283a`), fork/dependabot exclusion (`b5b964a80`), and the
  ~9 MB wasm-pack/CFT caches are per-ref but negligible (check.yaml:68-69 comment "not worth
  restricting"). Residual risk stands: with ≥3–4 heavy concurrent PRs, watch the cache-usage API for
  `dev`-scope evictions. Ticket 03 should record inventory before/after.
- **Partial-target poisoning under cancellation:** left `cache-on-failure` off deliberately
  (check.yaml has no `cache-on-failure: true`; pinned by test-ci-workflow.py:78-84) because with
  per-PR `cancel-in-progress`, a superseded run's partial `target/` saved under the immutable key
  would be exact-hit — and never overwritten — by every later run (commit message `8e647283a`).
  Correct call; do not "fix" by enabling it.
- **Key-transition cost:** `RUSTUP_HOME` + `env-vars: ImageOS` change every rust-cache key, so landing
  day invalidates all four existing `dev`-scope entries — expect one fully-cold green run (~33m+save
  overhead) immediately after merge before any warm data exists. Not a regression; ticket 03 must not
  mistake the transition run for the steady state.
- **Non-cache wall risks the branch does not address** (out of scope, noted so ticket 03/11/12 don't
  re-litigate): scheduler lock-wait/CPU saturation on 4-vCPU runners, buffered-log attribution
  limits, and the 5m37s cargo-audit/deny install (security workflow, off the PR critical path) — all
  per the audit doc §§"Inside the jobs", "Priorities and measurement".

## Exactly what ticket 03 must do to confirm

1. Land only the 5 listed commits (or their exact re-application) on `dev`; verify
   `git log --oneline dev..HEAD -- .github/` shows nothing else and
   `python3 scripts/check.py repo tooling` passes locally including the new
   `scripts/test-ci-workflow.py` suite.
2. Same-head A/B on hosted CI: push the cache change on a scratch ref at the *same code head* as a
   control run, then record for each of cold-transition run and second-push warm run: run/job URLs,
   per-job walls (native, web), rust-cache hit/miss + compressed bytes + restore/save seconds, exact
   restore key + scope, wasm-pack/CFT hit state, executed test counts (107 `run-wasm-tests` + all
   crate launches, 0 failed), and full green gate. Compare against control head `d650e1ddf` run
   https://github.com/runinit/boardstudio/actions/runs/40791505198 (web 33m37s, native 30m19s).
   Do not use red runs (e.g. 40776517605, 47m02s) as the "before".
3. Prove PR-scope isolation: on a test PR, show push 2 restores push 1's `refs/pull/N/merge` save
   (restore log cites the N/merge scope), and show the cache-inventory API has no new writes to the
   `dev`/`main` scopes from PR runs.
4. Record cache-inventory (entries, bytes, scopes) before landing and after the warm run; flag any
   `dev`-scope eviction as a quota finding for follow-up (possible save-if tightening or scheduled
   `dev` warming).
5. Confirm the transition run (first post-merge `dev` run, expected cold + save overhead) is not
   scored as steady state; score steady state from the run after.
6. If warm wall lands outside 22–27m, do not re-tune blindly: collect the audit doc's priority-2
   telemetry (child start/end, CPU/peak-RSS, target-lock wait) before proposing scheduler/runner
   changes — per the audit doc, no resource bottleneck is established yet.
