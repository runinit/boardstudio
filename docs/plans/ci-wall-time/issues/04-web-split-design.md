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
empty. The handoff reports 44 runner, 11 lane and 38 receipt tests passed. Real-browser parity,
enforced isolated targets and fresh assets, and authenticated receipt/run provenance remain
promotion blockers. This is local experimental work, not a production gate change or topology
approval; no speedup is established.

## Local baseline — 2026-10-29

One unsharded `python3 scripts/check.py build browser` passed at
`feat/dependency-aware-web-execution@932bce6f266b395fc6bf3649a14587ad261f08f9` in 746.5s
(build 299.6s, browser 446.6s). The [baseline record](../wiki/baseline.md#local-web-baseline--2026-10-29)
links the log and results and records counts and cache conditions. This is not a same-head split
comparison, hosted CI speed claim, or fully cold-cache full-gate result. The human design decision
and split execution parity, isolation, fresh assets and provenance remain open.
