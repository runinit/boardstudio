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
Ticket 04's human topology and artifact-sharing decision, actual browser parity and measured
shard/transfer walls remain open. No speed gain is claimed.
