# 04: Web-split design

Status: ready-for-human
Type: grilling
Blocked by: [03: Warm-baseline measurement](03-warm-baseline-measure.md)
Parent: [CI wall time map](../map.md)
Related: [13: Browser structural](../../check-speed/issues/13-browser-structural.md), [16: Partition proof](16-web-partition-proof.md), [17: Native topology](17-native-topology.md)

## Question

How should the single `web` job (build plus browser serial) split into parallel jobs — which
shards, which artifacts are shared, and what does the transfer cost?

## Pointers

- 2026-10-29: 03 provides warm anchors; measure the actual dependency graph and consumer inputs
  before choosing a full-coverage topology. The reduced PR tier in 09 is superseded, not a blocker.
  Compare independent crate batches and presentation consumers, build overlap versus build-once
  handoff, artifact compatibility and provenance, restores, setup and the longest shard. Do not
  assume every consumer needs the site or that a shared build is beneficial.
- Close to check-speed 13 option A (one cargo build) but scoped differently: 13 targets local
  tooling, this targets CI runners — that distinction is why it lives here.
- Ticket 16 is non-executing accounting, not proof of launch or test-name parity. Evaluate the
  existing `scripts/verification_receipt.py` for actual execution evidence and aggregation only
  after choosing a topology; identify gaps for browser page, crate and smoke coverage.
- This remains a human-in-the-loop topology and artifact decision with the user once the numbers
  exist. Preserve all tests, isolation, prerequisites and fail-closed required-check semantics.

## Execution experiment — 2026-10-29

On `feat/dependency-aware-web-execution` at `932bce6f2`, the experimental runner rejects every
nonzero page invocation. Pinned wasm-bindgen 0.2.129 can suppress the post-summary console-read
warning and emit the ordinary failure chain, so that chain is not proof of completion. This is
deliberately stricter than the intended allowlisted-failure policy; the allowlist is presently
empty. The handoff reports 44 runner, 11 lane and 38 receipt tests passed. At that handoff real-browser parity,
enforced isolated targets and fresh assets, and authenticated receipt/run provenance remain
promotion blockers. This is local experimental work, not a production gate change or topology
approval; no speedup is established.

## Selected trust design — 2026-10-29

The selected design under the user's delegated authority is recorded in the
[dated decision](../wiki/decisions.md): independent site and crates jobs on standard hosted
runners, fresh workspaces and complete local lane validation; stable `web` requires both direct
host job conclusions to succeed. Receipts and failure uploads are diagnostic, not acceptance
evidence; no artifact transfer is merge-critical. Preserve release's `native`/`web` dependencies
and least token privileges. This does not change the gate, ownership or ticket status.

The trust and full-rerun policy assumptions in that decision are prerequisites, not established
facts. In particular, workflow and gate-defining PR scripts, fixtures, config and allowlist must
all have enforceable review protection; a host conclusion cannot prove hostile PR code honest.
Actual protections and partial-rerun behavior need hosted verification. Validate independent
writable targets, fresh assets, full command and named-result parity, failure/skip/cancellation
handling, required-check configuration, and comparable cold and warm end-to-end walls before
promotion. Continue the existing trusted-branch cache policy; no new PR-scoped Rust-cache saves
until scope, trust, reachability, quota and eviction are validated. Cold misses run the full gate.

The page runner's nonzero-exit fail-closed behavior described below remains stricter than the
intended allowlist policy; the allowlist is empty. Local parity and sequential timings do not
establish the aspirational approximately ten-minute cold full-gate target.

## Local baseline — 2026-10-29

One unsharded `python3 scripts/check.py build browser` passed at
`feat/dependency-aware-web-execution@932bce6f266b395fc6bf3649a14587ad261f08f9` in 746.5s
(build 299.6s, browser 446.6s). The [baseline record](../wiki/baseline.md#local-web-baseline--2026-10-29)
links the log and results and records counts and cache conditions. This is not a same-head split
comparison, hosted CI speed claim, or fully cold-cache full-gate result. The human design decision
and split execution parity, isolation, fresh assets and provenance remained open at that point.

## Local same-head split parity — 2026-10-29

Independent reviewer thread `f53c3ba4-2c72-4101-8558-5900b098eaea` compared the unsharded
baseline at `feat/dependency-aware-web-execution@932bce6f266b395fc6bf3649a14587ad261f08f9`
with sequential local site and crate lanes, run ID `websplit-932bce6f-20261029-local-01`.
Names and outcomes matched for 14 crate launches and 587 tests, including both filtered launch
pairs; 77 page names across three filters; 13 ordered drag checks; 364 ordered CAD browser checks
including repeated descriptions; and eight provider Node tests. The existing aggregate accepted
the lane receipts against the current plan; it does **not** compare them to the baseline.

Sources: [baseline log](/home/chris/.t3/worktrees/ts-boardstudio2/t3-897cd35c/.evidence/web-baseline-932bce6f-wyru0xk7.log),
[site receipt](/home/chris/.t3/worktrees/ts-boardstudio2/t3-9e9e2b3a/.evidence/site-websplit-932bce6f-20261029-local-01.json),
and [crate receipt](/home/chris/.t3/worktrees/ts-boardstudio2/t3-71bb0344/.evidence/crates-websplit-932bce6f-20261029-local-01.json).
These local artifacts may not be available elsewhere. Site took 506.9s and crates 439.970s,
sequentially; the 746.5s unsharded baseline is not a comparable full-gate or CI speed measurement,
and these runs establish no parallel speedup.

This is local, unauthenticated test-result parity: source identity was locally clean, receipts do
not authenticate provenance, and the baseline did not record per-command numeric exit codes.
Drag build digests differed; the matching empty-change digest is not a source-tree hash. Enforced
isolation, fresh assets, authenticated provenance, the human topology decision, and comparable
hosted full-gate evidence remain open. No production gate change or CI readiness is claimed.

## Experimental workflow and review — 2026-10-29

`ci/experimental-web-gate-topology@4e238b6f9c6253d1d7a9a25bf6fb36c74ebaf20c` implements
the selected two-job design and stable `web` aggregate; it is not deployed. An independent review
confirmed the planned lane inventory and aggregate structure by inspection, but identified a
missing wasm-pack cache writer and opt-in workflow tests. Follow-up `@3d4a3bc9` adds one
successful site-lane cache writer limited to `main`/`dev` pushes. `@54e58b336e3c4c202ef2eaf5918e1e0192586318`
registers the workflow contract in the tooling gate, declares pinned PyYAML setup, and checks exact
triggers. The handoffs report workflow tests 7/7, check tests 31/31, quick-selection tests 15/15,
and lane tests 12/12 passing; the full tooling gate and `actionlint` were not run.

The review's protection, hosted required-check, fork, cancellation and partial-rerun findings
remain promotion blockers. The full-rerun policy is not enforced by the aggregate. Receipts are
diagnostic, not authenticated proof; cache behavior, hosted parity and cold timing remain unverified.
The package setup requires access to the pinned package or a populated pip cache. No hosted CI,
push, merge or performance claim follows from these local changes.

The native workflow experiment is a separate branch: at `perf/native-ci-topology@90411c08` it
moves the tooling gate to `native-tooling`. Combining the experiments will require installing the
pinned dependency in that job and updating the web contract's exact job inventory and native
setup assertions; neither branch alone validates the combined workflow.
