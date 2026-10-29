# Node-free gate lane — completion plan (2026-10-29)

Planning only. This document implements nothing, runs nothing, and removes no
check. Local work only until a step below names an explicit user gate.

## Verified state (all citations checked 2026-10-29)

### Tickets (on `origin/ci-wall-time-plan@08964f4a7`, not on this branch)

- 19 Ergogen oracle: `Status: ready-for-agent`
  (`origin/ci-wall-time-plan:docs/plans/ci-wall-time/issues/19-node-free-ergogen-oracle.md:3`),
  blocked by 18 (ibid. `:5`). One-case prototype only: all 42 authored cases
  byte-identical to a Node reference (ibid. `:38-39`), but "not a run of the
  real generator CLI", loader "not a candidate to ship as-is", Chromium ran
  `--no-sandbox` (ibid. `:40-44`). Next experiment required: real CLI
  `--check`, all 42 cases, invalid/wrong-HEAD/dirty/protected-path/
  atomic-write/subprocess controls (ibid. `:99-105`).
- 21 Tooling tests: `ready-for-agent`, blocked by 19
  (`…/21-node-free-tooling-tests.md:3-5`). Warns a browser page is not
  automatically equivalent to the model-lookup test's bare V8 context, and
  the CAD worker test exercises its real download STL/oracle helper
  (ibid. `:17-18`).
- 22 Gate verification: `ready-for-agent`, blocked by 19 + 21
  (`…/22-node-free-gate-verification.md:3-5`); hosted-action runtimes
  unaudited and never to be marked Node-free unaudited (ibid. `:21-23`).
  Its latest comment still says three tooling `.cjs` tests and
  `node --version` remain (ibid. `:59`) — that count is stale by one after
  the CAD-worker cutover below.
- 23 Performance baseline: `ready-for-agent`, blocked by 22
  (`…/23-migrated-gate-performance-baseline.md:3-5`); local timings do not
  prove a ten-minute hosted gate (ibid. `:22`).
- Map: near-term focus forbids weakening/removing checks and hosted runs or
  merges until the user returns to GitHub work
  (`origin/ci-wall-time-plan:docs/plans/ci-wall-time/map.md:3-8`); ticket 21
  status records the CAD worker move at `9d76fb00` (local only) with
  "model-lookup, two `.cjs` checks and `node --version` remain"
  (ibid. `:325-326`); out of scope includes any change to
  `stack/02-split-board` (ibid. `:376-378`).

### Landed locally (`research/chromiumoxide-web-split`, 4 ahead of origin)

`6cf70965d` face-completeness browser regression → `0f5d8ae18` face gate →
`3d816250c` CAD provider regressions in browser → `9d76fb001` CAD worker
protocol in browser (removes `cad/scripts/test-measure-cad-worker.cjs` from
the gate; manual download validator retains its checks). Tickets
18/20/20a/20b/20c are resolved for the local replacement decision only
(`origin/ci-wall-time-plan:…/18-face-completeness-replacement.md:3`,
`…/20-remaining-cad-provider-regressions.md:3`,
`…/20c-shared-batch-cutover.md:3`). Do not redo any of these.

### Remaining Node surface in the gate at `9d76fb001`

- `scripts/check.py:71` — `TOOLING_TESTS` still lists
  `scripts/test-model-lookup.cjs` and `scripts/test-ergogen-parity.cjs`
  (the two remaining `.cjs` tooling tests).
- `scripts/check.py:164` — tooling step starts with `["node", "--version"]`;
  `scripts/check.py:167` dispatches any `.cjs` script via `node`.
- `scripts/test-check.py:59-61` — asserts Node stays a prerequisite for
  exactly those two tests.
- Intentionally **not** in scope: the manual download validator
  `cad/scripts/check-cad-page-downloads.py:92` shells to Node on purpose.
  It is not a gate step; do not plan its removal.

### In-flight work to sequence around (do not duplicate, do not touch)

Another session's dirty checkout
`/home/chris/.local/share/opencode/worktree/3ca6f9/chromiumoxide-web-split-experiment`
(same branch head `9d76fb001`, files modified 2026-10-29 15:06–15:45)
holds uncommitted, unreviewed WIP: a Node-free Ergogen generator
(`scripts/ergogen-parity/{generate,verified_runtime,sandbox_generate,host_probe}*`,
namespace-sandboxed, Node masked, Chrome still `--no-sandbox` inside) and a
model-lookup browser pilot (`scripts/model-lookup-pilot*`, documenting
adaptations vs the bare-V8 test: worker-prototype `TEMPORARY`/`PERSISTENT`
leak, stricter CSP, unexecuted branches undetected). This plan assumes that
session finishes or hands off that WIP; it does not re-plan its internals.

Standing constraints applied throughout: keep every required check until
its replacement is validated **and** independently reviewed; never edit the
vendor fixture paths `cad/scripts/provider_pages.py:62-63`
(Panasonic_EVQPUL_EVQPUC.step, infused-kim SMD_0805_Capacitor.step); record
the material impact of any removal; no contact with `stack/*`, `main`, `dev`.

## Ordered steps

### 1. Reconcile the in-flight experiment WIP
- **Owner:** the experiment-checkout session (its author), then this lane's
  implementing session.
- **Do:** land the Ergogen-generator and model-lookup-pilot WIP as local
  commits on `research/chromiumoxide-web-split`, or record in tickets 19/21
  what was discarded and why. Update ticket 22's stale "three `.cjs`"
  count to two when the CAD-worker cutover is recorded there.
- **Acceptance:** `git status` of that checkout is clean or every dirty
  file is accounted for in a ticket comment; no gate file changes behaviour
  yet (old `.cjs` tests still registered in `scripts/check.py:71`).
- **User gate:** none (local bookkeeping). Discarding the other session's
  WIP instead of landing it requires that session's owner, not this lane.

### 2. Finish the Node-free Ergogen oracle (ticket 19)
- **Owner:** implementing agent session (continuing step 1's WIP).
- **Do:** real CLI `--check` + output generation + byte comparison for all
  42 cases; reproduce invalid/wrong-HEAD/dirty-cache, protected-path,
  atomic-write and subprocess-failure controls; corruption fails before
  executing or writing; adequate sandboxing — the ticket's prototype used
  `--no-sandbox` with file access
  (`origin/ci-wall-time-plan:…/19-node-free-ergogen-oracle.md:44,69`) and
  the WIP's namespace boundary must be assessed, not assumed; per-package
  licence/notice and provenance review for any bundled runtime (ibid.
  `:48-56`).
- **Acceptance:** Node-free run matches the committed 212,288-byte fixture
  exactly and every negative control of `scripts/test-ergogen-parity.cjs`
  has a demonstrated equivalent; old required test still in the gate.
- **User gate:** any decision to regenerate or alter the committed fixture,
  or to accept a weaker sandbox than the Node CLI's filesystem/Git
  protections, is the user's — not the implementer's.

### 3. Independent review of the Ergogen replacement
- **Owner:** a reviewer session that did not implement step 2.
- **Do:** one combined spec + standards review (gate-contract change) of
  step 2's diff, parity evidence and sandbox claim.
- **Acceptance:** written verdict in ticket 19; objections either fixed or
  recorded as blockers. No cutover before this passes.
- **User gate:** none, unless the review surfaces a fixture or sandbox
  decision reserved in step 2.

### 4. Cut over the Ergogen gate registration
- **Owner:** implementing agent session.
- **Do:** register the Node-free check, remove
  `scripts/test-ergogen-parity.cjs` from `TOOLING_TESTS`
  (`scripts/check.py:71`), update `scripts/test-check.py:59-61`, record in
  ticket 19 what coverage changed (expected: none) and any impact.
- **Acceptance:** gate inventory diff shows exactly one-for-one replacement
  with identical named cases; the removed Node test remains runnable
  manually during a soak period or its removal impact is recorded.
- **User gate:** none if steps 2–3 passed with no reserved decisions
  (standing rule: removal only after validation + independent review).

### 5. Finish the model-lookup replacement (ticket 21, first half)
- **Owner:** implementing agent session (continuing the pilot from step 1).
- **Do:** prove parity with the bare-V8 `node:vm` test
  (`scripts/test-model-lookup.cjs:5`), or enumerate every adaptation the
  pilot already documents (prototype-chain constants, CSP string-codegen,
  unexecuted-branch detection, async completion window) and justify each
  as not weakening regression detection.
- **Acceptance:** same named cases and failure diagnostics pass Node-free;
  each accepted adaptation is written into ticket 21 with its impact.
- **User gate:** accepting any adaptation that detects strictly less than
  the bare-V8 test (e.g. unexecuted-branch globals) is a user decision —
  that is a coverage change, not an implementation detail.

### 6. Cut over model-lookup and remove the Node version probe
- **Owner:** implementing agent session.
- **Do:** register the replacement, remove `scripts/test-model-lookup.cjs`
  from `scripts/check.py:71`; once no `.cjs` remains in the tooling step,
  remove `["node", "--version"]` (`scripts/check.py:164`) and the `.cjs`
  dispatch (`scripts/check.py:167`); update `scripts/test-check.py:59-61`.
  Independently re-review this cutover (spec + standards) as in step 3.
- **Acceptance:** `rg 'node' scripts/check.py` shows no gate invocation of
  Node; named-case inventory unchanged; review verdict in ticket 21, which
  can then resolve. The manual validator at
  `cad/scripts/check-cad-page-downloads.py:92` is untouched and still
  documented as intentionally Node.
- **User gate:** none beyond step 5's, provided review passes.

### 7. Complete local Node-free gate verification (ticket 22, local half)
- **Owner:** implementing agent session, with an independent recheck by a
  second session.
- **Do:** reconcile original vs replacement command/named-case inventories
  across all steps (repo, tooling, lint, build, test, browser); run the
  complete local gate at one head with isolated targets; audit the actual
  process tree and executable provenance, stating what the evidence cannot
  establish (renamed/embedded runtimes), per
  `origin/ci-wall-time-plan:…/22-node-free-gate-verification.md:16-23`.
  Includes verifying the packaged-provider freshness limits already
  recorded (shared package stale; private stamp does not authenticate
  output, ibid. `:59-60`).
- **Acceptance:** full-gate result, inventory reconciliation and audit
  limits recorded in ticket 22; independent recheck agrees. Focused suite
  passes do not substitute.
- **User gate:** none (local run). No hosted run is implied by success.

### 8. Hosted-action runtime audit (ticket 22, hosted half)
- **Owner:** implementing agent session (audit locally from workflow
  sources first: `.github/workflows/check.yaml`, no `setup-node` present
  at audit time, but wasm-pack/dx/action internals are unaudited).
- **Do:** identify every runtime hosted actions and build subprocesses
  actually execute; propose a Node-free hosted path if one is needed.
- **Acceptance:** ticket 22 records either a defensible Node-free hosted
  boundary or a concrete blocker; nothing is labelled Node-free unaudited.
- **User gate:** **any hosted run, workflow change, push, or merge requires
  explicit user authorization.** Audit-from-source alone needs none.

### 9. Migrated-gate performance baseline (ticket 23)
- **Owner:** implementing agent session.
- **Do:** only after step 7 (and step 8's audit) — same-head unsharded vs
  split local runs with comparable cache/resource conditions, reconciled
  inventories; sequential parity kept separate from concurrent wall time
  (`origin/ci-wall-time-plan:…/23-migrated-gate-performance-baseline.md:14-22`).
- **Acceptance:** local receipt in ticket 23 stating plainly that it does
  not prove a ten-minute hosted gate.
- **User gate:** **hosted comparison runs require explicit authorization.**

### 10. Landing
- **Owner:** user, with an agent session executing.
- **Do:** promote the locally verified branch toward `dev` only through
  the user's chosen route; update tickets 19/21/22/23 and the map from
  stale counts to final state.
- **Acceptance:** landed head has a green complete gate receipt.
- **User gate:** **merges, PRs and branch-protection changes are entirely
  the user's call.** This plan opens no PR.

## Critical path

Steps 1 → 2 → 3 → 4 and 1 → 5 → 6 converge at 7; then 8 → 9 → 10.
The long pole is step 2 (sandbox + CLI-protection parity for Ergogen);
steps 5–6 can proceed in parallel with 2–4 once step 1 lands.

## Highest-risk assumption

That a browser-hosted Ergogen runtime can simultaneously keep the Node
CLI's Git/filesystem protections and run under adequate sandboxing: the
only demonstrated configurations so far are a `--no-sandbox` prototype
(ticket 19) and a namespace wrapper whose inner Chrome still runs
`--no-sandbox` (in-flight WIP). If the sandbox review (step 2/3) rejects
that boundary, ticket 19 blocks 21's probe removal, 22 and 23 — the whole
lane stalls at its first step with no fallback yet designed.
