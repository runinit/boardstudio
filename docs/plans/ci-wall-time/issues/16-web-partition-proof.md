# 16: Web partition completeness prototype

Status: resolved
Type: prototype
Parent: [CI wall time map](../map.md)
Related: [04: Web-split design](04-web-split-design.md)

## Scope

Claimed 2026-10-29 by the local web partition implementation sub-agent. Prototype only a
deterministic partition and fail-closed completeness check based on the existing browser plan,
with hermetic tests. Do not execute shards, change CI, reuse artifacts, or assert a speedup.
This does not decide or approve ticket 04's human-owned topology and artifact-sharing decision.

Acceptance for the prototype: preserve the exact existing browser commands, including separate
page listing, page-run isolation and filtered crate launches; reject omitted, duplicated and
unexpected launches and missing, failed or stale shard results. Explain the source-identity and
execution evidence still needed before any aggregate can serve as a required gate.

## Outcome — 2026-10-29

Non-executing prototype `research/web-partition-proof` at `52cd0df34e0e93e9d97f1ec96deccf53906e0a18`
partitions the supplied `scripts/check.py` browser plan deterministically and checks reported
commands, unique shards, success, source identity and run identity. Hermetic proof tests pass
5/5, including missing, duplicate and unexpected launches/shards, failures, stale identities
and command-plan parity; `test-check.py` passes 31/31. The test is explicitly opt-in, not a
new gate. No browser or Cargo build, full gate, timing, hosted CI, PR or merge was run.

The contiguous partition is an accounting example, **not a runnable or approved topology**:
it may divide dependent commands. Reports and identities are supplied by the caller; the proof
cannot authenticate them or establish that a launch actually ran. To integrate, bind reports to
observed completed launches and independently established source and unique run identities;
preserve the existing runner's failure, known-failure, listing and isolation semantics and
verify test-name parity. Independent execution needs isolated writable targets and browser
sessions, prerequisite ordering, and artifact compatibility/provenance checks before reuse.
At this prototype's completion, ticket 04's human topology and artifact-sharing decision, actual
browser parity and measured shard/transfer walls remained open. No speed gain is claimed.

## Subsequent experiment — 2026-10-29

The separate execution experiment `feat/dependency-aware-web-execution@932bce6f2` rejects every
nonzero page invocation because the pinned wasm-bindgen 0.2.129 warning can be suppressed even
when the ordinary failure chain appears. This is stricter than the intended allowlisted-failure
policy; the allowlist is currently empty. Its handoff reports 44 runner, 11 lane and 38 receipt
tests passed. At that handoff real-browser parity, enforced isolated targets and fresh assets, and authenticated
receipt/run provenance remain promotion blockers. This does not retroactively make this ticket's
accounting prototype execution proof or change the production gate; see [04](04-web-split-design.md).

On 2026-10-29, one unsharded local build/browser baseline at
`feat/dependency-aware-web-execution@932bce6f266b395fc6bf3649a14587ad261f08f9` passed in
746.5s. See the [baseline record](../wiki/baseline.md#local-web-baseline--2026-10-29) for source
artifacts, coverage and conditions. It is not a split comparison or execution proof for this
prototype, a hosted CI speed claim, or a fully cold-cache full-gate result; ticket 04 remains open.

The later [local same-head comparison](04-web-split-design.md#local-same-head-split-parity--2026-10-29)
on 2026-10-29 found test-result parity for the separate execution experiment. It does not turn
this ticket's non-executing prototype into execution proof or resolve ticket 04.
