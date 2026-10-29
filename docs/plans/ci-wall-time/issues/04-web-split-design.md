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

## Chromiumoxide baseline correction — 2026-10-29

The user corrected the experiment's browser baseline: the target is the Chromiumoxide migration,
not the older Playwright path. A concurrent attempt at `932bce6f` failed site preflight because
`NODE_PATH` was missing; crates passed in 474.6s without site running alongside it. A retry was
stopped when the mismatch was identified. Neither is a valid migrated-gate comparison. Previous
local parity and timings on `932bce6f` are historical results for that older source only.

The user selected `stack/04-browser-migration@5a9c8ab2bdf65d8debeebabb40a9342a1a0e3a18` as
the source for a separate experiment, without modifying the stack branch, and requires **no Node
anywhere in the gate**. Its drag smoke uses the Rust/Chromiumoxide harness and its web-tools action
does not install Playwright, but `cad/scripts/cad_build.py` still requires Node to run eight
packaged-provider regression scripts; `scripts/check.py` also registers Node-based tooling tests.
Do not remove or skip checks for a timing result. A no-Node full gate with equivalent coverage must
be established before this performance experiment proceeds. No migrated baseline, concurrent
timing, hosted run or speedup is established.

The user authorized a local Node-free migration on a separate experimental branch, preserving
test cases and behavior; hosted CI remains untouched. Read-only inventory identified three
direct Node tooling tests, an indirect `node --version` measurement probe, and the eight
packaged-provider regressions as explicit blockers. Migration must retain the actual behavior
checks, eight named zero-repair cases, package freshness and failure propagation; browser-side
JavaScript through Chromium is not a Node process. Pinned third-party action runtimes and the
subprocess trees of build tools remain unaudited. A later authorized gate run must observe its
process tree rather than assuming a PATH shim proves the absence of Node. Do not claim migrated
coverage or performance until validated.

On the separate `research/chromiumoxide-web-split` branch, `2a2e30e5c4bb0884b9829f1375be43b624fa9c80`
removes the `node --version` environment probe from CAD measurement capture and explicitly logs
that it was not probed. Other probes and `NODE_PATH` provenance remain. The handoff reports all
30 focused Python tests passed, with mocked negative controls for bare and absolute Node paths;
these were not independently rerun for this entry. No Node-based gate test was removed or replaced
by this change; the full gate and its subprocesses remain unaudited.

The first opt-in face-completeness port was initially blocked by absent generated WEB outputs and
an unavailable tracer. With the user's approval, a distribution-signed `strace` was downloaded,
checked against local package metadata and keyring, and extracted privately; controls observed
descendant `execve` and `execveat`. A fresh `--no-opt` CAD WEB build at `2a2e30e5` completed in
47.094s in an offline network namespace with `/usr/bin/node` masked. The JS, WASM and input stamp
exist and the stamp matches. The wrapper exited 1 because its initial scan incorrectly matched
47 command-line arguments and paths containing `npm` or the Node-mask path; a subsequent scan of
all 1,211 recorded executable paths found no executable named Node, npm, npx or Playwright.
Evidence is local at `chromiumoxide-web-split-experiment/.evidence/cad-web-no-node-20261029T140203Z-f137af59-*`.
This is not proof against embedded or renamed runtimes, a full-gate Node-independence claim, a
real-provider regression result, or a timing comparison. The face-completeness port continues.

Read-only tooling-test design found that the Chromiumoxide harness may run the actual model-lookup
and CAD worker code with controlled fakes, but an ordinary browser realm is not the bare V8 context
used by the existing model test. The CAD worker test also exercises the real download STL/oracle
helper, not just the worker. The Ergogen test additionally exercises a real CLI, filesystem and
Git safety behavior; its upstream parser and CommonJS dependencies are not vendored. A fixture-only
replacement would lose coverage. No-Node upstream execution, exact dependency provenance and
byte-for-byte generator parity remain unproven; resolve these before removing existing gate paths.

An additional read-only Ergogen investigation found the pinned upstream commit in local Git
objects and reported that the locked package tarballs were present locally with matching archive
integrity. The required clean external checkout and installed dependency tree were absent. A
Chromiumoxide prototype loading authenticated original CommonJS sources is plausible, but the
locked dependency graph's browser initialization and byte-identical fixture output have not been
demonstrated. Archive integrity does not authenticate an arbitrary installed tree, and browser
bundles require separate provenance and license review. Preserve the real upstream oracle, CLI,
filesystem protections and exact output; do not replace them with fixture-only checks.
